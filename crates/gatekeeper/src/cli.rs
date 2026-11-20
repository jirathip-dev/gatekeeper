use gatekeeper_engine::Error;
use gatekeeper_model::{Filter, GateId, Host, PackId, Shard, Trigger};
use std::env;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

pub fn say(line: &str) {
    writeln!(std::io::stdout(), "{line}").ok();
}

pub fn usage_error(e: &impl std::fmt::Display) -> ExitCode {
    writeln!(std::io::stderr(), "gatekeeper: {e}").ok();
    ExitCode::from(2)
}

pub fn exit(failed: bool) -> ExitCode {
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Parsed `--flag value` pairs and bare `--switch`es; anything else is a usage error.
pub struct Args {
    values: Vec<(String, String)>,
    switches: Vec<String>,
}

impl Args {
    pub fn parse(args: &[String], valued: &[&str], switches: &[&str]) -> Result<Args, Error> {
        let mut out = Args {
            values: Vec::new(),
            switches: Vec::new(),
        };
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            if valued.contains(&arg.as_str()) {
                let v = it
                    .next()
                    .ok_or_else(|| Error(format!("{arg} needs a value")))?;
                out.values.push((arg.clone(), v.clone()));
            } else if switches.contains(&arg.as_str()) {
                out.switches.push(arg.clone());
            } else {
                return Err(Error(format!("unexpected argument `{arg}`")));
            }
        }
        Ok(out)
    }

    pub fn value(&self, flag: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(f, _)| f == flag)
            .map(|(_, v)| v.as_str())
    }

    pub fn values<'a>(&'a self, flag: &'a str) -> impl Iterator<Item = &'a str> {
        self.values
            .iter()
            .filter(move |(f, _)| f == flag)
            .map(|(_, v)| v.as_str())
    }

    pub fn has(&self, switch: &str) -> bool {
        self.switches.iter().any(|s| s == switch)
    }

    pub fn filter(&self) -> Result<Filter, Error> {
        let packs = self
            .values("--pack")
            .flat_map(str::split_whitespace)
            .map(|p| PackId::parse(p).ok_or_else(|| Error(format!("--pack {p}: no such pack"))))
            .collect::<Result<_, _>>()?;
        let gate = self
            .value("--gate")
            .map(|g| GateId::try_from(g.to_owned()).map_err(Error))
            .transpose()?;
        let shard = self
            .value("--shard")
            .map(|s| Shard::try_from(s.to_owned()).map_err(Error))
            .transpose()?;
        Ok(Filter {
            packs,
            gate,
            leg: self.value("--leg").map(str::to_owned),
            shard,
            trigger: Trigger::default(),
        })
    }
}

pub fn root() -> Result<PathBuf, Error> {
    env::current_dir().map_err(|e| Error(format!("current directory: {e}")))
}

fn var(name: &str) -> String {
    env::var(name).unwrap_or_default()
}

/// The host kind, from the runner's environment: a GitHub-hosted runner, a self-hosted one, or a lane's host.
pub fn host() -> Host {
    if var("GITHUB_ACTIONS") != "true" {
        return Host::Lane;
    }
    if var("RUNNER_ENVIRONMENT") == "self-hosted" {
        return Host::SelfHosted;
    }
    Host::GithubHosted {
        warm_cache: var("GATEKEEPER_CACHE_HIT") == "true",
    }
}

/// When the job started: `GATEKEEPER_JOB_START`, epoch seconds, stamped by the job's first step or the wrapper.
pub fn job_start() -> Option<SystemTime> {
    let secs = var("GATEKEEPER_JOB_START").trim().parse().ok()?;
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(secs))
}

pub fn cache() -> PathBuf {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(env::temp_dir);
    base.join("gatekeeper")
}
