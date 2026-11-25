use crate::run::CheckCx as Cx;
use crate::{Error, Result, runner};
use gatekeeper_model::Glob;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const CONFIG: &str = "ci/cap.toml";
const RAISES: &str = "ci/cap.d";

#[derive(Deserialize)]
struct Counted {
    #[serde(default = "rust_globs")]
    count: Vec<Glob>,
    #[serde(default)]
    exclude: Vec<Glob>,
    #[serde(default)]
    skip_blank: bool,
    #[serde(default)]
    stop_at_cfg_test: bool,
}

fn rust_globs() -> Vec<Glob> {
    ["src/**/*.rs", "crates/*/src/**/*.rs"]
        .into_iter()
        .filter_map(|g| Glob::try_from(g.to_owned()).ok())
        .collect()
}

impl Counted {
    fn lines(&self, root: &Path) -> usize {
        let hit = |gs: &[Glob], p: &str| gs.iter().any(|g| g.matches(p));
        let mut files = BTreeSet::new();
        for g in &self.count {
            walk(root, &g.dir(), &mut files);
        }
        let files = files
            .into_iter()
            .filter(|p| hit(&self.count, p) && !hit(&self.exclude, p));
        files.map(|p| self.file_lines(&root.join(p))).sum()
    }

    fn file_lines(&self, p: &Path) -> usize {
        let cut = self.stop_at_cfg_test && p.extension().is_some_and(|x| x == "rs");
        let text = fs::read_to_string(p).unwrap_or_default();
        let body = text
            .lines()
            .take_while(|l| !(cut && crate::health::count::starts_tail(l)));
        body.filter(|l| !(self.skip_blank && l.trim().is_empty()))
            .count()
    }
}

fn walk(root: &Path, p: &str, out: &mut BTreeSet<String>) {
    let dir = root.join(p);
    if dir.is_file() {
        out.insert(p.to_owned());
    }
    for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let sub = if p.is_empty() {
            name
        } else {
            format!("{p}/{name}")
        };
        walk(root, &sub, out);
    }
}

fn ts_lines(p: &Path) -> usize {
    let each = fs::read_dir(p).into_iter().flatten().flatten();
    let n = |p: &Path| match p.extension() {
        _ if p.is_dir() => ts_lines(p),
        Some(x) if x == "ts" => fs::read_to_string(p).map_or(0, |t| t.lines().count()),
        _ => 0,
    };
    each.map(|e| n(&e.path())).sum()
}

fn cap_table(text: &str) -> Option<toml::Table> {
    let v: toml::Table = toml::from_str(text).ok()?;
    let fleets = v.get("fleet").and_then(toml::Value::as_table);
    let fleets = fleets.into_iter().flat_map(|f| f.values());
    std::iter::once(&v)
        .chain(fleets.filter_map(toml::Value::as_table))
        .find(|t| t.contains_key("line_cap"))
        .cloned()
}

fn trailer(l: &str) -> bool {
    let num = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    let Some((old, rest)) = l
        .trim()
        .strip_prefix("Cap: ")
        .and_then(|t| t.split_once(" → "))
    else {
        return false;
    };
    let Some((new, refs)) = rest.split_once(" (#") else {
        return false;
    };
    let Some((issue, q)) = refs.strip_suffix(')').and_then(|t| t.split_once(", q")) else {
        return false;
    };
    [old, new, issue, q].into_iter().all(num)
}

fn untrailed(cx: &Cx, base: &str) -> Result<Vec<String>> {
    let range = format!("{base}..HEAD");
    let args = [
        "log",
        "--no-merges",
        "--format=%x1e%h%x1f%B",
        &range,
        "--",
        CONFIG,
        RAISES,
    ];
    let log = runner::ok("git", &args, cx.root)?;
    let each = log
        .split('\x1e')
        .skip(1)
        .map(|c| c.split_once('\x1f').unwrap_or((c, "")));
    Ok(each
        .filter(|(_, body)| !body.lines().any(trailer))
        .map(|(sha, _)| {
            format!(
                "{sha} edits the line cap without a `Cap: <old> → <new> (#issue, q<N>)` trailer"
            )
        })
        .collect())
}

fn highest(cx: &Cx, base: i64) -> Result<(i64, Option<String>)> {
    let dir = fs::read_dir(cx.root.join(RAISES));
    let mut files: Vec<_> = (dir.into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    let mut top = (base, None);
    for p in files {
        let text = fs::read_to_string(&p).map_err(|e| Error::at(&p, e))?;
        let line_cap = toml::from_str::<toml::Table>(&text).ok();
        let line_cap = line_cap.and_then(|t| t.get("line_cap")?.as_integer());
        if let Some(n) = line_cap.filter(|n| *n > top.0) {
            let at = p.strip_prefix(cx.root).unwrap_or(&p);
            top = (n, Some(at.display().to_string()));
        }
    }
    Ok(top)
}

/// The counted lines under the cap in `ci/cap.toml` or a higher raise in `ci/cap.d/`; with a base, each commit that edits the cap carries its `Cap:` trailer.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let mut out = match cx.base {
        Some(base) => untrailed(cx, base)?,
        None => Vec::new(),
    };
    let text =
        fs::read_to_string(cx.root.join(CONFIG)).map_err(|e| Error::at(Path::new(CONFIG), e))?;
    let table = cap_table(&text).ok_or_else(|| Error(format!("no line_cap in {CONFIG}")))?;
    let at = |k: &str| table.get(k).and_then(toml::Value::as_integer);
    let base_cap = at("line_cap").ok_or_else(|| Error(format!("no line_cap in {CONFIG}")))?;
    let warn = at("warn_within").unwrap_or(500);
    let rule: Counted = toml::Value::Table(table)
        .try_into()
        .map_err(|e| Error::at(Path::new(CONFIG), e))?;
    let (cap, from) = highest(cx, base_cap)?;
    let n = i64::try_from(rule.lines(cx.root)).unwrap_or(i64::MAX);
    let counted = format!("src lines (formatted): {n} / cap {cap}");
    let left = cap - n;
    let mut note = Vec::new();
    if left < 0 {
        out.push(counted);
    } else {
        note.push(counted);
    }
    note.extend(from.map(|f| format!("cap raised by {f}")));
    let web = ts_lines(&cx.root.join("web/src"));
    note.push(format!(
        "web lines (TypeScript, soft budget, not capped): {web}"
    ));
    if (0..warn).contains(&left) {
        note.push(format!("WARN: {left} lines of headroom (< {warn})"));
    }
    cx.note.set(note.join("; ")).ok();
    Ok(out)
}
