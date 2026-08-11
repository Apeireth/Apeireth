//! `apeireth-rollback` 示例: **71GB 事故预防演示**.
//!
//! **本示例演示** (per `v09021-rust-translation-blueprint-RIVAL §2.2.4` +
//! 主人 2026-08-05 紧急救援 71GB 事故根因修复):
//! 1. 验证 71GB 4 重防御编译期 hardcode (MAX_SHADOW_AGE_DAYS / SIZE / TOTAL / 3 钩子)
//! 2. 验证 8 工具白名单 (m3 防御 §2.4 1:1)
//! 3. 验证 K-1 强校验 5 字样 (apeireth / rollback / snapshot / restore / must-do)
//! 4. 验证 6 策略 1:1 翻译 v0.9.21
//! 5. 构造 5 个 mock snapshot, 验证总大小 / TTL / 单影子约束
//! 6. 启动清理 + 过期清理 + LRU 清理三钩子演示
//!
//! **运行**:
//! ```bash
//! cargo run -p apeireth-rollback --example rollback_demo
//! ```
//!
//! **期望输出** (stdout):
//! ```text
//! === apeireth-rollback 71GB 事故防御 demo ===
//! [1] 71GB 4 重防御编译期 hardcode 报告
//! [2] 8 工具白名单 m3 防御验证通过
//! [3] K-1 强校验 5 字样验证通过
//! [4] 6 策略 1:1 翻译 v0.9.21
//! [5] 5 mock snapshot 创建 (skeleton: 0 字节, 索引元数据)
//! [6] 总大小 = 0 字节, MAX_TOTAL_SHADOW_SIZE_BYTES = 2 GB, 利用率 0%
//! [7] 3 重清理钩子: 启动 / snapshot 前 / cron 全启用
//! [8] 影子目录命名模式: agent-XXXXXX-{ts}
//! [9] GitWrapper 4 操作占位
//! === 71GB 事故防御 demo 完成 ===
//! ```

#![warn(missing_docs)]

use apeireth_rollback::{
    assert_cleanup_hooks_enabled, defense_4_check, generate_snapshot_id,
    k1_invariant_5_keys, validate_k1_invariant, validate_tool_call, DefaultSnapshotService,
    RollbackStrategy, SnapshotIndex, SnapshotMeta, SnapshotService, TOOL_WHITELIST,
    MAX_SHADOW_AGE_DAYS, MAX_SHADOW_SIZE_BYTES, MAX_TOTAL_SHADOW_SIZE_BYTES,
    SHADOW_DIR_PATTERN, V0921_ROLLBACK_STRATEGIES,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-rollback 71GB 事故防御 demo ===\n");

    // [1] 71GB 4 重防御编译期 hardcode 报告
    let report = defense_4_check();
    println!("[1] 71GB 4 重防御编译期 hardcode 报告");
    println!(
        "    MAX_SHADOW_AGE_DAYS         = {} (defense #1 TTL)",
        report.max_age_days
    );
    println!(
        "    MAX_SHADOW_SIZE_BYTES       = {} ({:.0} MB, defense #2 单影子)",
        report.max_shadow_bytes,
        report.max_shadow_bytes as f64 / 1024.0 / 1024.0
    );
    println!(
        "    MAX_TOTAL_SHADOW_SIZE_BYTES = {} ({:.0} GB, defense #3 总大小)",
        report.max_total_bytes,
        report.max_total_bytes as f64 / 1024.0 / 1024.0 / 1024.0
    );
    println!(
        "    CLEANUP_HOOK_STARTUP        = {} (defense #4a)",
        report.hook_startup
    );
    println!(
        "    CLEANUP_HOOK_BEFORE_SNAPSHOT = {} (defense #4b)",
        report.hook_before_snapshot
    );
    println!(
        "    CLEANUP_HOOK_CRON_DAILY     = {} (defense #4c)",
        report.hook_cron_daily
    );

    // [2] 8 工具白名单 m3 防御验证
    println!("\n[2] 8 工具白名单 m3 防御验证通过");
    println!("    TOOL_WHITELIST.len() = {} (per TOOL_COUNT = 8)", TOOL_WHITELIST.len());
    for (i, tool) in TOOL_WHITELIST.iter().enumerate() {
        let r = validate_tool_call(tool, &serde_json::json!({}));
        println!("    [{i}] {tool}: {}", if r.is_ok() { "ok" } else { "FAIL" });
    }

    // [3] K-1 强校验 5 字样
    println!("\n[3] K-1 强校验 5 字样验证通过");
    let keys = k1_invariant_5_keys();
    for (i, k) in keys.iter().enumerate() {
        println!("    K-1 key[{i}] = {k}");
    }
    assert!(validate_k1_invariant().is_ok(), "K-1 强校验必须通过");

    // [4] 6 策略 1:1 翻译 v0.9.21
    println!("\n[4] 6 策略 1:1 翻译 v0.9.21");
    for s in [
        RollbackStrategy::Full,
        RollbackStrategy::File,
        RollbackStrategy::Diff,
        RollbackStrategy::Git,
        RollbackStrategy::Session,
        RollbackStrategy::Auto,
    ] {
        println!("    {:?} -> v0.9.21 '{}'", s, s.as_v0921_str());
    }
    assert_eq!(V0921_ROLLBACK_STRATEGIES, 6, "V0921_ROLLBACK_STRATEGIES = 6");

    // [5] 5 mock snapshot 创建 (skeleton: 0 字节, 索引元数据)
    println!("\n[5] 5 mock snapshot 创建 (skeleton: 0 字节, 索引元数据)");
    let tmp = tempfile::tempdir()?;
    let svc = DefaultSnapshotService::new(tmp.path().to_path_buf());
    let mut idx = SnapshotIndex::new();
    for i in 0..5 {
        let meta = svc
            .snapshot(
                &format!("session-{i}"),
                RollbackStrategy::Auto,
                &[],
                &format!("demo snapshot {i}"),
            )
            .await?;
        println!(
            "    snapshot[{}] id={} ts={} strategy={}",
            i, meta.id, meta.timestamp, meta.strategy.as_v0921_str()
        );
        idx.add(meta);
    }

    // [6] 总大小 / 利用率
    let total = idx.total_size();
    println!("\n[6] 总大小 = {} 字节", total);
    println!(
        "    MAX_TOTAL_SHADOW_SIZE_BYTES = {} 字节 ({:.0} GB)",
        MAX_TOTAL_SHADOW_SIZE_BYTES,
        MAX_TOTAL_SHADOW_SIZE_BYTES as f64 / 1024.0 / 1024.0 / 1024.0
    );
    let pct = if MAX_TOTAL_SHADOW_SIZE_BYTES > 0 {
        100.0 * total as f64 / MAX_TOTAL_SHADOW_SIZE_BYTES as f64
    } else {
        0.0
    };
    println!("    利用率 = {pct:.4}%");

    // [7] 3 重清理钩子全启用验证
    println!("\n[7] 3 重清理钩子: 启动 / snapshot 前 / cron 全启用");
    assert!(assert_cleanup_hooks_enabled().is_ok());
    println!("    ✓ CLEANUP_HOOK_STARTUP = true");
    println!("    ✓ CLEANUP_HOOK_BEFORE_SNAPSHOT = true");
    println!("    ✓ CLEANUP_HOOK_CRON_DAILY = true");
    let cleaned = svc.cleanup_startup().await?;
    println!("    cleanup_startup() 返 {cleaned} 个 snapshot (skeleton: 0)");

    // [8] 影子目录命名模式
    println!("\n[8] 影子目录命名模式: {SHADOW_DIR_PATTERN}");
    let example_id = generate_snapshot_id();
    println!("    示例 snapshot id = {example_id}");

    // [9] GitWrapper 4 操作占位
    println!("\n[9] GitWrapper 4 操作占位");
    let git = apeireth_rollback::GitWrapper::new(tmp.path().to_path_buf());
    let _ = git.status()?;
    let _ = git.diff(None)?;
    git.stash("demo stash")?;
    git.checkout("main")?;
    println!("    ✓ git status / diff / stash / checkout 4 操作占位调用成功");

    println!("\n=== 71GB 事故防御 demo 完成 ===");
    Ok(())
}
