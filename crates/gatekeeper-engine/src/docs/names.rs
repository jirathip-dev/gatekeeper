use super::scope::Doc;
use super::text::Text;
use crate::arch::any;
use crate::{Result, runner};
use gatekeeper_model::{Glob, Names, Prefix, SetName, Source};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) struct Sets<'a>(Vec<(&'a SetName, &'a [Prefix], BTreeSet<String>)>);

fn list(root: &Path, source: &Source, listed: &[Glob]) -> Result<(Option<String>, Vec<String>)> {
    match source {
        Source::File(f) => {
            let mut bad = vec![];
            if !any(listed, f) {
                bad.push(format!("names: `{f}` is not listed as generated"));
            }
            match std::fs::read_to_string(root.join(f)) {
                Ok(t) => Ok((Some(t), bad)),
                Err(e) => Ok((None, vec![format!("names: `{f}`: {e}")])),
            }
        }
        Source::Run(cmd) => {
            let out = runner::capture("sh", &["-c", cmd], root)?;
            if out.status.success() {
                return Ok((
                    Some(String::from_utf8_lossy(&out.stdout).into_owned()),
                    vec![],
                ));
            }
            let code = out.status.code().unwrap_or(-1);
            let err = String::from_utf8_lossy(&out.stderr);
            let end = err.trim().lines().last().unwrap_or_default().to_owned();
            Ok((None, vec![format!("names: `{cmd}` exited {code}: {end}")]))
        }
    }
}

pub(crate) fn read<'a>(
    root: &Path,
    names: Option<&'a Names>,
    listed: &[Glob],
) -> Result<(Sets<'a>, Vec<String>)> {
    let Some(names) = names else {
        return Ok((Sets(vec![]), vec![]));
    };
    let (text, mut bad) = match list(root, &names.source, listed)? {
        (Some(t), bad) => (t, bad),
        (None, bad) => return Ok((Sets(vec![]), bad)),
    };
    let mut found = BTreeMap::<SetName, BTreeSet<String>>::new();
    for (n, l) in text.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
        let mut f = l.splitn(3, '\t');
        let set = f.next().map(|s| SetName::try_from(s.to_owned()));
        match (set, f.next()) {
            (Some(Ok(set)), Some(name)) if !name.is_empty() && !name.contains('`') => {
                found.entry(set).or_default().insert(name.to_owned());
            }
            _ => bad.push(format!(
                "names: line {} is not <set> TAB <name> [TAB <description>]",
                n + 1
            )),
        }
    }
    let mut sets = vec![];
    for (set, prefixes) in &names.sets {
        let held = found.remove(set).unwrap_or_default();
        if held.is_empty() {
            bad.push(format!("names: set `{set}` has no name"));
        }
        sets.push((set, prefixes.as_slice(), held));
    }
    Ok((Sets(sets), bad))
}

pub(crate) fn unknown(sets: &Sets, skip: &[Glob], doc: &Doc, text: &Text) -> Vec<String> {
    let mut bad = vec![];
    for s in text.spans.iter().filter(|s| !any(skip, s)) {
        let claims = sets.0.iter().flat_map(|(set, prefixes, held)| {
            prefixes
                .iter()
                .filter_map(move |p| p.claim(s).map(|w| (set, w, held)))
        });
        let claims: Vec<_> = claims.collect();
        if claims.iter().any(|(_, w, held)| held.contains(*w)) {
            continue;
        }
        if let Some((set, w, _)) = claims.first() {
            bad.push(format!("{}: `{s}`: no {set} `{w}`", doc.path));
        }
    }
    bad
}
