use super::oxlint::report;
use crate::Result;
use crate::ratchet::reports;
use crate::run::CheckCx as Cx;

/// Pinned oxlint with the pack's complexity config alone; each function over a threshold is a finding.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let report = report(cx, "ts/files/complexity/", &["--disable-nested-config"])?;
    Ok(reports::oxlint(&report))
}
