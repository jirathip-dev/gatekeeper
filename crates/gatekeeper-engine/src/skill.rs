use crate::fetch::{self, ToolFile};
use crate::{Error, Result, VERSION, packs, selftest};
use gatekeeper_model::{Gate, PackId, Unkeyed};
use std::fs;
use std::path::Path;

const SHOWN_LINES: usize = 20;

/// One file of the skill: its path under the skills folder and its text.
pub struct SkillFile {
    pub path: String,
    pub text: String,
}

fn rerun(gate: &Gate) -> String {
    format!("./.gatekeeper/run check --gate {}", gate.id)
}

fn example_path(gate: &Gate) -> String {
    format!("examples/{}.md", gate.id.file_stem())
}

fn tools_md() -> Result<String> {
    let head = "\n## Pinned tools\n\n\
         A gate's `run` line names one as `{tool:<name>}`: gatekeeper fetches it by version, checks its sha256 and \
         puts the cached path there, on a lane's host and on a runner alike. A tool with no upstream binary is built \
         once per version by gatekeeper's `tools` workflow; never build it from source in a repo's CI.\n\n";
    let lines = packs::tool_files()
        .map(|(name, path, text)| {
            let file = fetch::parse(path, text)?;
            let built = match &file {
                ToolFile::Binary(t) => t.build.as_ref().map_or(String::new(), |b| {
                    format!(
                        ", built by gatekeeper for {} (`{}`)",
                        b.targets.join(", "),
                        b.bin
                    )
                }),
                ToolFile::Node(_) => String::new(),
            };
            Ok(format!("- `{name}` {}{built}\n", file.version()))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(head.to_owned() + &lines.concat())
}

fn skill_md(packs: &[(PackId, String)]) -> String {
    let head = format!(
        "---\nname: gatekeeper\ndescription: Read, fix and rerun a failing gatekeeper gate (gatekeeper {VERSION}).\n---\n\n\
         # gatekeeper {VERSION}\n\n\
         `./.gatekeeper/run check` is what CI runs and what a lane runs. A failing gate prints `FAIL  <pack>/<gate>`, \
         its log's tail, its `fix:` line, its log path and the command that reruns it alone.\n\n\
         1. Read the gate's entry in its pack file below: what it checks and how to fix it.\n\
         2. Fix the cause, never the gate: no pack file is copied into the repo or edited there.\n\
         3. Rerun the one gate: `./.gatekeeper/run check --gate <pack>/<gate>`, then `./.gatekeeper/run check`.\n\n\
         `./.gatekeeper/run list` prints the planned gates with their budgets and why one is skipped. \
         A repo's own gates live in `.gatekeeper/gates/<gate>.toml` as `local/<gate>` and print their `fix` the same way.\n\n\
         With `[cache]` in `.gatekeeper/config.toml` a gate whose inputs already passed prints `cached` with the run that proved it. \
         A rerun of a cached pass that fails now prints `cached pass does not hold`: the gate reads a file it does not declare; \
         add it to the gate's `paths` or set `cache = false`. `--no-cache` runs and records nothing. \
         A `projects` gate keeps one record per project, over that project, every project it needs and every file outside all projects \
         that `[scope] ignore` keeps: a change reruns the projects it reaches, and the row says how many ran and how many were cached. \
         A sharded CI job keeps one record for its whole set of projects and is cached only while that set and its files are unchanged.\n\n\
         A gate with `[select]` runs only the test files a change can reach, passed to its `run` line in place of `{{files}}`. \
         Its row prints `selected N of M test files in Xs:` with a count per rule (closure, always, readers, project-readers, widened), \
         or `selected all M test files:` and why, then `list: target/gatekeeper/<pack>-<gate>.selection.json`. \
         Why did a test run, or not? That file names each selected file with every rule that selected it: `closure` (vitest lists its imports \
         reaching a changed file), `always`, `reads` (its code names a changed file's path or basename, or it walks that file's project or a \
         directory above it), `names-project`, `widened` (a deleted file's project or import specifier). Rerun a failing test with the same \
         command: the same tree and base select the same files. A selection that cannot be computed fails the gate with its \
         `.select.log`; it never runs fewer tests. \
         `./.gatekeeper/run replay --gate <pack>/<gate> --runs runs.jsonl --other '<cmd> {{base}}'` measures it against another selector: \
         each line of `runs.jsonl` is `{{\"pr\", \"base\", \"head\", \"failed\"}}`, both select in a scratch worktree of the head, and it prints \
         per run the counts, the files only one chose and each missed failure, then the totals, into `target/gatekeeper/replay/`; \
         it exits 1 when gatekeeper missed a failure.\n\n\
         ## Packs\n\n"
    );
    let list = packs
        .iter()
        .map(|(pack, about)| format!("- [{pack}](packs/{pack}.md): {about}\n"))
        .collect::<Vec<_>>()
        .concat();
    head + &list
}

fn pack_md(pack: PackId, about: &str, gates: &[Gate]) -> String {
    let list = gates.iter().map(|g| {
        let rule = g.rule.iter().zip(1..).map(|(l, i)| format!("{i}. {l}\n")).collect::<Vec<_>>().concat();
        let rule = if rule.is_empty() { rule } else { format!("\n{rule}") };
        format!(
            "\n## {}\n\n{}\n{rule}\n- fix: {}\n- budget: {}s\n- cached: {}\n- rerun: `{}`\n- what a failure looks like: [{}](../{})\n",
            g.id,
            g.about,
            g.fix,
            g.budget.0,
            Unkeyed::of(g).map_or("yes".into(), |why| format!("no, {why}")),
            rerun(g),
            g.id.file_stem(),
            example_path(g)
        )
    }).collect::<Vec<_>>().concat();
    format!("# The {pack} pack\n\n{about}\n{list}")
}

fn case_change(prefix: &str) -> String {
    packs::under(prefix)
        .map(|(rel, bytes)| match (rel, std::str::from_utf8(bytes)) {
            ("message", Ok(text)) => format!(
                "Commits with the message:\n\n```\n{}\n```\n\n",
                text.trim_end()
            ),
            ("body", Ok(text)) => format!(
                "With the PR body (`{{head}}` is the case's commit):\n\n```\n{}\n```\n\n",
                text.trim_end()
            ),
            ("env", Ok(text)) => {
                format!("With the environment:\n\n```\n{}\n```\n\n", text.trim_end())
            }
            ("expect" | "passes", _) => String::new(),
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

fn example_md(gate: &Gate) -> String {
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

/// # Errors
pub fn files() -> Result<Vec<SkillFile>> {
    let mut abouts = Vec::new();
    let mut out = Vec::new();
    for pack in PackId::BUILT_IN {
        let about = packs::pack_file(pack)?.about;
        let gates = packs::pack_gates(pack)?;
        out.push(SkillFile {
            path: format!("packs/{pack}.md"),
            text: pack_md(pack, &about, &gates),
        });
        out.extend(gates.iter().map(|g| SkillFile {
            path: example_path(g),
            text: example_md(g),
        }));
        abouts.push((pack, about));
    }
    out.insert(
        0,
        SkillFile {
            path: "SKILL.md".into(),
            text: skill_md(&abouts) + &tools_md()?,
        },
    );
    Ok(out)
}

/// # Errors
pub fn write(to: &Path) -> Result<Vec<SkillFile>> {
    let root = to.join("gatekeeper");
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|e| Error::at(&root, e))?;
    }
    let files = files()?;
    for f in &files {
        let path = root.join(&f.path);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| Error::at(dir, e))?;
        }
        fs::write(&path, &f.text).map_err(|e| Error::at(&path, e))?;
    }
    Ok(files)
}
