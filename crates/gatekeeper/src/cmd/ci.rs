use crate::cli::{self, Args, say};
use gatekeeper_engine::{Error, cache, ci, config, plan};
use gatekeeper_model::{Config, Pin, Plan};
use serde_json::{Map, Value};
use std::path::Path;
use std::process::ExitCode;

const RUNNERS: &str = r#"{"linux": "ubuntu-latest", "macos": "macos-latest"}"#;

fn planned(args: &Args) -> Result<(Plan, Map<String, Value>, Config), Error> {
    let root = cli::root()?;
    let config = config::load(&root)?;
    let runners: Map<String, Value> =
        serde_json::from_str(args.value("--runners").unwrap_or(RUNNERS))
            .map_err(|e| Error(format!("--runners: {e}")))?;
    let hosts = |p| ci::host(&runners, p);
    let plan = plan::repo(&config, &root, &hosts, &args.filter()?)?;
    Ok((plan, runners, config))
}

fn plan_matrix(args: &Args) -> Result<String, Error> {
    let (plan, runners, config) = planned(args)?;
    let full = args.has("--all");
    let mode = cache::mode(
        config.cache.as_ref(),
        &cli::event(),
        full,
        false,
        &config.integration,
    );
    let checkout = matches!(config.pin, Pin::Checkout(_));
    ci::matrix(&plan, &runners, checkout, &mode)
}

fn done(args: &Args) -> Result<bool, Error> {
    let (plan, _, _) = planned(args)?;
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

/// `ci plan [--pack P] [--runners JSON] [--all]` prints the job matrices; `ci done --reports DIR [--runners JSON]` merges the reports.
pub fn run(args: &[String]) -> ExitCode {
    let Some((sub, rest)) = args.split_first() else {
        return cli::usage_error(&"ci: plan or done");
    };
    let parsed = Args::parse(rest, &["--pack", "--runners", "--reports"], &["--all"]);
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
