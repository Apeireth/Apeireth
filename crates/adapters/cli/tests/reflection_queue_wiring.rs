//! R8-C 延迟反思队列的**生产接线**真账（`gov.reflection_queue`）。
//!
//! # 这个测在钉什么
//!
//! 接线前：生产装配只注入 gate + 审计 sink ⇒ ⇒ 链用**默认调度器**
//! ⇒ ⇒ 默认调度器只发 token、**永不到期** ⇒ ⇒ **生产零反思、零反思审计记录**。
//! 接线后：开关**开** ⇒ ⇒ 注入 [`DelayedReflectionScheduler`]
//! ⇒ ⇒ 每次成功审计排一个反思项 ⇒ ⇒ **下一轮治理评估**补一条
//! `governance.reflection_due` 审计记录。
//!
//! # 三条断言各钉一件事
//!
//! 1. **开 ⇒ ⇒ 反思真的到期并被审计**（生产路径上可观测的那条记录）；
//! 2. **关 ⇒ ⇒ 零反思审计记录**（默认**零行为变化**的可观测证据）；
//! 3. **开关只认 `"1"`**（与 `APEIRETH_ENABLE_SYSTEM_ONE` 同形）。
//!
//! ⭐ **反证纪律**：把 `adapt_gate_with_scheduler` 的注入去掉（或让
//! `reflection_scheduler_for` 恒返回 `None`）⇒ ⇒ 断言 1 **必须红**
//! ⇒ ⇒ ⇒ 所以断言 1 不是恒真断言。

use std::sync::Arc;

use apeireth_cli::governance_chain::{
    adapt_gate_with, adapt_gate_with_scheduler, reflection_queue_enabled, reflection_scheduler_for,
    ENABLE_REFLECTION_QUEUE_ENV, REFLECTION_QUEUE_DELAY_MS,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, AuditSink, Decision, GovernanceHook, GovernanceRequest, InMemoryAuditSink,
};
use serde_json::Value;

/// 放行型 gate（治理链的 Gate 段）——只关心反思调度，不关心判定语义。
struct AllowAll;

#[async_trait::async_trait]
impl GovernanceHook for AllowAll {
    fn name(&self) -> &str {
        "allow_all"
    }

    async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
        Decision::Allow
    }
}

/// 跑一次真实治理评估（请求在函数内构造 ⇒ ⇒ 借用生命周期天然正确）。
async fn evaluate(hook: &Arc<dyn GovernanceHook>) -> Decision {
    let capability = CapabilityId::new("tool.repo").expect("valid test capability");
    let arguments = Value::Null;
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    hook.evaluate_verbose(&request).await.decision
}

/// 审计账本里 `governance.reflection_due` 的条数。
fn reflection_due_count(sink: &InMemoryAuditSink) -> usize {
    sink.records()
        .iter()
        .filter(|record| record.event_kind == "governance.reflection_due")
        .count()
}

/// ① 开关**开** ⇒ ⇒ 反思在**下一轮**治理评估时到期并被审计。
#[tokio::test]
async fn an_enabled_reflection_queue_produces_a_due_reflection_audit() {
    let sink = Arc::new(InMemoryAuditSink::new());
    let scheduler = reflection_scheduler_for(true).expect("开关开 ⇒ ⇒ 必须有调度器");
    let hook = adapt_gate_with_scheduler(
        Arc::new(AllowAll),
        Arc::clone(&sink) as Arc<dyn AuditSink>,
        Some(scheduler),
    );

    // 第一轮：产生一条审计 ⇒ ⇒ 反思项被**排入队列**（但本轮不到期）。
    let first = evaluate(&hook).await;
    assert!(first.is_allowed(), "{first:?}");
    assert_eq!(
        reflection_due_count(&sink),
        0,
        "第一轮不得有到期反思（延迟队列 ⇒ ⇒ 推迟到下一轮）"
    );

    // 第二轮：**先排空到期项** ⇒ ⇒ 第一轮排的那条反思被审计。
    let second = evaluate(&hook).await;
    assert!(second.is_allowed(), "{second:?}");
    assert_eq!(
        reflection_due_count(&sink),
        1,
        "第二轮必须补一条 governance.reflection_due 审计记录（反思真的到期了）"
    );
}

/// ② 开关**关** ⇒ ⇒ 零反思审计记录（默认**零行为变化**）。
#[tokio::test]
async fn a_disabled_reflection_queue_produces_no_reflection_audit() {
    let sink = Arc::new(InMemoryAuditSink::new());
    assert!(
        reflection_scheduler_for(false).is_none(),
        "开关关 ⇒ ⇒ 不得注入调度器"
    );
    // ⭐ 这就是生产默认走的那条路（`adapt_gate_with` ⇒ ⇒ `None`）。
    let hook = adapt_gate_with(Arc::new(AllowAll), Arc::clone(&sink) as Arc<dyn AuditSink>);

    evaluate(&hook).await;
    evaluate(&hook).await;
    evaluate(&hook).await;

    assert_eq!(
        reflection_due_count(&sink),
        0,
        "默认路径 ⇒ ⇒ 零反思审计记录（零行为变化）"
    );
    // 而普通治理审计**照常**有 ⇒ ⇒ 证明「不是审计面整体没工作」。
    assert!(
        sink.records()
            .iter()
            .any(|record| record.event_kind == "governance.allowed"),
        "普通治理审计必须照常写（否则这条断言是假绿）"
    );
}

/// ③ 开关**只认 `"1"`**（与 `APEIRETH_ENABLE_SYSTEM_ONE` 同形）。
#[test]
fn the_reflection_queue_switch_only_accepts_exactly_one() {
    // 纯函数形态不依赖环境 ⇒ ⇒ 可并行跑。
    for value in [Some("1"), Some(" 1 ")] {
        assert!(reflection_queue_enabled(value), "{value:?} ⇒ ⇒ 应当开");
    }
    for value in [
        None,
        Some(""),
        Some("0"),
        Some("true"),
        Some("yes"),
        Some("on"),
    ] {
        assert!(!reflection_queue_enabled(value), "{value:?} ⇒ ⇒ 应当关");
    }
    assert_eq!(
        ENABLE_REFLECTION_QUEUE_ENV,
        "APEIRETH_ENABLE_REFLECTION_QUEUE"
    );
    assert_eq!(REFLECTION_QUEUE_DELAY_MS, 0);
}
