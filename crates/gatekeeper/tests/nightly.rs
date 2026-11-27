use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "packs = []\nintegration = \"staging\"\npin = \"checkout\"\n";
const SOAK: &str = "about = \"the soak tests\"\nfix = \"fix the test the log names\"\nbudget_secs = 5\nwhen = \"nightly\"\nrun = \"true\"\n";
const UNIT: &str = "about = \"the unit tests\"\nfix = \"none\"\nbudget_secs = 5\nrun = \"true\"\n";

fn repo(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(root.join(".gatekeeper/gates")).expect("gates dir");
    fs::write(root.join(".gatekeeper/config.toml"), CONFIG).expect("config");
    fs::write(root.join(".gatekeeper/gates/soak.toml"), SOAK).expect("soak gate");
    fs::write(root.join(".gatekeeper/gates/unit.toml"), UNIT).expect("unit gate");
    root
}

fn check(root: &Path, event: Option<&str>) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gatekeeper"));
    cmd.arg("check")
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"));
    for var in [
        "GITHUB_ACTIONS",
        "GITHUB_EVENT_NAME",
        "GITHUB_EVENT_PATH",
        "GITHUB_REF",
        "RUNNER_ENVIRONMENT",
        "GATEKEEPER_CACHE_HIT",
        "GATEKEEPER_JOB_START",
    ] {
        cmd.env_remove(var);
    }
    if let Some(event) = event {
        cmd.env("GITHUB_ACTIONS", "true")
            .env("RUNNER_ENVIRONMENT", "github-hosted")
            .env("GITHUB_EVENT_NAME", event)
            .env("GITHUB_REF", "refs/pull/7/merge");
    }
    let out = cmd.output().expect("gatekeeper runs");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{text}");
    text
}

fn line<'a>(out: &'a str, gate: &str) -> &'a str {
    let find = out
        .lines()
        .find(|l| l.split_whitespace().nth(1) == Some(gate));
    find.unwrap_or_else(|| panic!("no line for {gate} in\n{out}"))
}

#[test]
fn a_pull_request_and_a_lane_skip_a_nightly_gate_with_its_reason() {
    let root = repo("nightly-skipped");
    for (who, event) in [("lane", None), ("pull request", Some("pull_request"))] {
        let out = check(&root, event);
        let soak = line(&out, "local/soak");
        assert!(soak.starts_with("skip  "), "{who}: {soak}");
        assert!(
            soak.ends_with("a nightly gate: it runs on the workflow's schedule event"),
            "{who}: {soak}"
        );
        assert!(
            line(&out, "local/unit").starts_with("ok    "),
            "{who}: {out}"
        );
    }
}

#[test]
fn a_scheduled_run_runs_only_nightly_gates() {
    let root = repo("nightly-scheduled");
    let out = check(&root, Some("schedule"));
    assert!(line(&out, "local/soak").starts_with("ok    "), "{out}");
    let unit = line(&out, "local/unit");
    assert!(unit.starts_with("skip  "), "{unit}");
    assert!(
        unit.ends_with("a scheduled run runs nightly gates only"),
        "{unit}"
    );
}
