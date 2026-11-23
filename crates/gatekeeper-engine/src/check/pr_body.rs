use crate::run::CheckCx as Cx;
use crate::{Error, Result, runner};
use std::fs;

fn merge_of(subject: &str, integration: &str) -> bool {
    let s = subject
        .strip_prefix("Merge ")
        .map(|s| s.strip_prefix("remote-tracking ").unwrap_or(s));
    let s = s.map(|s| s.strip_prefix("branch ").unwrap_or(s));
    let s = s.map(|s| s.strip_prefix('\'').unwrap_or(s));
    let s = s.and_then(|s| {
        s.strip_prefix("origin/")
            .unwrap_or(s)
            .strip_prefix(integration)
    });
    s.is_some_and(|t| {
        matches!(
            t.strip_prefix('\'').unwrap_or(t).chars().next(),
            None | Some(' ')
        )
    })
}

fn names(low: &str, sha: &str) -> bool {
    let c = sha.get(..7).unwrap_or(sha).to_ascii_lowercase();
    (low.match_indices(&c)).any(|(i, _)| !low[..i].ends_with(|ch: char| ch.is_ascii_hexdigit()))
}

fn named_head(cx: &Cx, body: &str, head: &str) -> Option<String> {
    let git = |a: &[&str]| {
        runner::ok("git", a, cx.root)
            .ok()
            .map(|s| s.trim().to_owned())
    };
    let low = body.to_ascii_lowercase();
    let mut c = head.to_owned();
    while !names(&low, &c) {
        let subject = git(&["log", "-1", "--format=%s", &c]);
        let parent = subject
            .filter(|s| merge_of(s, cx.integration))
            .and_then(|_| git(&["rev-parse", &format!("{c}^1")]));
        let Some(p) = parent else {
            return Some(format!(
                "the PR body's evidence names no sha of head {head:.7}: re-run the gates at the head and `gh pr edit --body` with it"
            ));
        };
        c = p;
    }
    None
}

fn fault(cx: &Cx, body: &str, head: &str) -> Option<String> {
    let lines = || body.lines().map(str::trim);
    if !lines().any(|l| l.eq_ignore_ascii_case("## For humans")) {
        return Some(
            "the PR body has no `## For humans` section: add 2-4 plain lines, `gh pr edit --body`"
                .into(),
        );
    }
    let generated = |l: &&str| {
        l.trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
            .starts_with("Generated with")
    };
    if let Some(l) = lines().find(generated) {
        return Some(format!(
            "the PR body carries `{l}`: drop it, `gh pr edit --body`"
        ));
    }
    named_head(cx, body, head)
}

/// The PR body names the head or a commit under its integration merges, has `## For humans`, has no "Generated with" line.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let Some(file) = cx.pr_body else {
        cx.note.set("no PR body: nothing to check".into()).ok();
        return Ok(Vec::new());
    };
    let body = fs::read_to_string(file).map_err(|e| Error::at(file, e))?;
    let head = match cx.head.filter(|h| !h.is_empty()) {
        Some(h) => h.to_owned(),
        None => runner::ok("git", &["rev-parse", "HEAD"], cx.root)?
            .trim()
            .to_owned(),
    };
    if head.len() < 7 || !head.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error(format!(
            "the head {head:?} is no commit sha: give at least 7 hex digits"
        )));
    }
    Ok(fault(cx, &body, &head)
        .map(|f| format!("PR_BODY: {f}"))
        .into_iter()
        .collect())
}
