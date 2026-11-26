#[must_use]
pub fn add(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}

#[allow(clippy::too_many_lines, reason = "the fixture hides it from rust/clippy")]
#[must_use]
pub fn long(a: u32) -> u32 {
    let mut n = a;
    n = n.wrapping_add(0);
    n = n.wrapping_add(1);
    n = n.wrapping_add(2);
    n = n.wrapping_add(3);
    n = n.wrapping_add(4);
    n = n.wrapping_add(5);
    n = n.wrapping_add(6);
    n = n.wrapping_add(7);
    n = n.wrapping_add(8);
    n = n.wrapping_add(9);
    n = n.wrapping_add(10);
    n = n.wrapping_add(11);
    n = n.wrapping_add(12);
    n = n.wrapping_add(13);
    n = n.wrapping_add(14);
    n = n.wrapping_add(15);
    n = n.wrapping_add(16);
    n = n.wrapping_add(17);
    n = n.wrapping_add(18);
    n = n.wrapping_add(19);
    n = n.wrapping_add(20);
    n = n.wrapping_add(21);
    n = n.wrapping_add(22);
    n = n.wrapping_add(23);
    n = n.wrapping_add(24);
    n = n.wrapping_add(25);
    n = n.wrapping_add(26);
    n = n.wrapping_add(27);
    n = n.wrapping_add(28);
    n = n.wrapping_add(29);
    n = n.wrapping_add(30);
    n = n.wrapping_add(31);
    n = n.wrapping_add(32);
    n = n.wrapping_add(33);
    n = n.wrapping_add(34);
    n = n.wrapping_add(35);
    n = n.wrapping_add(36);
    n = n.wrapping_add(37);
    n = n.wrapping_add(38);
    n = n.wrapping_add(39);
    n = n.wrapping_add(40);
    n = n.wrapping_add(41);
    n = n.wrapping_add(42);
    n = n.wrapping_add(43);
    n = n.wrapping_add(44);
    n = n.wrapping_add(45);
    n = n.wrapping_add(46);
    n = n.wrapping_add(47);
    n = n.wrapping_add(48);
    n = n.wrapping_add(49);
    n = n.wrapping_add(50);
    n = n.wrapping_add(51);
    n = n.wrapping_add(52);
    n = n.wrapping_add(53);
    n = n.wrapping_add(54);
    n = n.wrapping_add(55);
    n = n.wrapping_add(56);
    n = n.wrapping_add(57);
    n = n.wrapping_add(58);
    n = n.wrapping_add(59);
    n = n.wrapping_add(60);
    n = n.wrapping_add(61);
    n
}
