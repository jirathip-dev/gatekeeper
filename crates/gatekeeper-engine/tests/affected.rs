use gatekeeper_engine::scope;
use gatekeeper_model::{Affected, Glob, Graph, Trigger};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Case {
    trigger: Trigger,
    expect: Affected,
    ignore: Vec<Glob>,
    graph: Graph,
}

#[test]
fn each_affected_projects_case_selects_what_its_folder_expects() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/affected");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).expect("tests/affected") {
        let path = entry.expect("entry").path().join("case.toml");
        let case: Case =
            toml::from_str(&std::fs::read_to_string(&path).expect("case")).expect("toml");
        let got = scope::affected(&case.graph, case.trigger, &case.ignore);
        assert_eq!(got, case.expect, "{}", path.display());
        seen += 1;
    }
    assert_eq!(seen, 6);
}
