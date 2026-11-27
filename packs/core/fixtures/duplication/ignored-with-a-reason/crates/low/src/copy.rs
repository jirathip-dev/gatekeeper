pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
