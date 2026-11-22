enum Token {
    Starts(&'static str, Option<&'static str>),
    Name(&'static str),
    Phrase(&'static str),
}

const WALKERS: [Token; 11] = [
    Token::Starts("readdir", None),
    Token::Starts("opendir", None),
    Token::Starts("scandir", None),
    Token::Starts("execFile", None),
    Token::Starts("spawn", None),
    Token::Starts("glob", Some("global")),
    Token::Name("fdir"),
    Token::Name("execSync"),
    Token::Phrase("import.meta.glob"),
    Token::Phrase("ls-files"),
    Token::Phrase("git grep"),
];

const COMMENTS: [&str; 4] = ["//", "/*", "*", "<!--"];
const QUOTES: [char; 3] = ['\'', '"', '`'];
const SCRIPTS: [&str; 8] = ["js", "jsx", "ts", "tsx", "mjs", "cjs", "mts", "cts"];

pub(super) fn code(text: &str) -> String {
    let kept = text.lines().filter(|line| {
        let start = line.trim_start();
        !COMMENTS.iter().any(|c| start.starts_with(c))
    });
    kept.collect::<Vec<_>>().join("\n")
}

pub(super) fn walks(code: &str) -> bool {
    let names: Vec<&str> = code.split(|c: char| !is_ident(c)).collect();
    WALKERS.iter().any(|token| match token {
        Token::Starts(start, not) => names
            .iter()
            .any(|n| n.starts_with(start) && not.is_none_or(|x| !n.starts_with(x))),
        Token::Name(name) => names.contains(name),
        Token::Phrase(phrase) => code.contains(phrase),
    })
}

pub(super) fn has_token(code: &str, needle: &str) -> bool {
    code.match_indices(needle).any(|(i, _)| {
        !code[..i].chars().next_back().is_some_and(in_token)
            && !code[i + needle.len()..]
                .chars()
                .next()
                .is_some_and(in_token)
    })
}

pub(super) fn named_dir(code: &str, path: &str) -> Option<String> {
    let mut dir = path;
    while let Some((parent, _)) = dir.rsplit_once('/') {
        if names(code, parent) {
            return Some(parent.to_owned());
        }
        dir = parent;
    }
    None
}

pub(super) fn imports(code: &str, deleted: &str) -> bool {
    let mut parts = deleted.rsplit('/');
    let stem = parts.next().unwrap_or(deleted);
    let stem = stem.rsplit_once('.').map_or(stem, |(s, _)| s);
    let Some(name) = (if stem == "index" {
        parts.next()
    } else {
        Some(stem)
    }) else {
        return false;
    };
    let tail = format!("/{name}");
    quoted(code).any(|s| {
        s.ends_with(&tail)
            || SCRIPTS
                .iter()
                .any(|ext| s.ends_with(&format!("{tail}.{ext}")))
    })
}

fn names(code: &str, dir: &str) -> bool {
    let segments: Vec<&str> = dir.split('/').collect();
    let [first, rest @ ..] = segments.as_slice() else {
        return false;
    };
    code.match_indices(first).any(|(i, _)| {
        let before = code[..i].chars().next_back();
        let mut tail = &code[i + first.len()..];
        if rest.is_empty() {
            let edge = |c: Option<char>| c.is_some_and(|c| c == '/' || QUOTES.contains(&c));
            return edge(before) && edge(tail.chars().next());
        }
        if before.is_some_and(in_token) {
            return false;
        }
        for segment in rest {
            let joined = tail.trim_start_matches(is_separator);
            if joined.len() == tail.len() || !joined.starts_with(segment) {
                return false;
            }
            tail = &joined[segment.len()..];
        }
        !tail.chars().next().is_some_and(in_token)
    })
}

fn quoted(code: &str) -> impl Iterator<Item = &str> {
    code.lines().flat_map(|line| {
        let mut found = Vec::new();
        let mut rest = line;
        while let Some(open) = rest.find(QUOTES) {
            let quote = &rest[open..=open];
            let after = &rest[open + 1..];
            let Some(close) = after.find(quote) else {
                break;
            };
            found.push(&after[..close]);
            rest = &after[close + 1..];
        }
        found
    })
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

fn in_token(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.')
}

fn is_separator(c: char) -> bool {
    matches!(c, '/' | '\\' | ',' | '+' | ' ' | '\t') || QUOTES.contains(&c)
}
