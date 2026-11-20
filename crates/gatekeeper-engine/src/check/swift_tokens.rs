use crate::run::CheckCx as Cx;
use crate::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const ALLOWED: &str = ".gatekeeper/tokens-allowed.txt";
const COLOURS: [&str; 24] = [
    "white", "black", "red", "green", "blue", "gray", "grey", "yellow", "orange", "purple", "pink",
    "cyan", "magenta", "teal", "navy", "silver", "maroon", "olive", "lime", "aqua", "fuchsia",
    "mint", "indigo", "brown",
];
const COLOUR_INITS: [&str; 5] = ["red", "white", "hue", ".sRGB", ".displayP3"];
const LABELS: [&str; 29] = [
    "spacing",
    "width",
    "height",
    "minWidth",
    "maxWidth",
    "minHeight",
    "maxHeight",
    "idealWidth",
    "idealHeight",
    "minLength",
    "radius",
    "cornerRadius",
    "lineWidth",
    "size",
    "duration",
    "horizontalSpacing",
    "verticalSpacing",
    "rowGap",
    "gap",
    "gutter",
    "least",
    "x",
    "y",
    "top",
    "bottom",
    "leading",
    "trailing",
    "horizontal",
    "vertical",
];
const CALLS: [&str; 7] = [
    "padding",
    "ui",
    "offset",
    "blur",
    "cornerRadius",
    "milliseconds",
    "seconds",
];

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn ident(s: &str) -> &str {
    let end = s.bytes().position(|b| !is_word(b)).unwrap_or(s.len());
    s.get(..end).unwrap_or_default()
}

fn number(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    let digits = |from: usize| {
        b.iter()
            .skip(from)
            .take_while(|c| c.is_ascii_digit())
            .count()
    };
    let mut end = usize::from(b.first() == Some(&b'-'));
    let whole = digits(end);
    if whole == 0 {
        return None;
    }
    end += whole;
    let fraction = digits(end + 1);
    if b.get(end) == Some(&b'.') && fraction > 0 {
        end += 1 + fraction;
    }
    (!b.get(end).copied().is_some_and(is_word)).then(|| s.get(..end))?
}

fn call(rest: &str) -> Option<(usize, &str)> {
    let args = rest.strip_prefix('(')?;
    let tail = args
        .strip_prefix('.')
        .and_then(|t| t.get(ident(t).len()..)?.strip_prefix(','))
        .map_or(args, str::trim_start);
    let n = number(tail)?;
    Some((rest.len() - tail.len() + n.len(), n))
}

fn at(src: &str, i: usize, prev: Option<u8>) -> Option<(String, String)> {
    let rest = src.get(i..)?;
    if rest.starts_with("#colorLiteral") {
        return Some(("#colorLiteral".into(), "#colorLiteral".into()));
    }
    if let Some(after) = rest.strip_prefix('.') {
        let name = ident(after);
        let tail = after.get(name.len()..)?;
        let bare = !prev.is_some_and(|p| is_word(p) || p == b')' || p == b']');
        if bare && COLOURS.contains(&name) {
            let what = format!(".{name}");
            return Some((what.clone(), what));
        }
        if name == "hex" && tail.starts_with('(') {
            let what = format!(".hex({}", ident(tail.get(1..)?));
            return Some((what.clone(), what));
        }
        let (len, n) = call(tail).filter(|_| CALLS.contains(&name))?;
        let what = rest.get(..1 + name.len() + len)?.split_whitespace();
        return Some((what.collect::<Vec<_>>().join(" "), n.into()));
    }
    if prev.is_some_and(is_word) || !rest.bytes().next().is_some_and(is_word) {
        return None;
    }
    let name = ident(rest);
    let tail = rest.get(name.len()..)?;
    if name == "Color" {
        let init = tail.strip_prefix('(').and_then(|a| {
            let found = COLOUR_INITS.iter().find(|k| a.starts_with(*k));
            found.map(|k| format!("Color({k}"))
        });
        let member = tail.strip_prefix('.').map(ident);
        let member = member
            .filter(|m| COLOURS.contains(m))
            .map(|m| format!("Color.{m}"));
        return init.or(member).map(|w| (w.clone(), w));
    }
    let n = tail.strip_prefix(':').map(str::trim_start).and_then(number);
    n.filter(|_| LABELS.contains(&name))
        .map(|n| (format!("{name}: {n}"), n.to_owned()))
}

fn raws(src: &str) -> Vec<(String, String)> {
    let bytes = src.as_bytes();
    let prev = |i: usize| i.checked_sub(1).and_then(|p| bytes.get(p)).copied();
    (0..bytes.len())
        .filter_map(|i| at(src, i, prev(i)))
        .collect()
}

fn swift_files(root: &Path, dir: &str, out: &mut Vec<String>) -> Result<()> {
    let path = root.join(dir);
    let entries = fs::read_dir(&path).map_err(|e| Error::at(&path, e))?;
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    names.sort();
    for name in names.iter().filter(|n| !n.starts_with('.')) {
        let rel = format!("{dir}/{name}");
        if root.join(&rel).is_dir() {
            swift_files(root, &rel, out)?;
        } else if Path::new(name).extension().is_some_and(|x| x == "swift") {
            out.push(rel);
        }
    }
    Ok(())
}

fn allowed(root: &Path) -> Result<(BTreeSet<String>, BTreeSet<String>)> {
    let path = root.join(ALLOWED);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(Error::at(&path, e)),
    };
    let (mut values, mut files) = (BTreeSet::new(), BTreeSet::new());
    for line in text.lines() {
        let mut words = line.split_whitespace();
        match (words.next(), words.next()) {
            (Some("swift"), Some(raw)) => values.insert(raw.to_owned()),
            (Some("file"), Some(file)) => files.insert(file.to_owned()),
            _ => false,
        };
    }
    Ok((values, files))
}

/// No raw colour or raw size in the `[swift] tokens` folders but the allow-list's; each occurrence is a numbered finding for the baseline.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let swift = cx.swift()?;
    let (values, skip) = allowed(cx.root)?;
    let mut files = Vec::new();
    for dir in &swift.tokens {
        swift_files(cx.root, dir.trim_end_matches('/'), &mut files)?;
    }
    let mut findings = Vec::new();
    for file in files.iter().filter(|f| !skip.contains(*f)) {
        let path = cx.root.join(file);
        let src = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for (what, raw) in raws(&src) {
            let zero = raw
                .trim_start_matches('-')
                .bytes()
                .all(|b| b == b'0' || b == b'.');
            if zero || values.contains(&raw) {
                continue;
            }
            let n = seen.entry(what.clone()).or_default();
            *n += 1;
            findings.push(format!("{file}: raw `{what}` #{n}"));
        }
    }
    Ok(findings)
}
