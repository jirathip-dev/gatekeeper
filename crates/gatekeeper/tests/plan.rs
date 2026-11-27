mod common;

use common::git;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

const UNIT: &str = "about = \"unit tests per project\"\nfix = \"none\"\nbudget_secs = 5\nprojects = [\"crates/*\"]\nrun = \"echo ran {projects}\"\n";

fn workspace(root: &Path) {
    let crates = [
        ("a", ""),
        ("b", "\n[dependencies]\na = { path = \"../a\" }\n"),
        ("c", ""),
    ];
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("ws");
    fs::write(root.join(".gitignore"), "target/\n").expect("ignore");
    for (name, deps) in crates {
        fs::create_dir_all(root.join("crates").join(name)).expect("crate dir");
        let manifest = format!("[package]\nname = \"{name}\"\n{deps}");
        fs::write(root.join(format!("crates/{name}/Cargo.toml")), manifest).expect("crate");
    }
}

#[test]
fn a_projects_gate_runs_over_the_affected_projects_and_a_file_outside_them_widens_the_run() {
    let config = format!("{CONFIG}\n[scope]\ngraphs = [\"cargo\"]\nignore = [\"docs/**\"]\n");
    let root = repo("affected", &config, &[("unit", UNIT)]);
    workspace(&root);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "base"]);
    let base = git(&root, &["rev-parse", "HEAD"]);
    fs::write(root.join("crates/a/lib.rs"), "").expect("change");
    let (code, out) = gk(&root, &["check", "--base", &base], &[]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("scope: 2 affected projects: crates/a crates/b"),
        "{out}"
    );
    assert!(out.contains("      selected: crates/a crates/b"), "{out}");
    let log = fs::read_to_string(root.join("target/gatekeeper/local-unit.log")).expect("log");
    assert!(log.contains("ran crates/a crates/b\n"), "{log}");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "a"]);
    let base = git(&root, &["rev-parse", "HEAD"]);
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::write(root.join("docs/guide.md"), "").expect("doc");
    let (code, out) = gk(&root, &["check", "--base", &base], &[]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("scope: 0 affected projects"), "{out}");
    assert!(
        out.contains("no affected project matches its projects"),
        "{out}"
    );
    fs::write(root.join("README.md"), "").expect("readme");
    let (code, out) = gk(&root, &["check", "--base", &base], &[]);
    assert_eq!(code, 0, "{out}");
    let widened = "scope: every project: `README.md` is outside every project";
    assert!(out.contains(widened), "{out}");
    assert!(
        out.contains("      selected: crates/a crates/b crates/c"),
        "{out}"
    );
    let reports = root.join("reports/gates-local");
    fs::create_dir_all(&reports).expect("reports");
    fs::copy(
        root.join("target/gatekeeper/report.json"),
        reports.join("report.json"),
    )
    .expect("copy");
    let (code, done) = gk(&root, &["ci", "done", "--reports", "reports"], &[]);
    assert_eq!(code, 0, "{done}");
    assert!(done.contains(widened), "{done}");
    assert!(
        done.contains("- selected by local/unit in local: crates/a crates/b crates/c"),
        "{done}"
    );
}

const SHARDED: &str = "about = \"sharded\"\nfix = \"none\"\nbudget_secs = 5\nshards = { hosted = 3, self-hosted = 2 }\nrun = \"echo {shard}\"\n";
const DB_LEG: &str =
    "about = \"db\"\nfix = \"none\"\nbudget_secs = 5\nleg = \"db\"\nrun = \"true\"\n";
const SEED_LEG: &str =
    "about = \"seed\"\nfix = \"none\"\nbudget_secs = 5\nleg = \"seed\"\nrun = \"true\"\n";
const LEGS: &str =
    "\n[legs.db]\nplatform = \"linux\"\n\n[legs.seed]\nplatform = \"linux\"\nafter = \"db\"\n";

#[test]
fn ci_plan_splits_a_sharded_gate_per_runner_kind_and_puts_after_legs_in_their_own_matrix() {
    let root = repo(
        "ci-shards",
        &format!("{CONFIG}{LEGS}"),
        &[("unit", SHARDED), ("db", DB_LEG), ("seed", SEED_LEG)],
    );
    let (code, out) = gk(&root, &["ci", "plan"], &[]);
    assert_eq!(code, 0, "{out}");
    let (matrix, after) = out.trim().split_once('\n').expect("two lines");
    let matrix: serde_json::Value =
        serde_json::from_str(matrix.strip_prefix("matrix=").expect("matrix")).expect("json");
    let jobs: Vec<String> = matrix["include"]
        .as_array()
        .expect("include")
        .iter()
        .map(|j| format!("{} {} {}", j["leg"], j["shard"], j["artifact"]))
        .collect();
    assert_eq!(
        jobs,
        [
            r#""db" "" "gates-db""#,
            r#""local" "1/3" "gates-local-1""#,
            r#""local" "2/3" "gates-local-2""#,
            r#""local" "3/3" "gates-local-3""#,
        ]
    );
    assert!(
        after.contains(r#""after":"db""#) && after.contains(r#""leg":"seed""#),
        "{after}"
    );
    let self_hosted = r#"{"linux": ["self-hosted", "linux"]}"#;
    let (_, out) = gk(&root, &["ci", "plan", "--runners", self_hosted], &[]);
    assert!(
        out.contains(r#""shard":"2/2""#) && !out.contains("3/3"),
        "{out}"
    );
    let (code, out) = gk(&root, &["ci", "done", "--reports", "reports"], &[]);
    assert_eq!(code, 1);
    assert!(out.contains("job local 3/3: no report"), "{out}");
    let ci = [("GITHUB_ACTIONS", "true")];
    let (code, out) = gk(&root, &["check", "--leg", "local", "--shard", "2/3"], &ci);
    assert_eq!(code, 0, "{out}");
    let log = fs::read_to_string(root.join("target/gatekeeper/local-unit.log")).expect("log");
    assert!(log.contains("2/3\n"), "{log}");
    let (code, out) = gk(&root, &["check", "--leg", "local", "--shard", "2/4"], &ci);
    assert_eq!(code, 2);
    assert!(
        out.contains("--shard 2/4: leg local splits into 3 here"),
        "{out}"
    );
}

#[test]
fn a_leg_or_projects_the_config_does_not_back_is_a_config_error() {
    let nowhere = repo("leg-missing", CONFIG, &[("db", DB_LEG)]);
    let (code, out) = gk(&nowhere, &["check"], &[]);
    assert_eq!(code, 2);
    assert!(
        out.contains("local/db: leg `db` has no [legs.db] in the config"),
        "{out}"
    );
    let unscoped = repo("projects-unscoped", CONFIG, &[("unit", UNIT)]);
    let (code, out) = gk(&unscoped, &["check"], &[]);
    assert_eq!(code, 2);
    assert!(
        out.contains("local/unit: `projects` needs [scope] graphs"),
        "{out}"
    );
    let chained = format!("{CONFIG}{LEGS}\n[legs.late]\nplatform = \"linux\"\nafter = \"seed\"\n");
    let late =
        "about = \"late\"\nfix = \"none\"\nbudget_secs = 5\nleg = \"late\"\nrun = \"true\"\n";
    let root = repo(
        "legs-chained",
        &chained,
        &[("db", DB_LEG), ("seed", SEED_LEG), ("late", late)],
    );
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 2);
    assert!(out.contains("[legs.late] after = `seed`"), "{out}");
}

#[test]
fn ci_done_prints_each_legs_time_against_the_entry_and_writes_this_runs() {
    let one = "about = \"one\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"true\"\n";
    let root = repo("legs", CONFIG, &[("one", one)]);
    let (code, out) = gk(&root, &["check"], &[]);
    assert_eq!(code, 0, "{out}");
    let text = fs::read_to_string(root.join("target/gatekeeper/report.json")).expect("report");
    let mut report: serde_json::Value = serde_json::from_str(&text).expect("json");
    report["jobs"][0]["setup"] = 10_000.into();
    report["jobs"][0]["gates"][0]["took"] = 90_000.into();
    let reports = root.join("reports/gates-local");
    fs::create_dir_all(&reports).expect("reports");
    fs::write(reports.join("report.json"), report.to_string()).expect("write");
    let args = ["ci", "done", "--reports", "reports", "--timings-out", "out"];
    let (code, done) = gk(&root, &args, &[]);
    assert_eq!(code, 0, "{done}");
    assert!(done.contains("\nlegs: local 1m40s\n"), "{done}");
    let entry = root.join("entry");
    fs::create_dir_all(&entry).expect("entry");
    fs::write(
        entry.join("health.json"),
        r#"{"schema": 1, "legs": {"local": 60000}}"#,
    )
    .expect("health");
    let mut args = args.to_vec();
    args.extend(["--entry", "entry"]);
    let (code, done) = gk(&root, &args, &[]);
    assert_eq!(code, 0, "a slower leg only warns: {done}");
    assert!(
        done.contains("legs: local 1m40s (integration 1m00s) WARN +66%"),
        "{done}"
    );
    fs::write(
        entry.join("health.json"),
        r#"{"schema": 1, "legs": {"local": 95000}}"#,
    )
    .expect("health");
    let (_, done) = gk(&root, &args, &[]);
    assert!(
        done.contains("legs: local 1m40s (integration 1m35s)\n"),
        "{done}"
    );
    let written: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("out/health.json")).expect("written"))
            .expect("json");
    assert_eq!(written["legs"]["local"], 100_000, "{written}");
}
