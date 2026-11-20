use crate::run::CheckCx as Cx;
use crate::{BUILD, Error, Result, VERSION, config, packs};
use std::fs;
use std::path::Path;

const WRAPPER: &str = ".gatekeeper/run";

fn call(file: &str, n: usize, line: &str, out: &mut Vec<String>) {
    let Some((_, rest)) = line.split_once("uses:") else {
        return;
    };
    let (target, comment) = rest.split_once('#').unwrap_or((rest, ""));
    let Some((path, git_ref)) = target.trim().split_once('@') else {
        return;
    };
    if path.starts_with("./") || !path.contains("/gatekeeper/.github/workflows/") {
        return;
    }
    let at = format!("{file}:{n}");
    if git_ref.len() != 40
        || !git_ref
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        out.push(format!("{at}: @{git_ref} is not a 40-hex commit"));
    } else if BUILD != Some(git_ref) {
        out.push(format!(
            "{at}: @{git_ref} is not {}, the commit this gatekeeper was built from",
            BUILD.unwrap_or("known")
        ));
    }
    let comment = comment.trim();
    if comment != format!("v{VERSION}") {
        out.push(format!(
            "{at}: the comment is {comment}, the pin is v{VERSION}"
        ));
    }
}

fn workflows(root: &Path, out: &mut Vec<String>) -> Result<()> {
    let dir = root.join(".github/workflows");
    let Ok(entries) = fs::read_dir(&dir) else {
        return Ok(());
    };
    let mut files: Vec<_> = entries.filter_map(|e| Some(e.ok()?.path())).collect();
    files.sort();
    for path in files.iter().filter(|p| p.is_file()) {
        let text = fs::read_to_string(path).map_err(|e| Error::at(path, e))?;
        let file = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
        for (i, line) in text.lines().enumerate() {
            if line
                .split('#')
                .next()
                .unwrap_or("")
                .contains("secrets: inherit")
            {
                out.push(format!(
                    "{file}:{}: secrets: inherit; pass each secret by name",
                    i + 1
                ));
            }
            call(&file, i + 1, line, out);
        }
    }
    Ok(())
}

/// Every gatekeeper workflow call names this build's commit and version and no `secrets: inherit`; the wrapper is this version's.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    config::load(cx.root)?;
    let mut out = Vec::new();
    workflows(cx.root, &mut out)?;
    let wrapper = fs::read(cx.root.join(WRAPPER)).unwrap_or_default();
    if packs::text("core/files/run").map(str::as_bytes) != Some(wrapper.as_slice()) {
        out.push(format!(
            "{WRAPPER} differs from the wrapper of gatekeeper {VERSION}; run ./.gatekeeper/run pin {VERSION}"
        ));
    }
    Ok(out)
}
