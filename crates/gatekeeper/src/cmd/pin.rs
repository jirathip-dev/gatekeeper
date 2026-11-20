use crate::cli::{self, say};
use gatekeeper_engine::{pin, run};
use std::io::Write as _;
use std::process::ExitCode;

/// `pin <version>`: check that release's signed list, then write `[pin]`, the `uses:` lines and the wrapper.
pub fn run(args: &[String]) -> ExitCode {
    let [version] = args else {
        return cli::usage_error(&"usage: gatekeeper pin <version>");
    };
    let root = match cli::root() {
        Ok(root) => root,
        Err(e) => return cli::usage_error(&e),
    };
    match pin::pin(&root, version, &run::out_dir(&root).join("pin")) {
        Ok(lines) => {
            for l in &lines {
                say(l);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            writeln!(std::io::stderr(), "gatekeeper pin: {e}").ok();
            ExitCode::from(1)
        }
    }
}
