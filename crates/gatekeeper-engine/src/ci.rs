use crate::{Error, Result};
use gatekeeper_model::{Millis, Plan, Report, Toolchain};
use serde_json::{Map, Value, json};
use std::fmt::Write as _;
use std::path::Path;

/// `12.3s`.
#[must_use]
pub fn secs(t: Millis) -> String {
    format!("{}.{}s", t.0 / 1000, t.0 % 1000 / 100)
}

/// # Errors
pub fn matrix(plan: &Plan, runners: &Map<String, Value>, checkout: bool) -> Result<Value> {
    let include = plan
        .jobs
        .iter()
        .map(|job| {
            let key = job.platform.runner();
            let runs_on = runners
                .get(key)
                .ok_or_else(|| Error(format!("runners: no label for {key}")))?;
            let rust = checkout || job.setup.contains(&Toolchain::Rust);
            let node = job.setup.contains(&Toolchain::Node);
            Ok(json!({ "leg": job.leg, "runs-on": runs_on, "rust": rust, "node": node }))
        })
        .collect::<Result<Vec<Value>>>()?;
    Ok(json!({ "include": include }))
}

/// # Errors
pub fn done(plan: &Plan, reports: &Path) -> Result<(String, Vec<String>)> {
    let mut table = String::from(
        "| job | host | setup | gate | took | budget | | note |\n|---|---|---|---|---|---|---|---|\n",
    );
    let mut problems = Vec::new();
    for job in &plan.jobs {
        let path = reports
            .join(format!("gates-{}", job.leg))
            .join("report.json");
        let Ok(text) = std::fs::read_to_string(&path) else {
            problems.push(format!("job {}: no report ({})", job.leg, path.display()));
            continue;
        };
        let report: Report = serde_json::from_str(&text).map_err(|e| Error::at(&path, e))?;
        for jr in &report.jobs {
            for g in &jr.gates {
                writeln!(
                    table,
                    "| {} | {} | {} | {} | {} | {} s | {} | {} |",
                    jr.leg,
                    jr.host.label(),
                    secs(jr.setup),
                    g.id,
                    secs(g.took),
                    g.budget.0,
                    g.outcome.word(),
                    g.note.as_deref().unwrap_or_default()
                )
                .map_err(|e| Error(e.to_string()))?;
                if g.outcome.fails() {
                    problems.push(format!("{}: {}", g.id, g.outcome.word()));
                }
            }
        }
    }
    Ok((table, problems))
}
