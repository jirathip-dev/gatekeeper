use crate::health::{self, flaky, ratchet};
use crate::run::CheckCx as Cx;
use crate::{Result, runner};
use gatekeeper_model::{Ceiling, Flaky, Lines, TestId, TestsConfig};
use std::path::Path;

const LOWER: &str = "./.gatekeeper/run baseline core/test-health";

pub(crate) fn lines(root: &Path, tests: &TestsConfig) -> Result<Lines> {
    let files = runner::ok("git", &["ls-files", "-z"], root)?;
    Ok(health::totals(root, files.split('\0'), tests))
}

fn ratio(lines: Lines) -> String {
    let r = lines.ratio();
    format!("{}.{:02}", r / 100, r % 100)
}

fn changed(cx: &Cx, base: &str, files: &[(String, Ceiling)]) -> Result<Vec<String>> {
    let args = [
        "diff",
        "--name-status",
        "--no-renames",
        base,
        "HEAD",
        "--",
        ratchet::DIR,
    ];
    let diff = runner::ok("git", &args, cx.root)?;
    let each = diff.lines().filter_map(|l| l.split_once('\t'));
    let (added, touched): (Vec<_>, Vec<_>) = each.partition(|(status, _)| *status == "A");
    let mut out: Vec<String> = touched
        .iter()
        .map(|(_, p)| {
            format!(
                "{p}: a ceiling file is never edited or deleted; lower it with a new file ({LOWER})"
            )
        })
        .collect();
    let new = |p: &str| added.iter().any(|(_, a)| *a == p);
    let base_ceiling = files.iter().filter(|(p, _)| !new(p)).map(|(_, c)| *c).min();
    for (p, c) in files.iter().filter(|(p, _)| new(p)) {
        if let Some(b) = base_ceiling.filter(|b| c >= b) {
            out.push(format!(
                "{p}: ceiling {c} is not under the base's ceiling {b}: a ceiling only falls"
            ));
        }
    }
    Ok(out)
}

fn renewed(
    cx: &Cx,
    base: &str,
    now: &[(String, std::result::Result<Flaky, String>)],
) -> Result<Vec<String>> {
    let args = [
        "diff",
        "--name-status",
        "--no-renames",
        base,
        "HEAD",
        "--",
        flaky::DIR,
    ];
    let diff = runner::ok("git", &args, cx.root)?;
    let gone = diff.lines().filter_map(|l| l.strip_prefix("D\t"));
    let was = |p: &str| -> Option<TestId> {
        let text = runner::ok("git", &["show", &format!("{base}:{p}")], cx.root).ok()?;
        Some(toml::from_str::<Flaky>(&text).ok()?.test)
    };
    let tests: Vec<TestId> = gone.filter_map(was).collect();
    let added = |p: &str| diff.lines().any(|l| l.strip_prefix("A\t") == Some(p));
    let each = now.iter().filter(|(p, _)| added(p));
    let renewed = each.filter_map(|(p, f)| Some((p, &f.as_ref().ok()?.test)));
    let renewed = renewed.filter(|(_, t)| tests.contains(t));
    Ok(renewed
        .map(|(p, t)| format!("{p}: {t} replaces its deleted entry: a quarantine is never renewed"))
        .collect())
}

fn flaky(cx: &Cx) -> Result<Vec<String>> {
    let entries = flaky::read(cx.root);
    let mut out = match cx.base {
        Some(base) => renewed(cx, base, &entries)?,
        None => Vec::new(),
    };
    for (at, entry) in entries {
        let f = match entry {
            Ok(f) => f,
            Err(e) => {
                out.push(format!("{at}: no flaky entry: {e}"));
                continue;
            }
        };
        let hex = f.sha.len() >= 7 && f.sha.len() <= 40;
        if !hex || !f.sha.chars().all(|c| c.is_ascii_hexdigit()) {
            out.push(format!("{at}: sha `{}` is no commit sha", f.sha));
        }
        if f.since > cx.today {
            out.push(format!(
                "{at}: since {} is after today {}",
                f.since, cx.today
            ));
        }
        match f.issue {
            None => out.push(format!(
                "{at}: {} has no issue: add `issue = <n>`, the issue that fixes it",
                f.test
            )),
            Some(n) if f.until() < cx.today => out.push(format!(
                "{at}: quarantine ended {} (#{n}): fix or delete {}, then delete this file",
                f.until().plus(1),
                f.test
            )),
            Some(_) => {}
        }
    }
    Ok(out)
}

/// The test-to-src line ratio under the lowest ceiling in `ci/tests/ratio.d/`, no ceiling file edited, deleted or raised; each flaky entry with its issue, in its 7 days.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let lines = lines(cx.root, cx.tests)?;
    let files = ratchet::read(cx.root)?;
    let mut out = match cx.base {
        Some(base) => changed(cx, base, &files)?,
        None => Vec::new(),
    };
    out.extend(flaky(cx)?);
    let counted = format!(
        "tests {} / src {} lines: ratio {}",
        lines.test,
        lines.src,
        ratio(lines)
    );
    match ratchet::lowest(&files) {
        None => out.push(format!(
            "no ceiling in {}: start it ({LOWER})",
            ratchet::DIR
        )),
        Some((at, c)) if !c.holds(lines) => {
            out.push(format!(
                "ratio {} is over the ceiling {c} ({at})",
                ratio(lines)
            ));
        }
        Some((_, c)) if Ceiling::fits(lines) < *c => out.push(format!(
            "ratio {} is a step under the ceiling {c}: lower it ({LOWER})",
            ratio(lines)
        )),
        Some(_) => {}
    }
    let ceiling = ratchet::lowest(&files).map_or_else(|| "none".to_owned(), |(_, c)| c.to_string());
    cx.note.set(format!("{counted}, ceiling {ceiling}")).ok();
    Ok(out)
}
