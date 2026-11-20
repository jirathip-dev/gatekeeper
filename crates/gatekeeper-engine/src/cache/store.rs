use crate::{Error, Result};
use gatekeeper_model::{GateReport, Key, KeyParts, Outcome, Proof, Record, Tier};
use std::fs;
use std::path::{Path, PathBuf};

/// A pass that ran: the only thing `write` takes, built only from `Outcome::Passed`.
pub(crate) struct Ran(Record);

impl Ran {
    pub(crate) fn of(report: &GateReport, key: Key, parts: KeyParts, proof: Proof) -> Option<Ran> {
        matches!(report.outcome, Outcome::Passed { .. }).then(|| {
            Ran(Record {
                key,
                parts,
                proof,
                took: report.took,
            })
        })
    }
}

fn path(store: &Path, tier: Tier, key: &Key) -> PathBuf {
    store.join(tier.to_string()).join(format!("{}.json", key.0))
}

/// The record at `key` in `tier`, `None` when there is none.
pub(crate) fn read(store: &Path, tier: Tier, key: &Key) -> Option<serde_json::Result<Record>> {
    let text = fs::read_to_string(path(store, tier, key)).ok()?;
    Some(serde_json::from_str(&text))
}

pub(crate) fn write(store: &Path, ran: &Ran) -> Result<()> {
    let to = path(store, ran.0.proof.tier, &ran.0.key);
    let part = to.with_extension("part");
    if let Some(dir) = to.parent() {
        fs::create_dir_all(dir).map_err(|e| Error::at(dir, e))?;
    }
    let json = serde_json::to_string_pretty(&ran.0).map_err(|e| Error(e.to_string()))?;
    fs::write(&part, json).map_err(|e| Error::at(&part, e))?;
    fs::rename(&part, &to).map_err(|e| Error::at(&to, e))
}

pub(crate) fn remove(store: &Path, tier: Tier, key: &Key) {
    fs::remove_file(path(store, tier, key)).ok();
}
