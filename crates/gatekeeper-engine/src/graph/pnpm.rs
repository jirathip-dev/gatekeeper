use super::{Named, members};
use crate::{Error, Result};
use gatekeeper_model::Glob;
use serde_json::Value;
use std::fs;
use std::path::Path;

const DEPENDENCIES: [&str; 4] = [
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
];

fn patterns(text: &str) -> Vec<String> {
    let mut listing = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let item = line.trim();
        if item.is_empty() || item.starts_with('#') {
            continue;
        }
        if !line.starts_with([' ', '\t', '-']) {
            listing = item.starts_with("packages:");
        } else if let Some(item) = item.strip_prefix('-').filter(|_| listing) {
            out.push(item.trim().trim_matches(['\'', '"']).to_owned());
        }
    }
    out
}

fn package(root: &Path, dir: &str) -> Result<Named> {
    let path = root.join(dir).join("package.json");
    let text = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
    let json: Value = serde_json::from_str(&text).map_err(|e| Error::at(&path, e))?;
    let name = json.get("name").and_then(Value::as_str).unwrap_or(dir);
    let needs = DEPENDENCIES
        .iter()
        .filter_map(|k| json.get(*k).and_then(Value::as_object))
        .flat_map(|deps| deps.keys().cloned());
    Ok(Named {
        name: name.to_owned(),
        root: dir.to_owned(),
        needs: needs.collect(),
    })
}

pub(super) fn read(root: &Path) -> Result<Vec<Named>> {
    let path = root.join("pnpm-workspace.yaml");
    let text = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
    let (mut include, mut except) = (Vec::new(), Vec::new());
    for pattern in patterns(&text) {
        let (list, pattern) = match pattern.strip_prefix('!') {
            Some(p) => (&mut except, p),
            None => (&mut include, pattern.as_str()),
        };
        let glob = Glob::try_from(pattern.trim_start_matches("./").to_owned());
        list.push(glob.map_err(|e| Error::at(&path, e))?);
    }
    let dirs = members(root, &include, &except, "package.json")?;
    dirs.iter().map(|d| package(root, d)).collect()
}
