use crate::health::count;
use crate::run::CheckCx as Cx;
use crate::{Error, Result, packs, runner};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Lang {
    ext: Vec<String>,
    hits: Vec<String>,
}

#[derive(Deserialize)]
struct Patterns {
    lang: Vec<Lang>,
}

fn file_hits(path: &str, text: &str, side: count::Side, hits: &[String]) -> Vec<String> {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let tested = count::tagged(side, text).filter_map(|(test, line)| test.then_some(line));
    for line in tested {
        for hit in hits {
            *seen.entry(hit).or_default() += line.matches(hit.as_str()).count();
        }
    }
    let each = seen
        .into_iter()
        .flat_map(|(hit, n)| (1..=n).map(move |i| (hit, i)));
    each.map(|(hit, i)| format!("{path}: {hit} #{i}")).collect()
}

/// Sleeps and wall-clock reads in test lines, by the patterns in the core pack's `test-sleeps.toml`; `[tests] soak` files are exempt.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let patterns: Patterns =
        toml::from_str(packs::text("core/files/test-sleeps.toml").unwrap_or_default())
            .map_err(|e| Error(format!("test-sleeps patterns: {e}")))?;
    let files = runner::ok("git", &["ls-files", "-z"], cx.root)?;
    let mut out = Vec::new();
    for path in files
        .split('\0')
        .filter(|p| !cx.tests.soak.iter().any(|g| g.matches(p)))
    {
        let ext = path.rsplit_once('.').map_or("", |(_, x)| x);
        let Some(lang) = patterns
            .lang
            .iter()
            .find(|l| l.ext.iter().any(|e| e == ext))
        else {
            continue;
        };
        let Some(side) = count::side(path, cx.tests) else {
            continue;
        };
        let text = std::fs::read_to_string(cx.root.join(path)).unwrap_or_default();
        out.extend(file_hits(path, &text, side, &lang.hits));
    }
    Ok(out)
}
