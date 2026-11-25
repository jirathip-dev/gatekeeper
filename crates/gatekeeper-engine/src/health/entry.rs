use crate::{Error, Result};
use gatekeeper_model::{Covered, Health, JobReport, Millis};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

const FILE: &str = "health.json";

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
    let mut coverage = was.coverage;
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
    let legs = format!("legs: {}", each.join(" · "));
    if said.is_empty() {
        return Ok(legs);
    }
    Ok(format!("{legs}\ncoverage: {}", said.join(" · ")))
}
