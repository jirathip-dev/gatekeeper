use crate::fetch::{self, sha256_hex};
use crate::{BUILD, Result, SOURCE, VERSION, runner};
use gatekeeper_model::{Gate, Step};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Which gatekeeper on which platform, and each probe's output, run once a run.
pub struct Ident {
    pub engine: String,
    pub platform: String,
    pub probes: RefCell<BTreeMap<String, String>>,
}

impl Ident {
    /// This build and the repo-built tools on this host; `os` is the runner image when CI names one, else `uname -sr`.
    #[must_use]
    pub fn current(root: &Path, os: Option<String>, own: &BTreeMap<String, PathBuf>) -> Ident {
        let mut engine = match BUILD {
            Some(build) => format!("release {VERSION} {build}"),
            None => format!("checkout {SOURCE}"),
        };
        for (tool, path) in own {
            let sum = std::fs::read(path).map_or_else(|e| e.to_string(), |b| sha256_hex(&b));
            engine.extend([" ", tool, "=", &sum]);
        }
        let probes = RefCell::default();
        let ident = Ident {
            engine,
            platform: String::new(),
            probes,
        };
        let os = os.unwrap_or_else(|| ident.probe("uname -sr", root));
        let target = fetch::host_target().unwrap_or("other");
        Ident {
            platform: format!("{target} {os}"),
            ..ident
        }
    }

    fn probe(&self, command: &str, root: &Path) -> String {
        if let Some(out) = self.probes.borrow().get(command) {
            return out.clone();
        }
        let out = match runner::capture("sh", &["-c", command], root) {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_owned(),
            Ok(o) => format!("exit {}", o.status.code().unwrap_or(-1)),
            Err(e) => e.0,
        };
        self.probes.borrow_mut().insert(command.into(), out.clone());
        out
    }

    pub(crate) fn versions(&self, gate: &Gate, root: &Path) -> BTreeMap<String, String> {
        let setup = gate.setup.iter().map(|t| t.probe().to_owned());
        let commands = setup.chain(gate.versions.iter().cloned());
        commands
            .map(|c| (self.probe(&c, root), c))
            .map(|(o, c)| (c, o))
            .collect()
    }

    pub(crate) fn tools(&self, gate: &Gate) -> Result<BTreeMap<String, String>> {
        let named = gate.steps.iter().flat_map(|s| match s {
            Step::Tool { tool, .. } => vec![tool.as_str()],
            Step::Run { run } => fetch::named_in(run).collect(),
            Step::Check { .. } => Vec::new(),
        });
        let stub = |t: &str| self.probes.borrow().get(&format!("tool {t}")).cloned();
        named
            .map(|t| Ok((t.to_owned(), stub(t).map_or_else(|| fetch::pin(t), Ok)?)))
            .collect()
    }
}
