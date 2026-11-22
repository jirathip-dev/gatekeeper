use gatekeeper_model::{Shard, TestFile, Timings};
use std::collections::BTreeSet;

const NO_TIME_MS: u64 = 1000;

/// This shard's slice of `files`: longest first, each to the shard with the least time so far.
#[must_use]
pub fn split(files: &BTreeSet<TestFile>, timings: &Timings, shard: Shard) -> Vec<TestFile> {
    let mut known: Vec<u64> = files
        .iter()
        .filter_map(|f| timings.files.get(f))
        .map(|t| t.0)
        .collect();
    known.sort_unstable();
    let median = known.get(known.len() / 2).copied().unwrap_or(NO_TIME_MS);
    let time = |f: &TestFile| timings.files.get(f).map_or(median, |t| t.0);
    let mut order: Vec<(u64, &TestFile)> = files.iter().map(|f| (time(f), f)).collect();
    order.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let mut load = vec![0_u64; usize::try_from(shard.of.get()).unwrap_or(1)];
    let mut slice = BTreeSet::new();
    for (ms, file) in order {
        let lightest = load.iter().enumerate().min_by_key(|(_, l)| **l);
        let at = lightest.map_or(0, |(i, _)| i);
        if let Some(l) = load.get_mut(at) {
            *l += ms;
        }
        if u32::try_from(at + 1).ok() == Some(shard.index.get()) {
            slice.insert(file.clone());
        }
    }
    slice.into_iter().collect()
}
