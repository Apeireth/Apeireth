//! # `test_main_chain_e2e_full.rs` — C1 跨 crate 主链路 e2e 场景测试 (10 场景·companion 真件版, 待启用)
//!
//! **状态**: PENDING — apeireth-companion 并行 WIP (diary.rs:145 BTreeMap 等)
//! 当前不可编译, 本文件暂存 `tests/pending_companion_wip/` (cargo 不编译此目录).
//! **启用步骤** (companion 编译恢复后): ① 移回 `tests/` ② Cargo.toml dev-deps
//! 恢复 `apeireth-companion = { path = "../apeireth-companion" }` ③ 跑
//! `cargo test -p apeireth-integration-e2e --test test_main_chain_e2e_full`.
//!
//! **任务**: 6dec693a (team-work-doc §4 C1 系统性欠账)
//! **职责**: 端到端验证五段主链路 `记忆 → 注入 → 工具 → 反思 → 送达`
//! + 关键边界条件, 全部跨 crate 真 API 串联, 0 装 PASS.
//! **与基线版关系**: `test_main_chain_e2e.rs` 为不依赖 companion 的基线版
//! (context-fold/历史流机制); 本文件用 companion 真件
//! (build_memory_injection / ReflectionScheduler / MultiSink). 两版并存互补.
//!
//! **场景清单** (覆盖五段主链路):
//! - S1  记忆写入 → 次轮注入可见 (memory → injection)
//! - S2  会话恢复: 文件库关闭重开, episodes 完整恢复 (memory 持久化)
//! - S3  工具调用 → 审批 → 执行 → 审计留痕 (tool 4 件套)
//! - S4  工具黑名单拒绝 → 拒绝审计留痕 (tool 边界)
//! - S5  反思周期触发 → 产出落盘 (VirtualClock 快进, reflection → memory)
//! - S6  反思 importance 积累触发 (反思边界: 周期未到也触发)
//! - S7  反思深度反思器 mock 注入 → 洞察落盘 (mock LLM trait 注入)
//! - S8  多 sink 送达 (MultiSink 扇出 + BroadcastSink)
//! - S9  送达部分失败隔离 (delivery 边界: 1 sink 故障不阻塞其余)
//! - S10 五段全链路串联 (记忆→注入→工具→反思→送达→恢复验证)
//!
//! **确定性保证**: VirtualClock (时间 0 等待) / MockReflector (mock LLM,
//! trait 注入) / RecordingSink (内存录音) / tempdir 文件库 (0 污染生产库).
//! 0 网络 / 0 真实 LLM / 0 sleep 等待.
//!
//! **跑法**: `cargo test -p apeireth-integration-e2e -j 4 --test test_main_chain_e2e_full`

use std::sync::{Arc, Mutex};

use apeireth_companion::daemon::{BroadcastSink, MultiSink, Sink};
use apeireth_companion::memory_injection::build_memory_injection;
use apeireth_companion::reflection::{ReflectionReflector, ReflectionScheduler};
use apeireth_core::clock::VirtualClock;
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_approval::{
    ApprovalDecision, ApprovalManager, AutoApproveHandler, BlacklistRule, WhitelistRule,
};
use apeireth_tool_registry::ToolRegistry;
use apeireth_tool_runtime::{RecordStore, ToolCallParser, ToolExecutor};
use apeireth_tools::register_all;
use chrono::TimeZone;
use tempfile::tempdir;

// ============================================================================
// 测试基础设施 (确定性 mock)
// ============================================================================

/// 固定起点虚拟时钟: 2026-08-16 06:00 UTC.
fn vclock() -> VirtualClock {
    VirtualClock::new(
        chrono::Utc
            .with_ymd_and_hms(2026, 8, 16, 6, 0, 0)
            .single()
            .unwrap(),
    )
}

/// mock 深度反思器 (mock LLM): 返回固定洞察文本, 0 网络 0 随机.
struct MockReflector(String);

#[async_trait::async_trait]
impl ReflectionReflector for MockReflector {
    async fn reflect(&self, _context: &str) -> Result<String, String> {
        Ok(self.0.clone())
    }
}

/// 录音 sink: 内存记录所有送达文本 (断言用).
#[derive(Clone, Default)]
struct RecordingSink(Arc<Mutex<Vec<String>>>);

impl RecordingSink {
    fn messages(&self) -> Vec<String> {
        self.0.lock().expect("recording sink mutex").clone()
    }
}

#[async_trait::async_trait]
impl Sink for RecordingSink {
    async fn send(&self, text: &str) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|e| format!("lock: {e}"))?
            .push(text.to_string());
        Ok(())
    }
}

/// 故障 sink: 永远失败 (边界测试用).
struct FailingSink;

#[async_trait::async_trait]
impl Sink for FailingSink {
    async fn send(&self, _text: &str) -> Result<(), String> {
        Err("模拟通道故障".to_string())
    }
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

// ============================================================================
// S1 记忆写入 → 次轮注入可见 (memory → injection)
// ============================================================================
//
// 链路: SqliteMemoryStore (文件库) 写 3 条 → 关闭重开 (模拟次轮) →
//       recent_episodes 检索 → build_memory_injection 渲染 →
//       断言注入块含编号条目 + 反幻觉闭世界指令 + 120 字截断边界.
#[tokio::test]
async fn test_s1_memory_write_then_injection_visible_next_round() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("s1.db");

    // 第 1 轮: 写入记忆
    {
        let store = SqliteMemoryStore::open(&db_path).unwrap();
        store
            .put_episode(&episode("s1-e1", 1_700_000_000, "user", "主人明天要交线代作业", "sess-1"))
            .unwrap();
        store
            .put_episode(&episode(
                "s1-e2",
                1_700_000_001,
                "assistant",
                "好的, 我记住线代作业这件事了",
                "sess-1",
            ))
            .unwrap();
            // 边界: 超长条目 (>120 字), 注入渲染必须截断
        let long_content = "x".repeat(200);
        store
            .put_episode(&episode("s1-e3", 1_700_000_002, "user", &long_content, "sess-1"))
            .unwrap();
        // drop store = 关闭库 (模拟本轮结束)
    }

    // 第 2 轮: 重开库 → 检索 → 注入
    let store2 = SqliteMemoryStore::open(&db_path).unwrap();
    let recent = store2.recent_episodes("sess-1", 10).unwrap();
    assert_eq!(recent.len(), 3, "次轮应看到上一轮写入的全部 3 条记忆");

    let entries: Vec<String> = recent.iter().map(|e| e.content.clone()).collect();
    let injection = build_memory_injection(&entries);

    assert!(injection.contains("[记忆证据"), "注入块必须有闭世界头");
    assert!(
        injection.contains("主人明天要交线代作业"),
        "注入内容必须可见上一轮记忆: {injection}"
    );
    assert!(injection.contains("1. "), "条目必须编号");
    assert!(
        injection.contains("禁止说「我记得我们以前聊过」"),
        "反幻觉指令必须存在"
    );
    // 边界: 200 字条目被截断到 120 字
    assert!(
        injection.contains(&"x".repeat(120)),
        "截断后前 120 字应在注入里"
    );
    assert!(
        !injection.contains(&"x".repeat(121)),
        "超过 120 字的部分必须被截断"
    );

    // 边界: 空记忆 → 空注入 (不编造)
    assert_eq!(build_memory_injection(&[]), "", "空记忆不应产生注入块");
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
                    1_700_000_000 + (i * 10 + j as usize) as i64,
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
    assert!(recent.windows(2).all(|w| w[0].timestamp <= w[1].timestamp), "恢复后时间升序");

    let q = apeireth_memory::EpisodeQuery::new()
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

    // 3. 审批: 白名单含 FileOperator → Allow
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
//       record_failure 留痕 → 审计可查拒绝原因.
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
// 链路: 文件库 + VirtualClock → 周期未到 tick=0 → 快进 1 天 → tick=1 →
//       反思记录写回真库 → **关闭重开库验证落盘** (不是内存假象).
#[tokio::test]
async fn test_s5_reflection_period_trigger_writes_to_disk() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("s5.db");
    let vc = Arc::new(vclock());

    {
        let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
        let mut sched = ReflectionScheduler::new(
            Arc::clone(&store),
            Arc::clone(&vc) as Arc<dyn apeireth_core::clock::Clock>,
            "did-s5",
        )
        .with_period(chrono::Duration::days(1));

        // 周期未到 → 0 (确定性: 虚拟时钟未动)
        assert_eq!(sched.tick().await, 0, "周期未到不应反思");

        // 快进 1 天 (0 等待)
        vc.advance(chrono::Duration::days(1));
        assert_eq!(sched.tick().await, 1, "周期到应完成 1 轮反思");
        assert_eq!(sched.cycles_completed(), 1);
        // drop store → 关库
    }

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
#[tokio::test]
async fn test_s6_reflection_importance_surge_triggers_before_period() {
    let dir = tempdir().unwrap();
    let store = Arc::new(SqliteMemoryStore::open(dir.path().join("s6.db")).unwrap());
    let vc = Arc::new(vclock());

    // 16 条 importance=10 → 和 160 > 150 阈值
    for i in 0..16u32 {
        store
            .put_episode(&episode(
                &format!("s6-e{i}"),
                1_700_000_000 + i64::from(i),
                "user",
                &format!("【imp:10】重要事件 {i}"),
                "me",
            ))
            .unwrap();
    }

    let mut sched = ReflectionScheduler::new(
        Arc::clone(&store),
        Arc::clone(&vc) as Arc<dyn apeireth_core::clock::Clock>,
        "did-s6",
    )
    .with_period(chrono::Duration::days(365)); // 周期故意拉长, 排除周期触发

    // 时钟未动 (0 快进) → 只能靠 importance 积累触发
    assert_eq!(sched.tick().await, 1, "importance 和 160>150 应触发反思");
    assert_eq!(sched.cycles_completed(), 1);

    let eps = store.recent_episodes("me", 50).unwrap();
    assert!(
        eps.iter().any(|e| e.id.starts_with("reflect-")),
        "importance 触发的反思也应落盘"
    );
}

// ============================================================================
// S7 反思深度反思器 (mock LLM) → 洞察落盘
// ============================================================================
//
// mock LLM 注入点: ReflectionReflector trait. 周期到 → reflector 产出
// 固定洞察 → 落盘内容必须含「深度反思」前缀 + 洞察文本.
#[tokio::test]
async fn test_s7_reflection_with_mock_reflector_persists_insight() {
    let dir = tempdir().unwrap();
    let store = Arc::new(SqliteMemoryStore::open(dir.path().join("s7.db")).unwrap());
    let vc = Arc::new(vclock());

    let mut sched = ReflectionScheduler::new(
        Arc::clone(&store),
        Arc::clone(&vc) as Arc<dyn apeireth_core::clock::Clock>,
        "did-s7",
    )
    .with_period(chrono::Duration::hours(1))
    .with_reflector(Arc::new(MockReflector("洞察: 主人最近压力大, 建议主动关心".into())));

    vc.advance(chrono::Duration::hours(1));
    assert_eq!(sched.tick().await, 1);

    let eps = store.recent_episodes("me", 10).unwrap();
    let refl = eps.iter().find(|e| e.id.starts_with("reflect-")).expect("反思落盘");
    assert!(refl.content.contains("【深度反思】"), "应含深度反思前缀: {}", refl.content);
    assert!(
        refl.content.contains("主人最近压力大"),
        "mock 洞察必须落盘: {}",
        refl.content
    );
}

// ============================================================================
// S8 多 sink 送达 (MultiSink 扇出 + BroadcastSink)
// ============================================================================
//
// 链路: MultiSink 挂 2 RecordingSink + 1 BroadcastSink → 一次 send →
//       全部通道收到同一文本 (扇出真送达, 非单通道).
#[tokio::test]
async fn test_s8_multi_sink_fanout_delivers_to_all() {
    let r1 = RecordingSink::default();
    let r2 = RecordingSink::default();
    let (tx, mut rx) = tokio::sync::broadcast::channel::<String>(16);

    let multi = MultiSink::new()
        .push(Box::new(r1.clone()))
        .push(Box::new(BroadcastSink::new(tx)))
        .push(Box::new(r2.clone()));

    multi.send("你好, 该休息了").await.expect("全成功应 Ok");

    assert_eq!(r1.messages(), vec!["你好, 该休息了"], "sink1 必须收到");
    assert_eq!(r2.messages(), vec!["你好, 该休息了"], "sink2 必须收到");
    let broadcast_msg = rx.recv().await.expect("broadcast 通道必须收到");
    assert_eq!(broadcast_msg, "你好, 该休息了");
}

// ============================================================================
// S9 送达部分失败隔离 (边界: 1 sink 故障不阻塞其余)
// ============================================================================
//
// 边界: 中间 sink 故障 → MultiSink 继续向后续 sink 送达 → 返回 Err
// (如实上报故障), 但健康通道不受影响.
#[tokio::test]
async fn test_s9_sink_failure_isolation() {
    let r1 = RecordingSink::default();
    let r2 = RecordingSink::default();

    let multi = MultiSink::new()
        .push(Box::new(r1.clone()))
        .push(Box::new(FailingSink)) // 中间故障
        .push(Box::new(r2.clone()));

    let res = multi.send("带病送达测试").await;
    assert!(res.is_err(), "有 sink 失败必须如实返回 Err");
    assert!(res.unwrap_err().contains("模拟通道故障"), "错误信息透传");

    // 隔离性: 故障 sink 前后两个健康 sink 均送达
    assert_eq!(r1.messages(), vec!["带病送达测试"], "故障前的 sink 不受影响");
    assert_eq!(r2.messages(), vec!["带病送达测试"], "故障后的 sink 不被跳过");
}

// ============================================================================
// S10 五段全链路串联: 记忆→注入→工具→反思→送达 (+恢复验证)
// ============================================================================
//
// 完整主链路单测试贯穿:
// 1. 记忆: 文件库写 2 条 (含 imp 标记)
// 2. 注入: 检索 → build_memory_injection 闭世界证据块
// 3. 工具: mock LLM 输出含 FileOperator marker → 白名单审批 → 真执行 → 审计
// 4. 反思: VirtualClock 快进 + MockReflector → 洞察落盘
// 5. 送达: 汇总文本经 MultiSink (2 RecordingSink) 送达
// 6. 恢复: 关库重开 → 记忆 + 反思 + 审计全部可查
#[tokio::test]
async fn test_s10_full_main_chain_memory_to_delivery() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("s10.db");
    let vc = Arc::new(vclock());

    // ---------- 段 1: 记忆写入 ----------
    let store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    store
        .put_episode(&episode(
            "s10-e1",
            1_700_000_000,
            "user",
            "【imp:10】主人周五要做项目汇报",
            "me",
        ))
        .unwrap();
    store
        .put_episode(&episode(
            "s10-e2",
            1_700_000_001,
            "user",
            "【imp:10】汇报材料还在草稿箱",
            "me",
        ))
        .unwrap();

    // ---------- 段 2: 次轮注入 ----------
    let recent = store.recent_episodes("me", 10).unwrap();
    let injection = build_memory_injection(
        &recent.iter().map(|e| e.content.clone()).collect::<Vec<_>>(),
    );
    assert!(injection.contains("项目汇报"), "注入必须含记忆证据");

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
        "基于记忆证据 [{first_line}] 我先读草稿.\n<<<[TOOL_REQUEST]>>>\ntool_name:<<<FileOperator>>>\nop:<<<read>>>\npath:<<<{}>>>\n<<<[END_TOOL_REQUEST]>>>",
        first_line = injection.lines().next().unwrap_or(""),
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
    let record_id = record_store
        .record_execution(call, &exec_result, false)
        .await
        .expect("全链路审计写入");
    assert!(record_id.starts_with("tcr-"));

    // ---------- 段 4: 反思 (VirtualClock + mock LLM) ----------
    let mut sched = ReflectionScheduler::new(
        Arc::clone(&store),
        Arc::clone(&vc) as Arc<dyn apeireth_core::clock::Clock>,
        "did-s10",
    )
    .with_period(chrono::Duration::hours(2))
    .with_reflector(Arc::new(MockReflector("洞察: 汇报临近, 应提醒主人整理草稿".into())));
    vc.advance(chrono::Duration::hours(2));
    assert_eq!(sched.tick().await, 1, "全链路反思应触发");

    // ---------- 段 5: 多 sink 送达 ----------
    let r1 = RecordingSink::default();
    let r2 = RecordingSink::default();
    let multi = MultiSink::new()
        .push(Box::new(r1.clone()))
        .push(Box::new(r2.clone()));
    let delivery_text = format!(
        "【主链路汇总】证据: 项目汇报 | 工具: FileOperator success={} | 反思: 第 1 轮完成",
        exec_result.success
    );
    multi.send(&delivery_text).await.expect("全链路送达");
    assert_eq!(r1.messages(), vec![delivery_text.clone()]);
    assert_eq!(r2.messages(), vec![delivery_text]);
    assert!(delivery_text.contains("项目汇报"));

    // ---------- 段 6: 恢复验证 (关库重开) ----------
    drop(store); // 关闭
    let store2 = SqliteMemoryStore::open(&db_path).unwrap();
    assert_eq!(store2.count_by_session("me").unwrap(), 3, "2 条记忆 + 1 条反思均应恢复");
    let restored = store2.recent_episodes("me", 20).unwrap();
    assert!(
        restored.iter().any(|e| e.id.starts_with("reflect-") && e.content.contains("汇报临近")),
        "反思洞察跨重启恢复"
    );
    let audit = RecordStore::new(Arc::new(store2))
        .list_for_tool("FileOperator")
        .unwrap();
    assert_eq!(audit.len(), 1, "工具审计跨重启恢复");
}
