use super::Results;
use gatekeeper_model::{Key, KeyParts, Record, Refusal, Tier};
use sha2::{Digest, Sha256};

pub(crate) enum Lookup {
    Miss,
    Refused(Tier, Refusal),
    Hit(Record),
    Recheck(Tier, Record),
}

fn judge(
    tier: Tier,
    key: &Key,
    parts: &KeyParts,
    record: &Record,
    res: &Results,
) -> Option<Refusal> {
    let max_age = u64::from(res.config.max_age_days.get()) * 86_400;
    [
        (record.parts != *parts, Refusal::PartsDiffer),
        (record.key != *key, Refusal::KeyMismatch),
        (record.proof.tier != tier, Refusal::WrongTier),
        (
            res.now.saturating_sub(record.proof.at) > max_age,
            Refusal::Expired,
        ),
    ]
    .into_iter()
    .find_map(|(bad, why)| bad.then_some(why))
}

fn sampled(key: &Key, res: &Results) -> bool {
    let digest = Sha256::digest(format!("{}{}", key.0, res.salt));
    let mut word = [0u8; 4];
    word.copy_from_slice(digest.get(..4).unwrap_or(&[0; 4]));
    u32::from_be_bytes(word) % res.config.recheck == 0
}

pub(crate) fn lookup(
    key: &Key,
    parts: &KeyParts,
    found: Vec<(Tier, Option<Record>)>,
    res: &Results,
) -> Lookup {
    let mut refused = None;
    for (tier, record) in found {
        let why = record.as_ref().map_or(Some(Refusal::Unreadable), |r| {
            judge(tier, key, parts, r, res)
        });
        match (why, record) {
            (Some(why), _) => {
                refused.get_or_insert((tier, why));
            }
            (None, Some(record)) if sampled(key, res) => return Lookup::Recheck(tier, record),
            (None, Some(record)) => return Lookup::Hit(record),
            (None, None) => {}
        }
    }
    refused.map_or(Lookup::Miss, |(tier, why)| Lookup::Refused(tier, why))
}
