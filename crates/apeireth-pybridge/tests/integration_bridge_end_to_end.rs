//! R128 阶段 A Stage 2 — bridge.rs 端到端集成测试 (per decision-57 §2.1 P10-2)
//!
//! 借鉴 Stage 1 `bridge.rs` (PyO3 928 + Python::attach + Bound API) + Stage 2
//! `end_to_end_smoke_check()` / `cross_language_smoke_check()` 公共 API.
//!
//! # 集成测试目标
//!
//! - 跨 build 一致 (默认 build + python-ext build)
//! - 跨模块协同 (bridge + bridge_pool + r11_compat + type_convert + error)
//! - 端到端路径 (health_check + try_call_or_degrade + call_python_builtin +
//!   episode_to_json + session_to_json + note_to_json + BridgeError 4 路径)
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改
//! - A1 R11 baseline 0.8682/0.8532/0.9063 严守 (apeireth-asi 17 文件 0 触碰)
//! - B1 24 LOCKED 入口签名 0 改 (24 LOCKED 中无 pybridge, 自由改 src)
//! - C1 0 commit (写到 reports 不动 git)

use apeireth_pybridge::{
    call_python_builtin, call_python_function, cross_language_smoke_check, end_to_end_smoke_check,
    episode_to_json, health_check, is_known_r11_module, is_module_available, note_to_json,
    placeholder, python_ext_enabled, python_is_available, python_version_string,
    r11_compat_version, r11_lookup_module, r11_module_category, r11_module_count, session_to_json,
    try_call_or_degrade, BridgeError, BridgeHealth, R11Category, R11ModuleInfo, R11_COMPAT_VERSION,
    R11_MODULE_COUNT, SuggestedAction,
};

// 1. placeholder 跨 build 一致 (Stage 1 严守)
#[test]
fn stage2_e2e_bridge_placeholder_idempotent() {
    assert!(placeholder().contains("apeireth-pybridge"));
    assert!(placeholder().contains("ADR 0007") || placeholder().contains("feature-gated"));
    // 同一地址 (Stage 1 严守)
    assert_eq!(placeholder().as_ptr(), placeholder().as_ptr());
}

// 2. python_version_string 跨 build 一致非空
#[test]
fn stage2_e2e_bridge_python_version_string_non_empty() {
    let v = python_version_string();
    assert!(!v.is_empty(), "python_version_string 跨 build 非空");
}

// 3. python_is_available 跨 build 返回 bool (不假设值, 0 装 PASS)
#[test]
fn stage2_e2e_bridge_python_is_available_returns_bool() {
    let _: bool = python_is_available();
    // 0 假设具体值 (Stage 1 严守, 0 装"已实施")
}

// 4. is_module_available 跨 build cfg 一致
#[test]
fn stage2_e2e_bridge_is_module_available_math_dual_build() {
    let avail = is_module_available("math");
    if python_ext_enabled() {
        // python-ext: 真实 import math (Stage 1 r125_9_known_module_stable)
        assert!(avail, "python-ext 必能找到 math");
    } else {
        // 默认 build: 永远 = false (Stage 1 cfg-守门)
        assert!(!avail, "默认 build math 不可用");
    }
}

// 5. health_check 返回 BridgeHealth 字段一致 (跨 build, R11 字段严守)
#[test]
fn stage2_e2e_bridge_health_check_r11_fields_stable() {
    let h = health_check();
    assert_eq!(h.r11_compat_version, R11_COMPAT_VERSION);
    assert_eq!(h.r11_module_count, R11_MODULE_COUNT);
    assert_eq!(h.r11_module_count, 1103);
}

// 6. BridgeError 4 变体 + suggested_action 跨 build 一致 (Stage 1 严守 4 路径)
#[test]
fn stage2_e2e_bridge_error_4_variants() {
    let m = BridgeError::ModuleNotFound("x".into());
    assert_eq!(m.suggested_action(), SuggestedAction::Degrade);
    assert!(!m.is_recoverable());

    let c = BridgeError::CallFailed("x".into());
    assert_eq!(c.suggested_action(), SuggestedAction::Retry);
    assert!(c.is_recoverable());

    let g = BridgeError::GilError("x".into());
    assert_eq!(g.suggested_action(), SuggestedAction::Retry);
    assert!(g.is_recoverable());

    let i = BridgeError::InvalidArg("x".into());
    assert_eq!(i.suggested_action(), SuggestedAction::Fail);
    assert!(!i.is_recoverable());
}

// 7. try_call_or_degrade 4 路径 (Stage 1 严守) — 端到端集成测试
#[test]
fn stage2_e2e_bridge_try_call_or_degrade_4_paths() {
    // 路径 1: invalid module → (None, Degrade)
    let (r, a) = try_call_or_degrade("apeireth.does.not.exist", "f", &[]);
    assert!(r.is_none());
    assert_eq!(a, SuggestedAction::Degrade);

    // 路径 2: invalid args (空 module) → (None, Fail)
    let (r, a) = try_call_or_degrade("", "f", &["x"]);
    assert!(r.is_none());
    assert_eq!(a, SuggestedAction::Fail);

    // 路径 3: 默认 build + json.dumps (cfg-gated 走 ModuleNotFound 降级)
    if !python_ext_enabled() {
        let (r, a) = try_call_or_degrade("json", "dumps", &["hello"]);
        assert!(r.is_none(), "默认 build 下 json.dumps 不可用");
        assert_eq!(a, SuggestedAction::Degrade);
    }
}

// 8. call_python_builtin 透传入参校验 (Stage 1 严守, 默认 build 降级)
#[test]
fn stage2_e2e_bridge_call_python_builtin_validation_propagates() {
    let r = call_python_builtin("", "x", "y");
    assert!(r.is_err());
    let e = r.unwrap_err();
    assert!(!e.is_recoverable());
    assert_eq!(e.suggested_action(), SuggestedAction::Fail);

    // 默认 build + 有效 module 也降级 (cfg-守门)
    if !python_ext_enabled() {
        let r2 = call_python_builtin("json", "dumps", "hello");
        assert!(r2.is_err());
        assert_eq!(r2.unwrap_err().suggested_action(), SuggestedAction::Degrade);
    }
}

// 9. call_python_function 跨 build cfg 守门 (Stage 1 r125_9_call_python_function_default_build_degrades)
#[test]
fn stage2_e2e_bridge_call_python_function_default_build_degrades() {
    if !python_ext_enabled() {
        let r = call_python_function("json", "dumps", &["x"]);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().suggested_action(), SuggestedAction::Degrade);
    }
    // python-ext 下: Stage 1 r127_2_py_call_python_with_kwargs_propagates 等真调用覆盖
}

// 10. episode_to_json / session_to_json / note_to_json 三件套 roundtrip (Stage 1 严守)
#[test]
fn stage2_e2e_bridge_apeireth_core_types_roundtrip() {
    let ep = apeireth_core::Episode {
        id: "ep-stage2".into(),
        timestamp: 1_700_000_000,
        role: "user".into(),
        content: "stage2 integration test".into(),
        session_id: "s-stage2".into(),
    };
    let ep_json = episode_to_json(&ep).expect("Episode serialize");
    let ep_parsed: apeireth_core::Episode =
        serde_json::from_str(&ep_json).expect("Episode roundtrip");
    assert_eq!(ep_parsed.id, "ep-stage2");
    assert_eq!(ep_parsed.content, "stage2 integration test");

    let s = apeireth_core::Session {
        id: "s-stage2".into(),
        started_at: 1,
        last_active_at: 2,
    };
    let s_json = session_to_json(&s).expect("Session serialize");
    let s_parsed: apeireth_core::Session = serde_json::from_str(&s_json).expect("Session roundtrip");
    assert_eq!(s_parsed.id, "s-stage2");

    let n = apeireth_core::Note {
        id: "n-stage2".into(),
        timestamp: 3,
        content: "stage2 note".into(),
        source_episode_ids: vec!["ep-stage2".into()],
        confidence: 0.9,
        tags: vec!["integration".into(), "stage2".into()],
    };
    let n_json = note_to_json(&n).expect("Note serialize");
    let n_parsed: apeireth_core::Note = serde_json::from_str(&n_json).expect("Note roundtrip");
    assert_eq!(n_parsed.id, "n-stage2");
    assert_eq!(n_parsed.tags.len(), 2);
}

// 11. r11_compat 跨 crate 集成 (bridge + r11_compat 协同)
#[test]
fn stage2_e2e_bridge_r11_compat_integration() {
    assert_eq!(r11_module_count(), 1103);
    assert!(is_known_r11_module("apeireth.memory.store"));
    assert!(!is_known_r11_module("apeireth.nope.nope"));
    assert_eq!(
        r11_module_category("apeireth.memory.store"),
        R11Category::Memory
    );
    let info: R11ModuleInfo =
        r11_lookup_module("apeireth.memory.v1141").expect("baseline module");
    assert!(info.is_baseline);
    assert_eq!(info.category, R11Category::Memory);
    assert_eq!(r11_compat_version(), R11_COMPAT_VERSION);
}

// 12. Stage 2 end_to_end_smoke_check 端到端 (新加 API 跨 build 验证)
#[test]
fn stage2_e2e_bridge_end_to_end_smoke_check_callable() {
    let smoke = end_to_end_smoke_check();
    assert_eq!(smoke.r11_module_count, 1103);
    assert_eq!(smoke.python_ext_active, python_ext_enabled());
    assert_eq!(smoke.pool_max_idle, 32);
    assert_eq!(smoke.pool_idle_timeout_secs, 90);
    let out = format!("{smoke}");
    assert!(out.contains("r11"));
    assert!(out.contains("pool"));
}

// 13. Stage 2 cross_language_smoke_check 端到端 (新加 API 跨 build 验证)
#[test]
fn stage2_e2e_bridge_cross_language_smoke_check_callable() {
    let smoke = cross_language_smoke_check();
    assert_eq!(smoke.r11_module_count, 1103);
    assert_eq!(smoke.python_ext_active, python_ext_enabled());
    // 默认 build 下 bidirectional_ok 必 = false (0 装 PASS 严守)
    if !smoke.python_ext_active {
        assert!(!smoke.bidirectional_ok);
    }
    let out = format!("{smoke}");
    assert!(out.contains("bidirectional_ok"));
}

// 14. BridgeHealth Display 包含 r11 + modules (Stage 1 cfg-无关严守)
#[test]
fn stage2_e2e_bridge_health_display_contains_r11() {
    let h = BridgeHealth {
        python_version: "stage2-test".into(),
        r11_compat_version: "R14-stage2",
        r11_module_count: 1103,
        python_available: false,
    };
    let s = format!("{h}");
    assert!(s.contains("r11"), "Display 含 r11: {s}");
    assert!(s.contains("modules"), "Display 含 modules: {s}");
    assert!(s.contains("stage2-test"), "Display 含 python_version: {s}");
}

// 15. 跨 Stage 1+2 API 协同 (bridge + bridge_pool + r11 + type_convert + error)
#[test]
fn stage2_e2e_bridge_cross_api_integration_full() {
    // 端到端: bridge + bridge_pool + r11 + type_convert 4 模块协同
    use apeireth_pybridge::{BridgeModulePool, PoolConfig};

    let pool = BridgeModulePool::with_config(PoolConfig {
        max_idle: 8,
        idle_timeout_secs: 60,
    });
    let stats = pool.stats();
    let smoke = end_to_end_smoke_check();
    let cls = cross_language_smoke_check();

    // pool stats + smoke.pool_stats 跨 API 一致
    assert_eq!(stats.cached_modules, 0);
    assert_eq!(smoke.pool_max_idle, 32, "smoke 默认 max_idle=32 (跟 BridgeModulePool::default 一致)");

    // r11 字段跨 3 API 一致
    assert_eq!(r11_module_count(), 1103);
    assert_eq!(smoke.r11_module_count, 1103);
    assert_eq!(cls.r11_module_count, 1103);

    // BridgeError 4 路径 + r11_compat_version 严守
    let _ = BridgeError::ModuleNotFound("stage2".into());
    assert_eq!(r11_compat_version(), R11_COMPAT_VERSION);
}
