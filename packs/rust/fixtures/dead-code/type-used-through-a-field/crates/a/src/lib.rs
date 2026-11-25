pub struct Inner(pub u8);

pub struct Outer {
    pub inner: Inner,
}

#[must_use]
pub fn outer() -> Outer {
    Outer { inner: Inner(1) }
}
