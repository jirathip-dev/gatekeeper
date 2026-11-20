use crate::cli::{self, Args, say};
use gatekeeper_engine::skill;
use std::path::Path;
use std::process::ExitCode;

/// `skill --to DIR`: write this version's skill to `DIR/gatekeeper/`.
pub fn run(args: &[String]) -> ExitCode {
    let parsed = Args::parse(args, &["--to"], &[]).and_then(|a| {
        let to = a
            .value("--to")
            .ok_or_else(|| gatekeeper_engine::Error("skill needs --to <dir>".into()))?
            .to_owned();
        skill::write(Path::new(&to))
    });
    let files = match parsed {
        Ok(files) => files,
        Err(e) => return cli::usage_error(&e),
    };
    let (examples, docs): (Vec<_>, Vec<_>) =
        files.iter().partition(|f| f.path.starts_with("examples/"));
    let docs: Vec<String> = docs
        .iter()
        .map(|f| format!("gatekeeper/{}", f.path))
        .collect();
    say(&format!(
        "wrote {}, gatekeeper/examples/ ({})",
        docs.join(", "),
        examples.len()
    ));
    ExitCode::SUCCESS
}
