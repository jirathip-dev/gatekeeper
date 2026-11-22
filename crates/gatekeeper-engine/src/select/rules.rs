use super::readers;
use gatekeeper_model::{
    Always, Change, ChangeKind, Evidence, Full, Read, Select, Selection, TestFile, Trigger, Walk,
    Why, Widen,
};
use std::collections::BTreeSet;

/// The test files a change can reach under a `[select]` gate, each with every rule that selected it.
#[must_use]
pub fn select(trigger: &Trigger, gate: &Select, evidence: &Evidence) -> Selection {
    let changes = match trigger {
        Trigger::Everything(why) => return Selection::Full(Full::NotAChange(why.clone())),
        Trigger::Change(changes) => changes,
    };
    let full = |c: &&Change| {
        c.path.starts_with(".gatekeeper/") || gate.full.iter().any(|g| g.matches(&c.path))
    };
    if let Some(change) = changes.iter().find(full) {
        return Selection::Full(Full::File(change.path.clone()));
    }
    let picked = evidence.universe.iter().filter_map(|test| {
        let whys = whys(test, changes, gate, evidence);
        (!whys.is_empty()).then(|| (test.clone(), whys))
    });
    Selection::of(picked.collect())
}

fn whys(
    test: &TestFile,
    changes: &BTreeSet<Change>,
    gate: &Select,
    ev: &Evidence,
) -> BTreeSet<Why> {
    let code = readers::code(ev.texts.get(test).map_or("", String::as_str));
    let mut whys = BTreeSet::new();
    if ev.closure.contains(test) {
        whys.insert(Why::Closure);
    }
    if gate.always.iter().any(|g| g.matches(&test.0)) {
        whys.insert(Why::Always(Always::Path));
    }
    let own = ev.graph.owner(&test.0).map(String::as_str);
    if own.is_none() {
        whys.insert(Why::Always(Always::NoProject));
    }
    let walks = readers::walks(&code);
    for change in changes {
        whys.extend(reads(&code, walks, own, change, ev));
    }
    whys
}

fn reads(code: &str, walks: bool, own: Option<&str>, change: &Change, ev: &Evidence) -> Vec<Why> {
    let path = &change.path;
    let home = ev.graph.owner(path).map(String::as_str);
    let shared = home.filter(|h| own == Some(*h));
    let name = path.rsplit('/').next().unwrap_or(path);
    let mut whys = Vec::new();
    if code.contains(path.as_str()) {
        whys.push(Why::Reads(Read::Path(path.clone())));
    }
    if name.trim_start_matches('.').contains('.') && readers::has_token(code, name) {
        whys.push(Why::Reads(Read::Basename(path.clone())));
    }
    if let Some(root) = shared.filter(|_| walks) {
        whys.push(Why::Reads(Read::Walks(Walk::Project(root.to_owned()))));
    }
    if let Some(dir) = readers::named_dir(code, path).filter(|_| walks) {
        whys.push(Why::Reads(Read::Walks(Walk::Dir(dir))));
    }
    if let Some(root) = home.filter(|r| *r != "." && readers::has_token(code, r)) {
        whys.push(Why::NamesProject(root.to_owned()));
    }
    let tested = name.contains(".test.") || name.contains(".spec.");
    if change.kind == ChangeKind::Deleted && !tested {
        if shared.is_some() {
            whys.push(Why::Widened(Widen::FormerProject(path.clone())));
        }
        if readers::imports(code, path) {
            whys.push(Why::Widened(Widen::Specifier(path.clone())));
        }
    }
    whys
}
