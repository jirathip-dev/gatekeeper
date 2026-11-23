use crate::Result;
use crate::arch::{baseline, comments, config, docs, frozen, generated, scan};
use crate::run::CheckCx as Cx;
use std::fs;

/// `.drover/arch.toml`'s layers, bans, sizes, frozen, generated and shared files, comments and docs; sizes and crossings in its baseline pass.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let Some(a) = config::load(cx.root)? else {
        cx.note
            .set(format!("no {}: nothing to check", config::PATH))
            .ok();
        return Ok(Vec::new());
    };
    let listed = baseline::read(cx.root, a.baseline());
    let scan = scan::scan(cx.root, &a, &listed);
    let (diff, was) = match cx.base {
        Some(b) => (frozen::diff(cx.root, b)?, frozen::at_base(cx.root, b)?),
        None => (Vec::new(), Vec::new()),
    };
    let mut bad = scan.bad;
    bad.extend(frozen::adds(&a, &was, &diff));
    bad.extend(generated::unsized_shared(&a));
    bad.extend(generated::stale(cx.root, &a)?);
    bad.extend(comments::found(cx.root, &a, &diff, &scan.files));
    if let Some(d) = &a.docs {
        bad.extend(docs::stale(cx.root, d, &scan.files));
        let body = cx.pr_body.map(fs::read_to_string).transpose();
        let body = body.map_err(|e| crate::Error(format!("the PR body: {e}")))?;
        if body.is_some_and(|b| !b.contains("no doc:")) {
            bad.extend(docs::undocumented(d, &diff));
        }
    }
    let files = scan.files.len();
    cx.note
        .set(format!("arch: {files} files, {} findings", bad.len()))
        .ok();
    Ok(bad)
}
