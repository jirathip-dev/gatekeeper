use crate::cli::{self, Args, say};
use gatekeeper_engine::select::replay::{self, Picked, Replay, ReplayRun};
use gatekeeper_engine::{Error, config, run};
use gatekeeper_model::GateId;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn need<'a>(args: &'a Args, flag: &str) -> Result<&'a str, Error> {
    args.value(flag)
        .ok_or_else(|| Error(format!("replay needs {flag}")))
}

fn runs(path: &str) -> Result<Vec<ReplayRun>, Error> {
    let text = fs::read_to_string(path).map_err(|e| Error(format!("--runs {path}: {e}")))?;
    let lines = text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty());
    let parse = |(i, l): (usize, &str)| {
        serde_json::from_str(l).map_err(|e| Error(format!("{path}:{}: {e}", i + 1)))
    };
    lines.map(parse).collect()
}

fn row(cells: [&str; 10]) -> String {
    let [
        pr,
        head,
        changed,
        ours,
        theirs,
        only_ours,
        only_theirs,
        failed,
        missed,
        missed_other,
    ] = cells;
    format!(
        "| {pr:<4} | {head:<7} | {changed:>7} | {ours:<19} | {theirs:<10} | {only_ours:>15} | {only_theirs:>10} | {failed:>6} | {missed:>18} | {missed_other:>5} |"
    )
}

fn side(picked: &Picked, universe: usize) -> String {
    match picked.full.as_deref() {
        None => picked.selected.to_string(),
        Some("all") => format!("all ({universe})"),
        Some(why) => format!("all: {why}"),
    }
}

fn percent(n: usize, of: usize) -> String {
    let permille = n * 1000 / of.max(1);
    format!("{}.{} %", permille / 10, permille % 10)
}

const HEAD: [&str; 10] = [
    "pr",
    "head",
    "changed",
    "gatekeeper",
    "other",
    "only gatekeeper",
    "only other",
    "failed",
    "missed: gatekeeper",
    "other",
];

fn report(r: &Replay) -> Vec<String> {
    let mut lines = vec![row(HEAD)];
    let mut missed = Vec::new();
    for run in &r.runs {
        let short = run.head.get(..7).unwrap_or(&run.head);
        let n = |set: &std::collections::BTreeSet<_>| set.len().to_string();
        let cells = [
            run.pr.to_string(),
            short.to_owned(),
            run.changed.to_string(),
            side(&run.gatekeeper, run.universe),
            side(&run.other, run.universe),
            n(&run.only_gatekeeper),
            n(&run.only_other),
            n(&run.failed),
            n(&run.missed_gatekeeper),
            n(&run.missed_other),
        ];
        lines.push(row(cells.each_ref().map(String::as_str)));
        let by = [
            ("gatekeeper", &run.missed_gatekeeper),
            ("other", &run.missed_other),
        ];
        for (who, files) in by {
            for file in files {
                missed.push(format!("missed by {who}: pr {} {short} {}", run.pr, file.0));
            }
        }
    }
    let t = &r.totals;
    let (ours, theirs) = (&t.gatekeeper, &t.other);
    lines.push(format!(
        "totals: gatekeeper {} file runs ({}), {} full · other {} ({}), {} full · missed: gatekeeper {}, other {}",
        ours.selected,
        percent(ours.selected, t.file_runs),
        ours.full,
        theirs.selected,
        percent(theirs.selected, t.file_runs),
        theirs.full,
        ours.missed,
        theirs.missed
    ));
    let by =
        t.by.iter()
            .map(|(name, n)| format!("{} {n}", name.replace('-', " ")));
    lines.push(format!("by rule: {}", by.collect::<Vec<_>>().join(" · ")));
    lines.extend(missed);
    lines
}

fn replay(args: &Args, root: &Path) -> Result<bool, Error> {
    let gate = GateId::try_from(need(args, "--gate")?.to_owned()).map_err(Error)?;
    let runs = runs(need(args, "--runs")?)?;
    let config = config::load(root)?;
    let r = replay::replay(&config, root, &gate, &runs, need(args, "--other")?)?;
    let lines = report(&r);
    let dir = run::out_dir(root).join("replay");
    let json = serde_json::to_string_pretty(&r).map_err(|e| Error(e.to_string()))?;
    let (json_at, md_at) = (dir.join("replay.json"), dir.join("replay.md"));
    fs::write(&json_at, json).map_err(|e| Error::at(&json_at, e))?;
    let mut md = lines.clone();
    md.insert(1, row(["---"; 10]));
    fs::write(&md_at, md.join("\n") + "\n").map_err(|e| Error::at(&md_at, e))?;
    let universe = r.runs.last().map_or(0, |run| run.universe);
    say(&format!(
        "replay {gate} · {} runs of {} pull requests · {universe} test files",
        r.totals.runs, r.totals.pull_requests
    ));
    for line in &lines {
        say(line);
    }
    say("wrote target/gatekeeper/replay/replay.json and replay.md");
    Ok(r.totals.gatekeeper.missed > 0)
}

/// `replay --gate ID --runs FILE --other CMD`: both selectors over each listed run; exit 1 when gatekeeper missed a failure.
pub fn run(args: &[String]) -> ExitCode {
    let parsed = Args::parse(args, &["--gate", "--runs", "--other"], &[]);
    let root = match cli::root() {
        Ok(root) => root,
        Err(e) => return cli::usage_error(&e),
    };
    match parsed.and_then(|a| replay(&a, &root)) {
        Ok(missed) => cli::exit(missed),
        Err(e) => cli::usage_error(&e),
    }
}
