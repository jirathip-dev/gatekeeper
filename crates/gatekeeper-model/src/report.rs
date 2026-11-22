use crate::{
    Affected, CacheNote, GateId, Hit, Platform, ProjectCache, Proof, Secs, Selected, Shard, Skip,
    TestFile,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Milliseconds: a measured time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Millis(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Host {
    GithubHosted { warm_cache: bool },
    SelfHosted,
    Lane,
}

impl Host {
    #[must_use]
    pub fn budget_mode(self) -> BudgetMode {
        match self {
            Host::GithubHosted { warm_cache: true } => BudgetMode::Enforce,
            _ => BudgetMode::Report,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Host::GithubHosted { warm_cache: true } => "github-hosted",
            Host::GithubHosted { warm_cache: false } => "github-hosted (cold cache)",
            Host::SelfHosted => "self-hosted",
            Host::Lane => "lane",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetMode {
    Enforce,
    Report,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Passed { slow: bool },
    Cached(Hit),
    Failed(Failure),
    OverBudget,
    Skipped(Skip),
}

impl Outcome {
    #[must_use]
    pub fn fails(&self) -> bool {
        matches!(self, Outcome::Failed(_) | Outcome::OverBudget)
    }

    #[must_use]
    pub fn word(&self) -> &'static str {
        match self {
            Outcome::Passed { slow: false } => "ok",
            Outcome::Passed { slow: true } => "slow",
            Outcome::Cached(_) => "cached",
            Outcome::Failed(_) => "FAIL",
            Outcome::OverBudget => "OVER",
            Outcome::Skipped(_) => "skip",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Failure {
    Exit(i32),
    Timeout,
    MissingTool(String),
    CacheMismatch(Proof),
    CannotRunHere(Platform),
}

/// A finished gate's outcome: its steps' result, its time against its budget on this host.
#[must_use]
pub fn judge(steps: Result<(), Failure>, took: Millis, budget: Secs, host: Host) -> Outcome {
    let over = took.0 > u64::from(budget.0).saturating_mul(1000);
    match (steps, over, host.budget_mode()) {
        (Err(f), _, _) => Outcome::Failed(f),
        (Ok(()), true, BudgetMode::Enforce) => Outcome::OverBudget,
        (Ok(()), slow, _) => Outcome::Passed { slow },
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub version: String,
    #[serde(default)]
    pub affected: Option<Affected>,
    pub jobs: Vec<JobReport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobReport {
    pub leg: String,
    pub shard: Shard,
    pub host: Host,
    pub setup: Millis,
    pub gates: Vec<GateReport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GateReport {
    pub id: GateId,
    pub outcome: Outcome,
    pub took: Millis,
    pub budget: Secs,
    pub log: String,
    #[serde(default)]
    pub kept: Vec<Kept>,
    /// The project roots the gate ran over: the run's selected tests.
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheNote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<Selected>,
    /// Each test file's time, from the JSON report a `[select]` gate's line wrote to `{report}`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub files: BTreeMap<TestFile, Millis>,
}

/// What the result cache saved and caught over a run's gates.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CacheTotals {
    pub gates: u32,
    pub cached: u32,
    pub ran: Millis,
    pub saved: Millis,
    pub rechecked: u32,
    pub differed: u32,
    pub refused: u32,
}

impl CacheTotals {
    #[must_use]
    pub fn of<'a>(gates: impl IntoIterator<Item = &'a GateReport>) -> CacheTotals {
        let mut t = CacheTotals::default();
        for g in gates {
            if let Some(CacheNote::Projects(p)) = &g.cache {
                t.add_projects(g, p.values());
                continue;
            }
            t.gates += 1;
            match &g.outcome {
                Outcome::Cached(hit) => {
                    (t.cached, t.saved.0) = (t.cached + 1, t.saved.0 + hit.took.0);
                }
                _ => t.ran.0 += g.took.0,
            }
            let differed = matches!(g.outcome, Outcome::Failed(Failure::CacheMismatch(_)));
            t.differed += u32::from(differed);
            t.rechecked += u32::from(matches!(g.cache, Some(CacheNote::Rechecked(_))));
            t.refused += u32::from(matches!(g.cache, Some(CacheNote::Refused(_))));
        }
        t
    }

    fn add_projects<'a>(&mut self, g: &GateReport, p: impl Iterator<Item = &'a ProjectCache>) {
        let mut rechecked = 0;
        for note in p {
            self.gates += 1;
            match note {
                ProjectCache::Cached(hit) => {
                    (self.cached, self.saved.0) = (self.cached + 1, self.saved.0 + hit.took.0);
                }
                ProjectCache::Rechecked(_) => rechecked += 1,
                ProjectCache::Refused(_) => self.refused += 1,
                ProjectCache::Ran => {}
            }
        }
        self.rechecked += rechecked;
        if !matches!(g.outcome, Outcome::Cached(_)) {
            self.ran.0 += g.took.0;
        }
        if matches!(g.outcome, Outcome::Failed(Failure::CacheMismatch(_))) {
            self.differed += rechecked;
        }
    }
}

/// An upload's files, by their repo paths; a copy of each is under `files/<name>/` of the out folder.
#[derive(Debug, Serialize, Deserialize)]
pub struct Kept {
    pub name: String,
    pub files: Vec<String>,
}
