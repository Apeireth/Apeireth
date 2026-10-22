//! R1b-2c · 审批社交信号源（生产面，默认关零发布）。
//!
//! 生产链：CLI 审批决定 → [`BioApprovalSocialSource::publish`] → [`NervousBus`]
//! 的 [`NervousSignalKind::ApprovalSocial`](crate::canonical::nervous_bus::NervousSignalKind)
//! 信号 → 调质订阅者（`bio_tone.rs`）映射为 `SocialPositive` / `SocialNegative`。
//!
//! 设计边界（0 装口径）：
//! - **只发布事实**：`ApprovalDecision::Approve` → `Approved`；`Reject` → `Rejected`；
//!   `Cancel` 是人收回请求、**未形成批/拒结论** → 不发布（不编事件）。
//! - **不带理由文本**：拒绝理由留在审批面审计记录里，不进信号（避免 PII 与
//!   二次真源）；信号只带三态 + 来源标签。
//! - **默认关零变化**：装配根只在调质开关开时才交出本句柄（
//!   `ProductionModules::bio_approval_social_source` 返回 `None`），因此开关关
//!   时审批路径不发布、总线计数为零，行为与未接线逐字节一致。
//! - **不做订阅/状态推进**：订阅在 `bio_tone.rs`，状态只在 `ToneState`；本模块
//!   不读时钟、不落盘、不做审批判定。

use apeireth_runtime::canonical::ApprovalDecision;

use crate::canonical::nervous_bus::{ApprovalSocialOutcome, NervousBus, NervousSignal};

/// 审批决定来源标签（信号归因用，稳定短词表；不含会话/审批 id）。
pub const APPROVAL_SOCIAL_SOURCE: &str = "cli.approval";

/// 审批决定 → 社交三态事实。
///
/// `Cancel` = 人收回请求，不构成社会结论，返回 `None`（不发布、不编事件）。
#[must_use]
pub fn approval_social_outcome(decision: &ApprovalDecision) -> Option<ApprovalSocialOutcome> {
    match decision {
        ApprovalDecision::Approve => Some(ApprovalSocialOutcome::Approved),
        ApprovalDecision::Reject { .. } => Some(ApprovalSocialOutcome::Rejected),
        ApprovalDecision::Cancel { .. } => None,
    }
}

/// 审批社交信号源：把一次真实人工决定发布到总线。
///
/// 句柄由装配根在调质开关开时构造并交给适配器；开关关时适配器拿 `None`，
/// 本类型不参与任何路径。
#[derive(Debug, Clone)]
pub struct BioApprovalSocialSource {
    bus: NervousBus,
}

impl BioApprovalSocialSource {
    /// 绑定到一条总线（同一装配根的 `ProductionModules::nervous_bus`）。
    #[must_use]
    pub fn new(bus: NervousBus) -> Self {
        Self { bus }
    }

    /// 总线上游句柄（只读；订阅者由机制侧注册，本模块不注册）。
    #[must_use]
    pub fn bus(&self) -> &NervousBus {
        &self.bus
    }

    /// 发布一次审批决定信号；返回本次投递数（`Cancel` 不发布 → `None`）。
    ///
    /// `None` 与"发布但零订阅者"是两件事：前者=未形成社会结论（不编事件），
    /// 后者=已发布事实但当前无订阅者（投递数 0，计数仍记账）。
    pub fn publish(&self, decision: &ApprovalDecision) -> Option<usize> {
        let outcome = approval_social_outcome(decision)?;
        Some(self.bus.publish(&NervousSignal::approval_social(
            outcome,
            APPROVAL_SOCIAL_SOURCE,
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::*;
    use crate::canonical::nervous_bus::NervousSignalKind;

    #[test]
    fn decision_mapping_is_the_three_state_fact_only() {
        assert_eq!(
            approval_social_outcome(&ApprovalDecision::Approve),
            Some(ApprovalSocialOutcome::Approved)
        );
        assert_eq!(
            approval_social_outcome(&ApprovalDecision::Reject {
                reason: Some("PII 理由不进信号".to_string()),
            }),
            Some(ApprovalSocialOutcome::Rejected)
        );
        assert_eq!(
            approval_social_outcome(&ApprovalDecision::Cancel { reason: None }),
            None
        );
    }

    #[test]
    fn publish_delivers_the_signal_with_source_label_and_no_reason_text() {
        let bus = NervousBus::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let probe = Arc::clone(&seen);
        bus.subscribe(NervousSignalKind::ApprovalSocial, move |signal| {
            probe.lock().push(signal.clone());
        });
        let source = BioApprovalSocialSource::new(bus.clone());

        let delivered = source.publish(&ApprovalDecision::Reject {
            reason: Some("含隐私的理由文本".to_string()),
        });
        assert_eq!(delivered, Some(1));
        assert_eq!(
            seen.lock().as_slice(),
            &[NervousSignal::ApprovalSocial {
                outcome: ApprovalSocialOutcome::Rejected,
                source: APPROVAL_SOCIAL_SOURCE.to_string(),
            }]
        );
        assert_eq!(bus.published_count(NervousSignalKind::ApprovalSocial), 1);
    }

    #[test]
    fn cancel_publishes_nothing_and_counts_stay_zero() {
        let bus = NervousBus::new();
        let source = BioApprovalSocialSource::new(bus.clone());
        assert_eq!(
            source.publish(&ApprovalDecision::Cancel { reason: None }),
            None
        );
        assert_eq!(bus.published_count(NervousSignalKind::ApprovalSocial), 0);
        assert!(bus.is_idle());
    }

    #[test]
    fn publish_without_subscribers_still_counts_as_one_fact() {
        let bus = NervousBus::new();
        let source = BioApprovalSocialSource::new(bus.clone());
        assert_eq!(
            source.publish(&ApprovalDecision::Approve),
            Some(0),
            "无订阅者时投递 0, 但发布事实仍记账"
        );
        assert_eq!(bus.published_count(NervousSignalKind::ApprovalSocial), 1);
        assert!(source.bus().snapshot().published[&NervousSignalKind::ApprovalSocial] == 1);
    }
}
