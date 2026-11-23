use crate::run::CheckCx as Cx;
use crate::{Error, Result, runner};
use serde::Serialize;
use std::fs;

const FILE: &str = ".drover/rules.md";
const MAX_LINES: usize = 30;

/// `checks.json`: every lesson check in the rules file and whether it passed this run; drover's lesson lane reads it.
#[derive(Serialize)]
pub struct RulesReport {
    pub file: String,
    pub checks: Vec<LessonCheck>,
}

/// A rules line `- don't X (enforced by: <name> `<cmd>`, shadow since <date>)`, or `active since`.
#[derive(Serialize)]
pub struct LessonCheck {
    pub line: usize,
    pub name: String,
    pub cmd: String,
    pub state: CheckState,
    pub since: String,
    pub passed: bool,
}

/// A shadow check reports and never fails the gate; an active one fails it.
#[derive(Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum CheckState {
    Shadow,
    Active,
}

fn parse(line: usize, l: &str) -> Option<LessonCheck> {
    let (state, rest) = (l
        .split_once("shadow since ")
        .map(|x| (CheckState::Shadow, x.1)))
    .or_else(|| {
        l.split_once("active since ")
            .map(|x| (CheckState::Active, x.1))
    })?;
    let name = l.split_once("enforced by: ")?.1;
    let name = name.split([' ', '`', ',', ')']).next()?;
    let cmd = l.split('`').nth(1).filter(|c| !c.is_empty())?;
    let since = rest.get(..10)?;
    (!name.is_empty()).then(|| LessonCheck {
        line,
        name: name.into(),
        cmd: cmd.into(),
        state,
        since: since.into(),
        passed: false,
    })
}

fn malformed(text: &str) -> Vec<String> {
    let unenforced = |l: &str| {
        let named = l.split_once("(enforced by: ").map(|x| x.1.trim_start());
        !named.is_some_and(|n| !n.is_empty() && !n.starts_with(')'))
    };
    let lines: Vec<&str> = text.lines().collect();
    let mut bad: Vec<String> = (lines.iter().zip(1..))
        .filter(|(l, _)| l.starts_with("- ") && unenforced(l))
        .map(|(_, n)| {
            format!(
                "FAIL {FILE}: line {n} names no enforcer: end it `(enforced by: <check>)` or `(enforced by: none)`"
            )
        })
        .collect();
    if lines.len() > MAX_LINES {
        bad.push(format!(
            "FAIL {FILE}: {} lines, over {MAX_LINES}: delete the rules a check now enforces",
            lines.len()
        ));
    }
    bad
}

/// The rules file's line format and length; every lesson check runs, an active one that fails fails the gate; writes `checks.json`.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let text = fs::read_to_string(cx.root.join(FILE)).unwrap_or_default();
    let mut out = malformed(&text);
    let mut would = Vec::new();
    let mut checks: Vec<LessonCheck> = (text.lines().zip(1..))
        .filter_map(|(l, n)| parse(n, l))
        .collect();
    for c in &mut checks {
        let o = runner::capture("sh", &["-c", &c.cmd], cx.root)?;
        c.passed = o.status.success();
        if c.passed {
            continue;
        }
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        match c.state {
            CheckState::Active => {
                out.push(format!("FAIL {}: `{}`\n{}", c.name, c.cmd, said.trim_end()));
            }
            CheckState::Shadow => would.push(c.name.clone()),
        }
    }
    if !would.is_empty() {
        let n = would.len();
        let s = if n == 1 { "" } else { "s" };
        let line = format!("{n} shadow check{s} would fail: {}", would.join(", "));
        cx.note.set(line).ok();
    }
    let report = RulesReport {
        file: FILE.into(),
        checks,
    };
    let json = serde_json::to_string_pretty(&report).map_err(|e| Error(e.to_string()))?;
    fs::create_dir_all(cx.scratch).map_err(|e| Error::at(cx.scratch, e))?;
    let path = cx.scratch.join("checks.json");
    fs::write(&path, json + "\n").map_err(|e| Error::at(&path, e))?;
    Ok(out)
}
