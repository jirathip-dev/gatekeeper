use crate::Excuses;
use serde::Deserialize;
use std::fmt;

/// `[duplication]`: every key optional, the gate on without the table.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DuplicationConfig {
    pub ignore: Excuses,
}

/// The smallest clone jscpd reports, so the headroom under a ceiling never holds a new one.
pub const DUPLICATION_STEP: u32 = 10;

/// What jscpd never reads, whatever the repo says.
pub const DUPLICATION_IGNORED: [&str; 9] = [
    "**/fixtures/**",
    "**/__fixtures__/**",
    "**/testdata/**",
    "**/__snapshots__/**",
    "**/golden/**",
    "**/vendor/**",
    "**/third_party/**",
    "**/*.d.ts",
    "**/*.min.*",
];

/// A counted record's name: its ceiling folder is `ci/ratchet/<name>.d`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountName(String);

impl CountName {
    #[must_use]
    pub fn duplication() -> CountName {
        CountName("duplication".into())
    }

    #[must_use]
    pub fn dir(&self) -> String {
        format!("ci/ratchet/{}.d", self.0)
    }
}

impl fmt::Display for CountName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One measured count: its record, the number, the step a ceiling moves by, the unit and the gate that writes it.
#[derive(Clone, Debug)]
pub struct Counted {
    pub name: CountName,
    pub n: u32,
    pub step: u32,
    pub unit: &'static str,
    pub gate: &'static str,
}
