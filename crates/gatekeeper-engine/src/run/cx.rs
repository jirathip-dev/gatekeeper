use crate::cache::Results;
use crate::{Error, Result, fetch, runner};
use gatekeeper_model::{
    Date, DocsConfig, Host, Platform, PrivateNames, RustPack, SwiftPack, TestsConfig, TsPack,
};
use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What a built-in check reads: the repo, its scratch folder, the tool cache, the packs' tables, the base, the PR's values; where it notes a summary.
pub struct CheckCx<'a> {
    pub root: &'a Path,
    pub scratch: &'a Path,
    pub cache: &'a Path,
    pub rust: &'a RustPack,
    pub ts: Option<&'a TsPack>,
    pub swift: Option<&'a SwiftPack>,
    pub docs: &'a DocsConfig,
    pub tools: &'a BTreeMap<String, PathBuf>,
    pub base: Option<&'a str>,
    pub names: Option<&'a PrivateNames>,
    pub head: Option<&'a str>,
    pub pr_body: Option<&'a Path>,
    pub integration: &'a str,
    pub note: &'a OnceCell<String>,
    pub tests: &'a TestsConfig,
    pub today: Date,
}

impl CheckCx<'_> {
    /// # Errors
    pub fn ts(&self) -> Result<&TsPack> {
        self.ts.ok_or_else(|| {
            Error("the ts pack's checks need [ts] in .gatekeeper/config.toml".into())
        })
    }

    /// # Errors
    pub fn swift(&self) -> Result<&SwiftPack> {
        self.swift.ok_or_else(|| {
            Error("the swift pack's checks need [swift] in .gatekeeper/config.toml".into())
        })
    }

    /// # Errors
    pub fn tool(&self, name: &str) -> Result<PathBuf> {
        fetch::resolve(name, self.cache, self.tools)
    }
}

/// Where and how a plan runs; `out` takes the logs and `report.json`.
#[non_exhaustive]
pub struct RunCtx {
    pub root: PathBuf,
    pub out: PathBuf,
    pub cache: PathBuf,
    pub base: Option<String>,
    pub pr_body: Option<PathBuf>,
    /// `DROVER_PRIVATE_NAMES`, read once by the cli; a fixture case's from its `env`.
    pub names: Option<PrivateNames>,
    /// `GATEKEEPER_HEAD`, read once by the cli; a fixture case's commit.
    pub head: Option<String>,
    pub integration: String,
    pub host: Host,
    pub job_start: Option<SystemTime>,
    pub rust: RustPack,
    /// The reports and kept files of the leg an `after` job waits for, downloaded in CI.
    pub after_reports: Option<PathBuf>,
    pub ts: Option<TsPack>,
    /// `None` when this run neither reads nor writes the result cache.
    pub results: Option<Results>,
    pub swift: Option<SwiftPack>,
    pub docs: DocsConfig,
    /// `[tools]`: the tools this repo builds itself, by absolute path.
    pub tools: BTreeMap<String, PathBuf>,
    /// The platform this host is: a gate of another fails `needs <platform>`.
    pub platform: Platform,
    /// Set for every step: a fixture case's `env`.
    pub env: Vec<(String, String)>,
    /// `--timings`: the recorded times a sharded `[select]` gate splits by, one `<gate>.json` each.
    pub timings: Option<PathBuf>,
    pub tests: TestsConfig,
    /// `GATEKEEPER_TODAY` or the clock, in UTC, read once by the cli; a fixture case's from its `env`.
    pub today: Date,
}

impl RunCtx {
    /// What a built-in check of this run reads.
    #[must_use]
    pub fn check<'a>(&'a self, scratch: &'a Path, note: &'a OnceCell<String>) -> CheckCx<'a> {
        CheckCx {
            root: &self.root,
            scratch,
            cache: &self.cache,
            rust: &self.rust,
            ts: self.ts.as_ref(),
            swift: self.swift.as_ref(),
            docs: &self.docs,
            tools: &self.tools,
            base: self.base.as_deref(),
            names: self.names.as_ref(),
            head: self.head.as_deref(),
            pr_body: self.pr_body.as_deref(),
            integration: &self.integration,
            note,
            tests: &self.tests,
            today: self.today,
        }
    }

    /// A lane's plain run of the repo at `root`: no base, no result cache, this host's platform.
    #[must_use]
    pub fn lane(root: &Path, cache: &Path) -> RunCtx {
        RunCtx {
            root: root.to_path_buf(),
            out: out_dir(root),
            cache: cache.to_path_buf(),
            base: None,
            pr_body: None,
            names: None,
            head: None,
            integration: String::new(),
            host: Host::Lane,
            job_start: None,
            rust: RustPack::default(),
            after_reports: None,
            ts: None,
            results: None,
            swift: None,
            docs: DocsConfig::default(),
            tools: BTreeMap::new(),
            platform: platform(),
            env: Vec::new(),
            timings: None,
            tests: TestsConfig::default(),
            today: Date::utc(SystemTime::now()),
        }
    }
}

/// The platform this binary runs on.
#[must_use]
pub fn platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::Macos
    } else {
        Platform::Linux
    }
}

#[must_use]
pub fn out_dir(root: &Path) -> PathBuf {
    root.join("target/gatekeeper")
}

/// The merge base with `origin/<integration>`, when that branch is fetched.
#[must_use]
pub fn default_base(root: &Path, integration: &str) -> Option<String> {
    let branch = format!("origin/{integration}");
    let out = runner::ok("git", &["merge-base", &branch, "HEAD"], root).ok()?;
    Some(out.trim().to_owned())
}
