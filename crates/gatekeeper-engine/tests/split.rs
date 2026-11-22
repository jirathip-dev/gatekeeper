use gatekeeper_engine::select::split;
use gatekeeper_model::{Millis, Shard, TestFile, Timings};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::num::NonZeroU32;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    #[serde(rename = "about")]
    _about: String,
    shards: NonZeroU32,
    files: BTreeSet<TestFile>,
    #[serde(default)]
    timings: BTreeMap<TestFile, u64>,
    slices: Vec<Vec<TestFile>>,
}

fn slices(files: &BTreeSet<TestFile>, timings: &Timings, of: NonZeroU32) -> Vec<Vec<TestFile>> {
    let shards = (1..=of.get()).filter_map(NonZeroU32::new);
    shards
        .map(|index| split(files, timings, Shard { index, of }))
        .collect()
}

#[test]
fn each_split_case_slices_exactly_as_its_folder_expects() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/split");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).expect("tests/split") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        let text = fs::read_to_string(path.join("case.toml")).expect("case.toml");
        let case: Case = toml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let files = case.timings.into_iter().map(|(f, ms)| (f, Millis(ms)));
        let timings = Timings {
            schema: 1,
            files: files.collect(),
        };
        let got = slices(&case.files, &timings, case.shards);
        assert_eq!(got, case.slices, "{name}");
        seen += 1;
    }
    assert_eq!(seen, 5);
}

#[test]
fn for_any_timings_the_shards_are_a_partition() {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move |n: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state % n
    };
    for _ in 0..500 {
        let files: BTreeSet<TestFile> = (0..next(40))
            .map(|i| TestFile(format!("t{i}.test.ts")))
            .collect();
        let timed = files.iter().filter(|_| next(3) > 0);
        let timed: Vec<TestFile> = timed.cloned().collect();
        let timings = Timings {
            schema: 1,
            files: timed.into_iter().map(|f| (f, Millis(next(5000)))).collect(),
        };
        let of = NonZeroU32::new(u32::try_from(next(8) + 1).expect("u32")).expect("non-zero");
        let got = slices(&files, &timings, of);
        let total: usize = got.iter().map(Vec::len).sum();
        let union: BTreeSet<TestFile> = got.into_iter().flatten().collect();
        assert_eq!(union, files, "{timings:?} over {of}");
        assert_eq!(total, files.len(), "a file fell in two shards: {timings:?}");
    }
}
