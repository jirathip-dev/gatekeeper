use std::time::Duration;

pub fn in_ci() -> bool {
    std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true")
}

pub fn assert_under(took: Duration, limit: Duration, what: &str) {
    if in_ci() {
        assert!(took < limit, "{what}: took {took:?}, limit {limit:?}");
    } else {
        eprintln!("{what}: took {took:?}, limit {limit:?} asserted in CI only");
    }
}
