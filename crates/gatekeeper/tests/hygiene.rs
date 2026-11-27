use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn dir(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(&root).expect("dir");
    root
}

fn gk(root: &Path, args: &[&str], env: &[(&str, &str)]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gatekeeper"));
    cmd.args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"));
    for var in [
        "GITHUB_ACTIONS",
        "GITHUB_EVENT_PATH",
        "GITHUB_EVENT_NAME",
        "RUNNER_ENVIRONMENT",
    ] {
        cmd.env_remove(var);
    }
    let out = cmd
        .envs(env.iter().copied())
        .output()
        .expect("gatekeeper runs");
    let text = String::from_utf8_lossy(&out.stdout) + String::from_utf8_lossy(&out.stderr);
    (out.status.code().expect("exit code"), text.into_owned())
}

#[test]
fn off_for_core_hygiene_is_refused_in_a_public_repos_ci_run_only() {
    let root = dir("hygiene-off");
    fs::create_dir_all(root.join(".gatekeeper")).expect("dir");
    let config = "packs = [\"core\"]\nintegration = \"staging\"\npin = \"checkout\"\n\n[off]\n\"core/hygiene\" = \"private repo: nothing here is published\"\n";
    fs::write(root.join(".gatekeeper/config.toml"), config).expect("config");
    let private = root.join("event.json");
    fs::write(&private, r#"{"repository":{"private":true}}"#).expect("event");
    let private = private.to_str().expect("utf8");
    let says =
        "[off] `core/hygiene`: a public repo's CI run checks it; only a private repo turns it off";
    let (code, out) = gk(&root, &["list"], &[("GITHUB_ACTIONS", "true")]);
    assert_eq!(code, 2, "public CI: {out}");
    assert!(out.contains(says), "public CI: {out}");
    let private_ci = [("GITHUB_ACTIONS", "true"), ("GITHUB_EVENT_PATH", private)];
    for (run, env) in [("lane", &[][..]), ("private CI", &private_ci[..])] {
        let (code, out) = gk(&root, &["list"], env);
        assert_eq!(code, 0, "{run}: {out}");
        assert!(
            out.contains("private repo: nothing here is published"),
            "{run}: {out}"
        );
    }
}

#[test]
fn fixture_writes_the_shared_names_case_and_nothing_else() {
    let root = dir("hygiene-fixture");
    let to = root.join("golden");
    fs::create_dir_all(&to).expect("dir");
    fs::write(to.join("stale"), "old").expect("stale");
    let args = ["fixture", "core/hygiene/shared-names", "--to", "golden"];
    let (code, out) = gk(&root, &args, &[]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("wrote "), "{out}");
    let case = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packs/core/fixtures/hygiene/shared-names");
    let mut names: Vec<String> = fs::read_dir(&to)
        .expect("golden")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["expect", "lines.md", "names"]);
    for f in &names {
        assert_eq!(
            fs::read(to.join(f)).expect("written"),
            fs::read(case.join(f)).expect("case"),
            "{f}"
        );
    }
    for bad in ["core/hygiene/nope", "core/hygiene", "core/hygiene/_base"] {
        let (code, out) = gk(&root, &["fixture", bad, "--to", "x"], &[]);
        assert_eq!(code, 2, "{bad}: {out}");
        assert!(!root.join("x").exists(), "{bad}");
    }
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

fn commit(root: &Path, message: &str) -> String {
    git(root, &["commit", "-q", "--allow-empty", "-m", message]);
    git(root, &["rev-parse", "HEAD"])
}

fn hygiene_on(root: &Path, event: &str, before: &str) -> (i32, String) {
    let path = root.join("event.json");
    let payload = format!(
        r#"{{"before":"{before}","repository":{{"private":false}},"pull_request":{{"head":{{"repo":{{"full_name":"o/r"}}}}}}}}"#
    );
    fs::write(&path, payload).expect("event");
    let ci = [
        ("GITHUB_ACTIONS", "true"),
        ("GITHUB_EVENT_NAME", event),
        ("GITHUB_EVENT_PATH", path.to_str().expect("utf8")),
        ("GITHUB_REF", "refs/pull/7/merge"),
        ("GITHUB_REPOSITORY", "o/r"),
        ("DROVER_PRIVATE_NAMES", "acme-secret"),
    ];
    let env = if event == "lane" { &ci[4..] } else { &ci[..] };
    gk(root, &["check", "--gate", "core/hygiene"], env)
}

#[test]
fn a_push_run_scans_only_the_pushed_commit_messages() {
    let root = dir("hygiene-push");
    fs::create_dir_all(root.join(".gatekeeper")).expect("dir");
    let config = "packs = [\"core\"]\nintegration = \"staging\"\npin = \"checkout\"\n";
    fs::write(root.join(".gatekeeper/config.toml"), config).expect("config");
    fs::write(root.join(".gitignore"), "cache/\ntarget/\nevent.json\n").expect("ignore");
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    let base = commit(&root, "init");
    git(&root, &["update-ref", "refs/remotes/origin/staging", &base]);
    let old = commit(&root, "release for acme-secret");
    let before = commit(&root, "clean one");
    commit(&root, "clean two");
    let old_hit = format!("private-name #1 at commit {} message", &old[..7]);
    let (code, out) = hygiene_on(&root, "push", &before);
    assert_eq!(code, 0, "clean push: {out}");
    assert!(!out.contains(&old_hit), "clean push: {out}");
    let pushed = git(&root, &["rev-parse", "HEAD"]);
    let leak = commit(&root, "leak acme-secret");
    let leak_hit = format!("private-name #1 at commit {} message", &leak[..7]);
    let (code, out) = hygiene_on(&root, "push", &pushed);
    assert_ne!(code, 0, "leaky push: {out}");
    assert!(out.contains(&leak_hit), "leaky push: {out}");
    assert!(!out.contains(&old_hit), "leaky push: {out}");
    let orphan = git(&root, &["commit-tree", "-m", "elsewhere", "HEAD^{tree}"]);
    let zeros = "0".repeat(40);
    let today = [
        ("push", zeros.as_str()),
        ("push", orphan.as_str()),
        ("push", ""),
        ("pull_request", before.as_str()),
        ("lane", before.as_str()),
    ];
    for (event, before) in today {
        let (code, out) = hygiene_on(&root, event, before);
        assert_ne!(code, 0, "{event} {before}: {out}");
        assert!(out.contains(&old_hit), "{event} {before}: {out}");
        assert!(out.contains(&leak_hit), "{event} {before}: {out}");
    }
}
