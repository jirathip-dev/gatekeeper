use crate::baseline::{self, Baseline};
use crate::cache::{self, Results};
use crate::runner::{self, Ended};
use crate::select::{self, Picked};
use crate::{Error, Result, VERSION, fetch, kept, packs};
use gatekeeper_model::{
    Decision, Failure, Gate, GateReport, Host, Job, JobReport, Millis, Outcome, PackId, Plan,
    Planned, Platform, Report, Shard, Skip, Step, SwiftPack, TsPack, judge,
};
use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// What a built-in check reads: the repo, its scratch folder, the tool cache, the packs' tables, the base; where it notes a summary.
pub struct CheckCx<'a> {
    pub root: &'a Path,
    pub scratch: &'a Path,
    pub cache: &'a Path,
    pub manifest: &'a str,
    pub ts: Option<&'a TsPack>,
    pub swift: Option<&'a SwiftPack>,
    pub tools: &'a BTreeMap<String, PathBuf>,
    pub base: Option<&'a str>,
    pub note: &'a OnceCell<String>,
}

impl CheckCx<'_> {
    /// # Errors
    pub fn ts(&self) -> Result<&TsPack> {
        self.ts.ok_or_else(|| {
            Error("the ts pack's checks need [ts] in .gatekeeper/config.toml".into())
        })
    }

    /// # Errors
    pub fn swift(&self) -> Result<&SwiftPack> {
        self.swift.ok_or_else(|| {
            Error("the swift pack's checks need [swift] in .gatekeeper/config.toml".into())
        })
    }

    /// # Errors
    pub fn tool(&self, name: &str) -> Result<PathBuf> {
        fetch::resolve(name, self.cache, self.tools)
    }
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
    /// The reports and kept files of the leg an `after` job waits for, downloaded in CI.
    pub after_reports: Option<PathBuf>,
    pub ts: Option<TsPack>,
    /// `None` when this run neither reads nor writes the result cache.
    pub results: Option<Results>,
    pub swift: Option<SwiftPack>,
    /// `[tools]`: the tools this repo builds itself, by absolute path.
    pub tools: BTreeMap<String, PathBuf>,
    /// The platform this host is: a gate of another fails `needs <platform>`.
    pub platform: Platform,
    /// Set for every step: a fixture case's `env`.
    pub env: Vec<(String, String)>,
    /// `--timings`: the recorded times a sharded `[select]` gate splits by, one `<gate>.json` each.
    pub timings: Option<PathBuf>,
}

/// The platform this binary runs on.
#[must_use]
pub fn platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::Macos
    } else {
        Platform::Linux
    }
}

pub(crate) fn millis(d: Duration) -> Millis {
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

fn pack_dir(pack: PackId, dir: &str, out: &Path) -> Result<PathBuf> {
    let to = out.join("pack").join(pack.as_str()).join(dir);
    packs::materialize(&format!("{pack}/{dir}/"), &to)?;
    Ok(to)
}

/// # Errors
pub fn script(root: &Path, pack: PackId, script: &str, args: &[String]) -> Result<i32> {
    let path = pack_dir(pack, "scripts", &out_dir(root))?.join(script);
    let mut all = vec![path.to_string_lossy().into_owned()];
    all.extend_from_slice(args);
    runner::attached("node", &all, root)
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

type At<'a> = (&'a Gate, &'a [(&'a str, String)], &'a [String]);

fn run_line(run: &str, (gate, vars, _): At, cx: &RunCtx) -> Result<String> {
    let base = cx
        .base
        .as_ref()
        .map_or(String::new(), |b| format!("--base {b}"));
    let body = cx
        .pr_body
        .as_ref()
        .map_or(String::new(), |p| p.to_string_lossy().into_owned());
    let mut line = run.replace("{--base}", &base).replace("{pr-body}", &body);
    if gate.select.is_some() {
        let report = select::report_path(&cx.out, &gate.id);
        line = line.replace("{files}", "\"$@\"");
        line = line.replace("{report}", &report.to_string_lossy());
    }
    for dir in ["files", "scripts"] {
        let to = pack_dir(gate.id.pack, dir, &cx.out)?;
        line = line.replace(&format!("{{{dir}}}"), &to.to_string_lossy());
    }
    for (var, value) in vars {
        line = line.replace(var, value);
    }
    for tool in fetch::named_in(run) {
        let bin = fetch::resolve(tool, &cx.cache, &cx.tools)?;
        line = line.replace(&format!("{{tool:{tool}}}"), &bin.to_string_lossy());
    }
    Ok(line)
}

fn step(
    step: &Step,
    at: At,
    cx: &RunCtx,
    log: &mut File,
    deadline: Instant,
    listed: &mut Baseline,
    note: &OnceCell<String>,
) -> std::result::Result<(), Failure> {
    match step {
        Step::Run { run } => match run_line(run, at, cx) {
            Ok(line) => ended(
                runner::shell(&line, (&cx.root, &cx.env, at.2), log, deadline),
                log,
            ),
            Err(e) => {
                writeln!(log, "{e}").ok();
                Err(Failure::Exit(127))
            }
        },
        Step::Tool { tool, args } => match fetch::resolve(tool, &cx.cache, &cx.tools) {
            Ok(bin) => ended(
                runner::logged((&bin, &cx.env), &expand(args, cx), &cx.root, log, deadline),
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
                cache: &cx.cache,
                manifest: &cx.manifest,
                ts: cx.ts.as_ref(),
                swift: cx.swift.as_ref(),
                tools: &cx.tools,
                base: cx.base.as_deref(),
                note,
            };
            let findings = listed.unlisted(baseline::findings(check, &cx));
            for line in note.get().into_iter().chain(&findings) {
                writeln!(log, "{line}").ok();
            }
            if findings.is_empty() {
                Ok(())
            } else {
                Err(Failure::Exit(1))
            }
        }
    }
}

fn here(gate: &Gate, cx: &RunCtx, log: &mut File) -> std::result::Result<(), Failure> {
    if gate.platform != Platform::Macos || cx.platform == Platform::Macos {
        return Ok(());
    }
    writeln!(
        log,
        "needs macos: {} runs on a macOS host, this one is {}; queue it to a macOS host",
        gate.id,
        cx.platform.runner()
    )
    .ok();
    Err(Failure::CannotRunHere(Platform::Macos))
}

pub(crate) fn gate(
    gate: &Gate,
    (shard, picked): (Shard, Option<&Picked>),
    projects: &[String],
    cx: &RunCtx,
) -> Result<GateReport> {
    let path = cx.out.join(format!("{}.log", gate.id.file_stem()));
    let mut log = File::create(&path).map_err(|e| Error::at(&path, e))?;
    let start = picked.map_or_else(Instant::now, |p| p.start);
    let deadline = start + Duration::from_secs(u64::from(gate.budget.0).max(1) * 5);
    let mut listed = Baseline::load(&cx.root, &gate.id)?;
    let vars = [
        ("{shard}", shard.to_string()),
        ("{projects}", projects.join(" ")),
    ];
    let at = (gate, &vars[..], picked.map_or(&[][..], |p| &p.files[..]));
    let summary = picked.map_or_else(OnceCell::new, |p| OnceCell::from(p.note.clone()));
    let result = here(gate, cx, &mut log)
        .and_then(|()| {
            let mut steps = gate.steps.iter();
            steps.try_for_each(|s| step(s, at, cx, &mut log, deadline, &mut listed, &summary))
        })
        .and_then(|()| {
            let mut stale = listed.stale().peekable();
            let none = stale.peek().is_none();
            for line in stale {
                writeln!(
                    log,
                    "stale: shrink the baseline (./.gatekeeper/run baseline {}): {line}",
                    gate.id
                )
                .ok();
            }
            if none { Ok(()) } else { Err(Failure::Exit(1)) }
        });
    let took = millis(start.elapsed());
    let kept = kept::collect(gate, &cx.root, &cx.out)?;
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
        kept,
        projects: projects.to_vec(),
        note: summary.into_inner(),
        cache: None,
        selection: None,
        files: BTreeMap::new(),
    })
}

pub(crate) fn unrun(planned: &Planned, outcome: Outcome) -> GateReport {
    GateReport {
        id: planned.gate.id.clone(),
        outcome,
        took: Millis(0),
        budget: planned.gate.budget,
        log: String::new(),
        kept: Vec::new(),
        projects: Vec::new(),
        note: None,
        cache: None,
        selection: None,
        files: BTreeMap::new(),
    }
}

fn after_passed(job: &Job, done: &[JobReport], cx: &RunCtx) -> Result<bool> {
    let Some(after) = &job.after else {
        return Ok(true);
    };
    let mut first = done.iter().filter(|j| j.leg == *after).peekable();
    if first.peek().is_some() {
        return Ok(first.flat_map(|j| &j.gates).all(|g| !g.outcome.fails()));
    }
    match &cx.after_reports {
        Some(dir) => kept::receive(dir, after, &cx.root),
        None => Ok(true),
    }
}

/// # Errors
pub fn run(plan: &Plan, cx: &RunCtx, seen: &mut dyn FnMut(&Gate, &GateReport)) -> Result<Report> {
    fs::create_dir_all(&cx.out).map_err(|e| Error::at(&cx.out, e))?;
    let mut report = Report {
        version: VERSION.into(),
        affected: plan.affected.clone(),
        jobs: Vec::new(),
    };
    for (i, job) in plan.jobs.iter().enumerate() {
        let setup = match cx.job_start {
            Some(start) if i == 0 => {
                millis(SystemTime::now().duration_since(start).unwrap_or_default())
            }
            _ => Millis(0),
        };
        let waited = after_passed(job, &report.jobs, cx)?;
        let mut gates = Vec::new();
        for planned in &job.gates {
            let done = match planned.decision {
                Decision::Run if !waited => unrun(planned, Outcome::Skipped(Skip::AfterFailed)),
                Decision::Run => match select::pick(planned, (plan, job.shard), cx) {
                    Err(failed) => *failed,
                    Ok(picked) => {
                        let at = (job.shard, picked.as_ref());
                        let run = |projects: &[String]| gate(&planned.gate, at, projects, cx);
                        let report = match &cx.results {
                            Some(res) => {
                                let digest = picked.as_ref().map(|p| p.digest.as_str());
                                let at = (job.shard, &plan.workspace, digest);
                                cache::gate(res, planned, at, &cx.root, run)?
                            }
                            None => run(&planned.projects)?,
                        };
                        select::reported(picked.as_ref(), report, &cx.root)?
                    }
                },
                Decision::Skip(skip) => unrun(planned, Outcome::Skipped(skip)),
            };
            seen(&planned.gate, &done);
            gates.push(done);
        }
        report.jobs.push(JobReport {
            leg: job.leg.clone(),
            shard: job.shard,
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
