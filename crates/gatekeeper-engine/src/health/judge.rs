use super::rerun::under;
use gatekeeper_model::{Case, Changed, Ended, Millis, RunKind, Secs, TestId};
use std::collections::{BTreeMap, BTreeSet};

/// A slow-list line: the gate's baseline file lists each test over budget this way.
pub const SLOW: &str = "over budget: ";

/// What a timed gate's verdict reads: its runs, the changed test code, its slow list, the budget.
pub struct Evidence<'a> {
    pub runs: &'a [Vec<Case>],
    pub changed: &'a BTreeSet<Changed>,
    pub slow: &'a BTreeSet<TestId>,
    pub budget: Secs,
    pub kind: RunKind,
    pub root: &'a str,
}

/// A timed gate's verdict: findings fail it; `kept` are the listed tests still slow, the rest are stale.
#[derive(Debug, Default)]
pub struct Verdict {
    pub summary: String,
    pub findings: Vec<String>,
    pub warnings: Vec<String>,
    pub kept: BTreeSet<TestId>,
}

fn matches(changed: &Changed, case: &Case, root: &str) -> bool {
    let in_binary = |b: &str| case.class == b || case.class.starts_with(&format!("{b}::bin/"));
    match changed {
        Changed::TsFile(path) => under(root, &case.class) == *path,
        Changed::RustFile { binary, module } => in_binary(binary) && case.name.starts_with(module),
        Changed::RustFn {
            binary,
            module,
            name,
        } => {
            in_binary(binary)
                && case.name.starts_with(module)
                && case.name.split("::").any(|s| s == name)
        }
    }
}

fn is_changed(case: &Case, e: &Evidence) -> bool {
    e.changed.iter().any(|c| matches(c, case, e.root))
}

pub fn over(took: Millis, budget: Secs) -> bool {
    took.0 > u64::from(budget.0) * 1000
}

fn secs(t: Millis) -> String {
    format!("{}.{} s", t.0 / 1000, t.0 % 1000 / 100)
}

/// The tests to run twice more: changed ones, and those over budget that are not on the slow list.
#[must_use]
pub fn rerun_set<'a>(first: &'a [Case], e: &Evidence) -> Vec<&'a Case> {
    if e.kind == RunKind::Integration {
        return Vec::new();
    }
    let ran = first.iter().filter(|c| c.ended != Ended::Skipped);
    let again =
        ran.filter(|c| is_changed(c, e) || over(c.took, e.budget) && !e.slow.contains(&c.id()));
    again.collect()
}

fn judged(case: &Case, runs: &[&Case], e: &Evidence, out: &mut Verdict) {
    let id = case.id();
    let n = runs.len();
    let passed = runs.iter().filter(|c| c.ended == Ended::Passed).count();
    match passed {
        0 if n > 1 => out.findings.push(format!("failed: {id} in all {n} runs")),
        0 => out.findings.push(format!("failed: {id}")),
        p if p < n => out
            .findings
            .push(format!("flaky: {id} passed {p} of {n} runs")),
        _ => {}
    }
    let fastest = runs.iter().map(|c| c.took).min().unwrap_or_default();
    let (listed, changed) = (e.slow.contains(&id), is_changed(case, e));
    if listed && fastest.0 * 2 >= u64::from(e.budget.0) * 1000 {
        out.kept.insert(id.clone());
    }
    if !over(fastest, e.budget) {
        return;
    }
    let b = e.budget.0;
    let took = secs(fastest);
    match () {
        () if listed && !changed => out
            .warnings
            .push(format!("warn: over {b} s, listed: {id} {took}")),
        () => {
            let why = if changed {
                " (changed in this change)"
            } else {
                ""
            };
            let line = format!("over {b} s: {id} took {took} at its fastest of {n}{why}");
            match e.kind {
                RunKind::PullRequest => out.findings.push(line),
                RunKind::Lane | RunKind::Integration => out.warnings.push(format!("warn: {line}")),
            }
        }
    }
}

/// The verdict of a timed gate's runs: the first is the whole line, the others its rerun set.
#[must_use]
pub fn verdict(e: &Evidence) -> Verdict {
    let Some((first, reruns)) = e.runs.split_first() else {
        return Verdict::default();
    };
    let again: BTreeSet<TestId> = rerun_set(first, e).iter().map(|c| c.id()).collect();
    let mut later = BTreeMap::<TestId, Vec<&Case>>::new();
    for case in reruns.iter().flatten().filter(|c| again.contains(&c.id())) {
        later.entry(case.id()).or_default().push(case);
    }
    let mut out = Verdict::default();
    let ran: Vec<&Case> = first.iter().filter(|c| c.ended != Ended::Skipped).collect();
    for case in &ran {
        let mut runs = vec![*case];
        runs.extend(later.remove(&case.id()).unwrap_or_default());
        judged(case, &runs, e, &mut out);
    }
    let listed = e.slow.iter().filter(|t| out.kept.contains(*t)).count();
    out.summary = format!(
        "{} tests · {} run 3 times · {listed} over {} s listed",
        ran.len(),
        again.len(),
        e.budget.0
    );
    out
}
