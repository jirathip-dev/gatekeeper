use crate::run::RunCtx;
use crate::{Error, Result, config, scope};
use gatekeeper_model::{Coverage, CoverageTool, GateId, Health, Tenths};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) const DIR: &str = "ci/tests/coverage.d";

pub(crate) struct Judged {
    pub lines: Tenths,
    pub note: String,
    pub findings: Vec<String>,
}

fn measured(root: &Path, cov: &Coverage) -> Result<Tenths> {
    let at = &cov.summary;
    let path = root.join(at);
    let text = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
    let json: Value = serde_json::from_str(&text).map_err(|e| Error(format!("{at}: {e}")))?;
    let (lines, count) = match cov.tool {
        CoverageTool::LlvmCov => (json.pointer("/data/0/totals/lines"), "count"),
        CoverageTool::Vitest => (json.pointer("/total/lines"), "total"),
    };
    let num = |k: &str| lines.and_then(|l| l.get(k)).and_then(Value::as_u64);
    let tenths = num("covered").zip(num(count));
    tenths.and_then(|(c, n)| Tenths::of(c, n)).ok_or_else(|| {
        Error(format!(
            "{at}: no line totals: is it a {} summary of a run that counted lines?",
            match cov.tool {
                CoverageTool::LlvmCov => "cargo-llvm-cov --json --summary-only",
                CoverageTool::Vitest => "vitest json-summary",
            }
        ))
    })
}

fn integration(cx: &RunCtx, gate: &GateId) -> Option<Tenths> {
    let text = fs::read_to_string(cx.timings.as_ref()?.join("health.json")).ok()?;
    let health: Health = serde_json::from_str(&text).ok()?;
    health
        .coverage
        .into_iter()
        .find(|c| c.gate == *gate)?
        .lines
        .into()
}

fn trailer(line: &str) -> Option<Tenths> {
    let num = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    let (old, rest) = line.trim().strip_prefix("Coverage: ")?.split_once(" → ")?;
    let (new, refs) = rest.split_once(" (#")?;
    let (issue, q) = refs.strip_suffix(')')?.split_once(", q")?;
    old.parse::<Tenths>().ok()?;
    (num(issue) && num(q)).then(|| new.parse().ok())?
}

fn drops(cx: &RunCtx, base: &str, gate: &GateId) -> Result<(Option<Tenths>, Vec<String>)> {
    let (mut floor, mut findings) = (None::<Tenths>, Vec::new());
    for (file, sha, body) in scope::added_by(&cx.root, base, DIR)? {
        let path = cx.root.join(&file);
        if path.extension().is_none_or(|x| x != "toml") || !path.exists() {
            continue;
        }
        let table = config::read_toml(&path)?;
        if table.get("gate").and_then(toml::Value::as_str) != Some(&gate.to_string()) {
            continue;
        }
        let why = table.get("why").and_then(toml::Value::as_str);
        let value = table
            .get("floor")
            .map_or_else(String::new, ToString::to_string);
        let parsed = value
            .parse::<Tenths>()
            .map_err(|e| Error(format!("{file}: floor {e}")));
        let at = parsed?;
        if why.is_none_or(|w| w.trim().is_empty()) {
            findings.push(format!("{file}: a drop names its reason in `why`"));
        } else if !body.lines().any(|l| trailer(l) == Some(at)) {
            findings.push(format!(
                "{file}: its commit {sha} carries no `Coverage: <old> → {at} (#issue, q<N>)` trailer naming the owner's decision"
            ));
        } else {
            floor = Some(floor.map_or(at, |f| f.min(at)));
        }
    }
    Ok((floor, findings))
}

pub(crate) fn judge(id: &GateId, cov: &Coverage, cx: &RunCtx) -> Result<Judged> {
    let lines = measured(&cx.root, cov)?;
    let said = |note: String, findings| {
        Ok(Judged {
            lines,
            note,
            findings,
        })
    };
    let (Some(base), Some(_)) = (cx.base.as_deref(), cx.head.as_deref()) else {
        return said(
            format!("line coverage {lines} %: not a pull request, nothing compared"),
            Vec::new(),
        );
    };
    let Some(was) = integration(cx, id) else {
        return said(
            format!(
                "line coverage {lines} %: no integration number yet (a first run or an evicted entry), so it passes"
            ),
            Vec::new(),
        );
    };
    let (floor, mut findings) = drops(cx, base, id)?;
    let note = format!("line coverage {lines} % (integration {was} %)");
    if Tenths(lines.0 + Tenths::TOLERANCE.0) >= was {
        return said(note, findings);
    }
    match floor {
        Some(f) if lines >= f => said(format!("{note}: a deliberate drop to {f} %"), findings),
        _ => {
            findings.push(format!(
                "line coverage {lines} % is more than {} under the integration branch's {was} %",
                Tenths::TOLERANCE
            ));
            said(note, findings)
        }
    }
}
