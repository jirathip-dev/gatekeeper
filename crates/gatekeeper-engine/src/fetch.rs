use crate::{Error, Result, packs, runner};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

/// `packs/<pack>/tools/<tool>.toml`: one pinned version, an asset per target.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolFile {
    pub version: String,
    pub asset: BTreeMap<String, Asset>,
}

/// A download checked against its sha256; `bin` is the executable's path inside a `.tar.gz`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub bin: Option<String>,
}

#[must_use]
pub fn host_target() -> Option<&'static str> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "linux") => Some("x86_64-unknown-linux-gnu"),
        ("aarch64", "macos") => Some("aarch64-apple-darwin"),
        _ => None,
    }
}

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .flat_map(|b| [b >> 4, b & 15])
        .filter_map(|n| char::from_digit(u32::from(n), 16))
        .collect()
}

pub(crate) fn tool(name: &str, cache: &Path) -> Result<PathBuf> {
    let (path, text) =
        packs::tool_file(name).ok_or_else(|| Error(format!("no pinned tool `{name}`")))?;
    let file: ToolFile = toml::from_str(text).map_err(|e| Error(format!("{path}: {e}")))?;
    let target = host_target().ok_or_else(|| Error("no pinned tools for this platform".into()))?;
    let asset = file
        .asset
        .get(target)
        .ok_or_else(|| Error(format!("{name} {}: no asset for {target}", file.version)))?;
    asset_path(asset, cache)
}

/// # Errors
pub fn asset_path(asset: &Asset, cache: &Path) -> Result<PathBuf> {
    let dir = cache.join("tools").join(&asset.sha256);
    let bin = dir.join(asset.bin.as_deref().unwrap_or("tool"));
    if bin.is_file() {
        return Ok(bin);
    }
    fs::create_dir_all(&dir).map_err(|e| Error::at(&dir, e))?;
    let part = dir.with_extension("part");
    let part_s = part.to_string_lossy();
    runner::ok("curl", &["-sSfL", "-o", &part_s, &asset.url], cache)?;
    let got = sha256_hex(&fs::read(&part).map_err(|e| Error::at(&part, e))?);
    if got != asset.sha256 {
        fs::remove_file(&part).map_err(|e| Error::at(&part, e))?;
        return Err(Error(format!(
            "{}: sha256 {got}, pinned {}: refused",
            asset.url, asset.sha256
        )));
    }
    if asset.bin.is_some() {
        runner::ok(
            "tar",
            &["-xzf", &part_s, "-C", &dir.to_string_lossy()],
            cache,
        )?;
        fs::remove_file(&part).map_err(|e| Error::at(&part, e))?;
    } else {
        fs::rename(&part, &bin).map_err(|e| Error::at(&bin, e))?;
    }
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).map_err(|e| Error::at(&bin, e))?;
    Ok(bin)
}
