use crate::cli::{self, Args, say};
use gatekeeper_engine::{Error, config, plan};
use gatekeeper_model::{Decision, Plan};
use std::process::ExitCode;

fn planned(args: &Args) -> Result<Plan, Error> {
    let root = cli::root()?;
    let config = config::load(&root)?;
    let host = cli::host();
    plan::repo(&config, &root, &|_| host, &args.filter()?)
}

/// `list [--json]`: the planned gates with their budgets.
pub fn run(args: &[String]) -> ExitCode {
    let plan = match Args::parse(args, &["--pack", "--gate"], &["--json"]).and_then(|a| {
        let json = a.has("--json");
        planned(&a).map(|p| (p, json))
    }) {
        Ok(ok) => ok,
        Err(e) => return cli::usage_error(&e),
    };
    let (plan, json) = plan;
    if json {
        return match serde_json::to_string_pretty(&plan) {
            Ok(text) => {
                say(&text);
                ExitCode::SUCCESS
            }
            Err(e) => cli::usage_error(&e),
        };
    }
    for p in plan.jobs.iter().flat_map(|j| &j.gates) {
        let skip = match p.decision {
            Decision::Run => String::new(),
            Decision::Skip(why) => format!("  (skip: {why})"),
        };
        say(&format!(
            "{:<18} {:>4}s  {}{skip}",
            p.gate.id, p.gate.budget.0, p.gate.about
        ));
    }
    ExitCode::SUCCESS
}
