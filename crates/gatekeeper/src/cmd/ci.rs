use crate::cli::{self, Args, say};
use gatekeeper_engine::{Error, ci, config, packs, plan};
use gatekeeper_model::{Pin, Plan};
use serde_json::{Map, Value};
use std::path::Path;
use std::process::ExitCode;

const RUNNERS: &str = r#"{"linux": "ubuntu-latest", "macos": "macos-latest"}"#;

fn planned(args: &Args) -> Result<(Plan, bool), Error> {
    let root = cli::root()?;
    let config = config::load(&root)?;
    let gates = packs::configured(&config, &root)?;
    let plan = plan::plan(&config, gates, cli::host(), &args.filter()?)?;
    Ok((plan, matches!(config.pin, Pin::Checkout(_))))
}

fn plan_matrix(args: &Args) -> Result<String, Error> {
    let (plan, checkout) = planned(args)?;
    let runners: Map<String, Value> =
        serde_json::from_str(args.value("--runners").unwrap_or(RUNNERS))
            .map_err(|e| Error(format!("--runners: {e}")))?;
    Ok(format!("matrix={}", ci::matrix(&plan, &runners, checkout)?))
}

fn done(args: &Args) -> Result<bool, Error> {
    let (plan, _) = planned(args)?;
    let reports = args
        .value("--reports")
        .ok_or_else(|| Error("ci done needs --reports DIR".into()))?;
    let (table, problems) = ci::done(&plan, Path::new(reports))?;
    say(&table);
    for p in &problems {
        say(&format!("FAIL {p}"));
    }
    Ok(!problems.is_empty())
}

/// `ci plan [--pack P] [--runners JSON]` prints the leg matrix; `ci done --reports DIR` merges the job reports.
pub fn run(args: &[String]) -> ExitCode {
    let Some((sub, rest)) = args.split_first() else {
        return cli::usage_error(&"ci: plan or done");
    };
    let parsed = Args::parse(rest, &["--pack", "--runners", "--reports"], &[]);
    match sub.as_str() {
        "plan" => match parsed.and_then(|a| plan_matrix(&a)) {
            Ok(line) => {
                say(&line);
                ExitCode::SUCCESS
            }
            Err(e) => cli::usage_error(&e),
        },
        "done" => match parsed.and_then(|a| done(&a)) {
            Ok(failed) => cli::exit(failed),
            Err(e) => cli::usage_error(&e),
        },
        other => cli::usage_error(&format!("ci {other}: plan or done")),
    }
}
