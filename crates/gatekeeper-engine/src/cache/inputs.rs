use crate::fetch::sha256_hex;
use crate::{Result, runner};
use gatekeeper_model::Gate;
use std::collections::BTreeMap;
use std::path::Path;

/// The worktree's files as a run starts: git's blob for a clean tracked file, the bytes' sha256 for a changed one.
pub struct Inputs {
    files: BTreeMap<String, String>,
    pub commit: Option<String>,
}

/// # Errors
pub fn read(root: &Path) -> Result<Inputs> {
    let mut files = BTreeMap::new();
    for entry in runner::ok("git", &["ls-files", "-s", "-z"], root)?.split('\0') {
        if let Some((meta, path)) = entry.split_once('\t') {
            let id = meta.rsplit_once(' ').map_or(meta, |(m, _)| m);
            files.insert(path.to_owned(), id.to_owned());
        }
    }
    let status = runner::ok(
        "git",
        &["status", "--porcelain", "-z", "--untracked-files=all"],
        root,
    )?;
    let mut entries = status.split('\0').filter(|e| e.len() > 3);
    let mut dirty = false;
    while let Some(entry) = entries.next() {
        dirty = true;
        let (xy, path) = entry.split_at(3);
        if xy.starts_with(['R', 'C']) {
            entries.next();
        }
        if xy.as_bytes().get(1) == Some(&b' ') {
            continue;
        }
        match std::fs::read(root.join(path)) {
            Ok(bytes) => files.insert(path.to_owned(), format!("file {}", sha256_hex(&bytes))),
            Err(_) => files.remove(path),
        };
    }
    let head = runner::ok("git", &["rev-parse", "HEAD"], root).ok();
    let commit = head.filter(|_| !dirty).map(|h| h.trim().to_owned());
    Ok(Inputs { files, commit })
}

impl Inputs {
    /// How many files a gate reads and the sha256 of their sorted `id path` lines.
    #[must_use]
    pub fn of(&self, gate: &Gate) -> (usize, String) {
        let lines: Vec<String> = self
            .files
            .iter()
            .filter(|(path, _)| gate.reads(path))
            .map(|(path, id)| format!("{id} {path}\n"))
            .collect();
        (lines.len(), sha256_hex(lines.concat().as_bytes()))
    }
}
