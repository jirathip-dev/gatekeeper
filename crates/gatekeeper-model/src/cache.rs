use crate::{Gate, GateId, Millis, Shard, Step};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU32;

/// `[cache]` in `.gatekeeper/config.toml`; the table's presence turns the result cache on.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheConfig {
    #[serde(default = "ten")]
    pub recheck: NonZeroU32,
    #[serde(default = "fourteen")]
    pub max_age_days: NonZeroU32,
    #[serde(default)]
    pub pull_requests: PullRequests,
}

fn ten() -> NonZeroU32 {
    NonZeroU32::MIN.saturating_add(9)
}

fn fourteen() -> NonZeroU32 {
    NonZeroU32::MIN.saturating_add(13)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PullRequests {
    #[default]
    Own,
    Read,
}

/// The hex sha256 of a record's `KeyParts`: its file name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Key(pub String);

/// Everything a gate's result depends on that gatekeeper can see.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyParts {
    pub schema: u32,
    pub engine: Engine,
    pub gate: GateId,
    pub gate_file: String,
    pub shard: Shard,
    pub projects: Vec<String>,
    pub platform: PlatformId,
    pub versions: BTreeMap<String, String>,
    pub tools: BTreeMap<String, String>,
    pub inputs: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Engine {
    Release { version: String, build: String },
    Checkout { binary: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformId {
    pub target: String,
    pub os: String,
}

/// Why a gate has no key and always runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Unkeyed {
    OptedOut,
    ReadsBase,
    ReadsPullRequest,
    KeepsFiles,
    NoVersion,
}

impl Unkeyed {
    #[must_use]
    pub fn of(gate: &Gate) -> Option<Unkeyed> {
        let arg = |a: &str| {
            gate.steps
                .iter()
                .any(|s| matches!(s, Step::Tool { args, .. } if args.iter().any(|x| x == a)))
        };
        let bare = |s: &Step| matches!(s, Step::Run { .. });
        if !gate.cache {
            Some(Unkeyed::OptedOut)
        } else if arg("{--base}") {
            Some(Unkeyed::ReadsBase)
        } else if arg("{--pr-body}") {
            Some(Unkeyed::ReadsPullRequest)
        } else if !gate.uploads.is_empty() {
            Some(Unkeyed::KeepsFiles)
        } else if gate.setup.is_empty() && gate.versions.is_empty() && gate.steps.iter().any(bare) {
            Some(Unkeyed::NoVersion)
        } else {
            None
        }
    }
}

impl fmt::Display for Unkeyed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Unkeyed::OptedOut => "cache = false",
            Unkeyed::ReadsBase => "reads the base",
            Unkeyed::ReadsPullRequest => "reads the PR body",
            Unkeyed::KeepsFiles => "keeps files",
            Unkeyed::NoVersion => "a `run` step with no setup or versions",
        })
    }
}

/// A folder of records: who may write it is decided by `mode`, never by the record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    Integration,
    PullRequest(u64),
    Host,
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tier::Integration => f.write_str("integration"),
            Tier::PullRequest(n) => write!(f, "pr-{n}"),
            Tier::Host => f.write_str("host"),
        }
    }
}

/// `<store>/<tier>/<key>.json`: a pass that really ran; there is no outcome field.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub key: Key,
    pub parts: KeyParts,
    pub proof: Proof,
    pub took: Millis,
}

/// The run that proved a pass: `commit` is `None` for a dirty tree, `run` on a lane's host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof {
    pub tier: Tier,
    pub at: u64,
    pub commit: Option<String>,
    pub run: Option<RunRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRef {
    pub repo: String,
    pub run: u64,
    pub attempt: u32,
}

fn utc(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or_default() + 719_468;
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (day, month) = (doy - (153 * mp + 2) / 5 + 1, (mp + 2) % 12 + 1);
    let (h, m) = (secs % 86_400 / 3600, secs % 3600 / 60);
    format!("{month:02}-{day:02} {h:02}:{m:02}")
}

impl fmt::Display for Proof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let commit = self.commit.as_deref().map(|c| c.get(..7).unwrap_or(c));
        match &self.run {
            None => write!(
                f,
                "lane run {} at {}",
                utc(self.at),
                commit.unwrap_or("a dirty tree")
            ),
            Some(r) => write!(
                f,
                "run {} · {} · {}",
                r.run,
                self.tier,
                commit.unwrap_or("-")
            ),
        }
    }
}

/// Which CI or lane run this is, read once at the edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Lane,
    PullRequest { number: u64, fork: bool },
    Push { branch: String },
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheMode {
    Off(Off),
    Use {
        read: Vec<Tier>,
        write: Option<Tier>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Off {
    NotConfigured,
    Flag,
    Fork,
    FullRun,
    NotAPullRequest,
}

impl fmt::Display for CacheMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheMode::Off(Off::NotConfigured) => f.write_str("cache off"),
            CacheMode::Off(Off::Flag) => f.write_str("cache off: --no-cache"),
            CacheMode::Off(Off::Fork) => f.write_str("cache off: a fork"),
            CacheMode::Off(Off::FullRun) => f.write_str("cache off: a full run"),
            CacheMode::Off(Off::NotAPullRequest) => f.write_str("cache off: not a pull request"),
            CacheMode::Use { read, write } => {
                let read: Vec<String> = read.iter().map(ToString::to_string).collect();
                let write = write.map_or("nothing".into(), |t| t.to_string());
                let read = if read.is_empty() {
                    "nothing".into()
                } else {
                    read.join(" ")
                };
                write!(f, "cache reads {read}, writes {write}")
            }
        }
    }
}

/// A record found and not trusted: the gate runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Refusal {
    KeyMismatch,
    PartsDiffer,
    WrongTier,
    Expired,
    Unreadable,
}

/// What the cache did for a gate that ran.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheNote {
    Unkeyed(Unkeyed),
    Refused(Refusal),
    Rechecked(Proof),
}

/// A skip by the cache: the proving run and its time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hit {
    pub proof: Proof,
    pub took: Millis,
}
