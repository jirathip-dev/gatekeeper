use crate::{Error, Result};
use gatekeeper_model::{
    Capability, Config, Decision, Filter, Gate, Host, Job, PackId, Plan, Planned, Skip,
};
use std::collections::BTreeSet;

fn decide(gate: &Gate, host: Host, changed: Option<&BTreeSet<String>>) -> Decision {
    let in_scope = |files: &BTreeSet<String>| {
        gate.paths.is_empty()
            || files
                .iter()
                .any(|f| gate.paths.iter().any(|g| g.matches(f)))
    };
    if !changed.is_none_or(in_scope) {
        return Decision::Skip(Skip::OutOfScope);
    }
    ci_only(gate, host).map_or(Decision::Run, Decision::Skip)
}

/// `Skip::CiOnly` when the gate needs containers and this is a lane's host.
#[must_use]
pub fn ci_only(gate: &Gate, host: Host) -> Option<Skip> {
    (host == Host::Lane && gate.needs.contains(&Capability::Containers))
        .then_some(Skip::CiOnly(Capability::Containers))
}

/// # Errors
pub fn plan(config: &Config, mut gates: Vec<Gate>, host: Host, filter: &Filter) -> Result<Plan> {
    for (id, secs) in &config.budgets {
        let gate = gates.iter_mut().find(|g| g.id == *id).ok_or_else(|| {
            Error(format!(
                "[budgets] names `{id}`, which no configured pack has"
            ))
        })?;
        gate.budget = *secs;
    }
    let legs: BTreeSet<PackId> = gates.iter().map(|g| g.id.pack).collect();
    if let Some(p) = filter.packs.iter().find(|p| !legs.contains(p)) {
        return Err(Error(format!(
            "--pack {p}: not a configured pack with gates"
        )));
    }
    if let Some(id) = filter
        .gate
        .as_ref()
        .filter(|id| !gates.iter().any(|g| g.id == **id))
    {
        return Err(Error(format!("--gate {id}: no such gate")));
    }
    let chosen = |g: &Gate| {
        (filter.packs.is_empty() || filter.packs.contains(&g.id.pack))
            && filter.gate.as_ref().is_none_or(|id| *id == g.id)
    };
    let mut jobs: Vec<Job> = Vec::new();
    for gate in gates.into_iter().filter(chosen) {
        let decision = decide(&gate, host, filter.changed.as_ref());
        if !jobs.iter().any(|j| j.leg == gate.id.pack) {
            jobs.push(Job {
                leg: gate.id.pack,
                platform: gate.platform,
                setup: BTreeSet::new(),
                gates: Vec::new(),
            });
        }
        let job = jobs
            .iter_mut()
            .find(|j| j.leg == gate.id.pack)
            .ok_or_else(|| Error("no job".into()))?;
        job.setup.extend(gate.setup.iter().copied());
        job.gates.push(Planned { gate, decision });
    }
    jobs.sort_by_key(|j| j.leg);
    Ok(Plan { jobs })
}
