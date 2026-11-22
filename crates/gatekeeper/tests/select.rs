use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "packs = [\"ts\"]\nintegration = \"staging\"\npin = \"checkout\"\n\n[ts]\nprojects = [\".\"]\ninstall = \"true\"\n\n[scope]\ngraphs = [\"pnpm\"]\n";
const UNIT: &str = "about = \"unit\"\nfix = \"fix the test\"\nbudget_secs = 30\nrun = \"printf '%s\\\\n' {files} > ran.txt\"\n\n[select]\nvitest = \"vitest.config.ts\"\nfull = []\n";
const SHARDED: &str = "about = \"unit\"\nfix = \"fix the test\"\nbudget_secs = 30\nshards = { hosted = 2, self-hosted = 1 }\nrun = \"sh report.sh {report} {files}\"\n\n[select]\nvitest = \"vitest.config.ts\"\nfull = []\n";
const REPORT: &str = "out=$1; shift\nprintf '%s\\n' \"$@\" > ran.txt\nsep=\nprintf '{\"testResults\":[' > \"$out\"\nfor f in \"$@\"; do\n  case \"$f\" in *a.test.ts) t=100 ;; *b.test.ts) t=200 ;; *) t=300 ;; esac\n  printf '%s{\"name\":\"%s/%s\",\"startTime\":1000,\"endTime\":%s}' \"$sep\" \"$PWD\" \"$f\" \"$((1000 + t))\" >> \"$out\"\n  sep=,\ndone\nprintf ']}\\n' >> \"$out\"\n";
const HOSTED: [(&str, &str); 2] = [
    ("GITHUB_ACTIONS", "true"),
    ("RUNNER_ENVIRONMENT", "github-hosted"),
];
const STUB: &str = "#!/bin/sh\necho \"$*\" >> \"$(dirname \"$0\")/args.txt\"\ncase \"$*\" in\n  *--changed*) cat \"$(dirname \"$0\")/closure.json\" ;;\n  *) cat \"$(dirname \"$0\")/universe.json\" ;;\nesac\n";

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    fs::write(path, text).expect("file");
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@localhost"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn listed(files: &[&str]) -> String {
    let entries: Vec<String> = files
        .iter()
        .map(|f| format!("{{\"file\":\"{f}\"}}"))
        .collect();
    format!("[{}]", entries.join(","))
}

fn repo(name: &str, tests: &[&str], (stub, unit): (&str, &str)) -> (PathBuf, String) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    write(&root, ".gatekeeper/config.toml", CONFIG);
    write(&root, ".gatekeeper/gates/unit.toml", unit);
    write(&root, ".gitignore", "target/\nnode_modules/\ncache/\n");
    write(&root, "pnpm-workspace.yaml", "packages:\n  - 'pkgs/*'\n");
    write(&root, "pkgs/a/package.json", "{\"name\": \"a\"}");
    write(&root, "pkgs/b/package.json", "{\"name\": \"b\"}");
    write(&root, "vitest.config.ts", "export default {};\n");
    for test in tests {
        write(&root, test, "import { it } from 'vitest';\n");
    }
    let bin = root.join("node_modules/.bin");
    write(&root, "node_modules/.bin/vitest", stub);
    let chmod = Command::new("chmod")
        .arg("+x")
        .arg(bin.join("vitest"))
        .status();
    assert!(chmod.expect("chmod").success());
    fs::write(bin.join("universe.json"), listed(tests)).expect("universe");
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "base"]);
    let base = git(&root, &["rev-parse", "HEAD"]);
    (root, base)
}

fn gk(root: &Path, args: &[&str]) -> (i32, String) {
    gk_on(root, args, &[])
}

fn gk_on(root: &Path, args: &[&str], env: &[(&str, &str)]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gatekeeper"));
    cmd.args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"));
    for var in [
        "GITHUB_ACTIONS",
        "RUNNER_ENVIRONMENT",
        "GATEKEEPER_JOB_START",
        "GATEKEEPER_CACHE_HIT",
        "GITHUB_EVENT_NAME",
        "GITHUB_EVENT_PATH",
        "GITHUB_REF_NAME",
    ] {
        cmd.env_remove(var);
    }
    cmd.envs(env.iter().copied());
    let out = cmd.output().expect("gatekeeper runs");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code().expect("exit code"), text)
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn check_prints_the_selection_and_writes_the_list() {
    let tests = [
        "pkgs/a/a.test.ts",
        "pkgs/b/b.test.ts",
        "pkgs/b/conf.test.ts",
    ];
    let (root, base) = repo("select-check", &tests, (STUB, UNIT));
    write(
        &root,
        "pkgs/b/conf.test.ts",
        "readFileSync(join(docs, 'conf.toml'));\n",
    );
    write(&root, "pkgs/a/a.ts", "export const a = 1;\n");
    write(&root, "docs/conf.toml", "x = 1\n");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "change"]);
    let closure = listed(&["pkgs/a/a.test.ts"]);
    fs::write(root.join("node_modules/.bin/closure.json"), closure).expect("closure");
    let (code, out) = gk(&root, &["check", "--gate", "local/unit", "--base", &base]);
    assert_eq!(code, 0, "{out}");
    let line = "      selected 2 of 3 test files in ";
    let at = out
        .find(line)
        .unwrap_or_else(|| panic!("no selection line in\n{out}"));
    let rest = out[at..].lines().next().expect("line");
    assert!(rest.ends_with("s: closure 1 · readers 1"), "{rest}");
    assert!(
        out.contains("      list: target/gatekeeper/local-unit.selection.json"),
        "{out}"
    );
    let args = read(&root, "node_modules/.bin/args.txt");
    let calls: Vec<&str> = args.lines().collect();
    let all = "list --filesOnly --json --passWithNoTests --config vitest.config.ts";
    assert_eq!(
        calls,
        [all.to_owned(), format!("{all} --changed {base}")],
        "{args}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&read(&root, "target/gatekeeper/local-unit.selection.json"))
            .expect("json");
    let files = serde_json::json!({
        "pkgs/a/a.test.ts": ["closure"],
        "pkgs/b/conf.test.ts": [{ "reads": { "basename": "docs/conf.toml" } }],
    });
    assert_eq!(json["files"], files, "{json}");
    assert_eq!(json["universe"], 3, "{json}");
    assert_eq!(json["base"], base.as_str(), "{json}");
    let ran = read(&root, "ran.txt");
    assert_eq!(ran, "pkgs/a/a.test.ts\npkgs/b/conf.test.ts\n");
}

#[test]
fn files_are_passed_as_arguments() {
    let tests: Vec<String> = (0..5000)
        .map(|i| format!("pkgs/a/src/a-long-enough-directory-name/test-{i:05}.test.ts"))
        .collect();
    let names: Vec<&str> = tests.iter().map(String::as_str).collect();
    let (root, _) = repo("select-arguments", &names, (STUB, UNIT));
    let (code, out) = gk(&root, &["check", "--gate", "local/unit", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("      selected all 5000 test files: --all or scope full"),
        "{out}"
    );
    let ran = read(&root, "ran.txt");
    let mut want = tests.clone();
    want.sort();
    assert_eq!(ran.lines().collect::<Vec<_>>(), want);
    assert!(ran.len() > 128 * 1024, "the list fits in one argument");
}

#[test]
fn a_failed_list_fails_the_gate() {
    let stub = "#!/bin/sh\necho 'boom: vitest crashed' >&2\nexit 1\n";
    let (root, base) = repo("select-failed", &["pkgs/a/a.test.ts"], (stub, UNIT));
    write(&root, "pkgs/a/a.ts", "export const a = 1;\n");
    let (code, out) = gk(&root, &["check", "--gate", "local/unit", "--base", &base]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("FAIL  local/unit"), "{out}");
    assert!(out.contains("boom: vitest crashed"), "{out}");
    assert!(
        out.contains("log: target/gatekeeper/local-unit.select.log"),
        "{out}"
    );
    assert!(
        !root.join("ran.txt").exists(),
        "the gate's line ran:\n{out}"
    );
}

#[test]
fn a_sharded_select_gate_needs_a_report() {
    let unit = SHARDED.replace("sh report.sh {report} {files}", "sh report.sh {files}");
    let (root, _) = repo("select-no-report", &["pkgs/a/a.test.ts"], (STUB, &unit));
    let (code, out) = gk(&root, &["check", "--gate", "local/unit", "--all"]);
    assert_eq!(code, 2, "{out}");
    assert!(
        out.contains("a sharded [select] gate needs `{report}` in `run`"),
        "{out}"
    );
    assert!(
        !root.join("ran.txt").exists(),
        "the gate's line ran:\n{out}"
    );
}

fn plan_lines(root: &Path, env: &[(&str, &str)]) -> (serde_json::Value, String) {
    let args = ["ci", "plan", "--timings", "gatekeeper-timings-1-9113"];
    let (code, out) = gk_on(root, &args, env);
    assert_eq!(code, 0, "{out}");
    let matrix = out.lines().find_map(|l| l.strip_prefix("matrix="));
    let matrix = serde_json::from_str(matrix.expect("matrix line")).expect("json");
    let save = out.lines().find(|l| l.starts_with("timings-save="));
    (matrix, save.expect("timings-save line").to_owned())
}

#[test]
fn ci_plan_names_one_timings_entry() {
    let (root, _) = repo("select-plan", &["pkgs/a/a.test.ts"], (STUB, SHARDED));
    let (matrix, save) = plan_lines(&root, &[]);
    let entries = matrix["include"].as_array().expect("include");
    let local: Vec<_> = entries.iter().filter(|e| e["leg"] == "local").collect();
    let shards: Vec<_> = local.iter().map(|e| e["shard"].clone()).collect();
    assert_eq!(shards, ["1/2", "2/2"], "{matrix}");
    for entry in &local {
        assert_eq!(entry["timings"], "gatekeeper-timings-1-9113", "{matrix}");
    }
    let others = entries.iter().filter(|e| e["leg"] != "local");
    assert!(others.clone().count() > 0, "{matrix}");
    assert!(
        others.clone().all(|e| e.get("timings").is_none()),
        "{matrix}"
    );
    assert_eq!(
        save, "timings-save=",
        "only a push to the integration branch writes"
    );
    let push = [("GITHUB_ACTIONS", "true"), ("GITHUB_EVENT_NAME", "push")];
    let staging = [push[0], push[1], ("GITHUB_REF_NAME", "staging")];
    assert_eq!(
        plan_lines(&root, &staging).1,
        "timings-save=gatekeeper-timings-1-"
    );
    let feature = [push[0], push[1], ("GITHUB_REF_NAME", "feature")];
    assert_eq!(plan_lines(&root, &feature).1, "timings-save=");
}

fn shard(root: &Path, index: u32, timings: Option<&str>) -> String {
    let at = format!("{index}/2");
    let mut args = vec!["check", "--gate", "local/unit", "--shard", &at, "--all"];
    args.extend(timings.iter().flat_map(|t| ["--timings", *t]));
    let (code, out) = gk_on(root, &args, &HOSTED);
    assert_eq!(code, 0, "{out}");
    let to = root.join(format!("reports/gates-local-{index}"));
    fs::create_dir_all(&to).expect("reports");
    fs::copy(
        root.join("target/gatekeeper/report.json"),
        to.join("report.json"),
    )
    .expect("copy");
    read(root, "ran.txt")
}

#[test]
fn ci_done_merges_the_times() {
    let tests = ["pkgs/a/a.test.ts", "pkgs/b/b.test.ts", "pkgs/b/c.test.ts"];
    let (root, _) = repo("select-timings", &tests, (STUB, SHARDED));
    write(&root, "report.sh", REPORT);
    assert_eq!(
        shard(&root, 1, None),
        "pkgs/a/a.test.ts\npkgs/b/c.test.ts\n"
    );
    assert_eq!(shard(&root, 2, None), "pkgs/b/b.test.ts\n");
    let done = ["ci", "done", "--pack", "local", "--reports", "reports"];
    let (code, out) = gk(&root, &[&done[..], &["--timings-out", "timings"]].concat());
    assert_eq!(code, 0, "{out}");
    let merged: serde_json::Value =
        serde_json::from_str(&read(&root, "timings/local-unit.json")).expect("json");
    let files = serde_json::json!({
        "pkgs/a/a.test.ts": 100, "pkgs/b/b.test.ts": 200, "pkgs/b/c.test.ts": 300,
    });
    assert_eq!(merged, serde_json::json!({ "schema": 1, "files": files }));
    assert_eq!(shard(&root, 1, Some("timings")), "pkgs/b/c.test.ts\n");
    let (code, out) = gk(&root, &done);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("FAIL local/unit: its shards selected different lists (local 1/2 "),
        "{out}"
    );
}

#[test]
fn an_empty_shard_is_skipped_and_passes() {
    let (root, _) = repo("select-empty-shard", &["pkgs/a/a.test.ts"], (STUB, SHARDED));
    let args = ["check", "--gate", "local/unit", "--shard", "2/2", "--all"];
    let (code, out) = gk_on(&root, &args, &HOSTED);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("no selected test file fell in this shard"),
        "{out}"
    );
    assert!(
        !root.join("ran.txt").exists(),
        "the gate's line ran:\n{out}"
    );
}
