use super::{Ident, Inputs};
use crate::fetch::sha256_hex;
use crate::{Error, Result};
use gatekeeper_model::{Gate, Key, KeyParts, Shard, Unkeyed};
use std::path::Path;

/// A gate's key and its parts, or why it has none.
pub enum Keyed {
    Key(Key, Box<KeyParts>),
    No(Unkeyed),
}

/// # Errors
pub fn keyed(
    gate: &Gate,
    (shard, projects): (Shard, &[String]),
    ident: &Ident,
    inputs: &Inputs,
    root: &Path,
) -> Result<Keyed> {
    if let Some(why) = Unkeyed::of(gate) {
        return Ok(Keyed::No(why));
    }
    let file = serde_json::to_string(gate).map_err(|e| Error(e.to_string()))?;
    let parts = KeyParts {
        schema: 1,
        engine: ident.engine.clone(),
        gate: gate.id.clone(),
        gate_file: sha256_hex(file.as_bytes()),
        shard,
        projects: projects.to_vec(),
        platform: ident.platform.clone(),
        versions: ident.versions(gate, root),
        tools: ident.tools(gate)?,
        inputs: inputs.of(gate).1,
    };
    let json = serde_json::to_string(&parts).map_err(|e| Error(e.to_string()))?;
    Ok(Keyed::Key(
        Key(sha256_hex(json.as_bytes())),
        Box::new(parts),
    ))
}
