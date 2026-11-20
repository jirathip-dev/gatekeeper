use crate::run::CheckCx as Cx;
use crate::{Error, Result, packs, runner};
use serde_json::Value;

const FILES: &str = "ts/files/oxlint/";

fn diagnostic(d: &Value) -> Option<String> {
    let file = d.get("filename")?.as_str()?;
    let code = d.get("code")?.as_str()?;
    let message = d.get("message")?.as_str()?;
    Some(format!("{file}: {code}: {message}"))
}

/// Pinned oxlint over the `[ts]` projects with the pack's config; each diagnostic is a finding.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let ts = cx.ts()?;
    let bin = cx.tool("oxlint")?;
    packs::materialize(FILES, cx.scratch)?;
    let config = cx
        .scratch
        .join("oxlintrc.json")
        .to_string_lossy()
        .into_owned();
    let mut args = vec!["-c", config.as_str(), "--format", "json"];
    args.extend(ts.projects.iter().map(String::as_str));
    let out = runner::capture(&bin.to_string_lossy(), &args, cx.root)?;
    let report: Value = serde_json::from_slice(&out.stdout).map_err(|e| {
        Error(format!(
            "oxlint printed no report ({e}): {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    })?;
    let findings: Vec<String> = report
        .get("diagnostics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(diagnostic)
        .collect();
    if findings.is_empty() && !out.status.success() {
        return Err(Error(format!(
            "oxlint exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(findings)
}
