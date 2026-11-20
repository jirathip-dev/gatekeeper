use crate::run::CheckCx as Cx;
use crate::{Error, Result, runner};
use serde_json::Value;

fn named(item: &Value) -> Option<String> {
    match item {
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(named)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        item => Some(item.get("name")?.as_str()?.to_owned()),
    }
}

fn issues(issue: &Value) -> Vec<String> {
    let Some((file, kinds)) = issue
        .get("file")
        .and_then(Value::as_str)
        .zip(issue.as_object())
    else {
        return Vec::new();
    };
    kinds
        .iter()
        .filter_map(|(kind, items)| Some((kind, items.as_array()?)))
        .flat_map(|(kind, items)| {
            items
                .iter()
                .filter_map(named)
                .map(move |name| format!("{file}: {kind}: {name}"))
        })
        .collect()
}

/// Pinned knip with the repo's own knip config; each unused file, export or dependency is a finding.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    cx.ts()?;
    let bin = cx.tool("knip")?;
    let out = runner::capture(
        &bin.to_string_lossy(),
        &["--reporter", "json", "--no-progress"],
        cx.root,
    )?;
    let report: Value = serde_json::from_slice(&out.stdout).map_err(|e| {
        Error(format!(
            "knip printed no report ({e}): {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    })?;
    let findings: Vec<String> = report
        .get("issues")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(issues)
        .collect();
    if findings.is_empty() && !out.status.success() {
        return Err(Error(format!(
            "knip exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(findings)
}
