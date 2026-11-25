use super::machete::{at, workspace};
use crate::config::read_toml;
use crate::ratchet::pubs::{self, Member};
use crate::run::CheckCx as Cx;
use crate::{Result, graph, runner};
use std::fs;
use toml::{Table, Value};

fn published(manifest: &Table, root: &Table) -> bool {
    let own = manifest.get("package").and_then(|p| p.get("publish"));
    let inherit = own
        .and_then(|p| p.get("workspace"))
        .and_then(Value::as_bool)
        == Some(true);
    let shared = root.get("workspace").and_then(|w| w.get("package"));
    match if inherit {
        shared.and_then(|p| p.get("publish"))
    } else {
        own
    } {
        Some(Value::Boolean(b)) => *b,
        Some(Value::Array(a)) => !a.is_empty(),
        _ => true,
    }
}

/// The `pub` scan over the workspace members' files: a `pub` value no other crate names, an item only tests name, a type nothing names.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let (dir, prefix) = workspace(cx);
    let root = read_toml(&dir.join("Cargo.toml"))?;
    let (mut members, mut exempt) = (Vec::new(), Vec::new());
    for n in graph::cargo::read(&dir)? {
        let own = if n.root == "." { "" } else { &n.root };
        let manifest = read_toml(&dir.join(at(own, "Cargo.toml")))?;
        let published = published(&manifest, &root);
        if published {
            exempt.push(n.name);
        }
        let dir = Some(at(&prefix, own)).filter(|d| !d.is_empty());
        let dir = dir.unwrap_or_else(|| ".".into());
        members.push(Member { dir, published });
    }
    let args = ["ls-files", "-co", "--exclude-standard", "--", "*.rs"];
    let listed = runner::ok("git", &args, &dir)?;
    let files: Vec<(String, String)> = listed
        .lines()
        .filter_map(|p| Some((at(&prefix, p), fs::read_to_string(dir.join(p)).ok()?)))
        .collect();
    if !exempt.is_empty() {
        cx.note
            .set(format!("published, so exempt: {}", exempt.join(", ")))
            .ok();
    }
    let exported = &cx.rust.exported.0;
    let stray = exported
        .iter()
        .filter(|g| !files.iter().any(|(p, _)| g.matches(p)))
        .map(|g| format!("[rust.exported] `{g}` matches no file"));
    let mut found: Vec<String> = stray.collect();
    found.extend(pubs::scan(&members, &files, exported));
    Ok(found)
}
