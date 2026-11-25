use crate::health::count::starts_tail;
use gatekeeper_model::Glob;
use std::collections::BTreeMap;

/// A workspace member: its folder from the workspace root, and whether its `pub` items are a published product.
pub struct Member {
    pub dir: String,
    pub published: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Value,
    Type,
}

const KINDS: [(&str, Kind); 8] = [
    ("fn", Kind::Value),
    ("const", Kind::Value),
    ("static", Kind::Value),
    ("struct", Kind::Type),
    ("enum", Kind::Type),
    ("union", Kind::Type),
    ("trait", Kind::Type),
    ("type", Kind::Type),
];

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Target {
    member: usize,
    part: String,
}

struct Item {
    path: String,
    kind: Kind,
    word: &'static str,
    name: String,
    target: Target,
}

#[derive(Default)]
struct Seen {
    src: u32,
    test: u32,
}

type Mentions = BTreeMap<String, BTreeMap<Target, Seen>>;

fn within<'a>(dir: &str, path: &'a str) -> Option<&'a str> {
    if dir == "." {
        return Some(path);
    }
    path.strip_prefix(dir)?.strip_prefix('/')
}

fn part(rel: &str, has_lib: bool) -> Option<String> {
    let mut parts = rel.splitn(3, '/');
    let first = parts.next()?;
    let stem = |f: &str| f.trim_end_matches(".rs").to_owned();
    match (first, parts.next(), parts.next()) {
        ("build.rs", None, None) => Some("build".into()),
        ("src", Some("main.rs"), None) => Some("main".into()),
        ("src", Some("bin"), Some(f)) => Some(format!("bin/{}", stem(f.split('/').next()?))),
        ("src", Some(_), _) => Some(if has_lib { "lib" } else { "main" }.into()),
        ("tests" | "benches" | "examples", Some(f), _) => Some(format!("{first}/{}", stem(f))),
        _ => None,
    }
}

fn code(line: &str) -> &str {
    let (mut quoted, mut escaped) = (false, false);
    for (i, c) in line.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '/' if !quoted && line.get(i + 1..).is_some_and(|r| r.starts_with('/')) => {
                return line.get(..i).unwrap_or(line);
            }
            _ => {}
        }
    }
    line
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let first = |w: &&str| {
        w.chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
    };
    text.split(move |c: char| !ident(c)).filter(first)
}

fn item(line: &str) -> Option<(&'static str, Kind, String)> {
    let tokens: Vec<&str> = line.strip_prefix("pub ")?.split_whitespace().collect();
    let before_fn = |i: usize| {
        let next = tokens.get(i + 1).copied();
        matches!(next, Some("fn" | "unsafe" | "async" | "extern"))
    };
    let modifier = |(i, t): &(usize, &&str)| {
        matches!(**t, "async" | "unsafe" | "extern")
            || t.starts_with('"')
            || **t == "const" && before_fn(*i)
    };
    let at = tokens.iter().enumerate().find(|it| !modifier(it))?.0;
    let &(word, kind) = KINDS.iter().find(|(k, _)| tokens.get(at) == Some(k))?;
    let mut rest = tokens.iter().skip(at + 1).filter(|t| **t != "mut");
    let name = words(rest.next()?).next()?;
    Some((word, kind, name.to_owned()))
}

fn count(mentions: &mut Mentions, target: &Target, text: &str, skip: Option<&str>, in_test: bool) {
    let mut skip = skip;
    for w in words(text) {
        if skip == Some(w) {
            skip = None;
            continue;
        }
        let seen = mentions.entry(w.to_owned()).or_default();
        let seen = seen.entry(target.clone()).or_default();
        if in_test {
            seen.test += 1;
        } else {
            seen.src += 1;
        }
    }
}

fn read(
    path: &str,
    text: &str,
    (target, exempt): (&Target, bool),
    out: (&mut Vec<Item>, &mut Mentions),
) {
    let (items, mentions) = out;
    let mut in_test = target.part.starts_with("tests/") || target.part.starts_with("benches/");
    let mut exported = false;
    for raw in text.lines() {
        in_test = in_test || starts_tail(raw);
        let line = code(raw).trim();
        let found = item(line).filter(|_| !in_test && !exempt && !exported);
        exported = match line {
            "" => exported,
            l if l.starts_with("#[") => {
                exported || l.contains("no_mangle") || l.contains("export_name")
            }
            _ => false,
        };
        if line.starts_with("pub use ") {
            continue;
        }
        let head = (line.starts_with("impl ") || line.starts_with("impl<"))
            .then(|| line.split_once(" for ").map_or(0, |(t, _)| t.len()));
        let line = head.map_or(line, |n| line.get(..n).unwrap_or_default());
        count(
            mentions,
            target,
            line,
            found.as_ref().map(|f| f.2.as_str()),
            in_test,
        );
        if let Some((word, kind, name)) = found {
            let path = path.to_owned();
            let target = target.clone();
            items.push(Item {
                path,
                kind,
                word,
                name,
                target,
            });
        }
    }
}

fn verdict(item: &Item, mentions: &Mentions) -> Option<&'static str> {
    let by = mentions.get(&item.name);
    let all = || by.into_iter().flatten();
    let src: u32 = all().map(|(_, s)| s.src).sum();
    let test: u32 = all().map(|(_, s)| s.test).sum();
    let other = all().any(|(t, s)| *t != item.target && s.src + s.test > 0);
    match item.kind {
        _ if src == 0 && test > 0 => Some("only tests name it"),
        Kind::Type if src == 0 => Some("nothing names it"),
        Kind::Value if !other => Some("no other crate names it"),
        _ => None,
    }
}

/// The `pub` scan over a workspace's `.rs` files, each `(path from the workspace root, text)`: one finding per `pub` item no other crate names, only tests name, or, for a type, nothing names.
#[must_use]
pub fn scan(members: &[Member], files: &[(String, String)], exported: &[Glob]) -> Vec<String> {
    let has_lib: Vec<bool> = (members.iter())
        .map(|m| {
            files
                .iter()
                .any(|(p, _)| within(&m.dir, p) == Some("src/lib.rs"))
        })
        .collect();
    let mut by_depth: Vec<_> = members.iter().enumerate().collect();
    by_depth.sort_by_key(|(_, m)| std::cmp::Reverse(m.dir.len()));
    let (mut items, mut mentions) = (Vec::new(), Mentions::new());
    for (path, text) in files {
        let Some((i, m, rel)) =
            (by_depth.iter()).find_map(|&(i, m)| Some((i, m, within(&m.dir, path)?)))
        else {
            continue;
        };
        let Some(part) = part(rel, has_lib.get(i).copied().unwrap_or_default()) else {
            continue;
        };
        let exempt = m.published || exported.iter().any(|g| g.matches(path));
        let target = Target { member: i, part };
        read(path, text, (&target, exempt), (&mut items, &mut mentions));
    }
    let mut met: BTreeMap<String, u32> = BTreeMap::new();
    let lines = items.iter().filter_map(|it| {
        let why = verdict(it, &mentions)?;
        Some(format!("{}: pub {} {}: {why}", it.path, it.word, it.name))
    });
    let lines: Vec<String> = lines.collect();
    lines
        .into_iter()
        .map(|l| {
            let n = met.entry(l.clone()).or_default();
            *n += 1;
            if *n > 1 { format!("{l} #{n}") } else { l }
        })
        .collect()
}
