use super::scope::{Class, Doc};
use crate::arch::any;
use gatekeeper_model::Glob;

pub(crate) fn unlisted(docs: &[Doc], listed: &[Glob]) -> Vec<String> {
    let agent = docs.iter().filter(|d| d.class == Class::AgentText);
    let unlisted = agent.filter(|d| !any(listed, &d.path));
    unlisted
        .map(|d| format!("{}: agent text is not listed as generated", d.path))
        .collect()
}
