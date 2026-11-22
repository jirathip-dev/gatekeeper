use crate::{Change, Glob, Host, Platform, Toolchain};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::NonZeroU32;

/// `[scope]`: the workspace graphs to read, and the files outside every project that widen nothing.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeConfig {
    pub graphs: BTreeSet<GraphKind>,
    #[serde(default)]
    pub ignore: Vec<Glob>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphKind {
    Pnpm,
    Cargo,
    SwiftPm,
}

/// `[legs.<name>]`: an extra CI job; `after` names the one leg it waits for and whose kept files it receives.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Leg {
    pub platform: Platform,
    #[serde(default)]
    pub setup: BTreeSet<Toolchain>,
    #[serde(default)]
    pub after: Option<String>,
}

/// The workspace's projects by root folder (`.` the repo root), each with the roots it needs.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct Graph {
    pub projects: BTreeMap<String, BTreeSet<String>>,
}

impl Graph {
    /// The project holding `file`: the deepest root it lies under.
    #[must_use]
    pub fn owner(&self, file: &str) -> Option<&String> {
        self.projects
            .keys()
            .filter(|root| root.as_str() == "." || file.starts_with(&format!("{root}/")))
            .max_by_key(|root| if root.as_str() == "." { 0 } else { root.len() })
    }

    /// `roots` and every project they need, directly or through another.
    #[must_use]
    pub fn reach(&self, roots: &[String]) -> BTreeSet<String> {
        let mut reach: BTreeSet<String> = roots.iter().cloned().collect();
        let mut todo: Vec<String> = roots.to_vec();
        while let Some(root) = todo.pop() {
            for need in self.projects.get(&root).into_iter().flatten() {
                if reach.insert(need.clone()) {
                    todo.push(need.clone());
                }
            }
        }
        reach
    }
}

/// The graph and `[scope] ignore` a plan was cut with: what a `projects` gate's record reads.
#[derive(Clone, Debug, Default)]
pub struct Workspace {
    pub graph: Graph,
    pub ignore: Vec<Glob>,
}

impl Workspace {
    /// Whether a project set's result reads `file`: a file of one it reaches, or one outside every project not ignored.
    #[must_use]
    pub fn reads(&self, reach: &BTreeSet<String>, file: &str) -> bool {
        if file.starts_with(".gatekeeper/") {
            return true;
        }
        match self.graph.owner(file) {
            Some(root) => reach.contains(root),
            None => !self.ignore.iter().any(|g| g.matches(file)),
        }
    }
}

/// What a run diffs against: the files a change touched, or everything and why.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    Change(BTreeSet<Change>),
    Everything(Widened),
}

impl Default for Trigger {
    fn default() -> Trigger {
        Trigger::Everything(Widened::Forced)
    }
}

/// The projects a change can reach, or every project and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Affected {
    All(Widened),
    Projects(BTreeSet<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Widened {
    NotAPullRequest,
    Forced,
    NoBase,
    OutsideEveryProject(String),
}

impl fmt::Display for Affected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Affected::Projects(p) => {
                let list: Vec<&str> = p.iter().map(String::as_str).collect();
                write!(f, "{} affected projects: {}", p.len(), list.join(" "))
            }
            Affected::All(Widened::NotAPullRequest) => f.write_str("every project: not a change"),
            Affected::All(Widened::Forced) => f.write_str("every project: --all or scope full"),
            Affected::All(Widened::NoBase) => f.write_str("every project: no base to diff"),
            Affected::All(Widened::OutsideEveryProject(file)) => {
                write!(f, "every project: `{file}` is outside every project")
            }
        }
    }
}

/// `shards = { hosted = N, self-hosted = M }`: the CI jobs a gate splits into; a lane runs it as one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Shards {
    pub hosted: NonZeroU32,
    pub self_hosted: NonZeroU32,
}

impl Default for Shards {
    fn default() -> Shards {
        Shards {
            hosted: NonZeroU32::MIN,
            self_hosted: NonZeroU32::MIN,
        }
    }
}

impl Shards {
    #[must_use]
    pub fn on(self, host: Host) -> NonZeroU32 {
        match host {
            Host::GithubHosted { .. } => self.hosted,
            Host::SelfHosted => self.self_hosted,
            Host::Lane => NonZeroU32::MIN,
        }
    }
}

/// `i/n`: one of a job's `n` shards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Shard {
    pub index: NonZeroU32,
    pub of: NonZeroU32,
}

impl Shard {
    pub const ONE: Shard = Shard {
        index: NonZeroU32::MIN,
        of: NonZeroU32::MIN,
    };
}

impl TryFrom<String> for Shard {
    type Error = String;

    fn try_from(s: String) -> Result<Shard, String> {
        s.split_once('/')
            .and_then(|(i, n)| Some((i.parse().ok()?, n.parse().ok()?)))
            .filter(|(index, of)| index <= of)
            .map(|(index, of)| Shard { index, of })
            .ok_or_else(|| format!("`{s}` is not a shard (i/n, 1 <= i <= n)"))
    }
}

impl From<Shard> for String {
    fn from(s: Shard) -> String {
        s.to_string()
    }
}

impl fmt::Display for Shard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.index, self.of)
    }
}
