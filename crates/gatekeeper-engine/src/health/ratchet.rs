use crate::Result;
use crate::baseline::Written;
use crate::ratchet::folder;
use gatekeeper_model::{Ceiling, Date, Lines};
use std::path::Path;

pub(crate) const DIR: &str = "ci/tests/ratio.d";

pub(crate) fn read(root: &Path) -> Result<Vec<(String, Ceiling)>> {
    folder::read(root, DIR)
}

pub(crate) fn write(root: &Path, today: Date, lines: Lines) -> Result<Written> {
    let fits = Ceiling::fits(lines);
    folder::write(root, (DIR, "the ratio"), today, (fits, fits.hundredths()))
}
