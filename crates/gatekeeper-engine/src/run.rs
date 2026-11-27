use crate::select;
use crate::{Error, Result, VERSION, cache, kept};
use gatekeeper_model::{
    Decision, Gate, GateReport, Job, JobReport, Millis, Outcome, Plan, Planned, Report, Skip,
};
use std::fs;
use std::time::SystemTime;

mod cx;
mod gate;
mod line;

pub use cx::{CheckCx, RunCtx, default_base, out_dir, platform, pushed_base};
pub(crate) use gate::gate;
pub use line::script;
pub(crate) use line::{Vars, filled};

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

fn one(planned: &Planned, plan: &Plan, job: &Job, cx: &RunCtx) -> Result<GateReport> {
    let picked = match select::pick(planned, (plan, job.shard), cx) {
        Ok(picked) => picked,
        Err(ended) => return Ok(*ended),
    };
    let run = |projects: &[String]| gate(&planned.gate, (job.shard, picked.as_ref()), projects, cx);
    let report = match &cx.results {
        Some(results) => {
            let files = picked.as_ref().map(|p| p.digest.as_str());
            cache::gate(
                results,
                planned,
                (job.shard, &plan.workspace, files),
                &cx.root,
                run,
            )?
        }
        None => run(&planned.projects)?,
    };
    select::reported(picked.as_ref(), report, &cx.root)
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
            Some(start) if i == 0 => SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .into(),
            _ => Millis(0),
        };
        let waited = after_passed(job, &report.jobs, cx)?;
        let mut gates = Vec::new();
        for planned in &job.gates {
            let unrun = |outcome| GateReport::unrun(&planned.gate, outcome);
            let done = match &planned.decision {
                Decision::Run if !waited => unrun(Outcome::Skipped(Skip::AfterFailed)),
                Decision::Run => one(planned, plan, job, cx)?,
                Decision::Skip(skip) => unrun(Outcome::Skipped(*skip)),
                Decision::Off(why) => unrun(Outcome::Off(why.clone())),
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
