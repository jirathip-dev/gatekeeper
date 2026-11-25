use gatekeeper_model::{Glob, Lines, TestsConfig};

#[derive(Clone, Copy)]
pub(crate) enum Side {
    Test,
    Src,
    SrcThenTestTail,
}

pub(crate) fn starts_tail(line: &str) -> bool {
    line.trim_start().starts_with("#[cfg(test)]")
}

pub(crate) fn side(path: &str, config: &TestsConfig) -> Option<Side> {
    let (dirs, name) = path.rsplit_once('/').unwrap_or(("", path));
    let ext = name.rsplit_once('.').map(|(_, x)| x)?;
    let dir = |f: &dyn Fn(&str) -> bool| dirs.split('/').any(f);
    let rust = ext == "rs";
    let test = match ext {
        "rs" => name == "tests.rs" || dir(&|d| d == "tests" || d == "benches"),
        "ts" | "tsx" | "mts" | "cts" if !name.ends_with(".d.ts") => {
            name.contains(".test.")
                || name.contains(".spec.")
                || dir(&|d| ["__tests__", "tests", "e2e"].contains(&d))
        }
        "swift" => dir(&|d| d.ends_with("Tests")),
        _ => return None,
    };
    let hit = |globs: &[Glob]| globs.iter().any(|g| g.matches(path));
    if hit(&config.exclude) {
        return None;
    }
    Some(match () {
        () if hit(&config.src) => Side::Src,
        () if test || hit(&config.test) => Side::Test,
        () if rust => Side::SrcThenTestTail,
        () => Side::Src,
    })
}

pub(crate) fn lines(side: Side, text: &str) -> Lines {
    let mut out = Lines::default();
    let mut tail = matches!(side, Side::Test);
    for line in text.lines() {
        tail = tail || matches!(side, Side::SrcThenTestTail) && starts_tail(line);
        match () {
            () if line.trim().is_empty() => {}
            () if tail => out.test += 1,
            () => out.src += 1,
        }
    }
    out
}
