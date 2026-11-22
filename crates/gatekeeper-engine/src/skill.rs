use crate::{Error, Result, VERSION, packs};
use example::example_md;
use gatekeeper_model::{Gate, PackId};
use pack::pack_md;
use std::fs;
use std::path::Path;
use tools::tools_md;

mod example;
mod pack;
mod tools;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/topics.rs"));
}

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
         A repo's own gates live in `.gatekeeper/gates/<gate>.toml` as `local/<gate>` and print their `fix` the same way.\n\n"
    );
    let topics = embedded::TOPICS
        .iter()
        .flat_map(|(_, text)| [text.trim_end(), "\n\n"])
        .collect::<String>();
    let list = packs
        .iter()
        .map(|(pack, about)| format!("- [{pack}](packs/{pack}.md): {about}\n"))
        .collect::<Vec<_>>()
        .concat();
    head + &topics + "## Packs\n\n" + &list
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
