use crate::baseline::Written;
use crate::{Error, Result, config};
use gatekeeper_model::Date;
use std::fmt::Display;
use std::fs;
use std::path::Path;
use std::str::FromStr;

pub(crate) fn read<T>(root: &Path, dir: &str) -> Result<Vec<(String, T)>>
where
    T: FromStr<Err: Display>,
{
    let mut out = Vec::new();
    for e in fs::read_dir(root.join(dir)).into_iter().flatten().flatten() {
        let path = e.path();
        if path.extension().is_none_or(|x| x != "toml") {
            continue;
        }
        let at = format!("{dir}/{}", e.file_name().to_string_lossy());
        let table = config::read_toml(&path)?;
        let value = table.get("ceiling").map(ToString::to_string);
        let ceiling = value
            .unwrap_or_default()
            .parse()
            .map_err(|e| Error(format!("{at}: {e}")))?;
        out.push((at, ceiling));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

pub(crate) fn lowest<T: Ord>(files: &[(String, T)]) -> Option<&(String, T)> {
    files.iter().min_by(|a, b| a.1.cmp(&b.1))
}

pub(crate) fn write<T>(
    root: &Path,
    (dir, what): (&str, &str),
    today: Date,
    (fits, stem): (T, impl Display),
) -> Result<Written>
where
    T: FromStr<Err: Display> + Ord + Display,
{
    let files = read::<T>(root, dir)?;
    let path = root.join(dir);
    let note = match lowest(&files) {
        Some((at, c)) if fits >= *c => {
            return Ok(Written {
                path,
                lines: 0,
                unlisted: Vec::new(),
                note: Some(format!(
                    "{what} fits {fits}, the ceiling is {c} ({at}): nothing written"
                )),
            });
        }
        Some((_, c)) => format!("ceiling {fits} (was {c})"),
        None => format!("ceiling {fits} (the first)"),
    };
    fs::create_dir_all(&path).map_err(|e| Error::at(&path, e))?;
    let file = path.join(format!("{today}-{stem}.toml"));
    fs::write(&file, format!("ceiling = {fits}\n")).map_err(|e| Error::at(&file, e))?;
    Ok(Written {
        path: file,
        lines: 1,
        unlisted: Vec::new(),
        note: Some(note),
    })
}
