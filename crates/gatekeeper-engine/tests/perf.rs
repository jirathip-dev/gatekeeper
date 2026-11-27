use gatekeeper_engine::cache::{self, Ident, Results};
use gatekeeper_engine::run::{self, RunCtx};
use gatekeeper_engine::{packs, plan};
use gatekeeper_model::{CacheConfig, Config, Filter, Graph, Host, Outcome, Report, Step, Tier};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[path = "support/ci.rs"]
mod ci;

fn copy(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).expect("fixture") {
        let path = entry.expect("entry").path();
        let dest = to.join(path.file_name().expect("name"));
        if path.is_dir() {
            fs::create_dir_all(&dest).expect("dir");
            copy(&path, &dest);
        } else {
            fs::copy(&path, &dest).expect("file");
        }
    }
}

fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(&root).expect("root");
    let packs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    copy(&packs_dir.join("core/fixtures/_base"), &root);
    copy(&packs_dir.join("rust/fixtures/_base"), &root);
    fs::write(root.join(".gitignore"), "target/\ncache/\n").expect("ignore");
    for args in [
        &["init", "-q"][..],
        &["add", "-A"],
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@localhost",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            "base",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .expect("git")
                .success()
        );
    }
    root
}

fn results(root: &Path) -> Results {
    let probes = [("echo 1", "1"), ("rustc -Vv", "rustc 1.91.0")];
    let config: CacheConfig = toml::from_str("recheck = 1000000").expect("cache");
    Results {
        store: root.join("cache/results"),
        read: vec![Tier::Host],
        write: Some(Tier::Host),
        config,
        now: 1_795_000_000,
        salt: "0".into(),
        run: None,
        ident: Ident {
            engine: "e".into(),
            platform: "t os".into(),
            probes: probes
                .map(|(c, o)| (c.to_owned(), o.to_owned()))
                .into_iter()
                .collect::<BTreeMap<_, _>>()
                .into(),
        },
        inputs: cache::inputs::read(root).expect("inputs"),
    }
}

fn check(root: &Path, cached: bool) -> (Duration, Report) {
    let config: Config = toml::from_str(
        "packs = [\"core\", \"rust\"]\nintegration = \"staging\"\npin = \"checkout\"\n",
    )
    .expect("config");
    let start = Instant::now();
    let mut gates = packs::configured(&config, root).expect("gates");
    for gate in &mut gates {
        gate.steps = vec![Step::Run { run: "true".into() }];
        gate.versions = vec!["echo 1".into()];
        gate.cache = true;
    }
    let plan = plan::plan(
        &config,
        gates,
        &|_| Host::Lane,
        &Graph::default(),
        &Filter::default(),
    )
    .expect("plan");
    let mut cx = RunCtx::lane(root, &root.join("cache"));
    cx.results = cached.then(|| results(root));
    let report = run::run(&plan, &cx, &mut |_, _| {}).expect("run");
    let took = start.elapsed();
    assert!(cx.out.join("report.json").is_file());
    (took, report)
}

#[test]
fn check_on_the_base_fixture_with_every_gate_stubbed_takes_under_200_ms() {
    let root = fixture("perf");
    for cached in [false, true] {
        let (took, report) = check(&root, cached);
        let gates: Vec<_> = report.jobs.iter().flat_map(|j| &j.gates).collect();
        assert_eq!(gates.len(), 19);
        assert!(
            gates
                .iter()
                .all(|g| matches!(g.outcome, Outcome::Passed { .. })),
            "{gates:?}"
        );
        ci::assert_under(took, Duration::from_millis(200), &format!("cache {cached}"));
    }
}

#[test]
fn a_check_whose_every_gate_hits_takes_under_200_ms() {
    let root = fixture("perf-hit");
    check(&root, true);
    let (took, report) = check(&root, true);
    let gates: Vec<_> = report.jobs.iter().flat_map(|j| &j.gates).collect();
    assert_eq!(gates.len(), 19);
    assert!(
        gates
            .iter()
            .all(|g| matches!(g.outcome, Outcome::Cached(_))),
        "{gates:?}"
    );
    ci::assert_under(took, Duration::from_millis(200), "every gate hits");
}
