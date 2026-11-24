use super::scope::Doc;
use crate::{Error, Result};
use std::path::Path;

#[derive(Default)]
pub(crate) struct Text {
    pub spans: Vec<String>,
    pub links: Vec<String>,
}

fn add(to: &mut Vec<String>, s: &str) {
    if !to.iter().any(|t| t == s) {
        to.push(s.to_owned());
    }
}

pub(crate) fn read(root: &Path, doc: &Doc) -> Result<Text> {
    let path = root.join(&doc.path);
    let text = std::fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
    let (mut out, mut fence) = (Text::default(), false);
    for l in text.lines() {
        let edge = l.trim_start().starts_with("```");
        fence ^= edge;
        if fence || edge {
            continue;
        }
        for s in l.split('`').skip(1).step_by(2) {
            add(&mut out.spans, s);
        }
        let links = l.split("](").skip(1);
        for t in links.filter_map(|r| r.split([')', ' ', '#']).next()) {
            if !t.is_empty() && !t.contains(':') {
                add(&mut out.links, t);
            }
        }
    }
    Ok(out)
}
