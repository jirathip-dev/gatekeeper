pub mod baseline;
pub mod check;
pub mod ci;
pub mod list;
pub mod selftest;

use std::process::ExitCode;

pub const VERBS: &[&str] = &["baseline", "check", "ci", "list", "selftest"];

pub fn run(name: &str, args: &[String]) -> Option<ExitCode> {
    match name {
        "baseline" => Some(baseline::run(args)),
        "check" => Some(check::run(args)),
        "ci" => Some(ci::run(args)),
        "list" => Some(list::run(args)),
        "selftest" => Some(selftest::run(args)),
        _ => None,
    }
}
