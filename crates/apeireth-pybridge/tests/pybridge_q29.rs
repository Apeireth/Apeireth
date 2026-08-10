//! Integration test for apeireth-pybridge (P29 恢复)

use apeireth_pybridge::{
    list_r11_modules_by_category, placeholder, python_is_available, python_version_string,
    BridgeError, BridgeHealth, R11Category, SuggestedAction, R11_MODULE_COUNT,
};

#[test]
fn q29_placeholder_returns_expected() {
    assert!(placeholder().contains("apeireth-pybridge"));
}

#[test]
fn q29_r11_module_count_is_correct() {
    assert_eq!(R11_MODULE_COUNT, 1103);
}

#[test]
fn q29_bridge_error_display_works() {
    let err = BridgeError::ModuleNotFound("test".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("module") || msg.contains("not found"));
}

#[test]
fn q29_suggested_action_variants() {
    let _ = SuggestedAction::Retry;
    let _ = SuggestedAction::Degrade;
    let _ = SuggestedAction::Fail;
}

#[test]
fn q29_python_is_available_returns_bool() {
    let _: bool = python_is_available();
}

#[test]
fn q29_python_version_string_not_empty() {
    let version = python_version_string();
    assert!(!version.is_empty());
}

#[test]
fn q29_bridge_health_struct_constructible() {
    let health = BridgeHealth {
        python_version: "n/a".to_string(),
        r11_compat_version: "0.1.0",
        r11_module_count: 0,
        python_available: false,
    };
    assert_eq!(health.python_available, false);
    assert_eq!(health.r11_module_count, 0);
}

#[test]
fn q29_r11_category_variants() {
    let _: R11Category = R11Category::Memory;
    let _: R11Category = R11Category::Identity;
    let _: R11Category = R11Category::Asi;
    let _: R11Category = R11Category::Philosophy;
    let _: R11Category = R11Category::Tools;
}

#[test]
fn q29_placeholder_idempotent() {
    assert_eq!(placeholder(), placeholder());
}

#[test]
fn q29_module_list_by_category() {
    let modules: Vec<String> = list_r11_modules_by_category(R11Category::Memory);
    let _ = modules;
}
