// an old comment
fn old() {}
// SAFETY: the pointer is valid
unsafe { f() }
/// One line.
pub fn a() {}
/// First.
/// Second.
pub fn b() {}
fn c() {
    // new
}
/* block
  still */
fn d() { let x = 1; // trailing
    let s = 2; /* inline */ }
const U: &str = "https://x // y /* z";
const R: &str = r#"a " // b"#;
const C: char = '"'; // after a char
fn e<'a>(v: &'a str) -> &'a str { v }
