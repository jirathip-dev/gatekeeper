use gatekeeper_model::PrivateNames;
use std::fmt::Display;
use std::marker::PhantomData;

struct Probe<T>(PhantomData<T>);

trait Undisplayable {
    fn displays(&self) -> bool {
        false
    }
}

impl<T> Undisplayable for Probe<T> {}

impl<T: Display> Probe<T> {
    fn displays(&self) -> bool {
        matches!(self, Probe(PhantomData))
    }
}

#[test]
fn the_probe_sees_a_display_impl() {
    assert!(Probe::<String>(PhantomData).displays());
}

#[test]
fn private_names_have_no_display() {
    assert!(!Probe::<PrivateNames>(PhantomData).displays());
}

#[test]
fn debug_prints_the_count_and_no_name() {
    let names = PrivateNames::parse("quillmoor\n  north-pier  \n\n").expect("names");
    assert_eq!(names.len(), 2);
    let shown = format!("{names:?} {names:#?}");
    assert_eq!(shown, "PrivateNames(2 redacted) PrivateNames(2 redacted)");
}

#[test]
fn a_list_with_no_name_is_none() {
    assert!(PrivateNames::parse(" \n\n").is_none());
}
