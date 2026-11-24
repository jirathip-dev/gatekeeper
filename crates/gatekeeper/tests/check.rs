use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "../../gatekeeper-engine/tests/support/ci.rs"]
mod ci;

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
const TS: &str = "\n[ts]\nprojects = [\".\"]\ninstall = \"true\"\n";
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
    let ts_alone = "integration = \"staging\"\npin = \"checkout\"\npacks = [\"ts\"]\n";
    for (name, config) in [
        ("check-ts-no-table", ts_alone.to_owned()),
        ("check-ts-no-pack", format!("{CONFIG}{TS}")),
    ] {
        let (code, out) = gk(&repo(name, &config, &[]), &["list"], &[]);
        assert_eq!(code, 2, "{out}");
        assert!(out.contains("go together"), "{out}");
    }
    let swift = "integration = \"staging\"\npin = \"checkout\"\npacks = [\"swift\"]\n";
    for (name, config, says) in [
        ("check-swift-no-table", swift.to_owned(), "go together"),
        (
            "check-swift-no-package",
            format!("{swift}[swift]\nsources = [\"Sources\"]\n"),
            "at least one of packages or projects",
        ),
        (
            "check-tools-unpinned",
            format!("{CONFIG}[tools]\ndrover = \"bin/drover\"\n"),
            "[tools] drover is no pinned tool",
        ),
    ] {
        let (code, out) = gk(&repo(name, &config, &[]), &["list"], &[]);
        assert_eq!(code, 2, "{out}");
        assert!(out.contains(says), "{out}");
    }
}

#[test]
fn a_select_gate_is_refused_where_it_cannot_select() {
    let head = "about = \"unit\"\nfix = \"none\"\nbudget_secs = 5\n";
    let table = "\n[select]\nvitest = \"vitest.config.ts\"\nfull = []\n";
    let run = "run = \"vitest run {files}\"\n";
    for (name, gate, says) in [
        (
            "select-steps",
            format!("{head}[[step]]\n{run}{table}"),
            "[select] runs one `run` line",
        ),
        (
            "select-projects",
            format!("{head}{run}projects = [\"apps/*\"]\n{table}"),
            "give [select] or `projects`",
        ),
        (
            "select-no-files",
            format!("{head}run = \"vitest run\"\n{table}"),
            "[select] needs `{files}`",
        ),
        (
            "select-sharded",
            format!("{head}{run}shards = {{ hosted = 2, self-hosted = 1 }}\n{table}"),
            "needs `{report}`",
        ),
        (
            "select-unconfigured",
            format!("{head}{run}{table}"),
            "local/unit: [select] needs [ts] and [scope]",
        ),
    ] {
        let (code, out) = gk(&repo(name, CONFIG, &[("unit", &gate)]), &["list"], &[]);
        assert_eq!(code, 2, "{name}: {out}");
        assert!(out.contains(says), "{name}: {out}");
    }
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
        "matrix={\"include\":[{\"after\":\"\",\"artifact\":\"gates-local\",\"leg\":\"local\",\"node\":false,\"results-integration\":\"\",\"results-own\":\"\",\"results-save\":\"\",\"results-tier\":\"\",\"runs-on\":\"ubuntu-latest\",\"rust\":true,\"shard\":\"\"}]}\nafter={\"include\":[]}\ntimings-save="
    );
    let ts = repo(
        "ci-ts",
        &format!("{}{TS}", CONFIG.replace("[]", "[\"ts\"]")),
        &[],
    );
    let (code, plan) = gk(&ts, &["ci", "plan"], &[]);
    assert_eq!(code, 0, "{plan}");
    assert_eq!(
        plan.trim(),
        "matrix={\"include\":[{\"after\":\"\",\"artifact\":\"gates-ts\",\"leg\":\"ts\",\"node\":true,\"results-integration\":\"\",\"results-own\":\"\",\"results-save\":\"\",\"results-tier\":\"\",\"runs-on\":\"ubuntu-latest\",\"rust\":true,\"shard\":\"\"}]}\nafter={\"include\":[]}\ntimings-save="
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
    let secs = row
        .strip_prefix("| local | github-hosted (cold cache) | ")
        .and_then(|rest| rest.split_once("s |"))
        .and_then(|(secs, _)| secs.parse::<f64>().ok());
    assert!(secs.is_some(), "{row}");
    if ci::in_ci() {
        assert!(
            row.starts_with("| local | github-hosted (cold cache) | 3."),
            "{row}"
        );
    } else {
        eprintln!("ci done row: {row}; its 3 s start asserted in CI only");
    }
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

const DEPS: &str =
    "about = \"deps\"\nfix = \"list it\"\nbudget_secs = 5\n\n[[step]]\ncheck = \"deps\"\n";

fn cargo_lock(sourced: usize) -> String {
    (0..sourced).fold("version = 4\n".to_owned(), |lock, i| {
        lock + &format!("\n[[package]]\nname = \"p{i}\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n")
    })
}

fn grown_deps_repo() -> (PathBuf, String) {
    let root = repo("deps", CONFIG, &[("deps", DEPS)]);
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\np0 = \"1\"\n",
    )
    .expect("manifest");
    fs::write(root.join("Cargo.lock"), cargo_lock(1)).expect("lock");
    fs::write(
        root.join(".gatekeeper/deps.toml"),
        "growth = 10\n\n[cargo]\np0 = \"the one\"\n",
    )
    .expect("list");
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "base"]);
    let base = git(&root, &["rev-parse", "HEAD"]);
    git(&root, &["update-ref", "refs/remotes/origin/staging", &base]);
    fs::write(root.join("Cargo.lock"), cargo_lock(12)).expect("grown");
    git(&root, &["commit", "-q", "-am", "grow"]);
    (root, base)
}

#[test]
fn deps_fails_lockfile_growth_alike_on_a_lane_and_in_ci_checks_only_the_list_on_a_push_and_notes_the_count()
 {
    let (root, base) = grown_deps_repo();
    let ci = [("GITHUB_ACTIONS", "true")];
    let (lane_code, lane) = gk(&root, &["check"], &[]);
    let (ci_code, in_ci) = gk(&root, &["check", "--base", &base], &ci);
    for (code, out) in [(lane_code, &lane), (ci_code, &in_ci)] {
        assert_eq!(code, 1, "{out}");
        assert!(
            out.contains("      lockfiles: Cargo.lock 1 -> 12 (+11)"),
            "{out}"
        );
        assert!(
            out.contains("lockfiles grew by 11 packages, over the limit of 10"),
            "{out}"
        );
    }
    let reports = root.join("reports/gates-local");
    fs::create_dir_all(&reports).expect("reports");
    fs::copy(
        root.join("target/gatekeeper/report.json"),
        reports.join("report.json"),
    )
    .expect("copy");
    let (code, table) = gk(&root, &["ci", "done", "--reports", "reports"], &[]);
    assert_eq!(code, 1, "{table}");
    let row = table
        .lines()
        .find(|l| l.contains("local/deps"))
        .expect("row");
    assert!(
        row.ends_with("| FAIL | lockfiles: Cargo.lock 1 -> 12 (+11) |  |"),
        "{row}"
    );
    git(&root, &["update-ref", "-d", "refs/remotes/origin/staging"]);
    let (code, push) = gk(&root, &["check"], &ci);
    assert_eq!(code, 0, "{push}");
    assert!(push.contains("      lockfiles: Cargo.lock 12\n"), "{push}");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n",
    )
    .expect("dropped");
    let (code, push) = gk(&root, &["check"], &ci);
    assert_eq!(code, 1, "{push}");
    assert!(push.contains("[cargo] p0: stale"), "{push}");
}

fn pinned_sha256(tool: &str, target: &str) -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../packs/core/tools/{tool}.toml"));
    let text = fs::read_to_string(path).expect("tool file");
    let table = format!("[asset.{target}]");
    let after = text.split_once(&table).expect("an asset for this target").1;
    let sha = after.split_once("sha256 = \"").expect("sha256").1;
    sha[..64].to_owned()
}

#[test]
fn a_run_line_gets_a_pinned_tool_by_name_from_the_cache_in_under_2_s_and_an_unpinned_name_is_refused()
 {
    let Some(target) = gatekeeper_engine::fetch::host_target() else {
        return;
    };
    let gate = "about = \"uses a tool\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"{tool:actionlint} > used\"\n";
    let root = repo("check-tool", CONFIG, &[("tool", gate)]);
    let cached = root
        .join("cache/gatekeeper/tools")
        .join(pinned_sha256("actionlint", target));
    fs::create_dir_all(&cached).expect("cache dir");
    fs::write(cached.join("actionlint"), "#!/bin/sh\necho cached actionlint\n").expect("stub");
    let mode = std::os::unix::fs::PermissionsExt::from_mode(0o755);
    fs::set_permissions(cached.join("actionlint"), mode).expect("executable");
    let started = std::time::Instant::now();
    let (code, out) = gk(&root, &["check"], &[]);
    ci::assert_under(
        started.elapsed(),
        std::time::Duration::from_secs(2),
        "a pinned tool from the cache",
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        fs::read_to_string(root.join("used")).expect("ran"),
        "cached actionlint\n"
    );

    let gate =
        "about = \"uses a tool\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"{tool:drover} x\"\n";
    let (code, out) = gk(
        &repo("check-tool-drover", CONFIG, &[("tool", gate)]),
        &["check"],
        &[],
    );
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("{tool:drover} is no pinned tool"), "{out}");
}

#[test]
fn an_off_gate_shows_its_reason_and_the_rest_of_its_pack_runs_and_passes() {
    let off = format!("{CONFIG}\n[off]\n\"local/a-bad\" = \"pre-adoption findings; own issue\"\n");
    let root = repo("check-off", &off, &[("a-bad", BAD), ("c-ok", OK)]);
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 0, "{out}");
    let line = out.lines().find(|l| l.starts_with("off   local/a-bad"));
    let line = line.expect("off line");
    assert!(
        line.ends_with("off: pre-adoption findings; own issue"),
        "{line}"
    );
    assert!(out.contains("ok    local/c-ok"), "{out}");
    let (code, list) = gk(&root, &["list"], &[]);
    assert_eq!(code, 0, "{list}");
    assert!(
        list.contains("(off: pre-adoption findings; own issue)"),
        "{list}"
    );
    let reports = root.join("reports/gates-local");
    fs::create_dir_all(&reports).expect("reports");
    let report = root.join("target/gatekeeper/report.json");
    fs::copy(report, reports.join("report.json")).expect("copy");
    let (code, table) = gk(&root, &["ci", "done", "--reports", "reports"], &[]);
    assert_eq!(code, 0, "{table}");
    let row = table
        .lines()
        .find(|l| l.contains("local/a-bad"))
        .expect("row");
    assert!(
        row.contains("| off | off: pre-adoption findings; own issue |"),
        "{row}"
    );
}

#[test]
fn an_off_entry_without_a_reason_for_an_unknown_gate_or_for_the_pin_is_a_config_error() {
    for (name, entry, says) in [
        (
            "off-no-reason",
            "\"local/c-ok\" = \" \"",
            "[off] `local/c-ok` needs a reason",
        ),
        (
            "off-unknown-gate",
            "\"local/nope\" = \"why\"",
            "[off] names `local/nope`, which no configured pack has",
        ),
        (
            "off-unknown-pack",
            "\"nopack/x\" = \"why\"",
            "`nopack/x` is not a gate id",
        ),
        (
            "off-unconfigured-pack",
            "\"rust/clippy\" = \"why\"",
            "[off] names `rust/clippy`, which no configured pack has",
        ),
        (
            "off-pin",
            "\"core/pin\" = \"why\"",
            "[off] `core/pin` guards the pin and stays on",
        ),
    ] {
        let config = format!("{CONFIG}\n[off]\n{entry}\n");
        let root = repo(name, &config, &[("c-ok", OK)]);
        for verb in ["check", "list"] {
            let (code, out) = gk(&root, &[verb], &[]);
            assert_eq!(code, 2, "{name} {verb}: {out}");
            assert!(out.contains(says), "{name} {verb}: {out}");
        }
    }
}
