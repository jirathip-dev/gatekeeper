use std::time::Duration;

pub fn in_ci() -> bool {
    std::env::var("GATEKEEPER_HOST").is_ok_and(|v| v == "ci")
}

pub fn assert_under(took: Duration, limit: Duration, what: &str) {
    if in_ci() {
        assert!(took < limit, "{what}: took {took:?}, limit {limit:?}");
    } else {
        eprintln!("{what}: took {took:?}, limit {limit:?} asserted in CI only");
    }
}
