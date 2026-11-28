use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;

const SHA: &str = "8d2e41c0aa11bb22cc33dd44ee55ff6677889900";
const OLD: &str = "0123456789abcdef0123456789abcdef01234567";
const SUM_LINUX: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SUM_MAC: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const CURL: &str = "#!/bin/sh\necho \"$4\" >> \"$FAKE_RELEASE/fetched\"\ncp \"$FAKE_RELEASE/$(basename \"$4\")\" \"$3\"\n";
const FARRIER: &str = "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$FAKE_RELEASE/farrier.args\"\n\
                       for a in \"$@\"; do [ -f \"$a\" ] && cat \"$a\" >> \"$FAKE_RELEASE/farrier.keys\"; done\n\
                       [ -f \"$FAKE_RELEASE/refuse\" ] && { echo 'Signature: no key verifies SHA256SUMS' >&2; exit 1; }\n\
                       dir=$3; while [ \"$1\" != --bin ]; do shift; done\n\
                       for asset in \"$dir/$2\"-*; do \"$asset\" --version || exit 1; done\n";
const CI: &str = "jobs:\n  gates:\n    uses: jirathip-dev/gatekeeper/.github/workflows/gates.yml@0123456789abcdef0123456789abcdef01234567 # v1.0.0\n    secrets:\n      private-names: ${{ secrets.NAMES }}\n  other:\n    steps:\n      - uses: actions/checkout@v4\n";

fn script(path: &Path, text: &str) {
    fs::write(path, text).expect("script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
}

fn wrapper() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/core/files/run"))
        .expect("wrapper")
}

fn setup(name: &str, comment: &str) -> (PathBuf, PathBuf) {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if base.exists() {
        fs::remove_dir_all(&base).expect("clean");
    }
    let (root, release, bin) = (base.join("repo"), base.join("release"), base.join("bin"));
    for dir in [
        &root.join(".gatekeeper"),
        &root.join(".github/workflows"),
        &release,
        &bin,
    ] {
        fs::create_dir_all(dir).expect("dir");
    }
    fs::write(
        root.join(".gatekeeper/config.toml"),
        format!("packs = [\"core\"]\nintegration = \"staging\"\n\n[tools]\nfarrier = \"../bin/farrier\"\n\n[pin]\nversion = \"1.0.0\"\nx86_64-unknown-linux-gnu = \"{OLD}{OLD}\"\n"),
    )
    .expect("config");
    fs::write(
        root.join(".gatekeeper/run"),
        "#!/bin/sh\necho old wrapper\n",
    )
    .expect("run");
    fs::write(root.join(".github/workflows/ci.yml"), CI).expect("ci");
    fs::write(
        release.join("SHA256SUMS"),
        format!("{SUM_MAC}  gatekeeper-aarch64-apple-darwin\n{SUM_LINUX}  gatekeeper-x86_64-unknown-linux-gnu\n"),
    )
    .expect("sums");
    fs::write(
        release.join("SHA256SUMS.minisig"),
        format!("untrusted comment: signature\nRUQsig\ntrusted comment: {comment}\nglobal\n"),
    )
    .expect("sig");
    for asset in [
        "gatekeeper-aarch64-apple-darwin",
        "gatekeeper-x86_64-unknown-linux-gnu",
    ] {
        fs::write(release.join(asset), "#!/bin/sh\necho gatekeeper 1.2.3\n").expect("asset");
    }
    fs::write(release.join("run"), wrapper()).expect("wrapper");
    script(&bin.join("curl"), CURL);
    script(&bin.join("farrier"), FARRIER);
    (root, release)
}

fn pin(root: &Path, release: &Path) -> (i32, String) {
    gatekeeper(root, release, "1.2.3")
}

fn gatekeeper(root: &Path, release: &Path, arg: &str) -> (i32, String) {
    let bin = release.with_file_name("bin");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new(env!("CARGO_BIN_EXE_gatekeeper"))
        .args(["pin", arg])
        .current_dir(root)
        .env("PATH", path)
        .env("FAKE_RELEASE", release)
        .env("XDG_CACHE_HOME", release.with_file_name("cache"))
        .output()
        .expect("pin runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().expect("code"), text)
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).expect("read")
}

#[test]
fn pin_writes_the_pin_the_uses_lines_and_the_wrapper_of_a_verified_release() {
    let (root, release) = setup("pin-ok", &format!("jirathip-dev/gatekeeper v1.2.3 {SHA}"));
    let (code, out) = pin(&root, &release);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(&format!("release v1.2.3: signature ok, commit {SHA}")),
        "{out}"
    );
    assert_eq!(
        read(&root, ".gatekeeper/config.toml"),
        format!(
            "packs = [\"core\"]\nintegration = \"staging\"\n\n[tools]\nfarrier = \"../bin/farrier\"\n\n[pin]\nversion = \"1.2.3\"\naarch64-apple-darwin = \"{SUM_MAC}\"\nx86_64-unknown-linux-gnu = \"{SUM_LINUX}\"\n"
        )
    );
    assert_eq!(
        read(&root, ".github/workflows/ci.yml"),
        CI.replace(&format!("@{OLD} # v1.0.0"), &format!("@{SHA} # v1.2.3"))
    );
    assert_eq!(read(&root, ".gatekeeper/run"), wrapper());
    let args = fs::read_to_string(release.join("farrier.args")).expect("farrier ran");
    for want in [
        "release\nverify\n",
        "--repo\njirathip-dev/gatekeeper\n",
        "--tag\nv1.2.3\n",
        &format!("--sha\n{SHA}\n"),
        "--bin\ngatekeeper\n",
    ] {
        assert!(args.contains(want), "{args}");
    }
    let keys = fs::read_to_string(release.join("farrier.keys")).expect("keys");
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let compiled = [repo.join("release.pub"), repo.join("release-backup.pub")]
        .map(|p| fs::read_to_string(p).expect("key"))
        .concat();
    assert_eq!(keys, compiled);

    let (code, again) = pin(&root, &release);
    assert_eq!(code, 0, "{again}");
    assert_eq!(again.matches(": unchanged").count(), 3, "{again}");
}

#[test]
fn pin_writes_nothing_when_farrier_refuses_the_release() {
    let (root, release) = setup(
        "pin-refused",
        &format!("jirathip-dev/gatekeeper v1.2.3 {SHA}"),
    );
    fs::write(release.join("refuse"), "").expect("refuse");
    let (code, out) = pin(&root, &release);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("v1.2.3: refused") && out.contains("Signature"),
        "{out}"
    );
    assert_eq!(read(&root, ".github/workflows/ci.yml"), CI);
    assert!(read(&root, ".gatekeeper/config.toml").contains("version = \"1.0.0\""));
}

#[test]
fn pin_refuses_a_signature_whose_trusted_comment_names_another_tag() {
    let (root, release) = setup(
        "pin-other-tag",
        &format!("jirathip-dev/gatekeeper v1.2.2 {SHA}"),
    );
    let (code, out) = pin(&root, &release);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("trusted comment is `jirathip-dev/gatekeeper v1.2.2"),
        "{out}"
    );
    assert!(!release.join("farrier.args").exists());
    assert_eq!(read(&root, ".github/workflows/ci.yml"), CI);
}

#[test]
fn pin_fetches_farrier_from_its_pinned_asset_and_refuses_one_whose_sha256_differs() {
    let (root, release) = setup(
        "pin-farrier-asset",
        &format!("jirathip-dev/gatekeeper v1.2.3 {SHA}"),
    );
    let config = read(&root, ".gatekeeper/config.toml")
        .replace("[tools]\nfarrier = \"../bin/farrier\"\n\n", "");
    fs::write(root.join(".gatekeeper/config.toml"), &config).expect("config");
    let target = gatekeeper_engine::fetch::host_target().expect("a pinned host target");
    let recipe = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/core/tools/farrier.toml"),
    )
    .expect("farrier.toml");
    let (_, pinned) = recipe
        .split_once(&format!("[asset.{target}]\nurl = \""))
        .expect("an asset for the host target");
    let url = &pinned[..pinned.find('"').expect("url ends")];
    let name = url.rsplit('/').next().expect("asset name");
    fs::write(release.join(name), "not the pinned farrier").expect("stand-in");
    let (code, out) = pin(&root, &release);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains(&format!("gatekeeper pin: farrier: {url}: sha256 "))
            && out.contains("refused"),
        "{out}"
    );
    assert_eq!(read(&release, "fetched"), format!("{url}\n"));
    assert!(!release.join("farrier.args").exists());
    assert_eq!(read(&root, ".gatekeeper/config.toml"), config);
    assert_eq!(read(&root, ".github/workflows/ci.yml"), CI);
}

const RELEASES: &str = r#"[
  {"tag_name": "v9.0.0", "draft": false, "prerelease": true, "assets": [{"name": "SHA256SUMS.minisig"}]},
  {"tag_name": "v8.0.0", "draft": true, "prerelease": false, "assets": [{"name": "SHA256SUMS.minisig"}]},
  {"tag_name": "v7.0.0", "draft": false, "prerelease": false, "assets": [{"name": "SHA256SUMS"}]},
  {"tag_name": "v1.1.0", "draft": false, "prerelease": false, "assets": [{"name": "SHA256SUMS.minisig"}]},
  {"tag_name": "v1.3.0", "draft": false, "prerelease": false, "assets": [{"name": "SHA256SUMS.minisig"}]},
  {"tag_name": "v1.2.3", "draft": false, "prerelease": false, "assets": [{"name": "SHA256SUMS.minisig"}]}
]"#;

fn with_releases(name: &str) -> (PathBuf, PathBuf) {
    let (root, release) = setup(name, &format!("jirathip-dev/gatekeeper v1.2.3 {SHA}"));
    fs::write(release.join("releases"), RELEASES).expect("releases");
    (root, release)
}

#[test]
fn outdated_exits_1_with_one_line_when_a_newer_signed_release_verifies() {
    let (root, release) = with_releases("pin-outdated-behind");
    let (code, out) = gatekeeper(&root, &release, "--outdated");
    assert_eq!(code, 1, "{out}");
    assert_eq!(out, "pinned 1.0.0, latest 1.2.3\n");
    let fetched = read(&release, "fetched");
    assert!(fetched.contains("/v1.3.0/SHA256SUMS.minisig"), "{fetched}");
    for skipped in ["v9.0.0", "v8.0.0", "v7.0.0", "v1.1.0"] {
        assert!(!fetched.contains(skipped), "{fetched}");
    }
}

#[test]
fn outdated_exits_0_when_the_pin_is_the_latest_verified_release() {
    let (root, release) = with_releases("pin-outdated-current");
    let config = read(&root, ".gatekeeper/config.toml").replace("1.0.0", "1.2.3");
    fs::write(root.join(".gatekeeper/config.toml"), config).expect("config");
    let (code, out) = gatekeeper(&root, &release, "--outdated");
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, "");
}

#[test]
fn outdated_exits_2_when_the_release_list_cannot_be_fetched() {
    let (root, release) = setup(
        "pin-outdated-offline",
        &format!("jirathip-dev/gatekeeper v1.2.3 {SHA}"),
    );
    let (code, out) = gatekeeper(&root, &release, "--outdated");
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("pin --outdated: curl"), "{out}");
}

#[test]
fn pin_latest_rewrites_the_same_files_as_pin_of_that_version() {
    let (by_version, release) = with_releases("pin-version");
    assert_eq!(pin(&by_version, &release).0, 0);
    let (latest, release) = with_releases("pin-latest");
    let (code, out) = gatekeeper(&latest, &release, "latest");
    assert_eq!(code, 0, "{out}");
    for rel in [
        ".gatekeeper/config.toml",
        ".github/workflows/ci.yml",
        ".gatekeeper/run",
    ] {
        assert_eq!(read(&latest, rel), read(&by_version, rel), "{rel}");
    }
}
