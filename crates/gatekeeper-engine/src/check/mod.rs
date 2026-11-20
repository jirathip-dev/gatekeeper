pub mod banned_apis;
pub mod deny_floor;
pub mod deps;
pub mod knip;
pub mod lint_floor;
pub mod oxlint;
pub mod pin;
pub mod tsc;

use crate::run::CheckCx;
use crate::{Error, Result};

pub const NAMES: &[&str] = &[
    "banned-apis",
    "deny-floor",
    "deps",
    "knip",
    "lint-floor",
    "oxlint",
    "pin",
    "tsc",
];

pub fn run(name: &str, cx: &CheckCx) -> Result<Vec<String>> {
    match name {
        "banned-apis" => banned_apis::run(cx),
        "deny-floor" => deny_floor::run(cx),
        "deps" => deps::run(cx),
        "knip" => knip::run(cx),
        "lint-floor" => lint_floor::run(cx),
        "oxlint" => oxlint::run(cx),
        "pin" => pin::run(cx),
        "tsc" => tsc::run(cx),
        _ => Err(Error(format!("no built-in check `{name}`"))),
    }
}
