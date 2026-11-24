use gatekeeper_engine::config;
use std::fs;
use std::path::PathBuf;

fn load(name: &str, docs: &str) -> String {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("docs-config")
        .join(name);
    fs::create_dir_all(root.join(".gatekeeper")).expect("dir");
    let text =
        format!("packs = [\"core\"]\nintegration = \"main\"\npin = \"checkout\"\n\n{docs}\n");
    fs::write(root.join(config::PATH), text).expect("config");
    match config::load(&root) {
        Ok(c) => format!("ok {}", c.docs.records.0.len()),
        Err(e) => e.0,
    }
}

#[test]
fn a_docs_table_with_a_wrong_key_or_value_is_a_load_error_that_names_it() {
    let cases = [
        (
            "unknown-key",
            "[docs]\npaths = [\"README.md\"]",
            "unknown field `paths`",
        ),
        (
            "empty-reason",
            "[docs.records]\n\"docs/old/**\" = \"  \"",
            "`docs/old/**` needs a reason",
        ),
        (
            "bad-glob",
            "[docs.records]\n\"../x/**\" = \"why\"",
            "`../x/**` is not a repo-relative glob",
        ),
        ("valid", "[docs.records]\n\"docs/old/**\" = \"why\"", "ok 1"),
        ("no-table", "", "ok 0"),
        (
            "names-both",
            "[docs.names]\nrun = \"sh n.sh\"\nfile = \"n.tsv\"",
            "needs exactly one of `run` and `file`",
        ),
        (
            "names-neither",
            "[docs.names.sets]\nverb = [\"tool {name}\"]",
            "needs exactly one of `run` and `file`",
        ),
        (
            "names-unknown-key",
            "[docs.names]\nrun = \"sh n.sh\"\nprogram = \"x\"",
            "unknown field `program`",
        ),
        (
            "prefix-without-name",
            "[docs.names]\nrun = \"sh n.sh\"\n[docs.names.sets]\nverb = [\"tool \"]",
            "prefix `tool `",
        ),
        (
            "prefix-with-nothing-before",
            "[docs.names]\nrun = \"sh n.sh\"\n[docs.names.sets]\nverb = [\"{name}\"]",
            "prefix `{name}`",
        ),
        (
            "set-with-a-capital",
            "[docs.names]\nrun = \"sh n.sh\"\n[docs.names.sets]\nVerb = [\"tool {name}\"]",
            "set `Verb`",
        ),
        (
            "names-valid",
            "[docs.names]\nrun = \"sh n.sh\"\n[docs.names.sets]\nverb = [\"tool {name}\"]",
            "ok 0",
        ),
    ];
    for (name, docs, want) in cases {
        let got = load(name, docs);
        assert!(got.contains(want), "{name}: {got}");
    }
}
