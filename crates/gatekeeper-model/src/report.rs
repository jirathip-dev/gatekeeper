use crate::{GateId, PackId, Secs, Skip};
use serde::{Deserialize, Serialize};

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
    pub jobs: Vec<JobReport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobReport {
    pub leg: PackId,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// An upload's files, by their repo paths; a copy of each is under `files/<name>/` of the out folder.
#[derive(Debug, Serialize, Deserialize)]
pub struct Kept {
    pub name: String,
    pub files: Vec<String>,
}
