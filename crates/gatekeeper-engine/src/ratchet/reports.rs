use crate::{Error, Result};
use serde::Deserialize;
use serde_json::Value;
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

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Unit {
    file: String,
    at: u64,
    name: String,
    over: String,
}

fn numbered(mut units: Vec<Unit>) -> Vec<String> {
    units.sort();
    units.dedup();
    let mut seen: BTreeMap<String, u32> = BTreeMap::new();
    let line = |u: Unit| {
        let n = seen
            .entry(format!("{}: {}: {}", u.file, u.name, u.over))
            .or_default();
        *n += 1;
        let name = match *n {
            1 => u.name,
            n => format!("{} #{n}", u.name),
        };
        format!("{}: {name}: {}", u.file, u.over)
    };
    units.into_iter().map(line).collect()
}

fn clippy_unit(line: &str) -> Option<Unit> {
    let m: Value = serde_json::from_str(line).ok()?;
    if m.get("reason")?.as_str()? != "compiler-message" {
        return None;
    }
    let msg = m.get("message")?;
    let (_, max) = msg.get("message")?.as_str()?.rsplit_once('/')?;
    let max = max.trim_end_matches(')');
    let over = match msg.pointer("/code/code")?.as_str()? {
        "clippy::too_many_lines" => format!("over {max} lines"),
        "clippy::cognitive_complexity" => format!("cognitive complexity over {max}"),
        _ => return None,
    };
    let span = msg.pointer("/spans/0")?;
    let text = span.pointer("/text/0/text")?.as_str()?;
    let name = text.split_once("fn ").and_then(|(_, rest)| {
        rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next()
    });
    Some(Unit {
        file: span.get("file_name")?.as_str()?.to_owned(),
        at: span.get("line_start")?.as_u64()?,
        name: name.map_or_else(|| "fn".to_owned(), |n| format!("fn {n}")),
        over,
    })
}

/// clippy's JSON messages: one `<file>: fn <name>: over <n> lines` or `…: cognitive complexity over <n>` line per function over a threshold.
#[must_use]
pub fn clippy(json_lines: &str) -> Vec<String> {
    numbered(json_lines.lines().filter_map(clippy_unit).collect())
}

fn oxlint_unit(d: &Value) -> Option<Unit> {
    let message = d.get("message")?.as_str()?;
    let (_, max) = message.split_once("Maximum allowed is ")?;
    let max = max.trim_end_matches('.');
    let over = match d.get("code")?.as_str()? {
        "eslint(complexity)" => format!("complexity over {max}"),
        "eslint(max-lines-per-function)" => format!("over {max} lines"),
        _ => return None,
    };
    let name = message.split('`').nth(1);
    Some(Unit {
        file: d.get("filename")?.as_str()?.to_owned(),
        at: d.pointer("/labels/0/span/offset")?.as_u64()?,
        name: name.map_or_else(|| "function".to_owned(), |n| format!("function {n}")),
        over,
    })
}

/// oxlint's JSON report: one `<file>: function <name>: complexity over <n>` or `…: over <n> lines` line per function over a threshold; an unnamed one is `function`, the second in a file `function #2`.
#[must_use]
pub fn oxlint(report: &Value) -> Vec<String> {
    let diagnostics = report.get("diagnostics").and_then(Value::as_array);
    numbered(
        diagnostics
            .into_iter()
            .flatten()
            .filter_map(oxlint_unit)
            .collect(),
    )
}
