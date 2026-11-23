pub(crate) mod cargo;
mod pnpm;
mod swiftpm;

use crate::{Error, Result};
use gatekeeper_model::{Glob, Graph, GraphKind};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const SKIPPED: [&str; 2] = ["node_modules", "target"];

pub(crate) struct Named {
    pub(crate) name: String,
    pub(crate) root: String,
    needs: BTreeSet<String>,
}

fn by_name(found: &[Named]) -> Vec<(String, BTreeSet<String>)> {
    let roots: BTreeMap<&str, &str> = found
        .iter()
        .map(|n| (n.name.as_str(), n.root.as_str()))
        .collect();
    found
        .iter()
        .map(|n| {
            let needs = n.needs.iter().filter_map(|d| roots.get(d.as_str()));
            let needs = needs.filter(|r| **r != n.root).map(|r| (*r).to_owned());
            (n.root.clone(), needs.collect())
        })
        .collect()
}

fn walk(root: &Path, rel: &str, manifest: &str, out: &mut Vec<String>) -> Result<()> {
    let dir = root.join(rel);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|e| Error::at(&dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().map_err(|e| Error::at(&dir, e))?.is_dir();
        if !is_dir || name.starts_with('.') || SKIPPED.contains(&name.as_str()) {
            continue;
        }
        let path = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        if entry.path().join(manifest).is_file() {
            out.push(path.clone());
        }
        walk(root, &path, manifest, out)?;
    }
    Ok(())
}

fn members(root: &Path, globs: &[Glob], except: &[Glob], manifest: &str) -> Result<Vec<String>> {
    let mut found = BTreeSet::new();
    for glob in globs {
        let mut dirs = Vec::new();
        walk(root, &glob.dir(), manifest, &mut dirs)?;
        let kept = dirs.into_iter().filter(|d| glob.matches(d));
        found.extend(kept.filter(|d| !except.iter().any(|g| g.matches(d))));
    }
    Ok(found.into_iter().collect())
}

/// # Errors
pub fn read(root: &Path, kinds: &BTreeSet<GraphKind>) -> Result<Graph> {
    let mut graph = Graph::default();
    for kind in kinds {
        let found = match kind {
            GraphKind::Cargo => by_name(&cargo::read(root)?),
            GraphKind::Pnpm => by_name(&pnpm::read(root)?),
            GraphKind::SwiftPm => swiftpm::read(root)?,
        };
        for (project, needs) in found {
            graph.projects.entry(project).or_default().extend(needs);
        }
    }
    Ok(graph)
}
