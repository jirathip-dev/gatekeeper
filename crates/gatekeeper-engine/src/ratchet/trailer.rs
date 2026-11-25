/// One `<word>: [<record>] <old> → <new> (#<issue>, q<N>)` line of a commit body.
#[derive(Debug, PartialEq, Eq)]
pub struct Trailer {
    pub record: Option<String>,
    pub old: u32,
    pub new: u32,
    pub issue: u32,
    pub ask: u32,
}

fn parse(line: &str, word: &str) -> Option<Trailer> {
    let rest = line.trim().strip_prefix(word)?.strip_prefix(": ")?;
    let (left, rest) = rest.split_once(" → ")?;
    let (record, old) = match left.rsplit_once(' ') {
        Some((r, old)) if !r.trim().is_empty() && !r.contains(' ') => (Some(r.to_owned()), old),
        Some(_) => return None,
        None => (None, left),
    };
    let (new, refs) = rest.split_once(" (#")?;
    let (issue, ask) = refs.strip_suffix(')')?.split_once(", q")?;
    let num = |t: &str| {
        t.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| t.parse().ok())?
    };
    Some(Trailer {
        record,
        old: num(old)?,
        new: num(new)?,
        issue: num(issue)?,
        ask: num(ask)?,
    })
}

/// Every well-formed `<word>:` trailer in the commit bodies, in order.
#[must_use]
pub fn find(bodies: &str, word: &str) -> Vec<Trailer> {
    bodies.lines().filter_map(|l| parse(l, word)).collect()
}
