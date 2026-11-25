use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "packs = [\"core\"]\nintegration = \"staging\"\npin = \"checkout\"\n";

fn repo(name: &str, config: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    write(&root, ".gatekeeper/config.toml", config);
    write(&root, "src/lib.rs", &"fn a() {}\n".repeat(10));
    write(&root, "tests/t.rs", &"fn t() {}\n".repeat(12));
    run(&root, "git", &["init", "-q"]);
    run(&root, "git", &["add", "-A"]);
    root
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    fs::write(path, text).expect("write");
}

fn run(root: &Path, bin: &str, args: &[&str]) -> (i32, String) {
    let out = Command::new(bin)
        .args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("GATEKEEPER_TODAY", "2026-12-09")
        .env_remove("GITHUB_ACTIONS")
        .output()
        .expect("runs");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code().expect("exit code"), text)
}

fn gk(root: &Path, args: &[&str]) -> (i32, String) {
    run(root, env!("CARGO_BIN_EXE_gatekeeper"), args)
}

fn ceilings(root: &Path) -> Vec<String> {
    let dir = fs::read_dir(root.join("ci/tests/ratio.d")).expect("ratio.d");
    let mut names: Vec<String> = dir
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn baseline_writes_the_first_ceiling_then_only_a_lower_one() {
    let root = repo("test-health-baseline", CONFIG);
    let check = ["check", "--gate", "core/test-health"];
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("no ceiling in ci/tests/ratio.d"), "{out}");
    let (code, out) = gk(&root, &["baseline", "core/test-health"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("ci/tests/ratio.d/2026-12-09-120.toml: ceiling 1.20 (the first)"),
        "{out}"
    );
    let text = fs::read_to_string(root.join("ci/tests/ratio.d/2026-12-09-120.toml"));
    assert_eq!(text.expect("ceiling"), "ceiling = 1.20\n");
    let (code, out) = gk(&root, &["baseline", "core/test-health"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("nothing written"), "{out}");
    assert_eq!(ceilings(&root), ["2026-12-09-120.toml"]);
    run(&root, "git", &["add", "-A"]);
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("tests 12 / src 10 lines: ratio 1.20, ceiling 1.20"),
        "{out}"
    );
    write(&root, "tests/t.rs", &"fn t() {}\n".repeat(5));
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("ratio 0.50 is a step under the ceiling 1.20"),
        "{out}"
    );
    let (code, out) = gk(&root, &["baseline", "core/test-health"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("2026-12-09-100.toml: ceiling 1.00 (was 1.20)"),
        "{out}"
    );
    run(&root, "git", &["add", "-A"]);
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_zero_budget_or_a_soak_glob_outside_tests_is_a_config_error() {
    for (tests, want) in [
        ("budget_secs = 0", "[tests] budget_secs is at least 1"),
        (
            "soak = [\"crates/*/src/**\"]",
            "[tests] soak `crates/*/src/**` can match a Rust file outside a tests/ folder",
        ),
    ] {
        let root = repo(
            "test-health-config",
            &format!("{CONFIG}\n[tests]\n{tests}\n"),
        );
        let (code, out) = gk(&root, &["check", "--gate", "core/test-health"]);
        assert_eq!(code, 2, "{out}");
        assert!(out.contains(want), "{out}");
    }
}

#[test]
fn baseline_lists_the_sleeps_then_only_shrinks() {
    const HIT: &str = concat!("thread::", "sleep(");
    let root = repo("test-sleeps-baseline", CONFIG);
    let sleep = format!("    std::{HIT}std::time::Duration::ZERO);\n");
    let wait = |n: usize| format!("#[test]\nfn w() {{\n{}}}\n", sleep.repeat(n));
    let check = ["check", "--gate", "core/test-sleeps"];
    let baseline = ["baseline", "core/test-sleeps"];
    let listed =
        |root: &Path| fs::read_to_string(root.join(".gatekeeper/baseline/core-test-sleeps.txt"));
    write(&root, "tests/w.rs", &wait(2));
    run(&root, "git", &["add", "-A"]);
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains(&format!("tests/w.rs: {HIT} #2")), "{out}");
    let (code, out) = gk(&root, &baseline);
    assert_eq!(code, 0, "{out}");
    let two = format!("tests/w.rs: {HIT} #1\ntests/w.rs: {HIT} #2\n");
    assert_eq!(listed(&root).expect("list"), two);
    run(&root, "git", &["add", "-A"]);
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 0, "{out}");
    write(&root, "tests/w.rs", &wait(3));
    let (code, out) = gk(&root, &baseline);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains(&format!("new   tests/w.rs: {HIT} #3")),
        "{out}"
    );
    assert_eq!(listed(&root).expect("list"), two);
    write(&root, "tests/w.rs", &wait(1));
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("stale: shrink the baseline"), "{out}");
    let (code, out) = gk(&root, &baseline);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        listed(&root).expect("list"),
        format!("tests/w.rs: {HIT} #1\n")
    );
    run(&root, "git", &["add", "-A"]);
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 0, "{out}");
}
