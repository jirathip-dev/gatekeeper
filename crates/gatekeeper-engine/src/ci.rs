use crate::{Error, Result};
use gatekeeper_model::{Host, Millis, Plan, Platform, Report, Toolchain, artifact, job_label};
use serde_json::{Map, Value, json};
use std::fmt::Write as _;
use std::path::Path;

/// `12.3s`.
#[must_use]
pub fn secs(t: Millis) -> String {
    format!("{}.{}s", t.0 / 1000, t.0 % 1000 / 100)
}

/// The host a platform's runner label names: self-hosted when the label is or lists `self-hosted`.
#[must_use]
pub fn host(runners: &Map<String, Value>, platform: Platform) -> Host {
    let label = runners.get(platform.runner());
    let listed = label.and_then(Value::as_array).into_iter().flatten();
    if label.and_then(Value::as_str) == Some("self-hosted")
        || listed.clone().any(|l| l == "self-hosted")
    {
        Host::SelfHosted
    } else {
        Host::GithubHosted { warm_cache: true }
    }
}

/// # Errors
pub fn matrix(plan: &Plan, runners: &Map<String, Value>, checkout: bool) -> Result<String> {
    let (mut first, mut after) = (Vec::new(), Vec::new());
    for job in &plan.jobs {
        let key = job.platform.runner();
        let runs_on = runners
            .get(key)
            .ok_or_else(|| Error(format!("runners: no label for {key}")))?;
        let rust = checkout || job.setup.contains(&Toolchain::Rust);
        let shard = if job.shard.of.get() == 1 {
            String::new()
        } else {
            job.shard.to_string()
        };
        let entry = json!({
            "leg": job.leg, "shard": shard, "artifact": artifact(&job.leg, job.shard),
            "after": job.after.clone().unwrap_or_default(), "runs-on": runs_on, "rust": rust,
        });
        if job.after.is_some() {
            &mut after
        } else {
            &mut first
        }
        .push(entry);
    }
    Ok(format!(
        "matrix={}\nafter={}",
        json!({ "include": first }),
        json!({ "include": after })
    ))
}

/// # Errors
pub fn done(plan: &Plan, reports: &Path) -> Result<(String, Vec<String>)> {
    let mut table = String::from(
        "| job | host | setup | gate | took | budget | |\n|---|---|---|---|---|---|---|\n",
    );
    let mut problems = Vec::new();
    let mut selected = String::new();
    for job in &plan.jobs {
        let path = reports
            .join(artifact(&job.leg, job.shard))
            .join("report.json");
        let label = job_label(&job.leg, job.shard);
        let Ok(text) = std::fs::read_to_string(&path) else {
            problems.push(format!("job {label}: no report ({})", path.display()));
            continue;
        };
        let report: Report = serde_json::from_str(&text).map_err(|e| Error::at(&path, e))?;
        if let (true, Some(affected)) = (selected.is_empty(), &report.affected) {
            writeln!(selected, "\nscope: {affected}\n").map_err(|e| Error(e.to_string()))?;
        }
        for jr in &report.jobs {
            for g in jr.gates.iter().filter(|g| !g.projects.is_empty()) {
                let line = g.projects.join(" ");
                writeln!(selected, "- selected by {} in {label}: {line}", g.id)
                    .map_err(|e| Error(e.to_string()))?;
            }
            for g in &jr.gates {
                writeln!(
                    table,
                    "| {} | {} | {} | {} | {} | {} s | {} |",
                    job_label(&jr.leg, jr.shard),
                    jr.host.label(),
                    secs(jr.setup),
                    g.id,
                    secs(g.took),
                    g.budget.0,
                    g.outcome.word()
                )
                .map_err(|e| Error(e.to_string()))?;
                if g.outcome.fails() {
                    problems.push(format!("{}: {}", g.id, g.outcome.word()));
                }
            }
        }
    }
    table.push_str(&selected);
    Ok((table, problems))
}
