use crate::config::read_toml;
use crate::ratchet::reports;
use crate::run::CheckCx as Cx;
use crate::{Error, Result, graph, runner};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) fn workspace(cx: &Cx) -> (PathBuf, String) {
    let dir = Path::new(&cx.rust.manifest)
        .parent()
        .unwrap_or(Path::new(""));
    (cx.root.join(dir), dir.to_string_lossy().into_owned())
}

pub(crate) fn at(prefix: &str, rel: &str) -> String {
    if prefix.is_empty() {
        rel.to_owned()
    } else {
        format!("{prefix}/{rel}")
    }
}

fn ignored(dir: &Path, member: &str) -> Result<Vec<String>> {
    let manifest = at(if member == "." { "" } else { member }, "Cargo.toml");
    let table = read_toml(&dir.join(&manifest))?;
    let lists = ["package", "workspace"].into_iter().filter_map(|k| {
        let machete = table.get(k)?.get("metadata")?.get("cargo-machete")?;
        machete.get("ignored")?.as_array()
    });
    let names = lists.flatten().filter_map(toml::Value::as_str);
    Ok(names
        .map(|n| format!("{manifest}: ignored for cargo-machete: {n}"))
        .collect())
}

/// Pinned cargo-machete with `--with-metadata`; each unused dependency, and each name in a manifest's machete `ignored` list, is a finding.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let (dir, prefix) = workspace(cx);
    let bin = cx.tool("cargo-machete")?;
    let out = runner::capture(&bin.to_string_lossy(), &["--with-metadata"], &dir)?;
    let mut found = reports::machete(&String::from_utf8_lossy(&out.stdout));
    if out.status.code() != Some(0) && (found.is_empty() || out.status.code() != Some(1)) {
        return Err(Error(format!(
            "cargo-machete exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let mut members: BTreeSet<String> = graph::cargo::read(&dir)?
        .into_iter()
        .map(|m| m.root)
        .collect();
    members.insert(".".into());
    for m in &members {
        found.extend(ignored(&dir, m)?);
    }
    Ok(found.into_iter().map(|f| at(&prefix, &f)).collect())
}
