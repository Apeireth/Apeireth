//! **P0-2 的接线证据**：上游闸拒掉的动作，护栏**看得见**了。
//!
//! # 这个测试证明什么
//!
//! 2026-10-27 的护栏接线审计（`docs/status/guard-wiring-audit-2026-10-27.md`）记的 **P0-2**：
//!
//! * 治理管线在**第一个非 allow 处短路** ⇒ ⇒ 排在护栏前面的闸拒掉的动作
//!   **根本不会进护栏的行为链**；
//! * 而护栏的**复合检测**（`has_same_effect_after_denial` / `has_retry_escalation`
//!   / `has_destructive_chain`）**依赖链里有被拒动作**
//!   ⇒ ⇒ 「**拒绝后换个工具再来一次**」（同效果绕过）这条护栏能力
//!   对**上游拒绝**是**失明**的。
//!
//! ⇒ 本测试证明**修好了**：`GovernancePipeline` 在短路时**通知**后面的 hook
//! ⇒ ⇒ 护栏把「上游已拒」记进链 ⇒ ⇒ 复合检测**看得见**。
//!
//! # 三条纪律
//!
//! 1. **不改短路语义**：下游 hook **仍然不被评估**（只是**收到通知**）
//!    ⇒ ⇒ 「上游拒绝 ⇒ 下游不得放行」这条**安全属性不变**；
//! 2. **有对照**：不发生上游拒绝时 ⇒ ⇒ 链里**不该**出现被拒动作；
//! 3. **只记事实**：通知**只**记录「被拒」⇒ ⇒ **不改变护栏的判定**。

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernancePipeline, GovernanceRequest, GovernanceVerdict,
};
use apeireth_guard::BehaviorChainGuardHook;
use async_trait::async_trait;

/// 一个**总是拒绝**的上游闸（模拟授权闸 / 提示注入闸 / 限流闸）。
struct AlwaysDenyHook {
    name: &'static str,
}

#[async_trait]
impl GovernanceHook for AlwaysDenyHook {
    fn name(&self) -> &str {
        self.name
    }

    async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
        Decision::Deny {
            reason: "上游闸拒绝（测试用）".to_string(),
        }
    }
}

fn make_dispatch_req<'a>(
    session: SessionId,
    trace: TraceId,
    round: u32,
    cap: &'a CapabilityId,
    args: &'a serde_json::Value,
) -> GovernanceRequest<'a> {
    GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: cap,
            arguments: args,
        },
        session,
        trace,
        round,
    )
}

#[tokio::test]
async fn an_upstream_denial_is_recorded_in_the_guard_chain() {
    // ⇒ 🎯 **核心**：上游拒绝 ⇒ ⇒ 护栏的链里**出现**那条被拒动作。
    let session = SessionId::new();
    let trace = TraceId::new();
    let guard = Arc::new(BehaviorChainGuardHook::new());
    let upstream = AlwaysDenyHook {
        name: "upstream_gate",
    };
    let pipeline = GovernancePipeline::new()
        .with(Arc::new(upstream))
        .with(guard.clone() as Arc<dyn GovernanceHook>);

    let cap = CapabilityId::new("fs.write").unwrap();
    let args = serde_json::json!({ "path": "src/main.rs", "content": "x" });
    let request = make_dispatch_req(session, trace, 1, &cap, &args);

    let verdict = pipeline.evaluate_verbose(&request).await;
    assert!(!verdict.is_allowed(), "上游闸应拒绝");

    let chain = guard
        .chain_for_trace(&session, &trace.to_string())
        .expect("护栏链应存在（上游拒绝已记入）");
    let denied_actions: Vec<_> = chain.actions().into_iter().filter(|a| a.denied).collect();
    assert_eq!(
        denied_actions.len(),
        1,
        "上游拒绝的那条动作应被记进护栏链（实际记了 {} 条）",
        denied_actions.len()
    );
}

#[tokio::test]
async fn without_an_upstream_denial_the_chain_has_no_denied_action() {
    // ⇒ **对照**：没有上游拒绝 ⇒ ⇒ 链里**不该**出现被拒动作。
    let session = SessionId::new();
    let trace = TraceId::new();
    let guard = Arc::new(BehaviorChainGuardHook::new());
    let pipeline = GovernancePipeline::new().with(guard.clone() as Arc<dyn GovernanceHook>);

    let cap = CapabilityId::new("fs.read").unwrap();
    let args = serde_json::json!({ "path": "src/main.rs" });
    let request = make_dispatch_req(session, trace, 1, &cap, &args);

    let verdict = pipeline.evaluate_verbose(&request).await;
    assert!(verdict.is_allowed(), "良性读应放行");

    let chain = guard
        .chain_for_trace(&session, &trace.to_string())
        .expect("护栏链应存在");
    let denied_actions: Vec<_> = chain.actions().into_iter().filter(|a| a.denied).collect();
    assert!(
        denied_actions.is_empty(),
        "没有上游拒绝 ⇒ 不该有被拒动作（实际 {} 条）",
        denied_actions.len()
    );
}

#[tokio::test]
async fn the_short_circuit_semantics_are_unchanged() {
    // ⇒ ⚠️ **安全属性不变**：上游拒绝 ⇒ ⇒ 下游 hook **仍然不被评估**
    // ⇒ ⇒ 只是**收到通知**（`on_denied`）⇒ ⇒ 判定权**没有**被转移。
    struct CountingHook {
        evaluated: Arc<std::sync::atomic::AtomicUsize>,
        notified: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl GovernanceHook for CountingHook {
        fn name(&self) -> &str {
            "downstream_counter"
        }

        async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
            self.evaluated
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Decision::Allow
        }

        async fn on_denied(&self, _request: &GovernanceRequest<'_>, _verdict: &GovernanceVerdict) {
            self.notified
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    let evaluated = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let notified = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let downstream = Arc::new(CountingHook {
        evaluated: evaluated.clone(),
        notified: notified.clone(),
    });
    let pipeline = GovernancePipeline::new()
        .with(Arc::new(AlwaysDenyHook {
            name: "upstream_gate",
        }))
        .with(downstream.clone() as Arc<dyn GovernanceHook>);

    let cap = CapabilityId::new("fs.write").unwrap();
    let args = serde_json::json!({ "path": "src/main.rs" });
    let request = make_dispatch_req(SessionId::new(), TraceId::new(), 1, &cap, &args);
    let verdict = pipeline.evaluate_verbose(&request).await;

    assert!(!verdict.is_allowed(), "上游拒绝 ⇒ 整条链拒绝");
    assert_eq!(
        evaluated.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "下游 hook **不该被评估**（短路语义不变）"
    );
    assert_eq!(
        notified.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "下游 hook **应该收到一次**上游拒绝通知"
    );
}
