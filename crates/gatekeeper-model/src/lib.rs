mod cache;
mod docs;
mod gate;
mod glob;
mod health;
mod names;
mod ratchet;
mod report;
mod scope;
mod select;

pub use cache::{
    CacheConfig, CacheMode, CacheNote, Event, Hit, Key, KeyParts, Off, ProjectCache, Proof,
    PullRequests, Record, Refusal, RunRef, Tier, Unkeyed,
};
pub use docs::{AGENT_TEXT, DOCS, DocsConfig, Excuses, Names, Prefix, RECORDS, SetName, Source};
pub use gate::{Capability, Gate, GateFile, PackFile, Platform, Step, Toolchain, Upload};
pub use glob::Glob;
pub use health::{
    Case, Ceiling, Changed, Coverage, CoverageTool, Covered, Date, Ended, Flaky, Health, Lines,
    RunKind, Runner, Sighting, Tenths, TestId, TestsConfig, Timed, When,
};
pub use names::PrivateNames;
pub use ratchet::{CountName, Counted, DUPLICATION_IGNORED, DUPLICATION_STEP, DuplicationConfig};
pub use report::{
    BudgetMode, CacheTotals, Failure, GateReport, Host, JobReport, Kept, Millis, Outcome, Report,
    judge,
};
pub use scope::{
    Affected, Graph, GraphKind, Leg, ScopeConfig, Shard, Shards, Trigger, Widened, Workspace,
};
pub use select::{
    Always, Change, ChangeKind, Evidence, Files, Full, Read, Select, Selected, Selection, TestFile,
    Timings, Walk, Why, Widen,
};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A pack of gates: a built-in one, or `local` for the repo's own `.gatekeeper/gates/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackId {
    Core,
    Rust,
    Swift,
    Ts,
    Supabase,
    Local,
}

impl PackId {
    pub const BUILT_IN: [PackId; 5] = [
        PackId::Core,
        PackId::Rust,
        PackId::Swift,
        PackId::Ts,
        PackId::Supabase,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            PackId::Core => "core",
            PackId::Rust => "rust",
            PackId::Swift => "swift",
            PackId::Ts => "ts",
            PackId::Supabase => "supabase",
            PackId::Local => "local",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<PackId> {
        PackId::BUILT_IN
            .into_iter()
            .chain([PackId::Local])
            .find(|p| p.as_str() == s)
    }
}

impl fmt::Display for PackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `pack/name`, the name lowercase letters, digits and dashes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GateId {
    pub pack: PackId,
    name: String,
}

impl GateId {
    #[must_use]
    pub fn new(pack: PackId, name: &str) -> Option<GateId> {
        is_name(name).then(|| GateId {
            pack,
            name: name.to_owned(),
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn file_stem(&self) -> String {
        format!("{}-{}", self.pack, self.name)
    }
}

impl TryFrom<String> for GateId {
    type Error = String;

    fn try_from(s: String) -> Result<GateId, String> {
        s.split_once('/')
            .and_then(|(p, n)| GateId::new(PackId::parse(p)?, n))
            .ok_or_else(|| format!("`{s}` is not a gate id (pack/name)"))
    }
}

impl From<GateId> for String {
    fn from(id: GateId) -> String {
        id.to_string()
    }
}

impl fmt::Display for GateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&format!("{}/{}", self.pack, self.name))
    }
}

/// A gate's or an upload's name: lowercase letters, digits and dashes.
#[must_use]
pub fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Whole seconds: a budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secs(pub u32);

/// `.gatekeeper/config.toml`, parsed once at the edge; an unknown key is an error.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub packs: BTreeSet<PackId>,
    pub integration: String,
    pub pin: Pin,
    #[serde(default)]
    pub budgets: BTreeMap<GateId, Secs>,
    #[serde(default)]
    pub off: BTreeMap<GateId, String>,
    #[serde(default)]
    pub rust: RustPack,
    #[serde(default)]
    pub scope: Option<ScopeConfig>,
    #[serde(default)]
    pub legs: BTreeMap<String, Leg>,
    pub ts: Option<TsPack>,
    #[serde(default)]
    pub cache: Option<CacheConfig>,
    pub swift: Option<SwiftPack>,
    #[serde(default)]
    pub tools: BTreeMap<String, String>,
    #[serde(default)]
    pub docs: DocsConfig,
    #[serde(default)]
    pub tests: TestsConfig,
    #[serde(default)]
    pub duplication: DuplicationConfig,
}

/// Which gatekeeper runs: this checkout's build, or a release by version and per-target sha256.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Pin {
    Checkout(CheckoutWord),
    Release(ReleasePin),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutWord {
    Checkout,
}

#[derive(Debug, Deserialize)]
pub struct ReleasePin {
    pub version: String,
    #[serde(flatten)]
    pub sha256: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RustPack {
    pub manifest: String,
    pub unsafe_crates: Vec<String>,
    pub exported: Excuses,
}

impl Default for RustPack {
    fn default() -> Self {
        RustPack {
            manifest: "Cargo.toml".into(),
            unsafe_crates: Vec::new(),
            exported: Excuses::default(),
        }
    }
}

/// `[ts]`: the folders `tsc -p` checks and oxlint lints, and the command that installs their dependencies.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TsPack {
    pub projects: Vec<String>,
    pub install: String,
}

/// `[swift]`: the packages and Xcode projects that build strict, the folders anti-slop lints and the token lint reads, the extra rules on.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwiftPack {
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub projects: Vec<XcodeProject>,
    pub sources: Vec<String>,
    #[serde(default)]
    pub tokens: Vec<String>,
    #[serde(default)]
    pub extra: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XcodeProject {
    pub path: String,
    pub scheme: String,
    #[serde(default)]
    pub destination: Option<String>,
}

/// What `check` and `list` narrow the configured gates to.
#[derive(Debug, Default)]
pub struct Filter {
    pub packs: BTreeSet<PackId>,
    pub gate: Option<GateId>,
    pub leg: Option<String>,
    pub shard: Option<Shard>,
    pub trigger: Trigger,
    pub repo: RepoRun,
    pub when: When,
}

/// Whose run this is, for a gate that needs the repo's secrets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RepoRun {
    #[default]
    Own,
    Private,
    Fork,
}

#[derive(Debug, Serialize)]
pub struct Plan {
    /// `None` when the config has no `[scope]`.
    pub affected: Option<Affected>,
    #[serde(skip)]
    pub workspace: Workspace,
    #[serde(skip)]
    pub trigger: Trigger,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Serialize)]
pub struct Job {
    pub leg: String,
    pub shard: Shard,
    pub after: Option<String>,
    pub platform: Platform,
    pub setup: BTreeSet<Toolchain>,
    pub gates: Vec<Planned>,
}

/// `leg` or `leg i/n`: a job's name in a table.
#[must_use]
pub fn job_label(leg: &str, shard: Shard) -> String {
    if shard.of == Shard::ONE.of {
        leg.to_owned()
    } else {
        format!("{leg} {shard}")
    }
}

/// `gates-leg` or `gates-leg-i`: the artifact a job uploads.
#[must_use]
pub fn artifact(leg: &str, shard: Shard) -> String {
    if shard.of == Shard::ONE.of {
        format!("gates-{leg}")
    } else {
        format!("gates-{leg}-{}", shard.index)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Planned {
    pub gate: Gate,
    pub decision: Decision,
    /// The affected project roots the gate's `projects` match: its `{projects}`.
    pub projects: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Decision {
    Run,
    Skip(Skip),
    Off(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Skip {
    OutOfScope,
    NoAffectedProject,
    AfterFailed,
    CiOnly(Capability),
    PrivateRepo,
    ForkPullRequest,
    EmptyShard,
    Nightly,
    NotNightly,
}

impl fmt::Display for Skip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Skip::OutOfScope => f.write_str("no changed file matches its paths"),
            Skip::NoAffectedProject => f.write_str("no affected project matches its projects"),
            Skip::AfterFailed => f.write_str("the leg it runs after failed or left no report"),
            Skip::CiOnly(c) => write!(f, "CI only on a lane's host: needs {c}"),
            Skip::PrivateRepo => f.write_str("a private repo has no public text to guard"),
            Skip::ForkPullRequest => {
                f.write_str("a fork's pull request has no secrets; it runs on the merge push")
            }
            Skip::EmptyShard => f.write_str("no selected test file fell in this shard"),
            Skip::Nightly => {
                f.write_str("a nightly gate: it runs on the workflow's schedule event")
            }
            Skip::NotNightly => f.write_str("a scheduled run runs nightly gates only"),
        }
    }
}
