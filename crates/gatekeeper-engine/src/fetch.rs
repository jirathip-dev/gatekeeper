use crate::{Error, Result, packs, runner};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

/// `packs/<pack>/tools/<tool>.toml`: one pinned version, an executable per target or a set of npm tarballs.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ToolFile {
    Binary(BinaryTool),
    Node(NodeTool),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryTool {
    pub version: String,
    pub asset: BTreeMap<String, Asset>,
}

/// npm tarballs laid out as one flat `node_modules`; `bin` is the script under it, run by its `node` shebang.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeTool {
    pub version: String,
    pub bin: String,
    pub package: Vec<Package>,
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

/// One npm tarball, unpacked to `node_modules/<name>`; with a `target`, only on that target.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub target: Option<String>,
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

fn parsed(name: &str) -> Result<(ToolFile, &'static str)> {
    let (path, text) =
        packs::tool_file(name).ok_or_else(|| Error(format!("no pinned tool `{name}`")))?;
    let file: ToolFile = toml::from_str(text).map_err(|e| Error(format!("{path}: {e}")))?;
    let target = host_target().ok_or_else(|| Error("no pinned tools for this platform".into()))?;
    Ok((file, target))
}

/// The sha256 a tool is pinned by on this target: what a result's key names it by.
pub(crate) fn pin(name: &str) -> Result<String> {
    let (file, target) = parsed(name)?;
    Ok(match file {
        ToolFile::Binary(f) => f
            .asset
            .get(target)
            .map_or_else(String::new, |a| a.sha256.clone()),
        ToolFile::Node(f) => f
            .package
            .iter()
            .filter(|p| p.target.as_deref().is_none_or(|t| t == target))
            .map(|p| p.sha256.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    })
}

pub(crate) fn tool(name: &str, cache: &Path) -> Result<PathBuf> {
    let (file, target) = parsed(name)?;
    match file {
        ToolFile::Binary(file) => {
            let asset = file
                .asset
                .get(target)
                .ok_or_else(|| Error(format!("{name} {}: no asset for {target}", file.version)))?;
            asset_path(asset, cache)
        }
        ToolFile::Node(file) => node_path(&file, target, cache),
    }
}

fn download(url: &str, sha256: &str, to: &Path, cache: &Path) -> Result<()> {
    runner::ok("curl", &["-sSfL", "-o", &to.to_string_lossy(), url], cache)?;
    let got = sha256_hex(&fs::read(to).map_err(|e| Error::at(to, e))?);
    if got != sha256 {
        fs::remove_file(to).map_err(|e| Error::at(to, e))?;
        return Err(Error(format!(
            "{url}: sha256 {got}, pinned {sha256}: refused"
        )));
    }
    Ok(())
}

fn executable(bin: &Path) -> Result<()> {
    fs::set_permissions(bin, fs::Permissions::from_mode(0o755)).map_err(|e| Error::at(bin, e))
}

/// # Errors
pub fn node_path(tool: &NodeTool, target: &str, cache: &Path) -> Result<PathBuf> {
    let wanted: Vec<&Package> = tool
        .package
        .iter()
        .filter(|p| p.target.as_deref().is_none_or(|t| t == target))
        .collect();
    let pins: Vec<&str> = wanted.iter().map(|p| p.sha256.as_str()).collect();
    let dir = cache
        .join("tools")
        .join(sha256_hex(pins.join("\n").as_bytes()));
    let bin = dir.join("node_modules").join(&tool.bin);
    if bin.is_file() {
        return Ok(bin);
    }
    let part = dir.with_extension("part");
    for stale in [&part, &dir] {
        if stale.exists() {
            fs::remove_dir_all(stale).map_err(|e| Error::at(stale, e))?;
        }
    }
    let tgz = part.join("package.tgz");
    for package in wanted {
        let into = part.join("node_modules").join(&package.name);
        fs::create_dir_all(&into).map_err(|e| Error::at(&into, e))?;
        download(&package.url, &package.sha256, &tgz, cache)?;
        let (tgz_s, into_s) = (tgz.to_string_lossy(), into.to_string_lossy());
        runner::ok(
            "tar",
            &["-xzf", &tgz_s, "-C", &into_s, "--strip-components=1"],
            cache,
        )?;
        fs::remove_file(&tgz).map_err(|e| Error::at(&tgz, e))?;
    }
    executable(&part.join("node_modules").join(&tool.bin))?;
    fs::rename(&part, &dir).map_err(|e| Error::at(&dir, e))?;
    Ok(bin)
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
    download(&asset.url, &asset.sha256, &part, cache)?;
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
    executable(&bin)?;
    Ok(bin)
}
