use super::config::{self, PATH};
use super::scan::{self, Scan};
use crate::baseline::Written;
use crate::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Default)]
pub(crate) struct Listed {
    pub sizes: BTreeMap<String, usize>,
    pub crossings: BTreeSet<String>,
}

fn parse(text: &str) -> Listed {
    let lines = text.lines().map(str::trim);
    let crossings = lines.clone().filter(|l| l.contains(" -> "));
    let sizes = lines.filter_map(|l| {
        let (k, n) = l.rsplit_once(' ')?;
        Some((k.to_owned(), n.parse().ok()?))
    });
    Listed {
        sizes: sizes.collect(),
        crossings: crossings.map(str::to_owned).collect(),
    }
}

pub(crate) fn read(root: &Path, file: &str) -> Listed {
    parse(&fs::read_to_string(root.join(file)).unwrap_or_default())
}

fn shrink(old: &Listed, now: Scan) -> (Listed, Vec<String>) {
    let (mut kept, mut unlisted) = (Listed::default(), vec![]);
    for (k, n) in now.sizes {
        match old.sizes.get(&k) {
            Some(&was) => {
                if n > was {
                    unlisted.push(format!("{k} {n}"));
                }
                kept.sizes.insert(k, n.min(was));
            }
            None => unlisted.push(format!("{k} {n}")),
        }
    }
    for k in now.crossed {
        if old.crossings.contains(&k) {
            kept.crossings.insert(k);
        } else {
            unlisted.push(k);
        }
    }
    (kept, unlisted)
}

/// `baseline core/arch`: the size offenders and layer crossings of today; once written, only shrinks.
pub fn write(root: &Path) -> Result<Written> {
    let a = config::load(root)?.ok_or_else(|| Error(format!("no {PATH}: no arch baseline")))?;
    let path = root.join(a.baseline());
    let now = scan::scan(root, &a, &Listed::default());
    let (kept, unlisted) = match fs::read_to_string(&path) {
        Ok(text) => shrink(&parse(&text), now),
        Err(_) => (
            Listed {
                sizes: now.sizes,
                crossings: now.crossed,
            },
            vec![],
        ),
    };
    let lines = kept.sizes.len() + kept.crossings.len();
    if lines == 0 {
        if path.exists() {
            fs::remove_file(&path).map_err(|e| Error::at(&path, e))?;
        }
    } else {
        let sizes = kept.sizes.iter().map(|(k, n)| format!("{k} {n}\n"));
        let text: String = sizes
            .chain(kept.crossings.iter().map(|k| format!("{k}\n")))
            .collect();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| Error::at(dir, e))?;
        }
        fs::write(&path, text).map_err(|e| Error::at(&path, e))?;
    }
    Ok(Written {
        path,
        lines,
        unlisted,
        note: None,
    })
}
