use super::scope::{Class, Doc};
use crate::arch::any;
use crate::arch::frozen::Change;
use gatekeeper_model::DocsConfig;
use std::collections::BTreeSet;

pub(crate) fn undocumented(d: &DocsConfig, docs: &[Doc], diff: &[Change]) -> Vec<String> {
    let doc = |f: &str| docs.iter().any(|x| x.path == f && x.class != Class::Record);
    let named = |n: &str| diff.iter().any(|c| doc(&c.file) && c.text.contains(n));
    let key = |n: &str| {
        !n.is_empty()
            && n.bytes()
                .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
    };
    let (mut seen, mut bad) = (BTreeSet::new(), vec![]);
    for c in diff.iter().filter(|c| any(&d.watch, &c.file)) {
        let t = c.text.trim_start().trim_start_matches("pub ");
        let Some((name, _)) = t
            .split_once(':')
            .filter(|(n, r)| key(n) && !r.starts_with(':'))
        else {
            continue;
        };
        if seen.insert(name) && !named(name) && !named(&name.replace('_', "-")) {
            let (f, how) = (&c.file, if c.added { "added" } else { "removed" });
            bad.push(format!(
                "{f}: `{name}` {how}, but no changed doc line names it (or `no doc: <why>` in the PR body)"
            ));
        }
    }
    bad
}
