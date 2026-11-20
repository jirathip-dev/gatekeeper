use crate::pin::{call, is_commit, workflow_files};
use crate::run::CheckCx as Cx;
use crate::{BUILD, Error, Result, VERSION, config, packs};
use std::fs;

const WRAPPER: &str = ".gatekeeper/run";

fn line(at: &str, line: &str, out: &mut Vec<String>) {
    if line
        .split('#')
        .next()
        .is_some_and(|l| l.contains("secrets: inherit"))
    {
        out.push(format!("{at}: secrets: inherit; pass each secret by name"));
    }
    let Some((_, git_ref, comment)) = call(line) else {
        return;
    };
    if !is_commit(git_ref) {
        out.push(format!("{at}: @{git_ref} is not a 40-hex commit"));
    } else if BUILD != Some(git_ref) {
        out.push(format!(
            "{at}: @{git_ref} is not {}, the commit this gatekeeper was built from",
            BUILD.unwrap_or("known")
        ));
    }
    if comment != format!("v{VERSION}") {
        out.push(format!(
            "{at}: the comment is {comment}, the pin is v{VERSION}"
        ));
    }
}

/// Every gatekeeper workflow call names this build's commit and version and no `secrets: inherit`; the wrapper is this version's.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    config::load(cx.root)?;
    let mut out = Vec::new();
    for path in workflow_files(cx.root) {
        let text = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
        let file = path
            .strip_prefix(cx.root)
            .unwrap_or(&path)
            .to_string_lossy();
        for (i, l) in text.lines().enumerate() {
            line(&format!("{file}:{}", i + 1), l, &mut out);
        }
    }
    let wrapper = fs::read(cx.root.join(WRAPPER)).unwrap_or_default();
    if packs::text("core/files/run").map(str::as_bytes) != Some(wrapper.as_slice()) {
        out.push(format!(
            "{WRAPPER} differs from the wrapper of gatekeeper {VERSION}; run ./.gatekeeper/run pin {VERSION}"
        ));
    }
    Ok(out)
}
