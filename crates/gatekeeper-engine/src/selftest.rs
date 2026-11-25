use crate::run::{self, RunCtx};
use crate::{Error, Result, config, packs, plan, runner};
use gatekeeper_model::{Gate, GateId, Host, PackId, Platform, PrivateNames, Shard, Skip};
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

pub(crate) const CASE_META: [&str; 9] = [
    "body",
    "checks.json",
    "env",
    "expect",
    "message",
    "names",
    "passes",
    "release",
    "remove",
];

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

fn write_body(prefix: &str, (base, head): (&str, &str), path: &Path) -> Result<Option<PathBuf>> {
    let Some(body) = packs::text(&format!("{prefix}body")) else {
        return Ok(None);
    };
    fs::write(path, body.replace("{head}", head).replace("{base}", base))
        .map_err(|e| Error::at(path, e))?;
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

fn typed<'a>(env: &'a [(String, String)], key: &str) -> &'a str {
    let last = env.iter().rev().find(|(k, _)| k == key);
    last.map_or("", |(_, v)| v.as_str())
}

/// # Errors
pub fn case_ctx(gate: &Gate, case: Option<&str>, out: &Path, bench: &Bench) -> Result<RunCtx> {
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
    let mut head = base.clone();
    let mut prefixes = vec![format!("{pack}/fixtures/{}/", gate.id.name())];
    let mut pr_body = None;
    let mut names = None;
    let mut release = false;
    if let Some(case) = case {
        let prefix = format!("{pack}/fixtures/{}/{case}/", gate.id.name());
        lay_case(&prefix, &dir)?;
        head = commit(
            &dir,
            packs::text(&format!("{prefix}message")).unwrap_or(case),
        )?;
        pr_body = write_body(&prefix, (&base, &head), &dir.with_extension("body"))?;
        names = packs::text(&format!("{prefix}names"));
        release = packs::text(&format!("{prefix}release")).is_some();
        prefixes.push(prefix);
    }
    let config = dir
        .join(config::PATH)
        .exists()
        .then(|| config::load(&dir))
        .transpose()?;
    if let Some(c) = config.as_ref().filter(|_| release) {
        let branch = format!("refs/remotes/origin/{}", c.integration);
        runner::ok("git", &["update-ref", &branch, &head], &dir)?;
    }
    let mut cx = RunCtx::lane(&dir, &bench.cache);
    cx.base = Some(base);
    cx.pr_body = pr_body;
    cx.env = env(&prefixes);
    cx.names = PrivateNames::parse(names.unwrap_or(typed(&cx.env, "DROVER_PRIVATE_NAMES")));
    let given = typed(&cx.env, "GATEKEEPER_HEAD");
    cx.head = Some(if given.is_empty() {
        head
    } else {
        given.to_owned()
    });
    cx.integration = config.as_ref().map_or_default(|c| c.integration.clone());
    cx.rust = config.as_ref().map_or_default(|c| c.rust.clone());
    cx.ts = config.as_ref().and_then(|c| c.ts.clone());
    cx.docs = config.as_ref().map_or_default(|c| c.docs.clone());
    cx.swift = config.and_then(|c| c.swift);
    cx.tools = bench.tools.clone();
    cx.platform = bench.platform;
    fs::create_dir_all(&cx.out).map_err(|e| Error::at(&cx.out, e))?;
    Ok(cx)
}

fn run_case(gate: &Gate, case: Option<&str>, out: &Path, bench: &Bench) -> Result<(bool, String)> {
    let cx = case_ctx(gate, case, out, bench)?;
    let report = run::gate(gate, (Shard::ONE, None), &[], &cx)?;
    let log = fs::read_to_string(cx.root.join(&report.log)).unwrap_or_default();
    Ok((report.outcome.fails(), log))
}

/// # Errors
pub fn write_case(id: &str, to: &Path) -> Result<Vec<&'static str>> {
    let parts: Vec<&str> = id.split('/').collect();
    let [pack, gate, case] = parts[..] else {
        return Err(Error(format!(
            "fixture `{id}`: name it <pack>/<gate>/<case>"
        )));
    };
    let prefix = format!("{pack}/fixtures/{gate}/{case}/");
    let files: Vec<_> = packs::under(&prefix).collect();
    if case.is_empty() || case == "_base" || files.is_empty() {
        return Err(Error(format!("fixture `{id}`: no such case")));
    }
    if to.exists() {
        fs::remove_dir_all(to).map_err(|e| Error::at(to, e))?;
    }
    packs::materialize(&prefix, to)?;
    Ok(files.into_iter().map(|(rel, _)| rel).collect())
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

fn report_differs(gate: &Gate, case: &str, out: &Path) -> Option<String> {
    let (pack, name) = (gate.id.pack, gate.id.name());
    let want = packs::text(&format!("{pack}/fixtures/{name}/{case}/checks.json"))?;
    let dir = out.join(pack.as_str()).join(name).join(case);
    let checks = fs::read_dir(run::out_dir(&dir).join("check"));
    let got = (checks.into_iter().flatten().flatten())
        .find_map(|e| fs::read_to_string(e.path().join("checks.json")).ok());
    (got.as_deref() != Some(want)).then(|| {
        format!(
            "{case}: the check wrote another checks.json\n{}",
            got.unwrap_or_default().trim_end()
        )
    })
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
        problems.extend(report_differs(gate, case, out));
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
