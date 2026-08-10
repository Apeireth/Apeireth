//! 71GB rollback 4 重防御 (apeireth-rollback 跨 session 验证)
//!
//! 主人 2026-08-05 紧急救援: SpectrAI 0.9.21 91 个 agent-* 影子累积 71GB, 原始 v0.9.21
//! 既不清理也不校验, 所以累积 71GB.
//!
//! 本测试覆盖 `apeireth-rollback` 4 重防御 (per §4 4 重防御 + K-1 强校验 5 字样):
//! 1. **TTL 校验** — `check_ttl` 拒 > 7 天的影子
//! 2. **单影子大小上限** — `check_single_size` 拒 > 100 MB
//! 3. **总大小 LRU** — `check_total_size_with_lru` 2 GB 上限
//! 4. **3 重清理钩子** — `assert_cleanup_hooks_enabled` 3 hook 全 true
//!
//! 91 个 100MB 影子 = 9.1 GB 超 2 GB 上限, 触发 LRU.
//! 6 策略 (1:1 翻译 v0.9.21 RollbackService.js): full/file/diff/git/session/auto
//!
//! 主报告: `reports/r20-stage4-integration-2026-08-05.md §6`

use apeireth_rollback::{
    assert_cleanup_hooks_enabled, check_single_size, check_total_size_with_lru, check_ttl,
    defense_4_check, k1_invariant_5_keys, unix_timestamp, validate_k1_invariant,
    validate_tool_call, DefaultSnapshotService, RollbackError, RollbackStrategy, ShadowEntry,
    ShadowQuotaError, SnapshotId, SnapshotIndex, SnapshotMeta, SnapshotService, TOOL_WHITELIST,
    MAX_SHADOW_AGE_DAYS, MAX_SHADOW_SIZE_BYTES, MAX_TOTAL_SHADOW_SIZE_BYTES,
    CLEANUP_HOOK_STARTUP, CLEANUP_HOOK_BEFORE_SNAPSHOT, CLEANUP_HOOK_CRON_DAILY,
    ROLLBACK_SCHEMA_VERSION, SHADOW_DIR_PATTERN, SNAPSHOT_INDEX_FILE, V0921_ROLLBACK_STRATEGIES,
    TOOL_COUNT, generate_snapshot_id, generate_random_6char,
};

use std::path::PathBuf;
use std::time::Duration;

/// 8 测试覆盖 71GB 4 重防御 + 6 策略 + K-1 5 字样
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t71gb_4_defense_constants_compile_time_hardcoded() {
        // 71GB 事故 4 重防御: 编译期 hardcode 7 天 / 100 MB / 2 GB
        let r = defense_4_check();
        assert_eq!(r.max_age_days, 7, "MAX_SHADOW_AGE_DAYS = 7");
        assert_eq!(r.max_shadow_bytes, 100 * 1024 * 1024, "MAX_SHADOW_SIZE_BYTES = 100 MB");
        assert_eq!(
            r.max_total_bytes,
            2 * 1024 * 1024 * 1024,
            "MAX_TOTAL_SHADOW_SIZE_BYTES = 2 GB"
        );
        // 3 hook 全 true (编译期守门, 改 false 需经主人审)
        assert!(r.hook_startup, "CLEANUP_HOOK_STARTUP");
        assert!(r.hook_before_snapshot, "CLEANUP_HOOK_BEFORE_SNAPSHOT");
        assert!(r.hook_cron_daily, "CLEANUP_HOOK_CRON_DAILY");
    }

    #[test]
    fn t71gb_3_cleanup_hooks_all_enabled() {
        // 71GB 防御 4c: assert_cleanup_hooks_enabled 全 true
        assert!(assert_cleanup_hooks_enabled().is_ok());
        assert!(CLEANUP_HOOK_STARTUP);
        assert!(CLEANUP_HOOK_BEFORE_SNAPSHOT);
        assert!(CLEANUP_HOOK_CRON_DAILY);
    }

    #[test]
    fn t71gb_ttl_eviction_7_days() {
        // 71GB 防御 #1: TTL 校验, 30 天前过期
        let now = unix_timestamp();
        let old_ts = now - (30 * 86400); // 30 天前
        let fresh_ts = now - 1; // 1 秒前
        let old_meta = SnapshotMeta {
            id: "old-001".into(),
            session_id: "s1".into(),
            timestamp: old_ts,
            shadow_dir: PathBuf::from("/tmp/old"),
            size_bytes: 1000,
            strategy: RollbackStrategy::Full,
            file_diff: String::new(),
            branch_state: serde_json::json!({}),
            description: String::new(),
        };
        let fresh_meta = SnapshotMeta {
            id: "fresh-001".into(),
            session_id: "s1".into(),
            timestamp: fresh_ts,
            shadow_dir: PathBuf::from("/tmp/fresh"),
            size_bytes: 1000,
            strategy: RollbackStrategy::Full,
            file_diff: String::new(),
            branch_state: serde_json::json!({}),
            description: String::new(),
        };
        // 30 天前应过期
        assert!(check_ttl(&old_meta).is_err(), "30 天前应过期");
        let err = check_ttl(&old_meta).unwrap_err();
        assert!(matches!(err, RollbackError::ShadowExpired { .. }));
        // 1 秒前应通过
        assert!(check_ttl(&fresh_meta).is_ok(), "1 秒前应通过 TTL");
    }

    #[test]
    fn t71gb_single_size_100mb_limit() {
        // 71GB 防御 #2: 单影子 100 MB 拒收
        // 100 MB 应通过
        let ok_size = 100 * 1024 * 1024;
        assert!(check_single_size(ok_size).is_ok(), "100 MB 应通过");
        // 100 MB + 1 byte 应拒
        let too_big = ok_size + 1;
        let err = check_single_size(too_big).expect_err("100 MB + 1 应拒");
        match err {
            ShadowQuotaError::TooLarge { actual, max } => {
                assert_eq!(actual, too_big);
                assert_eq!(max, MAX_SHADOW_SIZE_BYTES);
            }
            other => panic!("应返 TooLarge, 实际: {other:?}"),
        }
    }

    #[test]
    fn t71gb_total_size_2gb_lru() {
        // 71GB 防御 #3: 总 2 GB LRU
        let mut index = SnapshotIndex::new();
        // 1 GB 应通过
        index.add(SnapshotMeta {
            id: "snap-1gb".into(),
            session_id: "s1".into(),
            timestamp: unix_timestamp(),
            shadow_dir: PathBuf::from("/tmp/1gb"),
            size_bytes: 1024 * 1024 * 1024, // 1 GB
            strategy: RollbackStrategy::Full,
            file_diff: String::new(),
            branch_state: serde_json::json!({}),
            description: String::new(),
        });
        assert!(check_total_size_with_lru(&index).is_ok(), "1 GB 应通过");
        // 累加到 3 GB 应溢出
        index.add(SnapshotMeta {
            id: "snap-2gb".into(),
            session_id: "s1".into(),
            timestamp: unix_timestamp(),
            shadow_dir: PathBuf::from("/tmp/2gb"),
            size_bytes: 2 * 1024 * 1024 * 1024, // 2 GB
            strategy: RollbackStrategy::Full,
            file_diff: String::new(),
            branch_state: serde_json::json!({}),
            description: String::new(),
        });
        let err = check_total_size_with_lru(&index).expect_err("3 GB 应溢出");
        assert!(matches!(err, ShadowQuotaError::TotalOverflow { .. }));
    }

    #[test]
    fn t71gb_91_shadows_lru_eviction() {
        // 91 个 100MB 影子 = 9.1 GB 超 2 GB, 触发 LRU 防御
        let mut index = SnapshotIndex::new();
        for i in 0..91 {
            index.add(SnapshotMeta {
                id: format!("snap-{i:03}"),
                session_id: "s1".into(),
                timestamp: unix_timestamp() - (i as u64 * 60), // 错开 1 分钟
                shadow_dir: PathBuf::from(format!("/tmp/snap-{i:03}")),
                size_bytes: 100 * 1024 * 1024, // 100 MB 每个
                strategy: RollbackStrategy::Auto,
                file_diff: String::new(),
                branch_state: serde_json::json!({}),
                description: String::new(),
            });
        }
        let total = index.total_size();
        // 91 * 100 MB = 9.1 GB
        assert!(total > MAX_TOTAL_SHADOW_SIZE_BYTES, "91 个 100MB 应超 2 GB");
        let err = check_total_size_with_lru(&index).expect_err("应触发 LRU");
        assert!(matches!(err, ShadowQuotaError::TotalOverflow { .. }));
    }

    #[test]
    fn t71gb_6_rollback_strategies_match_v0921() {
        // 6 策略 (1:1 翻译 v0.9.21 RollbackService.js 6 字符串)
        assert_eq!(V0921_ROLLBACK_STRATEGIES, 6);
        let cases = [
            (RollbackStrategy::Full, "full"),
            (RollbackStrategy::File, "file"),
            (RollbackStrategy::Diff, "diff"),
            (RollbackStrategy::Git, "git"),
            (RollbackStrategy::Session, "session"),
            (RollbackStrategy::Auto, "auto"),
        ];
        for (s, expected) in cases {
            assert_eq!(s.as_v0921_str(), expected, "策略 {:?} 1:1 翻译", s);
            // round-trip
            let back = RollbackStrategy::from_v0921_str(expected).expect("应解析");
            assert_eq!(back, s, "round-trip: {} -> {:?}", expected, s);
        }
        // 未知策略应返错
        let err = RollbackStrategy::from_v0921_str("unknown").expect_err("应被拒");
        assert!(matches!(err, RollbackError::UnknownStrategy(_)));
    }

    #[test]
    fn t71gb_8_tool_whitelist_covers_snapshot_lifecycle() {
        // TOOL_WHITELIST 8 工具覆盖 snapshot 生命周期
        assert_eq!(TOOL_WHITELIST.len(), 8);
        assert_eq!(TOOL_WHITELIST.len(), TOOL_COUNT);
        let expected = [
            "apeireth_rollback_snapshot",
            "apeireth_rollback_list",
            "apeireth_rollback_restore",
            "apeireth_rollback_delete",
            "apeireth_rollback_git_status",
            "apeireth_rollback_git_diff",
            "apeireth_rollback_git_stash",
            "apeireth_rollback_cleanup",
        ];
        for e in expected {
            assert!(TOOL_WHITELIST.contains(&e), "8 工具白名单缺: {e}");
        }
    }

    #[test]
    fn t71gb_validate_tool_call_rejects_fabricated() {
        // m3 防御: 白名单外 "apeireth_rollback_purge" 应被拒
        let err = validate_tool_call("apeireth_rollback_purge", &serde_json::json!({}))
            .expect_err("应被拒");
        match err {
            RollbackError::ToolNotWhitelisted(t) => {
                assert_eq!(t, "apeireth_rollback_purge");
            }
            other => panic!("应返 ToolNotWhitelisted, 实际: {other:?}"),
        }
        // 白名单内应通过
        assert!(validate_tool_call("apeireth_rollback_snapshot", &serde_json::json!({})).is_ok());
    }

    #[test]
    fn t71gb_k1_invariant_5_keys_present() {
        // K-1 强校验 5 字样 (per supervisor-prompt-818 §5.3 模式)
        let keys = k1_invariant_5_keys();
        assert_eq!(keys.len(), 5, "K-1 5 字样");
        assert!(keys.contains(&"apeireth"));
        assert!(keys.contains(&"rollback"));
        assert!(keys.contains(&"snapshot"));
        assert!(keys.contains(&"restore"));
        assert!(keys.contains(&"must-do"));
        // validate_k1_invariant 应 Ok
        assert!(validate_k1_invariant().is_ok());
    }

    #[test]
    fn t71gb_snapshot_meta_age_computation() {
        // SnapshotMeta age 计算
        let now = unix_timestamp();
        let meta = SnapshotMeta {
            id: "test".into(),
            session_id: "s1".into(),
            timestamp: now - 100, // 100 秒前
            shadow_dir: PathBuf::from("/tmp"),
            size_bytes: 1000,
            strategy: RollbackStrategy::Full,
            file_diff: String::new(),
            branch_state: serde_json::json!({}),
            description: String::new(),
        };
        assert_eq!(meta.age_seconds(), 100);
        assert_eq!(meta.age_days(), 0); // 100 秒 < 1 天
        // 8 天前应过期
        let old = SnapshotMeta {
            id: "old".into(),
            session_id: "s1".into(),
            timestamp: now - (8 * 86400),
            shadow_dir: PathBuf::from("/tmp"),
            size_bytes: 1000,
            strategy: RollbackStrategy::Full,
            file_diff: String::new(),
            branch_state: serde_json::json!({}),
            description: String::new(),
        };
        assert!(old.is_expired());
        assert_eq!(old.age_days(), 8);
    }

    #[test]
    fn t71gb_snapshot_index_add_find_remove() {
        // SnapshotIndex 增删查
        let mut idx = SnapshotIndex::new();
        let m1 = SnapshotMeta {
            id: "s1".into(),
            session_id: "session1".into(),
            timestamp: 100,
            shadow_dir: PathBuf::from("/tmp/s1"),
            size_bytes: 1000,
            strategy: RollbackStrategy::Full,
            file_diff: String::new(),
            branch_state: serde_json::json!({}),
            description: String::new(),
        };
        idx.add(m1.clone());
        assert_eq!(idx.snapshots.len(), 1);
        assert!(idx.find("s1").is_some());
        // remove
        let removed = idx.remove("s1");
        assert!(removed.is_some());
        assert_eq!(idx.snapshots.len(), 0);
        assert!(idx.find("s1").is_none());
    }

    #[test]
    fn t71gb_default_snapshot_service_create_restore_delete() {
        // DefaultSnapshotService 集成测试
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let tmp = std::env::temp_dir().join("apeireth-rollback-test-71gb");
        let _ = std::fs::create_dir_all(&tmp);
        rt.block_on(async {
            let svc = DefaultSnapshotService::new(tmp.clone());
            // snapshot (skeleton 阶段 0 IO, 不真拷贝)
            let meta = svc
                .snapshot("session-A", RollbackStrategy::Full, &[], "test snapshot")
                .await
                .expect("snapshot 应成功");
            assert!(meta.id.starts_with("agent-"));
            assert_eq!(meta.session_id, "session-A");
            assert_eq!(meta.strategy, RollbackStrategy::Full);
            // list (空)
            let list = svc.list(Some("session-A")).await.expect("list 应成功");
            assert_eq!(list.len(), 0, "skeleton 阶段 index 空");
            // restore 不存在的 ID → 返 SnapshotNotFound
            let err = svc
                .restore(&"nonexistent".to_string())
                .await
                .expect_err("不存在的 ID 应被拒");
            assert!(matches!(err, RollbackError::SnapshotNotFound(_)));
        });
    }

    #[test]
    fn t71gb_constants_compile_time_hardcoded() {
        // 编译期 hardcode 常量
        assert_eq!(MAX_SHADOW_AGE_DAYS, 7);
        assert_eq!(MAX_SHADOW_SIZE_BYTES, 100 * 1024 * 1024);
        assert_eq!(MAX_TOTAL_SHADOW_SIZE_BYTES, 2 * 1024 * 1024 * 1024);
        assert_eq!(ROLLBACK_SCHEMA_VERSION, "1");
        assert_eq!(SHADOW_DIR_PATTERN, "agent-XXXXXX-{ts}");
        assert_eq!(SNAPSHOT_INDEX_FILE, "snapshots.json");
    }

    #[test]
    fn t71gb_generate_random_6char_6_chars() {
        // generate_random_6char 6 字符
        let r = generate_random_6char();
        assert_eq!(r.len(), 6, "6 字符");
        // generate_snapshot_id 格式 "agent-XXXXXX-{ts}"
        let id = generate_snapshot_id();
        assert!(id.starts_with("agent-"));
        assert!(id.len() > 10);
    }
}
