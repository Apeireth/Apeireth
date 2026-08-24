//! walk_all_crates — 遍历全部下游 crate, 触发每个 crate 顶层的 `__register_all_asserts`,
//! 然后用 `apeireth_verify::verify_all` 收集并打印结果.

fn main() {
    apeireth_verify::reset_for_tests();

    apeireth_core::__register_all_asserts();
    apeireth_council::__register_all_asserts();
    apeireth_sovereignty::__register_all_asserts();
    apeireth_supervisor::__register_all_asserts();
    apeireth_cognition::__register_all_asserts();
    apeireth_constraint::__register_all_asserts();

    let count = apeireth_verify::assertion_count();
    println!("[walk_all_crates] registered = {count}");

    match apeireth_verify::verify_all() {
        Ok((p, t)) => {
            println!("[walk_all_crates] verify_all OK: {p}/{t} passed");
            assert_eq!(p, t, "all registered assertions must pass");
        }
        Err(errs) => {
            eprintln!("[walk_all_crates] verify_all FAILED:");
            for e in errs {
                eprintln!("  - {e}");
            }
            std::process::exit(1);
        }
    }
}
