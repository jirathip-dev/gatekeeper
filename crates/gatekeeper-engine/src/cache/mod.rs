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
    CacheConfig, CacheNote, Failure, GateReport, Hit, Outcome, Planned, Proof, Refusal, RunRef,
    Shard, Tier,
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

/// One planned gate: skipped on a trusted hit, else run, and recorded when it passed.
pub(crate) fn gate(
    res: &Results,
    planned: &Planned,
    shard: Shard,
    root: &Path,
    run: impl FnOnce() -> Result<GateReport>,
) -> Result<GateReport> {
    let at = (shard, &planned.projects[..]);
    let (key, parts) = match key::keyed(&planned.gate, at, &res.ident, &res.inputs, root)? {
        Keyed::No(why) => {
            let report = run()?;
            let cache = Some(CacheNote::Unkeyed(why));
            return Ok(GateReport { cache, ..report });
        }
        Keyed::Key(key, parts) => (key, parts),
    };
    let found = res
        .read
        .iter()
        .filter_map(|t| Some((*t, store::read(&res.store, *t, &key)?.ok())));
    let (recheck, note) = match lookup::lookup(&key, &parts, found.collect(), res) {
        Lookup::Hit(record) => {
            let hit = Hit {
                proof: record.proof,
                took: record.took,
            };
            let projects = planned.projects.clone();
            return Ok(GateReport {
                projects,
                ..unrun(planned, Outcome::Cached(hit))
            });
        }
        Lookup::Miss => (None, None),
        Lookup::Refused(tier, why) => {
            if why == Refusal::Expired {
                store::remove(&res.store, tier, &key);
            }
            (None, Some(CacheNote::Refused(why)))
        }
        Lookup::Recheck(tier, r) => (
            Some((tier, r.proof.clone())),
            Some(CacheNote::Rechecked(r.proof)),
        ),
    };
    let mut report = GateReport {
        cache: note,
        ..run()?
    };
    if let (Some((tier, proof)), Outcome::Failed(_)) = (recheck, &report.outcome) {
        store::remove(&res.store, tier, &key);
        report.outcome = Outcome::Failed(Failure::CacheMismatch(proof));
    }
    if let Some(tier) = res.write {
        let (commit, run) = (res.inputs.commit.clone(), res.run.clone());
        let proof = Proof {
            tier,
            at: res.now,
            commit,
            run,
        };
        if let Some(ran) = Ran::of(&report, key, *parts, proof) {
            store::write(&res.store, &ran)?;
        }
    }
    Ok(report)
}
