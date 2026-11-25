use super::{RunCtx, out_dir};
use crate::{Result, fetch, packs, runner};
use gatekeeper_model::{Gate, PackId};
use std::path::{Path, PathBuf};

pub(crate) type Vars = Vec<(&'static str, String)>;

pub(super) fn expand(args: &[String], cx: &RunCtx) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        match arg.as_str() {
            "{--base}" => out.extend(
                cx.base
                    .iter()
                    .flat_map(|b| ["--base".to_owned(), b.clone()]),
            ),
            "{--pr-body}" => out.extend(
                cx.pr_body
                    .iter()
                    .flat_map(|p| ["--pr-body".to_owned(), p.to_string_lossy().into_owned()]),
            ),
            _ => out.push(arg.clone()),
        }
    }
    out
}

fn pack_dir(pack: PackId, dir: &str, out: &Path) -> Result<PathBuf> {
    let to = out.join("pack").join(pack.as_str()).join(dir);
    packs::materialize(&format!("{pack}/{dir}/"), &to)?;
    Ok(to)
}

/// # Errors
pub fn script(root: &Path, pack: PackId, script: &str, args: &[String]) -> Result<i32> {
    let path = pack_dir(pack, "scripts", &out_dir(root))?.join(script);
    let mut all = vec![path.to_string_lossy().into_owned()];
    all.extend_from_slice(args);
    runner::attached("node", &all, root)
}

pub(crate) fn filled(run: &str, gate: &Gate, vars: &Vars, cx: &RunCtx) -> Result<String> {
    let mut line = run.to_owned();
    for (var, value) in vars {
        line = line.replace(var, value);
    }
    let base = cx
        .base
        .as_ref()
        .map_or(String::new(), |b| format!("--base {b}"));
    let body = cx
        .pr_body
        .as_ref()
        .map_or(String::new(), |p| p.to_string_lossy().into_owned());
    line = line.replace("{--base}", &base).replace("{pr-body}", &body);
    for dir in ["files", "scripts"] {
        let to = pack_dir(gate.id.pack, dir, &cx.out)?;
        line = line.replace(&format!("{{{dir}}}"), &to.to_string_lossy());
    }
    for tool in fetch::named_in(run) {
        let bin = fetch::resolve(tool, &cx.cache, &cx.tools)?;
        line = line.replace(&format!("{{tool:{tool}}}"), &bin.to_string_lossy());
    }
    Ok(line)
}
