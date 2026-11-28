use crate::{Error, Result, fetch, runner};
use gatekeeper_model::{Config, Pin};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

pub const REPO: &str = "jirathip-dev/gatekeeper";
const KEYS: [(&str, &str); 2] = [
    ("release.pub", include_str!("../../../release.pub")),
    (
        "release-backup.pub",
        include_str!("../../../release-backup.pub"),
    ),
];
const CONFIG: &str = ".gatekeeper/config.toml";
const WRAPPER: &str = ".gatekeeper/run";

pub(crate) fn call(line: &str) -> Option<(&str, &str, &str)> {
    let (key, rest) = line.split_once("uses:")?;
    let (target, comment) = rest.split_once('#').unwrap_or((rest, ""));
    let (path, git_ref) = target.trim().split_once('@')?;
    if path.starts_with("./") || !path.contains("/gatekeeper/.github/workflows/") {
        return None;
    }
    let head = line.get(..key.len() + "uses:".len() + rest.find(path)? + path.len())?;
    Some((head, git_ref, comment.trim()))
}

pub(crate) fn is_commit(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

pub(crate) fn workflow_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(root.join(".github/workflows"))
        .into_iter()
        .flatten()
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    files
}

fn fetch(url: &str, to: &Path) -> Result<()> {
    let dest = to.to_string_lossy();
    runner::ok("curl", &["-sSfL", "-o", &dest, url], Path::new(".")).map(drop)
}

fn signed_commit(dir: &Path, tag: &str) -> Result<String> {
    let sig = dir.join("SHA256SUMS.minisig");
    let text = fs::read_to_string(&sig).map_err(|e| Error::at(&sig, e))?;
    let comment = text
        .lines()
        .find_map(|l| l.strip_prefix("trusted comment: "))
        .unwrap_or_default();
    match comment.split_whitespace().collect::<Vec<_>>().as_slice() {
        [repo, t, sha] if *repo == REPO && *t == tag && is_commit(sha) => Ok((*sha).to_owned()),
        _ => Err(Error(format!(
            "{tag}: the signature's trusted comment is `{comment}`, not `{REPO} {tag} <commit>`: refused"
        ))),
    }
}

type Verified = (String, BTreeMap<String, String>);

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
}

fn semver(version: &str) -> Option<[u32; 3]> {
    let mut parts = version.split('.').map(|p| p.parse().ok());
    let v = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(v)
}

fn verified(
    version: &str,
    scratch: &Path,
    farrier: &Path,
) -> Result<std::result::Result<Verified, Error>> {
    let tag = format!("v{version}");
    let dir = scratch.join(&tag);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| Error::at(&dir, e))?;
    }
    fs::create_dir_all(&dir).map_err(|e| Error::at(&dir, e))?;
    let base = format!("https://github.com/{REPO}/releases/download/{tag}");
    for name in ["SHA256SUMS", "SHA256SUMS.minisig"] {
        fetch(&format!("{base}/{name}"), &dir.join(name))?;
    }
    let list = fs::read_to_string(dir.join("SHA256SUMS")).map_err(|e| Error::at(&dir, e))?;
    let mut sums = BTreeMap::new();
    for line in list.lines() {
        let (sum, asset) = line.split_once("  ").unwrap_or((line, ""));
        let Some(target) = asset.strip_prefix("gatekeeper-") else {
            return Ok(Err(Error(format!(
                "{tag}: SHA256SUMS line `{line}` names no gatekeeper asset"
            ))));
        };
        let path = dir.join(asset);
        fetch(&format!("{base}/{asset}"), &path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .map_err(|e| Error::at(&path, e))?;
        sums.insert(target.to_owned(), sum.to_owned());
    }
    let sha = match signed_commit(&dir, &tag) {
        Ok(sha) => sha,
        Err(e) => return Ok(Err(e)),
    };
    let mut keys = Vec::new();
    for (name, key) in KEYS {
        let path = scratch.join(name);
        fs::write(&path, key).map_err(|e| Error::at(&path, e))?;
        keys.push(path.to_string_lossy().into_owned());
    }
    let dir_s = dir.to_string_lossy();
    let mut args = vec!["release", "verify", &dir_s, "--repo", REPO, "--tag", &tag];
    args.extend(["--sha", &sha, "--bin", "gatekeeper"]);
    for key in &keys {
        args.extend(["--key", key]);
    }
    Ok(runner::ok(&farrier.to_string_lossy(), &args, scratch)
        .map(|_| (sha, sums))
        .map_err(|e| Error(format!("{tag}: refused: {e}"))))
}

fn latest(scratch: &Path, farrier: &Path) -> Result<(String, Verified)> {
    let list = scratch.join("releases.json");
    fetch(
        &format!("https://api.github.com/repos/{REPO}/releases"),
        &list,
    )?;
    let text = fs::read_to_string(&list).map_err(|e| Error::at(&list, e))?;
    let releases: Vec<Release> =
        serde_json::from_str(&text).map_err(|e| Error(format!("{REPO} releases: {e}")))?;
    let mut signed: Vec<_> = releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter(|r| r.assets.iter().any(|a| a.name == "SHA256SUMS.minisig"))
        .filter_map(|r| {
            let version = r.tag_name.strip_prefix('v')?;
            Some((semver(version)?, version))
        })
        .collect();
    signed.sort_unstable();
    let mut refused = Vec::new();
    for (_, version) in signed.into_iter().rev() {
        match verified(version, scratch, farrier)? {
            Ok(v) => return Ok((version.to_owned(), v)),
            Err(e) => refused.push(e.0),
        }
    }
    Err(Error(format!(
        "no signed release verifies: {}",
        refused.join("; ")
    )))
}

fn write(root: &Path, rel: &str, new: &str, what: &str) -> Result<String> {
    let path = root.join(rel);
    if fs::read_to_string(&path).ok().as_deref() == Some(new) {
        return Ok(format!("{rel}: unchanged"));
    }
    fs::write(&path, new).map_err(|e| Error::at(&path, e))?;
    Ok(format!("{rel}: {what}"))
}

fn pin_table(text: &str, version: &str, sums: &BTreeMap<String, String>) -> Result<String> {
    let mut out = String::new();
    let mut in_pin = false;
    for line in text.lines() {
        in_pin = if line.starts_with('[') {
            line.trim() == "[pin]"
        } else {
            in_pin
        };
        let top_pin = line.split('=').next().is_some_and(|k| k.trim() == "pin");
        if !in_pin && !top_pin {
            writeln!(out, "{line}").ok();
        }
    }
    out = format!("{}\n\n[pin]\nversion = \"{version}\"\n", out.trim_end());
    for (target, sum) in sums {
        writeln!(out, "{target} = \"{sum}\"").ok();
    }
    toml::from_str::<Config>(&out).map_err(|e| Error(format!("{CONFIG}: {e}")))?;
    Ok(out)
}

fn workflows(root: &Path, sha: &str, version: &str, said: &mut Vec<String>) -> Result<()> {
    for path in workflow_files(root) {
        let text = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
        let (mut calls, mut new) = (0, String::new());
        for line in text.lines() {
            let head = call(line).map(|(head, _, _)| head);
            calls += usize::from(head.is_some());
            match head {
                Some(head) => writeln!(new, "{head}@{sha} # v{version}").ok(),
                None => writeln!(new, "{line}").ok(),
            };
        }
        if calls > 0 {
            let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy();
            let what = format!("{calls} uses: lines -> @{sha} # v{version}");
            said.push(write(root, &rel, &new, &what)?);
        }
    }
    Ok(())
}

fn setup(root: &Path, scratch: &Path, cache: &Path) -> Result<(String, Config, PathBuf)> {
    fs::create_dir_all(scratch).map_err(|e| Error::at(scratch, e))?;
    let config = root.join(CONFIG);
    let text = fs::read_to_string(&config).map_err(|e| Error::at(&config, e))?;
    let own: Config = toml::from_str(&text).map_err(|e| Error(format!("{CONFIG}: {e}")))?;
    let tools = crate::config::tools(&own, root);
    let farrier =
        fetch::resolve("farrier", cache, &tools).map_err(|e| Error(format!("farrier: {e}")))?;
    Ok((text, own, farrier))
}

/// # Errors
pub fn outdated(root: &Path, scratch: &Path, cache: &Path) -> Result<Option<String>> {
    let (_, own, farrier) = setup(root, scratch, cache)?;
    let Pin::Release(pin) = own.pin else {
        return Err(Error(format!("{CONFIG}: `pin` is not a release")));
    };
    let old = semver(&pin.version).ok_or_else(|| {
        Error(format!(
            "[pin] version `{}` is not MAJOR.MINOR.PATCH",
            pin.version
        ))
    })?;
    let (new, _) = latest(scratch, &farrier)?;
    Ok((semver(&new) > Some(old)).then(|| format!("pinned {}, latest {new}", pin.version)))
}

/// # Errors
pub fn pin(root: &Path, version: &str, scratch: &Path, cache: &Path) -> Result<Vec<String>> {
    if version != "latest" && semver(version).is_none() {
        return Err(Error(format!("`{version}` is not MAJOR.MINOR.PATCH")));
    }
    let (text, _, farrier) = setup(root, scratch, cache)?;
    let (version, (sha, sums)) = if version == "latest" {
        latest(scratch, &farrier)?
    } else {
        (version.to_owned(), verified(version, scratch, &farrier)??)
    };
    let version = version.as_str();
    let wrapper = scratch.join("run");
    let raw = format!("https://raw.githubusercontent.com/{REPO}/{sha}/packs/core/files/run");
    fetch(&raw, &wrapper)?;
    let wrapper = fs::read_to_string(&wrapper).map_err(|e| Error::at(&wrapper, e))?;
    let table = pin_table(&text, version, &sums)?;
    let mut said = vec![format!("release v{version}: signature ok, commit {sha}")];
    let what = format!("v{version} ({} checksums)", sums.len());
    said.push(write(root, CONFIG, &table, &what)?);
    workflows(root, &sha, version, &mut said)?;
    said.push(write(
        root,
        WRAPPER,
        &wrapper,
        &format!("the wrapper of v{version}"),
    )?);
    let run = root.join(WRAPPER);
    fs::set_permissions(&run, fs::Permissions::from_mode(0o755)).map_err(|e| Error::at(&run, e))?;
    Ok(said)
}
