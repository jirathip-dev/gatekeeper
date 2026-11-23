use super::any;
use super::config::{Arch, Collect, Frozen, PATH};
use crate::{Error, Result, runner};
use gatekeeper_model::Glob;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct Change {
    pub file: String,
    pub hunk: usize,
    pub line: usize,
    pub added: bool,
    pub created: bool,
    pub text: String,
}

pub(crate) fn diff(root: &Path, base: &str) -> Result<Vec<Change>> {
    let args = ["diff", "-U0", "--no-color", "--no-ext-diff", base];
    Ok(changes(&runner::ok("git", &args, root)?))
}

pub(crate) fn at_base(root: &Path, base: &str) -> Result<Vec<Glob>> {
    let out = runner::capture("git", &["show", &format!("{base}:./{PATH}")], root)?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let f: Frozen =
        toml::from_str(&text).map_err(|e| Error(format!("parsing {base}:{PATH}: {e}")))?;
    Ok(f.frozen)
}

fn changes(diff: &str) -> Vec<Change> {
    let (mut v, mut from, mut to, mut old, mut new, mut head, mut hunk) =
        (vec![], "", "", 0, 0, true, 0);
    let mut created = false;
    for l in diff.lines() {
        if l.starts_with("diff --git ") {
            (head, created) = (true, false);
        } else if let Some(h) = l.strip_prefix("@@ -") {
            let num = |s: &str| {
                s.split([',', ' '])
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0)
            };
            let (o, n) = h.split_once(" +").unwrap_or_default();
            (old, new, head, hunk) = (num(o), num(n), false, hunk + 1);
        } else if head {
            created |= l == "--- /dev/null";
            from = l.strip_prefix("--- a/").unwrap_or(from);
            to = l.strip_prefix("+++ b/").unwrap_or(to);
        } else if let Some((sign @ ("+" | "-"), text)) = l.split_at_checked(1) {
            let added = sign == "+";
            let (file, line) = if added {
                (to, &mut new)
            } else {
                (from, &mut old)
            };
            v.push(Change {
                file: file.into(),
                hunk,
                line: *line,
                added,
                created,
                text: text.into(),
            });
            *line += 1;
        }
    }
    v
}

pub(crate) fn adds(a: &Arch, frozen: &[Glob], diff: &[Change]) -> Vec<String> {
    let entries: Vec<String> = a.collect.iter().map(Collect::entries).collect();
    let instead = if entries.is_empty() {
        "a `[[collect]]` folder".to_string()
    } else {
        let folders: Vec<String> = entries.iter().map(|e| format!("`{e}`")).collect();
        folders.join(" or ")
    };
    let collected = |f: &str| {
        let globs = entries
            .iter()
            .filter_map(|e| Glob::try_from(e.clone()).ok());
        globs.into_iter().any(|g| g.matches(f))
    };
    let mut hunks = BTreeMap::<usize, (Vec<&Change>, usize)>::new();
    for c in diff
        .iter()
        .filter(|c| !c.created && any(frozen, &c.file) && !collected(&c.file))
    {
        let h = hunks.entry(c.hunk).or_default();
        if c.added {
            h.0.push(c);
        } else {
            h.1 += 1;
        }
    }
    let new = hunks
        .into_values()
        .flat_map(|(adds, gone)| adds.into_iter().skip(gone));
    new.map(|c| {
        let (f, n, t) = (&c.file, c.line, c.text.trim());
        format!(
            "{f}:{n}: `{t}` added to a frozen list: add one file per entry in {instead} instead"
        )
    })
    .collect()
}
