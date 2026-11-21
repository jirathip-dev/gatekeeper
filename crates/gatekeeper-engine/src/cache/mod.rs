pub mod inputs;
pub mod key;
mod lookup;
pub mod mode;
mod store;
pub mod versions;

pub use inputs::Inputs;
pub use mode::mode;
pub use versions::Ident;

use crate::Result;
use crate::run::unrun;
use gatekeeper_model::{
    CacheConfig, CacheNote, Failure, Gate, GateReport, Hit, Key, KeyParts, Millis, Outcome,
    Planned, ProjectCache, Proof, Refusal, RunRef, Shard, Tier, Unkeyed, Workspace,
};
use key::Keyed;
use lookup::Lookup;
use std::path::{Path, PathBuf};
use store::Ran;

/// A run with the cache on: the store, the tiers `mode` gave it, and what a key and a lookup need.
pub struct Results {
    pub store: PathBuf,
    pub read: Vec<Tier>,
    pub write: Option<Tier>,
    pub config: CacheConfig,
    pub now: u64,
    pub salt: String,
    pub run: Option<RunRef>,
    pub ident: Ident,
    pub inputs: Inputs,
}

/// Whether a gate keeps one record per project: a `projects` gate run unsharded.
#[must_use]
pub fn per_project(gate: &Gate, shard: Shard) -> bool {
    !gate.projects.is_empty() && shard.of == Shard::ONE.of
}

fn find(res: &Results, key: &Key, parts: &KeyParts) -> (Option<Tier>, ProjectCache) {
    let found = res
        .read
        .iter()
        .filter_map(|t| Some((*t, store::read(&res.store, *t, key)?.ok())));
    match lookup::lookup(key, parts, found.collect(), res) {
        Lookup::Hit(record) => {
            let (proof, took) = (record.proof, record.took);
            (None, ProjectCache::Cached(Hit { proof, took }))
        }
        Lookup::Miss => (None, ProjectCache::Ran),
        Lookup::Refused(tier, why) => {
            if why == Refusal::Expired {
                store::remove(&res.store, tier, key);
            }
            (None, ProjectCache::Refused(why))
        }
        Lookup::Recheck(tier, r) => (Some(tier), ProjectCache::Rechecked(r.proof)),
    }
}

struct Unit<'a> {
    projects: &'a [String],
    key: Key,
    parts: KeyParts,
    recheck: Option<Tier>,
    note: ProjectCache,
}

fn units<'a>(
    res: &Results,
    planned: &'a Planned,
    (shard, ws): (Shard, &Workspace),
    root: &Path,
) -> Result<std::result::Result<Vec<Unit<'a>>, Unkeyed>> {
    let units: Vec<&[String]> = if per_project(&planned.gate, shard) {
        planned.projects.iter().map(std::slice::from_ref).collect()
    } else {
        vec![&planned.projects[..]]
    };
    let mut found = Vec::new();
    for projects in units {
        let at = (shard, projects);
        match key::keyed(&planned.gate, at, &res.ident, (&res.inputs, ws), root)? {
            Keyed::No(why) => return Ok(Err(why)),
            Keyed::Key(key, parts) => {
                let (recheck, note) = find(res, &key, &parts);
                let parts = *parts;
                found.push(Unit {
                    projects,
                    key,
                    parts,
                    recheck,
                    note,
                });
            }
        }
    }
    Ok(Ok(found))
}

fn note(found: &[Unit], each: bool) -> Option<CacheNote> {
    let notes = || {
        let notes = found.iter().map(|u| (u.projects.concat(), u.note.clone()));
        Some(CacheNote::Projects(notes.collect()))
    };
    match found.first().map(|u| &u.note) {
        _ if each => notes(),
        Some(ProjectCache::Refused(why)) => Some(CacheNote::Refused(*why)),
        Some(ProjectCache::Rechecked(proof)) => Some(CacheNote::Rechecked(proof.clone())),
        _ => None,
    }
}

fn all_hit(found: &[Unit]) -> Option<Hit> {
    let hits = found.iter().filter_map(|u| match &u.note {
        ProjectCache::Cached(hit) => Some(hit),
        _ => None,
    });
    let hits: Vec<&Hit> = hits.collect();
    let first = hits.first().filter(|_| hits.len() == found.len())?;
    let took = Millis(hits.iter().map(|h| h.took.0).sum());
    let proof = first.proof.clone();
    Some(Hit { proof, took })
}

pub(crate) fn gate(
    res: &Results,
    planned: &Planned,
    at: (Shard, &Workspace),
    root: &Path,
    run: impl FnOnce(&[String]) -> Result<GateReport>,
) -> Result<GateReport> {
    let mut found = match units(res, planned, at, root)? {
        Ok(found) => found,
        Err(why) => {
            let report = run(&planned.projects)?;
            let cache = Some(CacheNote::Unkeyed(why));
            return Ok(GateReport { cache, ..report });
        }
    };
    let cache = note(&found, per_project(&planned.gate, at.0));
    if let Some(hit) = all_hit(&found) {
        return Ok(GateReport {
            projects: planned.projects.clone(),
            cache,
            ..unrun(planned, Outcome::Cached(hit))
        });
    }
    found.retain(|u| !matches!(u.note, ProjectCache::Cached(_)));
    let ran: Vec<String> = found.iter().flat_map(|u| u.projects.to_vec()).collect();
    let report = GateReport {
        cache,
        ..run(&ran)?
    };
    settle(res, report, found)
}

fn settle(res: &Results, mut report: GateReport, ran: Vec<Unit>) -> Result<GateReport> {
    if matches!(report.outcome, Outcome::Failed(_)) {
        for u in &ran {
            if let Some(tier) = u.recheck {
                store::remove(&res.store, tier, &u.key);
            }
        }
        let proofs = ran.iter().map(|u| match &u.note {
            ProjectCache::Rechecked(proof) => Some(proof.clone()),
            _ => None,
        });
        if let Some(Some(proof)) = proofs.reduce(Option::and) {
            report.outcome = Outcome::Failed(Failure::CacheMismatch(proof));
        }
    }
    let took = Millis(report.took.0 / u64::try_from(ran.len()).unwrap_or(1).max(1));
    for u in ran {
        record(res, &report, took, u.key, u.parts)?;
    }
    Ok(report)
}

fn record(
    res: &Results,
    report: &GateReport,
    took: Millis,
    key: Key,
    parts: KeyParts,
) -> Result<()> {
    let Some(tier) = res.write else {
        return Ok(());
    };
    let (commit, run) = (res.inputs.commit.clone(), res.run.clone());
    let proof = Proof {
        tier,
        at: res.now,
        commit,
        run,
    };
    match Ran::of(report, took, key, parts, proof) {
        Some(ran) => store::write(&res.store, &ran),
        None => Ok(()),
    }
}
