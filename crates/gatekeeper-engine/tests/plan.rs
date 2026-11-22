use gatekeeper_engine::run::{self, RunCtx};
use gatekeeper_engine::{ci, plan};
use gatekeeper_model::{Config, Filter, Host, job_label};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Expect {
    host: Host,
    leg: Option<String>,
    #[serde(default)]
    after_reports: bool,
    jobs: Vec<String>,
    #[serde(default)]
    outcomes: BTreeMap<String, String>,
    #[serde(default)]
    logs: BTreeMap<String, String>,
    done: Option<Vec<String>>,
}

fn repo(case: &Path, name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("plan")
        .join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(root.join(".gatekeeper/gates")).expect("gates dir");
    fs::copy(
        case.join("config.toml"),
        root.join(".gatekeeper/config.toml"),
    )
    .expect("config");
    for entry in fs::read_dir(case.join("gates")).expect("gates") {
        let path = entry.expect("entry").path();
        let to = root
            .join(".gatekeeper/gates")
            .join(path.file_name().expect("name"));
        fs::copy(&path, to).expect("gate");
    }
    root
}

fn ctx(root: &Path, case: &Path, expect: &Expect) -> RunCtx {
    RunCtx {
        out: run::out_dir(root),
        root: root.to_path_buf(),
        cache: root.join("cache"),
        base: None,
        pr_body: None,
        host: expect.host,
        job_start: None,
        manifest: "Cargo.toml".into(),
        after_reports: expect.after_reports.then(|| case.join("reports")),
        ts: None,
        results: None,
        swift: None,
        tools: std::collections::BTreeMap::new(),
        platform: run::platform(),
        env: Vec::new(),
        timings: None,
    }
}

fn run_case(case: &Path) {
    let name = case.file_name().expect("name").to_string_lossy();
    let root = repo(case, &name);
    let text = fs::read_to_string(case.join("expect.toml")).expect("expect");
    let expect: Expect = toml::from_str(&text).expect("expect toml");
    let text = fs::read_to_string(case.join("config.toml")).expect("config");
    let config: Config = toml::from_str(&text).expect("config toml");
    let filter = Filter {
        leg: expect.leg.clone(),
        ..Filter::default()
    };
    let plan = plan::repo(&config, &root, &|_| expect.host, &filter).expect("plan");
    let jobs: Vec<String> = plan
        .jobs
        .iter()
        .map(|j| job_label(&j.leg, j.shard))
        .collect();
    assert_eq!(jobs, expect.jobs, "{name}");
    if let Some(done) = &expect.done {
        let (_, problems) = ci::done(&plan, &case.join("reports"), None).expect("done");
        assert_eq!(problems.len(), done.len(), "{name}: {problems:?}");
        for (got, want) in problems.iter().zip(done) {
            assert!(got.starts_with(want.as_str()), "{name}: {got}");
        }
    }
    if expect.outcomes.is_empty() {
        return;
    }
    let cx = ctx(&root, case, &expect);
    let report = run::run(&plan, &cx, &mut |_, _| {}).expect("run");
    let got: BTreeMap<String, String> = report
        .jobs
        .iter()
        .flat_map(|j| &j.gates)
        .map(|g| (g.id.to_string(), g.outcome.word().to_owned()))
        .collect();
    assert_eq!(got, expect.outcomes, "{name}");
    for (gate, line) in &expect.logs {
        let log = format!("target/gatekeeper/{}.log", gate.replace('/', "-"));
        let text = fs::read_to_string(root.join(log)).expect("log");
        assert!(
            text.contains(line.as_str()),
            "{name}: {gate} log lacks `{line}`:\n{text}"
        );
    }
}

#[test]
fn each_shards_and_legs_case_plans_and_runs_as_its_folder_expects() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/plan");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).expect("tests/plan") {
        run_case(&entry.expect("entry").path());
        seen += 1;
    }
    assert_eq!(seen, 8);
}
