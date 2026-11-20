#[must_use]
fn g() -> u8 {
    1
}

pub fn f() {
    let _ = g();
}
