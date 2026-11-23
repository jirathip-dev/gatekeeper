#[must_use]
pub fn first(v: &[u8]) -> Option<u8> {
    if v.is_empty() {
        return None;
    }
    // SAFETY: v is not empty, so index 0 is in bounds.
    Some(unsafe { *v.get_unchecked(0) })
}
