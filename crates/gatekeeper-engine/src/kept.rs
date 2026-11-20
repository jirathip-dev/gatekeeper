use crate::{Error, Result};
use gatekeeper_model::{Gate, Kept, Report};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn walk(dir: &Path, rel: &str, skip: &Path, out: &mut Vec<String>) -> Result<()> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|e| Error::at(dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        let kind = entry.file_type().map_err(|e| Error::at(dir, e))?;
        if kind.is_dir() && entry.path() != skip {
            walk(&entry.path(), &path, skip, out)?;
        } else if kind.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

pub(crate) fn collect(gate: &Gate, root: &Path, out: &Path) -> Result<Vec<Kept>> {
    let mut kept = Vec::new();
    for upload in &gate.uploads {
        let dest = out.join("files").join(&upload.name);
        if dest.exists() {
            fs::remove_dir_all(&dest).map_err(|e| Error::at(&dest, e))?;
        }
        let mut files = BTreeSet::new();
        for glob in &upload.paths {
            let dir = glob.dir();
            let mut found = Vec::new();
            walk(&root.join(&dir), &dir, out, &mut found)?;
            files.extend(found.into_iter().filter(|p| glob.matches(p)));
        }
        for file in &files {
            let to = dest.join(file);
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::at(parent, e))?;
            }
            let from = root.join(file);
            fs::hard_link(&from, &to)
                .or_else(|_| fs::copy(&from, &to).map(drop))
                .map_err(|e| Error::at(&from, e))?;
        }
        kept.push(Kept {
            name: upload.name.clone(),
            files: files.into_iter().collect(),
        });
    }
    Ok(kept)
}

fn of_leg(name: &str, leg: &str) -> bool {
    let index = name.strip_prefix(&format!("gates-{leg}-"));
    name == format!("gates-{leg}")
        || index.is_some_and(|i| !i.is_empty() && i.bytes().all(|b| b.is_ascii_digit()))
}

pub(crate) fn receive(reports: &Path, leg: &str, root: &Path) -> Result<bool> {
    let mut gates = Vec::new();
    for entry in fs::read_dir(reports).map_err(|e| Error::at(reports, e))? {
        let dir = entry.map_err(|e| Error::at(reports, e))?.path();
        if !of_leg(&dir.file_name().unwrap_or_default().to_string_lossy(), leg) {
            continue;
        }
        let path = dir.join("report.json");
        let Ok(text) = fs::read_to_string(&path) else {
            return Ok(false);
        };
        let report: Report = serde_json::from_str(&text).map_err(|e| Error::at(&path, e))?;
        let files = dir.join("files");
        let found = report.jobs.into_iter().flat_map(|j| j.gates);
        gates.extend(found.map(|g| (files.clone(), g)));
    }
    if gates.is_empty() || gates.iter().any(|(_, g)| g.outcome.fails()) {
        return Ok(false);
    }
    for (files, gate) in &gates {
        for kept in &gate.kept {
            for file in &kept.files {
                let (from, to) = (files.join(&kept.name).join(file), root.join(file));
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent).map_err(|e| Error::at(parent, e))?;
                }
                fs::copy(&from, &to).map_err(|e| Error::at(&from, e))?;
            }
        }
    }
    Ok(true)
}
