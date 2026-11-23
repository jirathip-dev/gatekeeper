use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "packs = [\"core\"]\nintegration = \"staging\"\npin = \"checkout\"\n";
const ARCH: &str = "[[layer]]\nname = \"core\"\npaths = [\"src/core/**\"]\n\n[[layer]]\nname = \"server\"\npaths = [\"src/server/**\"]\nmods = [\"server\"]\n\n[[size]]\npaths = [\"src/**/*.rs\"]\nmax_file_lines = 3\nmax_fn_lines = 2\n";
const LISTED: &str = "src/big.rs 4\nsrc/big.rs:big 4\nsrc/core/a.rs -> server\n";

fn repo() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("arch-baseline");
    if root.exists() {
        fs::remove_dir_all(&root).expect("clean");
    }
    for (path, text) in [
        (".gatekeeper/config.toml", CONFIG),
        (".drover/arch.toml", ARCH),
        ("src/core/a.rs", "use server::api;\n"),
        ("src/big.rs", "fn big() {\n    a();\n    b();\n}\n"),
    ] {
        write(&root, path, text);
    }
    root
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    fs::write(path, text).expect("write");
}

fn gk(root: &Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_gatekeeper"))
        .args(args)
        .current_dir(root)
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env_remove("GITHUB_ACTIONS")
        .output()
        .expect("gatekeeper runs");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code().expect("exit code"), text)
}

#[test]
fn baseline_core_arch_writes_the_arch_baseline_once_and_then_only_shrinks_it() {
    let root = repo();
    let file = root.join(".drover/arch-baseline.txt");
    let check = ["check", "--gate", "core/arch"];
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("src/core/a.rs:1: layer `core` uses `server` (server::api)"),
        "{out}"
    );

    let (code, out) = gk(&root, &["baseline", "core/arch"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(".drover/arch-baseline.txt: 3 findings"),
        "{out}"
    );
    assert_eq!(fs::read_to_string(&file).expect("listed"), LISTED);
    assert!(!root.join(".gatekeeper/baseline").exists());
    let (code, out) = gk(&root, &check);
    assert_eq!(code, 0, "{out}");

    write(
        &root,
        "src/big.rs",
        "fn big() {\n    a();\n    b();\n    c();\n}\n",
    );
    write(&root, "src/core/b.rs", "use server::x;\n");
    let (code, out) = gk(&root, &["baseline", "core/arch"]);
    assert_eq!(code, 1, "{out}");
    for line in [
        "new   src/big.rs 5",
        "new   src/big.rs:big 5",
        "new   src/core/b.rs -> server",
    ] {
        assert!(out.contains(line), "missing `{line}` in\n{out}");
    }
    assert_eq!(fs::read_to_string(&file).expect("listed"), LISTED);

    write(&root, "src/big.rs", "fn big() {\n    a();\n}\n");
    write(&root, "src/core/a.rs", "use std::fmt;\n");
    fs::remove_file(root.join("src/core/b.rs")).expect("rm");
    let (code, out) = gk(&root, &["baseline", "core/arch"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        fs::read_to_string(&file).expect("listed"),
        "src/big.rs:big 3\n"
    );

    write(&root, "src/big.rs", "fn big() {}\n");
    let (code, out) = gk(&root, &["baseline", "core/arch"]);
    assert_eq!(code, 0, "{out}");
    assert!(!file.exists());
    assert_eq!(gk(&root, &check).0, 0);
}
