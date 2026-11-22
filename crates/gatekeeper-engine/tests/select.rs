use gatekeeper_engine::select::{self, vitest};
use gatekeeper_model::{
    Always, Change, ChangeKind, Evidence, Full, Graph, Read, Select, Selection, TestFile, Trigger,
    Walk, Why, Widen,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

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

fn trees(case: &Case, base: &BTreeMap<String, String>) -> [BTreeMap<String, String>; 2] {
    let mut base = base.clone();
    base.extend(case.files.clone());
    let mut head = base.clone();
    head.extend(case.change.clone());
    head.retain(|path, _| !case.delete.contains(path));
    [base, head]
}

fn tests(head: &BTreeMap<String, String>) -> BTreeMap<TestFile, String> {
    let tests = head
        .iter()
        .filter(|(path, _)| path.contains(".test.") || path.contains(".spec."));
    tests
        .map(|(path, text)| (TestFile(path.clone()), text.clone()))
        .collect()
}

fn run(case: &Case, base: &BTreeMap<String, String>, graph: &Graph) -> Selection {
    let [base, head] = trees(case, base);
    let texts = tests(&head);
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
    for (at, case) in cases(&dir) {
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
    assert_eq!(seen, 23);
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

fn cases(dir: &Path) -> Vec<(String, Case)> {
    let mut cases = Vec::new();
    for entry in fs::read_dir(dir).expect("tests/select") {
        let path = entry.expect("entry").path();
        let Ok(text) = fs::read_to_string(path.join("case.toml")) else {
            continue;
        };
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        let case = toml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        cases.push((name, case));
    }
    cases
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@localhost"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn lay_out(root: &Path, files: &BTreeMap<String, String>) {
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        fs::write(path, text).expect("file");
    }
}

#[test]
#[ignore = "the gate select-vitest runs it, after it installs _base's pinned vitest"]
fn each_case_lists_its_closure_and_its_tests_under_a_real_vitest() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/select");
    let modules =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/select-vitest/node_modules");
    let modules = modules
        .canonicalize()
        .expect("target/select-vitest: npm ci it first");
    let mut base = BTreeMap::new();
    tree(&dir.join("_base"), &dir.join("_base"), &mut base);
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("select-vitest");
    let cases = cases(&dir);
    assert_eq!(cases.len(), 23);
    let mut wrong = Vec::new();
    for (name, case) in cases {
        let root = out.join(&name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clean");
        }
        let [before, head] = trees(&case, &base);
        lay_out(&root, &before);
        fs::write(root.join(".gitignore"), "node_modules\n").expect("ignore");
        std::os::unix::fs::symlink(&modules, root.join("node_modules")).expect("link");
        git(&root, &["init", "-q"]);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "base"]);
        let sha = git(&root, &["rev-parse", "HEAD"]);
        for path in &case.delete {
            fs::remove_file(root.join(path)).expect("delete");
        }
        lay_out(&root, &case.change);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "--allow-empty", "-m", "change"]);
        let config = "vitest.config.ts";
        let closure = vitest::list(&root, config, Some(&sha)).expect("vitest --changed");
        if closure != case.closure {
            wrong.push(format!("{name}: vitest lists the closure {closure:?}"));
        }
        let universe = vitest::list(&root, config, None).expect("vitest list");
        let want: BTreeSet<TestFile> = tests(&head).into_keys().collect();
        if universe != want {
            wrong.push(format!("{name}: vitest lists the tests {universe:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
