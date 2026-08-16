//! R128 阶段 A Stage 3 — 端到端集成测试 (per decision-58 §2.1 P10-3)
//!
//! 借鉴 hyper 80 (LIFO 池复用) + servers 175 (多 endpoint dispatch).
//! 实施 Stage 3 端到端测试, 跑在所有 build (默认 build + python-ext build).
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改
//! - A1 R11 baseline 0.8682/0.8532/0.9063 严守
//! - B1 24 LOCKED 入口签名 0 改
//! - C1 0 commit (写到 reports 不动 git)
//! - C2 0 装 PASS 严守
//! - 0 主动 push

use apeireth_pybridge::{
    cross_language_smoke_check, end_to_end_smoke_check, is_module_available, python_ext_enabled,
    python_is_available, r11_compat_version, r11_module_count, stage3_cross_module_count,
    stage3_e2e_smoke, stage3_e2e_summary, BridgeModulePool, PoolConfig, Stage3E2ESmoke,
    R11_COMPAT_VERSION, R11_MODULE_COUNT,
};

// 1. Stage 3 端到端 smoke 跨 build 可调用 + Display 含 6 子模块
#[test]
fn stage3_e2e_smoke_callable_and_display() {
    let smoke = stage3_e2e_smoke();
    let display = format!("{smoke}");
    assert!(display.contains("Stage 3"));
    assert!(display.contains("pool"));
    assert!(display.contains("e2e_ok"));
    // 6 modules_in_scope
    assert_eq!(smoke.modules_in_scope.len(), 6);
}

// 2. Stage 3 端到端 smoke: 7 ASI 关键模块严守
#[test]
fn stage3_e2e_seven_asi_modules_locked() {
    let smoke = stage3_e2e_smoke();
    assert_eq!(smoke.asi_module_count, 7);
    assert!(!smoke.ceiling_critical_modules.is_empty());
    assert!(smoke
        .ceiling_critical_modules
        .iter()
        .any(|n| n.contains("v1458")));
}

// 3. Stage 3 端到端 smoke: R11 1103 模块严守
#[test]
fn stage3_e2e_r11_locked() {
    let smoke = stage3_e2e_smoke();
    assert_eq!(smoke.r11_module_count, 1103);
    assert_eq!(smoke.r11_compat_version, R11_COMPAT_VERSION);
    assert_eq!(r11_module_count(), 1103);
    assert_eq!(r11_compat_version(), R11_COMPAT_VERSION);
}

// 4. Stage 3 端到端 smoke: 池 cfg 默认 (max_idle=32, idle_timeout=90s)
#[test]
fn stage3_e2e_pool_config_defaults() {
    let smoke = stage3_e2e_smoke();
    assert_eq!(smoke.pool_max_idle, 32);
    assert_eq!(smoke.pool_idle_timeout_secs, 90);
    let pool = BridgeModulePool::new();
    assert_eq!(pool.config().max_idle, 32);
    assert_eq!(pool.config().idle_timeout_secs, 90);
}

// 5. Stage 3 端到端 smoke: 7 ASI 类别全部 1+ 模块
#[test]
fn stage3_e2e_asi_categories_in_use() {
    let smoke = stage3_e2e_smoke();
    assert_eq!(smoke.categories_in_use, 7);
}

// 6. Stage 3 端到端 smoke: e2e_ok 跟 python_ext_active 关联
#[test]
fn stage3_e2e_ok_python_ext_consistent() {
    let smoke = stage3_e2e_smoke();
    if !smoke.python_ext_active {
        assert!(!smoke.e2e_ok, "默认 build 0 装: e2e_ok 必 = false");
    } else {
        // python-ext: e2e_ok 取决于 Python 运行时 + math 模块可用
        let expected = python_is_available() && is_module_available("math");
        assert_eq!(smoke.e2e_ok, expected);
    }
}

// 7. Stage 3 跨模块协同计数 5/5
#[test]
fn stage3_e2e_cross_module_count_5_of_5() {
    let (ok, total) = stage3_cross_module_count();
    assert_eq!(total, 5);
    assert_eq!(ok, total, "Stage 3 5 子模块全可调用 (cfg-无关)");
}

// 8. Stage 3 跟 Stage 2 API 兼容 (不打破旧 API)
#[test]
fn stage3_e2e_backward_compat_with_stage2() {
    let s2_e2e = end_to_end_smoke_check();
    let s2_xlang = cross_language_smoke_check();
    let s3_e2e = stage3_e2e_smoke();

    // r11 跨 3 API 一致
    assert_eq!(s2_e2e.r11_module_count, s3_e2e.r11_module_count);
    assert_eq!(s2_xlang.r11_module_count, s3_e2e.r11_module_count);
    assert_eq!(s2_e2e.r11_compat_version, s3_e2e.r11_compat_version);
    assert_eq!(s2_xlang.r11_compat_version, s3_e2e.r11_compat_version);

    // pool cfg 跨 2 API 一致
    assert_eq!(s2_e2e.pool_max_idle, s3_e2e.pool_max_idle);
    assert_eq!(s2_e2e.pool_idle_timeout_secs, s3_e2e.pool_idle_timeout_secs);
}

// 9. Stage 3 e2e_summary 引用 decision-58 + Stage 3
#[test]
fn stage3_e2e_summary_cites_decision_58() {
    let summary = stage3_e2e_summary();
    assert!(summary.contains("decision-58"));
    assert!(summary.contains("Stage 3"));
    assert!(summary.contains("P10-3"));
    assert!(summary.contains("hyper"));
    assert!(summary.contains("PyO3"));
}

// 10. Stage 3 端到端 smoke python_ext_active 跟 cfg! 一致
#[test]
fn stage3_e2e_python_ext_active_matches_cfg() {
    let smoke = stage3_e2e_smoke();
    assert_eq!(smoke.python_ext_active, cfg!(feature = "python-ext"));
    assert_eq!(smoke.python_ext_active, python_ext_enabled());
}

// 11. Stage 3 端到端 smoke 跨 N 次调用稳定
#[test]
fn stage3_e2e_idempotent() {
    let s1 = stage3_e2e_smoke();
    let s2 = stage3_e2e_smoke();
    assert_eq!(s1.asi_module_count, s2.asi_module_count);
    assert_eq!(s1.r11_module_count, s2.r11_module_count);
    assert_eq!(s1.pool_max_idle, s2.pool_max_idle);
    assert_eq!(s1.modules_in_scope, s2.modules_in_scope);
}

// 12. Stage 3 端到端 smoke 池 stats 初始 0
#[test]
fn stage3_e2e_pool_stats_initial_zero() {
    let smoke = stage3_e2e_smoke();
    assert_eq!(smoke.pool_stats.cached_modules, 0);
    assert_eq!(smoke.pool_stats.hits, 0);
    assert_eq!(smoke.pool_stats.misses, 0);
    assert_eq!(smoke.pool_stats.evictions, 0);
    assert_eq!(smoke.pool_stats.hit_rate(), 0.0);
}

// 13. Stage 3 端到端 smoke + PoolConfig 调参 (max_idle=2)
#[test]
fn stage3_e2e_pool_custom_config() {
    let pool = BridgeModulePool::with_config(PoolConfig {
        max_idle: 2,
        idle_timeout_secs: 30,
    });
    let cfg = pool.config();
    assert_eq!(cfg.max_idle, 2);
    assert_eq!(cfg.idle_timeout_secs, 30);
    // 仍能构造 (cfg-无关)
    let stats = pool.stats();
    assert_eq!(stats.cached_modules, 0);
}

// 14. Stage 3 跨模块 6 子模块名 严守
#[test]
fn stage3_e2e_six_modules_in_scope() {
    let smoke: Stage3E2ESmoke = stage3_e2e_smoke();
    let expected = vec![
        "bridge",
        "bridge_pool",
        "asi_modules",
        "r11_compat",
        "type_convert",
        "python_bindings",
    ];
    for exp in expected {
        assert!(
            smoke.modules_in_scope.iter().any(|m| m == exp),
            "missing: {exp}"
        );
    }
}

// 15. Stage 3 端到端 smoke 完整 Display 字段 (含 r11 + pool + e2e)
#[test]
fn stage3_e2e_full_display_dump() {
    let smoke = stage3_e2e_smoke();
    let display = format!("{smoke}");
    // 必含字段
    assert!(display.contains("stage1_version"));
    assert!(display.contains("r11"));
    assert!(display.contains("pool"));
    assert!(display.contains("e2e_ok"));
    assert!(display.contains("ceiling_critical"));
    assert!(display.contains("python_ext"));
    assert!(display.contains("python_available"));
}
