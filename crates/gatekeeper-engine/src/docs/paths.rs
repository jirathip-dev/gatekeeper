use super::scope::Doc;
use super::text::Text;
use crate::arch::any;
use crate::runner;
use gatekeeper_model::Glob;
use std::path::{Path, PathBuf};

fn dir(root: &Path, doc: &Doc) -> PathBuf {
    root.join(Path::new(&doc.path).parent().unwrap_or(Path::new("")))
}

pub(crate) fn broken_links(root: &Path, doc: &Doc, text: &Text) -> Vec<String> {
    let dir = dir(root, doc);
    let gone = text.links.iter().filter(|t| !dir.join(t).exists());
    gone.map(|t| format!("{}: link `{t}` does not exist", doc.path))
        .collect()
}

pub(crate) fn stale(root: &Path, skip: &[Glob], doc: &Doc, text: &Text) -> Vec<String> {
    let dir = dir(root, doc);
    let mut bad = vec![];
    for s in text.spans.iter().filter(|s| !any(skip, s)) {
        let from = if s.starts_with("../") { &dir } else { root };
        bad.extend(missing_path(s, from).map(|g| format!("{}: `{s}`: {g}", doc.path)));
    }
    bad
}

fn ignored(path: &str, root: &Path) -> bool {
    let out = runner::capture("git", &["check-ignore", "-q", path], root);
    out.is_ok_and(|o| o.status.success())
}

fn missing_path(s: &str, root: &Path) -> Option<String> {
    let p = s.trim_start_matches("./");
    let odd = |c: char| c.is_whitespace() || "*<>{}…$~#?|()[]'\",@=".contains(c);
    if p.contains(odd) || p.contains("::") || p.contains("://") {
        return None;
    }
    let (path, line) = match p.rsplit_once(':') {
        Some((a, n)) => (a, Some(n.split('-').next()?.parse::<usize>().ok()?)),
        None => (p, None),
    };
    let first = path.split('/').next().unwrap_or_default();
    if !(path.contains('/') || line.is_some()) || first.is_empty() || !root.join(first).exists() {
        return None;
    }
    let path = path.trim_end_matches('/');
    let f = root.join(path);
    if !f.exists() {
        return (!ignored(path, root)).then(|| "no such path".into());
    }
    let n = line?;
    let short = std::fs::read_to_string(&f).map_or(0, |t| t.lines().count()) < n;
    short.then(|| format!("{path} has fewer than {n} lines"))
}
