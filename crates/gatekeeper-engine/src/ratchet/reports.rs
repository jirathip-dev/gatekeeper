/// cargo-machete's text report: one `<manifest>: unused dependency: <name>` line per name it lists.
#[must_use]
pub fn machete(stdout: &str) -> Vec<String> {
    let mut manifest = None;
    let mut out = Vec::new();
    for line in stdout.lines() {
        match (line.strip_prefix('\t'), manifest) {
            (Some(name), Some(m)) => out.push(format!("{m}: unused dependency: {}", name.trim())),
            (Some(_), None) => {}
            (None, _) => {
                let m = line
                    .split_once(" -- ")
                    .and_then(|(_, m)| m.strip_suffix(':'));
                manifest = m.map(|m| m.trim_start_matches("./"));
            }
        }
    }
    out
}
