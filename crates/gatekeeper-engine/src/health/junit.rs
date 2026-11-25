use crate::{Error, Result};
use gatekeeper_model::{Case, Ended, Millis};

fn unescape(text: &str) -> String {
    let named = [
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
    ];
    let out = named
        .iter()
        .fold(text.to_owned(), |t, (from, to)| t.replace(from, to));
    out.replace("&amp;", "&")
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let (_, rest) = tag.split_once(&format!(" {key}=\""))?;
    let (value, _) = rest.split_once('"')?;
    Some(unescape(value))
}

fn millis(secs: &str) -> Option<Millis> {
    let (whole, frac) = secs.split_once('.').unwrap_or((secs, ""));
    let frac: String = frac.chars().chain("000".chars()).take(3).collect();
    let whole = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    Some(Millis(whole * 1000 + frac.parse::<u64>().ok()?))
}

fn case(tag: &str, body: &str) -> Result<Case> {
    let field = |key| attr(tag, key).ok_or_else(|| Error(format!("a testcase without `{key}`")));
    let time = attr(tag, "time").unwrap_or_default();
    let ended = match () {
        () if body.contains("<failure") || body.contains("<error") => Ended::Failed,
        () if body.contains("<skipped") => Ended::Skipped,
        () => Ended::Passed,
    };
    Ok(Case {
        class: field("classname")?,
        name: field("name")?,
        took: millis(&time).ok_or_else(|| Error(format!("a testcase's time `{time}`")))?,
        ended,
    })
}

/// # Errors
pub fn read(junit: &str) -> Result<Vec<Case>> {
    if !junit.contains("<testsuite") {
        return Err(Error("not a JUnit file: no `testsuite`".into()));
    }
    let mut cases = Vec::new();
    let mut rest = junit;
    while let Some((_, after)) = rest.split_once("<testcase") {
        let (tag, after) = after
            .split_once('>')
            .ok_or_else(|| Error("an unclosed `testcase`".into()))?;
        let (body, after) = match tag.strip_suffix('/') {
            Some(_) => ("", after),
            None => after
                .split_once("</testcase>")
                .ok_or_else(|| Error("a `testcase` without its end".into()))?,
        };
        cases.push(case(tag, body)?);
        rest = after;
    }
    Ok(cases)
}
