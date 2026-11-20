use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIG: &str = "packs = []\nintegration = \"staging\"\npin = \"checkout\"\n";

fn repo(name: &str, config: &str, gates: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(root.join(".gatekeeper/gates")).expect("gates dir");
    fs::write(root.join(".gatekeeper/config.toml"), config).expect("config");
    for (gate, text) in gates {
        fs::write(root.join(format!(".gatekeeper/gates/{gate}.toml")), text).expect("gate");
    }
    root
}

fn gk(root: &Path, args: &[&str], env: &[(&str, &str)]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gatekeeper"));
    cmd.args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"));
    for var in [
        "GITHUB_ACTIONS",
        "RUNNER_ENVIRONMENT",
        "GATEKEEPER_CACHE_HIT",
        "GATEKEEPER_JOB_START",
    ] {
        cmd.env_remove(var);
    }
    cmd.envs(env.iter().copied());
    let Output {
        status,
        stdout,
        stderr,
        ..
    } = cmd.output().expect("gatekeeper runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    (status.code().expect("exit code"), text)
}

const BAD: &str =
    "about = \"fails\"\nfix = \"make it pass\"\nbudget_secs = 5\nrun = \"echo boom; exit 3\"\n";
const DB: &str = "about = \"needs a database\"\nfix = \"none\"\nbudget_secs = 5\nneeds = [\"containers\"]\nrun = \"true\"\n";
const OK: &str = "about = \"passes\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"true\"\n";

fn three(name: &str) -> PathBuf {
    repo(name, CONFIG, &[("a-bad", BAD), ("b-db", DB), ("c-ok", OK)])
}

#[test]
fn every_gate_runs_after_one_fails_and_the_failure_prints_its_fix_and_log() {
    let root = three("check-three");
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 1, "{out}");
    for line in [
        "FAIL  local/a-bad",
        "      boom",
        "      fix: make it pass",
        "log: target/gatekeeper/local-a-bad.log   rerun: ./.gatekeeper/run check --gate local/a-bad",
        "ok    local/c-ok",
        "1 failed: local/a-bad",
    ] {
        assert!(out.contains(line), "missing `{line}` in\n{out}");
    }
    let log = fs::read_to_string(root.join("target/gatekeeper/local-a-bad.log")).expect("log");
    assert!(log.contains("boom"));
}

#[test]
fn a_skip_prints_its_reason_and_ci_runs_what_a_lane_skips() {
    let root = three("check-skip");
    let (_, lane) = gk(&root, &["check"], &[]);
    let skip = lane
        .lines()
        .find(|l| l.starts_with("skip  local/b-db"))
        .expect("skip line");
    assert!(
        skip.ends_with("CI only on a lane's host: needs containers"),
        "{skip}"
    );
    let ci = [
        ("GITHUB_ACTIONS", "true"),
        ("RUNNER_ENVIRONMENT", "github-hosted"),
        ("GATEKEEPER_CACHE_HIT", "true"),
    ];
    let (_, hosted) = gk(&root, &["check"], &ci);
    assert!(hosted.contains("ok    local/b-db"), "{hosted}");
}

#[test]
fn a_passing_run_exits_0_and_reports_the_setup_time() {
    let root = three("check-setup");
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs()
        - 7;
    let (code, out) = gk(
        &root,
        &["check", "--gate", "local/c-ok"],
        &[("GATEKEEPER_JOB_START", &start.to_string())],
    );
    assert_eq!(code, 0, "{out}");
    let report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("target/gatekeeper/report.json")).expect("report"),
    )
    .expect("json");
    let setup = report["jobs"][0]["setup"].as_u64().expect("setup");
    assert!(setup >= 7000, "{report}");
    assert_eq!(report["jobs"][0]["gates"][0]["id"], "local/c-ok");
}

#[test]
fn usage_and_config_errors_exit_2() {
    let root = three("check-usage");
    assert_eq!(gk(&root, &["check", "--gate", "local/nope"], &[]).0, 2);
    assert_eq!(gk(&root, &["check", "--frobnicate"], &[]).0, 2);
    assert_eq!(gk(&root, &["nope"], &[]).0, 2);
    let typo = repo(
        "check-typo",
        &format!("{CONFIG}pakcs = []\n"),
        &[("c-ok", OK)],
    );
    let (code, out) = gk(&typo, &["check"], &[]);
    assert_eq!(code, 2);
    assert!(out.contains("pakcs"), "{out}");
}

#[test]
fn list_prints_the_planned_gates_with_budgets() {
    let root = three("list");
    let (code, out) = gk(&root, &["list"], &[]);
    assert_eq!(code, 0);
    assert!(
        out.lines()
            .any(|l| l.starts_with("local/a-bad") && l.contains("5s  fails")),
        "{out}"
    );
    let (code, json) = gk(&root, &["list", "--json"], &[]);
    assert_eq!(code, 0);
    let plan: serde_json::Value = serde_json::from_str(&json).expect("json");
    assert_eq!(plan["jobs"][0]["gates"].as_array().map(Vec::len), Some(3));
}

#[test]
fn ci_plan_emits_one_leg_per_pack_and_done_fails_a_missing_or_failed_report() {
    let root = repo("ci", CONFIG, &[("c-ok", OK)]);
    let (code, plan) = gk(&root, &["ci", "plan"], &[]);
    assert_eq!(code, 0);
    assert_eq!(
        plan.trim(),
        "matrix={\"include\":[{\"after\":\"\",\"artifact\":\"gates-local\",\"leg\":\"local\",\"runs-on\":\"ubuntu-latest\",\"rust\":true,\"shard\":\"\"}]}\nafter={\"include\":[]}"
    );
    let reports = root.join("reports");
    let (code, out) = gk(&root, &["ci", "done", "--reports", "reports"], &[]);
    assert_eq!(code, 1);
    assert!(out.contains("job local: no report"), "{out}");
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs()
        - 3;
    let ci = [
        ("GITHUB_ACTIONS", "true"),
        ("GATEKEEPER_JOB_START", &start.to_string()),
    ];
    assert_eq!(gk(&root, &["check", "--pack", "local"], &ci).0, 0);
    fs::create_dir_all(reports.join("gates-local")).expect("reports");
    let report = fs::read_to_string(root.join("target/gatekeeper/report.json")).expect("report");
    fs::write(reports.join("gates-local/report.json"), &report).expect("copy");
    let (code, table) = gk(&root, &["ci", "done", "--reports", "reports"], &[]);
    assert_eq!(code, 0, "{table}");
    let row = table
        .lines()
        .find(|l| l.contains("local/c-ok"))
        .expect("row");
    assert!(
        row.starts_with("| local | github-hosted (cold cache) | 3."),
        "{row}"
    );
    let mut over: serde_json::Value = serde_json::from_str(&report).expect("json");
    over["jobs"][0]["gates"][0]["outcome"] = "over-budget".into();
    fs::write(reports.join("gates-local/report.json"), over.to_string()).expect("over");
    let (code, out) = gk(&root, &["ci", "done", "--reports", "reports"], &[]);
    assert_eq!(code, 1);
    assert!(out.contains("FAIL local/c-ok: OVER"), "{out}");
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@localhost",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

const WEB: &str =
    "about = \"web\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"true\"\npaths = [\"web/**\"]\n";
const SRC: &str =
    "about = \"src\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"true\"\npaths = [\"src/*.rs\"]\n";

#[test]
fn a_gate_with_no_changed_file_in_its_paths_is_skipped_unless_all_or_no_base() {
    let root = repo("scope", CONFIG, &[("web", WEB), ("src", SRC), ("c-ok", OK)]);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "base"]);
    let base = git(&root, &["rev-parse", "HEAD"]);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("src/a.rs"), "").expect("a.rs");
    let (code, out) = gk(&root, &["check", "--base", &base], &[]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("· 1 files changed ·"), "{out}");
    let skip = out
        .lines()
        .find(|l| l.starts_with("skip  local/web"))
        .expect("web skipped");
    assert!(
        skip.ends_with("no changed file matches its paths"),
        "{skip}"
    );
    assert!(out.contains("ok    local/src"), "{out}");
    assert!(out.contains("ok    local/c-ok"), "{out}");
    git(&root, &["update-ref", "refs/remotes/origin/staging", &base]);
    let (_, lane) = gk(&root, &["check"], &[]);
    assert!(lane.contains("skip  local/web"), "{lane}");
    let ci = [("GITHUB_ACTIONS", "true")];
    let all = ["check", "--all"];
    for (args, env) in [(&all[..], &[][..]), (&["check"][..], &ci[..])] {
        let (code, out) = gk(&root, args, env);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("ok    local/web"), "{args:?} {env:?}\n{out}");
        assert!(out.contains("· every gate ·"), "{out}");
    }
    git(&root, &["update-ref", "-d", "refs/remotes/origin/staging"]);
    let (_, no_base) = gk(&root, &["check"], &[]);
    assert!(no_base.contains("base none · every gate"), "{no_base}");
    assert!(no_base.contains("ok    local/web"), "{no_base}");
}

const FLOOR: &str = "about = \"the deny floor\"\nfix = \"deny it\"\nbudget_secs = 5\n\n[[step]]\ncheck = \"deny-floor\"\n";
const DENY: &str = "[advisories]\nyanked = \"deny\"\n[bans]\nmultiple-versions = \"deny\"\nwildcards = \"allow\"\n[sources]\nunknown-registry = \"deny\"\nunknown-git = \"warn\"\n";

#[test]
fn a_baseline_passes_listed_findings_fails_new_and_stale_ones_and_only_shrinks() {
    let root = repo("baseline", CONFIG, &[("floor", FLOOR)]);
    let file = root.join(".gatekeeper/baseline/local-floor.txt");
    let deny = root.join("deny.toml");
    fs::write(&deny, DENY).expect("deny");
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("bans.wildcards is \"allow\""), "{out}");

    let (code, out) = gk(&root, &["baseline", "local/floor"], &[]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(".gatekeeper/baseline/local-floor.txt: 2 findings"),
        "{out}"
    );
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 0, "{out}");

    let fixed = DENY.replace("wildcards = \"allow\"", "wildcards = \"deny\"");
    fs::write(&deny, format!("{fixed}[licenses]\nallow = [\"WTFPL\"]\n")).expect("deny");
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("licence WTFPL is outside"), "{out}");
    fs::write(&deny, &fixed).expect("deny");
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("stale: shrink the baseline (./.gatekeeper/run baseline local/floor): deny.toml: bans.wildcards"),
        "{out}"
    );

    fs::write(&deny, format!("{fixed}[licenses]\nallow = [\"WTFPL\"]\n")).expect("deny");
    let (code, out) = gk(&root, &["baseline", "local/floor"], &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("new   deny.toml: licence WTFPL"), "{out}");
    let listed = fs::read_to_string(&file).expect("baseline");
    assert_eq!(listed.lines().count(), 1, "{listed}");
    assert!(listed.contains("sources.unknown-git"), "{listed}");

    fs::write(
        &deny,
        fixed.replace("unknown-git = \"warn\"", "unknown-git = \"deny\""),
    )
    .expect("deny");
    let (code, out) = gk(&root, &["baseline", "local/floor"], &[]);
    assert_eq!(code, 0, "{out}");
    assert!(!file.exists());
    assert_eq!(gk(&root, &["check"], &[]).0, 0);
    assert_eq!(gk(&root, &["baseline", "local/nope"], &[]).0, 2);
}

const SHOTS: &str = "about = \"ui\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"mkdir -p shots && echo a > shots/a.png; exit 1\"\n\n[[upload]]\nname = \"ui-390\"\npaths = [\"shots/*.png\"]\n\n[[upload]]\nname = \"db\"\npaths = [\"db/*.dump\"]\n";

#[test]
fn a_lane_sees_where_a_failed_gates_kept_files_are_and_what_matched_nothing() {
    let root = repo("kept-lane", CONFIG, &[("ui", SHOTS)]);
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("      files: ui-390 (1) target/gatekeeper/files/ui-390/\n"),
        "{out}"
    );
    assert!(
        out.contains("      files: db (0) nothing matched db/*.dump\n"),
        "{out}"
    );
    assert!(
        root.join("target/gatekeeper/files/ui-390/shots/a.png")
            .is_file()
    );
}
