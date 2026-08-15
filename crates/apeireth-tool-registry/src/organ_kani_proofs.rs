//! R177 tool-registry organ Kani proofs (W3)

#![allow(missing_docs)]

use crate::registry::{MockSyncTool, ToolRegistry};
use std::sync::Arc;

#[test]
fn r177_reg_01_new_empty() {
    let reg = ToolRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
    assert_eq!(reg.list().len(), 0);
}

#[test]
fn r177_reg_02_register_get() {
    let reg = ToolRegistry::new();
    let t = Arc::new(MockSyncTool { name: "echo".into() });
    reg.register("echo".into(), t.clone());
    assert_eq!(reg.len(), 1);
    let got = reg.get("echo");
    assert!(got.is_some());
}

#[test]
fn r177_reg_03_unregister_returns_tool() {
    let reg = ToolRegistry::new();
    let t = Arc::new(MockSyncTool { name: "x".into() });
    reg.register("x".into(), t);
    let removed = reg.unregister("x");
    assert!(removed.is_some());
    assert_eq!(reg.len(), 0);
}

#[test]
fn r177_reg_04_unregister_nonexistent() {
    let reg = ToolRegistry::new();
    let removed = reg.unregister("nope");
    assert!(removed.is_none());
}

#[test]
fn r177_reg_05_register_overwrite() {
    let reg = ToolRegistry::new();
    reg.register("a".into(), Arc::new(MockSyncTool { name: "a".into() }));
    reg.register("a".into(), Arc::new(MockSyncTool { name: "a-v2".into() }));
    assert_eq!(reg.len(), 1, "同名注册应覆盖 (size 不变)");
}

#[test]
fn r177_reg_06_list_sorted() {
    let reg = ToolRegistry::new();
    reg.register("z".into(), Arc::new(MockSyncTool { name: "z".into() }));
    reg.register("a".into(), Arc::new(MockSyncTool { name: "a".into() }));
    reg.register("m".into(), Arc::new(MockSyncTool { name: "m".into() }));
    let names = reg.list();
    assert_eq!(names, vec!["a", "m", "z"]);
}

#[test]
fn r177_reg_07_clear() {
    let reg = ToolRegistry::new();
    reg.register("a".into(), Arc::new(MockSyncTool { name: "a".into() }));
    reg.register("b".into(), Arc::new(MockSyncTool { name: "b".into() }));
    reg.clear();
    assert_eq!(reg.len(), 0);
    assert!(reg.is_empty());
}

#[test]
fn r177_reg_08_default_empty() {
    let reg = ToolRegistry::default();
    assert!(reg.is_empty());
}

#[test]
fn r177_reg_09_register_get_unregister_idempotent() {
    let reg = ToolRegistry::new();
    let t = Arc::new(MockSyncTool { name: "i".into() });
    reg.register("i".into(), t);
    assert!(reg.get("i").is_some());
    reg.unregister("i");
    assert!(reg.get("i").is_none());
    // 重复 unregister 不 panic
    assert!(reg.unregister("i").is_none());
}

#[test]
fn r177_reg_10_is_empty_after_register() {
    let reg = ToolRegistry::new();
    assert!(reg.is_empty());
    reg.register("t".into(), Arc::new(MockSyncTool { name: "t".into() }));
    assert!(!reg.is_empty());
    reg.unregister("t");
    assert!(reg.is_empty());
}

#[cfg(kani)]
#[kani::proof]
fn r177_reg_kani_01_new_empty_invariants() {
    let reg = ToolRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_reg_kani_02_register_get_idempotent() {
    let reg = ToolRegistry::new();
    let t = Arc::new(MockSyncTool { name: "k".into() });
    reg.register("k".into(), t);
    assert!(reg.get("k").is_some());
}
