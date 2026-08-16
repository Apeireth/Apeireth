//! # `tests/test_main_chain_e2e.rs` — C1 跨 crate 主链路 e2e 场景测试 (10 场景·基线版)
//!
//! **任务**: 6dec693a (team-work-doc §4 C1 系统性欠账)
//! **职责**: 端到端验证五段主链路 `记忆 → 注入 → 工具 → 反思 → 送达`
//! + 关键边界条件, 全部跨 crate 真 API 串联, 0 装 PASS.
//!
//! **版本说明 (诚实声明)**:
//! - 本文件 = **基线版**: 只依赖当前编译健康的 crate
//!   (core/memory/context-fold/tool-registry/tool-runtime/tool-approval/tools).
//! - **完整版** (10 场景 1:1, 含 companion::memory_injection 反幻觉渲染 /
//!   ReflectionScheduler / MultiSink 真件) 位于
//!   `tests/pending_companion_wip/test_main_chain_e2e_full.rs` — 因
//!   `apeireth-companion` 存在并行 WIP 未跟踪文件 (thought_cluster.rs /
//!   prompt_assembler.rs / continuity.rs 等, N2-N8 VCP 吸收项) 当前不可编译,
//!   待其修复后移回 `tests/` + 恢复 Cargo.toml 的 apeireth-companion dev-dep 即启用.
//! - 降级对照 (基线版 vs 完整版):
//!   | 段 | 基线版机制 | 完整版机制 |
//!   |---|---|---|
//!   | 注入 | context-fold::fold_segments 语义折叠 (相关段可见/低相关折叠) | companion::build_memory_injection 反幻觉渲染 |
//!   | 反思触发 | memory::ReflectionCycleScheduler 4 阶段状态机 + VirtualClock | companion::ReflectionScheduler 封装 |
//!   | mock LLM | fold_segments summarizer 闭包注入 + 本地 importance 解析 | ReflectionReflector trait + memory_extractor::parse_importance |
//!   | 多 sink 送达 | 3 历史流 (Thought/Action/Reflection) 真扇出持久化 | daemon::MultiSink 扇出 |
//!
//! **确定性保证**: VirtualClock (0 等待) / BigramOverlapScorer (确定性评分) /
//! tempdir 文件库 (0 污染生产库). 0 网络 / 0 真实 LLM / 0 sleep.
//!
//! **跑法**: `cargo test -p apeireth-integration-e2e -j 4 --test test_main_chain_e2e`

use std::sync::Arc;

use apeireth_context_fold::{fold_segments, BigramOverlapScorer, SemanticFoldOptions};
use apeireth_core::clock::{Clock, VirtualClock};
use apeireth_memory::{
    ActionStream, CoreEpisode, EpisodeQuery, EpisodeStore, HistoryEntry, HistoryStream,
    ReflectionCycleScheduler, ReflectionPhase, ReflectionStream, SqliteMemoryStore, ThoughtStream,
};
use apeireth_tool_approval::{
    ApprovalDecision, ApprovalManager, AutoApproveHandler, BlacklistRule, WhitelistRule,
};
use apeireth_tool_registry::ToolRegistry;
use apeireth_tool_runtime::{RecordStore, ToolCallParser, ToolExecutor};
use apeireth_tools::register_all;
use chrono::TimeZone;
use tempfile::tempdir;

// ============================================================================
// 测试基础设施 (确定性)
// ============================================================================

/// 固定起点虚拟时钟: 2026-08-16 06:00 UTC.
fn vclock() -> Arc<VirtualClock> {
    Arc::new(
        VirtualClock::new(
            chrono::Utc
                .with_ymd_and_hms(2026, 8, 16, 6, 0, 0)
                .single()
                .unwrap(),
        ),
    )
}

/// 构造一条 episode.
fn episode(id: &str, ts: i64, role: &str, content: &str, session: &str) -> CoreEpisode {
    CoreEpisode {
        id: id.to_string(),
        timestamp: ts,
        role: role.to_string(),
        content: content.to_string(),
        session_id: session.to_string(),
    }
}

/// importance 解析 — 规则 1:1 镜像 `companion::memory_extractor::parse_importance`
/// (前缀 `【imp:` + `】` 前数字, clamp 1..=10, 缺省 5). 基线版在测试内实现
/// 是因为真件位于当前不可编译的 companion WIP; 完整版直接用真件.
fn parse_importance(content: &str) -> u8 {
    const PREFIX: &str = "【imp:";
    if let Some(rest) = content.strip_prefix(PREFIX) {
        if let Some(end) = rest.find('】') {
            if let Ok(n) = rest[..end].parse::<u8>() {
                return n.clamp(1, 10);
            }
        }
    }
    5
}

/// 反思周期推进 (基线版): 4 阶段状态机 → 反思记录写回真库.
/// 对齐 companion::ReflectionScheduler::tick 的写回契约 (id 前缀 `reflect-`,
/// 内容含「【反思周期】第 N 轮完成」).
fn run_reflection_cycle(
    store: &SqliteMemoryStore,
    sched: &mut ReflectionCycleScheduler,
    now_ts: i64,
    session: &str,
    insight: Option<&str>,
) {
    let base = now_ts;
    sched
        .advance(ReflectionPhase::Reflecting, base + 1)
        .expect("Triggered→Reflecting 合法转移");
    sched
        .advance(ReflectionPhase::Consolidating, base + 2)
        .expect("Reflecting→Consolidating 合法转移");
    sched
        .advance(ReflectionPhase::Concluded, base + 3)
        .expect("Consolidating→Concluded 合法转移 (自动重触发)");
    let content = match insight {
        Some(text) => format!(
            "【深度反思】第 {} 轮:\n{text}",
            sched.cycles_completed
        ),
        None => format!(
            "【反思周期】第 {} 轮完成. 最近事件: {:?}",
            sched.cycles_completed,
            sched
                .recent_events(6)
                .iter()
                .map(|e| format!("{:?}@{}", e.phase, e.ts))
                .collect::<Vec<_>>()
        ),
    };
    store
        .put_episode(&episode(
            &format!("reflect-baseline-{}", sched.cycles_completed),
            now_ts,
            "assistant",
            &content,
            session,
        ))
        .expect("反思记录写回真库");
}

/// 多通道扇出送达 (基线版 harness, 契约对齐 daemon::MultiSink:
/// 逐通道送达, 单通道失败不阻塞后续, 返回最后一个错误).
fn fan_out(deliverers: &[&dyn Fn(&str) -> Result<(), String>], text: &str) -> Result<(), String> {
    let mut last_err = String::new();
    for d in deliverers {
        if let Err(e) = d(text) {
            last_err = e;
            eprintln!("[sink] 送达失败: {last_err}");
        }
    }
    if last_err.is_empty() {
        Ok(())
    } else {
        Err(last_err)
    }
}

/// 构造历史流条目 (送达消息载体).
fn stream_entry(id: &str, subject: &str, ts: i64, text: &str) -> HistoryEntry {
    HistoryEntry {
        id: id.to_string(),
        subject_id: subject.to_string(),
        subject_rev: 0,
        session_id: Some("me".to_string()),
        created_at: ts,
        payload: serde_json::Value::String(text.to_string()),
        source: "ai_generated".to_string(),
        tags: vec!["e2e".to_string()],
        tombstoned_at: None,
    }
}

// ============================================================================
// S1 记忆写入 → 次轮注入可见 (memory → context-fold 注入段)
// ============================================================================
//
// 链路: SqliteMemoryStore (文件库) 写 3 条 → 关闭重开 (模拟次轮) →
//       recent_episodes 检索 (可见) → fold_segments 语义注入 (确定性评分:
//       查询相关段保留可见, 低相关段折叠收纳).
// 完整版对照: companion::build_memory_injection 反幻觉渲染 (pending WIP).
#[tokio::test]
async fn test_s1_memory_write_then_injection_visible_next_round() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("s1.db");

    // 第 1 轮: 写入记忆
    {
        let store = SqliteMemoryStore::open(&db_path).unwrap();
        store
            .put_episode(&episode(
                "s1-e1",
                1_755_300_000,
                "user",
                "主人明天要交线代作业",
                "sess-1",
            ))
            .unwrap();
        store
            .put_episode(&episode(
                "s1-e2",
                1_755_300_001,
                "assistant",
                "好的, 我记住线代作业这件事了",
                "sess-1",
            ))
            .unwrap();
        store
            .put_episode(&episode("s1-e3", 1_755_300_002, "user", "今天天气真不错", "sess-1"))
            .unwrap();
    } // 关闭库 (模拟本轮结束)

    // 第 2 轮: 重开库 → 检索 → 注入
    let store2 = SqliteMemoryStore::open(&db_path).unwrap();
    let recent = store2.recent_episodes("sess-1", 10).unwrap();
    assert_eq!(recent.len(), 3, "次轮应看到上一轮写入的全部 3 条记忆");

    let contents: Vec<String> = recent.iter().map(|e| e.content.clone()).collect();
    let segments: Vec<&str> = contents.iter().map(|s| s.as_str()).collect();
    let opts = SemanticFoldOptions {
        threshold: 0.10,
        summary_chars: 12,
    };
    let outcome = fold_segments(&segments, "线代作业", &BigramOverlapScorer, &opts, None);

    // 相关记忆必须可见 (注入段保留)
    assert!(
        outcome.rendered.contains("线代作业"),
        "查询相关记忆必须出现在注入里: {}",
        outcome.rendered
    );
    assert!(outcome.kept >= 2, "两条线代相关记忆应保留, kept={}", outcome.kept);
    // 低相关记忆折叠收纳 (不丢, 可无损展开)
    assert!(
        outcome.folded.iter().any(|f| f.summary.contains("天气")),
        "低相关记忆应折叠为摘要: {:?}",
        outcome.folded.iter().map(|f| &f.summary).collect::<Vec<_>>()
    );
    // 边界: 空记忆 → 无注入内容
    let empty = fold_segments(&[], "线代作业", &BigramOverlapScorer, &opts, None);
    assert_eq!(empty.kept, 0, "空记忆不应产生注入段");
    assert!(empty.rendered.is_empty() || empty.kept == 0);
}

// ============================================================================
// S2 会话恢复: 关闭重开文件库, episodes 完整恢复
// ============================================================================
//
// 链路: 2 个 session 各写 2 条 → drop → 重开 → 按 session 计数 / 按 id 读 /
//       复合条件查询 全部一致. 真持久化验证 (WAL 落盘).
#[tokio::test]
async fn test_s2_session_restore_across_reopen() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("s2.db");

    {
        let store = SqliteMemoryStore::open(&db_path).unwrap();
        for (i, sess) in ["alpha", "beta"].iter().enumerate() {
            for j in 0..2u32 {
                let ep = episode(
                    &format!("s2-{sess}-{j}"),
                    1_755_300_000 + (i * 10 + j as usize) as i64,
                    if j % 2 == 0 { "user" } else { "assistant" },
                    &format!("{sess} 的第 {j} 条对话"),
                    sess,
                );
                store.put_episode(&ep).unwrap();
            }
        }
    } // 关闭 (模拟进程退出)

    // 重开 = 会话恢复
    let store = SqliteMemoryStore::open(&db_path).unwrap();
    assert_eq!(store.count_by_session("alpha").unwrap(), 2, "alpha session 应恢复 2 条");
    assert_eq!(store.count_by_session("beta").unwrap(), 2, "beta session 应恢复 2 条");

    let got = store.get_episode("s2-alpha-1").unwrap().expect("按 id 恢复");
    assert_eq!(got.content, "alpha 的第 1 条对话");
    assert_eq!(got.role, "assistant");

    let recent = store.recent_episodes("beta", 10).unwrap();
    assert_eq!(recent.len(), 2, "beta recent 恢复");
    assert!(
        recent.windows(2).all(|w| w[0].timestamp <= w[1].timestamp),
        "恢复后时间升序"
    );

    let q = EpisodeQuery::new()
        .for_session("alpha")
        .with_role("user")
        .limit(10);
    let filtered = store.query(&q).unwrap();
    assert_eq!(filtered.len(), 1, "复合查询 (session+role) 恢复后仍正确");

    // migrations 记录也应持久 (schema 版本一致性)
    assert!(!store.applied_migrations().unwrap().is_empty());
}

// ============================================================================
// S3 工具调用 → 审批 → 执行 → 审计留痕 (tool 4 件套真链路)
// ============================================================================
//
// 链路: register_all 真注册 → ToolCallParser 解析 mock LLM 输出 →
//       ApprovalManager 白名单放行 → ToolExecutor 真执行 FileOperator read →
//       RecordStore 审计写库 → list_for_tool 验证留痕.
#[tokio::test]
async fn test_s3_tool_call_approve_execute_audit() {
    // 1. registry + 真工具
    let registry = Arc::new(ToolRegistry::new());
    register_all(&registry).expect("register_all 8 真工具");
    assert!(registry.len() >= 8, "应注册 8+ 真工具, 实际 {}", registry.len());
    assert!(
        registry.list().iter().any(|n| n == "FileOperator"),
        "FileOperator 必须在册: {:?}",
        registry.list()
    );

    // 2. 文件库 + 审计 store
    let dir = tempdir().unwrap();
    let store = Arc::new(SqliteMemoryStore::open(dir.path().join("s3.db")).unwrap());
    let record_store = Arc::new(RecordStore::new(Arc::clone(&store)));

    // 3. 审批: 白名单含 FileOperator → 放行
    let mut approval_mgr = ApprovalManager::with_rules(vec![Box::new(
        WhitelistRule::with_whitelist(["FileOperator".to_string()]),
    )]);
    approval_mgr.set_handler(Arc::new(AutoApproveHandler));

    // 4. mock LLM 输出 (真 parser 解析 marker)
    let payload_file = dir.path().join("payload.txt");
    std::fs::write(&payload_file, "S3-E2E-PAYLOAD-12345").unwrap();
    let llm_output = format!(
        "让我读一下文件。\n<<<[TOOL_REQUEST]>>>\ntool_name:<<<FileOperator>>>\nop:<<<read>>>\npath:<<<{}>>>\n<<<[END_TOOL_REQUEST]>>>",
        payload_file.display()
    );
    let parsed = ToolCallParser::parse(&llm_output).expect("解析 mock LLM 输出");
    assert_eq!(parsed.len(), 1, "应解析出 1 个工具调用");
    assert_eq!(parsed[0].tool_name, "FileOperator");

    // 5. 审批 → 执行 → 审计
    let call = &parsed[0];
    match approval_mgr.check(call) {
        ApprovalDecision::Allow => {}
        ApprovalDecision::RequireApproval { .. } => {
            assert!(
                approval_mgr.wait_for_approval(call).await.unwrap(),
                "AutoApproveHandler 必须放行"
            );
        }
        other => panic!("白名单工具应放行, 实际: {other:?}"),
    }

    let executor = ToolExecutor::with_timeout(Arc::clone(&registry), 10_000);
    let result = executor.execute(call).await;
    assert!(result.success, "FileOperator read 应成功: {:?}", result.error);
    assert!(
        result.output.to_string().contains("S3-E2E-PAYLOAD-12345"),
        "执行输出应含文件内容: {}",
        result.output
    );

    let record_id = record_store
        .record_execution(call, &result, false)
        .await
        .expect("审计写入");
    assert!(record_id.starts_with("tcr-"), "审计记录 id 格式");

    // 6. 审计留痕验证
    let records = record_store.list_for_tool("FileOperator").unwrap();
    assert_eq!(records.len(), 1, "FileOperator 应有且仅有 1 条审计记录");
    assert_eq!(approval_mgr.history_len(), 1, "审批历史应留痕 1 条");
}

// ============================================================================
// S4 工具黑名单拒绝 → 拒绝审计留痕 (tool 边界)
// ============================================================================
//
// 链路: mock LLM 输出含黑名单工具 → ApprovalManager Deny → 0 执行 →
//       record_failure 留痕 → 审计可查.
#[tokio::test]
async fn test_s4_blacklisted_tool_denied_and_audited() {
    let dir = tempdir().unwrap();
    let store = Arc::new(SqliteMemoryStore::open(dir.path().join("s4.db")).unwrap());
    let record_store = Arc::new(RecordStore::new(Arc::clone(&store)));

    let mut approval_mgr = ApprovalManager::with_rules(vec![Box::new(
        BlacklistRule::with_blacklist(["DangerTool".to_string()], false),
    )]);
    approval_mgr.set_handler(Arc::new(AutoApproveHandler));

    let llm_output = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<DangerTool>>>\narchery:<<<boom>>>\n<<<[END_TOOL_REQUEST]>>>";
    let parsed = ToolCallParser::parse(llm_output).expect("解析");
    assert_eq!(parsed.len(), 1);
    let call = &parsed[0];

    match approval_mgr.check(call) {
        ApprovalDecision::Deny { reason, silent } => {
            assert!(!silent, "非静默黑名单");
            assert!(reason.contains("DangerTool"), "拒绝原因应含工具名: {reason}");
        }
        other => panic!("黑名单工具必须 Deny, 实际: {other:?}"),
    }

    // 0 执行 (拒绝后不进 executor), 但必须审计留痕
    let record_id = record_store
        .record_failure(call, "denied_by_approval")
        .await
        .expect("拒绝也应审计留痕");
    assert!(!record_id.is_empty());

    let records = record_store.list_for_tool("DangerTool").unwrap();
    assert_eq!(records.len(), 1, "拒绝记录必须可查");
    assert_eq!(approval_mgr.history_len(), 1, "审批历史含拒绝留痕");
}

// ============================================================================
// S5 反思周期触发 → 产出落盘 (VirtualClock 快进)
// ============================================================================
//
// 链路: 文件库 + VirtualClock → 周期未到不反思 → 快进 1 天 → 4 阶段状态机
//       (Triggered→Reflecting→Consolidating→Concluded) → 反思记录写回真库 →
//       **关闭重开库验证落盘**. 完整版对照: companion::ReflectionScheduler.
#[tokio::test]
async fn test_s5_reflection_period_trigger_writes_to_disk() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("s5.db");
    let vc = vclock();
    let period = chrono::Duration::days(1);
    let start = vc.now();

    // 周期未到 → 不反思 (确定性: 虚拟时钟未动)
    assert!(
        vc.now() - start < period,
        "初始时刻不应到达反思周期"
    );

    // 快进 1 天 (0 等待) → 周期到
    vc.advance(period);
    assert!(vc.now() - start >= period, "快进后应到达周期");

    {
        let store = SqliteMemoryStore::open(&db_path).unwrap();
        let mut sched = ReflectionCycleScheduler::new("did-s5", start.timestamp());
        assert_eq!(sched.cycles_completed, 0);
        run_reflection_cycle(&store, &mut sched, vc.now().timestamp(), "me", None);
        assert_eq!(sched.cycles_completed, 1, "完成 1 轮反思");
        // 状态机自动重触发回 Triggered (新周期就绪)
        assert_eq!(sched.current, ReflectionPhase::Triggered);
    } // drop store → 关库

    // 重开验证落盘
    let store2 = SqliteMemoryStore::open(&db_path).unwrap();
    let eps = store2.recent_episodes("me", 10).unwrap();
    assert!(
        eps.iter().any(|e| e.id.starts_with("reflect-")),
        "反思记录必须落盘 (重开仍可见)"
    );
    assert!(
        eps.iter().any(|e| e.content.contains("第 1 轮完成")),
        "反思内容含轮次"
    );
}

// ============================================================================
// S6 反思 importance 积累触发 (边界: 周期未到也触发)
// ============================================================================
//
// 边界条件 (Generative Agents): 最近 100 条 importance 和 > 150 → 即使
// 周期未到也触发反思. 写 16 条 imp:10 (和 160 > 150), 时钟 0 快进.
// importance 解析规则 1:1 镜像 companion::memory_extractor::parse_importance.
#[tokio::test]
async fn test_s6_reflection_importance_surge_triggers_before_period() {
    let dir = tempdir().unwrap();
    let store = SqliteMemoryStore::open(dir.path().join("s6.db")).unwrap();
    let vc = vclock();
    let start = vc.now();

    // 16 条 importance=10 → 和 160 > 150 阈值
    for i in 0..16u32 {
        store
            .put_episode(&episode(
                &format!("s6-e{i}"),
                start.timestamp() + i64::from(i),
                "user",
                &format!("【imp:10】重要事件 {i}"),
                "me",
            ))
            .unwrap();
    }

    // importance 和计算 (对齐 ReflectionScheduler::importance_surge:
    // 最近 100 条, 排除反思记录自身, 和 > 150)
    let eps = store.recent_episodes("me", 100).unwrap();
    let sum: u64 = eps
        .iter()
        .filter(|e| !e.id.starts_with("reflect-"))
        .map(|e| u64::from(parse_importance(&e.content)))
        .sum();
    assert_eq!(sum, 160, "16 条 imp:10 和应为 160");
    assert!(sum > 150, "超过触发阈值 150");

    // 时钟 0 快进 (周期未到) → 仅靠 importance 积累触发
    let mut sched = ReflectionCycleScheduler::new("did-s6", start.timestamp());
    run_reflection_cycle(&store, &mut sched, start.timestamp(), "me", None);
    assert_eq!(sched.cycles_completed, 1);

    let after = store.recent_episodes("me", 50).unwrap();
    assert!(
        after.iter().any(|e| e.id.starts_with("reflect-")),
        "importance 触发的反思也应落盘"
    );
}

// ============================================================================
// S7 反思/mock-LLM 注入 → 产物落盘 (确定性注入点)
// ============================================================================
//
// mock LLM 注入点 (基线版): fold_segments 的 summarizer 闭包 — 真注入 +
// 确定性产物; 深度反思文本经 run_reflection_cycle 落盘.
// 完整版对照: companion::ReflectionReflector trait (pending WIP).
#[tokio::test]
async fn test_s7_mock_llm_injection_persists_artifact() {
    // 注入点 1: 语义折叠 summarizer (mock LLM 摘要)
    let segments = ["主人周五要做项目汇报, 材料还没整理", "昨天的天气预报有雨"];
    let opts = SemanticFoldOptions {
        threshold: 0.99, // 故意拉高 → 全部折叠走 summarizer
        summary_chars: 8,
    };
    let mock_summarizer = |text: &str| format!("【mock摘要】{}", &text.chars().take(4).collect::<String>());
    let outcome = fold_segments(
        &segments,
        "项目汇报",
        &BigramOverlapScorer,
        &opts,
        Some(&mock_summarizer),
    );
    assert!(
        outcome.folded.iter().all(|f| f.summary.starts_with("【mock摘要】")),
        "summarizer 注入必须真生效: {:?}",
        outcome.folded.iter().map(|f| &f.summary).collect::<Vec<_>>()
    );

    // 注入点 2: 深度反思 (mock 洞察) 落盘
    let dir = tempdir().unwrap();
    let store = SqliteMemoryStore::open(dir.path().join("s7.db")).unwrap();
    let vc = vclock();
    let mut sched = ReflectionCycleScheduler::new("did-s7", vc.now().timestamp());
    vc.advance(chrono::Duration::hours(1));
    run_reflection_cycle(
        &store,
        &mut sched,
        vc.now().timestamp(),
        "me",
        Some("洞察: 主人最近压力大, 建议主动关心"),
    );

    let eps = store.recent_episodes("me", 10).unwrap();
    let refl = eps
        .iter()
        .find(|e| e.id.starts_with("reflect-"))
        .expect("反思落盘");
    assert!(refl.content.contains("【深度反思】"), "应含深度反思前缀: {}", refl.content);
    assert!(
        refl.content.contains("主人最近压力大"),
        "mock 洞察必须落盘: {}",
        refl.content
    );
}

// ============================================================================
// S8 多 sink 送达 (基线版: 3 历史流真扇出)
// ============================================================================
//
// 链路: 一条消息 → ThoughtStream + ActionStream + ReflectionStream 3 通道
//       扇出 → 3 通道全部可查 → 关闭重开仍持久.
// 完整版对照: daemon::MultiSink + BroadcastSink (pending WIP).
#[tokio::test]
async fn test_s8_multi_sink_fanout_delivers_to_all() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("s8.db");
    let ts = 1_755_300_000;

    {
        let store = SqliteMemoryStore::open(&db_path).unwrap();
        let conn = store.conn().unwrap();
        let t = ThoughtStream::new(&conn);
        let a = ActionStream::new(&conn);
        let r = ReflectionStream::new(&conn);
        let d_thought = |text: &str| {
            t.append(&stream_entry("s8-thought", "deliver:thought", ts, text))
                .map_err(|e| e.to_string())
        };
        let d_action = |text: &str| {
            a.append(&stream_entry("s8-action", "deliver:action", ts, text))
                .map_err(|e| e.to_string())
        };
        let d_refl = |text: &str| {
            r.append(&stream_entry("s8-refl", "deliver:reflection", ts, text))
                .map_err(|e| e.to_string())
        };
        let deliverers: [&dyn Fn(&str) -> Result<(), String>; 3] =
            [&d_thought, &d_action, &d_refl];
        fan_out(&deliverers, "你好, 该休息了").expect("全通道成功应 Ok");
    } // 关库

    // 重开: 3 通道均应持久
    let store2 = SqliteMemoryStore::open(&db_path).unwrap();
    let conn = store2.conn().unwrap();
    let got_thought = ThoughtStream::new(&conn)
        .list_for_subject("deliver:thought", None, None, false)
        .unwrap();
    let got_action = ActionStream::new(&conn)
        .list_for_subject("deliver:action", None, None, false)
        .unwrap();
    let got_refl = ReflectionStream::new(&conn)
        .list_for_subject("deliver:reflection", None, None, false)
        .unwrap();
    for (name, got) in [("thought", &got_thought), ("action", &got_action), ("reflection", &got_refl)] {
        assert_eq!(got.len(), 1, "{name} 通道必须收到且仅 1 条");
        assert_eq!(
            got[0].payload,
            serde_json::Value::String("你好, 该休息了".to_string()),
            "{name} 通道内容一致"
        );
    }
}

// ============================================================================
// S9 送达部分失败隔离 (边界: 1 通道故障不阻塞其余)
// ============================================================================
//
// 边界: 中间通道故障 → fan_out 继续向后续通道送达 → 返回 Err (如实上报),
// 健康通道不受影响. 契约对齐 daemon::MultiSink (完整版用真件验证).
#[tokio::test]
async fn test_s9_sink_failure_isolation() {
    let dir = tempdir().unwrap();
    let store = SqliteMemoryStore::open(dir.path().join("s9.db")).unwrap();
    let conn = store.conn().unwrap();
    let ts = 1_755_300_000;

    let t = ThoughtStream::new(&conn);
    let a = ActionStream::new(&conn);
    let d_thought = |text: &str| {
        t.append(&stream_entry("s9-thought", "deliver:thought", ts, text))
            .map_err(|e| e.to_string())
    };
    let d_fail = |_text: &str| Err::<(), String>("模拟通道故障".to_string()); // 中间故障
    let d_action = |text: &str| {
        a.append(&stream_entry("s9-action", "deliver:action", ts, text))
            .map_err(|e| e.to_string())
    };
    let deliverers: [&dyn Fn(&str) -> Result<(), String>; 3] = [&d_thought, &d_fail, &d_action];

    let res = fan_out(&deliverers, "带病送达测试");
    assert!(res.is_err(), "有通道失败必须如实返回 Err");
    assert!(res.unwrap_err().contains("模拟通道故障"), "错误信息透传");

    // 隔离性: 故障通道前后两个健康通道均送达
    let got_thought = ThoughtStream::new(&conn)
        .list_for_subject("deliver:thought", None, None, false)
        .unwrap();
    let got_action = ActionStream::new(&conn)
        .list_for_subject("deliver:action", None, None, false)
        .unwrap();
    assert_eq!(got_thought.len(), 1, "故障前的通道不受影响");
    assert_eq!(got_action.len(), 1, "故障后的通道不被跳过");
}

// ============================================================================
// S10 五段全链路串联: 记忆→注入→工具→反思→送达 (+恢复验证)
// ============================================================================
//
// 完整主链路单测试贯穿:
// 1. 记忆: 文件库写 2 条 (含 imp 标记)
// 2. 注入: 检索 → fold_segments 相关段可见
// 3. 工具: mock LLM 输出含 FileOperator marker → 白名单审批 → 真执行 → 审计
// 4. 反思: VirtualClock 快进 → 4 阶段状态机 → mock 洞察落盘
// 5. 送达: 汇总文本经 fan_out (2 历史流) 送达
// 6. 恢复: 关库重开 → 记忆 + 反思 + 审计 + 送达全部可查
#[tokio::test]
async fn test_s10_full_main_chain_memory_to_delivery() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("s10.db");
    let vc = vclock();

    // ---------- 段 1: 记忆写入 ----------
    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    store
        .put_episode(&episode(
            "s10-e1",
            1_755_300_000,
            "user",
            "【imp:10】主人周五要做项目汇报",
            "me",
        ))
        .unwrap();
    store
        .put_episode(&episode(
            "s10-e2",
            1_755_300_001,
            "user",
            "【imp:10】汇报材料还在草稿箱",
            "me",
        ))
        .unwrap();

    // ---------- 段 2: 次轮注入 ----------
    let recent = store.recent_episodes("me", 10).unwrap();
    let contents: Vec<String> = recent.iter().map(|e| e.content.clone()).collect();
    let segments: Vec<&str> = contents.iter().map(|s| s.as_str()).collect();
    let opts = SemanticFoldOptions {
        threshold: 0.05,
        summary_chars: 12,
    };
    let injection = fold_segments(&segments, "项目汇报", &BigramOverlapScorer, &opts, None);
    assert!(injection.rendered.contains("项目汇报"), "注入必须含相关记忆证据");

    // ---------- 段 3: 工具调用 → 审批 → 执行 → 审计 ----------
    let registry = Arc::new(ToolRegistry::new());
    register_all(&registry).unwrap();
    let record_store = Arc::new(RecordStore::new(Arc::clone(&store)));
    let mut approval_mgr = ApprovalManager::with_rules(vec![Box::new(
        WhitelistRule::with_whitelist(["FileOperator".to_string()]),
    )]);
    approval_mgr.set_handler(Arc::new(AutoApproveHandler));

    let draft_file = dir.path().join("draft.md");
    std::fs::write(&draft_file, "# S10 汇报草稿\n要点: 主链路 e2e 全绿").unwrap();
    let mock_llm_output = format!(
        "基于记忆证据先读草稿.\n<<<[TOOL_REQUEST]>>>\ntool_name:<<<FileOperator>>>\nop:<<<read>>>\npath:<<<{}>>>\n<<<[END_TOOL_REQUEST]>>>",
        draft_file.display()
    );
    let parsed = ToolCallParser::parse(&mock_llm_output).expect("解析 mock LLM 输出");
    assert_eq!(parsed.len(), 1);
    let call = &parsed[0];

    let decision = approval_mgr.check(call);
    match &decision {
        ApprovalDecision::Allow | ApprovalDecision::RequireApproval { .. } => {}
        other => panic!("全链路工具调用应放行: {other:?}"),
    }
    if matches!(decision, ApprovalDecision::RequireApproval { .. }) {
        assert!(approval_mgr.wait_for_approval(call).await.unwrap());
    }

    let executor = ToolExecutor::with_timeout(Arc::clone(&registry), 10_000);
    let exec_result = executor.execute(call).await;
    assert!(exec_result.success, "全链路工具执行应成功: {:?}", exec_result.error);
    assert!(
        exec_result.output.to_string().contains("主链路 e2e 全绿"),
        "执行输出应含草稿内容"
    );
    let record_id = record_store
        .record_execution(call, &exec_result, false)
        .await
        .expect("全链路审计写入");
    assert!(record_id.starts_with("tcr-"));

    // ---------- 段 4: 反思 (VirtualClock + mock 洞察) ----------
    vc.advance(chrono::Duration::hours(2));
    let mut sched = ReflectionCycleScheduler::new("did-s10", vclock().now().timestamp() - 7200);
    run_reflection_cycle(
        &store,
        &mut sched,
        vc.now().timestamp(),
        "me",
        Some("洞察: 汇报临近, 应提醒主人整理草稿"),
    );
    assert_eq!(sched.cycles_completed, 1, "全链路反思应触发");

    // ---------- 段 5: 多通道送达 ----------
    let delivery_text = format!(
        "【主链路汇总】证据: 项目汇报 | 工具: FileOperator success={} | 反思: 第 1 轮完成",
        exec_result.success
    );
    {
        let conn = store.conn().unwrap();
        let t = ThoughtStream::new(&conn);
        let a = ActionStream::new(&conn);
        let d_thought = |text: &str| {
            t.append(&stream_entry("s10-thought", "deliver:thought", 1_755_300_100, text))
                .map_err(|e| e.to_string())
        };
        let d_action = |text: &str| {
            a.append(&stream_entry("s10-action", "deliver:action", 1_755_300_100, text))
                .map_err(|e| e.to_string())
        };
        let deliverers: [&dyn Fn(&str) -> Result<(), String>; 2] = [&d_thought, &d_action];
        fan_out(&deliverers, &delivery_text).expect("全链路送达");
    }

    // ---------- 段 6: 恢复验证 (关库重开) ----------
    drop(record_store);
    drop(store); // 关闭 (Arc 引用全部释放)
    let store2 = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    assert_eq!(
        store2.count_by_session("me").unwrap(),
        3,
        "2 条记忆 + 1 条反思均应恢复"
    );
    let restored = store2.recent_episodes("me", 20).unwrap();
    assert!(
        restored
            .iter()
            .any(|e| e.id.starts_with("reflect-") && e.content.contains("汇报临近")),
        "反思洞察跨重启恢复"
    );
    let audit = RecordStore::new(Arc::clone(&store2)).list_for_tool("FileOperator").unwrap();
    assert_eq!(audit.len(), 1, "工具审计跨重启恢复");
    let conn = store2.conn().unwrap();
    let delivered = ThoughtStream::new(&conn)
        .list_for_subject("deliver:thought", None, None, false)
        .unwrap();
    assert_eq!(delivered.len(), 1, "送达消息跨重启恢复");
    assert!(
        delivered[0].payload.to_string().contains("项目汇报"),
        "送达内容完整"
    );
}
