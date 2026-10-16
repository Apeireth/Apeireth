//! R177 core organ smoke checks (W11)
//!
//! **诚实声明 (O-5, 2026-09-24 审计 M7)**: 本模块**不是** Kani 形式化证明。
//! 原文件以 "Kani proofs" 命名, 内容是 `String::len()==4` 级填充测试 ——
//! 不断言任何 domain 不变量, 却以形式化验证的名义虚增覆盖面, 属"命名强于
//! 实现"。现改名 smoke 并如实标注:
//! - `#[test]` 函数是编译/链接 smoke (证明模块可构建, 0 domain 断言);
//! - `#[cfg(kani)]` harness 引用的 Kani proof 在本仓不存在, 同样不是证明。
//!
//! 后续若要真形式化验证, 应对 domain 不变量书写 (如
//! `for all Lifecycle s: can_transition_to(s, s) == false`、电子环 11 节点),
//! 而不是留在本文件里。

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_core_smoke_01_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_core_smoke_02_string_basic() {
    let s = String::from("core");
    assert_eq!(s.len(), 4);
}

#[test]
fn r177_core_smoke_03_vec_basic() {
    let v: Vec<u32> = vec![1, 2];
    assert_eq!(v.len(), 2);
}

#[test]
fn r177_core_smoke_04_option_basic() {
    let o: Option<u32> = None;
    assert!(o.is_none());
}

#[test]
fn r177_core_smoke_05_result_basic() {
    let r: Result<u32, &str> = Err("x");
    assert!(r.is_err());
}

#[cfg(kani)]
#[kani::proof]
fn r177_core_kani_01_module_compiles() {
    // 非证明: 见模块 doc 诚实声明 (Kani harness 在本仓不存在)。
    let _ = std::mem::size_of::<u64>();
}

#[cfg(kani)]
#[kani::proof]
fn r177_core_kani_02_basic() {
    // 非证明: 见模块 doc 诚实声明。
    let v: Vec<u32> = vec![1];
    assert_eq!(v.len(), 1);
}
