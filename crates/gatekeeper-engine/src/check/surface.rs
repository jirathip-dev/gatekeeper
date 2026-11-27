use crate::docs::names;
use crate::ratchet::{folder, judge};
use crate::run::CheckCx as Cx;
use crate::{Error, Result, arch, config, runner};
use gatekeeper_model::{CountName, Counted, SURFACE_STEP, SetName};
use std::fs;

fn counted(set: &SetName, n: usize, unit: &'static str) -> Counted {
    Counted {
        name: CountName::surface(set),
        n: u32::try_from(n).unwrap_or(u32::MAX),
        step: SURFACE_STEP,
        unit,
        gate: "core/surface",
    }
}

pub(crate) fn measure(cx: &Cx) -> Result<(Vec<Counted>, Vec<String>)> {
    let config = config::load(cx.root)?;
    let surface = &config.surface;
    let (mut counts, mut out) = (Vec::new(), Vec::new());
    if let (false, Some(list)) = (surface.names.is_empty(), &config.docs.names) {
        let listed = arch::config::generated(cx.root)?;
        let (found, bad) = names::every(cx.root, list, &listed)?;
        let found = found.ok_or_else(|| Error(bad.join("; ")))?;
        for set in &surface.names {
            match found.get(set) {
                Some(held) => counts.push(counted(set, held.len(), "names")),
                None => out.push(format!(
                    "surface-{set}: the names list prints no `{set}` name: print the set or drop it from [surface] names"
                )),
            }
        }
    }
    if !surface.lines.is_empty() {
        let files = runner::ok("git", &["ls-files", "-z"], cx.root)?;
        let files: Vec<&str> = files.split('\0').filter(|f| !f.is_empty()).collect();
        for (set, globs) in &surface.lines {
            let read = files.iter().filter(|f| globs.iter().any(|g| g.matches(f)));
            let texts = read.map(|f| fs::read_to_string(cx.root.join(f)).unwrap_or_default());
            let n = texts.map(|t| t.lines().filter(|l| !l.trim().is_empty()).count());
            counts.push(counted(set, n.sum(), "lines"));
        }
    }
    Ok((counts, out))
}

/// Each `[surface]` count, a set's names in the docs gate's names list or a glob list's non-blank lines, under the lowest ceiling in `ci/ratchet/surface-<count>.d`.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let (counts, mut out) = measure(cx)?;
    if counts.is_empty() && out.is_empty() {
        cx.note.set("no [surface]: nothing is counted".into()).ok();
        return Ok(out);
    }
    let mut notes = Vec::new();
    for c in &counts {
        let files = folder::read::<u32>(cx.root, &c.name.dir())?;
        let lowest = folder::lowest(&files).map(|(_, n)| *n);
        out.extend(judge::ceiling(c, lowest));
        let ceiling = lowest.map_or_else(|| "none".to_owned(), |n| n.to_string());
        notes.push(format!("{} {} {} / ceiling {ceiling}", c.name, c.n, c.unit));
    }
    cx.note.set(notes.join(", ")).ok();
    Ok(out)
}
