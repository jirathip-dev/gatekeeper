use crate::Glob;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

pub const DOCS: [&str; 6] = [
    "README.md",
    "docs/**/*.md",
    "skill/**/*.md",
    "CLAUDE.md",
    "AGENTS.md",
    ".drover/*.md",
];
pub const RECORDS: [&str; 2] = ["docs/architecture/**", "docs/design/**"];
pub const AGENT_TEXT: [&str; 1] = ["skill/**/*.md"];

/// `[docs]` in `.gatekeeper/config.toml`: every key optional, the gate on without the table.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DocsConfig {
    pub also: Vec<Glob>,
    pub skip: Vec<Glob>,
    pub watch: Vec<Glob>,
    pub records: Excuses,
    pub names: Option<Names>,
}

/// `glob = "reason"` lines: what is excused, each with a reason that is not empty.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(try_from = "BTreeMap<String, String>")]
pub struct Excuses(pub Vec<Glob>);

impl TryFrom<BTreeMap<String, String>> for Excuses {
    type Error = String;

    fn try_from(m: BTreeMap<String, String>) -> Result<Excuses, String> {
        let globs = m.into_iter().map(|(g, why)| {
            if why.trim().is_empty() {
                return Err(format!("`{g}` needs a reason"));
            }
            Glob::try_from(g)
        });
        globs.collect::<Result<_, String>>().map(Excuses)
    }
}

/// `[docs.names]`: where the real names come from and how each set's names look in a doc.
#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "RawNames")]
pub struct Names {
    pub source: Source,
    pub sets: BTreeMap<SetName, Vec<Prefix>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNames {
    run: Option<String>,
    file: Option<String>,
    #[serde(default)]
    sets: BTreeMap<SetName, Vec<Prefix>>,
}

impl TryFrom<RawNames> for Names {
    type Error = String;

    fn try_from(r: RawNames) -> Result<Names, String> {
        let source = match (r.run, r.file) {
            (Some(c), None) => Source::Run(c),
            (None, Some(f)) => Source::File(f),
            _ => return Err("[docs.names] needs exactly one of `run` and `file`".into()),
        };
        Ok(Names {
            source,
            sets: r.sets,
        })
    }
}

/// `run = "…"` or `file = "…"`: exactly one.
#[derive(Clone, Debug)]
pub enum Source {
    Run(String),
    File(String),
}

/// A set's name: lowercase letters, digits and dashes, as a gate's.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct SetName(String);

impl TryFrom<String> for SetName {
    type Error = String;

    fn try_from(s: String) -> Result<SetName, String> {
        let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
        if s.is_empty() || !s.bytes().all(ok) {
            return Err(format!(
                "set `{s}`: lowercase letters, digits and dashes only"
            ));
        }
        Ok(SetName(s))
    }
}

impl fmt::Display for SetName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `<text>{name}`: the text is not empty and `{name}` ends it.
#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct Prefix(String);

impl TryFrom<String> for Prefix {
    type Error = String;

    fn try_from(s: String) -> Result<Prefix, String> {
        match s.strip_suffix("{name}") {
            Some(t) if !t.is_empty() && !t.contains("{name}") => Ok(Prefix(t.to_owned())),
            _ => Err(format!("prefix `{s}`: text, then `{{name}}` at the end")),
        }
    }
}

impl Prefix {
    /// The word a span claims: `Some` when the span starts with the prefix and a word follows it.
    #[must_use]
    pub fn claim<'a>(&self, span: &'a str) -> Option<&'a str> {
        let rest = span.strip_prefix(&self.0)?;
        if !rest.starts_with(|c: char| c.is_alphanumeric()) {
            return None;
        }
        let end = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(rest.len());
        Some(&rest[..end])
    }
}
