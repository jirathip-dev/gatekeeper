use crate::{Error, Result, runner};
use gatekeeper_model::{Millis, TestFile};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

#[derive(Deserialize)]
struct Listed {
    file: String,
}

/// # Errors
pub fn list(root: &Path, config: &str, changed: Option<&str>) -> Result<BTreeSet<TestFile>> {
    let bin = root.join("node_modules/.bin/vitest");
    if !bin.exists() {
        return Err(Error(format!(
            "{}: no vitest: [ts] install must install the repo's vitest",
            bin.display()
        )));
    }
    let mut args = vec!["list", "--filesOnly", "--json", "--passWithNoTests"];
    args.extend(["--config", config]);
    args.extend(changed.iter().flat_map(|base| ["--changed", base]));
    let out = runner::ok(&bin.to_string_lossy(), &args, root)?;
    let listed: Vec<Listed> = serde_json::from_str(&out)
        .map_err(|e| Error(format!("vitest {}: not a JSON list: {e}", args.join(" "))))?;
    let relative = relative(root)?;
    Ok(listed.iter().map(|l| relative(&l.file)).collect())
}

fn relative(root: &Path) -> Result<impl Fn(&str) -> TestFile> {
    let canonical = root.canonicalize().map_err(|e| Error::at(root, e))?;
    Ok(move |file: &str| {
        let path = Path::new(file);
        let rel = path
            .strip_prefix(&canonical)
            .or_else(|_| path.strip_prefix(root));
        TestFile(rel.unwrap_or(path).to_string_lossy().into_owned())
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonReport {
    test_results: Vec<FileResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileResult {
    name: String,
    start_time: f64,
    end_time: f64,
}

/// # Errors
pub fn times(root: &Path, path: &Path) -> Result<BTreeMap<TestFile, Millis>> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(BTreeMap::new());
    };
    let report: JsonReport = serde_json::from_str(&text)
        .map_err(|e| Error(format!("{}: not vitest's JSON report: {e}", path.display())))?;
    let relative = relative(root)?;
    let timed = report.test_results.iter().map(|r| {
        let secs = Duration::try_from_secs_f64((r.end_time - r.start_time) / 1000.0);
        (relative(&r.name), secs.unwrap_or_default().into())
    });
    Ok(timed.collect())
}
