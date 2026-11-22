use gatekeeper_engine::cache::{self, Ident, Results};
use gatekeeper_engine::fetch::{self, ToolFile};
use gatekeeper_engine::plan;
use gatekeeper_engine::run::{self, RunCtx};
use gatekeeper_model::{
    CacheMode, Change, ChangeKind, Config, Event, Filter, Host, RunRef, Trigger,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    gate: toml::Table,
    #[serde(default)]
    files: BTreeMap<String, String>,
    #[serde(default)]
    cache: Option<toml::Table>,
    #[serde(default)]
    fake_tool: Option<String>,
    #[serde(default)]
    scope: Option<toml::Table>,
    #[serde(default)]
    ts: Option<toml::Table>,
    run: Vec<Run>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Run {
    trigger: String,
    expect: String,
    #[serde(default)]
    contains: Option<String>,
    #[serde(default)]
    edit: BTreeMap<String, String>,
    #[serde(default)]
    gate: Option<toml::Table>,
    #[serde(default)]
    versions: BTreeMap<String, String>,
    #[serde(default)]
    engine: Option<String>,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    full: bool,
    #[serde(default)]
    no_cache: bool,
    #[serde(default)]
    age_days: u64,
    #[serde(default)]
    forge: Option<Forge>,
    #[serde(default)]
    expect_store: BTreeMap<String, usize>,
    #[serde(default)]
    same: Vec<String>,
    #[serde(default)]
    changed: Option<BTreeSet<String>>,
    #[serde(default)]
    ci: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Forge {
    tier: String,
    what: String,
    #[serde(default)]
    to: Option<String>,
}

const NOW: u64 = 1_795_000_000;

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@localhost"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn write(root: &Path, files: &BTreeMap<String, String>) {
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        fs::write(path, text).expect("file");
    }
}

fn write_gate(root: &Path, gate: &toml::Table) {
    let mut gate = gate.clone();
    for (k, v) in [("about", "a case"), ("fix", "none")] {
        gate.entry(k).or_insert(v.into());
    }
    gate.entry("budget_secs").or_insert(5.into());
    let text = toml::to_string(&gate).expect("gate toml");
    fs::write(root.join(".gatekeeper/gates/g.toml"), text).expect("gate");
}

fn event(trigger: &str) -> Event {
    match trigger.split_once(' ') {
        None if trigger == "lane" => Event::Lane,
        Some(("pr", n)) => Event::PullRequest {
            number: n.parse().expect("pr number"),
            fork: false,
        },
        Some(("fork", n)) => Event::PullRequest {
            number: n.parse().expect("pr number"),
            fork: true,
        },
        Some(("push", branch)) => Event::Push {
            branch: branch.into(),
        },
        _ if trigger == "other" => Event::Other,
        _ => panic!("trigger `{trigger}`"),
    }
}

fn records(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let Ok(entries) = fs::read_dir(dir) else {
        return BTreeMap::new();
    };
    entries
        .map(|e| e.expect("entry").path())
        .map(|p| {
            (
                p.to_string_lossy().into_owned(),
                fs::read(&p).expect("read"),
            )
        })
        .collect()
}

fn forge(store: &Path, forge: &Forge) {
    let dir = store.join(&forge.tier);
    for (path, bytes) in records(&dir) {
        let mut record: serde_json::Value = serde_json::from_slice(&bytes).expect("record");
        match forge.what.as_str() {
            "key" => record["key"] = "0".repeat(64).into(),
            "parts" => record["parts"]["inputs"] = "forged".into(),
            "move" => {
                let to = store.join(forge.to.as_deref().expect("to"));
                fs::create_dir_all(&to).expect("to");
                let name = Path::new(&path).file_name().expect("name");
                fs::rename(&path, to.join(name)).expect("move");
                continue;
            }
            other => panic!("forge `{other}`"),
        }
        fs::write(&path, record.to_string()).expect("forged");
    }
}

fn plant_tool(name: &str, cache: &Path) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../packs/core/tools/{name}.toml"));
    let file: ToolFile = toml::from_str(&fs::read_to_string(path).expect("tool")).expect("toml");
    let ToolFile::Binary(file) = file else {
        panic!("a binary tool")
    };
    let asset = &file.asset[fetch::host_target().expect("a pinned target")];
    let dir = cache.join("tools").join(&asset.sha256);
    fs::create_dir_all(&dir).expect("tool dir");
    let bin = dir.join(asset.bin.as_deref().unwrap_or("tool"));
    fs::write(&bin, "#!/bin/sh\nexit 0\n").expect("bin");
    Command::new("chmod")
        .arg("+x")
        .arg(&bin)
        .status()
        .expect("chmod");
}

fn results(config: &Config, run: &Run, root: &Path, store: &Path, i: u64) -> Option<Results> {
    let event = event(&run.trigger);
    let mode = cache::mode(
        config.cache.as_ref(),
        &event,
        run.full,
        run.no_cache,
        "staging",
    );
    let CacheMode::Use { read, write } = mode else {
        return None;
    };
    let ci = (event != Event::Lane).then(|| RunRef {
        repo: "o/r".into(),
        run: 100 + i,
        attempt: 1,
    });
    Some(Results {
        store: store.to_path_buf(),
        read,
        write,
        config: config.cache.clone().expect("cache"),
        now: NOW + run.age_days * 86_400,
        salt: i.to_string(),
        run: ci,
        ident: Ident {
            engine: run.engine.clone().unwrap_or("e1".into()),
            platform: run.platform.clone().unwrap_or("os1".into()),
            probes: run.versions.clone().into(),
        },
        inputs: cache::inputs::read(root).expect("inputs"),
    })
}

fn run_case(case_dir: &Path) {
    let name = case_dir
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    let text = fs::read_to_string(case_dir.join("case.toml")).expect("case.toml");
    let case: Case = toml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("cache")
        .join(&name);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clean");
    }
    let (root, tools) = (dir.join("repo"), dir.join("tools"));
    fs::create_dir_all(root.join(".gatekeeper/gates")).expect("gates dir");
    fs::write(root.join(".gitignore"), "target/\n").expect("ignore");
    write(&root, &case.files);
    if case.files.contains_key("node_modules/.bin/vitest") {
        let bin = root.join("node_modules/.bin/vitest");
        let chmod = Command::new("chmod").arg("+x").arg(bin).status();
        assert!(chmod.expect("chmod").success());
    }
    write_gate(&root, &case.gate);
    if let Some(tool) = &case.fake_tool {
        plant_tool(tool, &tools);
    }
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "base"]);
    let mut config = "packs = []\nintegration = \"staging\"\npin = \"checkout\"\n".to_owned();
    if let Some(table) = &case.scope {
        config.push_str("[scope]\n");
        config.push_str(&toml::to_string(table).expect("scope"));
    }
    if let Some(table) = &case.ts {
        config.push_str("[ts]\n");
        config.push_str(&toml::to_string(table).expect("ts"));
    }
    if let Some(table) = &case.cache {
        let mut table = table.clone();
        table.entry("recheck").or_insert(1_000_000.into());
        config.push_str("[cache]\n");
        config.push_str(&toml::to_string(&table).expect("cache"));
    }
    let config: Config = toml::from_str(&config).expect("config");
    for (i, step) in case.run.iter().enumerate() {
        run_step(
            &format!("{name} run {}", i + 1),
            step,
            &config,
            &dir,
            u64::try_from(i).expect("index"),
        );
    }
}

#[test]
fn each_cache_case_reports_what_its_runs_expect() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cache");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).expect("tests/cache") {
        run_case(&entry.expect("entry").path());
        seen += 1;
    }
    assert_eq!(seen, 39);
}

fn run_step(at: &str, step: &Run, config: &Config, dir: &Path, i: u64) {
    let (root, store, tools) = (dir.join("repo"), dir.join("store"), dir.join("tools"));
    write(&root, &step.edit);
    if let Some(gate) = &step.gate {
        write_gate(&root, gate);
    }
    if let Some(f) = &step.forge {
        forge(&store, f);
    }
    let before: Vec<_> = step.same.iter().map(|t| records(&store.join(t))).collect();
    let host = if step.ci {
        Host::GithubHosted { warm_cache: true }
    } else {
        Host::Lane
    };
    let filter = Filter {
        trigger: step.changed.clone().map_or_else(Trigger::default, |files| {
            let modified = |path| Change {
                path,
                kind: ChangeKind::Modified,
            };
            Trigger::Change(files.into_iter().map(modified).collect())
        }),
        ..Filter::default()
    };
    let plan = plan::repo(config, &root, &|_| host, &filter).expect("plan");
    let cx = RunCtx {
        out: run::out_dir(&root),
        root: root.clone(),
        cache: tools.clone(),
        base: step.changed.as_ref().map(|_| "HEAD".to_owned()),
        pr_body: None,
        host,
        job_start: None,
        manifest: "Cargo.toml".into(),
        after_reports: None,
        ts: config.ts.clone(),
        swift: None,
        tools: BTreeMap::new(),
        platform: run::platform(),
        env: Vec::new(),
        timings: None,
        results: results(config, step, &root, &store, i),
    };
    let report = run::run(&plan, &cx, &mut |_, _| {}).expect("run");
    let gate = &report.jobs[0].gates[0];
    assert_eq!(gate.outcome.word(), step.expect, "{at}: {gate:?}");
    let json = serde_json::to_string(gate).expect("json");
    if let Some(want) = &step.contains {
        assert!(json.contains(want.as_str()), "{at}: {json} lacks {want}");
    }
    for (tier, n) in &step.expect_store {
        assert_eq!(
            records(&store.join(tier)).len(),
            *n,
            "{at}: records in {tier}"
        );
    }
    for (tier, was) in step.same.iter().zip(before) {
        assert_eq!(records(&store.join(tier)), was, "{at}: {tier} changed");
    }
}
