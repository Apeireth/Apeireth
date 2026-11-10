//! `apeireth-guard::containment` — **收容指令**（护栏拒绝后的**义务**）。
//!
//! # 为什么
//!
//! 审计（W-33，实测见 `docs/status/guard-wiring-audit-2026-10-27.md`）指出：
//! [`crate::enforcement::EnforcementDirective::from_decision`] 已经算出了
//! `revoke_session_grant` / `mark_trace_high_risk` / `compensation_hint`
//! ⇒ 但**护栏 crate 外零消费者**（台账条目 `enforcement.directive_consumer`
//! = `NOT WIRED`）⇒ ⇒ 全仓唯一 `PermissionPolicy::revoke` 是**面板上的人工撤销**
//! ⇒ ⇒ **同会话内可重复越权**（一次高危越权被拒后，会话授权仍在 ⇒ 换个动作
//! 再来一次照样能过）。
//!
//! # 分层
//!
//! * **护栏**（本模块）：**只产指令**（它不知道 CLI / runtime 怎么执行）；
//! * **执行方**（CLI / runtime）：**消费指令** ⇒ 真的撤销授权。
//!
//! ⇒ ⇒ 这样护栏**不依赖**上层 ⇒ 也让「**有没有被执行**」**可机检**
//! （`scripts/check_guard_wiring.py` 的台账会因接线变化而响）。
//!
//! # 本模块的四件东西
//!
//! 1. [`ContainmentDirective`]：**可执行**的收容指令（带会话 id）；
//! 2. [`GrantRevocations`]：**会话授权的撤销状态**（执行方持有的**权威状态**）；
//! 3. [`apply`]：把指令落到状态上（**幂等**）；
//! 4. [`ContainmentGate`] + [`ContainmentConsumer`]：**收容执行闸**
//!    （护栏**发布**义务 ⇒ 执行方**消费** ⇒ 同会话再来一次**仍被拒**）。
//!
//! # 口径只有一处
//!
//! 「多危险才算高危、要不要撤销会话授权」由
//! [`crate::enforcement::EnforcementDirective::from_decision`] 独家决定；
//! 本模块只做**搬运**（补上会话 id）⇒ **不二次实现阈值**（否则两处口径会漂移）。

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use apeireth_governance::{Decision, GovernanceHook, GovernanceRequest, GovernanceVerdict};

use crate::enforcement::EnforcementDirective;
use crate::hook::BehaviorChainGuardHook;
use crate::introspection::GuardEventDto;

/// 一次评估里**回看**多少条判定事件（找护栏发布的收容义务用）。
///
/// # 为什么是「回看」而不是「精确命中」
///
/// 护栏把收容义务**发布**在判定事件里（[`GuardEventDto::containment`]）。
/// 执行面在守门链**外面** ⇒ 它看到的是 verdict，不是判定内部结构 ⇒ 只能按
/// 「会话 + trace」回看匹配的事件。回看窗口取 32 条：足够覆盖一个回合内的
/// 并行兄弟动作，又不会把整段历史读进内存。
const PUBLISHED_EVENT_SCAN: usize = 32;

/// 一条**收容指令**（护栏拒绝后发出 ⇒ 由执行方消费）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainmentDirective {
    /// 要**撤销授权**的会话（`SessionId` 的字符串形式）。
    pub revoke_session_grant: Option<String>,
    /// 把这条 trace 标成**高风险**。
    pub mark_trace_high_risk: bool,
    /// **补偿提示**（如「建议回滚已落盘的部分」）。
    pub compensation_hint: Option<String>,
}

/// **会话授权的撤销状态**（执行方持有的**权威状态**）。
///
/// # 为什么要有这个类型
///
/// 「撤销」若只是**调用一次** `revoke`，则**状态不可查** ⇒
/// 「同会话能否再来一次」就**只能靠跑一遍**才知道 ⇒ **不可机检**。
/// ⇒ 这里给一个**显式状态** ⇒ ⇒ 「撤销后该会话**确实**被挡住」可被**直接断言**。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrantRevocations {
    revoked: BTreeSet<String>,
}

impl GrantRevocations {
    /// 撤销某会话的授权（**幂等**）。
    pub fn revoke(&mut self, session_id: &str) {
        self.revoked.insert(session_id.to_string());
    }

    /// 该会话的授权**是否已被撤销**。
    pub fn is_revoked(&self, session_id: &str) -> bool {
        self.revoked.contains(session_id)
    }

    /// 已撤销的会话数（**可观测**）。
    pub fn len(&self) -> usize {
        self.revoked.len()
    }

    /// 一个都没撤销？
    pub fn is_empty(&self) -> bool {
        self.revoked.is_empty()
    }
}

/// 消费一条收容指令 ⇒ 把撤销落到**状态**上。
///
/// # 口径
///
/// * 只处理 `revoke_session_grant`（**有值才**撤销）；
/// * `mark_trace_high_risk` / `compensation_hint` **不由本函数处理**
///   （它们是**别的执行面**的事）⇒ ⇒ 本函数**返回**指令里需要**别的执行面**处理的部分
///   ⇒ ⇒ 执行方**不会**静默丢掉它们。
pub fn apply<'a>(
    revocations: &mut GrantRevocations,
    directive: &'a ContainmentDirective,
) -> &'a ContainmentDirective {
    if let Some(session_id) = directive.revoke_session_grant.as_deref() {
        revocations.revoke(session_id);
    }
    directive
}

impl ContainmentDirective {
    /// 从护栏算出的**收容义务**（[`EnforcementDirective`]）+ 会话 id
    /// ⇒ **可执行**的收容指令。
    ///
    /// # 为什么要有这一步
    ///
    /// 护栏算出的义务是「**要不要**撤销」（布尔），而**执行**需要知道
    /// 「撤销**哪个**会话」⇒ ⇒ 这一步把会话 id 补上。
    /// 阈值口径（风险分阈值 / 是否拒绝）**只由**
    /// [`EnforcementDirective::from_decision`] 决定 ⇒ 这里**不二次实现**阈值。
    pub fn from_enforcement(obligation: &EnforcementDirective, session_id: &str) -> Self {
        Self {
            revoke_session_grant: obligation
                .revoke_session_grant
                .then(|| session_id.to_string()),
            mark_trace_high_risk: obligation.mark_trace_high_risk,
            compensation_hint: obligation.compensation_hint.clone(),
        }
    }
}

/// **执行方**接口：护栏**发布**收容义务 ⇒ 执行方**消费**它。
///
/// # 为什么是接口而不是自由函数
///
/// 护栏 crate **不该**知道 CLI / runtime 怎么执行 ⇒「撤销会话授权」这件事由
/// **护栏 crate 外**的执行方做 ⇒ 这里只定义**契约**（本仓的执行方在
/// `apeireth-cli` 装配根）。
pub trait ContainmentConsumer: Send + Sync {
    /// 消费一条收容义务（护栏已算好）⇒ 落实它（撤销会话授权）。
    fn consume(&self, obligation: &EnforcementDirective, session_id: &str);

    /// 该会话的授权**是否已被撤销**（执行面据此 fail-closed）。
    fn is_revoked(&self, session_id: &str) -> bool;
}

/// **收容执行闸**：把护栏的拒绝接成「**撤销会话授权**」
/// ⇒ 同会话再来一次**仍被拒**（此前「拒绝了但授权还在」⇒ 可重复越权）。
///
/// # 位置
///
/// 它**包在**守门链**外面**（CLI 装配根把它装在治理链入口与 runtime 之间）。
/// ⇒ **不改动** pipeline 内部的顺序：护栏仍是 pipeline 的**末位**闸
/// （那是审计 P0-2 的事，本批不碰）。
///
/// # 口径
///
/// 1. **先查撤销状态**：会话授权已被撤销 ⇒ 直接拒（fail-closed），理由写明
///    「授权已撤销」⇒ 与「意图只读」这类**判定理由可区分**（审计说得清是谁拒的）；
/// 2. **再评估**：正常走守门链；
/// 3. **后消费**：守门拒绝 ⇒ 消费护栏**发布**的收容义务 ⇒ 执行方撤销该会话授权。
pub struct ContainmentGate {
    inner: Arc<dyn GovernanceHook>,
    guard: Arc<BehaviorChainGuardHook>,
    consumer: Arc<dyn ContainmentConsumer>,
    /// 已消费的收容义务数（**可观测**：义务被执行了几次要能被看见）。
    consumed_obligations: AtomicU64,
}

impl ContainmentGate {
    /// 构造收容执行闸。
    ///
    /// * `inner`：被包的守门链（生产 = 会话权限预设 + 治理 pipeline，护栏在 pipeline 末位）；
    /// * `guard`：护栏 hook —— 收容义务由它**发布**（判定事件），执行面据此消费；
    /// * `consumer`：执行方（撤销会话授权的真实落点）。
    pub fn new(
        inner: Arc<dyn GovernanceHook>,
        guard: Arc<BehaviorChainGuardHook>,
        consumer: Arc<dyn ContainmentConsumer>,
    ) -> Self {
        Self {
            inner,
            guard,
            consumer,
            consumed_obligations: AtomicU64::new(0),
        }
    }

    /// 已消费的收容义务数（**可观测**）。
    pub fn consumed_obligations(&self) -> u64 {
        self.consumed_obligations.load(Ordering::Relaxed)
    }

    /// 消费护栏为本次拒绝**发布**的收容义务 ⇒ 执行方落实（撤销会话授权）。
    ///
    /// 返回是否消费到（没消费到 ⇒ 本次拒绝**没有**护栏收容义务，例如拒绝来自
    /// 上游授权闸 ⇒ 不撤销任何会话）。
    fn consume_published_obligation(&self, request: &GovernanceRequest<'_>) -> bool {
        let session_id = request.session.to_string();
        let trace_id = request.trace.to_string();
        let events = self.guard.recent_events(Some(PUBLISHED_EVENT_SCAN));
        match published_obligation(&events, &session_id, &trace_id) {
            Some(obligation) => {
                self.consumer.consume(&obligation, &session_id);
                self.consumed_obligations.fetch_add(1, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }
}

#[async_trait::async_trait]
impl GovernanceHook for ContainmentGate {
    fn name(&self) -> &str {
        "guard_containment"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        self.evaluate_verbose(request).await.decision
    }

    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        let session_id = request.session.to_string();

        // ① 会话授权已被撤销 ⇒ 直接拒（fail-closed）。
        //    理由写明「授权已撤销」⇒ 与「意图只读」这类判定理由**可区分**。
        if self.consumer.is_revoked(&session_id) {
            return GovernanceVerdict::new(
                self.name(),
                Decision::deny(format!(
                    "session grant revoked by guard containment (session {session_id}): \
                     a previous high-risk denial revoked this session's authorization; \
                     re-authorization is required (revocation is explicit, never auto-restored)"
                )),
            );
        }

        // ② 正常评估（护栏在 pipeline 末位；上游闸的拒绝不产生护栏收容义务）。
        let verdict = self.inner.evaluate_verbose(request).await;

        // ③ 守门拒绝 ⇒ 消费护栏发布的收容义务 ⇒ 执行方撤销该会话授权
        //    ⇒ 同会话再来一次仍被拒（①）。
        if !verdict.is_allowed() {
            self.consume_published_obligation(request);
        }

        verdict
    }
}

/// 从护栏**发布**的判定事件里取出「本会话本 trace」的收容义务。
///
/// # 为什么走事件流
///
/// 护栏已经把义务算出来并**发布**在判定事件里（[`GuardEventDto::containment`]）
/// ⇒ 执行面只**消费**已发布的义务 ⇒ **阈值口径不二次实现**（不复制 0.90）
/// ⇒ 也不需要在护栏 crate 外再跑一遍判定。
///
/// # 口径
///
/// * 只看**同一会话 + 同一 trace** 的**非放行**事件 ⇒ 别的会话的拒绝不牵连本会话；
/// * 同一 trace 内**任一**事件带「撤销会话授权」义务 ⇒ 取它
///   （fail-closed：同一 trace 的兄弟动作若也高危，撤销对象是同一个会话 ⇒ 结论一致）。
pub fn published_obligation(
    events: &[GuardEventDto],
    session_id: &str,
    trace_id: &str,
) -> Option<EnforcementDirective> {
    let mut merged: Option<EnforcementDirective> = None;
    for event in events {
        if event.session_id != session_id || event.trace_id != trace_id {
            continue;
        }
        if event.decision == "allow" {
            continue;
        }
        let Some(obligation) = event.containment.as_ref() else {
            continue;
        };
        merged = Some(match merged {
            None => obligation.clone(),
            Some(existing) => EnforcementDirective {
                block_current_action: existing.block_current_action
                    || obligation.block_current_action,
                require_approval: existing.require_approval || obligation.require_approval,
                revoke_session_grant: existing.revoke_session_grant
                    || obligation.revoke_session_grant,
                mark_trace_high_risk: existing.mark_trace_high_risk
                    || obligation.mark_trace_high_risk,
                compensation_hint: existing
                    .compensation_hint
                    .clone()
                    .or_else(|| obligation.compensation_hint.clone()),
            },
        });
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directive_with(session: Option<&str>) -> ContainmentDirective {
        ContainmentDirective {
            revoke_session_grant: session.map(str::to_string),
            mark_trace_high_risk: true,
            compensation_hint: Some("review_prior_side_effects".to_string()),
        }
    }

    #[test]
    fn revoking_marks_the_session_as_revoked() {
        let mut revocations = GrantRevocations::default();
        assert!(revocations.is_empty());
        assert_eq!(revocations.len(), 0);

        revocations.revoke("session-a");

        assert!(revocations.is_revoked("session-a"));
        assert!(!revocations.is_empty());
        assert_eq!(revocations.len(), 1);
    }

    #[test]
    fn revoking_twice_is_idempotent_and_never_errors() {
        let mut revocations = GrantRevocations::default();
        revocations.revoke("session-a");
        revocations.revoke("session-a");

        assert!(revocations.is_revoked("session-a"));
        assert_eq!(revocations.len(), 1, "重复撤销不得把计数放大");
    }

    #[test]
    fn revocation_does_not_cross_sessions() {
        let mut revocations = GrantRevocations::default();
        revocations.revoke("session-a");

        assert!(!revocations.is_revoked("session-b"));
        assert_eq!(revocations.len(), 1);
    }

    #[test]
    fn apply_revokes_exactly_the_session_named_by_the_directive() {
        let mut revocations = GrantRevocations::default();
        let directive = directive_with(Some("session-a"));

        let pending = apply(&mut revocations, &directive);

        assert!(revocations.is_revoked("session-a"));
        assert!(!revocations.is_revoked("session-b"));
        assert_eq!(pending.revoke_session_grant.as_deref(), Some("session-a"));
    }

    #[test]
    fn apply_without_a_session_revokes_nothing() {
        let mut revocations = GrantRevocations::default();
        let directive = directive_with(None);

        apply(&mut revocations, &directive);

        assert!(revocations.is_empty());
        assert_eq!(revocations.len(), 0);
    }

    #[test]
    fn apply_returns_the_directive_so_other_obligations_are_not_dropped() {
        let mut revocations = GrantRevocations::default();
        let directive = directive_with(Some("session-a"));

        let pending = apply(&mut revocations, &directive);

        assert!(
            pending.mark_trace_high_risk,
            "高风险 trace 标记义务要留给别的执行面"
        );
        assert_eq!(
            pending.compensation_hint.as_deref(),
            Some("review_prior_side_effects"),
            "补偿线索要留给别的执行面"
        );
    }

    #[test]
    fn from_enforcement_maps_the_boolean_obligation_onto_the_session() {
        let obligation = EnforcementDirective {
            block_current_action: true,
            require_approval: false,
            revoke_session_grant: true,
            mark_trace_high_risk: true,
            compensation_hint: Some("review_prior_side_effects".to_string()),
        };

        let directive = ContainmentDirective::from_enforcement(&obligation, "session-a");

        assert_eq!(directive.revoke_session_grant.as_deref(), Some("session-a"));
        assert!(directive.mark_trace_high_risk);
        assert_eq!(
            directive.compensation_hint.as_deref(),
            Some("review_prior_side_effects")
        );
    }

    #[test]
    fn from_enforcement_without_revocation_carries_no_session() {
        let obligation = EnforcementDirective {
            block_current_action: false,
            require_approval: true,
            revoke_session_grant: false,
            mark_trace_high_risk: false,
            compensation_hint: None,
        };

        let directive = ContainmentDirective::from_enforcement(&obligation, "session-a");

        assert!(
            directive.revoke_session_grant.is_none(),
            "不需要撤销时不得凭空带上会话（否则会误伤会话）"
        );
        assert!(!directive.mark_trace_high_risk);
        assert!(directive.compensation_hint.is_none());
    }
}
