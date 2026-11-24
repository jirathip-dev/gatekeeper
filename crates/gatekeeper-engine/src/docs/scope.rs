use crate::arch::{any, walk};
use gatekeeper_model::{AGENT_TEXT, DOCS, DocsConfig, Glob, RECORDS};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    Record,
    AgentText,
    Current,
}

pub(crate) struct Doc {
    pub path: String,
    pub class: Class,
}

fn globs(fixed: &[&str]) -> Vec<Glob> {
    let all = fixed.iter().map(|g| Glob::try_from((*g).to_owned()));
    all.filter_map(Result::ok).collect()
}

pub(crate) fn docs(root: &Path, d: &DocsConfig) -> Vec<Doc> {
    let mut paths = globs(&DOCS);
    paths.extend(d.also.iter().cloned());
    let mut records = globs(&RECORDS);
    records.extend(d.records.0.iter().cloned());
    let agent = globs(&AGENT_TEXT);
    let named: Vec<String> = paths.iter().map(ToString::to_string).collect();
    let mut files = Vec::new();
    walk(root, "", &named, &mut files);
    files.sort();
    let docs = files.into_iter().filter(|f| any(&paths, f));
    docs.map(|path| {
        let class = if any(&records, &path) {
            Class::Record
        } else if any(&agent, &path) {
            Class::AgentText
        } else {
            Class::Current
        };
        Doc { path, class }
    })
    .collect()
}

pub(crate) fn unmatched(d: &DocsConfig, docs: &[Doc]) -> Vec<String> {
    let excuses = d.records.0.iter().map(|g| ("records", g));
    let none = excuses.filter(|(_, g)| !docs.iter().any(|doc| g.matches(&doc.path)));
    none.map(|(table, g)| format!("[docs.{table}] `{g}` matches no file"))
        .collect()
}
