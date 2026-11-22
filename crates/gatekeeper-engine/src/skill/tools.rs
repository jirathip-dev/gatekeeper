use crate::fetch::{self, ToolFile};
use crate::{Result, packs};

pub(super) fn tools_md() -> Result<String> {
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
