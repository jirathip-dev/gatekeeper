pub mod banned_apis;
pub mod deny_floor;
pub mod lint_floor;
pub mod pin;

use crate::run::CheckCx;
use crate::{Error, Result};

pub const NAMES: &[&str] = &["banned-apis", "deny-floor", "lint-floor", "pin"];

pub fn run(name: &str, cx: &CheckCx) -> Result<Vec<String>> {
    match name {
        "banned-apis" => banned_apis::run(cx),
        "deny-floor" => deny_floor::run(cx),
        "lint-floor" => lint_floor::run(cx),
        "pin" => pin::run(cx),
        _ => Err(Error(format!("no built-in check `{name}`"))),
    }
}
