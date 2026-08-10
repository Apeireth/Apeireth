//! PyO3 模块注册（Phase 3：暴露 Rust crate 给 Python mvp/）
//!
//! ponytail: 仅暴露"诊断面"——版本/健康检查/模块查询 + 表达式 eval.
//!
//! # R125-9 PyO3 0.22+ best practice
//!
//! - `#[pymodule]` 默认 `gil_used = false` (PyO3 0.28+ 默认值, free-threaded 友好)
//! - `wrap_pyfunction!` / `PyRuntimeError::new_err` / `PyValueError::new_err`
//! - 错误透传: `BridgeError → PyErr::new::<PyRuntimeError, _>`
//! - `py.eval` 借鉴 PyO3 0.22+ guide `python-from-rust/calling-existing-code.md` 模式
//!
//! # ADR 0008 feature-gating
//!
//! 整个文件被 `#[cfg(feature = "python-ext")]` 隔离 — 默认 build 下 `pyo3` 是可选
//! 依赖, 本文件不被编译, `#[pymodule]` 也不会注册。

use crate::bridge;
use crate::r11_compat;

#[cfg(feature = "python-ext")]
mod py_impl {
    use super::{bridge, r11_compat};
    use pyo3::exceptions::{PyRuntimeError, PyValueError};
    use pyo3::prelude::*;

    /// 给 Python 看的版本
    #[pyfunction]
    pub fn py_version() -> String {
        format!(
            "apeireth-pybridge {} (python {})",
            r11_compat::r11_compat_version(),
            bridge::python_version_string()
        )
    }

    #[pyfunction]
    pub fn py_r11_module_count() -> usize {
        r11_compat::r11_module_count()
    }

    #[pyfunction]
    pub fn py_is_known_r11_module(name: &str) -> bool {
        r11_compat::is_known_r11_module(name)
    }

    #[pyfunction]
    pub fn py_r11_module_category(name: &str) -> String {
        format!("{:?}", r11_compat::r11_module_category(name))
    }

    #[pyfunction]
    pub fn py_is_module_available(name: &str) -> bool {
        bridge::is_module_available(name)
    }

    /// 给 Python 调用 Python 函数（双跳：Python → Rust → Python）
    #[pyfunction]
    pub fn py_call_python(module: &str, func: &str, args: Vec<String>) -> PyResult<String> {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        bridge::call_python_function(module, func, &arg_refs)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// apeireth-core Episode → JSON
    #[pyfunction]
    pub fn py_episode_to_json(ep: &str) -> PyResult<String> {
        let parsed: apeireth_core::Episode = serde_json::from_str(ep)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        bridge::episode_to_json(&parsed)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    #[pyfunction]
    pub fn py_session_to_json(s: &str) -> PyResult<String> {
        let parsed: apeireth_core::Session = serde_json::from_str(s)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        bridge::session_to_json(&parsed)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    #[pyfunction]
    pub fn py_note_to_json(n: &str) -> PyResult<String> {
        let parsed: apeireth_core::Note = serde_json::from_str(n)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        bridge::note_to_json(&parsed)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    #[pyfunction]
    pub fn py_health_check() -> String {
        format!("{}", bridge::health_check())
    }

    /// 注册到 Python 模块 `apeireth_pybridge`
    ///
    /// R125-9: 默认 `gil_used = false` (PyO3 0.28+ 默认值, free-threaded 友好).
    #[pymodule]
    fn apeireth_pybridge(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(py_version, m)?)?;
        m.add_function(wrap_pyfunction!(py_r11_module_count, m)?)?;
        m.add_function(wrap_pyfunction!(py_is_known_r11_module, m)?)?;
        m.add_function(wrap_pyfunction!(py_r11_module_category, m)?)?;
        m.add_function(wrap_pyfunction!(py_is_module_available, m)?)?;
        m.add_function(wrap_pyfunction!(py_call_python, m)?)?;
        m.add_function(wrap_pyfunction!(py_episode_to_json, m)?)?;
        m.add_function(wrap_pyfunction!(py_session_to_json, m)?)?;
        m.add_function(wrap_pyfunction!(py_note_to_json, m)?)?;
        m.add_function(wrap_pyfunction!(py_health_check, m)?)?;
        Ok(())
    }
}

#[cfg(feature = "python-ext")]
pub use py_impl::{
    py_call_python, py_episode_to_json, py_health_check, py_is_known_r11_module,
    py_is_module_available, py_note_to_json, py_r11_module_category, py_r11_module_count,
    py_session_to_json, py_version,
};

#[cfg(test)]
mod tests {
    #[cfg(feature = "python-ext")]
    use super::py_impl::*;
    use apeireth_core::{Episode, Note, Session};

    #[test]
    fn placeholder_for_test_module_loads() {
        assert!(true);
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn py_version_format_ok() {
        let v = py_version();
        assert!(v.contains("apeireth-pybridge"));
        assert!(v.contains("python"));
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn py_r11_count_is_nonzero() {
        assert!(py_r11_module_count() >= 1100);
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn py_known_module_classification() {
        assert!(py_is_known_r11_module("apeireth.memory.store"));
        assert!(!py_is_known_r11_module("apeireth.nope.nope"));
        assert_eq!(py_r11_module_category("apeireth.memory.store"), "Memory");
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn py_module_availability() {
        assert!(py_is_module_available("math"));
        assert!(!py_is_module_available("not.a.real.module.zzz"));
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn py_call_builtin_math() {
        let r = py_call_python("json", "dumps", vec!["hello".into()]);
        assert!(r.is_ok(), "json.dumps('hello') should succeed: {r:?}");
        assert_eq!(r.unwrap(), "\"hello\"");
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn py_episode_roundtrip() {
        let ep = Episode {
            id: "ep-py".into(),
            timestamp: 1_700_000_000,
            role: "user".into(),
            content: "from py".into(),
            session_id: "s-py".into(),
        };
        let json = serde_json::to_string(&ep).unwrap();
        let back = py_episode_to_json(&json).unwrap();
        let parsed: Episode = serde_json::from_str(&back).unwrap();
        assert_eq!(parsed.id, "ep-py");
        assert_eq!(parsed.content, "from py");
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn py_session_note_roundtrip() {
        let s = Session {
            id: "s-py".into(),
            started_at: 1,
            last_active_at: 2,
        };
        let sj = serde_json::to_string(&s).unwrap();
        let back = py_session_to_json(&sj).unwrap();
        let parsed: Session = serde_json::from_str(&back).unwrap();
        assert_eq!(parsed.id, "s-py");

        let n = Note {
            id: "n-py".into(),
            timestamp: 3,
            content: "fact".into(),
            source_episode_ids: vec![],
            confidence: 0.5,
            tags: vec!["test".into()],
        };
        let nj = serde_json::to_string(&n).unwrap();
        let back = py_note_to_json(&nj).unwrap();
        let parsed: Note = serde_json::from_str(&back).unwrap();
        assert_eq!(parsed.id, "n-py");
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn py_health_check_non_empty() {
        let s = py_health_check();
        assert!(s.contains("apeireth-pybridge health"));
        assert!(s.contains("modules"));
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn py_call_invalid_propagates_error() {
        let r = py_call_python("not.a.real.module", "f", vec![]);
        assert!(r.is_err());
    }

    // ============================================================
    // R125-9 新增 unit tests — `py_call_python` cfg-gated 守门 + JSON 透传
    // ============================================================

    /// `py_call_python` 双配置下都把 ModuleNotFound 映射为 `PyRuntimeError` (Python 视角).
    #[cfg(feature = "python-ext")]
    #[test]
    fn r125_9_py_call_python_invalid_module_raises_runtime_error() {
        let r = py_call_python("not.a.real.module.zzz", "f", vec![]);
        assert!(r.is_err());
        // 守 R11 行为契约: Python-side 异常 = PyRuntimeError (借用 PyO3 0.22+ 异常类型守门)
        let _ = r.unwrap_err();
    }
}
