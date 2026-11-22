use gatekeeper_engine::plan;
use gatekeeper_engine::run::{self, RunCtx};
use gatekeeper_model::{Config, Filter, Host};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Expect {
    fails: bool,
    kept: Vec<String>,
}

fn run_case(case: &Path) {
    let name = case.file_name().expect("name").to_string_lossy();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("kept")
        .join(&*name);
    fs::remove_dir_all(&root).ok();
    fs::create_dir_all(root.join(".gatekeeper/gates")).expect("gates dir");
    fs::copy(
        case.join("gate.toml"),
        root.join(format!(".gatekeeper/gates/{name}.toml")),
    )
    .expect("gate");
    let expect = fs::read_to_string(case.join("expect.toml")).expect("expect");
    let expect: Expect = toml::from_str(&expect).expect("expect toml");
    let config: Config =
        toml::from_str("packs = []\nintegration = \"staging\"\npin = \"checkout\"\n")
            .expect("config");
    let plan = plan::repo(&config, &root, &|_| Host::Lane, &Filter::default()).expect("plan");
    let upload = plan.jobs[0].gates[0].gate.uploads[0].name.clone();
    let cx = RunCtx::lane(&root, &root.join("cache"));
    let report = run::run(&plan, &cx, &mut |_, _| {}).expect("run");
    let gate = &report.jobs[0].gates[0];
    assert_eq!(
        gate.outcome.fails(),
        expect.fails,
        "{name}: {:?}",
        gate.outcome
    );
    assert_eq!(gate.kept.len(), 1, "{name}");
    assert_eq!(gate.kept[0].name, upload, "{name}");
    assert_eq!(gate.kept[0].files, expect.kept, "{name}");
    let dest = cx.out.join("files").join(&upload);
    for file in &expect.kept {
        let copy = fs::read(dest.join(file)).expect("kept copy");
        assert_eq!(
            copy,
            fs::read(root.join(file)).expect("original"),
            "{name}: {file}"
        );
    }
    let json = fs::read_to_string(cx.out.join("report.json")).expect("report");
    for file in &expect.kept {
        assert!(json.contains(file.as_str()), "{name}: report lacks {file}");
    }
}

#[test]
fn each_kept_files_case_keeps_what_its_folder_expects() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/kept");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).expect("tests/kept") {
        run_case(&entry.expect("entry").path());
        seen += 1;
    }
    assert_eq!(seen, 3);
}
