use crate::{Glob, Millis, Secs};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;

/// `[tests]`: every key has a default, so a repo with no table is checked too.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsConfig {
    pub budget_secs: Secs,
    pub test: Vec<Glob>,
    pub src: Vec<Glob>,
    pub exclude: Vec<Glob>,
    pub soak: Vec<Glob>,
}

impl Default for TestsConfig {
    fn default() -> Self {
        TestsConfig {
            budget_secs: Secs(2),
            test: Vec::new(),
            src: Vec::new(),
            exclude: Vec::new(),
            soak: Vec::new(),
        }
    }
}

/// Counted lines: test and src.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lines {
    pub test: u64,
    pub src: u64,
}

impl std::ops::Add for Lines {
    type Output = Lines;

    fn add(self, o: Lines) -> Lines {
        Lines {
            test: self.test + o.test,
            src: self.src + o.src,
        }
    }
}

impl Lines {
    /// The ratio in hundredths, rounded up.
    #[must_use]
    pub fn ratio(self) -> u64 {
        (self.test * 100).div_ceil(self.src.max(1))
    }
}

/// A ratio ceiling in hundredths: 130 is 1.30; on the 0.05 grid and never under 1.00.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ceiling(u64);

impl Ceiling {
    pub const STEP: u64 = 5;

    /// The ratio rounded up to the grid, at least 1.00.
    #[must_use]
    pub fn fits(lines: Lines) -> Ceiling {
        Ceiling((lines.ratio().div_ceil(Self::STEP) * Self::STEP).max(100))
    }

    #[must_use]
    pub fn holds(self, lines: Lines) -> bool {
        lines.test * 100 <= self.0 * lines.src
    }

    #[must_use]
    pub fn hundredths(self) -> u64 {
        self.0
    }
}

impl FromStr for Ceiling {
    type Err = String;

    fn from_str(s: &str) -> Result<Ceiling, String> {
        let (whole, frac) = s.split_once('.').unwrap_or((s, "0"));
        let frac = format!("{frac:0<2}");
        let parsed = whole.parse::<u64>().ok().zip(frac.parse::<u64>().ok());
        match parsed {
            Some((w, f)) if frac.len() == 2 && w * 100 + f >= 100 && f % Self::STEP == 0 => {
                Ok(Ceiling(w * 100 + f))
            }
            _ => Err(format!(
                "`{s}` is no ceiling: a multiple of 0.05, at least 1.00"
            )),
        }
    }
}

impl fmt::Display for Ceiling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, self.0 % 100)
    }
}

/// A day in UTC, `2026-11-25`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    days: i64,
}

impl Date {
    #[must_use]
    pub fn utc(at: SystemTime) -> Date {
        let secs = at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        Date {
            days: i64::try_from(secs.as_secs() / 86_400).unwrap_or(0),
        }
    }

    fn civil(self) -> (i64, i64, i64) {
        let z = self.days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (yoe + era * 400 + i64::from(m <= 2), m, d)
    }

    fn of(y: i64, m: i64, d: i64) -> Date {
        let y = y - i64::from(m <= 2);
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let mp = if m > 2 { m - 3 } else { m + 9 };
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Date {
            days: era * 146_097 + doe - 719_468,
        }
    }
}

impl FromStr for Date {
    type Err = String;

    fn from_str(s: &str) -> Result<Date, String> {
        let parts: Vec<Option<i64>> = s.split('-').map(|p| p.parse().ok()).collect();
        let date = match parts[..] {
            [Some(y), Some(m), Some(d)] => Some(Date::of(y, m, d)),
            _ => None,
        };
        date.filter(|d| d.to_string() == s)
            .ok_or_else(|| format!("`{s}` is no date (YYYY-MM-DD)"))
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (y, m, d) = self.civil();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

/// `<entry>/health.json`: what the last push to the integration branch measured.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Health {
    pub schema: u32,
    pub legs: BTreeMap<String, Millis>,
}

/// `[timed]`: the gate's line runs tests and leaves a `JUnit` file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timed {
    pub runner: Runner,
    pub junit: String,
    #[serde(default = "here")]
    pub root: String,
}

fn here() -> String {
    ".".into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runner {
    Nextest,
    Vitest,
}

/// `<classname> <name>` as the `JUnit` file gives them.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TestId(pub String);

impl fmt::Display for TestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One `testcase` of a `JUnit` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    pub class: String,
    pub name: String,
    pub took: Millis,
    pub ended: Ended,
}

impl Case {
    #[must_use]
    pub fn id(&self) -> TestId {
        TestId(format!("{} {}", self.class, self.name))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    Passed,
    Failed,
    Skipped,
}

/// A changed piece of test code: a Rust test function, the test code of a Rust file, or a whole TS test file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Changed {
    RustFn {
        binary: String,
        module: String,
        name: String,
    },
    RustFile {
        binary: String,
        module: String,
    },
    TsFile(String),
}

/// What started a timed gate's run: it picks the rerun set and whether over budget fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunKind {
    PullRequest,
    Lane,
    Integration,
}
