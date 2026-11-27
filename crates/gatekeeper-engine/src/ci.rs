use crate::{Error, Result, health};
use gatekeeper_model::{
    CacheMode, CacheNote, CacheTotals, GateId, GateReport, Host, JobReport, Millis, Outcome, Plan,
    Platform, Report, Selected, Tier, Timings, Toolchain, artifact, job_label,
};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// The prefix of every timings entry; a push to the integration branch saves one under it.
pub const TIMINGS: &str = "gatekeeper-timings-1-";

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
    (mode, timings, save): (&CacheMode, &str, bool),
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
            let selects = job.gates.iter().any(|g| g.gate.select.is_some());
            let covers = job.gates.iter().any(|g| g.gate.coverage.is_some());
            if covers || selects && job.shard.of.get() > 1 {
                e.insert("timings".into(), timings.into());
            }
            if covers {
                e.insert("coverage".into(), true.into());
            }
        }
        entries.push(entry);
    }
    let (after, first): (Vec<_>, Vec<_>) = entries.into_iter().partition(|e| e["after"] != "");
    Ok(format!(
        "matrix={}\nafter={}\ntimings-save={}",
        json!({ "include": first }),
        json!({ "include": after }),
        if save { TIMINGS } else { "" }
    ))
}

#[derive(Default)]
struct Sliced {
    reports: BTreeMap<GateId, usize>,
    selected: BTreeMap<GateId, Vec<(String, Selected)>>,
    times: BTreeMap<GateId, Timings>,
}

impl Sliced {
    fn add(&mut self, label: String, g: &GateReport) {
        *self.reports.entry(g.id.clone()).or_default() += 1;
        if let Some(s) = &g.selection {
            let shards = self.selected.entry(g.id.clone()).or_default();
            shards.push((label, s.clone()));
        }
        if !g.files.is_empty() {
            let times = self.times.entry(g.id.clone()).or_default();
            times.schema = 1;
            times.files.extend(g.files.clone());
        }
    }

    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (id, shards) in &self.selected {
            let short = |(label, s): &(String, Selected)| {
                format!("{label} {}", s.digest.get(..8).unwrap_or(&s.digest))
            };
            let first = shards.first().map(|(_, s)| s);
            if shards
                .iter()
                .any(|(_, s)| Some(&s.digest) != first.map(|f| &f.digest))
            {
                let each: Vec<String> = shards.iter().map(short).collect();
                problems.push(format!(
                    "{id}: its shards selected different lists ({}); rerun all jobs",
                    each.join(", ")
                ));
                continue;
            }
            let ran: u32 = shards.iter().map(|(_, s)| s.slice).sum();
            let every = self.reports.get(id) == Some(&shards.len());
            if let (true, Some(f)) = (every, first.filter(|f| f.selected != ran)) {
                problems.push(format!(
                    "{id}: its shards ran {ran} of the {} selected test files; rerun all jobs",
                    f.selected
                ));
            }
        }
        problems
    }

    fn save(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir).map_err(|e| Error::at(dir, e))?;
        for (id, times) in &self.times {
            let path = dir.join(format!("{}.json", id.file_stem()));
            let json = serde_json::to_string_pretty(times).map_err(|e| Error(e.to_string()))?;
            std::fs::write(&path, json).map_err(|e| Error::at(&path, e))?;
        }
        Ok(())
    }
}

fn row(table: &mut String, jr: &JobReport, g: &GateReport) -> Result<()> {
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
        match &g.outcome {
            Outcome::Off(why) => format!("off: {why}"),
            _ => g.note.clone().unwrap_or_default(),
        },
        proved(g)
    )
    .map_err(|e| Error(e.to_string()))
}

/// # Errors
pub fn done(
    plan: &Plan,
    reports: &Path,
    (entry, timings_out): (Option<&Path>, Option<&Path>),
) -> Result<(String, Vec<String>)> {
    let mut table = String::from(
        "| job | host | setup | gate | took | budget | | note | proved by |\n|---|---|---|---|---|---|---|---|---|\n",
    );
    let mut problems = Vec::new();
    let mut selected = String::new();
    let mut jobs = Vec::new();
    let mut sliced = Sliced::default();
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
                sliced.add(job_label(&jr.leg, jr.shard), g);
                row(&mut table, jr, g)?;
                if g.outcome.fails() {
                    problems.push(format!("{}: {}", g.id, g.outcome.word()));
                }
            }
        }
        jobs.extend(report.jobs);
    }
    problems.extend(sliced.problems());
    if let Some(dir) = timings_out {
        sliced.save(dir)?;
    }
    if let Some(line) = cache_line(&CacheTotals::of(jobs.iter().flat_map(|j| &j.gates))) {
        writeln!(table, "\ncache: {line}").ok();
    }
    writeln!(
        table,
        "\n{}",
        health::entry::done(&jobs, entry, timings_out)?
    )
    .ok();
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
        (_, Some(note @ CacheNote::Projects(_))) => note.split().unwrap_or_default(),
        _ => String::new(),
    }
}
