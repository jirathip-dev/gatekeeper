use crate::config::read_toml;
use crate::run::CheckCx as Cx;
use crate::{Error, Result, packs, runner};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use toml::Table;

const PREFIX: &str = "rust/files/bans/";
const HEAD: &str = "[package]\nname = \"gatekeeper-bans\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
                    publish = false\n\n[lints]\nworkspace = true\n\n";

fn manifest(cx: &Cx) -> Result<String> {
    let repo = read_toml(&cx.root.join(cx.manifest))?;
    let lints = repo
        .get("workspace")
        .and_then(|w| w.get("lints"))
        .or_else(|| repo.get("lints"))
        .cloned()
        .unwrap_or_else(|| Table::new().into());
    let mut workspace = Table::new();
    workspace.insert("lints".into(), lints);
    let mut doc = Table::new();
    doc.insert("workspace".into(), workspace.into());
    let lints = toml::to_string(&doc).map_err(|e| Error(e.to_string()))?;
    Ok(format!("{HEAD}{lints}"))
}

fn refused(stdout: &str) -> BTreeSet<(String, String)> {
    let message = |line: &str| -> Option<Vec<(String, String)>> {
        let v: Value = serde_json::from_str(line).ok()?;
        let msg = v.get("message")?;
        let code = msg.get("code")?.get("code")?.as_str()?;
        let code = code.strip_prefix("clippy::").unwrap_or(code).to_owned();
        let spans = msg.get("spans")?.as_array()?;
        Some(
            spans
                .iter()
                .filter_map(|s| Some((s.get("file_name")?.as_str()?.to_owned(), code.clone())))
                .collect(),
        )
    };
    stdout.lines().filter_map(message).flatten().collect()
}

/// Each banned-API file is refused by clippy under the repo's own lints and toolchain, in one call.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let src = cx.scratch.join("src");
    fs::create_dir_all(&src).map_err(|e| Error::at(&src, e))?;
    fs::write(cx.scratch.join("Cargo.toml"), manifest(cx)?)
        .map_err(|e| Error::at(cx.scratch, e))?;
    packs::materialize(PREFIX, &src)?;
    let lints: Vec<&str> = packs::under(PREFIX)
        .filter_map(|(rel, _)| rel.strip_suffix(".rs"))
        .collect();
    let lib = lints
        .iter()
        .fold(String::new(), |s, l| s + "pub mod " + l + ";\n");
    fs::write(src.join("lib.rs"), lib).map_err(|e| Error::at(&src, e))?;
    let out = runner::capture(
        "cargo",
        &["clippy", "--quiet", "--message-format=json"],
        cx.scratch,
    )?;
    let hits = refused(&String::from_utf8_lossy(&out.stdout));
    let mut findings: Vec<String> = lints
        .iter()
        .filter(|l| !hits.contains(&(format!("src/{l}.rs"), (**l).to_owned())))
        .map(|l| {
            format!("{PREFIX}{l}.rs: clippy did not refuse it: the repo's lints let `{l}` through")
        })
        .collect();
    if !findings.is_empty() {
        findings.push(String::from_utf8_lossy(&out.stderr).trim().to_owned());
    }
    Ok(findings)
}
