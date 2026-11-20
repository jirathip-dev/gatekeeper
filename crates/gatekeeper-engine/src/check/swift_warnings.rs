use crate::run::CheckCx as Cx;
use crate::{Error, Result, runner};
use std::fs;
use std::process::Output;

const FLAG: &str = "GATEKEEPER_STRICT";
const AS_ERRORS: [&str; 2] = ["-warnings-as-errors", "treatAllWarnings(as: .error)"];

fn floor(package: &str, manifest: &str) -> Option<String> {
    let strict = manifest.contains(FLAG) && AS_ERRORS.iter().any(|a| manifest.contains(a));
    (!strict).then(|| {
        format!(
            "{package}/Package.swift: treats warnings as errors under no {FLAG}=1 (-warnings-as-errors when the flag is set)"
        )
    })
}

fn errors(what: &str, out: &Output) -> Vec<String> {
    if out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout) + String::from_utf8_lossy(&out.stderr);
    let lines: Vec<String> = text
        .lines()
        .filter(|l| l.contains("error:"))
        .map(|l| format!("{what}: {}", l.trim()))
        .collect();
    if lines.is_empty() {
        vec![format!("{what}: exited {}", out.status)]
    } else {
        lines
    }
}

/// Each `[swift]` package holds the strict flag and builds with its tests under it; each Xcode project builds with warnings as errors.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let swift = cx.swift()?;
    let mut findings = Vec::new();
    for package in &swift.packages {
        let path = cx.root.join(package).join("Package.swift");
        let manifest = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
        findings.extend(floor(package, &manifest));
        let flag = format!("{FLAG}=1");
        let args = [
            flag.as_str(),
            "swift",
            "build",
            "--build-tests",
            "--package-path",
            package,
        ];
        let out = runner::capture("env", &args, cx.root)?;
        findings.extend(errors(&args.join(" "), &out));
    }
    for project in &swift.projects {
        let mut args = vec!["-project", &project.path, "-scheme", &project.scheme];
        if let Some(d) = &project.destination {
            args.extend(["-destination", d]);
        }
        args.extend([
            "build",
            "CODE_SIGNING_ALLOWED=NO",
            "SWIFT_TREAT_WARNINGS_AS_ERRORS=YES",
            "GCC_TREAT_WARNINGS_AS_ERRORS=YES",
        ]);
        let out = runner::capture("xcodebuild", &args, cx.root)?;
        findings.extend(errors(&format!("xcodebuild {}", project.path), &out));
    }
    Ok(findings)
}
