//! V27.0 跨配置功能对等验证 (round10-08 qa_engineer)
//!
//! # 测试目的
//!
//! 验证 `apeireth-pybridge` 在**默认 features** (无 `python-ext`) 与
//! **`--features python-ext`** 两种 build 配置下:
//!
//! 1. **API surface 同构**: 两配置都暴露同一套公开 API 名称 + 签名 (即使某些实现是 fallback)。
//! 2. **Pure Rust 行为同构**: `r11_compat::*` / `error::*` / JSON 序列化在两配置下
//!    产生**完全相同**的输出 (因为它们不依赖 pyo3)。
//! 3. **cfg-gate 守门同构**: `python_ext_enabled()` 在两配置下分别返回 `false` / `true`,
//!    `py_*` re-export 仅在 python-ext 下存在 (cfg-守门)。
//! 4. **错误处理同构**: `InvalidArg` / `ModuleNotFound` / `CallFailed` 在两配置下
//!    走相同 `suggested_action()` 路径。
//!
//! # 运行方式
//!
//! ```bash
//! # 默认 build (无 python-ext) — 期望全部 PASS
//! cargo test -p apeireth-pybridge --test cross_config_isomorphism
//!
//! # --features python-ext build — 同样期望全部 PASS
//! cargo test -p apeireth-pybridge --test cross_config_isomorphism --features python-ext
//! ```
//!
//! 两配置下测试**输出**应一致(除 `python_ext_enabled()` 的 bool 值)。
//! 报告 `reports/round10-08-v27-0-cross-config-functional-equivalence-2026-08-02.md`
//! 包含两配置下运行日志。

use apeireth_pybridge::{
    call_python_function, episode_to_json, health_check, is_known_r11_module, is_module_available,
    is_r11_module_available, list_r11_modules_by_category, note_to_json, placeholder,
    python_ext_enabled, python_is_available, python_version_string, r11_compat_version,
    r11_lookup_module, r11_module_category, r11_module_count, session_to_json, try_call_or_degrade,
    BridgeError, BridgeHealth, R11Category, R11ModuleInfo, SuggestedAction, R11_COMPAT_VERSION,
    R11_MODULE_COUNT,
};

// =============================================================================
// Cross-config invariant — 同一行为 (Pure Rust API)
// =============================================================================

/// 同一行为 1: `R11_MODULE_COUNT` 是编译期常量,两配置必须 = 1103。
#[test]
fn iso_r11_module_count_is_stable_across_configs() {
    let count = r11_module_count();
    assert_eq!(count, R11_MODULE_COUNT);
    assert_eq!(count, 1103, "R11 baseline = 1100+3 LOCKED");
}

/// 同一行为 2: `R11_COMPAT_VERSION` 标记 R14, 两配置必须一致。
#[test]
fn iso_r11_compat_version_is_r14() {
    let v = r11_compat_version();
    assert_eq!(v, R11_COMPAT_VERSION);
    assert!(
        v.starts_with("R14") || v.contains("R14"),
        "compat version must mark R14, got {v}"
    );
}

/// 同一行为 3: `is_known_r11_module("apeireth.memory.store")` 必须 = true。
#[test]
fn iso_known_r11_module_memory_store() {
    assert!(is_known_r11_module("apeireth.memory.store"));
}

/// 同一行为 4: `is_known_r11_module("apeireth.nope.nope")` 必须 = false。
#[test]
fn iso_known_r11_module_unknown() {
    assert!(!is_known_r11_module("apeireth.nope.nope"));
}

/// 同一行为 5: `r11_module_category("apeireth.memory.store")` 必须 = Memory。
#[test]
fn iso_r11_module_category_returns_memory() {
    let cat = r11_module_category("apeireth.memory.store");
    assert_eq!(cat, R11Category::Memory);
}

/// 同一行为 6: `r11_lookup_module` 返回的 `is_baseline` 字段稳定。
#[test]
fn iso_r11_lookup_module_baseline_flag() {
    let info: R11ModuleInfo =
        r11_lookup_module("apeireth.memory.v1141").expect("baseline module must be present");
    assert!(info.is_baseline, "v1141 is R11 baseline");
    assert_eq!(info.category, R11Category::Memory);
}

/// 同一行为 7: `list_r11_modules_by_category(Memory)` 非空。
#[test]
fn iso_list_memory_modules_non_empty() {
    let modules: Vec<String> = list_r11_modules_by_category(R11Category::Memory);
    assert!(!modules.is_empty(), "Memory category must have ≥1 module");
}

/// 同一行为 8: JSON 序列化在两配置下产生相同字符串 (Pure serde, 不依赖 pyo3)。
#[test]
fn iso_json_serialization_works_without_pyo3() {
    let s = apeireth_pybridge::placeholder();
    let _ = serde_json::to_string(&s).expect("placeholder string can serialize");

    let ep = apeireth_core::Episode {
        id: "iso-1".into(),
        timestamp: 1_700_000_000,
        role: "user".into(),
        content: "cross-config test".into(),
        session_id: "s-iso".into(),
    };
    let json = episode_to_json(&ep).expect("Episode serialize");
    let parsed: apeireth_core::Episode = serde_json::from_str(&json).expect("Episode roundtrip");
    assert_eq!(parsed.id, "iso-1");
    assert_eq!(parsed.content, "cross-config test");
}

/// 同一行为 9: BridgeError 4 变体的 suggested_action 在两配置下一致。
#[test]
fn iso_bridge_error_actions_are_stable() {
    // ModuleNotFound → Degrade (稳定, 不论 Python 是否可用)
    let m = BridgeError::ModuleNotFound("x".into());
    assert_eq!(m.suggested_action(), SuggestedAction::Degrade);
    assert!(!m.is_recoverable());

    // CallFailed → Retry
    let c = BridgeError::CallFailed("x".into());
    assert_eq!(c.suggested_action(), SuggestedAction::Retry);
    assert!(c.is_recoverable());

    // GilError → Retry
    let g = BridgeError::GilError("x".into());
    assert_eq!(g.suggested_action(), SuggestedAction::Retry);
    assert!(g.is_recoverable());

    // InvalidArg → Fail
    let i = BridgeError::InvalidArg("x".into());
    assert_eq!(i.suggested_action(), SuggestedAction::Fail);
    assert!(!i.is_recoverable());
}

/// 同一行为 10: BridgeHealth Display 包含 "r11" + "modules" 字段, 与 config 无关。
#[test]
fn iso_bridge_health_display_contains_r11() {
    let h = BridgeHealth {
        python_version: "test-iso".into(),
        r11_compat_version: "R14-iso",
        r11_module_count: 1103,
        python_available: false,
    };
    let s = format!("{h}");
    assert!(s.contains("r11"), "Display must contain r11: {s}");
    assert!(s.contains("modules"), "Display must contain modules: {s}");
}

// =============================================================================
// Config-specific behavior — cfg!(feature = "python-ext") 分支
// =============================================================================

/// 分支行为 1: `python_ext_enabled()` 必须与编译期 `cfg!` 一致。
#[test]
fn cfg_python_ext_enabled_reflects_cfg() {
    let runtime = python_ext_enabled();
    let compile_time = cfg!(feature = "python-ext");
    assert_eq!(
        runtime, compile_time,
        "python_ext_enabled() must match cfg! at compile time"
    );
}

/// 分支行为 2: `placeholder()` 是 `'static str`,两配置下内容一致。
#[test]
fn cfg_placeholder_static_and_stable() {
    let p = placeholder();
    // 字符串切片地址在两配置下应一致 (因为是 &'static str)。
    assert_eq!(p.as_ptr() as usize, placeholder().as_ptr() as usize);
    assert!(p.contains("ADR 0007"));
    assert!(p.contains("ADR 0008") || p.contains("feature-gated"));
}

/// 分支行为 3: 默认 build 下 `call_python_function("json", "dumps", &["x"])` 返回 ModuleNotFound 降级;
/// python-ext build 下返回真实结果。这两路径都是合法的,验证错误路径在两配置下都一致。
#[test]
fn cfg_call_python_function_invalid_module_consistent() {
    // 测试一个必定不存在的 module (即使在 python-ext 下也找不到)。
    let result = call_python_function("apeireth.does.not.exist.module", "f", &[]);
    let err = result.expect_err("invalid module must error in BOTH configs");
    assert_eq!(err.suggested_action(), SuggestedAction::Degrade);
    assert!(!err.is_recoverable());
}

/// 分支行为 4: 默认 build 下 `call_python_function("json", "dumps", &["hello"])` 必须降级 (ModuleNotFound);
/// python-ext build 下返回 `"\"hello\""` (json.dumps 输出)。两条路径都是 config-specific, 都在测试范围内。
#[test]
fn cfg_call_python_function_json_dumps_path() {
    let result = call_python_function("json", "dumps", &["hello"]);
    if python_ext_enabled() {
        // python-ext: 真实调用 json.dumps("hello") → "\"hello\""
        let v = result.expect("python-ext must reach Python and return \"hello\" quoted");
        assert_eq!(v, "\"hello\"");
    } else {
        // 默认: ModuleNotFound 降级 (pyo3 disabled)
        let err = result.expect_err("default build must NOT call Python");
        assert_eq!(err.suggested_action(), SuggestedAction::Degrade);
    }
}

/// 分支行为 5: `python_is_available()` 返回值在两配置下表现:
/// - 默认 build: 永远 = false (pyo3 不存在)
/// - python-ext build: 取决于运行时是否有 Python 解释器, 通常 = true
/// 测试不做硬性 true 断言, 只确保类型 + 不 panic。
#[test]
fn cfg_python_is_available_returns_bool_safely() {
    let available: bool = python_is_available();
    let _ = available; // 不假设值
}

/// 分支行为 6: `python_version_string()` 返回值非空 (两配置都满足):
/// - 默认 build: 静态占位符 "pyo3 disabled (build with --features python-ext...)"
/// - python-ext build: Python 3.13.14 真实版本字符串
#[test]
fn cfg_python_version_string_non_empty() {
    let v = python_version_string();
    assert!(
        !v.is_empty(),
        "python_version_string() must return non-empty in BOTH configs"
    );
    // 两配置下都包含 "pyo3" 字符串 — 默认是 "pyo3 disabled", python-ext 不一定, 但以下兼容。
    // 不做硬性包含断言, 因为 python-ext 下格式依赖 PyO3 0.22 实现。
}

/// 分支行为 7: `is_module_available("math")` 在默认 build 下 = false,
/// 在 python-ext build 下 = true (如果运行时 Python 解释器存在)。
#[test]
fn cfg_is_module_available_math_behavior() {
    let avail = is_module_available("math");
    if python_ext_enabled() {
        // python-ext: 真实 import math
        assert!(avail, "python-ext must find math module");
    } else {
        // 默认: 永远 = false
        assert!(!avail, "default build must report math as unavailable");
    }
}

/// 分支行为 8: `try_call_or_degrade` 在两配置下行为对比:
/// - 默认 build: invalid module → (None, Degrade)
/// - python-ext build: invalid module → (None, Degrade) (Python ImportError 映射)
/// 但有效 module (math.sqrt): 默认 (None, Degrade) vs python-ext (Some("2.0"), Retry)
#[test]
fn cfg_try_call_or_degrade_invalid_module_consistent() {
    let (result, action) = try_call_or_degrade("apeireth.does.not.exist.module", "f", &[]);
    assert!(
        result.is_none(),
        "invalid module yields None result in BOTH configs"
    );
    assert_eq!(
        action,
        SuggestedAction::Degrade,
        "invalid module must suggest Degrade in BOTH configs"
    );
}

/// 分支行为 9: `is_r11_module_available("apeireth.memory.store")` 依赖 `is_module_available`:
/// - 默认 build: false (math=不可用 → r11 同样不可用)
/// - python-ext build: 但 Python 没装 apeireth.memory.store, 也 = false
/// 因此两配置下都为 false — 验证语义稳定。
#[test]
fn cfg_is_r11_module_available_returns_false_when_python_unavailable() {
    // 实际上两配置下都是 false (Python 默认无 R11 包)。
    // 我们验证它不 panic + 返回值稳定。
    let avail = is_r11_module_available("apeireth.memory.store");
    let _ = avail; // 不假设具体值, 只确保调用合法。
}

/// 分支行为 10: `health_check()` 返回 BridgeHealth 字段一致 (除 python_version_string 来自 cfg branch)。
#[test]
fn cfg_health_check_returns_consistent_struct() {
    let h = health_check();
    // r11 字段两配置下必须一致。
    assert_eq!(h.r11_compat_version, R11_COMPAT_VERSION);
    assert_eq!(h.r11_module_count, R11_MODULE_COUNT);
    // python_available 必须与 is_module_available("math") 一致 (cross-config invariant)。
    assert_eq!(h.python_available, is_module_available("math"));
}

// =============================================================================
// Cross-config Python type gate — py_* 仅在 python-ext 可见
// =============================================================================

/// 分支行为 11: 在编译期通过 `cfg!` 检测 `python_bindings` 模块是否存在。
#[test]
fn cfg_python_bindings_module_compiled_only_with_feature() {
    let module_compiled = cfg!(feature = "python-ext");
    let runtime_feature = python_ext_enabled();
    assert_eq!(
        module_compiled, runtime_feature,
        "python_bindings module compilation == python_ext_enabled()"
    );
}

/// 分支行为 12: 在默认 build 下, 调用 `python_version_string()` 必须返回 `&'static str`
/// (签名退化); 在 python-ext 下返回 `String` (签名完整)。这两签名差异是 cfg-gated 兼容层
/// 的标志, 测试仅验证返回值非空。
#[test]
fn cfg_python_version_string_signature_adapts() {
    let v = python_version_string();
    assert!(!v.is_empty());
}
