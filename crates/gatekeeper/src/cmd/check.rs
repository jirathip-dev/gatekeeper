use crate::cli::{self, Args, say};
use gatekeeper_engine::ci::secs;
use gatekeeper_engine::run::{self, RunCtx};
use gatekeeper_engine::{Error, VERSION, config, packs, plan};
use gatekeeper_model::{Gate, GateReport, Host, Outcome, Plan};
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
    let gates = packs::configured(&config, root)?;
    let host = cli::host();
    let given = args.value("--base").map(str::to_owned);
    let pushed = host != Host::Lane && given.is_none();
    let base = given.or_else(|| run::default_base(root, &config.integration));
    let mut filter = args.filter()?;
    if !args.has("--all") && !pushed {
        filter.changed = base.as_deref().map(|b| run::changed(root, b)).transpose()?;
    }
    let plan = plan::plan(&config, gates, host, &filter)?;
    let cx = RunCtx {
        root: root.to_path_buf(),
        out: run::out_dir(root),
        cache: cli::cache(),
        base,
        pr_body: args.value("--pr-body").map(PathBuf::from),
        host,
        job_start: cli::job_start(),
        manifest: config::manifest(&config).to_owned(),
        ts: config.ts.clone(),
    };
    Ok((plan, cx, filter.changed.map(|c| c.len())))
}

/// `check [--pack P] [--gate ID] [--base REV] [--pr-body FILE] [--all]`: plan and run every planned gate.
pub fn run(args: &[String]) -> ExitCode {
    let parsed = Args::parse(
        args,
        &["--pack", "--gate", "--base", "--pr-body"],
        &["--all"],
    );
    let root = match cli::root() {
        Ok(root) => root,
        Err(e) => return cli::usage_error(&e),
    };
    let (plan, cx, changed) = match parsed.and_then(|a| prepare(&a, &root)) {
        Ok(ready) => ready,
        Err(e) => return cli::usage_error(&e),
    };
    let legs: Vec<String> = plan.jobs.iter().map(|j| j.leg.to_string()).collect();
    let base = cx
        .base
        .as_deref()
        .map_or("none", |b| b.get(..12).unwrap_or(b));
    let scope = changed.map_or("every gate".to_owned(), |n| format!("{n} files changed"));
    say(&format!(
        "gatekeeper {VERSION} · packs {} · base {base} · {scope} · host {}",
        legs.join(" "),
        cx.host.label()
    ));
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
