use gatekeeper_model::{Key, KeyParts, Record, Refusal, Tier};
use sha2::{Digest, Sha256};
use std::num::NonZeroU32;

pub(crate) enum Lookup {
    Miss,
    Refused(Tier, Refusal),
    Hit(Record),
    Recheck(Tier, Record),
}

/// How old a record may be, how often a hit is rerun, and what picks the sample.
pub(crate) struct Policy<'a> {
    pub now: u64,
    pub max_age_secs: u64,
    pub recheck: NonZeroU32,
    pub salt: &'a str,
}

fn judge(tier: Tier, key: &Key, parts: &KeyParts, record: &Record, p: &Policy) -> Option<Refusal> {
    if record.parts != *parts {
        Some(Refusal::PartsDiffer)
    } else if record.key != *key {
        Some(Refusal::KeyMismatch)
    } else if record.proof.tier != tier {
        Some(Refusal::WrongTier)
    } else if p.now.saturating_sub(record.proof.at) > p.max_age_secs {
        Some(Refusal::Expired)
    } else {
        None
    }
}

fn sampled(key: &Key, p: &Policy) -> bool {
    let digest = Sha256::digest(format!("{}{}", key.0, p.salt));
    let mut word = [0u8; 4];
    word.copy_from_slice(digest.get(..4).unwrap_or(&[0; 4]));
    u32::from_be_bytes(word) % p.recheck == 0
}

/// The first trusted record wins; a refused one is reported only when none is trusted.
pub(crate) fn lookup(
    key: &Key,
    parts: &KeyParts,
    found: Vec<(Tier, Option<Record>)>,
    p: &Policy,
) -> Lookup {
    let mut refused = None;
    for (tier, record) in found {
        let Some(record) = record else {
            refused.get_or_insert((tier, Refusal::Unreadable));
            continue;
        };
        match judge(tier, key, parts, &record, p) {
            Some(why) => {
                refused.get_or_insert((tier, why));
            }
            None if sampled(key, p) => return Lookup::Recheck(tier, record),
            None => return Lookup::Hit(record),
        }
    }
    refused.map_or(Lookup::Miss, |(tier, why)| Lookup::Refused(tier, why))
}
