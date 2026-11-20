use super::members;
use crate::{Error, Result};
use gatekeeper_model::Glob;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn local_packages(text: &str) -> impl Iterator<Item = &str> {
    text.split(".package(").skip(1).filter_map(|rest| {
        let call = rest.split(')').next()?;
        call.split_once("path:")?.1.split('"').nth(1)
    })
}

fn join(dir: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|p| *p != ".").collect();
    for part in rel.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    Some(if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    })
}

pub(super) fn read(root: &Path) -> Result<Vec<(String, BTreeSet<String>)>> {
    let anywhere = Glob::try_from("**".to_owned()).map_err(Error)?;
    let mut dirs = members(root, &[anywhere], &[], "Package.swift")?;
    if root.join("Package.swift").is_file() {
        dirs.push(".".to_owned());
    }
    dirs.into_iter()
        .map(|dir| {
            let path = root.join(&dir).join("Package.swift");
            let text = fs::read_to_string(&path).map_err(|e| Error::at(&path, e))?;
            let needs = local_packages(&text).filter_map(|rel| join(&dir, rel));
            Ok((dir.clone(), needs.collect()))
        })
        .collect()
}
