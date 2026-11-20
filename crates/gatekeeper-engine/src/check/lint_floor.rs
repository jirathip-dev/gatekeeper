use crate::config::read_toml;
use crate::run::CheckCx as Cx;
use crate::{Error, Result, packs};
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

/// Every lint of the rust pack's floor is in the manifest's `[workspace.lints]` at its level or stricter.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let floor: Table =
        toml::from_str(packs::text("rust/files/lint-floor.toml").unwrap_or_default())
            .map_err(|e| Error(format!("lint floor: {e}")))?;
    let manifest = read_toml(&cx.root.join(cx.manifest))?;
    let lints = manifest
        .get("workspace")
        .and_then(|w| w.get("lints"))
        .or_else(|| manifest.get("lints"))
        .and_then(Value::as_table);
    let Some(lints) = lints else {
        return Ok(vec![format!("{}: no [workspace.lints]", cx.manifest)]);
    };
    let mut findings = Vec::new();
    for (group, wanted) in floor.iter().filter_map(|(g, t)| Some((g, t.as_table()?))) {
        for (lint, want) in wanted.iter().filter_map(|(l, v)| Some((l, v.as_str()?))) {
            let has = lints.get(group).and_then(|g| g.get(lint)).and_then(level);
            match has {
                None => findings.push(format!(
                    "{}: [workspace.lints.{group}] has no {lint}; the floor is \"{want}\"",
                    cx.manifest
                )),
                Some(has) if rank(has) < rank(want) => findings.push(format!(
                    "{}: [workspace.lints.{group}] {lint} is \"{has}\", under the floor \"{want}\"",
                    cx.manifest
                )),
                Some(_) => {}
            }
        }
    }
    Ok(findings)
}
