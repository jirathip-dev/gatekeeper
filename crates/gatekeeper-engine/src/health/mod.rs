pub(crate) mod count;
pub(crate) mod entry;
pub(crate) mod ratchet;

use gatekeeper_model::{Lines, TestsConfig};
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
