use crate::config::{PATH, read_toml};
use crate::graph::cargo;
use crate::run::CheckCx as Cx;
use crate::{Error, Result, packs};
use std::fs;
use std::path::Path;
use toml::{Table, Value};

fn level(v: &Value) -> Option<&str> {
    v.as_str().or_else(|| v.get("level")?.as_str())
}

fn rank(level: &str) -> u8 {
    match level {
        "warn" => 1,
        "deny" => 2,
        "forbid" => 3,
        _ => 0,
    }
}

fn under(floor: &Table, lints: &Table, at: &str) -> Vec<String> {
    let mut findings = Vec::new();
    for (group, wanted) in floor.iter().filter_map(|(g, t)| Some((g, t.as_table()?))) {
        for (lint, want) in wanted.iter().filter_map(|(l, v)| Some((l, v.as_str()?))) {
            match lints.get(group).and_then(|g| g.get(lint)).and_then(level) {
                None => findings.push(format!(
                    "{at}.{group}] has no {lint}; the floor is \"{want}\""
                )),
                Some(has) if rank(has) < rank(want) => findings.push(format!(
                    "{at}.{group}] {lint} is \"{has}\", under the floor \"{want}\""
                )),
                Some(_) => {}
            }
        }
    }
    findings
}

fn undocumented(root: &Path, dir: &Path, name: &str, out: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|e| Error::at(dir, e))? {
        let path = entry.map_err(|e| Error::at(dir, e))?.path();
        if path.is_dir() && !path.ends_with("target") {
            undocumented(root, &path, name, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
            let rel = path.strip_prefix(root).unwrap_or(&path).display();
            let mut documented = false;
            for (line, n) in text.lines().zip(1..) {
                let code = line.split("//").next().unwrap_or_default();
                if code.contains("unsafe {") && !documented && !line.contains("SAFETY:") {
                    out.push(format!(
                        "crate {name}: {rel}:{n}: unsafe with no // SAFETY:"
                    ));
                }
                let line = line.trim_start();
                documented = line.starts_with("//") && (documented || line.contains("SAFETY:"));
            }
        }
    }
    Ok(())
}

/// The rust pack's floor holds in `[workspace.lints]` and in each crate's own `[lints]`; `[rust] unsafe_crates` may lift `unsafe_code` but must deny undocumented unsafe blocks.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let floor: Table =
        toml::from_str(packs::text("rust/files/lint-floor.toml").unwrap_or_default())
            .map_err(|e| Error(format!("lint floor: {e}")))?;
    let path = cx.root.join(&cx.rust.manifest);
    let manifest = read_toml(&path)?;
    let workspace = manifest.get("workspace").and_then(|w| w.get("lints"));
    let lints = workspace.or_else(|| manifest.get("lints"));
    let Some(lints) = lints.and_then(Value::as_table) else {
        return Ok(vec![format!("{}: no [workspace.lints]", cx.rust.manifest)]);
    };
    let at = format!("{}: [workspace.lints", cx.rust.manifest);
    let mut findings = under(&floor, lints, &at);
    let dir = path.parent().unwrap_or(cx.root);
    let members = cargo::read(dir)?;
    let mut lifted = floor.clone();
    lifted.insert("rust".into(), Table::new().into());
    if let Some(Value::Table(clippy)) = lifted.get_mut("clippy") {
        clippy.insert("undocumented_unsafe_blocks".into(), "deny".into());
    }
    for m in members
        .iter()
        .filter(|m| m.root != "." || workspace.is_some())
    {
        let own = match read_toml(&dir.join(&m.root).join("Cargo.toml"))?.remove("lints") {
            Some(Value::Table(own)) => own,
            _ => Table::new(),
        };
        let at = format!("crate {}: {}/Cargo.toml: [lints", m.name, m.root);
        if cx.rust.unsafe_crates.contains(&m.name) {
            findings.extend(under(&lifted, &own, &at));
            undocumented(cx.root, &dir.join(&m.root), &m.name, &mut findings)?;
        } else if !own.contains_key("workspace") && !own.is_empty() {
            findings.extend(under(&floor, &own, &at));
        }
    }
    for n in &cx.rust.unsafe_crates {
        if !members.iter().any(|m| &m.name == n) {
            findings.push(format!("{PATH}: unsafe_crates: {n} is no workspace member"));
        }
    }
    Ok(findings)
}
