use crate::cli::{self, say};
use gatekeeper_engine::{pin, run};
use std::io::Write as _;
use std::process::ExitCode;

/// `pin <version>|latest`: check that release's signed list, then write `[pin]`, the `uses:` lines and the wrapper; `pin --outdated` exits 1 when a newer one verifies.
pub fn run(args: &[String]) -> ExitCode {
    let [arg] = args else {
        return cli::usage_error(&"usage: gatekeeper pin <version>|latest|--outdated");
    };
    let root = match cli::root() {
        Ok(root) => root,
        Err(e) => return cli::usage_error(&e),
    };
    let (scratch, cache) = (run::out_dir(&root).join("pin"), cli::cache());
    if arg == "--outdated" {
        return match pin::outdated(&root, &scratch, &cache) {
            Ok(Some(line)) => {
                say(&line);
                ExitCode::from(1)
            }
            Ok(None) => ExitCode::SUCCESS,
            Err(e) => cli::usage_error(&format!("pin --outdated: {e}")),
        };
    }
    match pin::pin(&root, arg, &scratch, &cache) {
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
