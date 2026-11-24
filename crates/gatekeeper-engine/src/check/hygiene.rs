use crate::run::CheckCx as Cx;
use crate::{Error, Result, runner};
use gatekeeper_model::PrivateNames;
use std::fs;

fn hits<'a>(names: &'a PrivateNames, text: &'a str, at: &'a str) -> impl Iterator<Item = String> {
    (text.lines().zip(1..))
        .filter_map(move |(l, n)| Some(format!("private-name #{} at {at}:{n}", names.hit(l)?)))
}

/// The tracked files, the commit messages since the base and the PR body against the private names; no list: `list-missing`.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let Some(names) = cx.names else {
        return Ok(vec![
            "list-missing: no private-names list for a public repo: set $DROVER_PRIVATE_NAMES (CI: the private-names secret); a private repo turns core/hygiene off with a reason".into(),
        ]);
    };
    let mut rows = Vec::new();
    let files = runner::ok("git", &["ls-files", "-z"], cx.root)?;
    for p in files.split('\0').filter(|p| !p.is_empty()) {
        let text = fs::read_to_string(cx.root.join(p)).unwrap_or_default();
        rows.extend(hits(names, &text, p));
    }
    if let Some(base) = cx.base {
        let range = format!("{base}..HEAD");
        let log = runner::ok("git", &["log", "-z", "--format=%H%n%B", &range], cx.root)?;
        for commit in log.split('\0') {
            let (sha, message) = commit.trim_start().split_once('\n').unwrap_or((commit, ""));
            let found = message.lines().filter_map(|l| names.hit(l));
            rows.extend(found.map(|n| format!("private-name #{n} at commit {sha:.7} message")));
        }
    }
    if let Some(file) = cx.pr_body {
        let text = fs::read_to_string(file).map_err(|e| Error::at(file, e))?;
        rows.extend(hits(names, &text, "PR body"));
    }
    let names = names.len();
    let note = format!(
        "{} hit(s) of {names} private name(s); names are never printed",
        rows.len()
    );
    cx.note.set(note).ok();
    Ok(rows)
}
