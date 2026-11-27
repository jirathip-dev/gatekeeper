use crate::run::{self, CheckCx};
use crate::{Error, Result, check, config, ratchet};
use gatekeeper_model::{Config, Date, Gate, GateId, Step};
use std::cell::OnceCell;
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[must_use]
pub fn path(root: &Path, id: &GateId) -> PathBuf {
    root.join(".gatekeeper/baseline")
        .join(format!("{}.txt", id.file_stem()))
}

fn lines(text: &str) -> impl Iterator<Item = String> + '_ {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
}

pub(crate) fn findings(name: &str, cx: &CheckCx) -> Vec<String> {
    let found = check::run(name, cx).unwrap_or_else(|e| vec![e.0]);
    found.iter().flat_map(|f| lines(f)).collect()
}

fn read(path: &Path) -> Result<Option<BTreeSet<String>>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(lines(&text).collect())),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::at(path, e)),
    }
}

pub(crate) struct Baseline {
    listed: BTreeSet<String>,
    met: BTreeSet<String>,
}

impl Baseline {
    pub(crate) fn load(root: &Path, id: &GateId) -> Result<Baseline> {
        Ok(Baseline {
            listed: read(&path(root, id))?.unwrap_or_default(),
            met: BTreeSet::new(),
        })
    }

    pub(crate) fn unlisted(&mut self, findings: Vec<String>) -> Vec<String> {
        let new = findings
            .iter()
            .filter(|f| !self.listed.contains(*f))
            .cloned()
            .collect();
        self.met.extend(findings);
        new
    }

    pub(crate) fn listed(&self) -> impl Iterator<Item = &String> {
        self.listed.iter()
    }

    pub(crate) fn stale(&self) -> impl Iterator<Item = &String> {
        self.listed.difference(&self.met)
    }
}

/// What `baseline <gate>` left: the file, its lines, and the new findings it would not add.
pub struct Written {
    pub path: PathBuf,
    pub lines: usize,
    pub unlisted: Vec<String>,
    /// What a writer says instead of the count of its lines.
    pub note: Option<String>,
}

fn check_steps(gate: &Gate) -> Vec<&String> {
    let checks = gate.steps.iter().filter_map(|s| match s {
        Step::Check { check } => Some(check),
        _ => None,
    });
    checks.collect()
}

fn with_cx<R>(
    name: &str,
    (root, cache): (&Path, &Path),
    (config, today): (&Config, Date),
    f: impl FnOnce(&CheckCx) -> R,
) -> R {
    let scratch = run::out_dir(root).join("check").join(name);
    f(&CheckCx {
        root,
        scratch: &scratch,
        cache,
        rust: &config.rust,
        ts: config.ts.as_ref(),
        swift: config.swift.as_ref(),
        docs: &config.docs,
        tools: &config::tools(config, root),
        base: None,
        pushed: None,
        names: None,
        head: None,
        pr_body: None,
        integration: &config.integration,
        note: &OnceCell::new(),
        tests: &config.tests,
        today,
    })
}

fn now(checks: Vec<&String>, at: (&Path, &Path), config: (&Config, Date)) -> BTreeSet<String> {
    let each = checks.into_iter();
    let found = each.flat_map(|name| with_cx(name, at, config, |cx| findings(name, cx)));
    found.collect()
}

fn ceilings(name: &str, root: &Path, cache: &Path, at: (&Config, Date)) -> Result<Written> {
    let measured = with_cx(name, (root, cache), at, |cx| match name {
        "surface" => check::surface::measure(cx).map(|(c, _)| c),
        _ => check::duplication::measure(cx).map(|(c, _, _)| vec![c]),
    });
    let mut each = Vec::new();
    for c in measured? {
        let fits = ratchet::judge::fits(c.n, c.step);
        let (dir, what) = (c.name.dir(), c.name.to_string());
        each.push(ratchet::folder::write(
            root,
            (&dir, &what),
            at.1,
            (fits, fits),
        )?);
    }
    if each.len() == 1 {
        return Ok(each.remove(0));
    }
    let said = each.iter().map(|w| {
        let path = w.path.strip_prefix(root).unwrap_or(&w.path);
        format!(
            "{}: {}",
            path.display(),
            w.note.as_deref().unwrap_or_default()
        )
    });
    let said: Vec<String> = said.collect();
    Ok(Written {
        path: root.join("ci/ratchet"),
        lines: each.iter().map(|w| w.lines).sum(),
        unlisted: Vec::new(),
        note: Some(if said.is_empty() {
            "no [surface]: nothing is counted".to_owned()
        } else {
            said.join("; ")
        }),
    })
}

/// # Errors
pub fn write(
    gate: &Gate,
    root: &Path,
    cache: &Path,
    (config, today): (&Config, Date),
) -> Result<Written> {
    let checks = check_steps(gate);
    if checks == ["arch"] {
        return crate::arch::baseline::write(root);
    }
    if checks == ["test-health"] {
        let lines = check::test_health::lines(root, &config.tests)?;
        return crate::health::ratchet::write(root, today, lines);
    }
    if let [name] = checks[..]
        && (name == "duplication" || name == "surface")
    {
        return ceilings(name, root, cache, (config, today));
    }
    let now = match &gate.timed {
        Some(timed) => {
            let mut cx = run::RunCtx::lane(root, cache);
            cx.tools = config::tools(config, root);
            cx.tests = config.tests.clone();
            crate::health::timed::slow(gate, timed, &cx)?
        }
        None if checks.is_empty() => {
            return Err(Error(format!(
                "{}: no check step, so no findings to list",
                gate.id
            )));
        }
        None => now(checks, (root, cache), (config, today)),
    };
    let path = path(root, &gate.id);
    let (keep, unlisted): (BTreeSet<String>, Vec<String>) = match read(&path)? {
        None => (now, Vec::new()),
        Some(old) => (
            old.intersection(&now).cloned().collect(),
            now.difference(&old).cloned().collect(),
        ),
    };
    if keep.is_empty() {
        if path.exists() {
            fs::remove_file(&path).map_err(|e| Error::at(&path, e))?;
        }
    } else {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| Error::at(dir, e))?;
        }
        let text = keep.iter().fold(String::new(), |t, l| t + l + "\n");
        fs::write(&path, text).map_err(|e| Error::at(&path, e))?;
    }
    Ok(Written {
        path,
        lines: keep.len(),
        unlisted,
        note: None,
    })
}
