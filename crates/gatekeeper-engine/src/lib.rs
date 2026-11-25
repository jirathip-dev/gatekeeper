pub(crate) mod arch;
pub mod baseline;
pub mod cache;
pub(crate) mod check;
pub mod ci;
pub mod config;
pub(crate) mod docs;
pub mod fetch;
pub mod graph;
pub mod health;
pub(crate) mod kept;
pub mod packs;
pub mod pin;
pub mod plan;
pub mod ratchet;
pub mod run;
pub(crate) mod runner;
pub mod scope;
pub mod select;
pub mod selftest;
pub mod skill;

use std::fmt;
use std::path::Path;

/// A config, pack, tool or process error, with what it concerns.
#[derive(Debug)]
pub struct Error(pub String);

impl Error {
    pub fn at(path: &Path, e: impl fmt::Display) -> Error {
        Error(format!("{}: {e}", path.display()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The commit this binary was built from, stamped by the release build as `GATEKEEPER_BUILD`.
pub const BUILD: Option<&str> = option_env!("GATEKEEPER_BUILD");

/// A hash of the sources and packs this binary was built from: a checkout's identity in a result's key.
pub const SOURCE: &str = env!("GATEKEEPER_SOURCE");
