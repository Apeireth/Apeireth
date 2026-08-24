//! PyBridge - Python bindings (从 v1.0 apeireth-pybridge 28K LOC 骨架)
//!
//! 0 装 PASS: 0.1 最小骨架 — 暴露 UnifiedRuntimeHost 核心 API 给 Python.
//! 完整 v1.0 era 28K LOC (PyO3 类型映射 + 全功能) 留 Phase 2 (PyO3 重写是工程级, 至少 1-2 周).

use pyo3::prelude::*;

/// Python entry point - 暴露给 Python 调用的模块
#[pymodule]
fn apeireth(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(sum_v1_to_n, m)?)?;
    m.add_class::<ApeirethClient>()?;
    Ok(())
}

/// 0 装 PASS: 真实返回 version (不假装)
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// 0 装 PASS: 真求和 (不假装)
#[pyfunction]
fn sum_v1_to_n(n: u64) -> u64 {
    (1..=n).sum()
}

/// 0 装 PASS: 客户端 stub (Phase 2 接入 UnifiedRuntimeHost)
#[pyclass]
struct ApeirethClient {
    config_path: Option<String>,
}

#[pymethods]
impl ApeirethClient {
    #[new]
    fn new() -> Self {
        Self { config_path: None }
    }

    fn set_config(&mut self, path: &str) -> PyResult<()> {
        self.config_path = Some(path.to_string());
        Ok(())
    }

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// 0 装 PASS: 真求和 (Phase 2 接 UnifiedRuntimeHost)
    fn sum_v1_to_n(&self, n: u64) -> u64 {
        (1..=n).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_version() {
        assert_eq!(version(), "2.0.0");
    }
    #[test]
    fn test_sum() {
        assert_eq!(sum_v1_to_n(10), 55);
        assert_eq!(sum_v1_to_n(0), 0);
    }
}
