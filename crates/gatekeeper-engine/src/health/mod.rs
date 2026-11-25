pub(crate) mod count;
pub(crate) mod entry;
pub mod judge;
pub mod junit;
pub(crate) mod ratchet;
pub mod rerun;
pub mod spans;
pub(crate) mod timed;

use crate::baseline::Baseline;
use gatekeeper_model::{Failure, Lines, TestsConfig};
use std::cell::OnceCell;
use std::fs::File;
use std::path::Path;
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

pub(crate) fn after(
    steps: Result<(), Failure>,
    at: &timed::Run,
    out: (&mut File, &mut Baseline, &OnceCell<String>),
) -> Result<(), Failure> {
    match (&at.gate.timed, steps) {
        (Some(t), steps @ (Ok(()) | Err(Failure::Exit(_)))) => timed::run(t, steps, at, out),
        (_, steps) => steps,
    }
}
