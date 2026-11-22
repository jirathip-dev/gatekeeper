mod readers;
mod rules;
mod split;
pub mod vitest;

pub use rules::select;
pub use split::split;

use crate::fetch::sha256_hex;
use crate::run::{RunCtx, millis, unrun};
use crate::runner::{self, Ended};
use crate::{Error, Result};
use gatekeeper_model::{
    Evidence, Failure, Full, GateId, GateReport, Outcome, Plan, Planned, Select, Selected,
    Selection, Shard, Skip, TestFile, Timings, Trigger, Why, Widened,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A `[select]` gate's test files for this job: its `{files}`, their digest in the key, the selection line.
pub struct Picked {
    pub files: Vec<String>,
    pub digest: String,
    pub note: String,
    pub start: Instant,
    pub report: PathBuf,
    pub selected: Selected,
}

#[derive(Serialize)]
struct Written<'a> {
    gate: &'a GateId,
    base: Option<&'a str>,
    universe: usize,
    digest: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    full: Option<&'a Full>,
    files: BTreeMap<&'a TestFile, &'a BTreeSet<Why>>,
}

impl Written<'_> {
    fn save(&self, out: &Path) -> Result<()> {
        let path = out.join(format!("{}.selection.json", self.gate.file_stem()));
        let json = serde_json::to_string_pretty(self).map_err(|e| Error(e.to_string()))?;
        fs::write(&path, json).map_err(|e| Error::at(&path, e))
    }
}

const FAMILIES: [&str; 5] = ["closure", "always", "readers", "project-readers", "widened"];

fn family(why: &Why) -> &'static str {
    match why {
        Why::Closure => FAMILIES[0],
        Why::Always(_) => FAMILIES[1],
        Why::Reads(_) => FAMILIES[2],
        Why::NamesProject(_) => FAMILIES[3],
        Why::Widened(_) => FAMILIES[4],
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn by(selection: &Selection) -> BTreeMap<String, u32> {
    let Selection::Files(files) = selection else {
        return BTreeMap::new();
    };
    let has = |name: &str, whys: &BTreeSet<Why>| whys.iter().any(|w| family(w) == name);
    let counted = FAMILIES.map(|name| {
        let n = files.reasons().values().filter(|w| has(name, w)).count();
        (name.to_owned(), count(n))
    });
    counted.into_iter().filter(|(_, n)| *n > 0).collect()
}

fn line(selection: &Selection, universe: usize, took: Duration) -> String {
    let why = match selection {
        Selection::Full(Full::File(path)) => format!("`{path}` changed (full)"),
        Selection::Full(Full::Empty) => "no rule selected a test".into(),
        Selection::Full(Full::NotAChange(widened)) => match widened {
            Widened::Forced => "--all or scope full".into(),
            Widened::NotAPullRequest => "not a change".into(),
            Widened::NoBase => "no base to diff".into(),
            Widened::OutsideEveryProject(path) => format!("`{path}` is outside every project"),
        },
        Selection::Files(files) => {
            let counts = by(selection);
            let by = FAMILIES.iter().filter_map(|f| Some((f, counts.get(*f)?)));
            let by: Vec<String> = by.map(|(name, n)| format!("{name} {n}")).collect();
            let secs = crate::ci::secs(millis(took));
            let n = files.reasons().len();
            return format!(
                "selected {n} of {universe} test files in {secs}: {}",
                by.join(" · ")
            );
        }
    };
    format!("selected all {universe} test files: {why}")
}

pub(crate) fn report_path(out: &Path, gate: &GateId) -> PathBuf {
    out.join(format!("{}.vitest.json", gate.file_stem()))
}

fn recorded(cx: &RunCtx, gate: &GateId) -> Result<Timings> {
    let Some(dir) = &cx.timings else {
        return Ok(Timings::default());
    };
    let path = dir.join(format!("{}.json", gate.file_stem()));
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| Error::at(&path, e)),
        Err(_) => Ok(Timings::default()),
    }
}

fn gather(
    select: &Select,
    plan: &Plan,
    cx: &RunCtx,
    (log, deadline): (&mut File, Instant),
) -> Result<(BTreeSet<TestFile>, Selection)> {
    let ts = cx
        .ts
        .as_ref()
        .ok_or_else(|| Error("[select] needs [ts]".into()))?;
    match runner::shell(&ts.install, (&cx.root, &cx.env, &[]), log, deadline)? {
        Ended::Exit(0) => {}
        ended => return Err(Error(format!("[ts] install `{}`: {ended:?}", ts.install))),
    }
    let universe = vitest::list(&cx.root, &select.vitest, None)?;
    let closure = match (&plan.trigger, &cx.base) {
        (Trigger::Everything(_), _) => BTreeSet::new(),
        (Trigger::Change(_), Some(base)) => vitest::list(&cx.root, &select.vitest, Some(base))?,
        (Trigger::Change(_), None) => return Err(Error("a change with no base to list".into())),
    };
    let mut texts = BTreeMap::new();
    for test in &universe {
        let path = cx.root.join(&test.0);
        let text = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
        texts.insert(test.clone(), text);
    }
    let ev = Evidence {
        universe: &universe,
        closure: &closure,
        texts: &texts,
        graph: &plan.workspace.graph,
    };
    let selection = rules::select(&plan.trigger, select, &ev);
    Ok((universe, selection))
}

fn chosen(
    planned: &Planned,
    select: &Select,
    (plan, cx, shard): (&Plan, &RunCtx, Shard),
    (log, start): (&mut File, Instant),
) -> Result<Picked> {
    let gate = &planned.gate;
    let deadline = start + Duration::from_secs(u64::from(gate.budget.0).max(1) * 5);
    let (universe, selection) = gather(select, plan, cx, (log, deadline))?;
    let unreasoned = BTreeSet::new();
    let (full, listed): (_, BTreeMap<_, _>) = match &selection {
        Selection::Full(full) => (
            Some(full),
            universe.iter().map(|t| (t, &unreasoned)).collect(),
        ),
        Selection::Files(f) => (None, f.reasons().iter().collect()),
    };
    let all: BTreeSet<TestFile> = listed.keys().map(|t| (*t).clone()).collect();
    let timings = recorded(cx, &gate.id)?;
    let split_by = serde_json::to_string(&timings).map_err(|e| Error(e.to_string()))?;
    let whole: Vec<&str> = all.iter().map(|t| t.0.as_str()).collect();
    let digest = sha256_hex(format!("{}\n{split_by}", whole.join("\n")).as_bytes());
    let slice = split(&all, &timings, shard);
    let files: Vec<String> = slice.iter().map(|t| t.0.clone()).collect();
    let written = Written {
        gate: &gate.id,
        base: cx.base.as_deref(),
        universe: universe.len(),
        digest: &digest,
        full,
        files: listed,
    };
    written.save(&cx.out)?;
    let mut note = line(&selection, universe.len(), start.elapsed());
    if shard.of.get() > 1 {
        note = format!("{note} · shard {shard} runs {}", files.len());
    }
    let report = report_path(&cx.out, &gate.id);
    fs::remove_file(&report).ok();
    let selected = Selected {
        universe: count(universe.len()),
        selected: count(all.len()),
        slice: count(files.len()),
        full: full.cloned(),
        by: by(&selection),
        digest,
        took: millis(start.elapsed()),
    };
    Ok(Picked {
        digest: sha256_hex(files.join("\n").as_bytes()),
        files,
        note,
        start,
        report,
        selected,
    })
}

pub(crate) fn pick(
    planned: &Planned,
    (plan, shard): (&Plan, Shard),
    cx: &RunCtx,
) -> std::result::Result<Option<Picked>, Box<GateReport>> {
    let Some(select) = &planned.gate.select else {
        return Ok(None);
    };
    let start = Instant::now();
    let path = cx
        .out
        .join(format!("{}.select.log", planned.gate.id.file_stem()));
    let picked = File::create(&path)
        .map_err(|e| Error::at(&path, e))
        .and_then(|mut log| {
            let picked = chosen(planned, select, (plan, cx, shard), (&mut log, start));
            if let Err(e) = &picked {
                writeln!(log, "the selection failed, so the gate fails: {e}").ok();
            }
            picked
        });
    let log = path.strip_prefix(&cx.root).unwrap_or(&path);
    let log = log.to_string_lossy().into_owned();
    let took = millis(start.elapsed());
    match picked {
        Ok(p) if p.files.is_empty() => Err(Box::new(GateReport {
            took,
            log,
            note: Some(p.note),
            selection: Some(p.selected),
            ..unrun(planned, Outcome::Skipped(Skip::EmptyShard))
        })),
        Ok(p) => Ok(Some(p)),
        Err(_) => Err(Box::new(GateReport {
            took,
            log,
            ..unrun(planned, Outcome::Failed(Failure::Exit(1)))
        })),
    }
}

pub(crate) fn reported(
    picked: Option<&Picked>,
    report: GateReport,
    root: &Path,
) -> Result<GateReport> {
    let Some(p) = picked else {
        return Ok(report);
    };
    Ok(GateReport {
        selection: Some(p.selected.clone()),
        files: vitest::times(root, &p.report)?,
        ..report
    })
}
