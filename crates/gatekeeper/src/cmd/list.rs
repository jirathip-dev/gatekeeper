use crate::cli::{self, Args, say};
use gatekeeper_engine::cache::key::{self, Keyed};
use gatekeeper_engine::cache::{Ident, inputs};
use gatekeeper_engine::{Error, config, plan};
use gatekeeper_model::{Decision, Plan};
use std::process::ExitCode;

fn planned(args: &Args) -> Result<(Plan, Vec<String>), Error> {
    let root = cli::root()?;
    let config = config::load(&root)?;
    let host = cli::host();
    let plan = plan::repo(&config, &root, &|_| host, &args.filter()?)?;
    let mut keys = Vec::new();
    if config.cache.is_some() {
        let (ident, inputs) = (Ident::current(&root, None), inputs::read(&root)?);
        for job in &plan.jobs {
            for p in &job.gates {
                let at = (job.shard, &p.projects[..]);
                keys.push(match key::keyed(&p.gate, at, &ident, &inputs, &root)? {
                    Keyed::Key(key, _) => {
                        let short = key.0.get(..12).unwrap_or_default();
                        format!("  key {short}… {} files", inputs.of(&p.gate).0)
                    }
                    Keyed::No(why) => format!("  not cached: {why}"),
                });
            }
        }
    }
    Ok((plan, keys))
}

/// `list [--json]`: the planned gates with their budgets, and with `[cache]` each one's key or why it has none.
pub fn run(args: &[String]) -> ExitCode {
    let plan = match Args::parse(args, &["--pack", "--gate"], &["--json"]).and_then(|a| {
        let json = a.has("--json");
        planned(&a).map(|p| (p, json))
    }) {
        Ok(ok) => ok,
        Err(e) => return cli::usage_error(&e),
    };
    let ((plan, keys), json) = plan;
    if json {
        return match serde_json::to_string_pretty(&plan) {
            Ok(text) => {
                say(&text);
                ExitCode::SUCCESS
            }
            Err(e) => cli::usage_error(&e),
        };
    }
    let mut keys = keys.into_iter();
    for p in plan.jobs.iter().flat_map(|j| &j.gates) {
        let skip = match p.decision {
            Decision::Run => String::new(),
            Decision::Skip(why) => format!("  (skip: {why})"),
        };
        let key = keys.next().unwrap_or_default();
        say(&format!(
            "{:<18} {:>4}s  {}{skip}{key}",
            p.gate.id, p.gate.budget.0, p.gate.about
        ));
    }
    ExitCode::SUCCESS
}
