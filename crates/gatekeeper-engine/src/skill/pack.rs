use super::{example_path, rerun};
use gatekeeper_model::{Gate, PackId, Unkeyed};

pub(super) fn pack_md(pack: PackId, about: &str, gates: &[Gate]) -> String {
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
