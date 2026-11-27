mod common;

use common::git;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "packs = [\"ts\"]\nintegration = \"staging\"\npin = \"checkout\"\n\n[ts]\nprojects = [\".\"]\ninstall = \"mkdir -p node_modules/.bin && cp stub/* node_modules/.bin/\"\n\n[scope]\ngraphs = [\"pnpm\"]\n";
const UNIT: &str = "about = \"unit\"\nfix = \"fix the test\"\nbudget_secs = 30\nrun = \"printf '%s\\\\n' {files} > ran.txt\"\n\n[select]\nvitest = \"vitest.config.ts\"\nfull = []\n";
const STUB: &str = "#!/bin/sh\ncase \"$*\" in\n  *--changed*) cat \"$(dirname \"$0\")/closure.json\" ;;\n  *) cat \"$(dirname \"$0\")/universe.json\" ;;\nesac\n";
const TESTS: [&str; 3] = ["pkgs/a/a.test.ts", "pkgs/b/b.test.ts", "pkgs/b/c.test.ts"];
const OTHER: &str = "test {base} = \"$(git rev-parse HEAD~1)\" && cat other.txt";

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    fs::write(path, text).expect("file");
}

fn listed(files: &[&str]) -> String {
    let entries: Vec<String> = files
        .iter()
        .map(|f| format!("{{\"file\":\"{f}\"}}"))
        .collect();
    format!("[{}]", entries.join(","))
}

fn repo(name: &str) -> (PathBuf, String) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    write(&root, ".gatekeeper/config.toml", CONFIG);
    write(&root, ".gatekeeper/gates/unit.toml", UNIT);
    write(&root, ".gitignore", "target/\nnode_modules/\ncache/\n");
    write(&root, "pnpm-workspace.yaml", "packages:\n  - 'pkgs/*'\n");
    write(&root, "pkgs/a/package.json", "{\"name\": \"a\"}");
    write(&root, "pkgs/b/package.json", "{\"name\": \"b\"}");
    write(&root, "vitest.config.ts", "export default {};\n");
    for test in TESTS {
        write(&root, test, "import { it } from 'vitest';\n");
    }
    write(&root, "stub/vitest", STUB);
    write(&root, "stub/universe.json", &listed(&TESTS));
    let chmod = Command::new("chmod")
        .arg("+x")
        .arg(root.join("stub/vitest"))
        .status();
    assert!(chmod.expect("chmod").success());
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "base"]);
    let base = git(&root, &["rev-parse", "HEAD"]);
    (root, base)
}

fn pull_request(root: &Path, base: &str, pr: u64, (closure, other): (&[&str], &[&str])) -> String {
    git(root, &["checkout", "-q", "-b", &format!("pr{pr}"), base]);
    let pkg = if pr == 1 { "pkgs/a" } else { "pkgs/b" };
    write(root, &format!("{pkg}/src{pr}.ts"), "export const x = 1;\n");
    write(root, "stub/closure.json", &listed(closure));
    write(root, "other.txt", &(other.join("\n") + "\n"));
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", &format!("pr {pr}")]);
    let head = git(root, &["rev-parse", "HEAD"]);
    git(root, &["checkout", "-q", base]);
    head
}

fn runs(root: &Path, lines: &[(u64, &str, &str, &[&str])]) {
    let text: Vec<String> = lines
        .iter()
        .map(|(pr, base, head, failed)| {
            let failed = serde_json::to_string(failed).expect("json");
            format!(
                "{{\"pr\": {pr}, \"base\": \"{base}\", \"head\": \"{head}\", \"failed\": {failed}}}"
            )
        })
        .collect();
    fs::write(root.join("runs.jsonl"), text.join("\n") + "\n").expect("runs");
}

fn gk(root: &Path, args: &[&str]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gatekeeper"));
    cmd.args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"));
    for var in [
        "GITHUB_ACTIONS",
        "RUNNER_ENVIRONMENT",
        "GATEKEEPER_JOB_START",
    ] {
        cmd.env_remove(var);
    }
    let out = cmd.output().expect("gatekeeper runs");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code().expect("exit code"), text)
}

fn replay(root: &Path) -> (i32, String) {
    let args = ["replay", "--gate", "local/unit", "--runs", "runs.jsonl"];
    gk(root, &[&args[..], &["--other", OTHER]].concat())
}

#[test]
fn replay_reports_counts_totals_and_missed_failures() {
    let (root, base) = repo("replay-report");
    let one = pull_request(&root, &base, 1, (&["pkgs/a/a.test.ts"], &TESTS));
    let two = (&["pkgs/b/b.test.ts"][..], &["pkgs/b/c.test.ts"][..]);
    let two = pull_request(&root, &base, 2, two);
    runs(
        &root,
        &[
            (1, &base, &one, &[]),
            (2, &base, &two, &["pkgs/b/b.test.ts"]),
        ],
    );
    let (code, out) = replay(&root);
    assert_eq!(code, 0, "{out}");
    let want = [
        "replay local/unit · 2 runs of 2 pull requests · 3 test files".to_owned(),
        "| pr   | head    | changed | gatekeeper          | other      | only gatekeeper | only other | failed | missed: gatekeeper | other |".to_owned(),
        format!("| 1    | {} |       3 | 1                   | all (3)    |               0 |          2 |      0 |                  0 |     0 |", &one[..7]),
        format!("| 2    | {} |       3 | 1                   | 1          |               1 |          1 |      1 |                  0 |     1 |", &two[..7]),
        "totals: gatekeeper 2 file runs (33.3 %), 0 full · other 4 (66.6 %), 1 full · missed: gatekeeper 0, other 1".to_owned(),
        "by rule: closure 2".to_owned(),
        format!("missed by other: pr 2 {} pkgs/b/b.test.ts", &two[..7]),
        "wrote target/gatekeeper/replay/replay.json and replay.md".to_owned(),
    ];
    assert_eq!(out.lines().collect::<Vec<_>>(), want, "{out}");
    let json = fs::read_to_string(root.join("target/gatekeeper/replay/replay.json")).expect("json");
    let json: serde_json::Value = serde_json::from_str(&json).expect("json");
    let second = &json["runs"][1];
    assert_eq!(second["base"], base.as_str(), "{json}");
    assert_eq!(
        second["only_gatekeeper"],
        serde_json::json!(["pkgs/b/b.test.ts"])
    );
    assert_eq!(
        second["only_other"],
        serde_json::json!(["pkgs/b/c.test.ts"])
    );
    assert_eq!(
        second["missed_other"],
        serde_json::json!(["pkgs/b/b.test.ts"])
    );
    assert_eq!(json["totals"]["file_runs"], 6, "{json}");
    let md = fs::read_to_string(root.join("target/gatekeeper/replay/replay.md")).expect("md");
    assert_eq!(md.lines().count(), 7, "{md}");
    assert!(!root.join("ran.txt").exists(), "replay ran the gate's line");
    assert!(!root.join("target/gatekeeper/replay/worktree").exists());
}

#[test]
fn replay_exits_1_on_a_missed_failure() {
    let (root, base) = repo("replay-missed");
    let three = (&["pkgs/a/a.test.ts"][..], &["pkgs/a/a.test.ts"][..]);
    let head = pull_request(&root, &base, 3, three);
    runs(&root, &[(3, &base, &head, &["pkgs/b/c.test.ts"])]);
    let (code, out) = replay(&root);
    assert_eq!(code, 1, "{out}");
    let missed = format!("missed by gatekeeper: pr 3 {} pkgs/b/c.test.ts", &head[..7]);
    assert!(out.lines().any(|l| l == missed), "{out}");
    assert!(out.contains("missed: gatekeeper 1, other 1"), "{out}");
}
