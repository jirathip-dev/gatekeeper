use crate::{Error, Result, VERSION, packs};
use gatekeeper_model::{Config, PackId, Pin};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PATH: &str = ".gatekeeper/config.toml";

/// # Errors
pub fn load(root: &Path) -> Result<Config> {
    let path = root.join(PATH);
    let text = std::fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
    let config: Config = toml::from_str(&text).map_err(|e| Error::at(&path, e))?;
    if let Pin::Release(pin) = &config.pin {
        if pin.version != VERSION {
            return Err(Error(format!(
                "{PATH}: [pin] names {}, this gatekeeper is {VERSION}",
                pin.version
            )));
        }
        let bad = |s: &String| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit());
        if let Some((target, _)) = pin.sha256.iter().find(|(_, s)| bad(s)) {
            return Err(Error(format!("{PATH}: [pin] {target} is not a sha256")));
        }
    }
    let tables = [
        (PackId::Ts, config.ts.is_some(), "projects, install"),
        (
            PackId::Swift,
            config.swift.is_some(),
            "sources, packages or projects",
        ),
    ];
    for (pack, table, keys) in tables {
        if config.packs.contains(&pack) != table {
            return Err(Error(format!(
                "{PATH}: the {pack} pack and its [{pack}] table ({keys}) go together"
            )));
        }
    }
    if let Some(swift) = &config.swift
        && (swift.sources.is_empty() || swift.packages.is_empty() && swift.projects.is_empty())
    {
        return Err(Error(format!(
            "{PATH}: [swift] names its sources and at least one of packages or projects"
        )));
    }
    if let Some(name) = config.tools.keys().find(|t| packs::tool_file(t).is_none()) {
        return Err(Error(format!("{PATH}: [tools] {name} is no pinned tool")));
    }
    Ok(config)
}

/// `[tools]`: the tools this repo builds itself, each at its path from the repo root.
#[must_use]
pub fn tools(config: &Config, root: &Path) -> BTreeMap<String, PathBuf> {
    let paths = config.tools.iter().map(|(t, p)| (t.clone(), root.join(p)));
    paths.collect()
}

pub(crate) fn read_toml(path: &Path) -> Result<toml::Table> {
    let text = std::fs::read_to_string(path).map_err(|e| Error::at(path, e))?;
    toml::from_str(&text).map_err(|e| Error::at(path, e))
}
