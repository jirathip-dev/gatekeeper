mod common;

use common::git;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "packs = []\nintegration = \"staging\"\npin = \"checkout\"\n";
const GATE: &str = "about = \"line coverage\"\nfix = \"cover it\"\nbudget_secs = 5\nneeds = [\"coverage\"]\nrun = \"true\"\n";
const LLVM: &str = "[coverage]\ntool = \"llvm-cov\"\nsummary = \"cov.json\"\n";
const VITEST: &str = "[coverage]\ntool = \"vitest\"\nsummary = \"cov.json\"\n";

fn llvm(covered: u64, count: u64) -> String {
    format!(
        r#"{{"data":[{{"files":[],"totals":{{"lines":{{"count":{count},"covered":{covered},"percent":0}}}}}}],"type":"llvm.coverage.json.export","version":"2.0.1"}}"#
    )
}

fn vitest(covered: u64, total: u64) -> String {
    format!(
        r#"{{"total":{{"lines":{{"total":{total},"covered":{covered},"skipped":0,"pct":0}}}},"/r/src/a.ts":{{"lines":{{"total":1,"covered":1,"skipped":0,"pct":100}}}}}}"#
    )
}

fn repo(name: &str, table: &str, integration: Option<u32>) -> (PathBuf, String) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(root.join(".gatekeeper/gates")).expect("gates");
    fs::write(root.join(".gatekeeper/config.toml"), CONFIG).expect("config");
    let gate = format!("{GATE}\n{table}");
    fs::write(root.join(".gatekeeper/gates/coverage.toml"), gate).expect("gate");
    fs::write(
        root.join(".gitignore"),
        "cov.json\ncache/\nentry/\ntarget/\n",
    )
    .expect("ignore");
    if let Some(lines) = integration {
        fs::create_dir_all(root.join("entry")).expect("entry");
        let health = format!(
            r#"{{"schema":1,"coverage":[{{"gate":"local/coverage","lines":{lines}}}],"legs":{{}}}}"#
        );
        fs::write(root.join("entry/health.json"), health).expect("health");
    }
    git(&root, &["init", "-q", "-b", "staging"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "base"]);
    let base = git(&root, &["rev-parse", "HEAD"]);
    (root, base)
}

fn gk(root: &Path, args: &[&str], ci: bool) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gatekeeper"));
    cmd.args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"));
    for var in ["GITHUB_ACTIONS", "RUNNER_ENVIRONMENT", "GATEKEEPER_HEAD"] {
        cmd.env_remove(var);
    }
    if ci {
        cmd.env("GITHUB_ACTIONS", "true")
            .env("GATEKEEPER_HEAD", "abc");
    }
    let out = cmd.output().expect("gatekeeper runs");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code().expect("exit code"), text)
}

fn pr(root: &Path, base: &str, summary: &str) -> (i32, String) {
    fs::write(root.join("cov.json"), summary).expect("summary");
    let args = ["check", "--gate", "local/coverage", "--base", base];
    let mut args = args.to_vec();
    args.extend(["--timings", "entry"]);
    gk(root, &args, true)
}

fn drop_commit(root: &Path, message: &str) {
    fs::create_dir_all(root.join("ci/tests/coverage.d")).expect("dir");
    fs::write(
        root.join("ci/tests/coverage.d/2026-12-10-local-coverage-68.toml"),
        "gate = \"local/coverage\"\nfloor = 68.0\nwhy = \"owner decision q3410: duplicate tests go\"\n",
    )
    .expect("drop");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", message]);
}

fn within_and_under(name: &str, table: &str, summary: fn(u64, u64) -> String) {
    let (root, base) = repo(name, table, Some(716));
    let (code, out) = pr(&root, &base, &summary(715, 1000));
    assert_eq!(code, 0, "{name}: {out}");
    assert!(
        out.contains("line coverage 71.5 % (integration 71.6 %)"),
        "{name}: {out}"
    );
    let (code, out) = pr(&root, &base, &summary(714, 1000));
    assert_eq!(code, 1, "{name}: {out}");
    assert!(
        out.contains("line coverage 71.4 % is more than 0.1 under the integration branch's 71.6 %"),
        "{name}: {out}"
    );
}

#[test]
fn both_summaries_pass_within_the_tolerance_and_fail_under_it() {
    within_and_under("coverage-llvm", LLVM, llvm);
    within_and_under("coverage-vitest", VITEST, vitest);
}

#[test]
fn a_summary_without_line_totals_fails() {
    let (root, base) = repo("coverage-wrong-tool", VITEST, Some(716));
    let (code, out) = pr(&root, &base, &llvm(715, 1000));
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("cov.json: no line totals: is it a vitest json-summary summary"),
        "{out}"
    );
}

#[test]
fn with_no_integration_number_the_gate_passes_and_says_so() {
    let (root, base) = repo("coverage-no-entry", LLVM, None);
    let (code, out) = pr(&root, &base, &llvm(10, 1000));
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("line coverage 1.0 %: no integration number yet"),
        "{out}"
    );
}

#[test]
fn a_drop_with_its_trailer_passes_and_one_without_fails() {
    let (root, base) = repo("coverage-drop", LLVM, Some(716));
    drop_commit(&root, "test diet\n\nCoverage: 71.6 → 68.0 (#1760, q3410)\n");
    let (code, out) = pr(&root, &base, &llvm(685, 1000));
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("line coverage 68.5 % (integration 71.6 %): a deliberate drop to 68.0 %"),
        "{out}"
    );
    let (code, out) = pr(&root, &base, &llvm(679, 1000));
    assert_eq!(code, 1, "under the drop's floor: {out}");

    let (root, base) = repo("coverage-drop-untrailed", LLVM, Some(716));
    drop_commit(&root, "test diet\n");
    let (code, out) = pr(&root, &base, &llvm(685, 1000));
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("2026-12-10-local-coverage-68.toml: its commit")
            && out.contains("carries no `Coverage: <old> → 68.0 (#issue, q<N>)` trailer"),
        "{out}"
    );
}

#[test]
fn a_drop_landed_before_this_change_is_spent() {
    let (root, _) = repo("coverage-drop-spent", LLVM, Some(716));
    drop_commit(&root, "test diet\n\nCoverage: 71.6 → 68.0 (#1760, q3410)\n");
    let base = git(&root, &["rev-parse", "HEAD"]);
    let (code, out) = pr(&root, &base, &llvm(685, 1000));
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("more than 0.1 under"), "{out}");
}

#[test]
fn a_lane_skips_the_gate_with_its_reason() {
    let (root, _) = repo("coverage-lane", LLVM, Some(716));
    let (code, out) = gk(&root, &["check", "--gate", "local/coverage"], false);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("CI only on a lane's host: needs the integration branch's coverage"),
        "{out}"
    );
}

#[test]
fn a_coverage_table_without_the_capability_is_refused() {
    let (root, _) = repo("coverage-no-needs", LLVM, None);
    let gate = format!("{}\n{LLVM}", GATE.replace("needs = [\"coverage\"]\n", ""));
    fs::write(root.join(".gatekeeper/gates/coverage.toml"), gate).expect("gate");
    let (code, out) = gk(&root, &["check", "--gate", "local/coverage"], false);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("needs `needs = [\"coverage\"]`"), "{out}");
}

#[test]
fn ci_done_keeps_the_integration_number_and_the_matrix_asks_for_the_entry() {
    let (root, base) = repo("coverage-done", LLVM, Some(716));
    let (code, out) = gk(
        &root,
        &["ci", "plan", "--timings", "gatekeeper-timings-1-9"],
        true,
    );
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(r#""coverage":true"#) && out.contains(r#""timings":"gatekeeper-timings-1-9""#),
        "{out}"
    );
    let (code, out) = pr(&root, &base, &llvm(720, 1000));
    assert_eq!(code, 0, "{out}");
    let reports = root.join("reports/gates-local");
    fs::create_dir_all(&reports).expect("reports");
    fs::copy(
        root.join("target/gatekeeper/report.json"),
        reports.join("report.json"),
    )
    .expect("report");
    let args = ["ci", "done", "--reports", "reports", "--entry", "entry"];
    let mut args = args.to_vec();
    args.extend(["--timings-out", "out"]);
    let (code, done) = gk(&root, &args, true);
    assert_eq!(code, 0, "{done}");
    assert!(
        done.contains("coverage: local/coverage 72.0 % (integration 71.6 %)"),
        "{done}"
    );
    let written = fs::read_to_string(root.join("out/health.json")).expect("written");
    let written: serde_json::Value = serde_json::from_str(&written).expect("json");
    assert_eq!(
        written["coverage"][0]["gate"], "local/coverage",
        "{written}"
    );
    assert_eq!(written["coverage"][0]["lines"], 720, "{written}");
}
