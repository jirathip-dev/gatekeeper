use super::machete::{at, workspace};
use crate::ratchet::reports;
use crate::run::CheckCx as Cx;
use crate::{Error, Result, packs, runner};

/// One more clippy pass with `too_many_lines` and `cognitive_complexity` forced on through every `allow`; each function over a threshold is a finding.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let (dir, prefix) = workspace(cx);
    packs::materialize("rust/files/", cx.scratch)?;
    let conf = format!("CLIPPY_CONF_DIR={}", cx.scratch.to_string_lossy());
    let args = [
        conf.as_str(),
        "cargo",
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
        "--message-format=json",
        "--",
        "--cap-lints",
        "warn",
        "-A",
        "warnings",
        "--force-warn",
        "clippy::too_many_lines",
        "--force-warn",
        "clippy::cognitive_complexity",
    ];
    let out = runner::capture("env", &args, &dir)?;
    if !out.status.success() {
        return Err(Error(format!(
            "cargo clippy exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let found = reports::clippy(&String::from_utf8_lossy(&out.stdout));
    Ok(found.into_iter().map(|f| at(&prefix, &f)).collect())
}
