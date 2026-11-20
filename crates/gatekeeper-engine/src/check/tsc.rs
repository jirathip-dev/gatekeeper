use crate::run::CheckCx as Cx;
use crate::{Error, Result, runner};
use serde_json::Value;
use std::path::{Path, PathBuf};

const STRICT_FLAGS: &[&str] = &[
    "alwaysStrict",
    "noImplicitAny",
    "noImplicitThis",
    "strictBindCallApply",
    "strictBuiltinIteratorReturn",
    "strictFunctionTypes",
    "strictNullChecks",
    "strictPropertyInitialization",
    "useUnknownInCatchVariables",
];

fn tsc(root: &Path, project: &str) -> Option<PathBuf> {
    root.join(project)
        .ancestors()
        .take_while(|d| d.starts_with(root))
        .map(|d| d.join("node_modules/.bin/tsc"))
        .find(|p| p.is_file())
}

fn floor(project: &str, shown: &[u8]) -> Vec<String> {
    let options = serde_json::from_slice::<Value>(shown)
        .ok()
        .and_then(|v| v.get("compilerOptions").cloned())
        .unwrap_or_default();
    let mut findings = Vec::new();
    if options.get("strict").and_then(Value::as_bool) != Some(true) {
        findings.push(format!(
            "{project}/tsconfig.json: does not set \"strict\": true"
        ));
    }
    for flag in STRICT_FLAGS {
        if options.get(flag).and_then(Value::as_bool) == Some(false) {
            findings.push(format!(
                "{project}/tsconfig.json: switches {flag} off; strict needs every strict flag on"
            ));
        }
    }
    findings
}

/// After `[ts] install`, each project's tsconfig holds the strict floor and `tsc --noEmit -p` passes.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let ts = cx.ts()?;
    let install = runner::capture("sh", &["-c", &ts.install], cx.root)?;
    if !install.status.success() {
        return Err(Error(format!(
            "[ts] install `{}` failed: {}",
            ts.install,
            String::from_utf8_lossy(&install.stderr).trim()
        )));
    }
    let mut findings = Vec::new();
    for project in &ts.projects {
        let tsc = tsc(cx.root, project).ok_or_else(|| {
            Error(format!(
                "{project}: no node_modules/.bin/tsc in it or above it: [ts] install must install typescript"
            ))
        })?;
        let tsc = tsc.to_string_lossy();
        let shown = runner::capture(&tsc, &["--showConfig", "-p", project], cx.root)?;
        findings.extend(floor(project, &shown.stdout));
        let out = runner::capture(
            &tsc,
            &["--noEmit", "--pretty", "false", "-p", project],
            cx.root,
        )?;
        if !out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let errors: Vec<String> = text.lines().map(str::to_owned).collect();
            if errors.is_empty() {
                findings.push(format!("{project}: tsc exited {}", out.status));
            }
            findings.extend(errors);
        }
    }
    Ok(findings)
}
