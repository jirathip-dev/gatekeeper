use gatekeeper_model::{Date, Flaky, TestId};
use std::fs;
use std::path::Path;

pub(crate) const DIR: &str = "ci/tests/flaky.d";

pub(crate) fn read(root: &Path) -> Vec<(String, Result<Flaky, String>)> {
    let mut out = Vec::new();
    for e in fs::read_dir(root.join(DIR)).into_iter().flatten().flatten() {
        let path = e.path();
        if path.extension().is_none_or(|x| x != "toml") {
            continue;
        }
        let at = format!("{DIR}/{}", e.file_name().to_string_lossy());
        let text = fs::read_to_string(&path).map_err(|e| e.to_string());
        out.push((
            at,
            text.and_then(|t| toml::from_str(&t).map_err(|e| e.message().to_owned())),
        ));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

pub(crate) fn entries(root: &Path) -> Vec<Flaky> {
    read(root).into_iter().filter_map(|(_, e)| e.ok()).collect()
}

/// The entry that quarantines `test` on `today`: its failures are warnings.
#[must_use]
pub fn quarantined<'a>(entries: &'a [Flaky], test: &TestId, today: Date) -> Option<&'a Flaky> {
    entries
        .iter()
        .find(|f| f.test == *test && f.since <= today && today <= f.until())
}

/// `demo-app tests::adds` → `demo-app-tests-adds`: an entry's file name.
#[must_use]
pub fn slug(test: &TestId) -> String {
    let each = test.0.split(|c: char| !c.is_ascii_alphanumeric());
    let words: Vec<String> = each
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    words.join("-")
}
