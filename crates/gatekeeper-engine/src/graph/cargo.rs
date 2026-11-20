use super::{Named, members};
use crate::config::read_toml;
use crate::{Error, Result};
use gatekeeper_model::Glob;
use std::path::Path;
use toml::{Table, Value};

fn globs(top: &Table, key: &str) -> Result<Vec<Glob>> {
    let list = top.get("workspace").and_then(|w| w.get(key));
    list.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|v| {
            let s = v.as_str().unwrap_or_default().to_owned();
            Glob::try_from(s).map_err(|e| Error(format!("Cargo.toml: workspace.{key}: {e}")))
        })
        .collect()
}

fn package(root: &Path, dir: &str) -> Result<Named> {
    let path = root.join(dir).join("Cargo.toml");
    let manifest = read_toml(&path)?;
    let name = manifest.get("package").and_then(|p| p.get("name"));
    let name = name
        .and_then(Value::as_str)
        .ok_or_else(|| Error::at(&path, "no package.name"))?;
    let needs = ["dependencies", "dev-dependencies", "build-dependencies"]
        .iter()
        .filter_map(|k| manifest.get(*k).and_then(Value::as_table))
        .flatten()
        .map(|(key, dep)| dep.get("package").and_then(Value::as_str).unwrap_or(key));
    Ok(Named {
        name: name.to_owned(),
        root: dir.to_owned(),
        needs: needs.map(str::to_owned).collect(),
    })
}

pub(super) fn read(root: &Path) -> Result<Vec<Named>> {
    let top = read_toml(&root.join("Cargo.toml"))?;
    let (include, except) = (globs(&top, "members")?, globs(&top, "exclude")?);
    let mut dirs = members(root, &include, &except, "Cargo.toml")?;
    if top.contains_key("package") {
        dirs.push(".".to_owned());
    }
    dirs.iter().map(|d| package(root, d)).collect()
}
