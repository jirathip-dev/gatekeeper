use super::any;
use super::config::Docs;
use super::frozen::Change;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn stale(root: &Path, d: &Docs, files: &[String]) -> Vec<String> {
    let mut bad = vec![];
    for p in files.iter().filter(|p| any(&d.paths, p)) {
        let dir = root.join(Path::new(p).parent().unwrap_or(Path::new("")));
        let mut fence = false;
        let text = std::fs::read_to_string(root.join(p)).unwrap_or_default();
        for (i, l) in text.lines().enumerate() {
            fence ^= l.trim_start().starts_with("```");
            if fence || l.trim_start().starts_with("```") {
                continue;
            }
            for s in l.split('`').skip(1).step_by(2) {
                let from = if s.starts_with("../") { &dir } else { root };
                let gap = (!any(&d.skip, s)).then(|| missing_path(s, from)).flatten();
                bad.extend(gap.map(|g| format!("{p}:{}: `{s}`: {g}", i + 1)));
            }
            for t in l
                .split("](")
                .skip(1)
                .filter_map(|r| r.split([')', ' ', '#']).next())
            {
                if !t.is_empty() && !t.contains(':') && !dir.join(t).exists() {
                    bad.push(format!("{p}:{}: link `{t}` does not exist", i + 1));
                }
            }
        }
    }
    bad
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
    let f = root.join(path.trim_end_matches('/'));
    if !f.exists() {
        return Some("no such path".into());
    }
    let n = line?;
    let short = std::fs::read_to_string(&f).map_or(0, |t| t.lines().count()) < n;
    short.then(|| format!("{path} has fewer than {n} lines"))
}

pub(crate) fn undocumented(d: &Docs, diff: &[Change]) -> Vec<String> {
    let docs = |n: &str| {
        diff.iter()
            .any(|c| any(&d.paths, &c.file) && c.text.contains(n))
    };
    let key = |n: &str| {
        !n.is_empty()
            && n.bytes()
                .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
    };
    let (mut seen, mut bad) = (BTreeSet::new(), vec![]);
    for c in diff.iter().filter(|c| any(&d.watch, &c.file)) {
        let t = c.text.trim_start().trim_start_matches("pub ");
        let Some((name, _)) = t
            .split_once(':')
            .filter(|(n, r)| key(n) && !r.starts_with(':'))
        else {
            continue;
        };
        if seen.insert(name) && !docs(name) && !docs(&name.replace('_', "-")) {
            let (f, n, how) = (&c.file, c.line, if c.added { "added" } else { "removed" });
            bad.push(format!(
                "{f}:{n}: `{name}` {how}, but no changed doc line names it (or `no doc: <why>` in the PR body)"
            ));
        }
    }
    bad
}
