use crate::Result;
use crate::ratchet::{change, trailer};
use crate::run::CheckCx as Cx;

/// No pack gate's list grew against the base, unless a commit carries its `Ratchet:` trailer or the change moves `[pin]`.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let Some(base) = cx.base else {
        cx.note.set("no base: nothing to compare".into()).ok();
        return Ok(Vec::new());
    };
    let grown = change::grown(cx.root, base)?;
    if grown.is_empty() {
        cx.note.set("no list grew".into()).ok();
        return Ok(Vec::new());
    }
    if change::pin_moved(cx.root, base) {
        cx.note
            .set("[pin] moved: a release may list more".into())
            .ok();
        return Ok(Vec::new());
    }
    let raised = trailer::find(&change::bodies(cx.root, base)?, "Ratchet");
    let held = grown
        .into_iter()
        .filter(|g| !raised.iter().any(|t| t.record.as_ref() == Some(&g.gate)));
    Ok(held
        .map(|g| {
            let s = if g.added == 1 { "" } else { "s" };
            format!(
                "{}: {} line{s} added; a list only shrinks (a raise carries `Ratchet: {} <old> → <new> (#issue, q<N>)`)",
                g.list, g.added, g.gate
            )
        })
        .collect())
}
