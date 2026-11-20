use crate::{GateId, Glob, Secs, Shards};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// `packs/<pack>/pack.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackFile {
    pub about: String,
    pub platform: Platform,
    #[serde(default)]
    pub setup: BTreeSet<Toolchain>,
}

/// A gate file: `packs/<pack>/gates/<name>.toml` or `.gatekeeper/gates/<name>.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateFile {
    pub about: String,
    pub fix: String,
    pub budget_secs: Secs,
    #[serde(default)]
    pub needs: BTreeSet<Capability>,
    #[serde(default)]
    pub setup: BTreeSet<Toolchain>,
    #[serde(default)]
    pub run: Option<String>,
    #[serde(default)]
    pub step: Vec<Step>,
    #[serde(default)]
    pub paths: Vec<Glob>,
    #[serde(default)]
    pub projects: Vec<Glob>,
    #[serde(default)]
    pub leg: Option<String>,
    #[serde(default)]
    pub shards: Shards,
    #[serde(default)]
    pub upload: Vec<Upload>,
    #[serde(default)]
    pub rule: Vec<String>,
}

/// Files a gate keeps after its last step, pass or fail: `[[upload]]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upload {
    pub name: String,
    pub paths: Vec<Glob>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gate {
    pub id: GateId,
    pub about: String,
    pub fix: String,
    pub budget: Secs,
    pub platform: Platform,
    pub setup: BTreeSet<Toolchain>,
    pub needs: BTreeSet<Capability>,
    pub steps: Vec<Step>,
    pub paths: Vec<Glob>,
    pub projects: Vec<Glob>,
    pub leg: String,
    pub shards: Shards,
    pub uploads: Vec<Upload>,
    pub rule: Vec<String>,
}

impl Gate {
    /// # Errors
    pub fn from_file(id: GateId, file: GateFile, pack: &PackFile) -> Result<Gate, &'static str> {
        let steps = match (file.run, file.step) {
            (Some(run), s) if s.is_empty() => vec![Step::Run { run }],
            (None, s) if !s.is_empty() => s,
            _ => return Err("give `run` or `[[step]]`, not both"),
        };
        if !file.paths.is_empty() && !file.projects.is_empty() {
            return Err("give `paths` or `projects`, not both");
        }
        let leg = file.leg.unwrap_or_else(|| id.pack.to_string());
        Ok(Gate {
            leg,
            id,
            about: file.about,
            fix: file.fix,
            budget: file.budget_secs,
            platform: pack.platform,
            setup: pack.setup.union(&file.setup).copied().collect(),
            needs: file.needs,
            steps,
            paths: file.paths,
            projects: file.projects,
            shards: file.shards,
            uploads: file.upload,
            rule: file.rule,
        })
    }
}

/// `run` is the repo's toolchain through `sh -c`; `tool` a pinned tool; `check` a built-in check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Step {
    Run {
        run: String,
    },
    Tool {
        tool: String,
        #[serde(default)]
        args: Vec<String>,
    },
    Check {
        check: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Any,
    Linux,
    Macos,
}

impl Platform {
    #[must_use]
    pub fn runner(self) -> &'static str {
        match self {
            Platform::Any | Platform::Linux => "linux",
            Platform::Macos => "macos",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Toolchain {
    Rust,
    Node,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Containers,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::Containers => f.write_str("containers"),
        }
    }
}
