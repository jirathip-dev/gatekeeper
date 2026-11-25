use super::RunCtx;
use super::line::{Vars, expand, filled};
use crate::baseline::{self, Baseline};
use crate::health::{self, timed};
use crate::runner::{self, Ended};
use crate::select::Picked;
use crate::{Error, Result, fetch, kept};
use gatekeeper_model::{Failure, Gate, GateReport, Platform, Shard, Step, judge};
use std::cell::OnceCell;
use std::fs::File;
use std::io::Write as _;
use std::time::{Duration, Instant};

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

type At<'a> = (&'a Gate, &'a Vars, &'a [String]);

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
        Step::Run { run } => match filled(run, at.0, at.1, cx) {
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
            let scratch = cx.out.join("check").join(check);
            let findings = listed.unlisted(baseline::findings(check, &cx.check(&scratch, note)));
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
    let mut vars = picked.map_or_else(Vars::new, Picked::vars);
    vars.extend([
        ("{shard}", shard.to_string()),
        ("{projects}", projects.join(" ")),
        ("{only}", String::new()),
    ]);
    let at = (gate, &vars, picked.map_or(&[][..], |p| &p.files[..]));
    let summary = picked.map_or_else(OnceCell::new, |p| OnceCell::from(p.note.clone()));
    let steps = here(gate, cx, &mut log).and_then(|()| {
        let mut steps = gate.steps.iter();
        steps.try_for_each(|s| step(s, at, cx, &mut log, deadline, &mut listed, &summary))
    });
    let run = timed::Run {
        gate,
        cx,
        vars: &vars,
        start,
        deadline,
    };
    let result = health::after(steps, &run, (&mut log, &mut listed, &summary)).and_then(|()| {
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
    let took = start.elapsed().into();
    let mut report = GateReport::unrun(gate, judge(result, took, gate.budget, cx.host));
    report.took = took;
    report.log = path
        .strip_prefix(&cx.root)
        .unwrap_or(&path)
        .to_string_lossy()
        .into_owned();
    report.kept = kept::collect(gate, &cx.root, &cx.out)?;
    report.projects = projects.to_vec();
    report.note = summary.into_inner();
    Ok(report)
}
