use crate::cli::{self, Args, say};
use gatekeeper_engine::cache::key::{self, Keyed};
use gatekeeper_engine::cache::{self, Ident, inputs};
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
        let (ident, inputs) = (
            Ident::current(&root, None, &config::tools(&config, &root)),
            inputs::read(&root)?,
        );
        let ws = &plan.workspace;
        for job in &plan.jobs {
            for p in &job.gates {
                let each = cache::per_project(&p.gate, job.shard);
                let units: Vec<&[String]> = if each {
                    p.projects.iter().map(std::slice::from_ref).collect()
                } else {
                    vec![&p.projects[..]]
                };
                let mut line = Vec::new();
                for unit in units {
                    let name = if each { unit.concat() } else { String::new() };
                    let at = (job.shard, unit);
                    let key = match key::keyed(&p.gate, at, &ident, (&inputs, ws), &root)? {
                        Keyed::Key(key, _) => {
                            let short = key.0.get(..12).unwrap_or_default();
                            let files = inputs.of(&p.gate, unit, ws).0;
                            format!("  key {short}… {files} files")
                        }
                        Keyed::No(why) => format!("  not cached: {why}"),
                    };
                    line.push(if each {
                        format!("\n    {name}{key}")
                    } else {
                        key
                    });
                }
                keys.push(line.concat());
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
