use gatekeeper_model::Counted;

/// `n` rounded up to a multiple of the step: the ceiling `baseline` writes.
#[must_use]
pub fn fits(n: u32, step: u32) -> u32 {
    n.div_ceil(step.max(1)) * step.max(1)
}

/// A count against the lowest ceiling of its folder: none, over it, or a whole step under it is a finding.
#[must_use]
pub fn ceiling(c: &Counted, lowest: Option<u32>) -> Vec<String> {
    let (name, n, unit) = (&c.name, c.n, c.unit);
    let lower = format!("./.gatekeeper/run baseline {}", c.gate);
    let found = match lowest {
        None => format!("{name}: no ceiling: {lower}"),
        Some(top) if n > top => format!("{name}: {n} {unit}, over the ceiling {top}"),
        Some(top) if n + c.step <= top => {
            format!("{name}: {n} {unit}, a step under the ceiling {top}: {lower}")
        }
        Some(_) => return Vec::new(),
    };
    vec![found]
}
