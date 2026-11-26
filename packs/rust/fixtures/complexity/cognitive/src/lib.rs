#[must_use]
pub fn add(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}

#[must_use]
pub fn tangled(a: u32, b: u32) -> u32 {
    match a {
        0 if b > 0 => a.wrapping_mul(0),
        1 if b > 1 => a.wrapping_mul(1),
        2 if b > 2 => a.wrapping_mul(2),
        3 if b > 3 => a.wrapping_mul(3),
        4 if b > 4 => a.wrapping_mul(4),
        5 if b > 5 => a.wrapping_mul(5),
        6 if b > 6 => a.wrapping_mul(6),
        7 if b > 7 => a.wrapping_mul(7),
        8 if b > 8 => a.wrapping_mul(8),
        9 if b > 9 => a.wrapping_mul(9),
        10 if b > 10 => a.wrapping_mul(10),
        11 if b > 11 => a.wrapping_mul(11),
        12 if b > 12 => a.wrapping_mul(12),
        13 if b > 13 => a.wrapping_mul(13),
        14 if b > 14 => a.wrapping_mul(14),
        15 if b > 15 => a.wrapping_mul(15),
        16 if b > 16 => a.wrapping_mul(16),
        17 if b > 17 => a.wrapping_mul(17),
        18 if b > 18 => a.wrapping_mul(18),
        19 if b > 19 => a.wrapping_mul(19),
        20 if b > 20 => a.wrapping_mul(20),
        21 if b > 21 => a.wrapping_mul(21),
        22 if b > 22 => a.wrapping_mul(22),
        23 if b > 23 => a.wrapping_mul(23),
        24 if b > 24 => a.wrapping_mul(24),
        25 if b > 25 => a.wrapping_mul(25),
        26 if b > 26 => a.wrapping_mul(26),
        27 if b > 27 => a.wrapping_mul(27),
        28 if b > 28 => a.wrapping_mul(28),
        29 if b > 29 => a.wrapping_mul(29),
        _ => b,
    }
}
