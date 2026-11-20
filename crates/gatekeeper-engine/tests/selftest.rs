use gatekeeper_engine::selftest;
use gatekeeper_model::{Gate, GateId, PackId, Platform, Secs, Shards, Step};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn always_passing(name: &str) -> Gate {
    Gate {
        id: GateId::new(PackId::Core, name).expect("id"),
        about: "passes".into(),
        fix: "none".into(),
        budget: Secs(5),
        platform: Platform::Linux,
        setup: BTreeSet::new(),
        needs: BTreeSet::new(),
        steps: vec![Step::Run { run: "true".into() }],
        paths: Vec::new(),
        projects: Vec::new(),
        leg: "core".into(),
        shards: Shards::default(),
        uploads: Vec::new(),
        rule: Vec::new(),
    }
}

fn out(name: &str) -> (PathBuf, PathBuf) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    (dir.join("selftest"), dir.join("cache"))
}

#[test]
fn a_gate_with_no_failing_case_fails_selftest() {
    let (out, cache) = out("selftest-uncovered");
    let result = selftest::gate(&always_passing("uncovered"), &out, &cache).expect("selftest");
    assert_eq!(result.cases, 0);
    assert!(
        result
            .problems
            .iter()
            .any(|p| p == "no failing fixture case"),
        "{:?}",
        result.problems
    );
}

#[test]
fn a_gate_that_passes_a_case_meant_to_fail_fails_selftest() {
    let (out, cache) = out("selftest-lenient");
    let result = selftest::gate(&always_passing("arch"), &out, &cache).expect("selftest");
    assert_eq!(result.cases, 6);
    assert_eq!(result.problems.len(), 6, "{:?}", result.problems);
    assert!(
        result
            .problems
            .iter()
            .all(|p| p.ends_with("passed; it must fail"))
    );
}
