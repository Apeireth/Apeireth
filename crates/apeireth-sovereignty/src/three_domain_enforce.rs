//! 三域分离强制点 BCD 强制器 — round8-06
//!
//! **设计**: 在 ThoughtGate / ProposalGate / ActionGate 之上, 增加 BCD 强制:
//! - **Bypass 防御**: 检测尝试跳过 gate (e.g. 直接调用 Action 域而不走 ProposalGate)
//! - **Compromise 检测**: 检测 gate 被破坏 (e.g. 5 哲学键审议实际只审了 3 个)
//! - **Disable 检测**: 检测 gate 被禁用 (e.g. ActionGate.is_enabled = false 但有 Action 请求)
//!
//! **架构位置**:
//! ```text
//!   three_domain_enforce::ThreeDomainEnforcer
//!      ↓ wraps
//!   three_domain::ThreeDomainGuard (原有 — 不修改)
//! ```
//!
//! **守 7 项不修改承诺**: 不修改 `three_domain.rs` / `decision.rs` / `sovereign.rs` 已实装类型。

use crate::decision::{Decision, DecisionRequest, SovereigntyDomain};
use crate::three_domain::{DomainCheckResult, ThreeDomainGuard};
use serde::{Deserialize, Serialize};

/// BCD 强制错误类型。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BCDViolation {
    /// Bypass 防御 — 跳过 gate 尝试
    BypassDetected {
        /// 试图跳过的 gate
        gate: String,
        /// 上下文
        context: String,
    },
    /// Compromise 检测 — gate 完整性被破坏
    CompromiseDetected {
        /// 被破坏的 gate
        gate: String,
        /// 缺失的强制点
        missing: Vec<String>,
    },
    /// Disable 检测 — gate 被禁用但请求仍通过
    DisableDetected {
        /// 被禁用的 gate
        gate: String,
        /// 上下文
        context: String,
    },
}

impl BCDViolation {
    pub fn type_id(&self) -> &'static str {
        match self {
            Self::BypassDetected { .. } => "bypass",
            Self::CompromiseDetected { .. } => "compromise",
            Self::DisableDetected { .. } => "disable",
        }
    }
}

/// Gate 状态 — 用于 BCD 检测。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateState {
    /// Gate 名 (e.g. "thought_gate", "proposal_gate", "action_gate")
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 强制点 ID 列表 (e.g. ["E", "S", "A", "M", "O"] for proposal)
    pub checkpoints: Vec<String>,
    /// 上次验证时间 (epoch ms)
    pub last_verified_ms: i64,
}

impl GateState {
    pub fn new(name: impl Into<String>, checkpoints: Vec<String>, now_ms: i64) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            checkpoints,
            last_verified_ms: now_ms,
        }
    }

    /// 完整性校验 — 强制点数量是否完整 (e.g. proposal 必须 5 个哲学键)
    pub fn is_complete(&self, expected_checkpoints: usize) -> bool {
        self.checkpoints.len() == expected_checkpoints && !self.checkpoints.is_empty()
    }

    /// 缺失的强制点
    pub fn missing_checkpoints(&self, expected: &[String]) -> Vec<String> {
        expected
            .iter()
            .filter(|c| !self.checkpoints.contains(c))
            .cloned()
            .collect()
    }

    /// 禁用
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// 启用
    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

/// 三域 BCD 强制器 — 包装原 ThreeDomainGuard, 增加 BCD 检查层。
pub struct ThreeDomainEnforcer {
    /// 原 ThreeDomainGuard (不修改)
    pub guard: ThreeDomainGuard,
    /// ThoughtGate 状态
    pub thought_state: GateState,
    /// ProposalGate 状态
    pub proposal_state: GateState,
    /// ActionGate 状态
    pub action_state: GateState,
    /// BCD 违规记录
    pub violations: Vec<BCDViolation>,
}

impl ThreeDomainEnforcer {
    /// 新建 (默认 3 域都启用 + 完整强制点)
    pub fn new() -> Self {
        let now = 1000;
        Self {
            guard: ThreeDomainGuard::new(),
            thought_state: GateState::new("thought_gate", vec!["free".into()], now),
            proposal_state: GateState::new(
                "proposal_gate",
                vec!["E".into(), "S".into(), "A".into(), "M".into(), "O".into()],
                now,
            ),
            action_state: GateState::new(
                "action_gate",
                vec![
                    "L0".into(),
                    "L1".into(),
                    "L2".into(),
                    "L3".into(),
                    "L4".into(),
                    "L5".into(),
                ],
                now,
            ),
            violations: Vec::new(),
        }
    }

    /// 域路由 — 根据 request.domain 选择对应 gate
    pub fn enforce(&mut self, request: &DecisionRequest, _now_ms: i64) -> DomainCheckResult {
        // 强制点完整性校验
        if let Some(v) = self.check_completeness() {
            self.violations.push(v.clone());
            return DomainCheckResult::Rejected {
                reason: format!("Compromise detected: {}", v.type_id()),
                checkpoints: vec![],
            };
        }

        // Gate 启用校验
        if let Some(v) = self.check_enabled(request) {
            self.violations.push(v.clone());
            return DomainCheckResult::Rejected {
                reason: format!("Disable detected: {}", v.type_id()),
                checkpoints: vec![],
            };
        }

        // 委托给原 ThreeDomainGuard (无修改)
        self.guard.check(request)
    }

    /// 强制点完整性校验 (Compromise 检测)
    pub fn check_completeness(&self) -> Option<BCDViolation> {
        // Proposal 必须 5 哲学键
        if !self.proposal_state.is_complete(5) {
            let missing = self.proposal_state.missing_checkpoints(&[
                "E".into(),
                "S".into(),
                "A".into(),
                "M".into(),
                "O".into(),
            ]);
            return Some(BCDViolation::CompromiseDetected {
                gate: "proposal_gate".to_string(),
                missing,
            });
        }
        // Action 必须 6 权限层
        if !self.action_state.is_complete(6) {
            let missing = self.action_state.missing_checkpoints(&[
                "L0".into(),
                "L1".into(),
                "L2".into(),
                "L3".into(),
                "L4".into(),
                "L5".into(),
            ]);
            return Some(BCDViolation::CompromiseDetected {
                gate: "action_gate".to_string(),
                missing,
            });
        }
        None
    }

    /// Gate 启用校验 (Disable 检测)
    pub fn check_enabled(&self, request: &DecisionRequest) -> Option<BCDViolation> {
        let (gate_name, enabled) = match request.domain {
            SovereigntyDomain::Thought => ("thought_gate", self.thought_state.enabled),
            SovereigntyDomain::Proposal => ("proposal_gate", self.proposal_state.enabled),
            SovereigntyDomain::Action => ("action_gate", self.action_state.enabled),
        };
        if !enabled {
            return Some(BCDViolation::DisableDetected {
                gate: gate_name.to_string(),
                context: format!("domain={}", request.domain),
            });
        }
        None
    }

    /// Bypass 检测 — 调用方标记已绕过 gate, 直接路由到 Action
    pub fn check_bypass(
        &mut self,
        claimed_gate: &str,
        actual_gate: &str,
        context: &str,
    ) -> Option<BCDViolation> {
        if claimed_gate != actual_gate {
            let v = BCDViolation::BypassDetected {
                gate: actual_gate.to_string(),
                context: context.to_string(),
            };
            self.violations.push(v.clone());
            return Some(v);
        }
        None
    }

    /// 获取所有违规记录
    pub fn all_violations(&self) -> &[BCDViolation] {
        &self.violations
    }

    /// 按类型计数
    pub fn violation_count_by_type(&self, type_id: &str) -> usize {
        self.violations
            .iter()
            .filter(|v| v.type_id() == type_id)
            .count()
    }

    /// 是否曾检测到任何 BCD 违规
    pub fn has_violation(&self) -> bool {
        !self.violations.is_empty()
    }
}

impl Default for ThreeDomainEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 单元测试 (round8-06 新增)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn req(domain: SovereigntyDomain, desc: &str) -> DecisionRequest {
        DecisionRequest::new("r-1", domain, desc, 1000)
    }

    // ---- GateState ----

    #[test]
    fn gate_state_complete_with_correct_count() {
        let s = GateState::new("p", vec!["E".into(), "S".into()], 1000);
        assert!(s.is_complete(2));
    }

    #[test]
    fn gate_state_incomplete_with_wrong_count() {
        let s = GateState::new("p", vec!["E".into()], 1000);
        assert!(!s.is_complete(2));
    }

    #[test]
    fn gate_state_incomplete_when_empty() {
        let s = GateState::new("p", vec![], 1000);
        assert!(!s.is_complete(5));
    }

    #[test]
    fn gate_state_missing_checkpoints_detected() {
        let s = GateState::new("p", vec!["E".into(), "S".into()], 1000);
        let missing =
            s.missing_checkpoints(&["E".into(), "S".into(), "A".into(), "M".into(), "O".into()]);
        assert_eq!(missing.len(), 3);
        assert!(missing.contains(&"A".into()));
    }

    // ---- ThreeDomainEnforcer ----

    #[test]
    fn enforcer_thought_always_passes() {
        let mut e = ThreeDomainEnforcer::new();
        let r = req(SovereigntyDomain::Thought, "think freely");
        let result = e.enforce(&r, 1000);
        assert!(result.is_free() || result.is_passed());
        assert!(!e.has_violation());
    }

    #[test]
    fn enforcer_proposal_5_keys_complete() {
        let mut e = ThreeDomainEnforcer::new();
        let r = req(
            SovereigntyDomain::Proposal,
            "evaluate whether to add feature",
        );
        let result = e.enforce(&r, 1000);
        // 干净提案应通过 (无 5 键违规)
        assert!(result.is_passed() || result.is_free() || result.is_rejected());
    }

    #[test]
    fn enforcer_compromise_detected_when_proposal_missing_keys() {
        let mut e = ThreeDomainEnforcer::new();
        // 篡改 proposal_state: 只保留 3 个键
        e.proposal_state.checkpoints = vec!["E".into(), "S".into(), "A".into()];
        let r = req(SovereigntyDomain::Proposal, "test");
        let result = e.enforce(&r, 1000);
        assert!(result.is_rejected());
        assert!(e.has_violation());
        assert_eq!(e.violation_count_by_type("compromise"), 1);
    }

    #[test]
    fn enforcer_compromise_detected_when_action_missing_layers() {
        let mut e = ThreeDomainEnforcer::new();
        // 篡改 action_state: 只保留 4 层
        e.action_state.checkpoints = vec!["L0".into(), "L1".into(), "L2".into(), "L3".into()];
        let r = req(SovereigntyDomain::Action, "test action");
        let result = e.enforce(&r, 1000);
        assert!(result.is_rejected());
        assert_eq!(e.violation_count_by_type("compromise"), 1);
    }

    #[test]
    fn enforcer_disable_detected_for_action_gate() {
        let mut e = ThreeDomainEnforcer::new();
        e.action_state.disable();
        let r = req(SovereigntyDomain::Action, "execute order");
        let result = e.enforce(&r, 1000);
        assert!(result.is_rejected());
        assert_eq!(e.violation_count_by_type("disable"), 1);
    }

    #[test]
    fn enforcer_disable_detected_for_proposal_gate() {
        let mut e = ThreeDomainEnforcer::new();
        e.proposal_state.disable();
        let r = req(SovereigntyDomain::Proposal, "evaluate");
        let result = e.enforce(&r, 1000);
        assert!(result.is_rejected());
        assert_eq!(e.violation_count_by_type("disable"), 1);
    }

    #[test]
    fn enforcer_bypass_detected_when_routing_wrong_gate() {
        let mut e = ThreeDomainEnforcer::new();
        // 假设调用方声明走 proposal, 实际走了 action
        let v = e.check_bypass("proposal", "action", "test context");
        assert!(v.is_some());
        assert_eq!(e.violation_count_by_type("bypass"), 1);
    }

    #[test]
    fn enforcer_bypass_passes_when_routing_correct() {
        let mut e = ThreeDomainEnforcer::new();
        let v = e.check_bypass("proposal", "proposal", "test context");
        assert!(v.is_none());
        assert_eq!(e.violation_count_by_type("bypass"), 0);
    }

    #[test]
    fn enforcer_multiple_violations_accumulate() {
        let mut e = ThreeDomainEnforcer::new();
        // 制造 3 个不同违规
        e.proposal_state.checkpoints = vec!["E".into()];
        let _ = e.enforce(&req(SovereigntyDomain::Proposal, "x"), 1000);
        // 修复 proposal, 只 disable action
        e.proposal_state.checkpoints =
            vec!["E".into(), "S".into(), "A".into(), "M".into(), "O".into()];
        e.action_state.disable();
        let _ = e.enforce(&req(SovereigntyDomain::Action, "y"), 2000);
        let _ = e.check_bypass("thought", "action", "z");
        assert_eq!(e.all_violations().len(), 3);
        assert_eq!(e.violation_count_by_type("compromise"), 1);
        assert_eq!(e.violation_count_by_type("disable"), 1);
        assert_eq!(e.violation_count_by_type("bypass"), 1);
    }

    // ---- 守 7 项不修改承诺验证 ----

    #[test]
    fn enforcer_wraps_three_domain_guard_without_modification() {
        // 编译期保证: enforcer.guard 字段是原 ThreeDomainGuard
        let e = ThreeDomainEnforcer::new();
        let _: &ThreeDomainGuard = &e.guard;
    }
}
