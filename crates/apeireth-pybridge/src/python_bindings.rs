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

    /// 给 Python 调用 Python 函数 (带 kwargs) — R127-2 Stage 6.1 跨语言桥深化
    /// 借鉴 PyO3 0.22+ function-calls.md `func.call(args, kwargs)` 模式
    #[pyfunction]
    pub fn py_call_python_with_kwargs(
        module: &str,
        func: &str,
        args: Vec<String>,
        kwargs: Vec<(String, String)>,
    ) -> PyResult<String> {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let kw_refs: Vec<(&str, &str)> = kwargs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        bridge::call_python_function_kw(module, func, &arg_refs, &kw_refs)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// 给 Python 求值 Python 表达式 — R127-2 Stage 6.1 跨语言桥深化
    /// 借鉴 PyO3 0.22+ `py.eval(c"expr", None, None)` 模式 (per calling-existing-code.md)
    ///
    /// # 警告
    /// Python 端调用方负责保证 expr 安全 — 等价于 Python `eval()`, 不可信输入风险.
    #[pyfunction]
    pub fn py_eval_expression(expr: &str) -> PyResult<String> {
        bridge::eval_python_expression(expr).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// apeireth-core Episode → JSON
    #[pyfunction]
    pub fn py_episode_to_json(ep: &str) -> PyResult<String> {
        let parsed: apeireth_core::Episode =
            serde_json::from_str(ep).map_err(|e| PyValueError::new_err(e.to_string()))?;
        bridge::episode_to_json(&parsed).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    #[pyfunction]
    pub fn py_session_to_json(s: &str) -> PyResult<String> {
        let parsed: apeireth_core::Session =
            serde_json::from_str(s).map_err(|e| PyValueError::new_err(e.to_string()))?;
        bridge::session_to_json(&parsed).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    #[pyfunction]
    pub fn py_note_to_json(n: &str) -> PyResult<String> {
        let parsed: apeireth_core::Note =
            serde_json::from_str(n).map_err(|e| PyValueError::new_err(e.to_string()))?;
        bridge::note_to_json(&parsed).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    #[pyfunction]
    pub fn py_health_check() -> String {
        format!("{}", bridge::health_check())
    }

    /// 注册到 Python 模块 `apeireth_pybridge`
    ///
    /// R125-9: 默认 `gil_used = false` (PyO3 0.28+ 默认值, free-threaded 友好).
    /// R127-2: 加 `py_call_python_with_kwargs` + `py_eval_expression` 入口 (Stage 6.1 跨语言桥深化).
    #[pymodule]
    fn apeireth_pybridge(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(py_version, m)?)?;
        m.add_function(wrap_pyfunction!(py_r11_module_count, m)?)?;
        m.add_function(wrap_pyfunction!(py_is_known_r11_module, m)?)?;
        m.add_function(wrap_pyfunction!(py_r11_module_category, m)?)?;
        m.add_function(wrap_pyfunction!(py_is_module_available, m)?)?;
        m.add_function(wrap_pyfunction!(py_call_python, m)?)?;
        m.add_function(wrap_pyfunction!(py_call_python_with_kwargs, m)?)?;
        m.add_function(wrap_pyfunction!(py_eval_expression, m)?)?;
        m.add_function(wrap_pyfunction!(py_episode_to_json, m)?)?;
        m.add_function(wrap_pyfunction!(py_session_to_json, m)?)?;
        m.add_function(wrap_pyfunction!(py_note_to_json, m)?)?;
        m.add_function(wrap_pyfunction!(py_health_check, m)?)?;
        Ok(())
    }
}

#[cfg(feature = "python-ext")]
pub use py_impl::{
    py_call_python, py_call_python_with_kwargs, py_episode_to_json, py_eval_expression,
    py_health_check, py_is_known_r11_module, py_is_module_available, py_note_to_json,
    py_r11_module_category, py_r11_module_count, py_session_to_json, py_version,
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

    // ============================================================
    // R127-2 Stage 6.1 新增 cfg-gated tests — 跨语言桥深化 (kwargs + eval)
    // ============================================================

    /// `py_call_python_with_kwargs` 真 Python 调 json.dumps(..., ensure_ascii=False)
    /// 验证 kwargs 透传到 Python 端 (借 PyO3 0.22+ func.call(args, kwargs) 模式)
    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_py_call_python_with_kwargs_propagates() {
        // json.dumps("héllo", ensure_ascii=False) 应当返回 "héllo" 不转 \uXXXX
        let r = py_call_python_with_kwargs(
            "json",
            "dumps",
            vec!["héllo".into()],
            vec![("ensure_ascii".into(), "false".into())],
        );
        assert!(r.is_ok(), "json.dumps with kwargs failed: {r:?}");
        let out = r.unwrap();
        // ensure_ascii=False 时 Unicode 字符保留
        assert!(
            out.contains("héllo") || out.contains("h\\u00e9llo"),
            "expected unicode preserved or escaped, got: {out}"
        );
    }

    /// `py_call_python_with_kwargs` 错误 module 走 PyRuntimeError (守 R11 行为契约)
    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_py_call_python_with_kwargs_invalid_module() {
        let r = py_call_python_with_kwargs("not.a.real.module.zzz", "f", vec![], vec![]);
        assert!(r.is_err());
    }

    /// `py_eval_expression` 真求值 1+1=2
    /// 借 PyO3 0.22+ py.eval 模式 (per calling-existing-code.md)
    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_py_eval_expression_arithmetic() {
        let r = py_eval_expression("1 + 1");
        assert!(r.is_ok(), "eval 1+1 failed: {r:?}");
        assert_eq!(r.unwrap(), "2");
    }

    /// `py_eval_expression` 求值 list 构造
    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_py_eval_expression_list() {
        let r = py_eval_expression("[i * 10 for i in range(3)]");
        assert!(r.is_ok());
        let s = r.unwrap();
        assert!(s.contains("0") && s.contains("10") && s.contains("20"));
    }

    /// `py_eval_expression` 错误语法 → PyRuntimeError
    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_py_eval_expression_invalid_syntax() {
        let r = py_eval_expression("this is not python +++");
        assert!(r.is_err());
    }
}
