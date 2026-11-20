use crate::runner::{self, Ended};
use crate::{Error, Result, VERSION, check, fetch, packs};
use gatekeeper_model::{
    Decision, Failure, Gate, GateReport, Host, JobReport, Millis, Outcome, Plan, Report, Step,
    judge,
};
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// What a built-in check reads: the repo, its own scratch folder, the Cargo manifest.
pub struct CheckCx<'a> {
    pub root: &'a Path,
    pub scratch: &'a Path,
    pub manifest: &'a str,
}

/// Where and how a plan runs; `out` takes the logs and `report.json`.
pub struct RunCtx {
    pub root: PathBuf,
    pub out: PathBuf,
    pub cache: PathBuf,
    pub base: Option<String>,
    pub pr_body: Option<PathBuf>,
    pub host: Host,
    pub job_start: Option<SystemTime>,
    pub manifest: String,
}

fn millis(d: Duration) -> Millis {
    Millis(u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn expand(args: &[String], cx: &RunCtx) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        match arg.as_str() {
            "{--base}" => out.extend(
                cx.base
                    .iter()
                    .flat_map(|b| ["--base".to_owned(), b.clone()]),
            ),
            "{--pr-body}" => out.extend(
                cx.pr_body
                    .iter()
                    .flat_map(|p| ["--pr-body".to_owned(), p.to_string_lossy().into_owned()]),
            ),
            _ => out.push(arg.clone()),
        }
    }
    out
}

fn ended(r: Result<Ended>, log: &mut File) -> std::result::Result<(), Failure> {
    match r {
        Ok(Ended::Exit(0)) => Ok(()),
        Ok(Ended::Exit(code)) => Err(Failure::Exit(code)),
        Ok(Ended::Timeout) => Err(Failure::Timeout),
        Err(e) => {
            writeln!(log, "{e}").ok();
            Err(Failure::Exit(127))
        }
    }
}

fn step(
    step: &Step,
    gate: &Gate,
    cx: &RunCtx,
    log: &mut File,
    deadline: Instant,
) -> std::result::Result<(), Failure> {
    match step {
        Step::Run { run } => {
            let files = cx.out.join("files").join(gate.id.pack.as_str());
            let prefix = format!("{}/files/", gate.id.pack);
            if let Err(e) = packs::materialize(&prefix, &files) {
                writeln!(log, "{e}").ok();
                return Err(Failure::Exit(127));
            }
            let line = run.replace("{files}", &files.to_string_lossy());
            ended(runner::shell(&line, &cx.root, log, deadline), log)
        }
        Step::Tool { tool, args } => match fetch::tool(tool, &cx.cache) {
            Ok(bin) => ended(
                runner::logged(&bin, &expand(args, cx), &cx.root, log, deadline),
                log,
            ),
            Err(e) => {
                writeln!(log, "{e}").ok();
                Err(Failure::MissingTool(tool.clone()))
            }
        },
        Step::Check { check } => {
            writeln!(log, "$ check {check}").ok();
            let cx = CheckCx {
                root: &cx.root,
                scratch: &cx.out.join("check").join(check),
                manifest: &cx.manifest,
            };
            let findings = check::run(check, &cx).unwrap_or_else(|e| vec![e.0]);
            for f in &findings {
                writeln!(log, "{f}").ok();
            }
            if findings.is_empty() {
                Ok(())
            } else {
                Err(Failure::Exit(1))
            }
        }
    }
}

pub(crate) fn gate(gate: &Gate, cx: &RunCtx) -> Result<GateReport> {
    let path = cx.out.join(format!("{}.log", gate.id.file_stem()));
    let mut log = File::create(&path).map_err(|e| Error::at(&path, e))?;
    let start = Instant::now();
    let deadline = start + Duration::from_secs(u64::from(gate.budget.0).max(1) * 5);
    let result = gate
        .steps
        .iter()
        .try_for_each(|s| step(s, gate, cx, &mut log, deadline));
    let took = millis(start.elapsed());
    Ok(GateReport {
        id: gate.id.clone(),
        outcome: judge(result, took, gate.budget, cx.host),
        took,
        budget: gate.budget,
        log: path
            .strip_prefix(&cx.root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned(),
    })
}

/// # Errors
pub fn run(plan: &Plan, cx: &RunCtx, seen: &mut dyn FnMut(&Gate, &GateReport)) -> Result<Report> {
    fs::create_dir_all(&cx.out).map_err(|e| Error::at(&cx.out, e))?;
    let mut report = Report {
        version: VERSION.into(),
        jobs: Vec::new(),
    };
    for (i, job) in plan.jobs.iter().enumerate() {
        let setup = match cx.job_start {
            Some(start) if i == 0 => {
                millis(SystemTime::now().duration_since(start).unwrap_or_default())
            }
            _ => Millis(0),
        };
        let mut gates = Vec::new();
        for planned in &job.gates {
            let done = match planned.decision {
                Decision::Run => gate(&planned.gate, cx)?,
                Decision::Skip(skip) => GateReport {
                    id: planned.gate.id.clone(),
                    outcome: Outcome::Skipped(skip),
                    took: Millis(0),
                    budget: planned.gate.budget,
                    log: String::new(),
                },
            };
            seen(&planned.gate, &done);
            gates.push(done);
        }
        report.jobs.push(JobReport {
            leg: job.leg,
            host: cx.host,
            setup,
            gates,
        });
    }
    let path = cx.out.join("report.json");
    let json = serde_json::to_string_pretty(&report).map_err(|e| Error(e.to_string()))?;
    fs::write(&path, json).map_err(|e| Error::at(&path, e))?;
    Ok(report)
}

#[must_use]
pub fn out_dir(root: &Path) -> PathBuf {
    root.join("target/gatekeeper")
}

/// The merge base with `origin/<integration>`, when that branch is fetched.
#[must_use]
pub fn default_base(root: &Path, integration: &str) -> Option<String> {
    let branch = format!("origin/{integration}");
    let out = runner::ok("git", &["merge-base", &branch, "HEAD"], root).ok()?;
    Some(out.trim().to_owned())
}
