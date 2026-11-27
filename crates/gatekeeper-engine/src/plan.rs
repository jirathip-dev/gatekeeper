use crate::{Error, Result, graph, packs, scope};
use gatekeeper_model::{
    Affected, Capability, Config, Decision, Filter, Gate, Graph, Host, Job, PackId, Plan, Planned,
    Platform, RepoRun, Shard, Skip, Trigger, When, Workspace, is_name,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::path::Path;

fn selected(gate: &Gate, graph: &Graph, affected: Option<&Affected>) -> Vec<String> {
    let roots: Vec<&String> = match affected {
        Some(Affected::Projects(hit)) => hit.iter().collect(),
        _ => graph.projects.keys().collect(),
    };
    let matched = roots.into_iter();
    let matched = matched.filter(|root| gate.projects.iter().any(|g| g.matches(root)));
    matched.cloned().collect()
}

fn decide(gate: &Gate, host: Host, filter: &Filter, projects: &[String]) -> Decision {
    match (gate.when, filter.when) {
        (When::Nightly, When::Always) => return Decision::Skip(Skip::Nightly),
        (When::Always, When::Nightly) => return Decision::Skip(Skip::NotNightly),
        _ => {}
    }
    let trigger = &filter.trigger;
    let in_scope = match trigger {
        Trigger::Change(files) => {
            gate.paths.is_empty() || files.iter().any(|f| gate.reads(&f.path))
        }
        Trigger::Everything(_) => true,
    };
    if !in_scope {
        return Decision::Skip(Skip::OutOfScope);
    }
    if !gate.projects.is_empty() && projects.is_empty() {
        return Decision::Skip(Skip::NoAffectedProject);
    }
    if gate.needs.contains(&Capability::PrivateNames) {
        match filter.repo {
            RepoRun::Own => {}
            RepoRun::Private => return Decision::Skip(Skip::PrivateRepo),
            RepoRun::Fork => return Decision::Skip(Skip::ForkPullRequest),
        }
    }
    ci_only(gate, host).map_or(Decision::Run, Decision::Skip)
}

/// `Skip::CiOnly` when the gate needs containers, the integration branch's coverage or a quiet host and this is a lane's host.
#[must_use]
pub fn ci_only(gate: &Gate, host: Host) -> Option<Skip> {
    let ci = [
        Capability::Containers,
        Capability::Coverage,
        Capability::QuietHost,
    ];
    let need = ci.into_iter().find(|c| gate.needs.contains(c));
    need.filter(|_| host == Host::Lane).map(Skip::CiOnly)
}

fn check_legs(config: &Config, gates: &[Gate]) -> Result<()> {
    for (name, leg) in &config.legs {
        if !is_name(name) || PackId::parse(name).is_some() {
            return Err(Error(format!(
                "[legs.{name}]: a leg is named with lowercase letters, digits and dashes, and not after a pack"
            )));
        }
        let Some(after) = &leg.after else { continue };
        let chained = config.legs.get(after).is_some_and(|l| l.after.is_some());
        if chained || !gates.iter().any(|g| g.leg == *after) {
            return Err(Error(format!(
                "[legs.{name}] after = `{after}`: name a leg with gates that itself runs after none"
            )));
        }
    }
    for gate in gates {
        if gate.leg != gate.id.pack.as_str() && !config.legs.contains_key(&gate.leg) {
            return Err(Error(format!(
                "{}: leg `{}` has no [legs.{}] in the config",
                gate.id, gate.leg, gate.leg
            )));
        }
        if !gate.projects.is_empty() && config.scope.is_none() {
            return Err(Error(format!(
                "{}: `projects` needs [scope] graphs in the config",
                gate.id
            )));
        }
        if gate.select.is_some() && (config.ts.is_none() || config.scope.is_none()) {
            return Err(Error(format!(
                "{}: [select] needs [ts] and [scope] graphs in the config",
                gate.id
            )));
        }
    }
    Ok(())
}

struct Scoped<'a> {
    hosts: &'a dyn Fn(Platform) -> Host,
    graph: &'a Graph,
    filter: &'a Filter,
    affected: Option<&'a Affected>,
}

fn leg_jobs(config: &Config, leg: &str, gates: Vec<Gate>, cx: &Scoped) -> Result<Vec<Job>> {
    let configured = config.legs.get(leg);
    let first = gates.first().map_or(Platform::Linux, |g| g.platform);
    let platform = configured.map_or(first, |l| l.platform);
    let host = (cx.hosts)(platform);
    let of = gates.iter().map(|g| g.shards.on(host)).max();
    let of = of.unwrap_or(NonZeroU32::MIN);
    let uneven = |g: &&Gate| ![NonZeroU32::MIN, of].contains(&g.shards.on(host));
    if let Some(g) = gates.iter().find(uneven) {
        return Err(Error(format!(
            "{}: leg `{leg}` splits into {of} shards on {}; each of its gates splits into 1 or {of}",
            g.id,
            host.label()
        )));
    }
    let mut setup: BTreeSet<_> = configured.map_or_else(BTreeSet::new, |l| l.setup.clone());
    setup.extend(gates.iter().flat_map(|g| g.setup.iter().copied()));
    let planned: Vec<Planned> = gates
        .into_iter()
        .map(|gate| {
            let projects = selected(&gate, cx.graph, cx.affected);
            let decision = match config.off.get(&gate.id) {
                Some(why) => Decision::Off(why.clone()),
                None => decide(&gate, host, cx.filter, &projects),
            };
            Planned {
                gate,
                decision,
                projects,
            }
        })
        .collect();
    let shards = (1..=of.get()).filter_map(NonZeroU32::new);
    Ok(shards
        .map(|index| Job {
            leg: leg.to_owned(),
            shard: Shard { index, of },
            after: configured.and_then(|l| l.after.clone()),
            platform,
            setup: setup.clone(),
            gates: planned
                .iter()
                .filter(|p| index == NonZeroU32::MIN || p.gate.shards.on(host) == of)
                .cloned()
                .collect(),
        })
        .collect())
}

fn check_off(
    config: &Config,
    gates: &[Gate],
    hosts: &dyn Fn(Platform) -> Host,
    repo: RepoRun,
) -> Result<()> {
    for (id, why) in &config.off {
        if why.trim().is_empty() {
            return Err(Error(format!("[off] `{id}` needs a reason")));
        }
        if id.pack == PackId::Core && id.name() == "pin" {
            return Err(Error(format!("[off] `{id}` guards the pin and stays on")));
        }
        let Some(gate) = gates.iter().find(|g| g.id == *id) else {
            return Err(Error(format!(
                "[off] names `{id}`, which no configured pack has"
            )));
        };
        let public_ci = repo != RepoRun::Private && hosts(gate.platform) != Host::Lane;
        if public_ci && gate.needs.contains(&Capability::PrivateNames) {
            return Err(Error(format!(
                "[off] `{id}`: a public repo's CI run checks it; only a private repo turns it off"
            )));
        }
    }
    Ok(())
}

fn check_filter(filter: &Filter, gates: &[Gate]) -> Result<()> {
    let packs: BTreeSet<PackId> = gates.iter().map(|g| g.id.pack).collect();
    if let Some(p) = filter.packs.iter().find(|p| !packs.contains(p)) {
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
    if let Some(leg) = filter
        .leg
        .as_ref()
        .filter(|l| !gates.iter().any(|g| g.leg == **l))
    {
        return Err(Error(format!("--leg {leg}: no gate runs in it")));
    }
    Ok(())
}

/// # Errors
pub fn plan(
    config: &Config,
    mut gates: Vec<Gate>,
    hosts: &dyn Fn(Platform) -> Host,
    graph: &Graph,
    filter: &Filter,
) -> Result<Plan> {
    for (id, secs) in &config.budgets {
        let gate = gates.iter_mut().find(|g| g.id == *id).ok_or_else(|| {
            Error(format!(
                "[budgets] names `{id}`, which no configured pack has"
            ))
        })?;
        gate.budget = *secs;
    }
    check_off(config, &gates, hosts, filter.repo)?;
    check_legs(config, &gates)?;
    check_filter(filter, &gates)?;
    let chosen = |g: &Gate| {
        (filter.packs.is_empty() || filter.packs.contains(&g.id.pack))
            && filter.gate.as_ref().is_none_or(|id| *id == g.id)
            && filter.leg.as_ref().is_none_or(|l| *l == g.leg)
    };
    let mut legs: BTreeMap<String, Vec<Gate>> = BTreeMap::new();
    for gate in gates.into_iter().filter(chosen) {
        legs.entry(gate.leg.clone()).or_default().push(gate);
    }
    let ignore = config.scope.as_ref().map(|s| s.ignore.clone());
    let affected = (ignore.as_ref()).map(|i| scope::affected(graph, filter.trigger.clone(), i));
    let cx = Scoped {
        hosts,
        graph,
        filter,
        affected: affected.as_ref(),
    };
    let mut jobs = Vec::new();
    for (leg, gates) in legs {
        jobs.extend(leg_jobs(config, &leg, gates, &cx)?);
    }
    if let Some(shard) = filter.shard {
        if let Some(job) = jobs.iter().find(|j| j.shard.of != shard.of) {
            return Err(Error(format!(
                "--shard {shard}: leg {} splits into {} here",
                job.leg, job.shard.of
            )));
        }
        jobs.retain(|j| j.shard == shard);
    }
    jobs.sort_by_key(|j| j.after.is_some());
    let (graph, ignore) = (graph.clone(), ignore.unwrap_or_default());
    let workspace = Workspace { graph, ignore };
    Ok(Plan {
        affected,
        workspace,
        trigger: filter.trigger.clone(),
        jobs,
    })
}

/// # Errors
pub fn repo(
    config: &Config,
    root: &Path,
    hosts: &dyn Fn(Platform) -> Host,
    filter: &Filter,
) -> Result<Plan> {
    let gates = packs::configured(config, root)?;
    let graph = match &config.scope {
        Some(s) => graph::read(root, &s.graphs)?,
        None => Graph::default(),
    };
    plan(config, gates, hosts, &graph, filter)
}
