use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn gk(name: &str, args: &[&str]) -> (i32, String) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    fs::create_dir_all(root.join(".gatekeeper")).expect("dir");
    fs::write(
        root.join(".gatekeeper/config.toml"),
        "packs = []\nintegration = \"staging\"\npin = \"checkout\"\n",
    )
    .expect("config");
    let out = Command::new(env!("CARGO_BIN_EXE_gatekeeper"))
        .arg("supabase")
        .args(args)
        .current_dir(&root)
        .env_remove("SUPABASE_ACCESS_TOKEN")
        .output()
        .expect("gatekeeper runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().expect("exit code"), text)
}

#[test]
fn apply_refuses_a_target_other_than_dev_or_prod() {
    let (code, out) = gk("supabase-bad-target", &["apply", "--target", "staging"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("apply needs --target dev|prod"), "{out}");
}

#[test]
fn status_runs_the_pack_script_which_needs_a_token() {
    let (code, out) = gk("supabase-status", &["status"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("supabase status: set SUPABASE_ACCESS_TOKEN and SUPABASE_PROJECT_REF"),
        "{out}"
    );
}
