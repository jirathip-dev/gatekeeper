pub mod baseline;
pub mod check;
pub mod ci;
pub mod list;
pub mod pin;
pub mod replay;
pub mod selftest;
pub mod skill;
pub mod supabase;

use std::process::ExitCode;

pub const VERBS: &[&str] = &[
    "baseline", "check", "ci", "list", "pin", "replay", "selftest", "skill", "supabase",
];

pub fn run(name: &str, args: &[String]) -> Option<ExitCode> {
    match name {
        "baseline" => Some(baseline::run(args)),
        "check" => Some(check::run(args)),
        "ci" => Some(ci::run(args)),
        "list" => Some(list::run(args)),
        "pin" => Some(pin::run(args)),
        "replay" => Some(replay::run(args)),
        "selftest" => Some(selftest::run(args)),
        "skill" => Some(skill::run(args)),
        "supabase" => Some(supabase::run(args)),
        _ => None,
    }
}
