use gatekeeper_model::{Case, Runner};
use std::collections::BTreeSet;

/// More tests than this in the rerun set run the whole line again.
pub const MOST: usize = 200;

fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// The test files of the rerun set, for a `[select]` gate's `{files}`.
#[must_use]
pub fn files(again: &[&Case], root: &str) -> Vec<String> {
    let files: BTreeSet<String> = again.iter().map(|c| under(root, &c.class)).collect();
    files.into_iter().collect()
}

/// A `JUnit` `classname` of vitest as a repo path: it is relative to `[timed] root`.
#[must_use]
pub fn under(root: &str, class: &str) -> String {
    match root.trim_end_matches('/') {
        "" | "." => class.to_owned(),
        root => format!("{root}/{class}"),
    }
}

/// `{only}`: the runner's own filter for exactly these tests; empty for none or over [`MOST`].
#[must_use]
pub fn only(runner: Runner, again: &[&Case]) -> String {
    if again.is_empty() || again.len() > MOST {
        return String::new();
    }
    match runner {
        Runner::Nextest => {
            let each: Vec<String> = again
                .iter()
                .map(|c| format!("(binary_id(={}) & test(={}))", c.class, c.name))
                .collect();
            format!("-E {}", quoted(&each.join(" | ")))
        }
        Runner::Vitest => {
            let files: BTreeSet<&str> = again.iter().map(|c| c.class.as_str()).collect();
            let each: Vec<String> = files.into_iter().map(quoted).collect();
            each.join(" ")
        }
    }
}
