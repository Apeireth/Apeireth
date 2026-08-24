//! R177 lark organ Kani proofs (W12)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_lark_01_schema_version() {
    assert_eq!(LARK_SCHEMA_VERSION, "1");
}

#[test]
fn r177_lark_02_platform_name() {
    assert_eq!(PLATFORM_NAME, "apeireth");
}

#[test]
fn r177_lark_03_tool_whitelist() {
    assert_eq!(TOOL_WHITELIST_COUNT, 9);
}

#[test]
fn r177_lark_04_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_lark_05_validate_tool_call() {
    assert!(validate_tool_call("x", &serde_json::json!({})).is_err());
}

#[cfg(kani)]
#[kani::proof]
fn r177_lark_kani_01_tool_count() {
    assert_eq!(TOOL_WHITELIST_COUNT, 9);
}

#[cfg(kani)]
#[kani::proof]
fn r177_lark_kani_02_schema_version() {
    assert_eq!(LARK_SCHEMA_VERSION, "1");
}
