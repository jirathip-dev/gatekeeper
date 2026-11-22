use gatekeeper_engine::fetch::{
    Asset, NodeTool, Package, ToolFile, asset_path, node_path, sha256_hex,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clean scratch");
    }
    fs::create_dir_all(&dir).expect("scratch");
    dir
}

#[test]
fn a_download_whose_sha256_differs_from_the_pin_is_refused_and_not_cached() {
    let dir = scratch("fetch-wrong");
    let file = dir.join("tool");
    fs::write(&file, b"#!/bin/sh\necho tool\n").expect("tool");
    let asset = Asset {
        url: format!("file://{}", file.display()),
        sha256: "0".repeat(64),
        bin: None,
    };
    let cache = dir.join("cache");
    let err = asset_path(&asset, &cache).expect_err("a wrong checksum must be refused");
    assert!(err.0.contains("refused"), "{err}");
    assert!(
        !cache
            .join("tools")
            .join("0".repeat(64))
            .join("tool")
            .exists()
    );
}

#[test]
fn a_download_with_the_pinned_sha256_is_cached_and_runnable() {
    let dir = scratch("fetch-right");
    let file = dir.join("tool");
    let bytes = b"#!/bin/sh\necho tool\n";
    fs::write(&file, bytes).expect("tool");
    let asset = Asset {
        url: format!("file://{}", file.display()),
        sha256: sha256_hex(bytes),
        bin: None,
    };
    let cache = dir.join("cache");
    let got = asset_path(&asset, &cache).expect("fetched");
    assert_eq!(fs::read(&got).expect("cached"), bytes);
    fs::remove_file(&file).expect("drop the source");
    assert_eq!(
        asset_path(&asset, &cache).expect("served from the cache"),
        got
    );
}

fn tarball(dir: &Path, name: &str, script: &str) -> (String, String) {
    let src = dir.join(name);
    fs::create_dir_all(src.join("package/bin")).expect("package");
    fs::write(src.join("package/bin/run.js"), script).expect("script");
    let tgz = dir.join(format!("{name}.tgz"));
    let made = Command::new("tar")
        .args([
            "-czf",
            &tgz.to_string_lossy(),
            "-C",
            &src.to_string_lossy(),
            "package",
        ])
        .status()
        .expect("tar runs");
    assert!(made.success());
    let bytes = fs::read(&tgz).expect("tgz");
    (format!("file://{}", tgz.display()), sha256_hex(&bytes))
}

fn node_tool(packages: Vec<Package>) -> NodeTool {
    NodeTool {
        version: "1.0.0".into(),
        bin: "tool/bin/run.js".into(),
        package: packages,
    }
}

#[test]
fn npm_tarballs_with_their_pinned_sha256_make_one_node_modules_and_other_targets_are_skipped() {
    let dir = scratch("fetch-node");
    let (url, sha256) = tarball(&dir, "tool", "#!/bin/sh\necho tool\n");
    let (other, other_sha) = tarball(&dir, "other", "elsewhere\n");
    let tool = node_tool(vec![
        Package {
            name: "tool".into(),
            url,
            sha256,
            target: None,
        },
        Package {
            name: "@scope/binding".into(),
            url: other,
            sha256: other_sha,
            target: Some("another-target".into()),
        },
    ]);
    let cache = dir.join("cache");
    let bin = node_path(&tool, "this-target", &cache).expect("fetched");
    let out = Command::new(&bin).output().expect("the bin runs");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "tool\n");
    let modules = bin.ancestors().nth(3).expect("node_modules");
    assert!(!modules.join("@scope").exists());
    assert_eq!(
        node_path(&tool, "this-target", &cache).expect("cached"),
        bin
    );
}

#[test]
fn an_npm_tarball_whose_sha256_differs_is_refused_and_nothing_is_installed() {
    let dir = scratch("fetch-node-wrong");
    let (url, _) = tarball(&dir, "tool", "#!/bin/sh\necho tool\n");
    let tool = node_tool(vec![Package {
        name: "tool".into(),
        url,
        sha256: "0".repeat(64),
        target: None,
    }]);
    let cache = dir.join("cache");
    let err =
        node_path(&tool, "this-target", &cache).expect_err("a wrong checksum must be refused");
    assert!(err.0.contains("refused"), "{err}");
    let installed = fs::read_dir(cache.join("tools"))
        .expect("tools dir")
        .filter_map(Result::ok)
        .any(|e| e.path().join("node_modules/tool/bin/run.js").exists());
    assert!(!installed);
}

fn built_archive(dir: &Path) -> (String, Vec<u8>) {
    let out = dir.join("out");
    fs::create_dir_all(out.join("bin")).expect("out");
    fs::write(out.join("bin/sqlx"), "#!/bin/sh\necho built\n").expect("bin");
    let archive = dir.join("sqlx-cli-0.8.6-target.tar.gz");
    let made = Command::new("tar")
        .args([
            "-czf",
            &archive.to_string_lossy(),
            "-C",
            &out.to_string_lossy(),
            ".",
        ])
        .status()
        .expect("tar runs");
    assert!(made.success());
    (
        format!("file://{}", archive.display()),
        fs::read(&archive).expect("archive"),
    )
}

#[test]
fn a_built_tools_archive_is_fetched_by_its_pin_and_refused_with_a_wrong_checksum() {
    let dir = scratch("fetch-built");
    let (url, bytes) = built_archive(&dir);
    let cache = dir.join("cache");
    let wrong = Asset {
        url: url.clone(),
        sha256: "f".repeat(64),
        bin: Some("bin/sqlx".into()),
    };
    let err = asset_path(&wrong, &cache).expect_err("a wrong checksum must be refused");
    assert!(err.0.contains("refused"), "{err}");
    assert!(
        !cache
            .join("tools")
            .join("f".repeat(64))
            .join("bin/sqlx")
            .exists()
    );
    let right = Asset {
        url,
        sha256: sha256_hex(&bytes),
        bin: Some("bin/sqlx".into()),
    };
    let bin = asset_path(&right, &cache).expect("fetched");
    let out = Command::new(&bin).output().expect("the bin runs");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "built\n");
}

// TODO(#41): farrier's [asset.*] pins land in #41, which drops this entry.
const UNPINNED_EXEMPT: &[&str] = &["packs/core/tools/farrier.toml"];

fn unpinned_build_targets(packs: &Path) -> Vec<String> {
    let mut unpinned = Vec::new();
    for pack in fs::read_dir(packs).expect("packs") {
        let Ok(tools) = fs::read_dir(pack.expect("pack").path().join("tools")) else {
            continue;
        };
        for file in tools {
            let path = file.expect("tool").path();
            if UNPINNED_EXEMPT.iter().any(|e| path.ends_with(e)) {
                continue;
            }
            let text = fs::read_to_string(&path).expect("tool file");
            let ToolFile::Binary(tool) = toml::from_str(&text).expect("tool file parses") else {
                continue;
            };
            let Some(build) = tool.build else { continue };
            unpinned.extend(
                build
                    .targets
                    .iter()
                    .filter(|t| !tool.asset.contains_key(*t))
                    .map(|t| format!("{}: {t}", path.display())),
            );
        }
    }
    unpinned
}

#[test]
fn every_pack_tool_with_a_build_recipe_pins_an_asset_for_each_build_target() {
    let packs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    let unpinned = unpinned_build_targets(&packs);
    assert!(unpinned.is_empty(), "no [asset.*] pin for: {unpinned:#?}");
}

#[test]
fn only_the_named_exemption_skips_the_build_target_pin_check() {
    let packs = scratch("fetch-unpinned").join("packs");
    let tools = packs.join("core/tools");
    fs::create_dir_all(&tools).expect("tools");
    let recipe = "version = \"1\"\n\n[build]\ntargets = [\"aarch64-apple-darwin\"]\nbudget_minutes = 1\nbin = \"bin/x\"\nscript = \"true\"\n";
    fs::write(tools.join("farrier.toml"), recipe).expect("farrier");
    fs::write(tools.join("other.toml"), recipe).expect("other");
    assert_eq!(
        unpinned_build_targets(&packs),
        [format!(
            "{}: aarch64-apple-darwin",
            tools.join("other.toml").display()
        )]
    );
}
