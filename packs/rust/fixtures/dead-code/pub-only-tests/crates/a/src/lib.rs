#[must_use]
pub fn api() -> u8 {
    1
}

#[must_use]
pub fn probe() -> u8 {
    2
}

#[cfg(test)]
mod tests {
    #[test]
    fn probe_is_two() {
        assert_eq!(super::probe(), 2);
    }
}
