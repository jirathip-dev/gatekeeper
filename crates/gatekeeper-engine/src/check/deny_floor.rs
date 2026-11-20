use crate::config::read_toml;
use crate::run::CheckCx as Cx;
use crate::{Error, Result, packs};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Floor {
    deny: Vec<String>,
    licences: Vec<String>,
}

/// `deny.toml` denies what the floor denies and allows no licence outside the shared list.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let floor: Floor =
        toml::from_str(packs::text("rust/files/deny-floor.toml").unwrap_or_default())
            .map_err(|e| Error(format!("deny floor: {e}")))?;
    let config = read_toml(&cx.root.join("deny.toml"))?;
    let mut findings = Vec::new();
    for key in &floor.deny {
        let leaf = key
            .split_once('.')
            .and_then(|(table, leaf)| config.get(table)?.get(leaf)?.as_str());
        if leaf != Some("deny") {
            findings.push(format!(
                "deny.toml: {key} is {}, the floor is \"deny\"",
                leaf.map_or("unset".to_owned(), |l| format!("\"{l}\""))
            ));
        }
    }
    let allowed = config
        .get("licenses")
        .and_then(|l| l.get("allow"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str);
    for licence in allowed.filter(|l| !floor.licences.iter().any(|f| f == l)) {
        findings.push(format!(
            "deny.toml: licence {licence} is outside the shared allow-list"
        ));
    }
    Ok(findings)
}
