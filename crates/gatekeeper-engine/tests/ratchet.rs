use gatekeeper_engine::ratchet::pubs::{self, Member};
use gatekeeper_engine::ratchet::{reports, trailer};
use gatekeeper_model::Glob;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct Dir {
    dir: String,
    published: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    #[serde(rename = "about")]
    _about: String,
    machete: Option<String>,
    jscpd: Option<String>,
    clippy: Option<String>,
    oxlint: Option<String>,
    #[serde(default)]
    members: Vec<Dir>,
    #[serde(default)]
    exported: Vec<Glob>,
    files: Option<BTreeMap<String, String>>,
    word: Option<String>,
    bodies: Option<String>,
    out: Vec<String>,
}

fn run(case: Case) -> Vec<String> {
    if let Some(stdout) = case.machete {
        return reports::machete(&stdout);
    }
    if let Some(report) = case.jscpd {
        let found = reports::jscpd(&report).expect("a jscpd report");
        let total = format!("{} duplicated lines", found.duplicated_lines);
        let each = found.by_format.iter().map(|(f, n)| format!("{f} {n}"));
        return std::iter::once(total)
            .chain(each)
            .chain(found.clones)
            .collect();
    }
    if let Some(json_lines) = case.clippy {
        return reports::clippy(&json_lines);
    }
    if let Some(json) = case.oxlint {
        return reports::oxlint(&serde_json::from_str(&json).expect("oxlint json"));
    }
    if let (Some(word), Some(bodies)) = (case.word, case.bodies) {
        let found = trailer::find(&bodies, &word);
        let line = |t: &trailer::Trailer| {
            let record = t
                .record
                .as_ref()
                .map_or_else(String::new, |r| format!("{r} "));
            format!("{record}{} → {} (#{}, q{})", t.old, t.new, t.issue, t.ask)
        };
        return found.iter().map(line).collect();
    }
    let members: Vec<Member> = (case.members.into_iter())
        .map(|d| Member {
            dir: d.dir,
            published: d.published,
        })
        .collect();
    let files: Vec<(String, String)> = case.files.expect("files").into_iter().collect();
    pubs::scan(&members, &files, &case.exported)
}

#[test]
fn each_ratchet_case_reads_its_input_into_exactly_its_lines() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ratchet");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).expect("tests/ratchet") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        let text = fs::read_to_string(path.join("case.toml")).expect("case.toml");
        let case: Case = toml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let want = case.out.clone();
        assert_eq!(run(case), want, "{name}");
        seen += 1;
    }
    assert_eq!(seen, 10);
}
