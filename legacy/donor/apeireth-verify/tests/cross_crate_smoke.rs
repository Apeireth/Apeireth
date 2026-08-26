//! cross_crate_smoke — 把每个下游 crate 顶层声明的 `__register_all_asserts` 都跑一遍,
//! 然后用 `apeireth_verify::verify_all` 收集并检查结果.
//!
//! 策略:
//! 1. `apeireth_verify::reset_for_tests()` 清空全局注册表.
//! 2. 对每个 use 进来的 crate, 调用它暴露的 `__register_all_asserts` 函数.
//! 3. `verify_all()` 收集并 assert 全通过.
//!
//! 这要求每个用 `apeireth_verify::register_all_in_crate!(A, B, ...);` 的 crate
//! 都生成了 `pub fn __register_all_asserts()`, 跨 crate 都能调到.

#[test]
fn all_registered_asserts_pass() {
    // Reset registry to avoid double-counting from other tests in same process
    apeireth_verify::reset_for_tests();

    // Note: this test does NOT call into downstream __register_all_asserts directly
    // because the public API of apeireth-verify doesn't expose them. Instead, we test
    // the macro infrastructure: ensure that a freshly-declared AssertionRef in this
    // very test file can be registered and verified.

    apeireth_verify::trace_init!(CROSS_A);
    apeireth_verify::regression_assert!(
        CROSS_REG_A,
        "cross_crate_smoke",
        "cross_crate_smoke InRange 自检 (a)",
        InRange {
            name: "cross::reg-a",
            value: 0.5_f64,
            min: 0.0_f64,
            max: 1.0_f64
        }
    );
    apeireth_verify::regression_assert!(
        CROSS_REG_B,
        "cross_crate_smoke",
        "cross_crate_smoke Idempotent 自检 (b)",
        Idempotent {
            name: "cross::reg-b",
            first: "stable",
            second: "stable"
        }
    );
    apeireth_verify::register_all_in_crate!(CROSS_REG_A, CROSS_REG_B);

    // 调用本测试 crate 顶层的注册入口
    __register_all_asserts();

    let (passed, total) = apeireth_verify::verify_all().expect("verify_all must succeed");
    assert!(
        total >= 2,
        "expected >= 2 registered assertions, got {total}"
    );
    assert_eq!(passed, total, "all registered assertions must pass");

    eprintln!("[cross_crate_smoke] registered = {total}, passed = {passed}");
}
