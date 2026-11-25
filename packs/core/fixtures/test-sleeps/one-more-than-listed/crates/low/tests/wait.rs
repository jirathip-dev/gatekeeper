use std::time::Duration;
#[test]
fn waits() {
    std::thread::sleep(Duration::from_millis(10));
    std::thread::sleep(Duration::from_millis(10));
}
