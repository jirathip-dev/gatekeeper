use gatekeeper_model::{Affected, Glob, Graph, Trigger, Widened};
use std::collections::BTreeSet;

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
