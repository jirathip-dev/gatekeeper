pub mod anti_slop;
pub mod arch;
pub mod banned_apis;
pub mod cap;
pub mod deny_floor;
pub mod deps;
pub mod docs;
pub mod hygiene;
pub mod knip;
pub mod lint_floor;
pub mod machete;
pub mod oxlint;
pub mod pin;
pub mod pr_body;
pub mod ratchet;
pub mod rules;
pub mod swift_tokens;
pub mod swift_warnings;
pub mod test_health;
pub mod tsc;
pub mod unused_pub;

use crate::run::CheckCx;
use crate::{Error, Result};

pub const NAMES: &[&str] = &[
    "anti-slop",
    "arch",
    "banned-apis",
    "cap",
    "deny-floor",
    "deps",
    "docs",
    "hygiene",
    "knip",
    "lint-floor",
    "machete",
    "oxlint",
    "pin",
    "pr-body",
    "ratchet",
    "rules",
    "swift-tokens",
    "swift-warnings",
    "test-health",
    "tsc",
    "unused-pub",
];

pub fn run(name: &str, cx: &CheckCx) -> Result<Vec<String>> {
    match name {
        "anti-slop" => anti_slop::run(cx),
        "arch" => arch::run(cx),
        "banned-apis" => banned_apis::run(cx),
        "cap" => cap::run(cx),
        "deny-floor" => deny_floor::run(cx),
        "deps" => deps::run(cx),
        "docs" => docs::run(cx),
        "hygiene" => hygiene::run(cx),
        "knip" => knip::run(cx),
        "lint-floor" => lint_floor::run(cx),
        "machete" => machete::run(cx),
        "oxlint" => oxlint::run(cx),
        "pin" => pin::run(cx),
        "pr-body" => pr_body::run(cx),
        "ratchet" => ratchet::run(cx),
        "rules" => rules::run(cx),
        "swift-tokens" => swift_tokens::run(cx),
        "swift-warnings" => swift_warnings::run(cx),
        "test-health" => test_health::run(cx),
        "tsc" => tsc::run(cx),
        "unused-pub" => unused_pub::run(cx),
        _ => Err(Error(format!("no built-in check `{name}`"))),
    }
}
