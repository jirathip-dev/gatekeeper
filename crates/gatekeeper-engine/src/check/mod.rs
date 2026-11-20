pub mod banned_apis;
pub mod deny_floor;
pub mod deps;
pub mod lint_floor;

use crate::run::CheckCx;
use crate::{Error, Result};

pub const NAMES: &[&str] = &["banned-apis", "deny-floor", "deps", "lint-floor"];

pub fn run(name: &str, cx: &CheckCx) -> Result<Vec<String>> {
    match name {
        "banned-apis" => banned_apis::run(cx),
        "deny-floor" => deny_floor::run(cx),
        "deps" => deps::run(cx),
        "lint-floor" => lint_floor::run(cx),
        _ => Err(Error(format!("no built-in check `{name}`"))),
    }
}
