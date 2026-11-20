use gatekeeper_model::{Host, Millis, Outcome, Secs, judge};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Case {
    host: Host,
    took_ms: u64,
    budget_secs: u32,
    expect: String,
}

#[test]
fn each_budget_mode_case_judges_as_its_folder_expects() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/budget");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).expect("tests/budget") {
        let path = entry.expect("entry").path().join("case.toml");
        let case: Case =
            toml::from_str(&std::fs::read_to_string(&path).expect("case")).expect("toml");
        let outcome = judge(
            Ok(()),
            Millis(case.took_ms),
            Secs(case.budget_secs),
            case.host,
        );
        let got = match outcome {
            Outcome::OverBudget => "over-budget",
            Outcome::Passed { slow: true } => "slow",
            Outcome::Passed { slow: false } => "ok",
            _ => "other",
        };
        assert_eq!(got, case.expect, "{}", path.display());
        assert_eq!(
            outcome.fails(),
            case.expect == "over-budget",
            "{}",
            path.display()
        );
        seen += 1;
    }
    assert_eq!(seen, 4);
}

#[test]
fn a_gate_inside_its_budget_is_ok_on_every_host() {
    let hosts = [
        Host::GithubHosted { warm_cache: true },
        Host::SelfHosted,
        Host::Lane,
    ];
    for host in hosts {
        assert_eq!(
            judge(Ok(()), Millis(2000), Secs(2), host),
            Outcome::Passed { slow: false }
        );
    }
}
