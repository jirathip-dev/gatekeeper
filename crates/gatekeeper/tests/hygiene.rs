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
