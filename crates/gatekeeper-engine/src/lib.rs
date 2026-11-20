pub mod baseline;
pub(crate) mod check;
pub mod ci;
pub mod config;
pub mod fetch;
pub mod graph;
pub(crate) mod kept;
pub mod packs;
pub mod plan;
pub mod run;
pub(crate) mod runner;
pub mod scope;
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
