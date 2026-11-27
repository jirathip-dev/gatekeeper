mod common;

use common::git;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "packs = []\nintegration = \"staging\"\npin = \"checkout\"\n\n[cache]\n";
const KEYED: &str = "about = \"passes\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"test ! -e target/flag\"\nversions = [\"echo 1\"]\n";
const SURE: &str =
    "packs = []\nintegration = \"staging\"\npin = \"checkout\"\n\n[cache]\nrecheck = 1000000\n";
const BARE: &str = "about = \"bare\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"true\"\n";

fn repo(name: &str, config: &str, gates: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(root.join(".gatekeeper/gates")).expect("gates dir");
    fs::write(root.join(".gatekeeper/config.toml"), config).expect("config");
    fs::write(
        root.join(".gitignore"),
        "target/\ncache/\nreports/\nevent.json\n",
    )
    .expect("ignore");
    for (gate, text) in gates {
        fs::write(root.join(format!(".gatekeeper/gates/{gate}.toml")), text).expect("gate");
    }
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "base"]);
    root
}

const CI_VARS: [&str; 13] = [
    "GITHUB_ACTIONS",
    "RUNNER_ENVIRONMENT",
    "GATEKEEPER_CACHE_HIT",
    "GATEKEEPER_JOB_START",
    "GITHUB_EVENT_NAME",
    "GITHUB_EVENT_PATH",
    "GITHUB_REF",
    "GITHUB_REF_NAME",
    "GITHUB_REPOSITORY",
    "GITHUB_RUN_ID",
    "GITHUB_RUN_ATTEMPT",
    "ImageOS",
    "ImageVersion",
];

fn gk(root: &Path, args: &[&str], env: &[(&str, &str)]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gatekeeper"));
    cmd.args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"));
    for var in CI_VARS {
        cmd.env_remove(var);
    }
    let out = cmd
        .envs(env.iter().copied())
        .output()
        .expect("gatekeeper runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().expect("exit code"), text)
}

fn pull_request(root: &Path, head: &str) -> Vec<(&'static str, String)> {
    let payload =
        format!("{{\"pull_request\": {{\"head\": {{\"repo\": {{\"full_name\": \"{head}\"}}}}}}}}");
    fs::write(root.join("event.json"), payload).expect("event");
    vec![
        ("GITHUB_ACTIONS", "true".into()),
        ("GITHUB_EVENT_NAME", "pull_request".into()),
        ("GITHUB_REF", "refs/pull/41/merge".into()),
        ("GITHUB_REPOSITORY", "o/r".into()),
        (
            "GITHUB_EVENT_PATH",
            root.join("event.json").to_string_lossy().into_owned(),
        ),
        ("GITHUB_RUN_ID", "7".into()),
    ]
}

fn borrowed<'a>(env: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    env.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

#[test]
fn a_second_check_prints_cached_with_its_proof() {
    let root = repo("cache-second", SURE, &[("keyed", KEYED), ("bare", BARE)]);
    let head = git(&root, &["rev-parse", "HEAD"]);
    let (code, first) = gk(&root, &["check"], &[]);
    assert_eq!(code, 0, "{first}");
    assert!(
        first.contains("· cache reads [\"host\"] writes [\"host\"]"),
        "{first}"
    );
    assert!(first.contains("ok    local/keyed"), "{first}");
    let (code, second) = gk(&root, &["check"], &[]);
    assert_eq!(code, 0, "{second}");
    let row = second
        .lines()
        .find(|l| l.starts_with("cached local/keyed"))
        .unwrap_or_else(|| panic!("no cached row:\n{second}"));
    assert!(
        row.ends_with(&format!("   lane run at {}", &head[..7])),
        "{row}"
    );
    assert!(second.contains("ok    local/bare"), "{second}");
    assert!(
        second.contains("2 gates passed · 1 of 2 gates cached · ran "),
        "{second}"
    );
    let (_, off) = gk(&root, &["check", "--no-cache"], &[]);
    assert!(off.contains("· cache off: --no-cache"), "{off}");
    assert!(off.contains("ok    local/keyed"), "{off}");
}

#[test]
fn a_cached_pass_that_fails_now_is_loud() {
    let config = format!("{CONFIG}recheck = 1\n");
    let root = repo("cache-loud", &config, &[("keyed", KEYED)]);
    assert_eq!(gk(&root, &["check"], &[]).0, 0);
    fs::write(root.join("target/flag"), "an undeclared input").expect("flag");
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("FAIL  local/keyed"), "{out}");
    assert!(
        out.contains("      cached pass does not hold: it passed in lane run at "),
        "{out}"
    );
    assert!(
        out.contains("Add it to `paths`, or set `cache = false`."),
        "{out}"
    );
    assert!(out.contains("1 rechecked, 1 differed"), "{out}");
    fs::remove_file(root.join("target/flag")).expect("unflag");
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("ok    local/keyed"),
        "the record was deleted: {out}"
    );
}

#[test]
fn list_shows_a_key_or_why_not() {
    let root = repo("cache-list", CONFIG, &[("keyed", KEYED), ("bare", BARE)]);
    let (code, out) = gk(&root, &["list"], &[]);
    assert_eq!(code, 0, "{out}");
    let line = |id: &str| {
        out.lines()
            .find(|l| l.starts_with(id))
            .unwrap_or_default()
            .to_owned()
    };
    assert!(line("local/keyed").contains("  key "), "{out}");
    assert!(line("local/keyed").ends_with("… 4 files"), "{out}");
    assert!(
        line("local/bare").ends_with("  not cached: a `run` step with no setup or versions"),
        "{out}"
    );
    let plain = repo(
        "cache-list-off",
        CONFIG.replace("[cache]\n", "").as_str(),
        &[("bare", BARE)],
    );
    let (_, out) = gk(&plain, &["list"], &[]);
    assert!(!out.contains("cached"), "{out}");
}

fn entry(plan: &str) -> serde_json::Value {
    let line = plan
        .lines()
        .find_map(|l| l.strip_prefix("matrix="))
        .expect("matrix");
    let matrix: serde_json::Value = serde_json::from_str(line).expect("json");
    matrix["include"][0].clone()
}

#[test]
fn ci_plan_names_the_tiers_of_its_trigger() {
    let root = repo("cache-plan", SURE, &[("keyed", KEYED)]);
    let pr = pull_request(&root, "o/r");
    let (code, plan) = gk(&root, &["ci", "plan"], &borrowed(&pr));
    assert_eq!(code, 0, "{plan}");
    let e = entry(&plan);
    assert_eq!(
        e["results-integration"],
        "gatekeeper-results-1-integration-gates-local-"
    );
    assert_eq!(e["results-own"], "gatekeeper-results-1-pr-41-gates-local-");
    assert_eq!(e["results-save"], "gatekeeper-results-1-pr-41-gates-local-");
    assert_eq!(e["results-tier"], "pr-41");
    let (_, full) = gk(&root, &["ci", "plan", "--all"], &borrowed(&pr));
    let e = entry(&full);
    assert_eq!(
        (&e["results-integration"], &e["results-own"]),
        (&"".into(), &"".into())
    );
    assert_eq!(e["results-save"], "gatekeeper-results-1-pr-41-gates-local-");
    let fork = pull_request(&root, "someone/r");
    let e = entry(&gk(&root, &["ci", "plan"], &borrowed(&fork)).1);
    for field in [
        "results-integration",
        "results-own",
        "results-save",
        "results-tier",
    ] {
        assert_eq!(e[field], "", "a fork: {field}");
    }
    let push = [
        ("GITHUB_ACTIONS", "true"),
        ("GITHUB_EVENT_NAME", "push"),
        ("GITHUB_REF_NAME", "staging"),
    ];
    let e = entry(&gk(&root, &["ci", "plan"], &push).1);
    assert_eq!(
        (&e["results-integration"], &e["results-own"]),
        (&"".into(), &"".into())
    );
    assert_eq!(
        e["results-save"],
        "gatekeeper-results-1-integration-gates-local-"
    );
    assert_eq!(e["results-tier"], "integration");
}

#[test]
fn ci_done_counts_the_cache() {
    let root = repo("cache-done", SURE, &[("keyed", KEYED)]);
    let pr = pull_request(&root, "o/r");
    let check = ["check", "--results", "cache/results"];
    let (code, first) = gk(&root, &check, &borrowed(&pr));
    assert_eq!(code, 0, "{first}");
    assert!(
        first.contains("cache reads [\"integration\", \"pr-41\"] writes [\"pr-41\"]"),
        "{first}"
    );
    assert!(root.join("cache/results/pr-41").is_dir(), "{first}");
    let (code, second) = gk(&root, &check, &borrowed(&pr));
    assert_eq!(code, 0, "{second}");
    assert!(second.contains("cached local/keyed"), "{second}");
    assert!(second.contains("run 7 · pr-41 · "), "{second}");
    fs::create_dir_all(root.join("reports/gates-local")).expect("reports");
    fs::copy(
        root.join("target/gatekeeper/report.json"),
        root.join("reports/gates-local/report.json"),
    )
    .expect("copy");
    let (code, table) = gk(
        &root,
        &["ci", "done", "--reports", "reports"],
        &borrowed(&pr),
    );
    assert_eq!(code, 0, "{table}");
    let row = table
        .lines()
        .find(|l| l.contains("local/keyed"))
        .expect("row");
    assert!(row.contains("| cached |"), "{row}");
    assert!(row.contains("run 7 · pr-41 · "), "{row}");
    assert!(
        table.contains("cache: 1 of 1 gates cached · ran 0.0s · saved "),
        "{table}"
    );
    assert!(
        table.contains("0 rechecked, 0 differed · 0 refused"),
        "{table}"
    );
    let fork = pull_request(&root, "someone/r");
    let (_, out) = gk(&root, &check, &borrowed(&fork));
    assert!(out.contains("cache off: a fork"), "{out}");
    assert!(out.contains("ok    local/keyed"), "{out}");
}

#[test]
fn a_rebuilt_repo_tool_misses() {
    let config = "packs = []\nintegration = \"staging\"\npin = \"checkout\"\n\n[tools]\nactionlint = \"target/al\"\n\n[cache]\nrecheck = 1000000\n";
    let gate =
        "about = \"t\"\nfix = \"none\"\nbudget_secs = 5\n\n[[step]]\ntool = \"actionlint\"\n";
    let root = repo("cache-own-tool", config, &[("tooled", gate)]);
    fs::create_dir_all(root.join("target")).expect("target");
    let tool = root.join("target/al");
    fs::write(&tool, "#!/bin/sh\nexit 0\n").expect("tool");
    Command::new("chmod")
        .arg("+x")
        .arg(&tool)
        .status()
        .expect("chmod");
    let (code, first) = gk(&root, &["check"], &[]);
    assert_eq!(code, 0, "{first}");
    let (_, second) = gk(&root, &["check"], &[]);
    assert!(second.contains("cached local/tooled"), "{second}");
    fs::write(&tool, "#!/bin/sh\n# rebuilt\nexit 0\n").expect("rebuilt");
    let (code, third) = gk(&root, &["check"], &[]);
    assert_eq!(code, 0, "{third}");
    assert!(third.contains("ok    local/tooled"), "{third}");
}

const PROJECTS: &str = "about = \"per project\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"test ! -e target/flag\"\nversions = [\"echo 1\"]\nprojects = [\"pkgs/*\"]\n";

fn three_projects(name: &str, cache: &str) -> PathBuf {
    let config = format!("{CONFIG}{cache}\n[scope]\ngraphs = [\"pnpm\"]\n");
    let root = repo(name, &config, &[("each", PROJECTS)]);
    let files = [
        ("pnpm-workspace.yaml", "packages:\n  - 'pkgs/*'\n"),
        ("pkgs/a/package.json", "{\"name\": \"a\"}"),
        (
            "pkgs/b/package.json",
            "{\"name\": \"b\", \"dependencies\": {\"a\": \"*\"}}",
        ),
        ("pkgs/c/package.json", "{\"name\": \"c\"}"),
    ];
    for (path, text) in files {
        fs::create_dir_all(root.join(path).parent().expect("parent")).expect("dir");
        fs::write(root.join(path), text).expect("file");
    }
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "projects"]);
    root
}

#[test]
fn a_change_in_one_project_reruns_it_and_its_dependents_only() {
    let root = three_projects("cache-projects", "recheck = 1000000\n");
    let (code, list) = gk(&root, &["list"], &[]);
    assert_eq!(code, 0, "{list}");
    for project in ["pkgs/a", "pkgs/b", "pkgs/c"] {
        let at = format!("\n    {project}  key ");
        assert!(list.contains(&at), "{list}");
    }
    assert_eq!(gk(&root, &["check"], &[]).0, 0);
    fs::write(root.join("pkgs/a/index.js"), "changed").expect("edit");
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("ok    local/each"), "{out}");
    assert!(out.contains("      selected: pkgs/a pkgs/b\n"), "{out}");
    assert!(out.contains("      projects: 2 run, 1 cached\n"), "{out}");
    assert!(out.contains("1 of 3 gates cached"), "{out}");
    let json = fs::read_to_string(root.join("target/gatekeeper/report.json")).expect("report");
    let report: serde_json::Value = serde_json::from_str(&json).expect("json");
    let cache = &report["jobs"][0]["gates"][0]["cache"]["projects"];
    assert_eq!(cache["pkgs/a"], "ran", "{json}");
    assert_eq!(cache["pkgs/b"], "ran", "{json}");
    assert!(cache["pkgs/c"]["cached"]["proof"].is_object(), "{json}");
    let (_, again) = gk(&root, &["check"], &[]);
    assert!(again.contains("cached local/each"), "{again}");
    assert!(again.contains("3 of 3 gates cached"), "{again}");
}

#[test]
fn ci_done_fails_on_a_project_recheck_that_differs() {
    let root = three_projects("cache-projects-done", "recheck = 1\n");
    let pr = pull_request(&root, "o/r");
    let check = ["check", "--results", "cache/results"];
    assert_eq!(gk(&root, &check, &borrowed(&pr)).0, 0);
    fs::create_dir_all(root.join("target")).expect("target");
    fs::write(root.join("target/flag"), "an undeclared input").expect("flag");
    let (code, out) = gk(&root, &check, &borrowed(&pr));
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("cached pass does not hold"), "{out}");
    fs::create_dir_all(root.join("reports/gates-local")).expect("reports");
    fs::copy(
        root.join("target/gatekeeper/report.json"),
        root.join("reports/gates-local/report.json"),
    )
    .expect("copy");
    let (code, table) = gk(
        &root,
        &["ci", "done", "--reports", "reports"],
        &borrowed(&pr),
    );
    assert_eq!(code, 1, "{table}");
    assert!(table.contains("3 rechecked, 3 differed"), "{table}");
}
