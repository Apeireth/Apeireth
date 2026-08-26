//! Fixture 5: in-process Rollback Service 验证 (per RIVAL 蓝图 §3.7 缺口 5)
//! + **71GB 事故根因修复 Fixture** (per 主人 2026-08-05 紧急救援)
//!
//! 测 3 类事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. **m3 防御**: `TOOL_WHITELIST` 编译期 hardcode 包含 8 rollback 工具
//! 2. **K-1 强校验**: 5 字样 (apeireth / rollback / snapshot / restore / must-do) 编译期 hardcode
//! 3. **71GB 事故根因修复**: 模拟 91 个 agent-xxxxxx 影子, 验证 4 重防御
//!
//! 5 P0 crate 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_rollback::{
    assert_cleanup_hooks_enabled, check_single_size, check_total_size_with_lru,
    check_ttl, defense_4_check, k1_invariant_5_keys, validate_k1_invariant, validate_tool_call,
    DefaultSnapshotService, RollbackError, RollbackStrategy, ShadowQuotaError, SnapshotIndex,
    SnapshotMeta, SnapshotService, TOOL_WHITELIST, MAX_SHADOW_AGE_DAYS, MAX_SHADOW_SIZE_BYTES,
    MAX_TOTAL_SHADOW_SIZE_BYTES, TOOL_COUNT, V0921_ROLLBACK_STRATEGIES,
};

use std::path::PathBuf;

// ====================================================================
// 类别 1: m3 防御 (TOOL_WHITELIST 8 工具)
// ====================================================================

#[test]
fn test_whitelist_contains_eight_rollback_tools() {
    assert_eq!(TOOL_WHITELIST.len(), 8, "8 工具");
    assert_eq!(TOOL_WHITELIST.len(), TOOL_COUNT, "TOOL_COUNT = 8");
    for tool in [
        "apeireth_rollback_snapshot",
        "apeireth_rollback_list",
        "apeireth_rollback_restore",
        "apeireth_rollback_delete",
        "apeireth_rollback_git_status",
        "apeireth_rollback_git_diff",
        "apeireth_rollback_git_stash",
        "apeireth_rollback_cleanup",
    ] {
        assert!(
            TOOL_WHITELIST.contains(&tool),
            "TOOL_WHITELIST 缺: {tool}"
        );
    }
}

#[test]
fn test_validate_tool_call_accepts_whitelisted() {
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        let result = validate_tool_call(tool, &args);
        assert!(result.is_ok(), "白名单工具 {tool} 应通过: {result:?}");
    }
}

#[test]
fn test_validate_tool_call_rejects_unknown() {
    // m3 hallucination 防御: 不在白名单的工具必须拒绝
    let args = serde_json::json!({});
    let result = validate_tool_call("apeireth_rollback_purge", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        RollbackError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_rollback_purge");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}

// ====================================================================
// 类别 2: K-1 强校验 5 字样 (per supervisor-prompt-818 §5.3 模式)
// ====================================================================

#[test]
fn k1_invariant_5_keys_present() {
    let keys = k1_invariant_5_keys();
    assert_eq!(keys.len(), 5, "K-1 必须 5 字样");
    assert!(keys.contains(&"apeireth"), "K-1 缺 'apeireth' (平台名)");
    assert!(keys.contains(&"rollback"), "K-1 缺 'rollback' (模块名)");
    assert!(keys.contains(&"snapshot"), "K-1 缺 'snapshot' (核心 API)");
    assert!(keys.contains(&"restore"), "K-1 缺 'restore' (核心 API)");
    assert!(keys.contains(&"must-do"), "K-1 缺 'must-do' (翻译 invariant)");
}

#[test]
fn k1_invariant_validate_function_passes() {
    assert!(
        validate_k1_invariant().is_ok(),
        "validate_k1_invariant 必须 Ok"
    );
}

// ====================================================================
// 类别 3: 71GB 事故 4 重防御编译期 hardcode
// ====================================================================

#[test]
fn t71gb_4_defense_constants_hardcoded() {
    let r = defense_4_check();
    // 4 重防御常量值
    assert_eq!(r.max_age_days, 7, "MAX_SHADOW_AGE_DAYS = 7 (TTL)");
    assert_eq!(
        r.max_shadow_bytes,
        100 * 1024 * 1024,
        "MAX_SHADOW_SIZE_BYTES = 100 MB (单影子)"
    );
    assert_eq!(
        r.max_total_bytes,
        2 * 1024 * 1024 * 1024,
        "MAX_TOTAL_SHADOW_SIZE_BYTES = 2 GB (总大小)"
    );
    // 3 清理钩子全启用
    assert!(r.hook_startup, "CLEANUP_HOOK_STARTUP = true");
    assert!(r.hook_before_snapshot, "CLEANUP_HOOK_BEFORE_SNAPSHOT = true");
    assert!(r.hook_cron_daily, "CLEANUP_HOOK_CRON_DAILY = true");
    // assert_cleanup_hooks_enabled 不返 Err
    assert!(assert_cleanup_hooks_enabled().is_ok());
}

#[test]
fn t71gb_single_shadow_size_enforced() {
    // 71GB 防御 #2: 单影子 100 MB 上限
    // < 100 MB: 通过
    assert!(check_single_size(50 * 1024 * 1024).is_ok());
    // = 100 MB: 通过
    assert!(check_single_size(MAX_SHADOW_SIZE_BYTES).is_ok());
    // > 100 MB: 拒绝
    let r = check_single_size(150 * 1024 * 1024);
    assert!(r.is_err());
    match r.unwrap_err() {
        ShadowQuotaError::TooLarge { actual, max } => {
            assert_eq!(actual, 150 * 1024 * 1024);
            assert_eq!(max, MAX_SHADOW_SIZE_BYTES);
        }
        ShadowQuotaError::TotalOverflow { .. } => panic!("期望 TooLarge, 实际 TotalOverflow"),
    }
}

#[test]
fn t71gb_total_size_with_lru_enforced() {
    // 71GB 防御 #3: 总大小 2 GB 上限 + LRU
    let mut idx = SnapshotIndex::new();
    // 0 字节: 通过
    assert!(check_total_size_with_lru(&idx).is_ok());
    // 模拟 5 个 100 MB snapshot (500 MB) - < 2 GB, 通过
    for i in 0..5 {
        idx.add(make_mock_meta(&format!("s-{i}"), 100 * 1024 * 1024));
    }
    assert!(check_total_size_with_lru(&idx).is_ok());
    // 模拟 25 个 100 MB snapshot (2.5 GB) - > 2 GB, 报错
    let mut big_idx = SnapshotIndex::new();
    for i in 0..25 {
        big_idx.add(make_mock_meta(&format!("s-{i}"), 100 * 1024 * 1024));
    }
    let r = check_total_size_with_lru(&big_idx);
    assert!(r.is_err());
    match r.unwrap_err() {
        ShadowQuotaError::TotalOverflow { actual, max } => {
            assert_eq!(actual, 25 * 100 * 1024 * 1024);
            assert_eq!(max, MAX_TOTAL_SHADOW_SIZE_BYTES);
        }
        ShadowQuotaError::TooLarge { .. } => panic!("期望 TotalOverflow, 实际 TooLarge"),
    }
}

#[test]
fn t71gb_ttl_check_expired() {
    // 71GB 防御 #1: TTL 7 天
    // 创建一个 30 天前的 snapshot (过期)
    let mut meta = make_mock_meta("old", 1024);
    meta.timestamp = apeireth_rollback::unix_timestamp() - 30 * 86_400;
    let r = check_ttl(&meta);
    assert!(r.is_err());
    match r.unwrap_err() {
        RollbackError::ShadowExpired { age_days, max_days } => {
            assert!(age_days >= 30, "age_days = {age_days}");
            assert_eq!(max_days, MAX_SHADOW_AGE_DAYS);
        }
        other => panic!("期望 ShadowExpired, 实际: {other:?}"),
    }
}

#[test]
fn t71gb_ttl_check_fresh() {
    // 1 天前的 snapshot (新鲜)
    let mut meta = make_mock_meta("fresh", 1024);
    meta.timestamp = apeireth_rollback::unix_timestamp() - 86_400;
    assert!(check_ttl(&meta).is_ok());
}

// ====================================================================
// **核心 Fixture: t71_gb_incident_defense**
// 模拟 91 个 agent-xxxxxx 影子, 每个 800 MB (mock, 实际小), 验证 4 重防御
// ====================================================================

#[tokio::test]
async fn t71_gb_incident_defense() {
    // 71GB 事故: 91 个影子, 每个 800 MB, 总 72.8 GB
    // 防御: 1) 单影子 100 MB 上限 (原 800 MB 应拒) 2) 总 2 GB 上限 3) TTL 7 天 4) 3 钩子
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut svc = DefaultSnapshotService::new(tmp.path().to_path_buf());

    println!("=== t71_gb_incident_defense ===");
    println!("模拟 91 个 agent-xxxxxx 影子 (原 v0.9.21 71GB 事故场景)");

    // 场景 A: 单影子 800 MB (事故实测大小)
    let r = check_single_size(800 * 1024 * 1024);
    assert!(r.is_err(), "单影子 800 MB 必须被 MAX_SHADOW_SIZE_BYTES 100 MB 拒绝");
    println!("[A] 单影子 800 MB 拒收 (per MAX_SHADOW_SIZE_BYTES 100 MB) ✓");

    // 场景 B: 模拟 91 个 mock snapshot, 每个 100 MB (本 crate 上限)
    // 总 9.1 GB, 超过 MAX_TOTAL_SHADOW_SIZE_BYTES 2 GB, 应触发 TotalOverflow
    let mut idx = SnapshotIndex::new();
    for i in 0..91 {
        idx.add(make_mock_meta(&format!("agent-{i:06x}"), 100 * 1024 * 1024));
    }
    let total = idx.total_size();
    println!(
        "[B] 91 个 mock 影子总大小 = {} 字节 ({:.1} GB)",
        total,
        total as f64 / 1024.0 / 1024.0 / 1024.0
    );
    let r = check_total_size_with_lru(&idx);
    assert!(r.is_err(), "91 个 100 MB 影子 (9.1 GB) 必超 2 GB 上限");
    println!("[B] 9.1 GB 超 MAX_TOTAL_SHADOW_SIZE_BYTES 2 GB, 触发 LRU 清理 ✓");

    // 场景 C: TTL 7 天 - 创建 30 天前 + 1 天前两个 snapshot
    let mut old = make_mock_meta("agent-old", 1024);
    old.timestamp = apeireth_rollback::unix_timestamp() - 30 * 86_400;
    let mut fresh = make_mock_meta("agent-fresh", 1024);
    fresh.timestamp = apeireth_rollback::unix_timestamp() - 86_400;
    assert!(check_ttl(&old).is_err(), "30 天前必须过期");
    assert!(check_ttl(&fresh).is_ok(), "1 天前必须新鲜");
    println!("[C] TTL 7 天: 30 天前过期, 1 天前新鲜 ✓");

    // 场景 D: 3 重清理钩子全启用
    assert!(assert_cleanup_hooks_enabled().is_ok());
    let cleaned = svc.cleanup_startup().await.expect("cleanup_startup");
    println!("[D] cleanup_startup() 返 {cleaned} (3 钩子全启用) ✓");

    // 场景 E: 6 策略 1:1 翻译
    assert_eq!(V0921_ROLLBACK_STRATEGIES, 6, "6 策略");
    let strategies = [
        ("full", RollbackStrategy::Full),
        ("file", RollbackStrategy::File),
        ("diff", RollbackStrategy::Diff),
        ("git", RollbackStrategy::Git),
        ("session", RollbackStrategy::Session),
        ("auto", RollbackStrategy::Auto),
    ];
    for (s_str, s_enum) in strategies {
        assert_eq!(s_enum.as_v0921_str(), s_str, "{s_str} roundtrip");
        assert_eq!(
            RollbackStrategy::from_v0921_str(s_str).expect("parse"),
            s_enum
        );
    }
    println!("[E] 6 策略 1:1 翻译 v0.9.21 ✓");

    // 场景 F: snapshot 服务接口 (in-process)
    let meta = svc
        .snapshot("t71-gb-session", RollbackStrategy::Auto, &[], "71GB 事故 fixture")
        .await
        .expect("snapshot ok");
    assert!(meta.id.starts_with("agent-"), "shadow dir pattern");
    println!("[F] SnapshotService::snapshot() in-process ✓ id={}", meta.id);

    // 场景 G: list / restore / delete 接口
    // 手动 add snapshot 到 index (skeleton 阶段 service 不持久化)
    svc.index.add(meta.clone());
    let all = svc.list(None).await.expect("list ok");
    assert!(all.iter().any(|m| m.id == meta.id), "snapshot 必在 list 内");
    svc.restore(&meta.id).await.expect("restore ok");
    svc.delete(&meta.id).await.expect("delete ok");
    println!("[G] SnapshotService list / restore / delete in-process ✓");

    // 场景 H: m3 防御 - 拒绝虚构的 rollback 工具
    let bad = validate_tool_call("apeireth_rollback_purge", &serde_json::json!({}));
    assert!(bad.is_err(), "虚构工具必须拒");
    println!("[H] m3 防御: 拒绝虚构工具 ✓");

    println!("=== 71GB 事故 4 重防御全部通过 ===");
}

// ====================================================================
// 类别 4: ShadowDirIndex (per snapshot 索引 1:1 翻译)
// ====================================================================

#[test]
fn test_snapshot_index_basic_ops() {
    let mut idx = SnapshotIndex::new();
    assert_eq!(idx.version, "1");
    assert_eq!(idx.snapshots.len(), 0);

    let meta1 = make_mock_meta("agent-aaa", 1024);
    let meta2 = make_mock_meta("agent-bbb", 2048);
    idx.add(meta1.clone());
    idx.add(meta2.clone());
    assert_eq!(idx.snapshots.len(), 2);

    let found = idx.find("agent-aaa").expect("find agent-aaa");
    assert_eq!(found.size_bytes, 1024);

    let removed = idx.remove("agent-aaa").expect("remove agent-aaa");
    assert_eq!(removed.id, "agent-aaa");
    assert_eq!(idx.snapshots.len(), 1);

    let total = idx.total_size();
    assert_eq!(total, 2048);
}

#[test]
fn test_snapshot_index_list_expired() {
    let mut idx = SnapshotIndex::new();
    let mut old = make_mock_meta("old", 1024);
    old.timestamp = apeireth_rollback::unix_timestamp() - 30 * 86_400;
    let fresh = make_mock_meta("fresh", 2048);
    idx.add(old);
    idx.add(fresh);
    let expired = idx.list_expired();
    assert_eq!(expired.len(), 1, "30 天前过期");
    assert_eq!(expired[0].id, "old");
}

// ====================================================================
// Helper: 构造 mock SnapshotMeta
// ====================================================================

fn make_mock_meta(id: &str, size_bytes: u64) -> SnapshotMeta {
    SnapshotMeta {
        id: id.to_string(),
        session_id: "mock-session".to_string(),
        timestamp: apeireth_rollback::unix_timestamp(),
        shadow_dir: PathBuf::from(format!("/tmp/{id}")),
        size_bytes,
        strategy: RollbackStrategy::Auto,
        file_diff: "mock diff".to_string(),
        branch_state: serde_json::json!({}),
        description: "71GB 事故 mock".to_string(),
    }
}
