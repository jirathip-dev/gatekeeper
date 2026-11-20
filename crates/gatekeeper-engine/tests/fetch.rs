use gatekeeper_engine::fetch::{Asset, asset_path, sha256_hex};
use std::fs;
use std::path::PathBuf;

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
