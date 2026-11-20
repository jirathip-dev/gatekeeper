use crate::{Error, Result, check};
use gatekeeper_model::{Config, Gate, GateFile, GateId, PackFile, PackId, Platform, Step};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/packs.rs"));
}

pub(crate) fn under(prefix: &str) -> impl Iterator<Item = (&'static str, &'static [u8])> + '_ {
    embedded::FILES
        .iter()
        .filter_map(move |(path, bytes)| Some((path.strip_prefix(prefix)?, *bytes)))
}

#[must_use]
pub(crate) fn text(path: &str) -> Option<&'static str> {
    let (_, bytes) = embedded::FILES.iter().find(|(p, _)| *p == path)?;
    std::str::from_utf8(bytes).ok()
}

#[must_use]
pub(crate) fn tool_file(name: &str) -> Option<(&'static str, &'static str)> {
    let suffix = format!("/tools/{name}.toml");
    let (path, _) = embedded::FILES.iter().find(|(p, _)| p.ends_with(&suffix))?;
    Some((path, text(path)?))
}

pub(crate) fn materialize(prefix: &str, dest: &Path) -> Result<()> {
    for (rel, bytes) in under(prefix) {
        let path = dest.join(rel);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| Error::at(dir, e))?;
        }
        fs::write(&path, bytes).map_err(|e| Error::at(&path, e))?;
    }
    Ok(())
}

fn parse_gate(id: GateId, path: &str, text: &str, pack: &PackFile) -> Result<Gate> {
    let file: GateFile = toml::from_str(text).map_err(|e| Error(format!("{path}: {e}")))?;
    let gate = Gate::from_file(id, file, pack)
        .ok_or_else(|| Error(format!("{path}: give `run` or `[[step]]`, not both")))?;
    for step in &gate.steps {
        match step {
            Step::Check { check } if !check::NAMES.contains(&check.as_str()) => {
                return Err(Error(format!("{path}: no built-in check `{check}`")));
            }
            Step::Tool { tool, .. } if tool_file(tool).is_none() => {
                return Err(Error(format!("{path}: no pinned tool `{tool}`")));
            }
            _ => {}
        }
    }
    Ok(gate)
}

fn gate_id(pack: PackId, path: &str) -> Result<GateId> {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    GateId::new(pack, stem).ok_or_else(|| Error(format!("{path}: `{stem}` is not a gate name")))
}

pub(crate) fn pack_gates(pack: PackId) -> Result<Vec<Gate>> {
    let meta = format!("{pack}/pack.toml");
    let file = text(&meta).ok_or_else(|| Error(format!("no pack `{pack}`")))?;
    let pack_file: PackFile = toml::from_str(file).map_err(|e| Error(format!("{meta}: {e}")))?;
    let prefix = format!("{pack}/gates/");
    under(&prefix)
        .filter(|(rel, _)| {
            !rel.contains('/') && Path::new(rel).extension().is_some_and(|x| x == "toml")
        })
        .map(|(rel, bytes)| {
            let path = format!("{prefix}{rel}");
            let text = std::str::from_utf8(bytes).map_err(|e| Error(format!("{path}: {e}")))?;
            parse_gate(gate_id(pack, rel)?, &path, text, &pack_file)
        })
        .collect()
}

pub(crate) fn local_gates(root: &Path) -> Result<Vec<Gate>> {
    let dir = root.join(".gatekeeper/gates");
    let Ok(entries) = fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let pack = PackFile {
        about: "the repo's own gates".into(),
        platform: Platform::Linux,
        setup: BTreeSet::new(),
    };
    let mut paths: Vec<_> = entries
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let text = fs::read_to_string(path).map_err(|e| Error::at(path, e))?;
            let shown = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
            parse_gate(gate_id(PackId::Local, &shown)?, &shown, &text, &pack)
        })
        .collect()
}

/// # Errors
pub fn configured(config: &Config, root: &Path) -> Result<Vec<Gate>> {
    let mut gates = Vec::new();
    for pack in &config.packs {
        if *pack == PackId::Local {
            return Err(Error(
                "`local` is not a pack to list: .gatekeeper/gates/ is always read".into(),
            ));
        }
        gates.extend(pack_gates(*pack)?);
    }
    gates.extend(local_gates(root)?);
    Ok(gates)
}
