pub(crate) mod baseline;
pub(crate) mod comments;
pub(crate) mod config;
pub(crate) mod frozen;
pub(crate) mod generated;
pub(crate) mod scan;

use gatekeeper_model::Glob;
use std::path::Path;

pub(crate) fn any(globs: &[Glob], path: &str) -> bool {
    globs.iter().any(|g| g.matches(path))
}

pub(crate) fn walk(root: &Path, dir: &str, named: &[String], out: &mut Vec<String>) {
    for e in std::fs::read_dir(root.join(dir))
        .into_iter()
        .flatten()
        .flatten()
    {
        let name = e.file_name().to_string_lossy().into_owned();
        let rel = if dir.is_empty() {
            name.clone()
        } else {
            format!("{dir}/{name}")
        };
        let listed = named.iter().any(|b| b.starts_with(&format!("{rel}/")));
        if (name.starts_with('.') && !listed)
            || name.starts_with("target")
            || name == "node_modules"
        {
            continue;
        }
        match e.file_type() {
            Ok(t) if t.is_dir() => walk(root, &rel, named, out),
            Ok(_) => out.push(rel),
            Err(_) => {}
        }
    }
}
