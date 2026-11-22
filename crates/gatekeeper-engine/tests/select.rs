use gatekeeper_engine::select;
use gatekeeper_model::{
    Always, Change, ChangeKind, Evidence, Full, Graph, Read, Select, Selection, TestFile, Trigger,
    Walk, Why, Widen,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const REASONS: [&str; 10] = [
    "closure",
    "always-path",
    "always-no-project",
    "reads-path",
    "reads-basename",
    "reads-walks-project",
    "reads-walks-dir",
    "names-project",
    "widened-former-project",
    "widened-specifier",
];
const FULLS: [&str; 3] = ["not-a-change", "file", "empty"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    #[serde(rename = "about")]
    _about: String,
    #[serde(default)]
    closure: BTreeSet<TestFile>,
    #[serde(default)]
    delete: BTreeSet<String>,
    #[serde(default)]
    trigger: Option<Trigger>,
    #[serde(default)]
    full: Option<Full>,
    #[serde(default)]
    select: Option<Select>,
    #[serde(default)]
    files: BTreeMap<String, String>,
    #[serde(default)]
    change: BTreeMap<String, String>,
    #[serde(default)]
    expect: BTreeMap<TestFile, BTreeSet<String>>,
}

fn reason(why: &Why) -> &'static str {
    match why {
        Why::Closure => "closure",
        Why::Always(Always::Path) => "always-path",
        Why::Always(Always::NoProject) => "always-no-project",
        Why::Reads(Read::Path(_)) => "reads-path",
        Why::Reads(Read::Basename(_)) => "reads-basename",
        Why::Reads(Read::Walks(Walk::Project(_))) => "reads-walks-project",
        Why::Reads(Read::Walks(Walk::Dir(_))) => "reads-walks-dir",
        Why::NamesProject(_) => "names-project",
        Why::Widened(Widen::FormerProject(_)) => "widened-former-project",
        Why::Widened(Widen::Specifier(_)) => "widened-specifier",
    }
}

fn full_kind(full: &Full) -> &'static str {
    match full {
        Full::NotAChange(_) => "not-a-change",
        Full::File(_) => "file",
        Full::Empty => "empty",
    }
}

fn tree(dir: &Path, at: &Path, into: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(at).expect("dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            tree(dir, &path, into);
        } else {
            let rel = path.strip_prefix(dir).expect("under _base");
            let text = fs::read_to_string(&path).expect("text");
            into.insert(rel.to_string_lossy().into_owned(), text);
        }
    }
}

fn trigger(case: &Case, base: &BTreeMap<String, String>) -> Trigger {
    let kind = |path: &String| {
        if base.contains_key(path) {
            ChangeKind::Modified
        } else {
            ChangeKind::Added
        }
    };
    let changed = case.change.keys().map(|path| Change {
        path: path.clone(),
        kind: kind(path),
    });
    let deleted = case.delete.iter().map(|path| {
        assert!(
            base.contains_key(path),
            "`{path}` is deleted but not in the base"
        );
        Change {
            path: path.clone(),
            kind: ChangeKind::Deleted,
        }
    });
    Trigger::Change(changed.chain(deleted).collect())
}

fn run(case: &Case, base: &BTreeMap<String, String>, graph: &Graph) -> Selection {
    let mut base = base.clone();
    base.extend(case.files.clone());
    let mut head = base.clone();
    head.extend(case.change.clone());
    head.retain(|path, _| !case.delete.contains(path));
    let texts: BTreeMap<TestFile, String> = head
        .into_iter()
        .filter(|(path, _)| path.contains(".test.") || path.contains(".spec."))
        .map(|(path, text)| (TestFile(path), text))
        .collect();
    let universe: BTreeSet<TestFile> = texts.keys().cloned().collect();
    let gate = case.select.clone().unwrap_or_else(|| Select {
        vitest: "vitest.config.ts".into(),
        always: Vec::new(),
        full: Vec::new(),
    });
    let evidence = Evidence {
        universe: &universe,
        closure: &case.closure,
        texts: &texts,
        graph,
    };
    let trigger = case.trigger.clone().unwrap_or_else(|| trigger(case, &base));
    select::select(&trigger, &gate, &evidence)
}

#[test]
fn each_selection_case_selects_exactly_what_its_folder_expects() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/select");
    let mut base = BTreeMap::new();
    tree(&dir.join("_base"), &dir.join("_base"), &mut base);
    let graph: Graph = toml::from_str(&base["graph.toml"]).expect("graph.toml");
    let (mut seen, mut reasons, mut fulls) = (0, BTreeSet::new(), BTreeSet::new());
    for entry in fs::read_dir(&dir).expect("tests/select") {
        let path = entry.expect("entry").path().join("case.toml");
        if !path.exists() {
            continue;
        }
        let case: Case = toml::from_str(&fs::read_to_string(&path).expect("case")).expect("toml");
        let at = path.display();
        match (run(&case, &base, &graph), &case.full) {
            (Selection::Full(got), Some(want)) => {
                assert!(case.expect.is_empty(), "{at}: give `full` or `expect`");
                assert_eq!(&got, want, "{at}");
                fulls.insert(full_kind(&got));
            }
            (Selection::Files(got), None) => {
                let got: BTreeMap<TestFile, BTreeSet<String>> = got
                    .reasons()
                    .iter()
                    .map(|(f, whys)| (f.clone(), whys.iter().map(|w| reason(w).into()).collect()))
                    .collect();
                assert_eq!(got, case.expect, "{at}");
                reasons.extend(got.into_values().flatten());
            }
            (got, _) => panic!("{at}: got {got:?}"),
        }
        seen += 1;
    }
    assert_eq!(seen, 22);
    for name in REASONS {
        assert!(reasons.contains(name), "no case selects a test by `{name}`");
    }
    for name in FULLS {
        assert!(
            fulls.contains(name),
            "no case runs the full suite by `{name}`"
        );
    }
}
