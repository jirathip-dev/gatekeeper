use super::judge::{self, Evidence, SLOW};
use super::{flaky, junit, rerun, spans};
use crate::baseline::Baseline;
use crate::run::{RunCtx, Vars, filled};
use crate::runner::{self, Ended as Exit};
use crate::{Error, Result, config};
use gatekeeper_model::{
    Case, Changed, Ended, Failure, Gate, Host, RunKind, Sighting, Step, TestId, Timed,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

pub(crate) struct Run<'a> {
    pub gate: &'a Gate,
    pub cx: &'a RunCtx,
    pub vars: &'a Vars,
    pub start: Instant,
    pub deadline: Instant,
}

fn fresh(cx: &RunCtx, timed: &Timed, since: SystemTime) -> Result<Vec<Case>> {
    let path = cx.root.join(&timed.junit);
    let modified = fs::metadata(&path).and_then(|m| m.modified());
    match modified {
        Ok(at) if at >= since => {}
        Ok(_) => return Err(Error(format!("{}: not written by this run", timed.junit))),
        Err(e) => return Err(Error(format!("{}: no JUnit file: {e}", timed.junit))),
    }
    junit::read(&fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?)
        .map_err(|e| Error(format!("{}: {e}", timed.junit)))
}

fn kind(cx: &RunCtx) -> RunKind {
    match (cx.host, &cx.base) {
        (Host::Lane, _) => RunKind::Lane,
        (_, Some(_)) => RunKind::PullRequest,
        (_, None) => RunKind::Integration,
    }
}

fn package(root: &Path, path: &str) -> Option<(String, String)> {
    let mut dir = Path::new(path).parent();
    while let Some(d) = dir {
        let table = config::read_toml(&root.join(d).join("Cargo.toml")).ok();
        let name = table
            .as_ref()
            .and_then(|t| t.get("package")?.get("name")?.as_str());
        if let Some(name) = name {
            return Some((d.to_string_lossy().into_owned(), name.to_owned()));
        }
        dir = d.parent();
    }
    None
}

fn changed(cx: &RunCtx) -> Result<BTreeSet<Changed>> {
    let Some(base) = &cx.base else {
        return Ok(BTreeSet::new());
    };
    let args = ["diff", "-U0", "--no-ext-diff", "--no-color", base.as_str()];
    let touched = spans::hunks(&runner::ok("git", &args, &cx.root)?);
    let (mut texts, mut krates) = (BTreeMap::new(), BTreeMap::new());
    for path in touched.keys() {
        if let Ok(text) = fs::read_to_string(cx.root.join(path)) {
            texts.insert(path.clone(), text);
        }
        krates.extend(
            package(&cx.root, path)
                .filter(|_| path.rsplit_once('.').is_some_and(|(_, x)| x == "rs")),
        );
    }
    Ok(spans::changed(&touched, &texts, &krates, &cx.tests))
}

fn soak(cx: &RunCtx) -> Result<BTreeMap<String, String>> {
    let files = runner::ok("git", &["ls-files", "-z"], &cx.root)?;
    let mut out = BTreeMap::new();
    let soaked = files
        .split('\0')
        .filter(|p| cx.tests.soak.iter().any(|g| g.matches(p)));
    for path in soaked {
        let krates: BTreeMap<String, String> = package(&cx.root, path).into_iter().collect();
        let key = if path.ends_with(".rs") {
            spans::binary(path, &krates)
        } else {
            Some(path.to_owned())
        };
        out.extend(key.map(|k| (k, path.to_owned())));
    }
    Ok(out)
}

fn again(
    timed: &Timed,
    tests: &[&Case],
    at: &Run,
    log: &mut File,
) -> std::result::Result<Vec<Case>, Failure> {
    let failed = |e: Error, log: &mut File| {
        writeln!(log, "{e}").ok();
        Failure::Exit(1)
    };
    let Some(Step::Run { run }) = at.gate.steps.first() else {
        return Err(Failure::Exit(1));
    };
    let mut vars = at.vars.clone();
    vars.retain(|(k, _)| *k != "{only}");
    vars.push(("{only}", rerun::only(timed.runner, tests)));
    let files = match at.gate.select {
        Some(_) => rerun::files(tests, &timed.root),
        None => Vec::new(),
    };
    let line = filled(run, at.gate, &vars, at.cx).map_err(|e| failed(e, log))?;
    let since = SystemTime::now();
    match runner::shell(&line, (&at.cx.root, &at.cx.env, &files), log, at.deadline) {
        Ok(Exit::Timeout) => Err(Failure::Timeout),
        Err(e) => Err(failed(e, log)),
        Ok(Exit::Exit(_)) => fresh(at.cx, timed, since).map_err(|e| failed(e, log)),
    }
}

fn listed(listed: &Baseline) -> BTreeSet<TestId> {
    let slow = listed.listed().filter_map(|l| l.strip_prefix(SLOW));
    slow.map(|t| TestId(t.to_owned())).collect()
}

fn sightings(seen: Vec<(TestId, u8)>, at: &Run) -> Vec<Sighting> {
    let head = at.cx.head.clone();
    let sha = head.or_else(|| runner::ok("git", &["rev-parse", "HEAD"], &at.cx.root).ok());
    let sha = sha.unwrap_or_default().trim().to_owned();
    let each = seen.into_iter().map(|(test, passed_on_rerun)| Sighting {
        test,
        sha: sha.clone(),
        on: at.cx.today,
        gate: at.gate.id.clone(),
        passed_on_rerun,
    });
    each.collect()
}

pub(crate) fn run(
    timed: &Timed,
    steps: std::result::Result<(), Failure>,
    at: &Run,
    (log, slow, note, sighted): super::Out,
) -> std::result::Result<(), Failure> {
    let since = SystemTime::now()
        .checked_sub(at.start.elapsed())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let first = match fresh(at.cx, timed, since) {
        Ok(first) => first,
        Err(e) => {
            writeln!(log, "{e}").ok();
            return steps.and(Err(Failure::Exit(1)));
        }
    };
    if steps.is_err() && first.iter().all(|c| c.ended != Ended::Failed) {
        return steps;
    }
    let changed = changed(at.cx).unwrap_or_else(|e| {
        writeln!(log, "changed tests: {e}").ok();
        BTreeSet::new()
    });
    let listed = listed(slow);
    let entries = flaky::entries(&at.cx.root);
    let soak = soak(at.cx).unwrap_or_else(|e| {
        writeln!(log, "soak files: {e}").ok();
        BTreeMap::new()
    });
    let e = Evidence {
        runs: &[],
        changed: &changed,
        slow: &listed,
        flaky: &entries,
        soak: &soak,
        budget: at.cx.tests.budget_secs,
        kind: kind(at.cx),
        root: &timed.root,
        today: at.cx.today,
    };
    let tests = judge::rerun_set(&first, &e);
    let mut runs = vec![first.clone()];
    for _ in [1, 2].iter().filter(|_| !tests.is_empty()) {
        runs.push(again(timed, &tests, at, log)?);
    }
    let verdict = judge::verdict(&Evidence { runs: &runs, ..e });
    slow.unlisted(verdict.kept.iter().map(|t| format!("{SLOW}{t}")).collect());
    for line in [&verdict.summary]
        .into_iter()
        .chain(&verdict.findings)
        .chain(&verdict.warnings)
    {
        writeln!(log, "{line}").ok();
    }
    note.set(verdict.summary).ok();
    sighted.set(sightings(verdict.sighted, at)).ok();
    if verdict.findings.is_empty() {
        Ok(())
    } else {
        Err(Failure::Exit(1))
    }
}

pub(crate) fn slow(gate: &Gate, timed: &Timed, cx: &RunCtx) -> Result<BTreeSet<String>> {
    let Some(Step::Run { run }) = gate.steps.first() else {
        return Err(Error(format!("{}: [timed] runs one line", gate.id)));
    };
    let line = filled(run, gate, &vec![("{only}", String::new())], cx)?;
    fs::create_dir_all(&cx.out).map_err(|e| Error::at(&cx.out, e))?;
    let path = cx.out.join(format!("{}.log", gate.id.file_stem()));
    let mut log = File::create(&path).map_err(|e| Error::at(&path, e))?;
    let deadline = Instant::now() + Duration::from_secs(u64::from(gate.budget.0).max(1) * 5);
    let since = SystemTime::now();
    runner::shell(&line, (&cx.root, &cx.env, &[]), &mut log, deadline)?;
    let cases = fresh(cx, timed, since)
        .map_err(|e| Error(format!("{}: {e}, see {}", gate.id, path.display())))?;
    let slow = cases.iter().filter(|c| c.ended != Ended::Skipped);
    let slow = slow.filter(|c| judge::over(c.took, cx.tests.budget_secs));
    Ok(slow.map(|c| format!("{SLOW}{}", c.id())).collect())
}
