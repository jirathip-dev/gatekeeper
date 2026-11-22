use crate::{Result, runner};
use gatekeeper_model::{Affected, Change, ChangeKind, Glob, Graph, Trigger, Widened};
use std::collections::BTreeSet;
use std::path::Path;

/// The projects a change can reach: those holding a changed file, then every project that needs one of them.
#[must_use]
pub fn affected(graph: &Graph, trigger: Trigger, ignore: &[Glob]) -> Affected {
    let files = match trigger {
        Trigger::Everything(why) => return Affected::All(why),
        Trigger::Change(files) => files,
    };
    let mut hit = BTreeSet::new();
    for file in files.into_iter().map(|c| c.path) {
        match graph.owner(&file) {
            Some(root) => {
                hit.insert(root.clone());
            }
            None if ignore.iter().any(|g| g.matches(&file)) => {}
            None => return Affected::All(Widened::OutsideEveryProject(file)),
        }
    }
    loop {
        let more: Vec<String> = graph
            .projects
            .iter()
            .filter(|(root, needs)| !hit.contains(*root) && !needs.is_disjoint(&hit))
            .map(|(root, _)| root.clone())
            .collect();
        if more.is_empty() {
            return Affected::Projects(hit);
        }
        hit.extend(more);
    }
}

/// # Errors
pub fn changed(root: &Path, base: &str) -> Result<BTreeSet<Change>> {
    let diff = runner::ok(
        "git",
        &["diff", "--name-status", "--no-renames", base],
        root,
    )?;
    let new = runner::ok("git", &["ls-files", "--others", "--exclude-standard"], root)?;
    let diffed = diff
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(status, path)| {
            let kind = match status {
                "A" => ChangeKind::Added,
                "D" => ChangeKind::Deleted,
                _ => ChangeKind::Modified,
            };
            (path, kind)
        });
    let added = new.lines().map(|path| (path, ChangeKind::Added));
    let changes = diffed.chain(added).map(|(path, kind)| Change {
        path: path.to_owned(),
        kind,
    });
    Ok(changes.collect())
}
