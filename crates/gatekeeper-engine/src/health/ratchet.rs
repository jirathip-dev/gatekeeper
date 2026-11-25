use crate::baseline::Written;
use crate::{Error, Result, config};
use gatekeeper_model::{Ceiling, Date, Lines};
use std::fs;
use std::path::Path;

pub(crate) const DIR: &str = "ci/tests/ratio.d";

pub(crate) fn read(root: &Path) -> Result<Vec<(String, Ceiling)>> {
    let mut out = Vec::new();
    for e in fs::read_dir(root.join(DIR)).into_iter().flatten().flatten() {
        let path = e.path();
        if path.extension().is_none_or(|x| x != "toml") {
            continue;
        }
        let at = format!("{DIR}/{}", e.file_name().to_string_lossy());
        let table = config::read_toml(&path)?;
        let value = table.get("ceiling").map(ToString::to_string);
        let ceiling = value
            .unwrap_or_default()
            .parse()
            .map_err(|e| Error(format!("{at}: {e}")))?;
        out.push((at, ceiling));
    }
    out.sort();
    Ok(out)
}

pub(crate) fn lowest(files: &[(String, Ceiling)]) -> Option<&(String, Ceiling)> {
    files.iter().min_by_key(|(_, c)| *c)
}

pub(crate) fn write(root: &Path, today: Date, lines: Lines) -> Result<Written> {
    let files = read(root)?;
    let fits = Ceiling::fits(lines);
    let dir = root.join(DIR);
    let note = match lowest(&files) {
        Some((at, c)) if fits >= *c => {
            return Ok(Written {
                path: dir,
                lines: 0,
                unlisted: Vec::new(),
                note: Some(format!(
                    "the ratio fits {fits}, the ceiling is {c} ({at}): nothing written"
                )),
            });
        }
        Some((_, c)) => format!("ceiling {fits} (was {c})"),
        None => format!("ceiling {fits} (the first)"),
    };
    fs::create_dir_all(&dir).map_err(|e| Error::at(&dir, e))?;
    let path = dir.join(format!("{today}-{}.toml", fits.hundredths()));
    fs::write(&path, format!("ceiling = {fits}\n")).map_err(|e| Error::at(&path, e))?;
    Ok(Written {
        path,
        lines: 1,
        unlisted: Vec::new(),
        note: Some(note),
    })
}
