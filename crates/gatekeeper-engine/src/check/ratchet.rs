use crate::Result;
use crate::ratchet::{change, trailer};
use crate::run::CheckCx as Cx;

fn raise(record: &str) -> String {
    format!("a raise carries `Ratchet: {record} <old> → <new> (#issue, q<N>)`")
}

/// No pack gate's list grew and no ceiling under `ci/ratchet/` was edited, deleted or raised against the base, unless a commit carries its `Ratchet:` trailer; a moved `[pin]` lets a list grow.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let Some(base) = cx.base else {
        cx.note.set("no base: nothing to compare".into()).ok();
        return Ok(Vec::new());
    };
    let grown = change::grown(cx.root, base)?;
    let touched = change::touched(cx.root, base)?;
    if grown.is_empty() && touched.is_empty() {
        cx.note.set("no list grew, no ceiling rose".into()).ok();
        return Ok(Vec::new());
    }
    let raised = trailer::find(&change::bodies(cx.root, base)?, "Ratchet");
    let named = |r: &str| raised.iter().any(|t| t.record.as_deref() == Some(r));
    let mut out = Vec::new();
    if !grown.is_empty() && change::pin_moved(cx.root, base) {
        cx.note
            .set("[pin] moved: a release may list more".into())
            .ok();
    } else {
        out.extend(grown.into_iter().filter(|g| !named(&g.gate)).map(|g| {
            let s = if g.added == 1 { "" } else { "s" };
            let raise = raise(&g.gate);
            format!(
                "{}: {} line{s} added; a list only shrinks ({raise})",
                g.list, g.added
            )
        }));
    }
    let held = touched.into_iter().filter(|t| !named(&t.count));
    out.extend(held.map(|t| format!("{}: {} ({})", t.file, t.why, raise(&t.count))));
    Ok(out)
}
