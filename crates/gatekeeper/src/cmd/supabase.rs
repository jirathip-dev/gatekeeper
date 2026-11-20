use crate::cli::{self, Args};
use gatekeeper_engine::{Error, config, run};
use gatekeeper_model::PackId;
use std::process::ExitCode;

fn script_args(args: &[String]) -> Result<(&'static str, Vec<String>), Error> {
    match args.split_first() {
        Some((verb, rest)) if verb == "status" => {
            Args::parse(rest, &[], &[])?;
            Ok(("status.mjs", Vec::new()))
        }
        Some((verb, rest)) if verb == "apply" => {
            let parsed = Args::parse(rest, &["--target"], &[])?;
            let branch = match parsed.value("--target") {
                Some("prod") => "main".to_owned(),
                Some("dev") => config::load(&cli::root()?)?.integration,
                _ => return Err(Error("apply needs --target dev|prod".into())),
            };
            let target = parsed.value("--target").unwrap_or_default().to_owned();
            Ok((
                "apply.mjs",
                vec!["--target".into(), target, "--branch".into(), branch],
            ))
        }
        _ => Err(Error(
            "usage: gatekeeper supabase status | supabase apply --target dev|prod".into(),
        )),
    }
}

/// `supabase status`, `supabase apply --target dev|prod`: the migration ledger, by the pack's scripts.
pub fn run(args: &[String]) -> ExitCode {
    let ran = script_args(args)
        .and_then(|(script, rest)| run::script(&cli::root()?, PackId::Supabase, script, &rest));
    match ran {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(e) => cli::usage_error(&e),
    }
}
