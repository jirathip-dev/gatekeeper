use super::flaky::slug;
use crate::{Error, Result};
use gatekeeper_model::{Covered, Health, JobReport, Millis, Sighting};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

const FILE: &str = "health.json";
const FLAKY: &str = "flaky.json";

fn entry_file(s: &Sighting) -> Result<String> {
    let mut t = toml::Table::new();
    t.insert("test".into(), s.test.0.clone().into());
    t.insert("since".into(), s.on.to_string().into());
    t.insert("sha".into(), s.sha.clone().into());
    toml::to_string(&t).map_err(|e| Error(e.to_string()))
}

fn sightings(dir: &Path, seen: &[&Sighting]) -> Result<()> {
    let each = dir.join("flaky");
    std::fs::remove_dir_all(&each).ok();
    std::fs::remove_file(dir.join(FLAKY)).ok();
    if seen.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(&each).map_err(|e| Error::at(&each, e))?;
    for s in seen {
        let path = each.join(format!("{}.toml", slug(&s.test)));
        std::fs::write(&path, entry_file(s)?).map_err(|e| Error::at(&path, e))?;
    }
    let path = dir.join(FLAKY);
    let json = serde_json::to_string_pretty(seen).map_err(|e| Error(e.to_string()))?;
    std::fs::write(&path, json).map_err(|e| Error::at(&path, e))
}

fn clock(t: Millis) -> String {
    let s = t.0 / 1000;
    format!("{}m{:02}s", s / 60, s % 60)
}

fn leg(name: &str, now: Millis, was: Option<&Millis>) -> String {
    let mut text = format!("{name} {}", clock(now));
    if let Some(&was) = was {
        write!(text, " (integration {})", clock(was)).ok();
        if now.0 * 5 > was.0 * 6 && now.0 > was.0 + 30_000 {
            write!(text, " WARN +{}%", (now.0 - was.0) * 100 / was.0.max(1)).ok();
        }
    }
    text
}

fn covered(jobs: &[JobReport], mut coverage: Vec<Covered>) -> (Vec<Covered>, Vec<String>) {
    let mut said = Vec::new();
    for g in jobs.iter().flat_map(|j| &j.gates) {
        let Some(lines) = g.coverage else { continue };
        let before = coverage.iter().position(|c| c.gate == g.id);
        let old = before.map(|i| coverage.remove(i).lines);
        let vs = old.map_or_else(
            || "no integration number".into(),
            |o| format!("integration {o} %"),
        );
        said.push(format!("{} {lines} % ({vs})", g.id));
        coverage.push(Covered {
            gate: g.id.clone(),
            lines,
        });
    }
    (coverage, said)
}

pub(crate) fn done(jobs: &[JobReport], entry: Option<&Path>, out: Option<&Path>) -> Result<String> {
    let mut legs = BTreeMap::<String, Millis>::new();
    for j in jobs {
        let took = j.setup.0 + j.gates.iter().map(|g| g.took.0).sum::<u64>();
        let slowest = legs.entry(j.leg.clone()).or_default();
        slowest.0 = slowest.0.max(took);
    }
    let read = entry.and_then(|d| std::fs::read_to_string(d.join(FILE)).ok());
    let was: Health = read
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let each: Vec<String> = legs
        .iter()
        .map(|(n, t)| leg(n, *t, was.legs.get(n)))
        .collect();
    let (coverage, said) = covered(jobs, was.coverage);
    if let Some(dir) = out {
        let path = dir.join(FILE);
        let health = Health {
            schema: 1,
            coverage,
            legs,
        };
        let json = serde_json::to_string_pretty(&health).map_err(|e| Error(e.to_string()))?;
        std::fs::create_dir_all(dir).map_err(|e| Error::at(dir, e))?;
        std::fs::write(&path, json).map_err(|e| Error::at(&path, e))?;
    }
    let seen: Vec<&Sighting> = jobs
        .iter()
        .flat_map(|j| &j.gates)
        .flat_map(|g| &g.flaky)
        .collect();
    let mut text = format!("legs: {}", each.join(" · "));
    if !said.is_empty() {
        write!(text, "\ncoverage: {}", said.join(" · ")).ok();
    }
    for s in &seen {
        write!(
            text,
            "\nflaky at {}: {} in {} failed, then passed {} of 2 reruns\n  list it: {}/{}.toml (in the artifact gates-done, under flaky/)",
            s.sha.get(..7).unwrap_or(&s.sha),
            s.test,
            s.gate,
            s.passed_on_rerun,
            super::flaky::DIR,
            slug(&s.test)
        )
        .ok();
    }
    if let Some(dir) = out {
        sightings(dir, &seen)?;
    }
    Ok(text)
}
