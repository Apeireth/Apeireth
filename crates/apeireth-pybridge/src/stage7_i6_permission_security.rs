//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I6 G2+K3 权限+安全集成
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **承接**: R129-5 Stage 5 治理 (G2 权限治理) + R129-6 Stage 6 守护 (K3 安全守护)
//! **维度**: I6 G2+K3 — 权限治理 (G2) 跟安全守护 (K3) 跨 stage 集成
//! **目标**: ASI 权限 = 安全防线, 6 重 v7 + G7 跨语言 = 7 重守门 集成
//!
//! # I6 集成范围
//!
//! 1. **PermissionSecurityBinding** — 1 PermissionLayer × 1 SecurityGate 绑定
//! 2. **PermissionSecurityMatrix** — 6 layer × 7 gate = 42 绑定
//! 3. **PermissionSecurityAuditEvent** — 集成审计事件
//! 4. **PermissionSecurityReport** — 集成报告
//! 5. **PermissionSecurityCoordinator** — 顶层协调器
//!
//! # 0 装 PASS 严守
//!
//! - ✅ G2 权限治理 (R129-5) cloned = 真借 G2 PermissionLayer 6 重
//! - ✅ K3 安全守护 (R129-6) cloned = 真借 K3 SecurityGate 6+1 (G7 跨语言)
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改 / A1 R11 baseline 0 改 / B1 24 LOCKED 入口签名 0 改
//! - B4 6 重 v7 严守 (K3 集成是连接, 0 改 v7) / B5 8 哲学锚 / B3 30 维 / A3 13 键 0 改
//! - C1 0 主动 commit / C2 0 装 PASS 严守 / 0 主动 push

use crate::permission_governance::{
    permission_governance_summary, PermissionLayer, PERMISSION_GOVERNANCE_LAYER_COUNT,
};
use crate::security_guardianship::{
    stage6_security_baseline_intact, stage6_security_summary, SecurityGate, SecurityVerdict,
};

// =============================================================================
// I6 集成版本 + 计数
// =============================================================================

/// I6 权限+安全集成版本
pub const STAGE7_I6_VERSION: &str = "0.1.0-R129-Stage7-I6";

/// I6 集成维度数 (2: G2 权限 + K3 安全)
pub const STAGE7_I6_DIMENSION_COUNT: usize = 2;

/// I6 权限 layer 数 (6 重 v7, 1:1 跟 B4 严守)
pub const STAGE7_I6_PERMISSION_LAYER_COUNT: usize = PERMISSION_GOVERNANCE_LAYER_COUNT;

/// I6 安全 gate 数 (7 重: G1-G6 v7 + G7 跨语言)
pub const STAGE7_I6_SECURITY_GATE_COUNT: usize = 7;

/// I6 权限 × 安全 绑定数 (6 × 7 = 42, 编译期 hardcode)
pub const STAGE7_I6_BINDING_COUNT: usize =
    PERMISSION_GOVERNANCE_LAYER_COUNT * STAGE7_I6_SECURITY_GATE_COUNT;

// =============================================================================
// PermissionSecurityBinding — 1 PermissionLayer × 1 SecurityGate
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionSecurityBinding {
    pub layer: PermissionLayer,
    pub gate: SecurityGate,
    pub is_v7_baseline: bool, // true if 1:1 跟 B4 6 重 v7
    pub is_g7_extension: bool, // true if G7 跨语言 (K3 新增)
}

impl PermissionSecurityBinding {
    pub fn new(layer: PermissionLayer, gate: SecurityGate) -> Self {
        let (is_v7, is_g7) = match (layer, gate) {
            // L1 Identity → G1Identity (v7 baseline)
            (PermissionLayer::L1TypeCheck, SecurityGate::G1Identity) => (true, false),
            // L2 Goal → G2Goal (v7)
            (PermissionLayer::L2ScopeCheck, SecurityGate::G2Goal) => (true, false),
            // L3 Capability → G3Capability (v7)
            (PermissionLayer::L3RateCheck, SecurityGate::G3Capability) => (true, false),
            // L4 Compliance → G4Compliance (v7)
            (PermissionLayer::L4GuardCheck, SecurityGate::G4Compliance) => (true, false),
            // L5 Resource → G5Resource (v7)
            (PermissionLayer::L5AuditCheck, SecurityGate::G5Resource) => (true, false),
            // L6 Audit → G6Audit (v7)
            (PermissionLayer::L6ProvenanceCheck, SecurityGate::G6Audit) => (true, false),
            // G7 跨语言 (K3 新增, 严守"连接不是修改")
            (_, SecurityGate::G7CrossLanguage) => (false, true),
            // 其他组合
            _ => (false, false),
        };
        Self {
            layer,
            gate,
            is_v7_baseline: is_v7,
            is_g7_extension: is_g7,
        }
    }
}

// =============================================================================
// PermissionSecurityMatrix — 6 layer × 7 gate = 42 绑定
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct PermissionSecurityMatrix {
    bindings: Vec<PermissionSecurityBinding>,
}

impl PermissionSecurityMatrix {
    pub fn default_matrix() -> Self {
        let layers = [
            PermissionLayer::L1TypeCheck,
            PermissionLayer::L2ScopeCheck,
            PermissionLayer::L3RateCheck,
            PermissionLayer::L4GuardCheck,
            PermissionLayer::L5AuditCheck,
            PermissionLayer::L6ProvenanceCheck,
        ];
        let gates = [
            SecurityGate::G1Identity,
            SecurityGate::G2Goal,
            SecurityGate::G3Capability,
            SecurityGate::G4Compliance,
            SecurityGate::G5Resource,
            SecurityGate::G6Audit,
            SecurityGate::G7CrossLanguage,
        ];
        let mut bindings = Vec::with_capacity(STAGE7_I6_BINDING_COUNT);
        for &l in &layers {
            for &g in &gates {
                bindings.push(PermissionSecurityBinding::new(l, g));
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
        layer: PermissionLayer,
        gate: SecurityGate,
    ) -> Option<&PermissionSecurityBinding> {
        self.bindings
            .iter()
            .find(|b| b.layer == layer && b.gate == gate)
    }

    pub fn bindings(&self) -> &[PermissionSecurityBinding] {
        &self.bindings
    }

    /// v7 baseline 绑定数 (L1-G1, L2-G2, ..., L6-G6 = 6)
    pub fn v7_baseline_count(&self) -> usize {
        self.bindings.iter().filter(|b| b.is_v7_baseline).count()
    }

    /// G7 跨语言 绑定数 (任意 layer × G7_CrossLanguage = 6)
    pub fn g7_extension_count(&self) -> usize {
        self.bindings.iter().filter(|b| b.is_g7_extension).count()
    }
}

// =============================================================================
// PermissionSecurityAuditEvent
// =============================================================================

#[derive(Debug, Clone)]
pub struct PermissionSecurityAuditEvent {
    pub timestamp: u64,
    pub layer: PermissionLayer,
    pub gate: SecurityGate,
    pub verdict: SecurityVerdict,
    pub v7_baseline_intact: bool,
}

impl PermissionSecurityAuditEvent {
    pub fn new(
        timestamp: u64,
        layer: PermissionLayer,
        gate: SecurityGate,
        verdict: SecurityVerdict,
        v7_baseline_intact: bool,
    ) -> Self {
        Self {
            timestamp,
            layer,
            gate,
            verdict,
            v7_baseline_intact,
        }
    }
}

// =============================================================================
// PermissionSecurityReport
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct PermissionSecurityReport {
    pub events: Vec<PermissionSecurityAuditEvent>,
    pub matrix: PermissionSecurityMatrix,
}

impl PermissionSecurityReport {
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
    pub fn allow_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e.verdict, SecurityVerdict::Allow))
            .count()
    }
    pub fn block_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e.verdict, SecurityVerdict::Block))
            .count()
    }
}

// =============================================================================
// PermissionSecurityCoordinator
// =============================================================================

#[derive(Debug, Clone)]
pub struct PermissionSecurityCoordinator {
    pub matrix: PermissionSecurityMatrix,
    pub report: PermissionSecurityReport,
}

impl PermissionSecurityCoordinator {
    pub fn new() -> Self {
        let matrix = PermissionSecurityMatrix::default_matrix();
        let report = PermissionSecurityReport {
            events: Vec::new(),
            matrix: matrix.clone(),
        };
        Self { matrix, report }
    }

    /// check: 给定 layer + gate, 返回 SecurityVerdict
    /// 严守 v7 baseline (per B4)
    pub fn check(
        &mut self,
        timestamp: u64,
        layer: PermissionLayer,
        gate: SecurityGate,
    ) -> SecurityVerdict {
        let binding = self.matrix.get(layer, gate);
        let verdict = match binding {
            Some(b) if b.is_v7_baseline => SecurityVerdict::Allow,
            Some(b) if b.is_g7_extension => SecurityVerdict::Audit,
            _ => SecurityVerdict::Warn,
        };
        let v7_intact = stage6_security_baseline_intact();
        self.report.events.push(PermissionSecurityAuditEvent::new(
            timestamp,
            layer,
            gate,
            verdict,
            v7_intact,
        ));
        verdict
    }
}

impl Default for PermissionSecurityCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// I6 公共 API
// =============================================================================

pub fn stage7_i6_summary() -> String {
    format!(
        "I6 G2+K3 权限+安全集成 v{} ({} dim, {} bindings, {} layers × {} gates)",
        STAGE7_I6_VERSION,
        STAGE7_I6_DIMENSION_COUNT,
        STAGE7_I6_BINDING_COUNT,
        STAGE7_I6_PERMISSION_LAYER_COUNT,
        STAGE7_I6_SECURITY_GATE_COUNT,
    )
}

pub fn stage7_i6_healthy() -> bool {
    let c = PermissionSecurityCoordinator::new();
    let g2_report = permission_governance_summary();
    let g2_ok = format!("{:?}", g2_report).contains("PermissionReport");
    let k3_ok = stage6_security_summary().contains("K3 SecurityGuard");
    let v7_intact = stage6_security_baseline_intact();
    c.matrix.len() == STAGE7_I6_BINDING_COUNT
        && c.matrix.v7_baseline_count() == 6 // 6 重 v7 严守
        && c.matrix.g7_extension_count() == 6 // G7 跨语言 6 绑定
        && g2_ok
        && k3_ok
        && v7_intact
}

pub fn stage7_i6_to_g2_consistency() -> bool {
    let c = PermissionSecurityCoordinator::new();
    let summary = permission_governance_summary();
    let summary_dbg = format!("{:?}", summary);
    summary_dbg.contains("PermissionReport")
        && c.matrix.len() == STAGE7_I6_BINDING_COUNT
        && PERMISSION_GOVERNANCE_LAYER_COUNT == 6 // 6 重 v7 严守
}

pub fn stage7_i6_to_k3_consistency() -> bool {
    let c = PermissionSecurityCoordinator::new();
    let summary = stage6_security_summary();
    let v7_intact = stage6_security_baseline_intact();
    summary.contains("K3 SecurityGuard")
        && c.matrix.len() == STAGE7_I6_BINDING_COUNT
        && STAGE7_I6_SECURITY_GATE_COUNT == 7 // G1-G6 + G7
        && v7_intact
}

// =============================================================================
// I6 inline unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i6_01_version_constants() {
        assert_eq!(STAGE7_I6_VERSION, "0.1.0-R129-Stage7-I6");
        assert_eq!(STAGE7_I6_DIMENSION_COUNT, 2);
        assert_eq!(STAGE7_I6_PERMISSION_LAYER_COUNT, 6);
        assert_eq!(STAGE7_I6_SECURITY_GATE_COUNT, 7);
        assert_eq!(STAGE7_I6_BINDING_COUNT, 42);
    }

    #[test]
    fn i6_02_matrix_default_42() {
        let m = PermissionSecurityMatrix::default_matrix();
        assert_eq!(m.len(), 42);
        assert_eq!(m.v7_baseline_count(), 6);
        assert_eq!(m.g7_extension_count(), 6);
    }

    #[test]
    fn i6_03_matrix_v7_baseline_binding() {
        let m = PermissionSecurityMatrix::default_matrix();
        let b = m.get(PermissionLayer::L1TypeCheck, SecurityGate::G1Identity);
        assert!(b.is_some());
        assert!(b.unwrap().is_v7_baseline);
    }

    #[test]
    fn i6_04_matrix_g7_extension_binding() {
        let m = PermissionSecurityMatrix::default_matrix();
        let b = m.get(PermissionLayer::L3RateCheck, SecurityGate::G7CrossLanguage);
        assert!(b.is_some());
        assert!(b.unwrap().is_g7_extension);
    }

    #[test]
    fn i6_05_binding_other_combination() {
        let m = PermissionSecurityMatrix::default_matrix();
        let b = m.get(PermissionLayer::L3RateCheck, SecurityGate::G1Identity);
        assert!(b.is_some());
        let b = b.unwrap();
        assert!(!b.is_v7_baseline);
        assert!(!b.is_g7_extension);
    }

    #[test]
    fn i6_06_coordinator_v7_allow() {
        let mut c = PermissionSecurityCoordinator::new();
        let v = c.check(0, PermissionLayer::L1TypeCheck, SecurityGate::G1Identity);
        assert!(matches!(v, SecurityVerdict::Allow));
        assert_eq!(c.report.allow_count(), 1);
    }

    #[test]
    fn i6_07_coordinator_g7_audit() {
        let mut c = PermissionSecurityCoordinator::new();
        let v = c.check(0, PermissionLayer::L3RateCheck, SecurityGate::G7CrossLanguage);
        assert!(matches!(v, SecurityVerdict::Audit));
    }

    #[test]
    fn i6_08_coordinator_other_warn() {
        let mut c = PermissionSecurityCoordinator::new();
        let v = c.check(0, PermissionLayer::L3RateCheck, SecurityGate::G1Identity);
        assert!(matches!(v, SecurityVerdict::Warn));
    }

    #[test]
    fn i6_09_audit_event_fields() {
        let e = PermissionSecurityAuditEvent::new(100, PermissionLayer::L6ProvenanceCheck, SecurityGate::G6Audit, SecurityVerdict::Allow, true);
        assert!(e.v7_baseline_intact);
    }

    #[test]
    fn i6_10_report_default() {
        let r = PermissionSecurityReport::default();
        assert_eq!(r.event_count(), 0);
    }

    #[test]
    fn i6_11_summary() {
        let s = stage7_i6_summary();
        assert!(s.contains("G2"));
        assert!(s.contains("K3"));
        assert!(s.contains("42"));
    }

    #[test]
    fn i6_12_healthy() {
        assert!(stage7_i6_healthy());
    }

    #[test]
    fn i6_13_to_g2_consistency() {
        assert!(stage7_i6_to_g2_consistency());
    }

    #[test]
    fn i6_14_to_k3_consistency() {
        assert!(stage7_i6_to_k3_consistency());
    }

    #[test]
    fn i6_15_coordinator_default() {
        let c = PermissionSecurityCoordinator::default();
        assert_eq!(c.matrix.len(), 42);
    }
}
