use super::flaky::quarantined;
use super::rerun::under;
use gatekeeper_model::{Case, Changed, Date, Ended, Flaky, Millis, RunKind, Secs, TestId};
use std::collections::{BTreeMap, BTreeSet};

/// A slow-list line: the gate's baseline file lists each test over budget this way.
pub const SLOW: &str = "over budget: ";

/// What a timed gate's verdict reads: its runs, the changed test code, its slow and flaky lists, the budget, the day.
pub struct Evidence<'a> {
    pub runs: &'a [Vec<Case>],
    pub changed: &'a BTreeSet<Changed>,
    pub slow: &'a BTreeSet<TestId>,
    pub flaky: &'a [Flaky],
    pub soak: &'a BTreeMap<String, String>,
    pub budget: Secs,
    pub kind: RunKind,
    pub root: &'a str,
    pub today: Date,
}

/// A timed gate's verdict: findings fail it; `kept` are the listed tests still slow, the rest are stale; `sighted` failed, then passed this many reruns on the integration branch.
#[derive(Debug, Default)]
pub struct Verdict {
    pub summary: String,
    pub findings: Vec<String>,
    pub warnings: Vec<String>,
    pub kept: BTreeSet<TestId>,
    pub sighted: Vec<(TestId, u8)>,
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

fn soak_file<'a>(case: &Case, e: &Evidence<'a>) -> Option<&'a String> {
    (e.soak.get(&case.class)).or_else(|| e.soak.get(&under(e.root, &case.class)))
}

fn is_changed(case: &Case, e: &Evidence) -> bool {
    e.changed.iter().any(|c| matches(c, case, e.root))
}

#[must_use]
pub fn over(took: Millis, budget: Secs) -> bool {
    took.0 > u64::from(budget.0) * 1000
}

fn secs(t: Millis) -> String {
    format!("{}.{} s", t.0 / 1000, t.0 % 1000 / 100)
}

/// The tests to run twice more: on the integration branch the failed ones; else changed ones, and those over budget that are not on the slow list.
#[must_use]
pub fn rerun_set<'a>(first: &'a [Case], e: &Evidence) -> Vec<&'a Case> {
    if e.kind == RunKind::Integration {
        return first.iter().filter(|c| c.ended == Ended::Failed).collect();
    }
    let ran = first.iter().filter(|c| c.ended != Ended::Skipped);
    let ran = ran.filter(|c| soak_file(c, e).is_none());
    let again =
        ran.filter(|c| is_changed(c, e) || over(c.took, e.budget) && !e.slow.contains(&c.id()));
    again.collect()
}

fn judged(case: &Case, runs: &[&Case], e: &Evidence, out: &mut Verdict) {
    let id = case.id();
    if let Some(file) = soak_file(case, e) {
        out.findings.push(format!(
            "soak: {id} ran here, but {file} is a [tests] soak file: it runs in a nightly gate only"
        ));
        return;
    }
    let n = runs.len();
    let passed = runs.iter().filter(|c| c.ended == Ended::Passed).count();
    let line = match passed {
        0 if n > 1 => Some(format!("failed: {id} in all {n} runs")),
        0 => Some(format!("failed: {id}")),
        p if p < n => Some(format!("flaky: {id} passed {p} of {n} runs")),
        _ => None,
    };
    let held = quarantined(e.flaky, &id, e.today);
    match (line, held) {
        (None, _) => {}
        (Some(line), Some(f)) => out.warnings.push(format!(
            "warn: quarantined until {} ({}): {line}",
            f.until(),
            f.issue
                .map_or_else(|| "no issue".to_owned(), |i| format!("#{i}"))
        )),
        (Some(line), None) if passed > 0 && e.kind == RunKind::Integration => {
            out.warnings.push(format!("warn: {line}: a sighting"));
            out.sighted
                .push((id.clone(), u8::try_from(passed).unwrap_or(u8::MAX)));
        }
        (Some(line), None) => out.findings.push(line),
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
