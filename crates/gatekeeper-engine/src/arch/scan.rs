use super::baseline::Listed;
use super::config::{Arch, Ban, Layer, Size};
use super::{any, walk};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const SLASHES: [&str; 13] = [
    "js", "jsx", "mjs", "cjs", "mts", "cts", "swift", "go", "kt", "java", "c", "h", "cpp",
];

#[derive(Default)]
pub(crate) struct Scan {
    pub files: Vec<String>,
    pub bad: Vec<String>,
    pub sizes: BTreeMap<String, usize>,
    pub crossed: BTreeSet<String>,
}

struct Line<'a> {
    path: &'a str,
    at: String,
    text: &'a str,
    rust: bool,
}

pub(crate) fn scan(root: &Path, a: &Arch, listed: &Listed) -> Scan {
    let mut s = Scan::default();
    walk(root, "", &a.named(), &mut s.files);
    s.files.sort();
    for p in &s.files.clone() {
        file(root, a, listed, p, &mut s);
    }
    s
}

fn file(root: &Path, a: &Arch, listed: &Listed, p: &str, s: &mut Scan) {
    let ext = Path::new(p)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or_default();
    let rust = ext == "rs";
    let code = rust || ext == "ts" || ext == "tsx";
    let slashes = code || SLASHES.contains(&ext);
    let bans: Vec<&Ban> = a
        .ban
        .iter()
        .filter(|b| any(&b.paths, p) && !any(&b.except, p))
        .collect();
    let rules: Vec<&Size> = a
        .size
        .iter()
        .filter(|r| any(&r.paths, p) && !any(&r.exclude, p))
        .collect();
    if !code && bans.is_empty() && rules.is_empty() {
        return;
    }
    let Ok(src) = std::fs::read_to_string(root.join(p)) else {
        return;
    };
    let lines: Vec<&str> = src.lines().collect();
    let own = a
        .layer
        .iter()
        .find(|l| code && any(&l.paths, p) && !any(&l.exclude, p));
    for (i, text) in lines.iter().enumerate() {
        let at = format!("{p}:{}", i + 1);
        let comment = slashes && text.trim_start().starts_with("//");
        for b in bans
            .iter()
            .filter(|b| b.hits(text) && (b.comments || !comment))
        {
            s.bad.push(format!(
                "{at}: `{}` is banned here: {}",
                b.pattern, b.reason
            ));
        }
        if let Some(own) = own.filter(|_| !comment) {
            let line = Line {
                path: p,
                at,
                text,
                rust,
            };
            cross(a, own, &line, listed, s);
        }
    }
    for r in rules {
        sizes(r, (p, ext), &lines, listed, s);
    }
}

fn cross(a: &Arch, own: &Layer, l: &Line, listed: &Listed, s: &mut Scan) {
    for t in targets(l.path, l.text, l.rust) {
        let to = a.layer.iter().find(|x| uses(x, &t));
        let to = to.filter(|x| x.name != own.name && !own.may_use.contains(&x.name));
        let Some(to) = to else { continue };
        let key = format!("{} -> {}", l.path, to.name);
        if !listed.crossings.contains(&key) {
            let (at, from, may) = (&l.at, &own.name, &own.may_use);
            s.bad.push(format!(
                "{at}: layer `{from}` uses `{}` ({t}); its may_use is {may:?}",
                to.name
            ));
        }
        s.crossed.insert(key);
    }
}

fn sizes(r: &Size, (p, ext): (&str, &str), lines: &[&str], listed: &Listed, s: &mut Scan) {
    let mut all = vec![(p.to_owned(), 1, lines.len(), r.max_file_lines)];
    let each = fns(lines, ext).into_iter();
    all.extend(each.map(|(name, at, n)| (format!("{p}:{name}"), at, n, r.max_fn_lines)));
    for (key, at, n, max) in all {
        let Some(max) = max.filter(|&m| n > m) else {
            continue;
        };
        let e = s.sizes.entry(key.clone()).or_default();
        *e = n.max(*e);
        match listed.sizes.get(&key) {
            Some(&was) if n <= was => {}
            was => s.bad.push(format!(
                "{p}:{at}: `{key}` is {n} lines, over {max} (baseline {was:?})"
            )),
        }
    }
}

fn uses(l: &Layer, t: &str) -> bool {
    let under =
        |m: &String| t == m || t.starts_with(&format!("{m}::")) || t.starts_with(&format!("{m}/"));
    l.mods.iter().any(under)
        || ["", ".ts", ".tsx", "/index.ts"]
            .iter()
            .any(|x| any(&l.paths, &format!("{t}{x}")))
}

fn rust_targets(l: &str) -> Vec<String> {
    let path = |c: char| !(c.is_alphanumeric() || c == '_' || c == ':');
    let mut t: Vec<String> = l
        .split(path)
        .filter(|w| w.contains("::"))
        .map(String::from)
        .collect();
    if let Some((head, rest)) = l.split_once("::{") {
        let pre = head.rsplit(path).next().unwrap_or_default();
        let items = rest
            .split(['}', ','])
            .map(str::trim)
            .filter(|x| !x.is_empty());
        t.extend(items.map(|x| format!("{pre}::{x}")));
    }
    t
}

fn targets(p: &str, l: &str, rust: bool) -> Vec<String> {
    if rust {
        return rust_targets(l);
    }
    let quoted = |s: &str| {
        let s = s.trim_start();
        let q = s.chars().next().filter(|c| "\"'`".contains(*c))?;
        s[1..].split(q).next().map(String::from)
    };
    let Some(spec) = ["from ", "import(", "require(", "import "]
        .iter()
        .filter_map(|k| l.split_once(k))
        .find_map(|(_, s)| quoted(s))
    else {
        return vec![];
    };
    if !spec.starts_with('.') {
        return vec![spec];
    }
    let mut parts: Vec<&str> = p.split('/').collect();
    parts.pop();
    for c in spec.split('/') {
        match c {
            "." => {}
            ".." => drop(parts.pop()),
            c => parts.push(c),
        }
    }
    vec![parts.join("/")]
}

fn fns(lines: &[&str], ext: &str) -> Vec<(String, usize, usize)> {
    let kw = match ext {
        "rs" => "fn ",
        "swift" => "func ",
        "ts" | "tsx" => "function ",
        _ => return vec![],
    };
    let ts = kw == "function ";
    let mut v = vec![];
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        let arrow = || {
            (ts && t.contains("=> {"))
                .then(|| t.trim_start_matches("export ").strip_prefix("const "))
        };
        let after = t
            .split_once(kw)
            .filter(|(pre, _)| pre.is_empty() || pre.ends_with(' '))
            .map(|x| x.1);
        let Some(name) = after
            .or_else(|| arrow().flatten())
            .and_then(|r| r.split(|c: char| !c.is_alphanumeric() && c != '_').next())
        else {
            continue;
        };
        if name.is_empty() || l.trim_end().ends_with([';', '}']) {
            continue;
        }
        let ind = l.len() - t.len();
        let end = lines
            .iter()
            .skip(i + 1)
            .position(|x| x.len() - x.trim_start().len() == ind && x.trim_start().starts_with('}'));
        if let Some(e) = end {
            v.push((name.to_string(), i + 1, e + 2));
        }
    }
    v
}
