use crate::{Error, Result};
use gatekeeper_model::Glob;
use serde::Deserialize;
use std::io::ErrorKind;
use std::path::Path;

pub(crate) const PATH: &str = ".drover/arch.toml";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Arch {
    #[serde(default)]
    pub layer: Vec<Layer>,
    #[serde(default)]
    pub ban: Vec<Ban>,
    #[serde(default)]
    pub size: Vec<Size>,
    pub baseline: Option<String>,
    pub comments: Option<Comments>,
    docs: Option<toml::Table>,
    #[serde(default, rename = "frozen")]
    _frozen: Vec<Glob>,
    #[serde(default)]
    pub collect: Vec<Collect>,
    #[serde(default)]
    pub generated: Vec<Glob>,
    pub generate: Option<String>,
    #[serde(default)]
    pub shared: Vec<Glob>,
}

impl Arch {
    pub(crate) fn named(&self) -> Vec<String> {
        let bans = self.ban.iter().flat_map(|b| &b.paths);
        let all = bans.chain(self.size.iter().flat_map(|s| &s.paths));
        all.map(ToString::to_string).collect()
    }

    pub(crate) fn baseline(&self) -> &str {
        self.baseline
            .as_deref()
            .unwrap_or(".drover/arch-baseline.txt")
    }
}

#[derive(Deserialize)]
pub(crate) struct Frozen {
    #[serde(default)]
    pub frozen: Vec<Glob>,
}

#[derive(Deserialize)]
struct Generated {
    #[serde(default)]
    generated: Vec<Glob>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Collect {
    dir: String,
    glob: Option<String>,
}

impl Collect {
    pub(crate) fn entries(&self) -> String {
        let glob = self.glob.as_deref().unwrap_or("*");
        format!("{}/{glob}", self.dir.trim_end_matches('/'))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Comments {
    pub policy: Policy,
    #[serde(default)]
    pub except: Vec<Glob>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Policy {
    On,
    Off,
    Strict,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Layer {
    pub name: String,
    pub paths: Vec<Glob>,
    #[serde(default)]
    pub mods: Vec<String>,
    #[serde(default)]
    pub may_use: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<Glob>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ban {
    pub pattern: String,
    pub paths: Vec<Glob>,
    #[serde(default)]
    pub except: Vec<Glob>,
    pub reason: String,
    #[serde(default)]
    pub comments: bool,
    #[serde(default)]
    pub ignore_case: bool,
}

impl Ban {
    pub(crate) fn hits(&self, line: &str) -> bool {
        if self.ignore_case {
            line.to_lowercase().contains(&self.pattern.to_lowercase())
        } else {
            line.contains(&self.pattern)
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Size {
    pub paths: Vec<Glob>,
    pub max_file_lines: Option<usize>,
    pub max_fn_lines: Option<usize>,
    #[serde(default)]
    pub exclude: Vec<Glob>,
}

pub(crate) fn parse(text: &str, at: &str) -> Result<Arch> {
    let a: Arch = toml::from_str(text).map_err(|e| Error(format!("parsing {at}: {e}")))?;
    if a.docs.is_some() {
        return Err(Error(format!(
            "{at}: [docs] moved to .gatekeeper/config.toml: move `watch` and `skip`, drop `paths`"
        )));
    }
    if !a.generated.is_empty() && a.generate.is_none() {
        return Err(Error(format!(
            "{at}: `generated` needs `generate`, the command that writes them"
        )));
    }
    Ok(a)
}

pub(crate) fn load(root: &Path) -> Result<Option<Arch>> {
    let path = root.join(PATH);
    match std::fs::read_to_string(&path) {
        Ok(text) => parse(&text, PATH).map(Some),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::at(&path, e)),
    }
}

pub(crate) fn generated(root: &Path) -> Result<Vec<Glob>> {
    let path = root.join(PATH);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(toml::from_str::<Generated>(&text).map_or_else(|_| vec![], |g| g.generated)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(Error::at(&path, e)),
    }
}
