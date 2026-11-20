pub mod baseline;
pub mod check;
pub mod ci;
pub mod list;
pub mod pin;
pub mod selftest;
pub mod skill;

use std::process::ExitCode;

pub const VERBS: &[&str] = &[
    "baseline", "check", "ci", "list", "pin", "selftest", "skill",
];

pub fn run(name: &str, args: &[String]) -> Option<ExitCode> {
    match name {
        "baseline" => Some(baseline::run(args)),
        "check" => Some(check::run(args)),
        "ci" => Some(ci::run(args)),
        "list" => Some(list::run(args)),
        "pin" => Some(pin::run(args)),
        "selftest" => Some(selftest::run(args)),
        "skill" => Some(skill::run(args)),
        _ => None,
    }
}
