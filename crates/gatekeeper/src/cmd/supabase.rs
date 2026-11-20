use crate::cli;
use gatekeeper_engine::{Error, config, run};
use gatekeeper_model::PackId;
use std::process::ExitCode;

/// `supabase status`, `supabase apply --target dev|prod`: the migration ledger, by the pack's scripts.
pub fn run(args: &[String]) -> ExitCode {
    let ran = cli::root().and_then(|root| {
        let script = match args.first().map(String::as_str) {
            Some(verb @ ("status" | "apply")) => format!("{verb}.ts"),
            _ => {
                return Err(Error(
                    "usage: supabase status | supabase apply --target dev|prod".into(),
                ));
            }
        };
        let mut rest = args.get(1..).unwrap_or_default().to_vec();
        rest.extend(["--integration".into(), config::load(&root)?.integration]);
        run::script(&root, PackId::Supabase, &script, &rest)
    });
    match ran {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(e) => cli::usage_error(&e),
    }
}
