use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;

const FAKE: &[u8] = b"#!/bin/sh\necho \"pinned gatekeeper: $*\"\n";

fn sha256(path: &Path) -> String {
    let out = Command::new("sh")
        .args([
            "-c",
            "{ sha256sum \"$0\" 2>/dev/null || shasum -a 256 \"$0\"; } | cut -d' ' -f1",
        ])
        .arg(path)
        .output()
        .expect("sha256");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn consumer(name: &str) -> (PathBuf, PathBuf, String) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    fs::create_dir_all(root.join(".gatekeeper")).expect("dir");
    let wrapper = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/core/files/run");
    fs::copy(&wrapper, root.join(".gatekeeper/run")).expect("wrapper");
    let fake = root.join("fake");
    fs::write(&fake, FAKE).expect("fake");
    let sum = sha256(&fake);
    let config = format!(
        "packs = [\"core\"]\nintegration = \"staging\"\n\n[pin]\nversion = \"9.9.9\"\n\
         x86_64-unknown-linux-gnu = \"{sum}\"\naarch64-apple-darwin = \"{sum}\"\n"
    );
    fs::write(root.join(".gatekeeper/config.toml"), config).expect("config");
    let cached = root.join("cache/gatekeeper/bin").join(&sum);
    fs::create_dir_all(&cached).expect("cache");
    (root, cached.join("gatekeeper"), sum)
}

fn wrapper(root: &Path) -> (i32, String) {
    let out = Command::new(root.join(".gatekeeper/run"))
        .args(["check", "--pack", "core"])
        .env("XDG_CACHE_HOME", root.join("cache"))
        .output()
        .expect("wrapper runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().expect("code"), text)
}

#[test]
fn the_wrapper_runs_the_cached_binary_whose_sha256_the_pin_names() {
    let (root, bin, _) = consumer("wrapper-hit");
    fs::write(&bin, FAKE).expect("bin");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod");
    let (code, out) = wrapper(&root);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out.trim(), "pinned gatekeeper: check --pack core");
}

#[test]
fn the_wrapper_refuses_a_cached_binary_whose_sha256_differs() {
    let (root, bin, _) = consumer("wrapper-mismatch");
    fs::write(&bin, b"#!/bin/sh\necho tampered\n").expect("bin");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod");
    let (code, out) = wrapper(&root);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("sha256 differs from [pin]: refused"), "{out}");
    assert!(!out.contains("tampered"), "{out}");
}
