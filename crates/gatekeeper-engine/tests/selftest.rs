use gatekeeper_engine::selftest::{self, Bench};
use gatekeeper_model::{Gate, GateId, Host, PackId, Platform, Secs, Shards, Step};
use std::collections::{BTreeMap, BTreeSet};
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
        versions: Vec::new(),
        cache: true,
        select: None,
    }
}

fn out(name: &str) -> (PathBuf, Bench) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let bench = Bench {
        host: Host::Lane,
        platform: Platform::Linux,
        cache: dir.join("cache"),
        tools: BTreeMap::new(),
    };
    (dir.join("selftest"), bench)
}

#[test]
fn a_gate_with_no_failing_case_fails_selftest() {
    let (out, bench) = out("selftest-uncovered");
    let result = selftest::gate(&always_passing("uncovered"), &out, &bench).expect("selftest");
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
    let (out, bench) = out("selftest-lenient");
    let result = selftest::gate(&always_passing("arch"), &out, &bench).expect("selftest");
    assert_eq!(result.cases, 6);
    assert_eq!(result.problems.len(), 6, "{:?}", result.problems);
    assert!(
        result
            .problems
            .iter()
            .all(|p| p.ends_with("passed; it must fail"))
    );
}

#[test]
fn a_macos_gate_on_a_linux_host_fails_every_case_with_needs_macos_and_the_any_platform_token_lint_runs()
 {
    let (out, bench) = out("selftest-needs-macos");
    let results = selftest::run(&[PackId::Swift], &bench, &out, &mut |_| {}).expect("selftest");
    for r in &results {
        let name = r.id.to_string();
        if name == "swift/tokens" {
            assert!(r.problems.is_empty(), "{name}: {:?}", r.problems);
            continue;
        }
        assert!(r.cases > 0, "{name}");
        assert!(
            r.problems
                .iter()
                .any(|p| p.starts_with("clean: failed") && p.contains("needs macos")),
            "{name}: {:?}",
            r.problems
        );
    }
    assert_eq!(results.len(), 3);
}
