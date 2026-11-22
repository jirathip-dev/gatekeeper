use crate::{Error, Result, runner};
use gatekeeper_model::TestFile;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

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
    let canonical = root.canonicalize().map_err(|e| Error::at(root, e))?;
    let relative = |file: &str| {
        let path = Path::new(file);
        let rel = path
            .strip_prefix(&canonical)
            .or_else(|_| path.strip_prefix(root));
        TestFile(rel.unwrap_or(path).to_string_lossy().into_owned())
    };
    Ok(listed.iter().map(|l| relative(&l.file)).collect())
}
