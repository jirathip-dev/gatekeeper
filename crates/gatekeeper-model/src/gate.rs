use crate::{Coverage, GateId, Glob, Secs, Select, Shards, Timed};
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
    pub platform: Option<Platform>,
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
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default = "yes")]
    pub cache: bool,
    #[serde(default)]
    pub select: Option<Select>,
    #[serde(default)]
    pub coverage: Option<Coverage>,
    pub timed: Option<Timed>,
}

fn yes() -> bool {
    true
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
    pub versions: Vec<String>,
    pub cache: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Select>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<Coverage>,
    pub timed: Option<Timed>,
}

impl Gate {
    /// Whether a change to `file` can change this gate's result: path scope and the cache's inputs alike.
    #[must_use]
    pub fn reads(&self, file: &str) -> bool {
        self.paths.is_empty()
            || file.starts_with(".gatekeeper/")
            || self.paths.iter().any(|g| g.matches(file))
    }

    /// # Errors
    pub fn from_file(id: GateId, file: GateFile, pack: &PackFile) -> Result<Gate, &'static str> {
        if file.select.is_some() {
            select_refusal(&file)?;
        }
        if file.coverage.is_some() && file.shards != Shards::default() {
            return Err("a [coverage] gate runs as one job: give it no `shards`");
        }
        if file.coverage.is_some() && !file.needs.contains(&Capability::Coverage) {
            return Err("a [coverage] gate needs `needs = [\"coverage\"]`: it runs in CI only");
        }
        if file.timed.is_some() {
            timed_refusal(&file)?;
        }
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
            platform: file.platform.unwrap_or(pack.platform),
            setup: pack.setup.union(&file.setup).copied().collect(),
            needs: file.needs,
            steps,
            paths: file.paths,
            projects: file.projects,
            shards: file.shards,
            uploads: file.upload,
            rule: file.rule,
            versions: file.versions,
            cache: file.cache && file.coverage.is_none(),
            select: file.select,
            coverage: file.coverage,
            timed: file.timed,
        })
    }
}

fn select_refusal(file: &GateFile) -> Result<(), &'static str> {
    let Some(run) = file.run.as_deref().filter(|_| file.step.is_empty()) else {
        return Err("[select] runs one `run` line, not `[[step]]`");
    };
    if !file.projects.is_empty() {
        return Err("give [select] or `projects`, not both");
    }
    if !run.contains("{files}") {
        return Err("[select] needs `{files}` in `run`: the selected test files go there");
    }
    let sharded = file.shards.hosted.get() > 1 || file.shards.self_hosted.get() > 1;
    if sharded && !run.contains("{report}") {
        return Err("a sharded [select] gate needs `{report}` in `run`: shards are split by time");
    }
    Ok(())
}

fn timed_refusal(file: &GateFile) -> Result<(), &'static str> {
    let Some(run) = file.run.as_deref().filter(|_| file.step.is_empty()) else {
        return Err("[timed] runs one `run` line, not `[[step]]`");
    };
    if file.select.is_none() && !run.contains("{only}") {
        return Err("[timed] needs `{only}` in `run`: the tests to run again go there");
    }
    Ok(())
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

impl Toolchain {
    /// The command whose output names this toolchain's version in a result's key.
    #[must_use]
    pub fn probe(self) -> &'static str {
        match self {
            Toolchain::Rust => "rustc -Vv",
            Toolchain::Node => "node --version",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Containers,
    PrivateNames,
    Coverage,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::Containers => f.write_str("containers"),
            Capability::PrivateNames => f.write_str("the private-names list"),
            Capability::Coverage => f.write_str("the integration branch's coverage"),
        }
    }
}
