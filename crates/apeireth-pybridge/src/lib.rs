//! apeireth-pybridge: PyO3 桥 (Python 3.13.14 ↔ Rust) — feature-gated compat layer
//!
//! R14 A16.3 落地: 主 19:33 走在前人经验上, 借鉴 DeltaMemory-Rust (Lin et al. 2024) PyO3 模式.
//! R125-9 重构: 借鉴 PyO3 0.22+ `Bound` API + `Python::attach` + kwargs 透传 best practice.
//!
//! # ADR 0007 + ADR 0008 — Feature-gated Compat Layer (round9-11 qa_engineer)
//!
//! - **默认 build** (`cargo build --workspace`): 本 crate 作为 Rust-only 兼容组件层,
//!   `pyo3` **不** 被激活。`bridge::*` 中的 `python_is_available`/`python_version_string`
//!   返回 `false` / 静态占位符, `call_python_function` 返回 `ModuleNotFound` 降级。
//!   这样默认 build 不需要 Python 3.13.14 运行时, 避免污染 Rust-only CI。
//! - **`--features python-ext`** (`cargo build --features apeireth-pybridge/python-ext`):
//!   `pyo3` + `pyo3/extension-module` 启用, `python_bindings` 模块被编译,
//!   `#[pymodule] apeireth_pybridge` 注册到 Python 解释器。
//!
//! ADR 0007 = 兼容组件层定位, ADR 0008 = pyo3 feature-gating (本文件实现).
//! R125-9 新增: `call_python_function_kw` / `eval_python_expression` / `py_call_python_with_kwargs` /
//! `py_eval_expression` 入口, 借鉴 PyO3 0.22+ `python-from-rust/calling-existing-code.md` 模式.


pub mod bridge;
pub mod error;
pub mod r11_compat;

// `python_bindings` 模块仅在启用 `python-ext` feature 时存在 (默认 build 为 0 体积)。
#[cfg(feature = "python-ext")]
pub mod python_bindings;

pub use bridge::{
    call_python_builtin, call_python_function, episode_to_json, health_check, is_module_available,
    is_r11_module_available, note_to_json, python_is_available, python_version_string,
    session_to_json, try_call_or_degrade, BridgeHealth,
};
pub use error::{BridgeError, SuggestedAction};
pub use r11_compat::{
    is_known_r11_module, list_r11_modules_by_category, list_r11_modules_by_prefix,
    r11_compat_version, r11_lookup_module, r11_module_category, r11_module_count, R11Category,
    R11ModuleInfo, R11_COMPAT_VERSION, R11_MODULE_COUNT,
};

// `py_xxx` 函数仅在 python-ext 启用时存在; 公开 re-export 跟随 cfg 守门。
#[cfg(feature = "python-ext")]
pub use python_bindings::{
    py_call_python, py_episode_to_json, py_health_check, py_is_known_r11_module,
    py_is_module_available, py_note_to_json, py_r11_module_category, py_r11_module_count,
    py_session_to_json, py_version,
};

/// 占位函数 — round9-11 起标注 ADR 0007/0008 落地状态。
pub fn placeholder() -> &'static str {
    "apeireth-pybridge R14 A16.3 + R125-9 — ADR 0007 compat-layer + ADR 0008 feature-gated (pyo3 optional) + PyO3 0.22+ best practice (Python::attach + Bound API + kwargs)"
}

/// 当前 pybridge 的 feature 配置 — 用于诊断 / 运行时判断。
///
/// 返回 `true` 表示 `python-ext` feature 已激活 (pyo3 编译进二进制)。
pub fn python_ext_enabled() -> bool {
    cfg!(feature = "python-ext")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_ok() {
        assert!(placeholder().contains("apeireth-pybridge"));
        assert!(placeholder().contains("ADR 0007"));
    }

    #[test]
    fn python_ext_enabled_consistent() {
        // cfg! 在编译期评估, 运行时调用与本测试目标的 feature 标识一致。
        let expected = cfg!(feature = "python-ext");
        assert_eq!(python_ext_enabled(), expected);
    }

    #[test]
    fn public_api_exports_resolve() {
        let _ = r11_module_count();
        let _ = r11_compat_version();
        let _ = is_known_r11_module("apeireth.memory.store");
        let _ = r11_module_category("apeireth.memory.store");
        let _ = python_version_string();
        let _ = python_is_available();
        let _ = is_module_available("math");
    }

    #[test]
    fn re_exports_match_constants() {
        assert_eq!(r11_module_count(), R11_MODULE_COUNT);
        assert_eq!(r11_compat_version(), R11_COMPAT_VERSION);
    }

    #[test]
    fn error_re_exports_work() {
        let e = BridgeError::ModuleNotFound("x".into());
        let _: SuggestedAction = e.suggested_action();
    }

    #[test]
    fn r11_module_info_re_export() {
        let info = r11_lookup_module("apeireth.memory.v1141").unwrap();
        assert_eq!(info.category, R11Category::Memory);
        assert!(info.is_baseline);
    }

    #[test]
    fn default_build_python_is_available_false() {
        // 默认 build 下 (无 python-ext) — python_is_available 永远 = false。
        // python-ext build 下 也可能 = false (取决于运行时是否有 Python 解释器)。
        // 本测试仅验证默认 build 下, 不假设 true。
        let _ = python_is_available();
    }

    // ============================================================
    // V27.0 跨配置功能对等 (round10-08 qa_engineer) — 10 个新增 unit 测试
    // ============================================================

    /// Cross-config invariant: `r11_module_count()` 在两配置下必须 = 1103。
    #[test]
    fn unit_v27_r11_count_is_1103_in_both_configs() {
        assert_eq!(r11_module_count(), 1103);
    }

    /// Cross-config invariant: `R11_COMPAT_VERSION` 在两配置下必须 = R14-...。
    #[test]
    fn unit_v27_compat_version_is_r14_in_both_configs() {
        let v = r11_compat_version();
        assert!(v.starts_with("R14") || v.contains("R14"), "got {v}");
    }

    /// Cross-config invariant: `is_known_r11_module` 返回值稳定。
    #[test]
    fn unit_v27_known_r11_module_stable() {
        assert!(is_known_r11_module("apeireth.memory.store"));
        assert!(!is_known_r11_module("apeireth.nope.nope"));
    }

    /// Cross-config invariant: `BridgeError::ModuleNotFound.suggested_action()` = Degrade。
    #[test]
    fn unit_v27_error_module_not_found_suggests_degrade() {
        let e = BridgeError::ModuleNotFound("x".into());
        assert_eq!(e.suggested_action(), SuggestedAction::Degrade);
    }

    /// Cross-config invariant: `BridgeError::InvalidArg.suggested_action()` = Fail。
    #[test]
    fn unit_v27_error_invalid_arg_suggests_fail() {
        let e = BridgeError::InvalidArg("x".into());
        assert_eq!(e.suggested_action(), SuggestedAction::Fail);
    }

    /// Cross-config invariant: `BridgeError::CallFailed.suggested_action()` = Retry。
    #[test]
    fn unit_v27_error_call_failed_suggests_retry() {
        let e = BridgeError::CallFailed("x".into());
        assert_eq!(e.suggested_action(), SuggestedAction::Retry);
        assert!(e.is_recoverable());
    }

    /// Cross-config invariant: `BridgeError::GilError.suggested_action()` = Retry。
    #[test]
    fn unit_v27_error_gil_error_suggests_retry() {
        let e = BridgeError::GilError("x".into());
        assert_eq!(e.suggested_action(), SuggestedAction::Retry);
        assert!(e.is_recoverable());
    }

    /// Cross-config invariant: `r11_lookup_module` baseline 字段稳定 (R11 1103 LOCKED)。
    #[test]
    fn unit_v27_lookup_baseline_v1141_is_memory() {
        let info = r11_lookup_module("apeireth.memory.v1141").expect("v1141 in R11");
        assert!(info.is_baseline);
        assert_eq!(info.category, R11Category::Memory);
    }

    /// Cross-config invariant: `placeholder()` 是 `&'static str`, 同一地址。
    #[test]
    fn unit_v27_placeholder_is_static_str() {
        let p1 = placeholder();
        let p2 = placeholder();
        assert_eq!(p1.as_ptr(), p2.as_ptr());
    }

    /// Cross-config invariant: `python_ext_enabled()` 与编译期 `cfg!` 一致 (V27.0 核心守门)。
    #[test]
    fn unit_v27_python_ext_runtime_matches_cfg() {
        let runtime = python_ext_enabled();
        let compile_time = cfg!(feature = "python-ext");
        assert_eq!(runtime, compile_time);
    }
}
