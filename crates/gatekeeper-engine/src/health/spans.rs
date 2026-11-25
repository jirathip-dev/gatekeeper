use super::count::{self, Side};
use gatekeeper_model::{Changed, TestsConfig};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;

/// The head lines each file of a `git diff -U0` touches; a pure deletion touches the line before it.
#[must_use]
pub fn hunks(diff: &str) -> BTreeMap<String, BTreeSet<u32>> {
    let mut out = BTreeMap::<String, BTreeSet<u32>>::new();
    let mut file = None;
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            file = path.strip_prefix("b/").map(str::to_owned);
            continue;
        }
        let (Some(path), Some(hunk)) = (&file, line.strip_prefix("@@ ")) else {
            continue;
        };
        let new = hunk
            .split(' ')
            .find_map(|w| w.strip_prefix('+'))
            .unwrap_or("0");
        let (from, n) = new.split_once(',').unwrap_or((new, "1"));
        let (from, n) = (from.parse().unwrap_or(0u32), n.parse().unwrap_or(1u32));
        let touched = out.entry(path.clone()).or_default();
        touched.extend(from.max(1)..from.max(1) + n.max(1));
    }
    out
}

fn is_test_attr(line: &str) -> bool {
    let line = line.trim();
    line == "#[test]" || line.starts_with("#[") && line.contains("::test")
}

fn fn_name(line: &str) -> Option<String> {
    let (_, rest) = line.split_once("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    Some(name).filter(|n| !n.is_empty())
}

/// A Rust file's test functions: each name with its lines, from its test attribute to its closing brace.
#[must_use]
pub fn test_fns(text: &str) -> Vec<(String, RangeInclusive<u32>)> {
    let mut out = Vec::new();
    let mut lines = (1u32..).zip(text.lines());
    while let Some((start, line)) = lines.next() {
        if !is_test_attr(line) {
            continue;
        }
        let (mut name, mut depth, mut opened) = (None, 0i64, false);
        for (at, line) in lines.by_ref() {
            name = name.or_else(|| fn_name(line));
            for c in line.chars().filter(|_| name.is_some()) {
                depth += match c {
                    '{' => 1,
                    '}' => -1,
                    _ => 0,
                };
                opened = opened || depth > 0;
            }
            if opened && depth <= 0 {
                out.extend(name.take().map(|n| (n, start..=at)));
                break;
            }
        }
    }
    out
}

fn owner(rel: &str, krate: &str) -> Option<(String, String)> {
    let module = |path: &str| {
        let path = path.trim_end_matches(".rs");
        let path = if path == "mod" {
            ""
        } else {
            path.trim_end_matches("/mod")
        };
        let mut m = path.replace('/', "::");
        if !m.is_empty() && !m.ends_with("::") {
            m.push_str("::");
        }
        m
    };
    if let Some(rest) = rel.strip_prefix("tests/") {
        let (target, inner) = rest.split_once('/').unwrap_or((rest, "main.rs"));
        let inner = if inner == "main.rs" { "" } else { inner };
        return Some((
            format!("{krate}::{}", target.trim_end_matches(".rs")),
            module(inner),
        ));
    }
    let rest = rel.strip_prefix("src/")?;
    Some(match rest {
        "main.rs" => (format!("{krate}::bin/{krate}"), String::new()),
        "lib.rs" => (krate.to_owned(), String::new()),
        _ => match rest.strip_prefix("bin/") {
            Some(bin) => (
                format!("{krate}::bin/{}", bin.trim_end_matches(".rs")),
                String::new(),
            ),
            None => (krate.to_owned(), module(rest)),
        },
    })
}

fn crate_of<'a>(path: &'a str, krates: &'a BTreeMap<String, String>) -> Option<(&'a str, &'a str)> {
    krates
        .iter()
        .filter_map(|(dir, name)| {
            let rel = match dir.as_str() {
                "" | "." => Some(path),
                dir => path.strip_prefix(dir)?.strip_prefix('/'),
            };
            rel.map(|r| (r, name.as_str()))
        })
        .min_by_key(|(rel, _)| rel.len())
}

fn rust(
    path: &str,
    text: &str,
    touched: &BTreeSet<u32>,
    side: Side,
    krates: &BTreeMap<String, String>,
) -> Vec<Changed> {
    let from = match side {
        Side::Test => 1,
        Side::Src => return Vec::new(),
        Side::SrcThenTestTail => match (1u32..)
            .zip(text.lines())
            .find(|(_, l)| count::starts_tail(l))
        {
            Some((at, _)) => at,
            None => return Vec::new(),
        },
    };
    let Some((binary, module)) = crate_of(path, krates).and_then(|(rel, k)| owner(rel, k)) else {
        return Vec::new();
    };
    let fns = test_fns(text);
    let each = touched.iter().filter(|l| **l >= from).map(|l| {
        match fns.iter().find(|(_, span)| span.contains(l)) {
            Some((name, _)) => Changed::RustFn {
                binary: binary.clone(),
                module: module.clone(),
                name: name.clone(),
            },
            None => Changed::RustFile {
                binary: binary.clone(),
                module: module.clone(),
            },
        }
    });
    each.collect()
}

/// The test code a diff changed: `touched` lines and head texts by path, `krates` crate dirs to package names.
#[must_use]
pub fn changed(
    touched: &BTreeMap<String, BTreeSet<u32>>,
    texts: &BTreeMap<String, String>,
    krates: &BTreeMap<String, String>,
    tests: &TestsConfig,
) -> BTreeSet<Changed> {
    let mut out = BTreeSet::new();
    for (path, lines) in touched {
        let (Some(side), Some(text)) = (count::side(path, tests), texts.get(path)) else {
            continue;
        };
        if path.ends_with(".rs") {
            out.extend(rust(path, text, lines, side, krates));
        } else if matches!(side, Side::Test) && !path.ends_with(".swift") {
            out.insert(Changed::TsFile(path.clone()));
        }
    }
    out
}
