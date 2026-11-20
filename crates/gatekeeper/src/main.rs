mod cli;
mod cmd;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = format!("usage: gatekeeper <{}> [options]", cmd::VERBS.join("|"));
    let Some((verb, rest)) = args.split_first() else {
        return cli::usage_error(&usage);
    };
    if verb == "--version" && rest.is_empty() {
        let build = gatekeeper_engine::BUILD.unwrap_or("unknown");
        cli::say(&format!(
            "gatekeeper {} ({build})",
            gatekeeper_engine::VERSION
        ));
        return ExitCode::SUCCESS;
    }
    cmd::run(verb, rest).unwrap_or_else(|| cli::usage_error(&usage))
}
