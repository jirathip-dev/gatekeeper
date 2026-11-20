use crate::run::{self, RunCtx};
use crate::{Error, Result, packs, runner};
use gatekeeper_model::{Gate, GateId, Host, PackId};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// One gate's fixture run: how many cases it has and what went wrong.
pub struct GateResult {
    pub id: GateId,
    pub cases: usize,
    pub problems: Vec<String>,
}

const GIT: [&str; 6] = [
    "-c",
    "user.name=gatekeeper",
    "-c",
    "user.email=gatekeeper@localhost",
    "-c",
    "commit.gpgsign=false",
];

fn commit(dir: &Path, message: &str) -> Result<String> {
    runner::ok("git", &["add", "-A"], dir)?;
    let mut args = GIT.to_vec();
    args.extend(["commit", "-q", "--allow-empty", "-m", message]);
    runner::ok("git", &args, dir)?;
    Ok(runner::ok("git", &["rev-parse", "HEAD"], dir)?
        .trim()
        .to_owned())
}

pub(crate) const CASE_META: [&str; 4] = ["expect", "message", "passes", "remove"];

fn lay_case(prefix: &str, dir: &Path) -> Result<()> {
    for (rel, bytes) in packs::under(prefix) {
        if rel == "remove" {
            for gone in String::from_utf8_lossy(bytes)
                .lines()
                .filter(|l| !l.is_empty())
            {
                let path = dir.join(gone);
                fs::remove_file(&path).map_err(|e| Error::at(&path, e))?;
            }
        } else if !CASE_META.contains(&rel) {
            let path = dir.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::at(parent, e))?;
            }
            fs::write(&path, bytes).map_err(|e| Error::at(&path, e))?;
        }
    }
    Ok(())
}

fn run_case(gate: &Gate, case: Option<&str>, out: &Path, cache: &Path) -> Result<(bool, String)> {
    let pack = gate.id.pack;
    let dir = out
        .join(pack.as_str())
        .join(gate.id.name())
        .join(case.unwrap_or("clean"));
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| Error::at(&dir, e))?;
    }
    packs::materialize(&format!("{pack}/fixtures/_base/"), &dir)?;
    runner::ok("git", &["init", "-q"], &dir)?;
    let base = commit(&dir, "base")?;
    if let Some(case) = case {
        let prefix = format!("{pack}/fixtures/{}/{case}/", gate.id.name());
        lay_case(&prefix, &dir)?;
        commit(
            &dir,
            packs::text(&format!("{prefix}message")).unwrap_or(case),
        )?;
    }
    let cx = RunCtx {
        out: run::out_dir(&dir),
        root: dir.clone(),
        cache: cache.to_path_buf(),
        base: Some(base),
        pr_body: None,
        host: Host::Lane,
        job_start: None,
        manifest: "Cargo.toml".into(),
    };
    fs::create_dir_all(&cx.out).map_err(|e| Error::at(&cx.out, e))?;
    let report = run::gate(gate, &cx)?;
    let log = fs::read_to_string(dir.join(&report.log)).unwrap_or_default();
    Ok((report.outcome.fails(), log))
}

pub(crate) fn cases(gate: &Gate) -> BTreeSet<&'static str> {
    let prefix = format!("{}/fixtures/{}/", gate.id.pack, gate.id.name());
    packs::under(&prefix)
        .filter_map(|(rel, _)| rel.split_once('/').map(|(case, _)| case))
        .collect()
}

pub(crate) fn passes(gate: &Gate, case: &str) -> bool {
    let path = format!("{}/fixtures/{}/{case}/passes", gate.id.pack, gate.id.name());
    packs::text(&path).is_some()
}

pub(crate) fn expect(gate: &Gate, case: &str) -> String {
    let path = format!("{}/fixtures/{}/{case}/expect", gate.id.pack, gate.id.name());
    packs::text(&path).unwrap_or_default().trim().to_owned()
}

/// # Errors
pub fn gate(gate: &Gate, out: &Path, cache: &Path) -> Result<GateResult> {
    let cases = cases(gate);
    let mut problems = Vec::new();
    if cases.iter().all(|case| passes(gate, case)) {
        problems.push("no failing fixture case".to_owned());
    }
    let (failed, log) = run_case(gate, None, out, cache)?;
    if failed {
        problems.push(format!("clean: failed on the base\n{}", log.trim_end()));
    }
    for case in &cases {
        let want = expect(gate, case);
        let run = run_case(gate, Some(case), out, cache)?;
        if passes(gate, case) {
            if run.0 {
                problems.push(format!(
                    "{case}: failed; it must pass\n{}",
                    run.1.trim_end()
                ));
            }
            continue;
        }
        match run {
            (false, _) => problems.push(format!("{case}: passed; it must fail")),
            (true, log) if want.is_empty() || !log.contains(&want) => problems.push(format!(
                "{case}: failed without printing `{want}`\n{}",
                log.trim_end()
            )),
            (true, _) => {}
        }
    }
    Ok(GateResult {
        id: gate.id.clone(),
        cases: cases.len(),
        problems,
    })
}

/// # Errors
pub fn run(
    packs: &[PackId],
    out: &Path,
    cache: &Path,
    seen: &mut dyn FnMut(&GateResult),
) -> Result<Vec<GateResult>> {
    let mut results = Vec::new();
    for pack in packs {
        for g in packs::pack_gates(*pack)? {
            let result = gate(&g, out, cache)?;
            seen(&result);
            results.push(result);
        }
    }
    Ok(results)
}

#[must_use]
pub fn out_dir(root: &Path) -> PathBuf {
    run::out_dir(root).join("selftest")
}
