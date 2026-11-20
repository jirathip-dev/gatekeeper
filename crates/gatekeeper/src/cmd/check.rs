use crate::cli::{self, Args, say};
use gatekeeper_engine::ci::secs;
use gatekeeper_engine::run::{self, RunCtx};
use gatekeeper_engine::{Error, VERSION, config, plan};
use gatekeeper_model::{Gate, GateReport, Host, Outcome, Plan, Trigger, Widened, job_label};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn tail(root: &Path, log: &str) -> Vec<String> {
    let text = std::fs::read_to_string(root.join(log)).unwrap_or_default();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let from = lines.len().saturating_sub(5);
    lines.iter().skip(from).map(|l| (*l).to_owned()).collect()
}

fn show(root: &Path, gate: &Gate, r: &GateReport) {
    let word = r.outcome.word();
    match &r.outcome {
        Outcome::Skipped(why) => say(&format!("{word:<5} {:<18} {:>16}  {why}", r.id, "")),
        outcome => {
            say(&format!(
                "{word:<5} {:<18} {:>6} / {:>4}s",
                r.id,
                secs(r.took),
                r.budget.0
            ));
            if !r.projects.is_empty() {
                say(&format!("      selected: {}", r.projects.join(" ")));
            }
            if let Some(note) = &r.note {
                say(&format!("      {note}"));
            }
            for kept in &r.kept {
                let dir = format!(
                    "{}/files/{}/",
                    r.log.rsplit_once('/').map_or("", |(d, _)| d),
                    kept.name
                );
                let globs: Vec<String> = gate
                    .uploads
                    .iter()
                    .filter(|u| u.name == kept.name)
                    .flat_map(|u| u.paths.iter().map(ToString::to_string))
                    .collect();
                match kept.files.len() {
                    0 => say(&format!(
                        "      files: {} (0) nothing matched {}",
                        kept.name,
                        globs.join(" ")
                    )),
                    n => say(&format!("      files: {} ({n}) {dir}", kept.name)),
                }
            }
            if outcome.fails() {
                for line in tail(root, &r.log) {
                    say(&format!("      {line}"));
                }
                say(&format!("      fix: {}", gate.fix));
                say(&format!(
                    "      log: {}   rerun: ./.gatekeeper/run check --gate {}",
                    r.log, r.id
                ));
            }
        }
    }
}

fn prepare(args: &Args, root: &Path) -> Result<(Plan, RunCtx, Option<usize>), Error> {
    let config = config::load(root)?;
    let host = cli::host();
    let given = args.value("--base").map(str::to_owned);
    let pushed = host != Host::Lane && given.is_none();
    let base = given.or_else(|| run::default_base(root, &config.integration));
    let mut filter = args.filter()?;
    filter.trigger = match &base {
        _ if args.has("--all") => Trigger::Everything(Widened::Forced),
        _ if pushed => Trigger::Everything(Widened::NotAPullRequest),
        None => Trigger::Everything(Widened::NoBase),
        Some(b) => Trigger::Change(run::changed(root, b)?),
    };
    let plan = plan::repo(&config, root, &|_| host, &filter)?;
    let cx = RunCtx {
        root: root.to_path_buf(),
        out: run::out_dir(root),
        cache: cli::cache(),
        base,
        pr_body: args.value("--pr-body").map(PathBuf::from),
        host,
        job_start: cli::job_start(),
        manifest: config::manifest(&config).to_owned(),
        after_reports: args.value("--after-reports").map(PathBuf::from),
        ts: config.ts.clone(),
    };
    let changed = match filter.trigger {
        Trigger::Change(files) => Some(files.len()),
        Trigger::Everything(_) => None,
    };
    Ok((plan, cx, changed))
}

const VALUED: [&str; 7] = [
    "--pack",
    "--leg",
    "--shard",
    "--gate",
    "--base",
    "--pr-body",
    "--after-reports",
];

fn header(plan: &Plan, cx: &RunCtx, changed: Option<usize>) {
    let legs: Vec<String> = plan
        .jobs
        .iter()
        .map(|j| job_label(&j.leg, j.shard))
        .collect();
    let base = cx
        .base
        .as_deref()
        .map_or("none", |b| b.get(..12).unwrap_or(b));
    let scope = changed.map_or("every gate".to_owned(), |n| format!("{n} files changed"));
    say(&format!(
        "gatekeeper {VERSION} · legs {} · base {base} · {scope} · host {}",
        legs.join(" "),
        cx.host.label()
    ));
    if let Some(affected) = &plan.affected {
        say(&format!("scope: {affected}"));
    }
}

/// `check [--pack P] [--leg L] [--shard I/N] [--gate ID] [--base REV] [--pr-body FILE] [--after-reports DIR] [--all]`.
pub fn run(args: &[String]) -> ExitCode {
    let parsed = Args::parse(args, &VALUED, &["--all"]);
    let root = match cli::root() {
        Ok(root) => root,
        Err(e) => return cli::usage_error(&e),
    };
    let (plan, cx, changed) = match parsed.and_then(|a| prepare(&a, &root)) {
        Ok(ready) => ready,
        Err(e) => return cli::usage_error(&e),
    };
    header(&plan, &cx, changed);
    let report = match run::run(&plan, &cx, &mut |gate, r| show(&root, gate, r)) {
        Ok(report) => report,
        Err(e) => return cli::usage_error(&e),
    };
    let gates = report.jobs.iter().flat_map(|j| &j.gates);
    let failed: Vec<String> = gates
        .clone()
        .filter(|g| g.outcome.fails())
        .map(|g| g.id.to_string())
        .collect();
    let setup = report
        .jobs
        .first()
        .map_or_else(String::new, |j| secs(j.setup));
    if failed.is_empty() {
        say(&format!("{} gates passed · setup {setup}", gates.count()));
    } else {
        say(&format!(
            "{} failed: {} · setup {setup}",
            failed.len(),
            failed.join(", ")
        ));
    }
    cli::exit(!failed.is_empty())
}
