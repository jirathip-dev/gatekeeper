use serde::{Deserialize, Serialize};
use std::fmt;

/// A repo-relative path pattern: `**` any number of folders, `*` and `?` inside one name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Glob(String);

impl Glob {
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        let pattern: Vec<&str> = self.0.split('/').collect();
        let path: Vec<&str> = path.split('/').collect();
        folders(&pattern, &path)
    }

    /// The folders before the first wildcard or the file name: where a search for matches starts.
    #[must_use]
    pub fn dir(&self) -> String {
        let parts: Vec<&str> = self.0.split('/').collect();
        let fixed = parts.iter().take_while(|p| !p.contains(['*', '?']));
        let fixed: Vec<&str> = fixed.take(parts.len().saturating_sub(1)).copied().collect();
        fixed.join("/")
    }
}

fn folders(pattern: &[&str], path: &[&str]) -> bool {
    match (pattern.split_first(), path.split_first()) {
        (None, None) => true,
        (Some((&"**", rest)), _) => {
            folders(rest, path) || path.split_first().is_some_and(|(_, p)| folders(pattern, p))
        }
        (Some((want, rest)), Some((name, tail))) => {
            one(want.as_bytes(), name.as_bytes()) && folders(rest, tail)
        }
        _ => false,
    }
}

fn one(want: &[u8], name: &[u8]) -> bool {
    match (want.split_first(), name.split_first()) {
        (None, None) => true,
        (Some((b'*', rest)), _) => {
            one(rest, name) || name.split_first().is_some_and(|(_, n)| one(want, n))
        }
        (Some((b'?', rest)), Some((_, tail))) => one(rest, tail),
        (Some((w, rest)), Some((n, tail))) => w == n && one(rest, tail),
        _ => false,
    }
}

impl TryFrom<String> for Glob {
    type Error = String;

    fn try_from(s: String) -> Result<Glob, String> {
        let bad = s.contains('\\') || s.split('/').any(|p| p.is_empty() || p == "." || p == "..");
        if bad {
            return Err(format!("`{s}` is not a repo-relative glob"));
        }
        Ok(Glob(s))
    }
}

impl From<Glob> for String {
    fn from(g: Glob) -> String {
        g.0
    }
}

impl fmt::Display for Glob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
