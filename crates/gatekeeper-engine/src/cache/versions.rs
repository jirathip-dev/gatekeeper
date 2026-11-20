use crate::fetch::{self, sha256_hex};
use crate::{BUILD, Error, Result, VERSION, runner};
use gatekeeper_model::{Engine, Gate, PlatformId, Step};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;

/// Which gatekeeper on which platform, and each probe's output, run once a run.
pub struct Ident {
    pub engine: Engine,
    pub platform: PlatformId,
    pub probes: RefCell<BTreeMap<String, String>>,
}

impl Ident {
    /// This binary on this host; `os` is the runner image when CI names one, else `uname -sr`.
    ///
    /// # Errors
    pub fn current(root: &Path, os: Option<String>) -> Result<Ident> {
        let engine = if let Some(build) = BUILD {
            Engine::Release {
                version: VERSION.into(),
                build: build.into(),
            }
        } else {
            let exe = std::env::current_exe().map_err(|e| Error(e.to_string()))?;
            let bytes = std::fs::read(&exe).map_err(|e| Error::at(&exe, e))?;
            Engine::Checkout {
                binary: sha256_hex(&bytes),
            }
        };
        let ident = Ident {
            engine,
            platform: PlatformId {
                target: fetch::host_target().unwrap_or("other").into(),
                os: String::new(),
            },
            probes: RefCell::default(),
        };
        let os = os.unwrap_or_else(|| ident.probe("uname -sr", root));
        Ok(Ident {
            platform: PlatformId {
                os,
                ..ident.platform
            },
            ..ident
        })
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

    /// Each toolchain's probe, then the gate's own `versions`, with their output.
    pub(crate) fn versions(&self, gate: &Gate, root: &Path) -> BTreeMap<String, String> {
        let setup = gate.setup.iter().map(|t| t.probe().to_owned());
        setup
            .chain(gate.versions.iter().cloned())
            .map(|c| {
                let out = self.probe(&c, root);
                (c, out)
            })
            .collect()
    }

    /// The pinned asset of each tool the gate's steps name, for this target.
    pub(crate) fn tools(&self, gate: &Gate) -> Result<BTreeMap<String, String>> {
        let mut tools = BTreeMap::new();
        for step in &gate.steps {
            if let Step::Tool { tool, .. } = step {
                let stub = self.probes.borrow().get(&format!("tool {tool}")).cloned();
                let pin = match stub {
                    Some(pin) => pin,
                    None => fetch::pin(tool)?,
                };
                tools.insert(tool.clone(), pin);
            }
        }
        Ok(tools)
    }
}
