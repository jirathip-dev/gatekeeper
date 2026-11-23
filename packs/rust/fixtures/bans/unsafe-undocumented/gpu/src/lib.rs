#[must_use]
pub fn first(v: &[u8]) -> Option<u8> {
    if v.is_empty() {
        return None;
    }
    Some(unsafe { *v.get_unchecked(0) })
}
