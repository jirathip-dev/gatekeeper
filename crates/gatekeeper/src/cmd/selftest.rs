use crate::cli::{self, Args, say};
use gatekeeper_engine::selftest::{self, Bench, GateResult};
use gatekeeper_engine::{config, run};
use gatekeeper_model::PackId;
use std::process::ExitCode;

fn show(r: &GateResult) {
    if let Some(why) = r.skipped {
        say(&format!("skip  {:<18} {why}", r.id));
        return;
    }
    if r.problems.is_empty() {
        say(&format!("ok    {:<18} clean and {} cases", r.id, r.cases));
        return;
    }
    say(&format!("FAIL  {}", r.id));
    for line in r.problems.iter().flat_map(|p| p.lines()) {
        say(&format!("      {line}"));
    }
}

/// `selftest [--pack P]`: every gate on its pack's base and on each of its cases.
pub fn run(args: &[String]) -> ExitCode {
    let filter = match Args::parse(args, &["--pack"], &[]).and_then(|a| a.filter()) {
        Ok(f) => f,
        Err(e) => return cli::usage_error(&e),
    };
    let packs: Vec<PackId> = if filter.packs.is_empty() {
        PackId::BUILT_IN.to_vec()
    } else {
        filter.packs.into_iter().collect()
    };
    let root = match cli::root() {
        Ok(root) => root,
        Err(e) => return cli::usage_error(&e),
    };
    let own = root
        .join(config::PATH)
        .exists()
        .then(|| config::load(&root));
    let tools = match own.transpose() {
        Ok(c) => c.map(|c| config::tools(&c, &root)).unwrap_or_default(),
        Err(e) => return cli::usage_error(&e),
    };
    let bench = Bench {
        host: cli::host(),
        platform: run::platform(),
        cache: cli::cache(),
        tools,
    };
    match selftest::run(&packs, &bench, &selftest::out_dir(&root), &mut show) {
        Ok(results) => {
            let bad = results.iter().filter(|r| !r.problems.is_empty()).count();
            say(&format!("{} gates, {bad} failed", results.len()));
            cli::exit(bad > 0)
        }
        Err(e) => cli::usage_error(&e),
    }
}
