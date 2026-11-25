pub fn small(a: u32, b: u32) -> u32 {
    let c = a * b + a - b;
    let d = c * 2 + a;
    d + c
}
