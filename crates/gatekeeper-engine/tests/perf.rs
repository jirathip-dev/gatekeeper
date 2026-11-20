use gatekeeper_engine::run::{self, RunCtx};
use gatekeeper_engine::{packs, plan};
use gatekeeper_model::{Config, Filter, Graph, Host, Step};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

#[test]
fn check_on_the_base_fixture_with_every_gate_stubbed_takes_under_200_ms() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("perf");
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(&root).expect("root");
    let packs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    copy(&packs_dir.join("core/fixtures/_base"), &root);
    copy(&packs_dir.join("rust/fixtures/_base"), &root);
    let config: Config = toml::from_str(
        "packs = [\"core\", \"rust\"]\nintegration = \"staging\"\npin = \"checkout\"\n",
    )
    .expect("config");
    let start = Instant::now();
    let mut gates = packs::configured(&config, &root).expect("gates");
    for gate in &mut gates {
        gate.steps = vec![Step::Run { run: "true".into() }];
    }
    let plan = plan::plan(
        &config,
        gates,
        &|_| Host::Lane,
        &Graph::default(),
        &Filter::default(),
    )
    .expect("plan");
    let cx = RunCtx {
        out: run::out_dir(&root),
        root: root.clone(),
        cache: root.join("cache"),
        base: None,
        pr_body: None,
        host: Host::Lane,
        job_start: None,
        manifest: "Cargo.toml".into(),
        after_reports: None,
    };
    let report = run::run(&plan, &cx, &mut |_, _| {}).expect("run");
    let took = start.elapsed();
    let gates: Vec<_> = report.jobs.iter().flat_map(|j| &j.gates).collect();
    assert_eq!(gates.len(), 8);
    assert!(gates.iter().all(|g| !g.outcome.fails()));
    assert!(cx.out.join("report.json").is_file());
    assert!(took < Duration::from_millis(200), "took {took:?}");
}
