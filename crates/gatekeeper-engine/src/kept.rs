use crate::{Error, Result};
use gatekeeper_model::{Gate, Kept};
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
