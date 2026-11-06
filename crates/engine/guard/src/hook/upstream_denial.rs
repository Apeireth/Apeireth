//! 护栏的**上游拒绝记录**（护栏接线审计 **P0-2**）。
//!
//! # 为什么独立成文件
//!
//! `hook.rs` 有**尺寸棘轮**（只减不增 ⇒ 见 `scripts/arch-baseline.txt`）
//! ⇒ ⇒ 这块逻辑独立成文件 ⇒ ⇒ **主文件不越线**（与 `hook/tests.rs` 同样的做法）。

use apeireth_governance::{GovernanceHook, GovernanceRequest, GovernanceVerdict};
use async_trait::async_trait;

use crate::chain::{ActionStatus, BehaviorChain};
use crate::hook::BehaviorChainGuardHook;
use crate::observation::SafetyObservation;

/// **上游拒绝** ⇒ ⇒ 把它记进行为链（2026-10-27，护栏审计 **P0-2**）。
///
/// # 为什么需要（**后果**）
///
/// 治理管线在**第一个非 allow 处短路** ⇒ ⇒ 护栏（排在后面）**根本不会被评估**
/// ⇒ ⇒ **被上游闸拒掉的动作不进护栏的行为链**
/// ⇒ ⇒ 而护栏的**复合检测**依赖链里有**被拒动作**：
/// * `BehaviorChain::has_same_effect_after_denial`（同效果绕过）
/// * `has_retry_escalation`（重试升级）
/// * `has_destructive_chain`（破坏链）
/// ⇒ ⇒ ⇒ 「**拒绝后换个工具再来一次**」对**上游拒绝**是**失明**的。
///
/// # 口径
///
/// * **只记「被拒」这一个事实** ⇒ ⇒ 不做任何判定 ⇒ ⇒ **不改变护栏的判定结果**
///   （只影响**后续**评估能看到这条历史）；
/// * 与护栏**自己**拒绝时的记法**完全一致**（同一条链、同样的 `denied` 标记）
///   ⇒ ⇒ ⇒ 下游的复合检测**分不出**这是护栏拒的还是上游拒的
///   ⇒ ⇒ ⇒ **这正是要的**（「拒绝过」这件事本身就够了）。
#[async_trait]
impl GovernanceHook for BehaviorChainGuardHook {
    fn name(&self) -> &str {
        "behavior_chain_guard"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> apeireth_governance::Decision {
        let (decision, _) = self.evaluate_internal(request);
        decision.decision
    }

    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        let (_, verdict) = self.evaluate_internal(request);
        verdict
    }

    async fn on_denied(&self, request: &GovernanceRequest<'_>, _verdict: &GovernanceVerdict) {
        let trace_id = request.trace.to_string();
        let key = (request.session, trace_id.clone());
        let mut chains = self.chains.lock();
        let chain = chains
            .entry(key)
            .or_insert_with(|| BehaviorChain::new(request.session.to_string(), trace_id.clone()));
        let actions = chain.actions();
        let denied_before = actions.iter().any(|action| action.denied);
        let retry_count = actions
            .iter()
            .rev()
            .take_while(|action| action.denied)
            .count() as u32;
        let prior_actions = actions
            .iter()
            .map(|action| action.capability_id.clone())
            .collect();
        let observation = SafetyObservation::from_governance_request_with_provider(
            request,
            retry_count,
            denied_before,
            prior_actions,
            Some(self.safety_provider.as_ref()),
        );
        let action_id = chain.add_action_with_id(&observation, request.round, request.action_id);
        chain.update_action_status(&action_id, ActionStatus::Denied);
    }
}

/// **可观测面**：会话态的规模（**运维能看见它有没有涨**）。
impl BehaviorChainGuardHook {
    /// 当前**被跟踪**的会话数。
    pub fn tracked_sessions(&self) -> usize {
        self.session_order.len()
    }

    /// 这个会话**当前是否还被跟踪**（**可验证 LRU 语义**）。
    pub fn tracks_session(&self, session_id: &apeireth_core::kernel::SessionId) -> bool {
        self.session_order.contains(session_id)
    }
}
