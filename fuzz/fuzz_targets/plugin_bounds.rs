#![no_main]

//! fuzz: 插件边界校验面（`foundation/plugin::bounds`）。
//!
//! **面**：插件系统的**沙箱边界第一道门** —— 扩展名/版本/权限/资源上限的判定。
//! 这些函数目前是**已导出但零调用点**（插件接线未做），所以 fuzz 它的价值不是
//! "防已接线的洞"，而是**在接线之前把边界验干净** —— 边界判定的 bug 一旦接线就会
//! 直接变成越权。
//!
//! **判据（契约断言）**：
//! - `check_permissions` 返回 `Ok` ⇒ 调用方授权集合**确实包含**每一项所需权限；
//! - `check_input_size` / `check_output_size` 返回 `Ok` ⇒ 实际大小**不超过**上限；
//!   （这两条正是该模块自己的语义，写成断言才能抓"放行了不该放行的值"）
//! - 全部不得 panic。

use std::collections::BTreeSet;

use apeireth_plugin::bounds::{
    check_call, check_input_size, check_output_size, check_permissions, validate_extension_name,
    validate_permissions, validate_version,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // 1) 名称/版本/权限的形状校验：不得 panic
    let _ = validate_extension_name(s);
    let _ = validate_version(s);

    // 2) 权限判定 + 契约断言：Ok ⇒ 每项所需权限都在调用方授权里
    let perms: Vec<String> = s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
    let _ = validate_permissions(&perms);

    let mut caller: BTreeSet<String> = BTreeSet::new();
    for p in &perms {
        caller.insert(p.clone());
    }
    if check_permissions("p", &perms, &caller).is_ok() {
        for need in &perms {
            assert!(
                caller.contains(need),
                "check_permissions 放行了未授权的权限: need={need:?} caller={caller:?}"
            );
        }
    }

    // 3) 资源上限判定 + 契约断言：Ok ⇒ 未超限
    //    构造一个由输入决定的上限，让比较边界随输入变化。
    let cap = (s.len() % 64) as usize;
    let bounds = apeireth_plugin::bounds::ResourceBounds {
        max_input_bytes: cap,
        max_output_bytes: cap,
        ..Default::default()
    };
    let n = (s.len() % 128) as usize;
    if check_input_size("p", n, &bounds).is_ok() {
        assert!(n <= bounds.max_input_bytes, "check_input_size 放行了超限输入: {n} > {}", bounds.max_input_bytes);
    }
    if check_output_size("p", n, &bounds).is_ok() {
        assert!(n <= bounds.max_output_bytes, "check_output_size 放行了超限输出: {n} > {}", bounds.max_output_bytes);
    }

    // 4) 组合判定
    let _ = check_call("p", &perms, &caller, n, &bounds);
});
