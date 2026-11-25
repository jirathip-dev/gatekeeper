#[test]
fn clock() {
    let start = std::time::Instant::now();
    assert!(start.elapsed().as_secs() < 1);
}
