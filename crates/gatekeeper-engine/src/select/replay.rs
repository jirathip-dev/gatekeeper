use super::{by, gather};
use crate::run::out_dir;
use crate::{Error, Result, graph, packs, runner, scope};
use gatekeeper_model::{
    Config, Full, GateId, GraphKind, Select, Selection, TestFile, Trigger, TsPack,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::Path;
use std::time::{Duration, Instant};

/// One line of `--runs`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRun {
    pub pr: u64,
    pub base: String,
    pub head: String,
    pub failed: BTreeSet<TestFile>,
}

/// `replay.json`.
#[derive(Serialize)]
pub struct Replay {
    pub gate: GateId,
    pub runs: Vec<Replayed>,
    pub totals: Totals,
}

/// One run selected by both selectors.
#[derive(Serialize)]
pub struct Replayed {
    pub pr: u64,
    pub head: String,
    pub base: String,
    pub changed: usize,
    pub universe: usize,
    pub gatekeeper: Picked,
    pub other: Picked,
    pub only_gatekeeper: BTreeSet<TestFile>,
    pub only_other: BTreeSet<TestFile>,
    pub failed: BTreeSet<TestFile>,
    pub missed_gatekeeper: BTreeSet<TestFile>,
    pub missed_other: BTreeSet<TestFile>,
}

/// One selector's pick of a run; `by` is empty for `other`.
#[derive(Serialize)]
pub struct Picked {
    pub selected: usize,
    pub full: Option<String>,
    pub by: BTreeMap<String, usize>,
}

/// The sums over every run.
#[derive(Serialize, Default)]
pub struct Totals {
    pub runs: usize,
    pub pull_requests: usize,
    pub file_runs: usize,
    pub gatekeeper: Side,
    pub other: Side,
    pub by: BTreeMap<String, usize>,
}

/// One selector's totals.
#[derive(Serialize, Default)]
pub struct Side {
    pub selected: usize,
    pub full: usize,
    pub missed: usize,
}

struct Gate<'a> {
    select: &'a Select,
    graphs: &'a BTreeSet<GraphKind>,
    ts: Option<&'a TsPack>,
    budget: Duration,
    other: &'a str,
}

fn ours(universe: &BTreeSet<TestFile>, selection: &Selection) -> (BTreeSet<TestFile>, Picked) {
    let (files, full, by) = match selection {
        Selection::Full(full) => {
            let why = match full {
                Full::File(path) => path.clone(),
                Full::Empty => "no rule".into(),
                Full::NotAChange(_) => "not a change".into(),
            };
            (universe.clone(), Some(why), BTreeMap::new())
        }
        Selection::Files(files) => {
            let by = by(files).map(|(name, n)| (name.to_owned(), n)).collect();
            (files.reasons().keys().cloned().collect(), None, by)
        }
    };
    let selected = files.len();
    (files, Picked { selected, full, by })
}

fn one(run: &ReplayRun, gate: &Gate, (root, wt, log): (&Path, &Path, &Path)) -> Result<Replayed> {
    let base = runner::ok("git", &["merge-base", &run.base, &run.head], root)?;
    let base = base.trim().to_owned();
    let diff = scope::changed(wt, &base)?;
    let changed = diff.len();
    let graph = graph::read(wt, gate.graphs)?;
    let mut log = File::create(log).map_err(|e| Error::at(log, e))?;
    let deadline = Instant::now() + gate.budget;
    let (universe, selection) = gather(
        (gate.select, gate.ts),
        (&Trigger::Change(diff), &graph),
        (wt, &[], Some(&base)),
        (&mut log, deadline),
    )?;
    let printed = runner::ok("sh", &["-c", &gate.other.replace("{base}", &base)], wt)?;
    let theirs: BTreeSet<TestFile> = printed
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| TestFile(l.to_owned()))
        .collect();
    let (mine, gatekeeper) = ours(&universe, &selection);
    let other = Picked {
        selected: theirs.len(),
        full: (!universe.is_empty() && theirs.is_superset(&universe)).then(|| "all".into()),
        by: BTreeMap::new(),
    };
    Ok(Replayed {
        pr: run.pr,
        head: run.head.clone(),
        base,
        changed,
        universe: universe.len(),
        gatekeeper,
        other,
        only_gatekeeper: &mine - &theirs,
        only_other: &theirs - &mine,
        failed: run.failed.clone(),
        missed_gatekeeper: &run.failed - &mine,
        missed_other: &run.failed - &theirs,
    })
}

fn totals(runs: &[Replayed]) -> Totals {
    let prs: BTreeSet<u64> = runs.iter().map(|r| r.pr).collect();
    let mut t = Totals {
        runs: runs.len(),
        pull_requests: prs.len(),
        ..Totals::default()
    };
    for r in runs {
        t.file_runs += r.universe;
        let sides = [
            (&mut t.gatekeeper, &r.gatekeeper, &r.missed_gatekeeper),
            (&mut t.other, &r.other, &r.missed_other),
        ];
        for (side, picked, missed) in sides {
            side.selected += picked.selected;
            side.full += usize::from(picked.full.is_some());
            side.missed += missed.len();
        }
        for (name, n) in &r.gatekeeper.by {
            *t.by.entry(name.clone()).or_default() += n;
        }
    }
    t
}

/// # Errors
pub fn replay(
    config: &Config,
    root: &Path,
    id: &GateId,
    runs: &[ReplayRun],
    other: &str,
) -> Result<Replay> {
    let gates = packs::configured(config, root)?;
    let found = gates.iter().find(|g| g.id == *id);
    let found = found.ok_or_else(|| Error(format!("--gate {id}: no such gate")))?;
    let select = (found.select.as_ref()).ok_or_else(|| Error(format!("{id} has no [select]")))?;
    let scope = (config.scope.as_ref()).ok_or_else(|| Error("[select] needs [scope]".into()))?;
    let gate = Gate {
        select,
        graphs: &scope.graphs,
        ts: config.ts.as_ref(),
        budget: Duration::from_secs(u64::from(found.budget.0).max(1) * 5),
        other,
    };
    let dir = out_dir(root).join("replay");
    fs::create_dir_all(&dir).map_err(|e| Error::at(&dir, e))?;
    let wt = dir.join("worktree");
    let at = wt.to_string_lossy();
    let mut replayed = Vec::new();
    for run in runs {
        runner::capture("git", &["worktree", "remove", "--force", &at], root)?;
        let add = ["worktree", "add", "--detach", "--force", &at, &run.head];
        runner::ok("git", &add, root)?;
        let log = dir.join(format!("{}.log", run.pr));
        let done = one(run, &gate, (root, &wt, &log));
        runner::ok("git", &["worktree", "remove", "--force", &at], root)?;
        replayed.push(done?);
    }
    let totals = totals(&replayed);
    Ok(Replay {
        gate: id.clone(),
        runs: replayed,
        totals,
    })
}
