use crate::{Error, Result, VERSION};
use gatekeeper_model::{Config, PackId, Pin};
use std::path::Path;

pub const PATH: &str = ".gatekeeper/config.toml";

/// # Errors
pub fn load(root: &Path) -> Result<Config> {
    let path = root.join(PATH);
    let text = std::fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
    let config: Config = toml::from_str(&text).map_err(|e| Error::at(&path, e))?;
    if let Pin::Release(pin) = &config.pin {
        if pin.version != VERSION {
            return Err(Error(format!(
                "{PATH}: [pin] names {}, this gatekeeper is {VERSION}",
                pin.version
            )));
        }
        let bad = |s: &String| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit());
        if let Some((target, _)) = pin.sha256.iter().find(|(_, s)| bad(s)) {
            return Err(Error(format!("{PATH}: [pin] {target} is not a sha256")));
        }
    }
    if config.packs.contains(&PackId::Ts) != config.ts.is_some() {
        return Err(Error(format!(
            "{PATH}: the ts pack and its [ts] table (projects, install) go together"
        )));
    }
    Ok(config)
}

#[must_use]
pub fn manifest(config: &Config) -> &str {
    config
        .rust
        .as_ref()
        .map_or("Cargo.toml", |r| r.manifest.as_str())
}

pub(crate) fn read_toml(path: &Path) -> Result<toml::Table> {
    let text = std::fs::read_to_string(path).map_err(|e| Error::at(path, e))?;
    toml::from_str(&text).map_err(|e| Error::at(path, e))
}
