use apeireth_verify as v;

v::regression_assert!(
    SMOKE_A,
    "smoke",
    "smoke InRange",
    InRange {
        name: "smoke::a",
        value: 1.0_f64,
        min: 0.0_f64,
        max: 1.0_f64
    }
);
v::regression_assert!(
    SMOKE_B,
    "smoke",
    "smoke Idempotent",
    Idempotent {
        name: "smoke::b",
        first: "x",
        second: "x"
    }
);
v::register_all_in_crate!(SMOKE_A, SMOKE_B);

#[test]
fn macro_works() {
    __register_all_asserts();
    let count = v::assertion_count();
    eprintln!("after-macro count = {count}");
    assert!(count >= 2, "expected >= 2 assertions, got {count}");
    let (p, t) = v::verify_all().unwrap();
    assert_eq!(p, t);
    assert!(t >= 2);
}
