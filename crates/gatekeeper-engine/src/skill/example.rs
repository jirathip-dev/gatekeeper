use super::rerun;
use crate::{packs, selftest};
use gatekeeper_model::Gate;

const SHOWN_LINES: usize = 20;

fn case_change(prefix: &str) -> String {
    packs::under(prefix)
        .map(|(rel, bytes)| match (rel, std::str::from_utf8(bytes)) {
            ("message", Ok(text)) => format!(
                "Commits with the message:\n\n```\n{}\n```\n\n",
                text.trim_end()
            ),
            ("body", Ok(text)) => format!(
                "With the PR body (`{{head}}` is the case's commit, `{{base}}` its parent):\n\n```\n{}\n```\n\n",
                text.trim_end()
            ),
            ("env", Ok(text)) => {
                format!("With the environment:\n\n```\n{}\n```\n\n", text.trim_end())
            }
            ("expect" | "passes", _) => String::new(),
            ("release", _) => "The integration branch is at the case's commit: a release PR moves main up to it.\n\n".to_owned(),
            ("checks.json", Ok(text)) => format!(
                "The check writes `checks.json`:\n\n```json\n{}\n```\n\n",
                text.trim_end()
            ),
            ("remove", Ok(text)) => text
                .lines()
                .filter(|l| !l.is_empty())
                .map(|gone| format!("Removes `{gone}`.\n\n"))
                .collect::<Vec<_>>()
                .concat(),
            (_, Ok(text)) if text.lines().count() <= SHOWN_LINES => {
                format!("Writes `{rel}`:\n\n```\n{}\n```\n\n", text.trim_end())
            }
            (_, _) => format!("Writes `{rel}` (not shown).\n\n"),
        })
        .collect()
}

pub(super) fn example_md(gate: &Gate) -> String {
    let head = format!(
        "# {}: what a failure looks like\n\n\
         Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails \
         unless the gate fails on it printing the line shown, so these examples cannot go stale.\n",
        gate.id
    );
    let cases = selftest::cases(gate).into_iter().map(|case| {
        let prefix = format!("{}/fixtures/{}/{case}/", gate.id.pack, gate.id.name());
        if selftest::passes(gate, case) {
            return format!("\n## {case}\n\n{}The gate passes.\n", case_change(&prefix));
        }
        format!(
            "\n## {case}\n\n{}The gate fails and its log prints `{}`.\n\nFix: {}. Rerun: `{}`\n",
            case_change(&prefix),
            selftest::expect(gate, case),
            gate.fix,
            rerun(gate)
        )
    }).collect::<Vec<_>>().concat();
    head + &cases
}
