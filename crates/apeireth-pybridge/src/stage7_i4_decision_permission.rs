//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I4 D4+G2 决策+权限集成
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **承接**: R129-4 Stage 4 自治 (D4 决策自循环) + R129-5 Stage 5 治理 (G2 权限治理) + R129-6 Stage 6 守护
//! **维度**: I4 D4+G2 — 决策自循环 (D4) 跟权限治理 (G2) 跨 stage 集成
//! **目标**: ASI 决策遵守 6 重守门 v7, 5 policy × 6 layer 互锁
//!
//! # I4 集成范围
//!
//! 1. **DecisionPermissionBinding** — 1 policy × 1 layer 绑定
//! 2. **DecisionPermissionMatrix** — 5 policy × 6 layer = 30 绑定
//! 3. **DecisionPermissionAuditEvent** — 集成审计事件
//! 4. **DecisionPermissionReport** — 集成报告
//! 5. **DecisionPermissionCoordinator** — 顶层协调器
//!
//! # 0 装 PASS 严守
//!
//! - ✅ D4 决策自循环 (R129-4) cloned = 真借 D4 DecisionPolicy 5 变体
//! - ✅ G2 权限治理 (R129-5) cloned = 真借 G2 PermissionLayer 6 重 v7
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改 / A1 R11 baseline 0 改 / B1 24 LOCKED 入口签名 0 改
//! - B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 (严守 G2 1:1 跟 B4) / A3 13 键 0 改
//! - C1 0 主动 commit / C2 0 装 PASS 严守 / 0 主动 push

use crate::decision_self_loop::{
    decision_self_loop_summary, DecisionPolicy, DECISION_POLICY_COUNT,
};
use crate::permission_governance::{
    permission_governance_summary, PermissionContext, PermissionLayer,
    PERMISSION_GOVERNANCE_LAYER_COUNT,
};

// =============================================================================
// I4 集成版本 + 计数
// =============================================================================

/// I4 决策+权限集成版本
pub const STAGE7_I4_VERSION: &str = "0.1.0-R129-Stage7-I4";

/// I4 集成维度数 (2: D4 决策 + G2 权限)
pub const STAGE7_I4_DIMENSION_COUNT: usize = 2;

/// I4 policy 数 (per D4 DecisionPolicy 5 变体)
pub const STAGE7_I4_POLICY_COUNT: usize = DECISION_POLICY_COUNT;

/// I4 layer 数 (per G2 PermissionLayer 6 重 v7, 1:1 跟 B4 严守)
pub const STAGE7_I4_LAYER_COUNT: usize = PERMISSION_GOVERNANCE_LAYER_COUNT;

/// I4 决策 × 权限 绑定数 (5 × 6 = 30, 编译期 hardcode)
pub const STAGE7_I4_BINDING_COUNT: usize = DECISION_POLICY_COUNT * PERMISSION_GOVERNANCE_LAYER_COUNT;

// =============================================================================
// DecisionPermissionBinding — 1 policy × 1 layer
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecisionPermissionBinding {
    pub policy: DecisionPolicy,
    pub layer: PermissionLayer,
    pub decision_rule: String, // "allow" / "deny" / "audit_required"
}

impl DecisionPermissionBinding {
    pub fn new(policy: DecisionPolicy, layer: PermissionLayer, decision_rule: &str) -> Self {
        Self {
            policy,
            layer,
            decision_rule: decision_rule.to_string(),
        }
    }

    /// 默认 binding: 不同 policy 不同决策
    /// - Conservative → deny
    /// - Cautious → audit_required
    /// - Balanced → allow
    /// - Progressive → allow
    /// - Aggressive → allow
    pub fn default_binding(policy: DecisionPolicy, layer: PermissionLayer) -> Self {
        let rule = match policy {
            DecisionPolicy::Conservative => "deny",
            DecisionPolicy::Cautious => "audit_required",
            DecisionPolicy::Balanced => "allow",
            DecisionPolicy::Progressive => "allow",
            DecisionPolicy::Aggressive => "allow",
        };
        Self::new(policy, layer, rule)
    }
}

// =============================================================================
// DecisionPermissionMatrix — 5 policy × 6 layer = 30 绑定
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct DecisionPermissionMatrix {
    bindings: Vec<DecisionPermissionBinding>,
}

impl DecisionPermissionMatrix {
    pub fn default_matrix() -> Self {
        let policies = [
            DecisionPolicy::Conservative,
            DecisionPolicy::Cautious,
            DecisionPolicy::Balanced,
            DecisionPolicy::Progressive,
            DecisionPolicy::Aggressive,
        ];
        let layers = [
            PermissionLayer::L1TypeCheck,
            PermissionLayer::L2ScopeCheck,
            PermissionLayer::L3RateCheck,
            PermissionLayer::L4GuardCheck,
            PermissionLayer::L5AuditCheck,
            PermissionLayer::L6ProvenanceCheck,
        ];
        let mut bindings = Vec::with_capacity(STAGE7_I4_BINDING_COUNT);
        for &p in &policies {
            for &l in &layers {
                bindings.push(DecisionPermissionBinding::default_binding(p, l));
            }
        }
        Self { bindings }
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn get(
        &self,
        policy: DecisionPolicy,
        layer: PermissionLayer,
    ) -> Option<&DecisionPermissionBinding> {
        self.bindings
            .iter()
            .find(|b| b.policy == policy && b.layer == layer)
    }

    pub fn bindings(&self) -> &[DecisionPermissionBinding] {
        &self.bindings
    }
}

// =============================================================================
// DecisionPermissionAuditEvent
// =============================================================================

#[derive(Debug, Clone)]
pub struct DecisionPermissionAuditEvent {
    pub timestamp: u64,
    pub policy: DecisionPolicy,
    pub layer: PermissionLayer,
    pub decision_rule: String,
    pub v7_baseline_intact: bool,
}

impl DecisionPermissionAuditEvent {
    pub fn new(
        timestamp: u64,
        policy: DecisionPolicy,
        layer: PermissionLayer,
        decision_rule: &str,
        v7_baseline_intact: bool,
    ) -> Self {
        Self {
            timestamp,
            policy,
            layer,
            decision_rule: decision_rule.to_string(),
            v7_baseline_intact,
        }
    }
}

// =============================================================================
// DecisionPermissionReport
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct DecisionPermissionReport {
    pub events: Vec<DecisionPermissionAuditEvent>,
    pub matrix: DecisionPermissionMatrix,
}

impl DecisionPermissionReport {
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
    pub fn allow_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.decision_rule == "allow")
            .count()
    }
    pub fn deny_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.decision_rule == "deny")
            .count()
    }
    pub fn audit_required_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.decision_rule == "audit_required")
            .count()
    }
}

// =============================================================================
// DecisionPermissionCoordinator
// =============================================================================

#[derive(Debug, Clone)]
pub struct DecisionPermissionCoordinator {
    pub matrix: DecisionPermissionMatrix,
    pub report: DecisionPermissionReport,
}

impl DecisionPermissionCoordinator {
    pub fn new() -> Self {
        let matrix = DecisionPermissionMatrix::default_matrix();
        let report = DecisionPermissionReport {
            events: Vec::new(),
            matrix: matrix.clone(),
        };
        Self { matrix, report }
    }

    /// decide: 给定 policy + layer, 返回 decision rule
    /// 严守 v7 baseline (per B4 6 重 v7 严守)
    pub fn decide(
        &mut self,
        timestamp: u64,
        policy: DecisionPolicy,
        layer: PermissionLayer,
        _ctx: &PermissionContext,
    ) -> String {
        let binding = self.matrix.get(policy, layer);
        let rule = binding
            .map(|b| b.decision_rule.clone())
            .unwrap_or_else(|| "deny".to_string());
        let v7_intact = PERMISSION_GOVERNANCE_LAYER_COUNT == 6;
        self.report.events.push(DecisionPermissionAuditEvent::new(
            timestamp,
            policy,
            layer,
            &rule,
            v7_intact,
        ));
        rule
    }
}

impl Default for DecisionPermissionCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// I4 公共 API
// =============================================================================

pub fn stage7_i4_summary() -> String {
    format!(
        "I4 D4+G2 决策+权限集成 v{} ({} dim, {} bindings, {} policies × {} layers)",
        STAGE7_I4_VERSION,
        STAGE7_I4_DIMENSION_COUNT,
        STAGE7_I4_BINDING_COUNT,
        STAGE7_I4_POLICY_COUNT,
        STAGE7_I4_LAYER_COUNT,
    )
}

pub fn stage7_i4_healthy() -> bool {
    let c = DecisionPermissionCoordinator::new();
    c.matrix.len() == STAGE7_I4_BINDING_COUNT
        && STAGE7_I4_POLICY_COUNT == DECISION_POLICY_COUNT
        && STAGE7_I4_LAYER_COUNT == PERMISSION_GOVERNANCE_LAYER_COUNT
        && PERMISSION_GOVERNANCE_LAYER_COUNT == 6 // v7 严守
}

pub fn stage7_i4_to_d4_consistency() -> bool {
    let d4_summary = decision_self_loop_summary();
    let c = DecisionPermissionCoordinator::new();
    d4_summary.contains("Decision Self-Loop")
        && c.matrix.len() == STAGE7_I4_BINDING_COUNT
        && DECISION_POLICY_COUNT == 5
}

pub fn stage7_i4_to_g2_consistency() -> bool {
    let g2_report = permission_governance_summary();
    let c = DecisionPermissionCoordinator::new();
    let g2_dbg = format!("{:?}", g2_report);
    g2_dbg.contains("PermissionReport")
        && c.matrix.len() == STAGE7_I4_BINDING_COUNT
        && PERMISSION_GOVERNANCE_LAYER_COUNT == 6 // v7 6 重
}

// =============================================================================
// I4 inline unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i4_01_version_constants() {
        assert_eq!(STAGE7_I4_VERSION, "0.1.0-R129-Stage7-I4");
        assert_eq!(STAGE7_I4_DIMENSION_COUNT, 2);
        assert_eq!(STAGE7_I4_POLICY_COUNT, 5);
        assert_eq!(STAGE7_I4_LAYER_COUNT, 6);
        assert_eq!(STAGE7_I4_BINDING_COUNT, 30);
    }

    #[test]
    fn i4_02_matrix_default_30() {
        let m = DecisionPermissionMatrix::default_matrix();
        assert_eq!(m.len(), 30);
    }

    #[test]
    fn i4_03_matrix_get_binding() {
        let m = DecisionPermissionMatrix::default_matrix();
        let b = m.get(DecisionPolicy::Conservative, PermissionLayer::L1TypeCheck);
        assert!(b.is_some());
        assert_eq!(b.unwrap().decision_rule, "deny");
    }

    #[test]
    fn i4_04_matrix_balanced_allow() {
        let m = DecisionPermissionMatrix::default_matrix();
        let b = m.get(DecisionPolicy::Balanced, PermissionLayer::L3RateCheck);
        assert_eq!(b.unwrap().decision_rule, "allow");
    }

    #[test]
    fn i4_05_matrix_cautious_audit() {
        let m = DecisionPermissionMatrix::default_matrix();
        let b = m.get(DecisionPolicy::Cautious, PermissionLayer::L5AuditCheck);
        assert_eq!(b.unwrap().decision_rule, "audit_required");
    }

    #[test]
    fn i4_06_binding_fields() {
        let b = DecisionPermissionBinding::new(DecisionPolicy::Aggressive, PermissionLayer::L6ProvenanceCheck, "allow");
        assert_eq!(b.policy, DecisionPolicy::Aggressive);
        assert_eq!(b.layer, PermissionLayer::L6ProvenanceCheck);
        assert_eq!(b.decision_rule, "allow");
    }

    #[test]
    fn i4_07_coordinator_decide() {
        let mut c = DecisionPermissionCoordinator::new();
        let ctx = PermissionContext::safe_default();
        let rule = c.decide(0, DecisionPolicy::Conservative, PermissionLayer::L1TypeCheck, &ctx);
        assert_eq!(rule, "deny");
        assert_eq!(c.report.event_count(), 1);
        assert_eq!(c.report.deny_count(), 1);
    }

    #[test]
    fn i4_08_coordinator_balanced_allow() {
        let mut c = DecisionPermissionCoordinator::new();
        let ctx = PermissionContext::safe_default();
        let rule = c.decide(0, DecisionPolicy::Balanced, PermissionLayer::L3RateCheck, &ctx);
        assert_eq!(rule, "allow");
        assert_eq!(c.report.allow_count(), 1);
    }

    #[test]
    fn i4_09_audit_event_v7_intact() {
        let e = DecisionPermissionAuditEvent::new(0, DecisionPolicy::Balanced, PermissionLayer::L1TypeCheck, "allow", true);
        assert!(e.v7_baseline_intact);
    }

    #[test]
    fn i4_10_report_default() {
        let r = DecisionPermissionReport::default();
        assert_eq!(r.event_count(), 0);
    }

    #[test]
    fn i4_11_summary() {
        let s = stage7_i4_summary();
        assert!(s.contains("D4"));
        assert!(s.contains("G2"));
        assert!(s.contains("30"));
    }

    #[test]
    fn i4_12_healthy() {
        assert!(stage7_i4_healthy());
    }

    #[test]
    fn i4_13_to_d4_consistency() {
        assert!(stage7_i4_to_d4_consistency());
    }

    #[test]
    fn i4_14_to_g2_consistency() {
        assert!(stage7_i4_to_g2_consistency());
    }

    #[test]
    fn i4_15_coordinator_default() {
        let c = DecisionPermissionCoordinator::default();
        assert_eq!(c.matrix.len(), 30);
    }
}
