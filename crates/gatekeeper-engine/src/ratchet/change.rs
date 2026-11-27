use crate::ratchet::folder;
use crate::{Result, config, runner};
use std::path::Path;

const LISTS: &str = ".gatekeeper/baseline";

pub(crate) struct Grown {
    pub(crate) list: String,
    pub(crate) gate: String,
    pub(crate) added: u32,
}

pub(crate) fn grown(root: &Path, base: &str) -> Result<Vec<Grown>> {
    let args = ["diff", "--numstat", "--no-renames", base, "--", LISTS];
    let out = runner::ok("git", &args, root)?;
    let each = out.lines().filter_map(|l| {
        let mut cols = l.splitn(3, '\t');
        let added: u32 = cols.next()?.parse().ok()?;
        let list = cols.nth(1)?;
        let stem = Path::new(list).file_stem()?.to_str()?;
        let (pack, name) = stem.split_once('-')?;
        let gate = format!("{pack}/{name}");
        let held =
            added > 0 && pack != "local" && Path::new(list).extension().is_some_and(|x| x == "txt");
        held.then(|| Grown {
            list: list.to_owned(),
            gate,
            added,
        })
    });
    Ok(each.collect())
}

pub(crate) fn pin_moved(root: &Path, base: &str) -> bool {
    let at = format!("{base}:{}", config::PATH);
    let before = runner::ok("git", &["show", &at], root).unwrap_or_default();
    let pin = |t: &str| toml::from_str::<toml::Table>(t).ok()?.get("pin").cloned();
    let now = std::fs::read_to_string(root.join(config::PATH)).unwrap_or_default();
    pin(&before) != pin(&now)
}

pub(crate) fn bodies(root: &Path, base: &str) -> Result<String> {
    let range = format!("{base}..HEAD");
    runner::ok("git", &["log", "--no-merges", "--format=%B", &range], root)
}

const CEILINGS: &str = "ci/ratchet";

pub(crate) struct Touched {
    pub(crate) file: String,
    pub(crate) count: String,
    pub(crate) why: String,
}

pub(crate) fn touched(root: &Path, base: &str) -> Result<Vec<Touched>> {
    let args = [
        "diff",
        "--name-status",
        "--no-renames",
        base,
        "HEAD",
        "--",
        CEILINGS,
    ];
    let diff = runner::ok("git", &args, root)?;
    let mut out = Vec::new();
    for (status, file) in diff.lines().filter_map(|l| l.split_once('\t')) {
        let Some(dir) = file.rsplit_once('/').map(|(d, _)| d) else {
            continue;
        };
        let Some(count) = dir
            .strip_prefix("ci/ratchet/")
            .and_then(|d| d.strip_suffix(".d"))
        else {
            continue;
        };
        let why = if status == "A" {
            let files = folder::read::<u32>(root, dir)?;
            let new = |p: &str| diff.lines().any(|l| l == format!("A\t{p}"));
            let base_lowest = files.iter().filter(|(p, _)| !new(p)).map(|(_, c)| *c).min();
            let mine = files.iter().find(|(p, _)| p == file).map(|(_, c)| *c);
            match (mine, base_lowest) {
                (Some(c), Some(b)) if c >= b => {
                    format!("ceiling {c} is not under the base's ceiling {b}: a ceiling only falls")
                }
                _ => continue,
            }
        } else {
            "a ceiling file is never edited or deleted; lower it with a new file".to_owned()
        };
        out.push(Touched {
            file: file.to_owned(),
            count: count.to_owned(),
            why,
        });
    }
    Ok(out)
}
