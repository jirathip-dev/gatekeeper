use crate::health::{self, ratchet};
use crate::run::CheckCx as Cx;
use crate::{Result, runner};
use gatekeeper_model::{Ceiling, Lines, TestsConfig};
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

/// The test-to-src line ratio under the lowest ceiling in `ci/tests/ratio.d/`; with a base, no ceiling file is edited, deleted or raised.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let lines = lines(cx.root, cx.tests)?;
    let files = ratchet::read(cx.root)?;
    let mut out = match cx.base {
        Some(base) => changed(cx, base, &files)?,
        None => Vec::new(),
    };
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
