use crate::config::read_toml;
use crate::run::CheckCx as Cx;
use crate::{Error, Result, runner};
use gatekeeper_model::Glob;
use serde::Deserialize;
use serde_json::Value as Json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use toml::{Table, Value};

const LIST: &str = ".gatekeeper/deps.toml";
const GROWTH: u32 = 10;
const CARGO_KINDS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
const NPM_KINDS: [&str; 4] = [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];
const NPM_LOCAL: [&str; 3] = ["workspace:", "file:", "link:"];

type Reader = fn(&Path) -> Result<BTreeSet<String>>;
type Counter = fn(&str) -> Result<usize>;

const ECOSYSTEMS: [(&str, Reader); 3] = [("cargo", cargo), ("npm", npm), ("swift", swift)];
const LOCKS: [(&str, Counter); 3] = [
    ("Cargo.lock", cargo_lock),
    ("pnpm-lock.yaml", |text| Ok(pnpm_lock(text))),
    ("Package.resolved", swift_lock),
];

#[derive(Deserialize)]
struct List {
    growth: u32,
    #[serde(flatten)]
    kinds: BTreeMap<String, BTreeMap<String, String>>,
}

fn read_opt(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::at(path, e)),
    }
}

fn read_json(path: &Path) -> Result<Json> {
    let text = fs::read_to_string(path).map_err(|e| Error::at(path, e))?;
    serde_json::from_str(&text).map_err(|e| Error::at(path, e))
}

fn expand(root: &Path, patterns: &[String]) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for pattern in patterns {
        let glob = Glob::try_from(pattern.clone()).map_err(Error)?;
        if !pattern.contains(['*', '?']) {
            dirs.push(root.join(pattern));
            continue;
        }
        let Ok(entries) = fs::read_dir(root.join(glob.dir())) else {
            continue;
        };
        let mut found: Vec<PathBuf> = entries
            .filter_map(|e| Some(e.ok()?.path()))
            .filter(|d| {
                d.strip_prefix(root)
                    .ok()
                    .and_then(Path::to_str)
                    .is_some_and(|rel| glob.matches(rel))
            })
            .collect();
        found.sort();
        dirs.extend(found);
    }
    Ok(dirs)
}

fn cargo_tables(m: &Table) -> Vec<&Table> {
    let mut scopes = vec![m];
    scopes.extend(
        m.get("target")
            .and_then(Value::as_table)
            .into_iter()
            .flat_map(|t| t.values().filter_map(Value::as_table)),
    );
    let mut tables: Vec<&Table> = scopes
        .iter()
        .flat_map(|s| CARGO_KINDS.iter().filter_map(|k| s.get(*k)?.as_table()))
        .collect();
    tables.extend(
        m.get("workspace")
            .and_then(|w| w.get("dependencies"))
            .and_then(Value::as_table),
    );
    tables
}

fn cargo(root: &Path) -> Result<BTreeSet<String>> {
    let top = root.join("Cargo.toml");
    if !top.exists() {
        return Ok(BTreeSet::new());
    }
    let top = read_toml(&top)?;
    let members: Vec<String> = top
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| Some(m.as_str()?.to_owned()))
        .collect();
    let mut manifests = vec![top];
    for dir in expand(root, &members)? {
        let path = dir.join("Cargo.toml");
        if path.exists() {
            manifests.push(read_toml(&path)?);
        }
    }
    let (mut named, mut local) = (BTreeSet::new(), BTreeSet::new());
    for (name, spec) in manifests.iter().flat_map(cargo_tables).flatten() {
        if spec.get("path").is_some() {
            local.insert(name.clone());
        } else {
            named.insert(name.clone());
        }
    }
    Ok(named.difference(&local).cloned().collect())
}

fn npm(root: &Path) -> Result<BTreeSet<String>> {
    let top = root.join("package.json");
    if !top.exists() {
        return Ok(BTreeSet::new());
    }
    let top = read_json(&top)?;
    let workspaces = top.get("workspaces");
    let patterns: Vec<String> = workspaces
        .and_then(|w| w.get("packages").or(Some(w)))
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| Some(p.as_str()?.to_owned()))
        .collect();
    let mut manifests = vec![top];
    for dir in expand(root, &patterns)? {
        let path = dir.join("package.json");
        if path.exists() {
            manifests.push(read_json(&path)?);
        }
    }
    let local: BTreeSet<&str> = manifests
        .iter()
        .filter_map(|m| m.get("name")?.as_str())
        .collect();
    let deps = manifests
        .iter()
        .flat_map(|m| NPM_KINDS.iter().filter_map(|k| m.get(*k)?.as_object()))
        .flatten();
    Ok(deps
        .filter(|(name, spec)| {
            let linked = spec
                .as_str()
                .is_some_and(|s| NPM_LOCAL.iter().any(|p| s.starts_with(p)));
            !linked && !local.contains(name.as_str())
        })
        .map(|(name, _)| name.clone())
        .collect())
}

fn swift(root: &Path) -> Result<BTreeSet<String>> {
    let Some(text) = read_opt(&root.join("Package.swift"))? else {
        return Ok(BTreeSet::new());
    };
    Ok(text
        .split(".package(")
        .skip(1)
        .filter_map(|call| {
            let call = call.split(')').next()?;
            let url = call.split_once("url:")?.1.split('"').nth(1)?;
            let name = url.trim_end_matches('/').rsplit('/').next()?;
            Some(name.trim_end_matches(".git").to_lowercase())
        })
        .collect())
}

fn cargo_lock(text: &str) -> Result<usize> {
    let lock: Table = toml::from_str(text).map_err(|e| Error(format!("Cargo.lock: {e}")))?;
    Ok(lock
        .get("package")
        .and_then(Value::as_array)
        .map_or(0, |p| {
            p.iter().filter(|p| p.get("source").is_some()).count()
        }))
}

fn pnpm_lock(text: &str) -> usize {
    text.lines()
        .skip_while(|l| l.trim_end() != "packages:")
        .skip(1)
        .take_while(|l| l.is_empty() || l.starts_with(' '))
        .filter(|l| l.starts_with("  ") && !l.starts_with("   ") && l.trim_end().ends_with(':'))
        .count()
}

fn swift_lock(text: &str) -> Result<usize> {
    let lock: Json =
        serde_json::from_str(text).map_err(|e| Error(format!("Package.resolved: {e}")))?;
    Ok(lock
        .get("pins")
        .or_else(|| lock.get("object")?.get("pins"))
        .and_then(Json::as_array)
        .map_or(0, Vec::len))
}

fn parse(text: &str) -> Result<List> {
    toml::from_str(text).map_err(|e| Error(format!("{LIST}: {e}")))
}

fn used(root: &Path) -> Result<BTreeMap<&'static str, BTreeSet<String>>> {
    ECOSYSTEMS
        .iter()
        .map(|(kind, read)| Ok((*kind, read(root)?)))
        .collect()
}

fn adopt(root: &Path, used: &BTreeMap<&str, BTreeSet<String>>) -> Result<Vec<String>> {
    let mut doc = Table::new();
    doc.insert("growth".into(), Value::Integer(GROWTH.into()));
    for (kind, names) in used.iter().filter(|(_, n)| !n.is_empty()) {
        let empty = names
            .iter()
            .map(|n| (n.clone(), Value::String(String::new())));
        doc.insert((*kind).into(), Value::Table(empty.collect()));
    }
    let path = root.join(LIST);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| Error::at(dir, e))?;
    }
    let text = toml::to_string(&doc).map_err(|e| Error(e.to_string()))?;
    fs::write(&path, text).map_err(|e| Error::at(&path, e))?;
    let count: usize = used.values().map(BTreeSet::len).sum();
    Ok(vec![format!(
        "{LIST}: wrote it with an empty reason for each of the {count} direct dependencies: write each reason and commit it"
    )])
}

fn listed(list: &List, used: &BTreeMap<&str, BTreeSet<String>>) -> Vec<String> {
    let mut findings = Vec::new();
    for (kind, names) in used {
        let entries = list.kinds.get(*kind);
        for name in names
            .iter()
            .filter(|n| !entries.is_some_and(|e| e.contains_key(*n)))
        {
            findings.push(format!(
                "{LIST}: [{kind}] {name}: unlisted direct dependency: ask the owner first, then list it with its reason"
            ));
        }
    }
    for (kind, entries) in &list.kinds {
        let Some(names) = used.get(kind.as_str()) else {
            findings.push(format!(
                "{LIST}: [{kind}] is no ecosystem: cargo, npm or swift"
            ));
            continue;
        };
        for (name, reason) in entries {
            if !names.contains(name) {
                findings.push(format!(
                    "{LIST}: [{kind}] {name}: stale: no manifest uses it, drop the entry"
                ));
            } else if reason.trim().is_empty() {
                findings.push(format!(
                    "{LIST}: [{kind}] {name}: empty reason: write why the repo uses it"
                ));
            }
        }
    }
    findings
}

fn show(root: &Path, base: &str, path: &str) -> Option<String> {
    runner::ok("git", &["show", &format!("{base}:{path}")], root).ok()
}

fn approved(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('#') else {
        return false;
    };
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && rest.get(digits..).is_some_and(|a| !a.trim().is_empty())
}

fn deps_lines(root: &Path, base: &str) -> Result<Vec<String>> {
    let log = runner::ok(
        "git",
        &["log", "--format=%B", &format!("{base}..HEAD")],
        root,
    )?;
    Ok(log
        .lines()
        .filter_map(|l| l.trim().strip_prefix("Deps:"))
        .map(|l| l.trim().to_owned())
        .collect())
}

fn counts(root: &Path, base: Option<&str>) -> Result<Vec<(&'static str, Option<usize>, usize)>> {
    let mut out = Vec::new();
    for (lock, count) in LOCKS {
        let head = read_opt(&root.join(lock))?;
        let old = base.and_then(|b| show(root, b, lock));
        if head.is_none() && old.is_none() {
            continue;
        }
        let now = head.as_deref().map(count).transpose()?.unwrap_or(0);
        let was = match base {
            Some(_) => Some(old.as_deref().map(count).transpose()?.unwrap_or(0)),
            None => None,
        };
        out.push((lock, was, now));
    }
    Ok(out)
}

fn change(was: usize, now: usize) -> String {
    if now >= was {
        format!("+{}", now - was)
    } else {
        format!("-{}", was - now)
    }
}

fn growth(cx: &Cx, list: &List) -> Result<Vec<String>> {
    let base = match cx.base {
        Some(b) => Some(runner::ok("git", &["merge-base", b, "HEAD"], cx.root)?),
        None => None,
    };
    let base = base.as_deref().map(str::trim);
    let counts = counts(cx.root, base)?;
    let shown: Vec<String> = counts
        .iter()
        .map(|(lock, was, now)| match was {
            Some(was) => format!("{lock} {was} -> {now} ({})", change(*was, *now)),
            None => format!("{lock} {now}"),
        })
        .collect();
    if !shown.is_empty() {
        cx.note.set(format!("lockfiles: {}", shown.join(", "))).ok();
    }
    let Some(base) = base else {
        return Ok(Vec::new());
    };
    let was: usize = counts.iter().filter_map(|(_, w, _)| *w).sum();
    let now: usize = counts.iter().map(|(_, _, n)| n).sum();
    let grew = now.saturating_sub(was);
    let limit = show(cx.root, base, LIST)
        .and_then(|t| parse(&t).ok())
        .map_or(list.growth, |old| old.growth.min(list.growth));
    if grew <= usize::try_from(limit).unwrap_or(usize::MAX) {
        return Ok(Vec::new());
    }
    let lines = deps_lines(cx.root, base)?;
    if lines.iter().any(|l| approved(l)) {
        return Ok(Vec::new());
    }
    let mut findings = vec![format!(
        "lockfiles grew by {grew} packages, over the limit of {limit} in {LIST}: ask the owner, then add a commit with the line `Deps: #<issue> <the owner's answer>`"
    )];
    findings.extend(
        lines
            .iter()
            .map(|l| format!("`Deps: {l}` names no `#<issue>` and answer")),
    );
    Ok(findings)
}

/// Every direct dependency is listed with a reason in `.gatekeeper/deps.toml`; the lockfiles grow by at most its `growth`.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let used = used(cx.root)?;
    let Some(text) = read_opt(&cx.root.join(LIST))? else {
        return adopt(cx.root, &used);
    };
    let list = parse(&text)?;
    let mut findings = listed(&list, &used);
    findings.extend(growth(cx, &list)?);
    Ok(findings)
}
