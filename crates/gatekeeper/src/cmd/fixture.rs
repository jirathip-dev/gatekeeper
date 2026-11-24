use crate::cli::{self, Args, say};
use gatekeeper_engine::{Error, selftest};
use std::path::Path;
use std::process::ExitCode;

/// `fixture <pack>/<gate>/<case> --to DIR`: write one fixture case's files to `DIR`.
pub fn run(args: &[String]) -> ExitCode {
    let Some((id, rest)) = args.split_first() else {
        return cli::usage_error(&"fixture needs <pack>/<gate>/<case> --to <dir>");
    };
    let written = Args::parse(rest, &["--to"], &[]).and_then(|a| {
        let to = a
            .value("--to")
            .ok_or_else(|| Error("fixture needs --to <dir>".into()))?;
        selftest::write_case(id, Path::new(to))
    });
    match written {
        Ok(files) => {
            say(&format!("wrote {}", files.join(", ")));
            ExitCode::SUCCESS
        }
        Err(e) => cli::usage_error(&e),
    }
}
