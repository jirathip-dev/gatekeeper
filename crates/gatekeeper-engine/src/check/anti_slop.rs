use crate::run::CheckCx as Cx;
use crate::{Error, Result, runner};

/// The prebuilt anti-slop-swift over `[swift] sources`, with the `[swift] extra` rules on; each violation is a finding.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let swift = cx.swift()?;
    let bin = cx.tool("anti-slop-swift")?;
    let enable = format!("--enable={}", swift.extra.join(","));
    let mut args: Vec<&str> = Vec::new();
    if !swift.extra.is_empty() {
        args.push(&enable);
    }
    args.extend(swift.sources.iter().map(String::as_str));
    let out = runner::capture(&bin.to_string_lossy(), &args, cx.root)?;
    let text = String::from_utf8_lossy(&out.stdout);
    let findings: Vec<String> = text
        .lines()
        .filter(|l| l.contains(": error: ["))
        .map(str::to_owned)
        .collect();
    if findings.is_empty() && !out.status.success() {
        return Err(Error(format!(
            "anti-slop exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(findings)
}
