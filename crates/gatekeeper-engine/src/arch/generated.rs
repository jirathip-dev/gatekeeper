use super::config::{Arch, PATH};
use super::{any, walk};
use crate::{Error, Result, runner};
use gatekeeper_model::Glob;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(crate) fn unsized_shared(a: &Arch) -> Vec<String> {
    let covered = |g: &Glob| {
        let g = g.to_string();
        let probe = g.replace("**", "x/y").replace('*', "x");
        let probe = if g.ends_with("/**") {
            probe + "/z.rs"
        } else {
            probe
        };
        a.size.iter().any(|s| any(&s.paths, &probe))
    };
    let why = "falls under no `[[size]]` rule: a shared file must be kept from growing";
    (a.shared.iter().filter(|g| !covered(g)))
        .map(|g| format!("{PATH}: shared `{g}` {why}"))
        .collect()
}

fn matched(root: &Path, globs: &[Glob]) -> Vec<String> {
    let named: Vec<String> = globs.iter().map(ToString::to_string).collect();
    let mut found = vec![];
    walk(root, "", &named, &mut found);
    found.into_iter().filter(|p| any(globs, p)).collect()
}

pub(crate) fn stale(root: &Path, a: &Arch) -> Result<Vec<String>> {
    let Some(cmd) = a.generate.as_deref().filter(|_| !a.generated.is_empty()) else {
        return Ok(vec![]);
    };
    let (globs, files): (Vec<Glob>, Vec<Glob>) = a
        .generated
        .iter()
        .cloned()
        .partition(|g| g.to_string().contains('*'));
    let mut before = BTreeMap::<String, Option<Vec<u8>>>::new();
    for p in files.iter().map(ToString::to_string) {
        before.insert(p.clone(), fs::read(root.join(&p)).ok());
    }
    for p in matched(root, &globs) {
        let at = root.join(&p);
        before.insert(p, fs::read(&at).ok());
        fs::remove_file(&at).map_err(|e| Error::at(&at, e))?;
    }
    let ran = runner::capture("sh", &["-c", cmd], root);
    let mut paths: BTreeSet<String> = before.keys().cloned().collect();
    paths.extend(matched(root, &globs));
    let mut bad = vec![];
    for p in paths {
        let (at, was) = (root.join(&p), before.get(&p).cloned().flatten());
        if fs::read(&at).ok() == was {
            continue;
        }
        bad.push(format!(
            "{p}: differs from what `{cmd}` writes: run it and commit the output, never hand-merge a generated file"
        ));
        match was {
            Some(b) => fs::write(&at, b),
            None => fs::remove_file(&at),
        }
        .map_err(|e| Error::at(&at, e))?;
    }
    let ran = ran?;
    if !ran.status.success() {
        let code = ran.status.code().unwrap_or(-1);
        let err = String::from_utf8_lossy(&ran.stderr);
        return Ok(vec![format!("`{cmd}` exited {code}: {}", err.trim())]);
    }
    Ok(bad)
}
