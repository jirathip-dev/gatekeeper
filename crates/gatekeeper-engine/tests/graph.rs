use gatekeeper_engine::graph;
use gatekeeper_model::{Graph, GraphKind};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Deserialize)]
struct Expect {
    kinds: BTreeSet<GraphKind>,
    graph: Graph,
}

#[test]
fn each_workspace_reader_reads_the_graph_its_folder_expects() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/graph");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).expect("tests/graph") {
        let root = entry.expect("entry").path();
        let text = std::fs::read_to_string(root.join("expect.toml")).expect("expect");
        let expect: Expect = toml::from_str(&text).expect("toml");
        let got = graph::read(&root, &expect.kinds).expect("graph");
        assert_eq!(got.projects, expect.graph.projects, "{}", root.display());
        seen += 1;
    }
    assert_eq!(seen, 3);
}
