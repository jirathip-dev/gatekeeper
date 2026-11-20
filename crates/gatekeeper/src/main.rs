mod cli;
mod cmd;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = format!("usage: gatekeeper <{}> [options]", cmd::VERBS.join("|"));
    let Some((verb, rest)) = args.split_first() else {
        return cli::usage_error(&usage);
    };
    cmd::run(verb, rest).unwrap_or_else(|| cli::usage_error(&usage))
}
