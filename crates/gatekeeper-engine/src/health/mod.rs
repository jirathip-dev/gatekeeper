pub(crate) mod count;
pub(crate) mod coverage;
pub(crate) mod entry;
pub mod flaky;
pub mod judge;
pub mod junit;
pub(crate) mod ratchet;
pub mod rerun;
pub mod spans;
pub(crate) mod timed;

use crate::baseline::Baseline;
use crate::run::RunCtx;
use gatekeeper_model::{Failure, Gate, Lines, Sighting, Tenths, TestsConfig};
use std::cell::OnceCell;
use std::fs::File;
use std::io::Write as _;
use std::path::Path;

pub(crate) fn coverage(
    gate: &Gate,
    cx: &RunCtx,
    log: &mut File,
    (note, covered): (&OnceCell<String>, &mut Option<Tenths>),
) -> Result<(), Failure> {
    let Some(cov) = &gate.coverage else {
        return Ok(());
    };
    let judged = coverage::judge(&gate.id, cov, cx).map_err(|e| {
        writeln!(log, "{e}").ok();
        Failure::Exit(1)
    })?;
    *covered = Some(judged.lines);
    writeln!(log, "{}", judged.note).ok();
    note.set(judged.note).ok();
    for line in &judged.findings {
        writeln!(log, "{line}").ok();
    }
    if judged.findings.is_empty() {
        Ok(())
    } else {
        Err(Failure::Exit(1))
    }
}

pub(crate) fn totals<'a>(
    root: &Path,
    files: impl Iterator<Item = &'a str>,
    tests: &TestsConfig,
) -> Lines {
    let counted = files.filter_map(|p| Some((p, count::side(p, tests)?)));
    let each = counted.map(|(p, side)| {
        let text = std::fs::read_to_string(root.join(p)).unwrap_or_default();
        count::lines(side, &text)
    });
    each.fold(Lines::default(), |sum, l| sum + l)
}

pub(crate) type Out<'a> = (
    &'a mut File,
    &'a mut Baseline,
    &'a OnceCell<String>,
    &'a OnceCell<Vec<Sighting>>,
);

pub(crate) fn after(steps: Result<(), Failure>, at: &timed::Run, out: Out) -> Result<(), Failure> {
    match (&at.gate.timed, steps) {
        (Some(t), steps @ (Ok(()) | Err(Failure::Exit(_)))) => timed::run(t, steps, at, out),
        (_, steps) => steps,
    }
}
