//! ASI Python 关键模块 Stage 1 整合集成测试
//!
//! R128 阶段 A Stage 1 (per decision-57 §2.1 P10-1):
//! 验证 Stage 1 整合的 7 个关键 ASI Python 模块
//! - V1077 / V1400 / V1447 / V1457 / V1458 / V1467 / V1470
//!
//! 集成测试覆盖:
//! 1. 7 模块元数据完整性 (per AsiModuleInfo)
//! 2. 关键模块架构常数 (V1077 dim 17 / V1400 cap 12+limit 6 / V1447 pair 35+probe 175
//!    / V1457 stage 5+probe 30+weights sum=1 / V1458 anchor 0.9105 LOCKED /
//!    V1467 endpoint 6 / V1470 cross-checks 12+total 36)
//! 3. V1458 ceiling chain math (anchor 0.9105, north_star 0.98, gap 0.0695, ±0.0001)
//! 4. V1400 12 capabilities + 6 limits (mirror Python)
//! 5. V1447 35 audit pairs (7 problems × 5 positions, 笛卡尔积)
//! 6. cfg-gated 0 装 PASS 严守 (默认 build 桥接 → ModuleNotFound)
//! 7. V1447_AUDIT_PAIRS const 矩阵正确性
//! 8. V1457 OperationalStage::ALL weight sum verify
//! 9. V1467 6 endpoints + 1 POST + 5 GET
//! 10. V1470 cross-check 12 per run × 3 runs = 36 total

use apeireth_pybridge::*;

// =============================================================================
// 1. Stage 1 版本 + 模块计数
// =============================================================================

#[test]
fn smoke_stage1_version_and_count() {
    assert_eq!(asi_stage1_version(), "0.1.0-R128-Stage1");
    assert_eq!(asi_stage1_module_count(), 7);
    assert_eq!(ASI_STAGE1_MODULE_COUNT, 7);
    assert_eq!(ASI_STAGE1_MODULES.len(), 7);
    assert_eq!(ASI_STAGE1_INFOS.len(), 7);
    assert_eq!(ASI_PYTHON_DIR, "apeireth/");
}

// =============================================================================
// 2. 7 模块 catalog 完整性
// =============================================================================

#[test]
fn smoke_all_7_modules_recognized() {
    for m in ASI_STAGE1_MODULES.iter() {
        assert!(is_known_asi_stage1_module(m), "module {m} should be known");
        let info = asi_lookup_module(m).expect("must lookup");
        assert_eq!(info.name, *m);
    }
    // 未知 module 不识别
    assert!(!is_known_asi_stage1_module("apeireth.v9999_unknown"));
    assert!(asi_lookup_module("apeireth.v9999_unknown").is_none());
}

#[test]
fn smoke_lookup_by_version() {
    for (version, expected_name) in [
        ("V1077", V1077_MODULE),
        ("V1400", V1400_MODULE),
        ("V1447", V1447_MODULE),
        ("V1457", V1457_MODULE),
        ("V1458", V1458_MODULE),
        ("V1467", V1467_MODULE),
        ("V1470", V1470_MODULE),
    ] {
        let info = asi_lookup_by_version(version).expect("must lookup by version");
        assert_eq!(info.version_tag, version);
        assert_eq!(info.name, expected_name);
    }
    assert!(asi_lookup_by_version("V9999").is_none());
}

#[test]
fn smoke_list_by_category() {
    use apeireth_pybridge::AsiCategory;

    // 每类至少 1 个
    for cat in [
        AsiCategory::Measurement,
        AsiCategory::SelfFramework,
        AsiCategory::CrossModularAudit,
        AsiCategory::OperationalRunbook,
        AsiCategory::CeilingChain,
        AsiCategory::HttpGateway,
        AsiCategory::BatchHarness,
    ] {
        let list = list_asi_stage1_modules_by_category(cat);
        assert_eq!(list.len(), 1, "category {cat:?} should have 1 module");
    }
}

#[test]
fn smoke_ceiling_critical_only_v1458() {
    let cc = list_ceiling_critical_modules();
    assert_eq!(cc.len(), 1);
    assert_eq!(cc[0].version_tag, "V1458");
    assert!(cc[0].is_ceiling_critical);
}

// =============================================================================
// 3. 关键模块架构常数
// =============================================================================

#[test]
fn smoke_v1077_17_dim() {
    assert_eq!(V1077_N_DIMENSIONS, 17);
    assert!((V1077_WEIGHT_SUM - 1.0).abs() < 1e-9);
    assert!((V1077_WEIGHT_TOLERANCE - 0.0001).abs() < 1e-9);
}

#[test]
fn smoke_v1400_12_cap_6_limit_12_rule() {
    assert_eq!(V1400_N_CAPABILITIES, 12);
    assert_eq!(V1400_N_LIMITS, 6);
    assert_eq!(V1400_N_RULES, 12);
    assert_eq!(V1400_CAPABILITIES.len(), 12);
    assert_eq!(V1400_LIMITS.len(), 6);
}

#[test]
fn smoke_v1447_35_pairs_175_probes_1190_links() {
    assert_eq!(V1447_N_PROBLEMS, 7);
    assert_eq!(V1447_N_POSITIONS, 5);
    assert_eq!(V1447_N_CLOSURE_KINDS, 5);
    assert_eq!(V1447_N_PAIRS, 35);
    assert_eq!(V1447_N_COMBINED_PROBES, 175);
    assert_eq!(V1447_N_CROSS_PAIR_LINKS, 1190);
    assert_eq!(V1447_AUDIT_PAIRS.len(), 35);
}

#[test]
fn smoke_v1457_5_stages_30_probes_sum_one() {
    assert_eq!(V1457_N_DEPLOYMENTS, 6);
    assert_eq!(V1457_N_STAGES, 5);
    assert_eq!(V1457_N_PROBES, 30);
    let sum: f64 = OperationalStage::ALL.iter().map(|s| s.weight()).sum();
    assert!(
        (sum - 1.0).abs() < V1458_TOLERANCE,
        "stage weight sum = {sum}"
    );
    assert!((V1457_STAGE_WEIGHT_SUM - 1.0).abs() < 1e-9);
}

#[test]
fn smoke_v1458_ceiling_chain_locked() {
    // 主 22:33 ASI 北极星 — 严守 V1256 unio_mystica 0.9105
    assert!((V1458_ANCHOR_VALUE - 0.9105).abs() < 1e-9);
    assert!((V1458_NORTH_STAR_CEILING - 0.98).abs() < 1e-9);
    assert!((V1458_ABSOLUTE_CEILING - 1.0).abs() < 1e-9);
    assert!((V1458_GAP_TO_NORTH_STAR - 0.0695).abs() < 1e-9);
    assert!((V1458_GAP_TO_CEILING - 0.0895).abs() < 1e-9);
    assert!((V1458_TOLERANCE - 0.0001).abs() < 1e-9);
    assert_eq!(V1458_N_CEILING_MODULES, 5);
    assert_eq!(V1458_N_DEPLOYMENT_CUBE_MODULES, 4);
    assert_eq!(V1458_N_BOUNDED_PROBES, 34);
}

#[test]
fn smoke_v1467_6_endpoints() {
    assert_eq!(V1467_N_ENDPOINTS, 6);
    assert_eq!(V1467Endpoint::ALL.len(), 6);
    // 1 POST + 5 GET
    let n_post = V1467Endpoint::ALL
        .iter()
        .filter(|e| e.method() == "POST")
        .count();
    let n_get = V1467Endpoint::ALL
        .iter()
        .filter(|e| e.method() == "GET")
        .count();
    assert_eq!(n_post, 1);
    assert_eq!(n_get, 5);
    // 路径都 /
    for ep in V1467Endpoint::ALL.iter() {
        assert!(ep.path().starts_with('/'));
    }
}

#[test]
fn smoke_v1470_12_per_run_36_total() {
    assert_eq!(V1470_MIN_BATCH_N, 2);
    assert_eq!(V1470_DEFAULT_BATCH_N, 3);
    assert_eq!(V1470_N_ENDPOINTS, 6);
    assert_eq!(V1470_N_CLIENT_PATHS, 2);
    assert_eq!(V1470_N_CROSS_CHECKS_PER_RUN, 12);
    assert_eq!(V1470_N_CROSS_CHECKS_TOTAL, 36);
}

// =============================================================================
// 4. V1458 ceiling chain math (per CeilingChainLock::LOCKED)
// =============================================================================

#[test]
fn smoke_v1458_ceiling_chain_lock_default_ok() {
    let lock = CeilingChainLock::LOCKED;
    assert!(lock.verify_internal_consistency());
    assert!(lock.no_inflation());
    assert!(lock.no_lowered_north_star());
    assert!(lock.no_lowered_ceiling());
    // 数学 gap 一致
    let computed_gap_ns = lock.north_star_ceiling - lock.anchor_value;
    let computed_gap_c = lock.absolute_ceiling - lock.anchor_value;
    assert!((computed_gap_ns - V1458_GAP_TO_NORTH_STAR).abs() < V1458_TOLERANCE);
    assert!((computed_gap_c - V1458_GAP_TO_CEILING).abs() < V1458_TOLERANCE);
}

#[test]
fn smoke_v1458_inflation_detected() {
    let inflated = CeilingChainLock {
        anchor_value: 0.95, // 0.95 > 0.9105
        ..CeilingChainLock::LOCKED
    };
    assert!(!inflated.no_inflation());
    assert!(!inflated.verify_internal_consistency());
}

#[test]
fn smoke_v1458_lowered_north_star_detected() {
    let lowered = CeilingChainLock {
        north_star_ceiling: 0.95, // 0.95 < 0.98
        ..CeilingChainLock::LOCKED
    };
    assert!(!lowered.no_lowered_north_star());
}

#[test]
fn smoke_v1458_v1411_0_99_alternative_accepted() {
    // V1411 用 0.99 (not 1.0), V1458 verify 接受两种
    let v1411 = CeilingChainLock {
        absolute_ceiling: 0.99,
        ..CeilingChainLock::LOCKED
    };
    assert!(v1411.verify_internal_consistency());
    assert!(v1411.no_lowered_ceiling());
}

// =============================================================================
// 5. V1400 12 能力 + 6 限制 镜像
// =============================================================================

#[test]
fn smoke_v1400_capabilities_unique_ids() {
    let mut ids: Vec<&str> = V1400_CAPABILITIES.iter().map(|c| c.id).collect();
    ids.sort();
    let n_unique = ids.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(n_unique, 12, "V1400 capabilities must have unique IDs");
    // evidence 字段非空
    for cap in V1400_CAPABILITIES.iter() {
        assert!(!cap.evidence.is_empty());
        assert!(!cap.label.is_empty());
    }
}

#[test]
fn smoke_v1400_limits_unique_ids_and_not_asi_claims() {
    let mut ids: Vec<&str> = V1400_LIMITS.iter().map(|l| l.id).collect();
    ids.sort();
    let n_unique = ids.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(n_unique, 6, "V1400 limits must have unique IDs");
    // 主 17:58 不假装: 每个 limit id 暗示 "不假装" / "不刷" / "0 装"
    for limit in V1400_LIMITS.iter() {
        assert!(!limit.id.is_empty());
        assert!(!limit.evidence.is_empty());
    }
    // 关键 limit ID 存在
    let ids: Vec<&str> = V1400_LIMITS.iter().map(|l| l.id).collect();
    assert!(ids.contains(&"not_phenomenal"));
    assert!(ids.contains(&"not_asi_achieved"));
    assert!(ids.contains(&"no_kpi_wash"));
}

// =============================================================================
// 6. V1447 35 audit pair 笛卡尔积 verify
// =============================================================================

#[test]
fn smoke_v1447_audit_pairs_cartesian() {
    // 7 problems × 5 positions = 35 unique pairs (no duplicates)
    let mut seen = std::collections::HashSet::new();
    for pair in V1447_AUDIT_PAIRS.iter() {
        let key = (pair.problem, pair.position);
        assert!(seen.insert(key), "duplicate audit pair: {key:?}");
    }
    assert_eq!(seen.len(), 35);
}

#[test]
fn smoke_v1447_problems_and_positions_all_present() {
    use apeireth_pybridge::{PhilosophicalProblem, V2Position};
    // 7 problems 全出现
    let mut problems_seen = std::collections::HashSet::new();
    let mut positions_seen = std::collections::HashSet::new();
    for pair in V1447_AUDIT_PAIRS.iter() {
        problems_seen.insert(pair.problem);
        positions_seen.insert(pair.position);
    }
    for p in PhilosophicalProblem::ALL.iter() {
        assert!(problems_seen.contains(p), "problem {p:?} missing");
    }
    for pos in V2Position::ALL.iter() {
        assert!(positions_seen.contains(pos), "position {pos:?} missing");
    }
}

// =============================================================================
// 7. cfg-gated 0 装 PASS 严守 (per decision-33 §2.3 C2)
// =============================================================================

#[test]
fn smoke_0装_pass_默认_build_degrades() {
    // 默认 build (无 python-ext): 桥接函数返回 ModuleNotFound
    if !python_ext_enabled() {
        let r1 = bridge_v1077_full_measure();
        assert!(r1.is_err());
        assert_eq!(r1.unwrap_err().suggested_action(), SuggestedAction::Degrade);

        let r2 = bridge_v1458_ceiling_audit();
        assert!(r2.is_err());
        assert_eq!(r2.unwrap_err().suggested_action(), SuggestedAction::Degrade);

        let r3 = bridge_v1457_deploy_all();
        assert!(r3.is_err());
        assert_eq!(r3.unwrap_err().suggested_action(), SuggestedAction::Degrade);
    }
}

#[test]
fn smoke_python_ext_enabled_consistent() {
    // python_ext_enabled() 必须 = cfg!(feature = "python-ext")
    let runtime = python_ext_enabled();
    let compile_time = cfg!(feature = "python-ext");
    assert_eq!(runtime, compile_time);
}

// =============================================================================
// 8. 综合 health check + all invariants
// =============================================================================

#[test]
fn smoke_stage1_health_struct() {
    let h = asi_stage1_health();
    assert_eq!(h.stage1_version, "0.1.0-R128-Stage1");
    assert_eq!(h.module_count, 7);
    assert_eq!(h.ceiling_critical_count, 1);
    assert_eq!(h.known_modules.len(), 7);
    assert_eq!(h.python_ext_active, cfg!(feature = "python-ext"));

    let s = format!("{h}");
    assert!(s.contains("0.1.0-R128-Stage1"));
    assert!(s.contains("关键模块数: 7"));
    assert!(s.contains("ceiling-critical: 1"));
    // 7 模块名都在
    for v_name in [
        "v1077_asi_v04_full_measurement",
        "v1400_asi_self_framework",
        "v1447_asi_cross_modular_audit",
        "v1457_asi_six_deployment_operational_runbook",
        "v1458_asi_north_star_ceiling_chain_audit",
        "v1467_asi_audit_http_gateway_history_diff",
        "v1470_asi_v1469_batch_harness_cross_client_equivalence",
    ] {
        assert!(s.contains(v_name), "health display missing {v_name}");
    }
}

#[test]
fn smoke_bridge_health_alias_matches() {
    let a = asi_stage1_health();
    let b = asi_stage1_bridge_health();
    assert_eq!(a, b);
}

#[test]
fn smoke_all_invariants_ok() {
    assert!(asi_stage1_all_invariants_ok());
    // 7 invariants 各自 verify
    assert!(asi_stage1_ceiling_chain_locked());
    assert!(asi_stage1_v1457_weights_sum_one());
    assert!(asi_stage1_v1447_pair_count());
    assert!(asi_stage1_v1077_dim_count());
    assert!(asi_stage1_v1400_capabilities_limits());
    assert!(asi_stage1_v1467_endpoint_count());
    assert!(asi_stage1_v1470_cross_checks());
}

// =============================================================================
// 9. Stage 1 完整 cross-validation
// =============================================================================

#[test]
fn smoke_stage1_cross_validation_complete() {
    // 7 模块 + 7 invariants + 7 module 元数据 + 0 装 PASS
    let h = asi_stage1_health();
    assert_eq!(h.module_count, 7);

    // 7 invariants verify
    assert!(asi_stage1_all_invariants_ok());

    // 7 module 元数据 各自 verify
    assert!(is_known_asi_stage1_module(V1077_MODULE));
    assert!(is_known_asi_stage1_module(V1400_MODULE));
    assert!(is_known_asi_stage1_module(V1447_MODULE));
    assert!(is_known_asi_stage1_module(V1457_MODULE));
    assert!(is_known_asi_stage1_module(V1458_MODULE));
    assert!(is_known_asi_stage1_module(V1467_MODULE));
    assert!(is_known_asi_stage1_module(V1470_MODULE));

    // 0 装 PASS
    if !python_ext_enabled() {
        assert!(bridge_v1077_full_measure().is_err());
        assert!(bridge_v1458_ceiling_audit().is_err());
        assert!(bridge_v1457_deploy_all().is_err());
    }
}

#[test]
fn smoke_no_unrelated_modules_known() {
    // Stage 1 严格只 7 个, 不混入 R11 1100+ 或 V1471+ 后续模块
    assert_eq!(ASI_STAGE1_MODULE_COUNT, 7);
    let known = ASI_STAGE1_MODULES
        .iter()
        .collect::<std::collections::HashSet<_>>();
    // Stage 1 不含 V1471 (audit monitor daemon)
    let v1471 = "apeireth.v1471_audit_monitor_daemon";
    assert!(!known.contains(&v1471));
    // Stage 1 不含 V1472 (daemon supervisor)
    let v1472 = "apeireth.v1472_daemon_supervisor";
    assert!(!known.contains(&v1472));
    // Stage 1 不含 V1473+ (alerting / aggregator / etc.)
    let v1473 = "apeireth.v1473_asi_v1472_alerting_engine";
    assert!(!known.contains(&v1473));
}

#[test]
fn smoke_v1400_evidence_contains_known_versions() {
    // evidence 字段应包含 V# 或 commit 引用
    let all_evidence: String = V1400_CAPABILITIES
        .iter()
        .map(|c| c.evidence)
        .collect::<Vec<&str>>()
        .join(" ");
    // 至少含 R11 baseline 3 值 或 V1318 / V1313-V1318 / V1256
    assert!(
        all_evidence.contains("V") || all_evidence.contains("commit") || all_evidence.contains("R"),
        "evidence should reference V# / commit / R#"
    );
}
