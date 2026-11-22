use crate::{Glob, Graph, Widened};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// `[select]` in a gate file; its presence makes the gate run over selected test files.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Select {
    pub vitest: String,
    #[serde(default)]
    pub always: Vec<Glob>,
    pub full: Vec<Glob>,
}

/// One changed file of `git diff --name-status --no-renames <base>` or the untracked files.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub path: String,
    pub kind: ChangeKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TestFile(pub String);

/// Everything the rules read: `universe` and `closure` from vitest, the rest from git and the tree.
#[derive(Clone, Copy, Debug)]
pub struct Evidence<'a> {
    pub universe: &'a BTreeSet<TestFile>,
    pub closure: &'a BTreeSet<TestFile>,
    pub texts: &'a BTreeMap<TestFile, String>,
    pub graph: &'a Graph,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    Full(Full),
    Files(Files),
}

impl Selection {
    /// The selected files with their reasons, or `Full(Empty)` when there are none.
    #[must_use]
    pub fn of(files: BTreeMap<TestFile, BTreeSet<Why>>) -> Selection {
        if files.is_empty() {
            Selection::Full(Full::Empty)
        } else {
            Selection::Files(Files(files))
        }
    }
}

/// A selection that is never empty: only `Selection::of` builds one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Files(BTreeMap<TestFile, BTreeSet<Why>>);

impl Files {
    #[must_use]
    pub fn reasons(&self) -> &BTreeMap<TestFile, BTreeSet<Why>> {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Full {
    NotAChange(Widened),
    File(String),
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Why {
    Closure,
    Always(Always),
    Reads(Read),
    NamesProject(String),
    Widened(Widen),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Always {
    Path,
    NoProject,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Read {
    Path(String),
    Basename(String),
    Walks(Walk),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Walk {
    Project(String),
    Dir(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Widen {
    FormerProject(String),
    Specifier(String),
}
