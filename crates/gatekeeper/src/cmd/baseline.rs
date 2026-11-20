use crate::cli::{self, say};
use gatekeeper_engine::{Error, baseline, config, packs};
use gatekeeper_model::GateId;
use std::process::ExitCode;

fn write(args: &[String]) -> Result<baseline::Written, Error> {
    let [id] = args else {
        return Err(Error(
            "baseline needs one gate id: baseline <pack/name>".into(),
        ));
    };
    let id = GateId::try_from(id.clone()).map_err(Error)?;
    let root = cli::root()?;
    let config = config::load(&root)?;
    let gates = packs::configured(&config, &root)?;
    let gate = gates
        .iter()
        .find(|g| g.id == id)
        .ok_or_else(|| Error(format!("{id}: no such gate")))?;
    baseline::write(gate, &root, &cli::cache(), &config)
}

/// `baseline <gate>`: list the gate's findings once; after that only drop the fixed ones.
pub fn run(args: &[String]) -> ExitCode {
    let written = match write(args) {
        Ok(w) => w,
        Err(e) => return cli::usage_error(&e),
    };
    let root = cli::root().unwrap_or_default();
    let path = written.path.strip_prefix(&root).unwrap_or(&written.path);
    match written.lines {
        0 => say(&format!("{}: no findings listed, no file", path.display())),
        n => say(&format!("{}: {n} findings", path.display())),
    }
    for line in &written.unlisted {
        say(&format!("new   {line}"));
    }
    if !written.unlisted.is_empty() {
        say(&format!(
            "{} new findings are not listed: a baseline only shrinks, fix them",
            written.unlisted.len()
        ));
    }
    cli::exit(!written.unlisted.is_empty())
}
