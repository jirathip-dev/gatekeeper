use crate::{Error, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

/// cargo-machete's text report: one `<manifest>: unused dependency: <name>` line per name it lists.
#[must_use]
pub fn machete(stdout: &str) -> Vec<String> {
    let mut manifest = None;
    let mut out = Vec::new();
    for line in stdout.lines() {
        match (line.strip_prefix('\t'), manifest) {
            (Some(name), Some(m)) => out.push(format!("{m}: unused dependency: {}", name.trim())),
            (Some(_), None) => {}
            (None, _) => {
                let m = line
                    .split_once(" -- ")
                    .and_then(|(_, m)| m.strip_suffix(':'));
                manifest = m.map(|m| m.trim_start_matches("./"));
            }
        }
    }
    out
}

/// What one jscpd run found: its duplicated lines, each by format, and its clones.
#[derive(Debug, Default)]
pub struct Duplication {
    pub duplicated_lines: u32,
    pub by_format: BTreeMap<String, u32>,
    pub clones: Vec<String>,
}

impl Duplication {
    /// `rust 120, typescript 8; 5 clones`.
    #[must_use]
    pub fn summary(&self) -> String {
        let each: Vec<String> = (self.by_format.iter())
            .map(|(f, n)| format!("{f} {n}"))
            .collect();
        format!("{}; {} clones", each.join(", "), self.clones.len())
    }
}

#[derive(Deserialize)]
struct Report {
    statistics: Statistics,
    duplicates: Vec<Pair>,
}

#[derive(Deserialize)]
struct Statistics {
    total: Total,
    formats: BTreeMap<String, Total>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Total {
    duplicated_lines: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pair {
    first_file: Span,
    second_file: Span,
    lines: u32,
}

#[derive(Deserialize)]
struct Span {
    name: String,
    start: u32,
    end: u32,
}

/// # Errors
pub fn jscpd(report: &str) -> Result<Duplication> {
    let r: Report =
        serde_json::from_str(report).map_err(|e| Error(format!("jscpd report: {e}")))?;
    let at = |s: &Span| format!("{}:{}-{}", s.name, s.start, s.end);
    let mut clones: Vec<String> = (r.duplicates.iter())
        .map(|p| {
            format!(
                "{} ~ {} ({} lines)",
                at(&p.first_file),
                at(&p.second_file),
                p.lines
            )
        })
        .collect();
    clones.sort();
    Ok(Duplication {
        duplicated_lines: r.statistics.total.duplicated_lines,
        by_format: (r.statistics.formats.into_iter())
            .map(|(f, s)| (f, s.duplicated_lines))
            .collect(),
        clones,
    })
}
