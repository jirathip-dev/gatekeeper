use crate::run::{self, RunCtx};
use crate::{Error, Result, config, packs, plan, runner};
use gatekeeper_model::{Gate, GateId, Host, PackId, Platform, Shard, Skip};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// One gate's fixture run: how many cases it has and what went wrong.
pub struct GateResult {
    pub id: GateId,
    pub cases: usize,
    pub problems: Vec<String>,
    pub skipped: Option<Skip>,
}

/// Where fixture cases run: the host kind and platform, the tool cache, the tools the repo builds itself.
pub struct Bench {
    pub host: Host,
    pub platform: Platform,
    pub cache: PathBuf,
    pub tools: BTreeMap<String, PathBuf>,
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

pub(crate) const CASE_META: [&str; 6] = ["body", "env", "expect", "message", "passes", "remove"];

fn env(prefixes: &[String]) -> Vec<(String, String)> {
    let texts = prefixes
        .iter()
        .filter_map(|p| packs::text(&format!("{p}env")));
    let lines: Vec<String> = texts
        .flat_map(|t| t.lines().map(str::to_owned).collect::<Vec<_>>())
        .collect();
    let pairs = lines.iter().filter_map(|l| l.split_once('='));
    pairs.map(|(k, v)| (k.to_owned(), v.to_owned())).collect()
}

fn write_body(prefix: &str, head: &str, path: &Path) -> Result<Option<PathBuf>> {
    let Some(body) = packs::text(&format!("{prefix}body")) else {
        return Ok(None);
    };
    fs::write(path, body.replace("{head}", head)).map_err(|e| Error::at(path, e))?;
    Ok(Some(path.to_owned()))
}

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

fn run_case(gate: &Gate, case: Option<&str>, out: &Path, bench: &Bench) -> Result<(bool, String)> {
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
    let mut prefixes = vec![format!("{pack}/fixtures/{}/", gate.id.name())];
    let mut pr_body = None;
    if let Some(case) = case {
        let prefix = format!("{pack}/fixtures/{}/{case}/", gate.id.name());
        lay_case(&prefix, &dir)?;
        let head = commit(
            &dir,
            packs::text(&format!("{prefix}message")).unwrap_or(case),
        )?;
        pr_body = write_body(&prefix, &head, &dir.with_extension("body"))?;
        prefixes.push(prefix);
    }
    let config = dir
        .join(config::PATH)
        .exists()
        .then(|| config::load(&dir))
        .transpose()?;
    let cx = RunCtx {
        out: run::out_dir(&dir),
        root: dir.clone(),
        cache: bench.cache.clone(),
        base: Some(base),
        pr_body,
        host: Host::Lane,
        job_start: None,
        manifest: config
            .as_ref()
            .map_or("Cargo.toml", config::manifest)
            .to_owned(),
        results: None,
        ts: config.as_ref().and_then(|c| c.ts.clone()),
        swift: config.and_then(|c| c.swift),
        tools: bench.tools.clone(),
        platform: bench.platform,
        after_reports: None,
        env: env(&prefixes),
        timings: None,
    };
    fs::create_dir_all(&cx.out).map_err(|e| Error::at(&cx.out, e))?;
    let report = run::gate(gate, (Shard::ONE, None), &[], &cx)?;
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
pub fn gate(gate: &Gate, out: &Path, bench: &Bench) -> Result<GateResult> {
    let cases = cases(gate);
    let mut problems = Vec::new();
    if cases.iter().all(|case| passes(gate, case)) {
        problems.push("no failing fixture case".to_owned());
    }
    let (failed, log) = run_case(gate, None, out, bench)?;
    if failed {
        problems.push(format!("clean: failed on the base\n{}", log.trim_end()));
    }
    for case in &cases {
        let want = expect(gate, case);
        let run = run_case(gate, Some(case), out, bench)?;
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
        skipped: None,
    })
}

/// # Errors
pub fn run(
    packs: &[PackId],
    bench: &Bench,
    out: &Path,
    seen: &mut dyn FnMut(&GateResult),
) -> Result<Vec<GateResult>> {
    let mut results = Vec::new();
    for pack in packs {
        for g in packs::pack_gates(*pack)? {
            let result = match plan::ci_only(&g, bench.host) {
                Some(skip) => GateResult {
                    id: g.id.clone(),
                    cases: cases(&g).len(),
                    problems: Vec::new(),
                    skipped: Some(skip),
                },
                None => gate(&g, out, bench)?,
            };
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
