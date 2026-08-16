//! PyAny ↔ Rust serde 类型转换 (R127-2 Stage 6.1 跨语言桥深化)
//!
//! 借鉴 PyO3 0.29.2 `guide/src/conversions/traits.md` + `conversions/tables.md` +
//! `python-from-rust/function-calls.md` (args + kwargs IntoPyDict).
//!
//! 桥 API = JSON 字符串中间表达 (借 serde_json + PyO3 str/repr):
//! - 避免直接处理 PyAny lifetime 复杂 (GIL scope, Bound<'py, T>)
//! - 用 serde_json::Value 统一 Python ↔ Rust 类型 (借鉴 PyO3 IntoPy/extract trait)
//!
//! 借鉴 ID: `R127-2-BORROW-PyO3/PyO3-stage-6-1-2026-08-10` (新 ID, 不跟 R125-9 冲突).

use crate::error::BridgeError;
use serde::{de::DeserializeOwned, Serialize};

/// Rust serde 任意类型 → JSON 字符串 (cfg-无关, 默认 build 可用)
pub fn rust_to_json<T: Serialize>(v: &T) -> Result<String, BridgeError> {
    serde_json::to_string(v).map_err(|e| BridgeError::InvalidArg(format!("rust_to_json: {e}")))
}

/// JSON 字符串 → Rust serde 任意类型 (cfg-无关, 默认 build 可用)
pub fn json_to_rust<T: DeserializeOwned>(s: &str) -> Result<T, BridgeError> {
    serde_json::from_str(s).map_err(|e| BridgeError::InvalidArg(format!("json_to_rust: {e}")))
}

#[cfg(feature = "python-ext")]
pub mod py {
    //! python-ext 下的 PyAny 转换 (cfg-gated)
    //!
    //! 借鉴 PyO3 0.22+ `extract()` / `IntoPy` 模式 (conversions/traits.md),
    //! 用 serde_json::Value 作中间类型, 覆盖 bool / i64 / f64 / String / list / dict / None.

    use super::*;
    use pyo3::prelude::*;
    use pyo3::types::{PyAnyMethods, PyDictMethods, PyListMethods, PyStringMethods};
    use pyo3::IntoPyObject;

    /// 任意 Python 对象 → JSON 字符串 (str() 优先, repr() 兜底)
    pub fn pyany_to_json(any: &Bound<'_, PyAny>) -> Result<String, BridgeError> {
        any.str()
            .map(|s| s.to_string())
            .or_else(|_| any.repr().map(|s| s.to_string()))
            .map_err(|e| BridgeError::CallFailed(format!("pyany_to_json str/repr: {e}")))
    }

    /// 任意 Python 对象 → Rust serde JSON 值
    /// 借鉴 PyO3 0.22+ extract 类型链 (conversions/traits.md)
    pub fn pyany_to_json_value(any: &Bound<'_, PyAny>) -> Result<serde_json::Value, BridgeError> {
        // 1) None → JSON null (借 pyo3 `is_none()`)
        if any.is_none() {
            return Ok(serde_json::Value::Null);
        }
        // 2) bool → JSON bool (注意: 必须先 bool 后 int, Python bool 是 int 子类)
        if let Ok(b) = any.extract::<bool>() {
            return Ok(serde_json::Value::Bool(b));
        }
        // 3) i64 → JSON number
        if let Ok(i) = any.extract::<i64>() {
            return Ok(serde_json::json!(i));
        }
        // 4) f64 → JSON number
        if let Ok(f) = any.extract::<f64>() {
            return Ok(serde_json::json!(f));
        }
        // 5) String → JSON string
        if let Ok(s) = any.extract::<String>() {
            return Ok(serde_json::Value::String(s));
        }
        // 6) list → JSON array (借 pyo3 PyList + PyListMethods::is_instance)
        if any.is_instance_of::<pyo3::types::PyList>() {
            let list = any
                .cast::<pyo3::types::PyList>()
                .map_err(|e| BridgeError::CallFailed(format!("cast PyList: {e}")))?;
            let mut arr = Vec::with_capacity(list.len());
            for item in list.iter() {
                arr.push(pyany_to_json_value(&item)?);
            }
            return Ok(serde_json::Value::Array(arr));
        }
        // 7) dict → JSON object (借 pyo3 PyDict + PyDictMethods::is_instance)
        if any.is_instance_of::<pyo3::types::PyDict>() {
            let dict = any
                .cast::<pyo3::types::PyDict>()
                .map_err(|e| BridgeError::CallFailed(format!("cast PyDict: {e}")))?;
            let mut obj = serde_json::Map::new();
            for (key, value) in dict.iter() {
                let k = key
                    .extract::<String>()
                    .map_err(|e| BridgeError::CallFailed(format!("dict key extract: {e}")))?;
                let v = pyany_to_json_value(&value)?;
                obj.insert(k, v);
            }
            return Ok(serde_json::Value::Object(obj));
        }
        // 8) fallback: str() → JSON string
        let repr = any
            .str()
            .map(|s| s.to_string())
            .or_else(|_| any.repr().map(|s| s.to_string()))
            .map_err(|e| BridgeError::CallFailed(format!("fallback repr: {e}")))?;
        Ok(serde_json::Value::String(repr))
    }

    /// JSON 值 → 任意 Python 对象 (反方向)
    /// 借 pyo3 ToPyObject trait + json.loads 路径 (避免自实现每类型)
    pub fn json_value_to_pyany<'py>(
        py: Python<'py>,
        v: &serde_json::Value,
    ) -> Result<Bound<'py, PyAny>, BridgeError> {
        use pyo3::types::PyAnyMethods;
        // 借 json.dumps + json.loads 走 Python 路径 (借用 stdlib, 避免自实现每类型)
        let json_str = serde_json::to_string(v)
            .map_err(|e| BridgeError::InvalidArg(format!("json_value serialize: {e}")))?;
        let json_mod = py
            .import("json")
            .map_err(|e| BridgeError::CallFailed(format!("import json: {e}")))?;
        // 用 getattr + call1 模式 (Bound<PyModule> 走 PyAnyMethods 拿 loads)
        let loads_fn = json_mod
            .getattr("loads")
            .map_err(|e| BridgeError::CallFailed(format!("getattr json.loads: {e}")))?;
        loads_fn
            .call1((json_str,))
            .map_err(|e| BridgeError::CallFailed(format!("json.loads: {e}")))
    }
}

/// 类型转换守卫 trait (R127-2 借鉴 PyO3 conversions/traits.md)
///
/// Rust 函数可实现 `BridgeConvert` 来声明"我接受任意 Python 类型 + 返回 JSON 字符串"
/// 避免每函数重复写 `is_none + extract<bool> + extract<i64> + ...` 类型链.
///
/// cfg-无关 trait, 默认 build 也能 derive (借用 serde 自动).
pub trait BridgeConvert: Sized + Serialize + DeserializeOwned {
    /// Rust → Python (借 rust_to_json + json.loads)
    #[cfg(feature = "python-ext")]
    fn to_python<'py>(
        &self,
        py: pyo3::prelude::Python<'py>,
    ) -> Result<pyo3::prelude::Bound<'py, pyo3::prelude::PyAny>, BridgeError> {
        use pyo3::types::PyAnyMethods;
        let json = rust_to_json(self)?;
        // 用 getattr + call1 模式 (借 pyo3 0.22+ Bound API)
        let json_mod = py
            .import("json")
            .map_err(|e| BridgeError::CallFailed(format!("to_python import json: {e}")))?;
        let loads_fn = json_mod
            .getattr("loads")
            .map_err(|e| BridgeError::CallFailed(format!("to_python getattr loads: {e}")))?;
        loads_fn
            .call1((json,))
            .map_err(|e| BridgeError::CallFailed(format!("to_python json.loads: {e}")))
    }

    /// Python → Rust (借 pyany_to_json_value + serde_json)
    #[cfg(feature = "python-ext")]
    fn from_python(
        any: &pyo3::prelude::Bound<'_, pyo3::prelude::PyAny>,
    ) -> Result<Self, BridgeError> {
        let v = py::pyany_to_json_value(any)?;
        serde_json::from_value(v).map_err(|e| BridgeError::InvalidArg(format!("from_python: {e}")))
    }
}

// =============================================================================
// R128 阶段 A Stage 2 集成测试辅助 (per decision-57 §2.1 P10-2 跨语言调用验证)
// 借 PyO3 0.22+ conversions/traits.md (extract 模式) + 借用 Stage 1 pyany_to_json_value
// cfg-无关: 默认 build 提供 stub, python-ext build 提供真实现 (跨语言 round-trip 集成测试用)
// =============================================================================

/// Stage 2 集成测试 stub (默认 build, 0 体积)
/// 跑集成测试时给个静态字符串 "stage2-type-convert-default-stub" 守门
#[cfg(not(feature = "python-ext"))]
pub fn pyany_to_json_string_stub() -> &'static str {
    "stage2-type-convert-default-stub"
}

/// Stage 2 集成测试入口 (默认 build 走 stub)
/// python-ext build 走真 pyany_to_json_value → JSON 字符串 (用于双向 cross-language 测试)
#[cfg(not(feature = "python-ext"))]
pub fn end_to_end_type_convert_stub() -> Result<String, BridgeError> {
    Ok(rust_to_json(&"stage2-default-build")?)
}

/// 任意 Python 对象 → JSON 字符串 (cfg-gated 借 Stage 1 pyany_to_json_value)
/// 借 serde_json::to_string(&Value) 把 Value → String (跨语言 bridge 集成测试)
#[cfg(feature = "python-ext")]
pub fn pyany_to_json_string(
    any: &pyo3::prelude::Bound<'_, pyo3::prelude::PyAny>,
) -> Result<String, BridgeError> {
    let v = py::pyany_to_json_value(any)?;
    serde_json::to_string(&v)
        .map_err(|e| BridgeError::CallFailed(format!("pyany_to_json_string: {e}")))
}

/// 双向 Roundtrip 公共 API (cfg-无关, 默认 build 0 体积)
/// Rust struct → JSON String → (Python 端) JSON String → Rust struct
/// 用于 Stage 2 cross_language_bidirectional 集成测试
pub fn type_convert_roundtrip_json<T>(v: &T) -> Result<(String, T), BridgeError>
where
    T: Serialize + DeserializeOwned,
{
    let j1 = rust_to_json(v)?;
    let back: T = json_to_rust(&j1)?;
    Ok((j1, back))
}

// 自动为所有 (Serialize + DeserializeOwned) 实现 BridgeConvert
impl<T> BridgeConvert for T where T: Serialize + DeserializeOwned {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Sample {
        a: i64,
        b: String,
        c: Vec<f64>,
    }

    #[test]
    fn rust_to_json_basic() {
        let s = Sample {
            a: 42,
            b: "hello".into(),
            c: vec![1.0, 2.5, 3.14],
        };
        let j = rust_to_json(&s).expect("serialize");
        assert!(j.contains("\"a\":42"));
        assert!(j.contains("\"b\":\"hello\""));
        assert!(j.contains("\"c\":[1.0,2.5,3.14]"));
    }

    #[test]
    fn json_to_rust_basic() {
        let j = r#"{"a":7,"b":"world","c":[1.0,2.0]}"#;
        let s: Sample = json_to_rust(j).expect("deserialize");
        assert_eq!(s.a, 7);
        assert_eq!(s.b, "world");
        assert_eq!(s.c, vec![1.0, 2.0]);
    }

    #[test]
    fn json_roundtrip() {
        let s = Sample {
            a: 99,
            b: "roundtrip".into(),
            c: vec![42.0],
        };
        let j = rust_to_json(&s).unwrap();
        let back: Sample = json_to_rust(&j).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn rust_to_json_invalid_arg_on_bad_serde() {
        // serde_json::to_string on any Serialize should not fail
        // but json_to_rust on bad JSON should error
        let r: Result<Sample, _> = json_to_rust("not json");
        assert!(r.is_err());
        let e = r.unwrap_err();
        assert_eq!(e.suggested_action(), SuggestedAction_from_br());
    }

    fn SuggestedAction_from_br() -> crate::error::SuggestedAction {
        crate::error::SuggestedAction::Fail
    }

    #[test]
    fn json_to_rust_type_mismatch() {
        let r: Result<Sample, _> = json_to_rust(r#"{"a":"not a number","b":"x","c":[]}"#);
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("json_to_rust"));
    }

    // ============================================================
    // R127-2 Stage 6.1 新增 cfg-gated tests
    // ============================================================

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pyyany_to_json_value_none() {
        pyo3::Python::attach(|py| {
            // py.None() 返 Py<PyAny>, 用 bind 拿 Bound<PyAny>
            let none = py.None().into_bound(py);
            let v = py::pyany_to_json_value(&none).expect("None → null");
            assert_eq!(v, serde_json::Value::Null);
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pyyany_to_json_value_bool() {
        pyo3::Python::attach(|py| {
            use pyo3::IntoPyObject;
            let t = true.into_pyobject(py).unwrap();
            let v = py::pyany_to_json_value(&t).expect("True → bool");
            assert_eq!(v, serde_json::Value::Bool(true));
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pyyany_to_json_value_int() {
        pyo3::Python::attach(|py| {
            use pyo3::IntoPyObject;
            let i = 42i64.into_pyobject(py).unwrap();
            let v = py::pyany_to_json_value(&i).expect("42 → number");
            assert_eq!(v, serde_json::json!(42));
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pyyany_to_json_value_string() {
        pyo3::Python::attach(|py| {
            use pyo3::types::PyStringMethods;
            let s = pyo3::types::PyString::new(py, "hello");
            let v = py::pyany_to_json_value(&s).expect("str → string");
            assert_eq!(v, serde_json::Value::String("hello".to_string()));
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pyyany_to_json_value_list() {
        pyo3::Python::attach(|py| {
            // py.eval 接受 &CStr, 用 c"..." 字面量
            let list = py.eval(c"[1, 2, 3]", None, None).unwrap();
            let v = py::pyany_to_json_value(&list).expect("list → array");
            assert_eq!(v, serde_json::json!([1, 2, 3]));
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pyyany_to_json_value_dict() {
        pyo3::Python::attach(|py| {
            let dict = py.eval(c"{'a': 1, 'b': 'two'}", None, None).unwrap();
            let v = py::pyany_to_json_value(&dict).expect("dict → object");
            assert_eq!(v, serde_json::json!({"a": 1, "b": "two"}));
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_bridge_convert_roundtrip() {
        pyo3::Python::attach(|py| {
            let s = Sample {
                a: 7,
                b: "rt".into(),
                c: vec![1.5],
            };
            let any = BridgeConvert::to_python(&s, py).expect("to_python");
            let back: Sample = BridgeConvert::from_python(&any).expect("from_python");
            assert_eq!(s, back);
        });
    }
}
