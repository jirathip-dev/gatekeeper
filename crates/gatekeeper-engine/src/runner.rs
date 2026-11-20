use crate::{Error, Result};
use std::fs::File;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// How a started process ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    Exit(i32),
    Timeout,
}

/// Runs `program args` in `cwd` with its output appended to `log`, killed at `deadline`.
pub fn logged(
    program: &Path,
    args: &[String],
    cwd: &Path,
    log: &mut File,
    deadline: Instant,
) -> Result<Ended> {
    writeln!(log, "$ {} {}", program.display(), args.join(" "))
        .map_err(|e| Error(e.to_string()))?;
    let err = log.try_clone().map_err(|e| Error(e.to_string()))?;
    let out = log.try_clone().map_err(|e| Error(e.to_string()))?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()
        .map_err(|e| Error(format!("{}: {e}", program.display())))?;
    let mut pause = Duration::from_millis(1);
    loop {
        if let Some(status) = child.try_wait().map_err(|e| Error(e.to_string()))? {
            return Ok(Ended::Exit(status.code().unwrap_or(-1)));
        }
        if Instant::now() >= deadline {
            child.kill().map_err(|e| Error(e.to_string()))?;
            child.wait().map_err(|e| Error(e.to_string()))?;
            writeln!(log, "killed: over five times the budget")
                .map_err(|e| Error(e.to_string()))?;
            return Ok(Ended::Timeout);
        }
        sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(50));
    }
}

/// `sh -c line` in `cwd`, logged.
pub fn shell(line: &str, cwd: &Path, log: &mut File, deadline: Instant) -> Result<Ended> {
    logged(
        Path::new("sh"),
        &["-c".to_owned(), line.to_owned()],
        cwd,
        log,
        deadline,
    )
}

/// Runs `program args` in `cwd` on the caller's terminal; its exit code.
pub fn attached(program: &str, args: &[String], cwd: &Path) -> Result<i32> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|e| Error(format!("{program}: {e}")))?;
    Ok(status.code().unwrap_or(-1))
}

/// Runs a short command and returns its output; no timeout, nothing logged.
pub fn capture(program: &str, args: &[&str], cwd: &Path) -> Result<Output> {
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| Error(format!("{program}: {e}")))
}

/// Runs a short command and fails unless it exits 0.
pub fn ok(program: &str, args: &[&str], cwd: &Path) -> Result<String> {
    let out = capture(program, args, cwd)?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    Err(Error(format!(
        "{program} {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr).trim()
    )))
}
