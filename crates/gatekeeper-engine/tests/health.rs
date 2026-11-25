use gatekeeper_engine::health::judge::{self, Evidence};
use gatekeeper_engine::health::{junit, rerun, spans};
use gatekeeper_engine::run::{self, RunCtx};
use gatekeeper_engine::{baseline, plan};
use gatekeeper_model::{
    Case, Config, Date, Filter, Host, RunKind, Runner, Secs, TestId, TestsConfig,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pure {
    #[serde(rename = "about")]
    _about: String,
    runner: Runner,
    #[serde(default = "here")]
    root: String,
    trigger: RunKind,
    runs: Vec<String>,
    #[serde(default)]
    slow: BTreeSet<TestId>,
    #[serde(default)]
    diff: String,
    #[serde(default)]
    head: BTreeMap<String, String>,
    #[serde(default)]
    krates: BTreeMap<String, String>,
    rerun: Vec<TestId>,
    only: String,
    #[serde(default)]
    findings: Vec<String>,
    #[serde(default)]
    warnings: Vec<String>,
    #[serde(default)]
    stale: BTreeSet<TestId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Run {
    #[serde(rename = "about")]
    _about: String,
    gate: String,
    #[serde(default)]
    runs: Vec<String>,
    #[serde(default)]
    base: BTreeMap<String, String>,
    #[serde(default)]
    head: BTreeMap<String, String>,
    #[serde(default)]
    refused: Option<String>,
    #[serde(default)]
    calls: Vec<String>,
    #[serde(default)]
    fails: bool,
    #[serde(default)]
    log: Vec<String>,
    #[serde(default)]
    baseline: Option<String>,
}

fn here() -> String {
    ".".into()
}

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/health")
}

fn cases<T: for<'de> Deserialize<'de>>(want_gate: bool) -> Vec<(String, T)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir()).expect("tests/health") {
        let path = entry.expect("entry").path();
        let Ok(text) = fs::read_to_string(path.join("case.toml")) else {
            continue;
        };
        if text.contains("\ngate = ") != want_gate {
            continue;
        }
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        let case = toml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        out.push((name, case));
    }
    out
}

fn junit_file(name: &str) -> Vec<Case> {
    let text = fs::read_to_string(dir().join("_junit").join(name)).expect("junit");
    junit::read(&text).unwrap_or_else(|e| panic!("{name}: {}", e.0))
}

fn ids(cases: &[&Case]) -> Vec<TestId> {
    cases.iter().map(|c| c.id()).collect()
}

#[test]
fn each_pure_case_reruns_and_judges_as_its_folder_expects() {
    let mut seen = 0;
    for (name, case) in cases::<Pure>(false) {
        let runs: Vec<Vec<Case>> = case.runs.iter().map(|f| junit_file(f)).collect();
        let touched = spans::hunks(&case.diff);
        let tests = TestsConfig::default();
        let changed = spans::changed(&touched, &case.head, &case.krates, &tests);
        let e = Evidence {
            runs: &runs,
            changed: &changed,
            slow: &case.slow,
            budget: Secs(2),
            kind: case.trigger,
            root: &case.root,
        };
        let again = judge::rerun_set(runs.first().expect("a first run"), &e);
        assert_eq!(ids(&again), case.rerun, "{name}: the rerun set");
        assert_eq!(
            rerun::only(case.runner, &again),
            case.only,
            "{name}: {{only}}"
        );
        let verdict = judge::verdict(&e);
        assert_eq!(verdict.findings, case.findings, "{name}: findings");
        assert_eq!(verdict.warnings, case.warnings, "{name}: warnings");
        let stale: BTreeSet<TestId> = case.slow.difference(&verdict.kept).cloned().collect();
        assert_eq!(stale, case.stale, "{name}: stale");
        seen += 1;
    }
    assert_eq!(seen, 9);
}

#[test]
fn junit_files_of_both_runners_read_every_case() {
    let nextest = junit_file("nextest.xml");
    let vitest = junit_file("vitest.xml");
    assert_eq!(nextest.len(), 4);
    assert_eq!(vitest.len(), 4);
    let failed = |cases: &[Case]| {
        let f = cases
            .iter()
            .filter(|c| c.ended == gatekeeper_model::Ended::Failed);
        f.map(Case::id).collect::<Vec<_>>()
    };
    assert_eq!(
        failed(&nextest),
        [TestId("demo-app tests::fails_on_purpose".into())]
    );
    assert_eq!(
        failed(&vitest),
        [TestId("src/board.test.ts board > sums & <counts>".into())]
    );
    assert!(junit::read("<html></html>").is_err());
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@localhost"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn lay_out(root: &Path, files: &BTreeMap<String, String>) {
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        fs::write(path, text).expect("file");
    }
}

fn repo(name: &str, case: &Run) -> (PathBuf, String) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("health")
        .join(name);
    fs::remove_dir_all(&root).ok();
    fs::create_dir_all(&root).expect("root");
    lay_out(&root, &case.base);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "--allow-empty", "-m", "base"]);
    let sha = git(&root, &["rev-parse", "HEAD"]);
    lay_out(&root, &case.head);
    fs::create_dir_all(root.join(".gatekeeper/gates")).expect("gates");
    fs::write(root.join(".gatekeeper/gates/test.toml"), &case.gate).expect("gate");
    fs::copy(dir().join("_runner.sh"), root.join("runner.sh")).expect("runner");
    for (n, file) in (1..).zip(&case.runs) {
        fs::copy(
            dir().join("_junit").join(file),
            root.join(format!("run{n}.xml")),
        )
        .expect("run");
    }
    (root, sha)
}

#[test]
fn a_timed_gate_runs_through_a_stand_in_runner_as_its_folder_expects() {
    let config: Config =
        toml::from_str("packs = []\nintegration = \"staging\"\npin = \"checkout\"\n")
            .expect("config");
    let mut seen = 0;
    for (name, case) in cases::<Run>(true) {
        let (root, sha) = repo(&name, &case);
        seen += 1;
        let plan = plan::repo(&config, &root, &|_| Host::Lane, &Filter::default());
        if let Some(want) = &case.refused {
            let got = plan.err().map(|e| e.0).unwrap_or_default();
            assert!(got.contains(want.as_str()), "{name}: refused with {got}");
            continue;
        }
        let plan = plan.expect("plan");
        let cache = root.join("cache");
        if let Some(want) = &case.baseline {
            let gate = &plan.jobs[0].gates[0].gate;
            let today: Date = "2026-11-25".parse().expect("date");
            baseline::write(gate, &root, &cache, (&config, today)).expect("baseline");
            let got = fs::read_to_string(baseline::path(&root, &gate.id)).expect("slow list");
            assert_eq!(&got, want, "{name}");
            continue;
        }
        let mut cx = RunCtx::lane(&root, &cache);
        cx.base = Some(sha);
        let report = run::run(&plan, &cx, &mut |_, _| {}).expect("run");
        let gate = &report.jobs[0].gates[0];
        assert_eq!(
            gate.outcome.fails(),
            case.fails,
            "{name}: {:?}",
            gate.outcome
        );
        let calls = fs::read_to_string(root.join("calls")).unwrap_or_default();
        assert_eq!(
            calls.lines().collect::<Vec<_>>(),
            case.calls,
            "{name}: calls"
        );
        let log = fs::read_to_string(root.join(&gate.log)).expect("log");
        for line in &case.log {
            assert!(
                log.contains(line.as_str()),
                "{name}: the log lacks `{line}`:\n{log}"
            );
        }
    }
    assert_eq!(seen, 4);
}
