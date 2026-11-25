use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn skill(to: &Path) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_gatekeeper"))
        .args(["skill", "--to"])
        .arg(to)
        .output()
        .expect("gatekeeper runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().expect("exit code"), text)
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn tree(root: &Path) -> Vec<(PathBuf, String)> {
    let mut paths = Vec::new();
    files(root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let text = fs::read_to_string(&p).expect("text");
            (
                p.strip_prefix(root).expect("under root").to_path_buf(),
                text,
            )
        })
        .collect()
}

#[test]
fn skill_to_writes_this_versions_skill_equal_to_the_checked_in_one_and_replaces_a_stale_one() {
    let to = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("skill-to");
    let stale = to.join("gatekeeper/examples/gone-gate.md");
    fs::create_dir_all(stale.parent().expect("parent")).expect("stale dir");
    fs::write(&stale, "stale").expect("stale");
    let (code, out) = skill(&to);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out.trim(),
        "wrote gatekeeper/SKILL.md, gatekeeper/packs/core.md, gatekeeper/packs/rust.md, gatekeeper/packs/swift.md, gatekeeper/packs/ts.md, gatekeeper/packs/supabase.md, gatekeeper/examples/ (23)"
    );
    assert!(!stale.exists());
    let skill_md = fs::read_to_string(to.join("gatekeeper/SKILL.md")).expect("SKILL.md");
    assert!(skill_md.contains(&format!("# gatekeeper {}", env!("CARGO_PKG_VERSION"))));
    let core_md = fs::read_to_string(to.join("gatekeeper/packs/core.md")).expect("core.md");
    assert!(
        core_md.contains("- rerun: `./.gatekeeper/run check --gate core/arch`"),
        "{core_md}"
    );
    assert!(
        core_md.contains("1. Crypto, TLS, signatures and parsers of files from outside are never written in-house")
            && core_md.contains("5. To add a dependency: ask the owner first"),
        "{core_md}"
    );
    let checked_in = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skill/gatekeeper");
    assert!(
        tree(&to.join("gatekeeper")) == tree(&checked_in),
        "skill/gatekeeper/ is stale: run `just gen`"
    );
}

#[test]
fn skill_without_to_is_a_usage_error() {
    let out = Command::new(env!("CARGO_BIN_EXE_gatekeeper"))
        .arg("skill")
        .output()
        .expect("gatekeeper runs");
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{err}");
    assert!(err.contains("skill needs --to <dir>"), "{err}");
}
