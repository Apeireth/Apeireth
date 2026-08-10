//! 高层 Python ↔ Rust 桥
//!
//! 主 19:33 走在前人经验上：借鉴 DeltaMemory-Rust (Lin et al. 2024) PyO3 模式 +
//! PyO3 0.22+ `Bound` API best practice (per `borrowed-repos/PyO3/guide/src/python-from-rust`).
//!
//! # PyO3 0.22+ 借鉴点 (R125-9 重构)
//!
//! - `Python::with_gil` → `Python::attach` (PyO3 0.26 改名, free-threading 友好)
//! - `py.import_bound(name)` → `py.import(name)` (PyO3 0.23 重命名)
//! - `PyString::new_bound` → `PyString::new` (PyO3 0.23 重命名)
//! - `PyTuple::new_bound` → `PyTuple::new` (PyO3 0.23 重命名)
//! - `Python::version()` deprecated → `Python::version_str()` (PyO3 0.29)
//! - `e.is_instance_of::<PyImportError>` 区分 ImportError vs 其他错误
//!
//! # ADR 0008 feature-gating
//!
//! 默认 build (无 `python-ext` feature) 下, PyO3 类型与 `Python::attach` 调用被 cfg 隔离 —
//! Rust-only 函数 (`episode_to_json` 等) 仍可用, 只是在没有 Python 解释器时
//! `python_version_string` 返回静态占位符 + `python_is_available` 永远 = false +
//! `call_python_function` 返回 `ModuleNotFound` 降级。`cargo build --workspace` 默认 0 Python。

use crate::error::{BridgeError, SuggestedAction};
use crate::r11_compat;
use crate::r11_compat::R11_MODULE_COUNT;

#[cfg(feature = "python-ext")]
use pyo3::prelude::*;
#[cfg(feature = "python-ext")]
use pyo3::types::{PyAnyMethods, PyString, PyTuple};

/// `Python::attach` 薄包装 — 复用入口, 减 cfg-gated 重复 (R125-9 helper).
#[cfg(feature = "python-ext")]
fn with_python<F, R>(f: F) -> R
where
    F: FnOnce(Python<'_>) -> R,
{
    Python::attach(f)
}

/// Python 解释器版本字符串 (默认 build 返回静态占位符).
#[cfg(feature = "python-ext")]
pub fn python_version_string() -> String {
    // R125-9: PyO3 0.29+ 把 `Python::version()` 标 deprecated, 改 `version_str()` (associated fn).
    pyo3::Python::version_str().to_string()
}

#[cfg(not(feature = "python-ext"))]
pub fn python_version_string() -> &'static str {
    "pyo3 disabled (build with --features python-ext to embed Python 3.13.14)"
}

/// Python 解释器是否可用
#[cfg(feature = "python-ext")]
pub fn python_is_available() -> bool {
    with_python(|_py| -> Result<(), pyo3::PyErr> { Ok(()) }).is_ok()
}

#[cfg(not(feature = "python-ext"))]
pub fn python_is_available() -> bool {
    false
}

/// 检查模块是否可导入
#[cfg(feature = "python-ext")]
pub fn is_module_available(module_name: &str) -> bool {
    !module_name.is_empty() && with_python(|py| py.import(module_name).is_ok())
}

#[cfg(not(feature = "python-ext"))]
pub fn is_module_available(_module_name: &str) -> bool {
    false
}

/// 检查模块是否同时也是 R11 已知模块
pub fn is_r11_module_available(module_name: &str) -> bool {
    r11_compat::is_known_r11_module(module_name) && is_module_available(module_name)
}

/// 调用 Python 函数并把结果序列化为字符串
#[cfg(feature = "python-ext")]
pub fn call_python_function(
    module_name: &str,
    func_name: &str,
    args: &[&str],
) -> Result<String, BridgeError> {
    validate_args(module_name, func_name)?;
    let raw = with_python(|py| call_py_func(py, module_name, func_name, args));
    map_call_result(raw, module_name, func_name)
}

/// 默认 build (无 `python-ext`): 立即返回 `ModuleNotFound` 降级.
#[cfg(not(feature = "python-ext"))]
pub fn call_python_function(
    module_name: &str,
    func_name: &str,
    args: &[&str],
) -> Result<String, BridgeError> {
    validate_args(module_name, func_name)?;
    let _ = args;
    Err(BridgeError::ModuleNotFound(format!(
        "{module_name}: pyo3 disabled — rebuild with --features python-ext to call Python"
    )))
}

/// 调用 Python 内置模块函数（短路径）
pub fn call_python_builtin(
    module_name: &str,
    func_name: &str,
    arg: &str,
) -> Result<String, BridgeError> {
    call_python_function(module_name, func_name, &[arg])
}

/// apeireth-core Episode 序列化成 Python dict 可消费的 JSON
pub fn episode_to_json(ep: &apeireth_core::Episode) -> Result<String, BridgeError> {
    serde_json::to_string(ep).map_err(|e| BridgeError::InvalidArg(format!("Episode serialize: {e}")))
}

pub fn session_to_json(s: &apeireth_core::Session) -> Result<String, BridgeError> {
    serde_json::to_string(s).map_err(|e| BridgeError::InvalidArg(format!("Session serialize: {e}")))
}

pub fn note_to_json(n: &apeireth_core::Note) -> Result<String, BridgeError> {
    serde_json::to_string(n).map_err(|e| BridgeError::InvalidArg(format!("Note serialize: {e}")))
}

/// 安全分派：调用 Python 并按错误建议处置
pub fn try_call_or_degrade(
    module_name: &str,
    func_name: &str,
    args: &[&str],
) -> (Option<String>, SuggestedAction) {
    match call_python_function(module_name, func_name, args) {
        Ok(v) => (Some(v), SuggestedAction::Retry),
        Err(e) => (None, e.suggested_action()),
    }
}

/// 诊断报告（健康检查）
#[derive(Debug, Clone)]
pub struct BridgeHealth {
    pub python_version: String,
    pub r11_compat_version: &'static str,
    pub r11_module_count: usize,
    pub python_available: bool,
}

impl std::fmt::Display for BridgeHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge health:\n  python: {} (available={})\n  r11: {} ({} modules)",
            self.python_version,
            self.python_available,
            self.r11_compat_version,
            self.r11_module_count
        )
    }
}

pub fn health_check() -> BridgeHealth {
    BridgeHealth {
        python_version: python_version_string().to_string(),
        r11_compat_version: r11_compat::r11_compat_version(),
        r11_module_count: r11_compat::r11_module_count(),
        python_available: python_is_available(),
    }
}

pub fn r11_module_count_re_export() -> usize {
    R11_MODULE_COUNT
}

// =============================================================================
// 内部 helper (R125-9)
// =============================================================================

/// 入参校验 (cfg-无关)
fn validate_args(module_name: &str, func_name: &str) -> Result<(), BridgeError> {
    if module_name.is_empty() {
        return Err(BridgeError::InvalidArg("module_name is empty".into()));
    }
    if func_name.is_empty() {
        return Err(BridgeError::InvalidArg("func_name is empty".into()));
    }
    Ok(())
}

#[cfg(feature = "python-ext")]
fn call_py_func<'py>(
    py: Python<'py>,
    module_name: &str,
    func_name: &str,
    args: &[&str],
) -> Result<String, pyo3::PyErr> {
    let module = py.import(module_name)?;
    let func = module.getattr(func_name)?;
    let bound_args: Vec<Bound<'py, PyAny>> = args
        .iter()
        .map(|s| PyString::new(py, s).into_any())
        .collect();
    let result = func.call1(PyTuple::new(py, &bound_args)?)?;
    result
        .str()
        .map(|s| s.to_string())
        .or_else(|_| result.repr().map(|s| s.to_string()))
}

/// R125-9: 区分 ImportError (ModuleNotFound → Degrade) vs 其他 (CallFailed → Retry).
/// 守 R11 行为契约 — 借鉴 PyO3 0.22+ `PyErr::is_instance_of` 模式.
#[cfg(feature = "python-ext")]
fn map_call_result(
    raw: Result<String, pyo3::PyErr>,
    module_name: &str,
    func_name: &str,
) -> Result<String, BridgeError> {
    raw.map_err(|e| {
        pyo3::Python::attach(|py| {
            if e.is_instance_of::<pyo3::exceptions::PyImportError>(py) {
                BridgeError::ModuleNotFound(format!("{module_name}: {e}"))
            } else {
                BridgeError::CallFailed(format!("{func_name}: {e}"))
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::python_ext_enabled;

    #[test]
    fn health_check_runs() {
        let h = health_check();
        assert!(h.r11_module_count > 0);
    }

    #[test]
    fn try_call_or_degrade_propagates_failure() {
        let (result, action) = try_call_or_degrade("", "x", &["y"]);
        assert!(result.is_none());
        assert_eq!(action, SuggestedAction::Fail);
    }

    #[test]
    fn bridge_health_display_contains_r11() {
        let h = BridgeHealth {
            python_version: "3.13.14".into(),
            r11_compat_version: "0.1.0",
            r11_module_count: 1103,
            python_available: true,
        };
        let s = format!("{h}");
        assert!(s.contains("r11"));
    }

    // ============================================================
    // R125-9 新增 unit tests (5 项, cfg-无关优先)
    // ============================================================

    /// `validate_args` cfg-无关 — 双配置一致拒绝空字符串.
    #[test]
    fn r125_9_validate_args_rejects_empty() {
        for (m, f) in [("", "f"), ("m", "")] {
            let err = validate_args(m, f).expect_err("empty must error");
            assert_eq!(err.suggested_action(), SuggestedAction::Fail);
        }
        assert!(validate_args("math", "sqrt").is_ok());
    }

    /// `call_python_builtin` 透传入参校验 — 默认 build 下 builtin 也走 ModuleNotFound 降级.
    #[test]
    fn r125_9_call_python_builtin_validation_propagates() {
        let r = call_python_builtin("", "x", "y");
        assert!(r.is_err());
        assert!(!r.unwrap_err().is_recoverable());
    }

    /// `try_call_or_degrade` 双配置下都把 `InvalidArg` 映射为 (None, Fail).
    #[test]
    fn r125_9_try_call_or_degrade_invalid_arg_yields_fail() {
        let (v, action) = try_call_or_degrade("", "f", &["x"]);
        assert!(v.is_none());
        assert_eq!(action, SuggestedAction::Fail);
    }

    /// `call_python_function` 双配置下都用 `ModuleNotFound` 降级 (cfg-gated 守门).
    #[test]
    fn r125_9_call_python_function_default_build_degrades() {
        if !python_ext_enabled() {
            let r = call_python_function("json", "dumps", &["x"]);
            assert!(r.is_err());
            assert_eq!(r.unwrap_err().suggested_action(), SuggestedAction::Degrade);
        }
    }
}
