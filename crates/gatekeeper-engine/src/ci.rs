use crate::{Error, Result};
use gatekeeper_model::{
    CacheMode, CacheNote, CacheTotals, GateReport, Host, Millis, Outcome, Plan, Platform, Report,
    Tier, Toolchain, artifact, job_label,
};
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
    if label.into_iter().chain(listed).any(|l| l == "self-hosted") {
        Host::SelfHosted
    } else {
        Host::GithubHosted { warm_cache: true }
    }
}

fn results(mode: &CacheMode, artifact: &str) -> Value {
    let (read, write) = match mode {
        CacheMode::Use { read, write } => (&read[..], *write),
        CacheMode::Off(_) => (&[][..], None),
    };
    let prefix = |t: Tier| format!("gatekeeper-results-1-{t}-{artifact}-");
    let own = read.iter().find(|t| matches!(t, Tier::PullRequest(_)));
    json!({
        "results-integration": if read.contains(&Tier::Integration) { prefix(Tier::Integration) } else { String::new() },
        "results-own": own.map_or_else(String::new, |t| prefix(*t)),
        "results-save": write.map_or_else(String::new, prefix),
        "results-tier": write.map_or_else(String::new, |t| t.to_string()),
    })
}

/// # Errors
pub fn matrix(
    plan: &Plan,
    runners: &Map<String, Value>,
    checkout: bool,
    mode: &CacheMode,
) -> Result<String> {
    let mut entries = Vec::new();
    for job in &plan.jobs {
        let key = job.platform.runner();
        let runs_on = runners
            .get(key)
            .ok_or_else(|| Error(format!("runners: no label for {key}")))?;
        let rust = checkout || job.setup.contains(&Toolchain::Rust);
        let node = job.setup.contains(&Toolchain::Node);
        let shard = (job.shard.of.get() > 1).then(|| job.shard.to_string());
        let name = artifact(&job.leg, job.shard);
        let mut entry = json!({
            "leg": job.leg, "shard": shard.unwrap_or_default(), "artifact": name,
            "after": job.after.clone().unwrap_or_default(), "runs-on": runs_on, "rust": rust, "node": node,
        });
        if let (Some(e), Value::Object(r)) = (entry.as_object_mut(), results(mode, &name)) {
            e.extend(r);
        }
        entries.push(entry);
    }
    let (after, first): (Vec<_>, Vec<_>) = entries.into_iter().partition(|e| e["after"] != "");
    Ok(format!(
        "matrix={}\nafter={}",
        json!({ "include": first }),
        json!({ "include": after })
    ))
}

/// # Errors
pub fn done(plan: &Plan, reports: &Path) -> Result<(String, Vec<String>)> {
    let mut table = String::from(
        "| job | host | setup | gate | took | budget | | note | proved by |\n|---|---|---|---|---|---|---|---|---|\n",
    );
    let mut problems = Vec::new();
    let mut selected = String::new();
    let mut all = Vec::new();
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
            writeln!(selected, "\nscope: {affected}\n").ok();
        }
        for jr in &report.jobs {
            for g in jr.gates.iter().filter(|g| !g.projects.is_empty()) {
                let line = g.projects.join(" ");
                writeln!(selected, "- selected by {} in {label}: {line}", g.id).ok();
            }
            for g in &jr.gates {
                writeln!(
                    table,
                    "| {} | {} | {} | {} | {} | {} s | {} | {} | {} |",
                    job_label(&jr.leg, jr.shard),
                    jr.host.label(),
                    secs(jr.setup),
                    g.id,
                    secs(g.took),
                    g.budget.0,
                    g.outcome.word(),
                    g.note.as_deref().unwrap_or_default(),
                    proved(g)
                )
                .map_err(|e| Error(e.to_string()))?;
                if g.outcome.fails() {
                    problems.push(format!("{}: {}", g.id, g.outcome.word()));
                }
            }
        }
        all.extend(report.jobs.into_iter().flat_map(|j| j.gates));
    }
    if let Some(line) = cache_line(&CacheTotals::of(&all)) {
        writeln!(table, "\ncache: {line}").ok();
    }
    table.push_str(&selected);
    Ok((table, problems))
}

/// `2 of 4 gates cached · ran 1.0s · saved 41.0s (97 %) · 1 rechecked, 0 differed · 0 refused`, when the cache did anything.
#[must_use]
pub fn cache_line(t: &CacheTotals) -> Option<String> {
    let share = t.saved.0 * 100 / (t.ran.0 + t.saved.0).max(1);
    let (ran, saved) = (secs(t.ran), secs(t.saved));
    (t.cached + t.rechecked + t.refused > 0).then(|| format!(
        "{} of {} gates cached · ran {ran} · saved {saved} ({share} %) · {} rechecked, {} differed · {} refused",
        t.cached, t.gates, t.rechecked, t.differed, t.refused
    ))
}

fn proved(g: &GateReport) -> String {
    match (&g.outcome, &g.cache) {
        (Outcome::Cached(hit), _) => hit.proof.to_string(),
        (_, Some(CacheNote::Rechecked(proof))) => format!("rechecked: {proof}"),
        (_, Some(CacheNote::Refused(why))) => format!("refused: {why:?}"),
        _ => String::new(),
    }
}
