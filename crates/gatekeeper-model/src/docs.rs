use crate::Glob;
use serde::Deserialize;
use std::collections::BTreeMap;

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
