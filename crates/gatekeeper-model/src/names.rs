use std::fmt;

/// The fleet's private names for one run; no Display or Serialize, and Debug prints only the count.
pub struct PrivateNames(Vec<String>);

impl PrivateNames {
    /// One name a line, trimmed; `None` when no name is left.
    #[must_use]
    pub fn parse(text: &str) -> Option<PrivateNames> {
        let names: Vec<String> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect();
        (!names.is_empty()).then_some(PrivateNames(names))
    }

    /// How many names the list holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Never true: a list is built only with a name in it.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for PrivateNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrivateNames({} redacted)", self.0.len())
    }
}
