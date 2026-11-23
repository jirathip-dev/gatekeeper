use super::any;
use super::config::{Arch, Policy};
use super::frozen::Change;
use gatekeeper_model::Glob;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) fn found(root: &Path, a: &Arch, diff: &[Change], files: &[String]) -> Vec<String> {
    match &a.comments {
        Some(c) if c.policy == Policy::Off => new_comments(root, diff, &c.except),
        Some(c) if c.policy == Policy::Strict => files
            .iter()
            .filter(|f| commented(f, &c.except))
            .flat_map(|f| bad_comments(root, f, None))
            .collect(),
        _ => vec![],
    }
}

fn commented(f: &str, except: &[Glob]) -> bool {
    [".rs", ".ts", ".tsx"].iter().any(|x| f.ends_with(x)) && !any(except, f)
}

fn new_comments(root: &Path, diff: &[Change], except: &[Glob]) -> Vec<String> {
    let mut added = BTreeMap::<&str, Vec<usize>>::new();
    for c in diff
        .iter()
        .filter(|c| c.added && commented(&c.file, except))
    {
        added
            .entry(&c.file)
            .or_default()
            .push(c.line.saturating_sub(1));
    }
    added
        .into_iter()
        .flat_map(|(f, at)| bad_comments(root, f, Some(&at)))
        .collect()
}

fn bad_comments(root: &Path, f: &str, only: Option<&[usize]>) -> Vec<String> {
    let src = std::fs::read_to_string(root.join(f)).unwrap_or_default();
    let lines: Vec<&str> = src.lines().collect();
    let com = comment_lines(&lines, Path::new(f).extension().is_some_and(|x| x == "rs"));
    let at: Vec<usize> = only.map_or_else(|| (0..lines.len()).collect(), <[usize]>::to_vec);
    at.into_iter()
        .filter(|&i| com.get(i) == Some(&true) && !allowed(&lines, &com, i))
        .map(|i| {
            format!(
                "{f}:{}: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`",
                i + 1
            )
        })
        .collect()
}

fn allowed(lines: &[&str], com: &[bool], i: usize) -> bool {
    let t = lines.get(i).unwrap_or(&"").trim();
    let todo = t
        .strip_prefix("// TODO(#")
        .and_then(|r| r.split(')').next());
    if todo.is_some_and(|n| n.parse::<u32>().is_ok()) {
        return true;
    }
    if t.starts_with("// SAFETY:") {
        return lines.get(i + 1).is_some_and(|n| n.contains("unsafe"));
    }
    let one_doc = (t.starts_with("///") && !t.starts_with("////"))
        || (t.starts_with("/**") && t.ends_with("*/"));
    let mut item = lines.iter().skip(i + 1).map(|l| l.trim_start());
    let item = item.find(|l| !l.starts_with("#[") && !l.starts_with('@'));
    let public = item.is_some_and(|l| l.starts_with("pub ") || l.starts_with("export "));
    one_doc && public && (i == 0 || com.get(i - 1) == Some(&false))
}

#[derive(Default)]
struct Lexer {
    block: bool,
    quote: Option<char>,
    raw: Option<usize>,
}

impl Lexer {
    fn line(&mut self, s: &[char], rust: bool) -> bool {
        let (mut c, mut i) = (self.block, 0);
        while let Some(&ch) = s.get(i) {
            let next = s.get(i + 1).copied();
            let prev = i.checked_sub(1).and_then(|j| s.get(j));
            if self.block {
                self.block = !(ch == '*' && next == Some('/'));
                i += if self.block { 1 } else { 2 };
            } else if let Some(n) = self.raw {
                let hashes = s.iter().skip(i + 1).take(n).filter(|&&x| x == '#').count();
                let end = ch == '"' && hashes == n;
                self.raw = if end { None } else { self.raw };
                i += if end { n + 1 } else { 1 };
            } else if let Some(q) = self.quote {
                self.quote = if ch == q { None } else { self.quote };
                i += if ch == '\\' { 2 } else { 1 };
            } else if ch == '/' && next == Some('/') {
                return true;
            } else if ch == '/' && next == Some('*') {
                (c, self.block, i) = (true, true, i + 2);
            } else if rust
                && ch == 'r'
                && matches!(next, Some('"' | '#'))
                && prev.is_none_or(|c| !c.is_alphanumeric())
            {
                let n = s.iter().skip(i + 1).take_while(|&&x| x == '#').count();
                self.raw = (s.get(i + 1 + n) == Some(&'"')).then_some(n);
                i += if self.raw.is_some() { n + 2 } else { 1 };
            } else if ch == '"' || (!rust && matches!(ch, '\'' | '`')) {
                (self.quote, i) = (Some(ch), i + 1);
            } else if rust && ch == '\'' && (next == Some('\\') || s.get(i + 2) == Some(&'\'')) {
                let close = s
                    .get(i + 3..)
                    .and_then(|r| r.iter().position(|&x| x == '\''));
                i += close.filter(|_| next == Some('\\')).map_or(3, |p| p + 4);
            } else {
                i += 1;
            }
        }
        c
    }
}

fn comment_lines(lines: &[&str], rust: bool) -> Vec<bool> {
    let mut lexer = Lexer::default();
    lines
        .iter()
        .map(|l| lexer.line(&l.chars().collect::<Vec<_>>(), rust))
        .collect()
}
