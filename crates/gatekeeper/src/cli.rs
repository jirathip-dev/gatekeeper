use gatekeeper_engine::Error;
use gatekeeper_engine::cache::{self, Ident, Results};
use gatekeeper_model::{
    CacheMode, Config, Event, Filter, GateId, Host, Off, PackId, RunRef, Shard, Trigger,
};
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

/// Which run this is: a lane's, or the GitHub event; a pull request whose head repo is unknown counts as a fork.
pub fn event() -> Event {
    if var("GITHUB_ACTIONS") != "true" {
        return Event::Lane;
    }
    match var("GITHUB_EVENT_NAME").as_str() {
        "pull_request" => {
            let gh_ref = var("GITHUB_REF");
            let number = gh_ref
                .strip_prefix("refs/pull/")
                .and_then(|r| r.split('/').next());
            let Some(number) = number.and_then(|n| n.parse().ok()) else {
                return Event::Other;
            };
            let payload = std::fs::read_to_string(var("GITHUB_EVENT_PATH")).unwrap_or_default();
            let payload: serde_json::Value = serde_json::from_str(&payload).unwrap_or_default();
            let head = payload.pointer("/pull_request/head/repo/full_name");
            let fork = head.and_then(serde_json::Value::as_str) != Some(&var("GITHUB_REPOSITORY"));
            Event::PullRequest { number, fork }
        }
        "push" => Event::Push {
            branch: var("GITHUB_REF_NAME"),
        },
        _ => Event::Other,
    }
}

/// The run's cache mode, and with it on, where records live and what proves a pass: `--results` in CI, the host folder on a lane.
pub fn results(
    config: &Config,
    args: &Args,
    root: &std::path::Path,
) -> Result<(CacheMode, Option<Results>), Error> {
    let event = event();
    let (full, off) = (args.has("--all"), args.has("--no-cache"));
    let mode = cache::mode(
        config.cache.as_ref(),
        &event,
        full,
        off,
        &config.integration,
    );
    let store = match (args.value("--results"), &event) {
        (Some(dir), _) => PathBuf::from(dir),
        (None, Event::Lane) => self::cache().join("results"),
        (None, _) => return Ok((CacheMode::Off(Off::NotConfigured), None)),
    };
    let (CacheMode::Use { read, write }, Some(cfg)) = (&mode, &config.cache) else {
        return Ok((mode, None));
    };
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
    let now = now.map_or(0, |d| d.as_secs());
    let run = var("GITHUB_RUN_ID").parse().ok().map(|run| RunRef {
        repo: var("GITHUB_REPOSITORY"),
        run,
        attempt: var("GITHUB_RUN_ATTEMPT").parse().unwrap_or(1),
    });
    let salt = run.as_ref().map_or_else(
        || (now / 60).to_string(),
        |r| format!("{}-{}", r.run, r.attempt),
    );
    let image = env::var("ImageOS").ok();
    let os = image.map(|i| format!("{i} {}", var("ImageVersion")));
    let results = Results {
        store,
        read: read.clone(),
        write: *write,
        config: cfg.clone(),
        now,
        salt,
        run,
        ident: Ident::current(root, os)?,
        inputs: cache::inputs::read(root)?,
    };
    Ok((mode, Some(results)))
}
