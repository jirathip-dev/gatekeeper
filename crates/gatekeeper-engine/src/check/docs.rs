use crate::arch::{config, frozen};
use crate::docs::scope::{self, Class};
use crate::docs::{names, paths, text, watch};
use crate::run::CheckCx as Cx;
use crate::{Error, Result};
use std::fs;

/// Docs name real paths, links and names, and a watched key changes with a doc line; records get the link check only.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let listed = config::generated(cx.root)?;
    let docs = scope::docs(cx.root, cx.docs);
    let (sets, mut bad) = names::read(cx.root, cx.docs.names.as_ref(), &listed)?;
    bad.extend(scope::unmatched(cx.docs, &docs));
    for doc in &docs {
        let text = text::read(cx.root, doc)?;
        if doc.class != Class::Record {
            bad.extend(paths::stale(cx.root, &cx.docs.skip, doc, &text));
            bad.extend(names::unknown(&sets, &cx.docs.skip, doc, &text));
        }
        bad.extend(paths::broken_links(cx.root, doc, &text));
    }
    let body = cx.pr_body.map(fs::read_to_string).transpose();
    let body = body.map_err(|e| Error(format!("the PR body: {e}")))?;
    if let (Some(base), true) = (cx.base, body.is_some_and(|b| !b.contains("no doc:"))) {
        bad.extend(watch::undocumented(
            cx.docs,
            &docs,
            &frozen::diff(cx.root, base)?,
        ));
    }
    cx.note
        .set(format!(
            "docs: {} files, {} findings",
            docs.len(),
            bad.len()
        ))
        .ok();
    Ok(bad)
}
