#![cfg(target_os = "linux")]

use glib::prelude::ToVariant;

// Exercises the out-parameter used by every string iterator entry point.
#[test]
fn variant_string_iterator_reads_both_ends() {
    let variant = vec!["first", "middle", "last"].to_variant();
    let iter = || variant.array_iter_str().unwrap();
    assert_eq!(iter().collect::<Vec<_>>(), ["first", "middle", "last"]);
    assert_eq!(
        iter().rev().collect::<Vec<_>>(),
        ["last", "middle", "first"]
    );
    assert_eq!(iter().nth(1), Some("middle"));
    assert_eq!(iter().nth_back(1), Some("middle"));
    assert_eq!(iter().last(), Some("last"));
}
