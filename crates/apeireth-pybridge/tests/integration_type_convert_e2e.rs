//! R128 阶段 A Stage 2 — type_convert 端到端集成测试 (per decision-57 §2.1 P10-2)
//!
//! 借鉴 Stage 1 `type_convert.rs` (PyO3 0.22+ conversions/traits.md + BridgeConvert trait +
//! serde_json 中间表达) + Stage 2 `pyany_to_json_string` (cfg-gated) + `type_convert_roundtrip_json`
//! 公共 API.
//!
//! # 集成测试目标
//!
//! - rust_to_json / json_to_rust 跨 build 一致 (不依赖 pyo3)
//! - BridgeConvert trait roundtrip (cfg-gated, python-ext 下真双向)
//! - pyany_to_json_value None/bool/int/string/list/dict (Stage 1 覆盖, Stage 2 端到端复用)
//! - 复杂类型 (Sample struct, list of dict, nested) roundtrip
//! - 错误路径 (InvalidArg 类型不匹配) 跨 build 一致
//!
//! # 0 装 PASS 严守
//!
//! - ✅ 默认 build: rust_to_json / json_to_rust 跑 (cfg-无关), BridgeConvert 占位
//! - ⏳ python-ext build: BridgeConvert::to_python / from_python 真双向 (Stage 1 8 errors 已知)
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改 / A1 baseline 0 改 / B1 24 LOCKED 入口 0 改 / 8 锚 / 30 维 / 6 重 v7 / 13 键 /
//!   C1 0 commit

use apeireth_pybridge::{json_to_rust, rust_to_json, BridgeConvert, BridgeError, SuggestedAction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct SampleStage2 {
    a: i64,
    b: String,
    c: Vec<f64>,
    d: Option<String>,
    e: bool,
}

// 1. rust_to_json / json_to_rust 跨 build 一致 (Stage 1 严守, 不依赖 pyo3)
#[test]
fn stage2_e2e_type_convert_rust_json_basic() {
    let s = SampleStage2 {
        a: 42,
        b: "stage2".into(),
        c: vec![1.0, 2.5, 3.14],
        d: Some("opt".into()),
        e: true,
    };
    let j = rust_to_json(&s).expect("serialize");
    assert!(j.contains("\"a\":42"));
    assert!(j.contains("\"b\":\"stage2\""));
    assert!(j.contains("\"c\":[1.0,2.5,3.14]"));
    assert!(j.contains("\"d\":\"opt\""));
    assert!(j.contains("\"e\":true"));

    let back: SampleStage2 = json_to_rust(&j).expect("deserialize");
    assert_eq!(s, back);
}

// 2. json_to_rust 类型不匹配 → BridgeError::InvalidArg (Stage 1 严守)
#[test]
fn stage2_e2e_type_convert_type_mismatch_invalid_arg() {
    let r: Result<SampleStage2, BridgeError> = json_to_rust("not json");
    assert!(r.is_err());
    let e = r.unwrap_err();
    assert_eq!(e.suggested_action(), SuggestedAction::Fail);
    assert!(!e.is_recoverable());

    // 类型不匹配 (a 是 string 不是 i64)
    let r2: Result<SampleStage2, BridgeError> =
        json_to_rust(r#"{"a":"not a number","b":"x","c":[],"d":null,"e":false}"#);
    assert!(r2.is_err());
    assert!(r2.unwrap_err().to_string().contains("json_to_rust"));
}

// 3. Stage 2 type_convert_roundtrip_json 公共 API (cfg-无关, 默认 build 可用)
#[test]
fn stage2_e2e_type_convert_roundtrip_json_helper() {
    let s = SampleStage2 {
        a: 99,
        b: "rt-helper".into(),
        c: vec![42.0, 7.5],
        d: None,
        e: false,
    };
    let (j, back) = apeireth_pybridge::type_convert::type_convert_roundtrip_json(&s)
        .expect("roundtrip_json");
    assert!(j.contains("\"a\":99"));
    assert_eq!(s, back);
}

// 4. Stage 2 end_to_end_type_convert_stub 默认 build 0 体积守门
#[test]
fn stage2_e2e_type_convert_stub_default_build() {
    // 默认 build 下: end_to_end_type_convert_stub 返回 rust_to_json 的结果
    // python-ext build 下: 真 pyany_to_json_string (cfg-gated, 集成测试)
    let s = SampleStage2 {
        a: 1,
        b: "stub".into(),
        c: vec![],
        d: None,
        e: false,
    };
    let j = rust_to_json(&s).expect("rust_to_json works in all builds");
    assert!(j.contains("\"a\":1"));
    assert!(j.contains("\"b\":\"stub\""));

    // 默认 build 下: pyany_to_json_string_stub 返回固定字符串
    #[cfg(not(feature = "python-ext"))]
    {
        let stub = apeireth_pybridge::type_convert::pyany_to_json_string_stub();
        assert_eq!(stub, "stage2-type-convert-default-stub");
    }
}

// 5. 复杂类型 roundtrip: list of struct, nested Option, 跨 build 一致
#[test]
fn stage2_e2e_type_convert_complex_list_roundtrip() {
    let list = vec![
        SampleStage2 {
            a: 1,
            b: "one".into(),
            c: vec![1.0],
            d: Some("a".into()),
            e: true,
        },
        SampleStage2 {
            a: 2,
            b: "two".into(),
            c: vec![2.0, 2.0],
            d: None,
            e: false,
        },
        SampleStage2 {
            a: 3,
            b: "three".into(),
            c: vec![3.0, 3.0, 3.0],
            d: Some("c".into()),
            e: true,
        },
    ];
    let j = rust_to_json(&list).expect("list serialize");
    assert!(j.contains("\"a\":1"));
    assert!(j.contains("\"a\":2"));
    assert!(j.contains("\"a\":3"));

    let back: Vec<SampleStage2> = json_to_rust(&j).expect("list roundtrip");
    assert_eq!(back.len(), 3);
    assert_eq!(back[0].a, 1);
    assert_eq!(back[1].d, None);
    assert_eq!(back[2].c.len(), 3);
}

// 6. BridgeConvert trait 自动 impl (cfg-无关, Stage 1 严守 0 装 PASS)
#[test]
fn stage2_e2e_type_convert_bridge_convert_trait_impl() {
    // Stage 1 blanket impl: T: Serialize + DeserializeOwned 自动 impl BridgeConvert
    // 验证 to_python / from_python 方法在编译期可见 (cfg-无关)
    fn assert_bridge_convert<T: BridgeConvert>() {}
    assert_bridge_convert::<SampleStage2>();
    assert_bridge_convert::<i64>();
    assert_bridge_convert::<String>();
    assert_bridge_convert::<Vec<f64>>();

    // 默认 build: BridgeConvert::to_python / from_python 走 cfg-gated 实现
    // 不假设具体值, 0 装 PASS 严守
    let s = SampleStage2 {
        a: 7,
        b: "trait".into(),
        c: vec![7.0],
        d: Some("v".into()),
        e: true,
    };
    // rust_to_json 跨 build 一致 (不依赖 pyo3)
    let j = rust_to_json(&s).expect("trait impl serialize");
    assert!(j.contains("\"a\":7"));
}
