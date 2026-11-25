use crate::ratchet::{folder, judge, reports};
use crate::run::CheckCx as Cx;
use crate::{Error, Result, arch, config, packs, runner};
use gatekeeper_model::{CountName, Counted, DUPLICATION_IGNORED, DUPLICATION_STEP, Glob, PackId};
use std::fs;
use std::path::Path;

type Format = (PackId, &'static str, &'static [&'static str]);

const FORMATS: [Format; 6] = [
    (PackId::Rust, "rust", &["rs"]),
    (PackId::Ts, "typescript", &["ts", "mts", "cts"]),
    (PackId::Ts, "tsx", &["tsx"]),
    (PackId::Ts, "javascript", &["js", "mjs", "cjs"]),
    (PackId::Ts, "jsx", &["jsx"]),
    (PackId::Swift, "swift", &["swift"]),
];

const IGNORE_START: &str = concat!("jscpd", ":ignore-start");

struct Scan {
    formats: Vec<&'static str>,
    exts: Vec<&'static str>,
    ignored: Vec<Glob>,
    findings: Vec<String>,
}

fn scan(cx: &Cx) -> Result<Scan> {
    let config = config::load(cx.root)?;
    let mut out = Scan {
        formats: Vec::new(),
        exts: Vec::new(),
        ignored: Vec::new(),
        findings: Vec::new(),
    };
    for (_, format, exts) in FORMATS.iter().filter(|(p, _, _)| config.packs.contains(p)) {
        out.formats.push(format);
        out.exts.extend(*exts);
    }
    let fixed = DUPLICATION_IGNORED
        .iter()
        .map(|g| Glob::try_from((*g).to_owned()));
    out.ignored = fixed
        .collect::<std::result::Result<_, String>>()
        .map_err(Error)?;
    out.ignored.extend(arch::config::generated(cx.root)?);
    let args = [
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ];
    let files = runner::ok("git", &args, cx.root)?;
    let files: Vec<&str> = files.split('\0').filter(|f| !f.is_empty()).collect();
    for g in &config.duplication.ignore.0 {
        if !files.iter().any(|f| g.matches(f)) {
            out.findings.push(format!(
                "[duplication.ignore] `{g}` matches no file: drop it"
            ));
        }
    }
    out.ignored.extend(config.duplication.ignore.0);
    let read = files.iter().filter(|f| {
        let ext = Path::new(f).extension().and_then(|x| x.to_str());
        ext.is_some_and(|x| out.exts.contains(&x)) && !out.ignored.iter().any(|g| g.matches(f))
    });
    for f in read {
        let text = fs::read_to_string(cx.root.join(f)).unwrap_or_default();
        if text.contains(IGNORE_START) {
            out.findings.push(format!(
                "{f}: a {IGNORE_START} comment hides a clone from the count: delete it"
            ));
        }
    }
    Ok(out)
}

fn jscpd(cx: &Cx, scan: &Scan) -> Result<reports::Duplication> {
    if scan.formats.is_empty() {
        return Ok(reports::Duplication::default());
    }
    let dir = cx.scratch.join("jscpd");
    fs::create_dir_all(&dir).map_err(|e| Error::at(&dir, e))?;
    let settings = dir.join("jscpd.json");
    let text = packs::text("core/files/jscpd.json").unwrap_or_default();
    fs::write(&settings, text).map_err(|e| Error::at(&settings, e))?;
    let report = dir.join("jscpd-report.json");
    fs::remove_file(&report).ok();
    let ignored: Vec<String> = scan.ignored.iter().map(ToString::to_string).collect();
    let (settings, out) = (settings.to_string_lossy(), dir.to_string_lossy());
    let (formats, ignored) = (scan.formats.join(","), ignored.join(","));
    let args = [
        "--config", &settings, "--format", &formats, "--ignore", &ignored,
    ];
    let args = [&args[..], &["--output", &out, "--silent", "."]].concat();
    let bin = cx.tool("jscpd")?;
    let run = runner::capture(&bin.to_string_lossy(), &args, cx.root)?;
    if !run.status.success() {
        return Err(Error(format!(
            "jscpd exited {}: {}",
            run.status,
            String::from_utf8_lossy(&run.stderr).trim()
        )));
    }
    let text = fs::read_to_string(&report).map_err(|e| Error::at(&report, e))?;
    reports::jscpd(&text)
}

pub(crate) fn measure(cx: &Cx) -> Result<(Counted, reports::Duplication, Vec<String>)> {
    let scan = scan(cx)?;
    let found = jscpd(cx, &scan)?;
    let counted = Counted {
        name: CountName::duplication(),
        n: found.duplicated_lines,
        step: DUPLICATION_STEP,
        unit: "duplicated lines",
        gate: "core/duplication",
    };
    Ok((counted, found, scan.findings))
}

/// Pinned jscpd over the repo's code with the pack's settings; its duplicated lines under the lowest ceiling in `ci/ratchet/duplication.d`.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let (counted, found, mut out) = measure(cx)?;
    let files = folder::read::<u32>(cx.root, &counted.name.dir())?;
    let lowest = folder::lowest(&files).map(|(_, c)| *c);
    out.extend(judge::ceiling(&counted, lowest));
    if lowest.is_some_and(|c| counted.n > c) {
        out.extend(found.clones.iter().cloned());
    }
    let ceiling = lowest.map_or_else(|| "none".to_owned(), |c| c.to_string());
    let note = format!(
        "{} duplicated lines / ceiling {ceiling} ({})",
        counted.n,
        found.summary()
    );
    cx.note.set(note).ok();
    Ok(out)
}
