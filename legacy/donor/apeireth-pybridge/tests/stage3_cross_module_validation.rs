//! R128 阶段 A Stage 3 — 跨模块集成验证测试 (per decision-58 §2.1 P10-3)
//!
//! 借鉴 PyO3 928 pybridge (Python ↔ Rust 全链路验证) + superpowers 234 skill cross-cutting.
//! 实施 Stage 3 跨模块集成验证, 跑在所有 build (默认 build + python-ext build).
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改
//! - A1 R11 baseline 0.8682/0.8532/0.9063 严守
//! - B1 24 LOCKED 入口签名 0 改
//! - C1 0 commit
//! - C2 0 装 PASS 严守
//! - 0 主动 push

use apeireth_pybridge::{
    asi_lookup_by_version, asi_lookup_module, asi_stage1_module_count, asi_stage1_version,
    list_asi_stage1_modules_by_category, list_ceiling_critical_modules, probe_asi_to_r11,
    probe_bridge_to_pool, probe_bridge_to_r11, probe_core_to_bridge, probe_pool_to_type_convert,
    r11_compat_version, r11_module_category, r11_module_count, stage3_cross_module_probes,
    AsiCategory, BridgeError, CrossModuleKind, CrossModuleReport, HardWallsVerify, R11Category,
    SuggestedAction, R11_COMPAT_VERSION, R11_MODULE_COUNT,
};

// 1. 5 探针全部返回 ok (cfg-无关, 默认 build 跑 0 体积)
#[test]
fn stage3_xmod_5_probes_all_ok() {
    let p1 = probe_bridge_to_pool();
    let p2 = probe_bridge_to_r11();
    let p3 = probe_pool_to_type_convert();
    let p4 = probe_asi_to_r11();
    let p5 = probe_core_to_bridge();
    assert!(p1.ok, "P1 bridge↔pool fail: {:?}", p1);
    assert!(p2.ok, "P2 bridge↔r11 fail: {:?}", p2);
    assert!(p3.ok, "P3 pool↔type_convert fail: {:?}", p3);
    assert!(p4.ok, "P4 asi↔r11 fail: {:?}", p4);
    assert!(p5.ok, "P5 core↔bridge fail: {:?}", p5);
}

// 2. stage3_cross_module_probes 整体报告 all_ok
#[test]
fn stage3_xmod_probes_all_ok_aggregate() {
    let r = stage3_cross_module_probes();
    assert_eq!(r.probe_results.len(), 5);
    assert!(r.all_ok);
}

// 3. 5 CrossModuleKind variant 唯一
#[test]
fn stage3_xmod_5_kinds_unique() {
    let kinds = [
        CrossModuleKind::BridgeToPool,
        CrossModuleKind::BridgeToR11,
        CrossModuleKind::PoolToTypeConvert,
        CrossModuleKind::AsiToR11,
        CrossModuleKind::CoreToBridge,
    ];
    let mut seen = std::collections::HashSet::new();
    for k in &kinds {
        assert!(seen.insert(k), "kind {k:?} 重复");
    }
    assert_eq!(seen.len(), 5);
}

// 4. CrossModuleReport Display 字段完整
#[test]
fn stage3_xmod_report_display() {
    let r = stage3_cross_module_probes();
    let s = format!("{r}");
    assert!(s.contains("Stage 3"));
    assert!(s.contains("decision-58"));
    assert!(s.contains("5/5 OK"));
    assert!(s.contains("BridgeToPool"));
    assert!(s.contains("BridgeToR11"));
    assert!(s.contains("PoolToTypeConvert"));
    assert!(s.contains("AsiToR11"));
    assert!(s.contains("CoreToBridge"));
    assert!(s.contains("modules in scope"));
}

// 5. HardWallsVerify 全 10 项 PASS
#[test]
fn stage3_xmod_hard_walls_all_pass() {
    let v = HardWallsVerify::auto_verify();
    assert!(v.all_pass(), "8 硬墙 (10 项) 全 PASS 必通过, got: {v}");
}

// 6. HardWallsVerify Display 字段完整
#[test]
fn stage3_xmod_hard_walls_display() {
    let v = HardWallsVerify::auto_verify();
    let s = format!("{v}");
    assert!(s.contains("B2"));
    assert!(s.contains("A1"));
    assert!(s.contains("B1"));
    assert!(s.contains("B5"));
    assert!(s.contains("B3"));
    assert!(s.contains("B4"));
    assert!(s.contains("A3"));
    assert!(s.contains("C1"));
    assert!(s.contains("C2"));
    assert!(s.contains("C3"));
    assert!(s.contains("decision-33"));
    assert!(s.contains("decision-58"));
}

// 7. P1 探针 (bridge↔pool) 详细 verify
#[test]
fn stage3_xmod_p1_bridge_to_pool_detail() {
    let p = probe_bridge_to_pool();
    assert_eq!(p.kind, CrossModuleKind::BridgeToPool);
    assert!(p.ok);
    assert!(p.detail.contains("pool_max_idle=32"));
    assert!(p.detail.contains("pool_idle_timeout=90"));
}

// 8. P2 探针 (bridge↔r11) 详细 verify
#[test]
fn stage3_xmod_p2_bridge_to_r11_detail() {
    let p = probe_bridge_to_r11();
    assert_eq!(p.kind, CrossModuleKind::BridgeToR11);
    assert!(p.ok);
    assert!(p.detail.contains("r11_count=1103"));
    assert!(p.detail.contains("Memory"));
}

// 9. P3 探针 (pool↔type_convert) 详细 verify (JSON 字段)
#[test]
fn stage3_xmod_p3_pool_to_type_convert_detail() {
    let p = probe_pool_to_type_convert();
    assert_eq!(p.kind, CrossModuleKind::PoolToTypeConvert);
    assert!(p.ok);
    assert!(p.detail.contains("hits"));
    assert!(p.detail.contains("misses"));
    assert!(p.detail.contains("cached_modules"));
}

// 10. P4 探针 (asi↔r11) 详细 verify
#[test]
fn stage3_xmod_p4_asi_to_r11_detail() {
    let p = probe_asi_to_r11();
    assert_eq!(p.kind, CrossModuleKind::AsiToR11);
    assert!(p.ok);
    assert!(p.detail.contains("asi_count=7"));
    assert!(p.detail.contains("r11_count=1103"));
}

// 11. P5 探针 (core↔bridge) 详细 verify (3 类型 OK)
#[test]
fn stage3_xmod_p5_core_to_bridge_detail() {
    let p = probe_core_to_bridge();
    assert_eq!(p.kind, CrossModuleKind::CoreToBridge);
    assert!(p.ok);
    assert!(p.detail.contains("Episode OK: true"));
    assert!(p.detail.contains("Session OK: true"));
    assert!(p.detail.contains("Note OK: true"));
}

// 12. Stage 1 7 ASI 关键模块 + 7 类别全覆盖
#[test]
fn stage3_xmod_asi_7_categories_full() {
    let categories = [
        AsiCategory::Measurement,
        AsiCategory::SelfFramework,
        AsiCategory::CrossModularAudit,
        AsiCategory::OperationalRunbook,
        AsiCategory::CeilingChain,
        AsiCategory::HttpGateway,
        AsiCategory::BatchHarness,
    ];
    let mut total = 0;
    for cat in categories {
        let mods = list_asi_stage1_modules_by_category(cat);
        assert!(!mods.is_empty(), "Stage 1 类别 {cat:?} 必有 ≥ 1 模块");
        total += mods.len();
    }
    assert_eq!(total, 7);
    assert_eq!(asi_stage1_module_count(), 7);
}

// 13. ceiling_critical 至少 1 个 (V1458)
#[test]
fn stage3_xmod_ceiling_critical_v1458() {
    let cc = list_ceiling_critical_modules();
    assert!(!cc.is_empty());
    assert!(cc.iter().any(|m| m.version_tag == "V1458"));
    assert!(cc.iter().any(|m| m.is_ceiling_critical));
}

// 14. asi_lookup_by_version / asi_lookup_module 双查
#[test]
fn stage3_xmod_asi_lookup_dual_api() {
    for version in [
        "V1077", "V1400", "V1447", "V1457", "V1458", "V1467", "V1470",
    ] {
        let by_v = asi_lookup_by_version(version);
        assert!(by_v.is_some(), "V{version} 必须查到");
        let info = by_v.unwrap();
        let by_n = asi_lookup_module(info.name);
        assert!(by_n.is_some());
        assert_eq!(by_n.unwrap().version_tag, info.version_tag);
    }
}

// 15. R11 1103 module count 跨 4 API 一致
#[test]
fn stage3_xmod_r11_count_4_apis() {
    assert_eq!(r11_module_count(), 1103);
    assert_eq!(R11_MODULE_COUNT, 1103);
    assert_eq!(r11_compat_version(), R11_COMPAT_VERSION);
    // compile-time const 一致
    const _: () = assert!(R11_MODULE_COUNT == 1103);
}

// 16. r11_module_category 严守
#[test]
fn stage3_xmod_r11_module_categories() {
    assert_eq!(
        r11_module_category("apeireth.memory.store"),
        R11Category::Memory
    );
    assert_eq!(
        r11_module_category("apeireth.identity.continuity"),
        R11Category::Identity
    );
    assert_eq!(
        r11_module_category("apeireth.asi.council"),
        R11Category::Asi
    );
    assert_eq!(
        r11_module_category("apeireth.tools.permissions"),
        R11Category::Tools
    );
    assert_eq!(
        r11_module_category("apeireth.bridge.compat"),
        R11Category::Bridge
    );
    assert_eq!(
        r11_module_category("apeireth.unknown.nope"),
        R11Category::Unknown
    );
}

// 17. BridgeError 4 variant 跨 build 严守 (Stage 1 已锁)
#[test]
fn stage3_xmod_bridge_error_4_variants() {
    let variants = vec![
        BridgeError::ModuleNotFound("x".into()),
        BridgeError::CallFailed("x".into()),
        BridgeError::GilError("x".into()),
        BridgeError::InvalidArg("x".into()),
    ];
    assert_eq!(variants.len(), 4);
    // 4 个建议 action 严守
    assert_eq!(variants[0].suggested_action(), SuggestedAction::Degrade);
    assert_eq!(variants[1].suggested_action(), SuggestedAction::Retry);
    assert_eq!(variants[2].suggested_action(), SuggestedAction::Retry);
    assert_eq!(variants[3].suggested_action(), SuggestedAction::Fail);
}

// 18. Stage 1 7 关键模块名以 "apeireth." 开头
#[test]
fn stage3_xmod_7_module_names_prefix_locked() {
    use apeireth_pybridge::{
        V1077_INFO, V1400_INFO, V1447_INFO, V1457_INFO, V1458_INFO, V1467_INFO, V1470_INFO,
    };
    let all = [
        V1077_INFO, V1400_INFO, V1447_INFO, V1457_INFO, V1458_INFO, V1467_INFO, V1470_INFO,
    ];
    for info in all {
        assert!(info.name.starts_with("apeireth."), "{}", info.name);
        assert!(info.version_tag.starts_with("V"), "{}", info.version_tag);
        assert!(!info.description.is_empty(), "{}", info.version_tag);
    }
}

// 19. Stage 3 报告 modules_in_scope 6 子模块
#[test]
fn stage3_xmod_6_modules_in_scope() {
    let r = stage3_cross_module_probes();
    assert_eq!(r.modules_in_scope.len(), 6);
    let expected = vec![
        "bridge",
        "bridge_pool",
        "type_convert",
        "asi_modules",
        "r11_compat",
        "apeireth-core (cross-crate)",
    ];
    for e in &expected {
        assert!(r.modules_in_scope.iter().any(|m| m == e), "missing: {e}");
    }
}

// 20. Stage 3 报告跨 stage1 + r11 版本严守
#[test]
fn stage3_xmod_stage1_r11_versions_locked() {
    let r = stage3_cross_module_probes();
    assert_eq!(r.stage1_version, asi_stage1_version());
    assert_eq!(r.r11_compat_version, r11_compat_version());
    assert_eq!(r.r11_compat_version, R11_COMPAT_VERSION);
}

// 21. Stage 3 跨模块综合 verify (8 硬墙 + 5 探针 + 6 子模块)
#[test]
fn stage3_xmod_full_integration() {
    let walls = HardWallsVerify::auto_verify();
    let report = stage3_cross_module_probes();
    assert!(walls.all_pass());
    assert!(report.all_ok);
    assert_eq!(report.probe_results.len(), 5);
    assert_eq!(report.modules_in_scope.len(), 6);
}

// 22. Stage 3 跨模块 5 probe 跨 N 次调用稳定
#[test]
fn stage3_xmod_idempotent_runs() {
    let r1 = stage3_cross_module_probes();
    let r2 = stage3_cross_module_probes();
    assert_eq!(r1.probe_results.len(), r2.probe_results.len());
    for (a, b) in r1.probe_results.iter().zip(r2.probe_results.iter()) {
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.ok, b.ok);
    }
    assert_eq!(r1.all_ok, r2.all_ok);
}

// 23. 单探针函数 5 调 ok 跨 N 次稳定
#[test]
fn stage3_xmod_single_probes_idempotent() {
    for _ in 0..3 {
        let p1 = probe_bridge_to_pool();
        let p2 = probe_bridge_to_r11();
        let p3 = probe_pool_to_type_convert();
        let p4 = probe_asi_to_r11();
        let p5 = probe_core_to_bridge();
        assert!(p1.ok && p2.ok && p3.ok && p4.ok && p5.ok);
    }
}
