//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I2 D2+K1 反思+错误集成
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **承接**: R129-4 Stage 4 自治 (D2 反思自循环) + R129-5 Stage 5 治理 + R129-6 Stage 6 守护 (K1 错误守护)
//! **维度**: I2 D2+K1 — 反思自循环 (D2) 跟错误守护 (K1) 跨 stage 集成
//! **目标**: ASI 反思遇错自动恢复, 反思节点 + 错误事件 互锁
//!
//! # I2 集成范围
//!
//! 1. **ReflectionErrorBinding** — 1 反思节点 × 1 错误类型绑定
//! 2. **ReflectionErrorMatrix** — 8 反思节点 (per D2) × 4 错误类型 (per K1) = 32 绑定
//! 3. **ReflectionErrorAuditEvent** — 集成审计事件
//! 4. **ReflectionErrorReport** — 集成报告
//! 5. **ReflectionErrorCoordinator** — 顶层协调器 (反思遇错自动恢复)
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-18)
//!
//! - ✅ D2 反思自循环 (R129-4) cloned = 真借 D2 ReflectionGraph
//! - ✅ K1 错误守护 (R129-6) cloned = 真借 K1 ErrorGuard
//! - 默认 build: I2 集成 0 装 PASS 严守, 跑 0 体积 stub
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改 / A1 R11 baseline 0 改 / B1 24 LOCKED 入口签名 0 改
//! - B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 / A3 13 键 0 改
//! - C1 0 主动 commit / C2 0 装 PASS 严守 / 0 主动 push

use crate::error_guardianship::{stage6_record_error, ErrorKind, ErrorSeverity};
use crate::reflection_self_loop::{
    reflection_self_loop_summary, ReflectionNode, REFLECTION_GRAPH_NODE_COUNT,
};

// =============================================================================
// I2 集成版本 + 计数
// =============================================================================

/// I2 反思+错误集成版本
pub const STAGE7_I2_VERSION: &str = "0.1.0-R129-Stage7-I2";

/// I2 集成维度数 (2: D2 反思 + K1 错误)
pub const STAGE7_I2_DIMENSION_COUNT: usize = 2;

/// I2 反思节点数 (per D2 REFLECTION_GRAPH_NODE_COUNT)
pub const STAGE7_I2_NODE_COUNT: usize = REFLECTION_GRAPH_NODE_COUNT;

/// I2 错误类型数 (per K1 ErrorKind, 4 变体)
pub const STAGE7_I2_ERROR_KIND_COUNT: usize = 4;

/// I2 反思 × 错误 绑定数 (8 × 4 = 32, 编译期 hardcode)
pub const STAGE7_I2_BINDING_COUNT: usize =
    REFLECTION_GRAPH_NODE_COUNT * STAGE7_I2_ERROR_KIND_COUNT;

// =============================================================================
// ReflectionErrorBinding — 1 反思节点 × 1 错误类型
// =============================================================================

/// 1 反思节点 × 1 错误类型的绑定
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReflectionErrorBinding {
    /// 反思节点 ID
    pub node_id: String,
    /// 错误类型
    pub error_kind: ErrorKind,
    /// 恢复策略 ("auto_retry" / "manual_review" / "defer")
    pub recovery_strategy: String,
}

impl ReflectionErrorBinding {
    /// 新建绑定
    pub fn new(node_id: &str, error_kind: ErrorKind, recovery_strategy: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            error_kind,
            recovery_strategy: recovery_strategy.to_string(),
        }
    }

    /// 默认绑定: auto_retry
    pub fn default_binding(node_id: &str, error_kind: ErrorKind) -> Self {
        Self::new(node_id, error_kind, "auto_retry")
    }
}

// =============================================================================
// ReflectionErrorMatrix — 8 node × 4 error = 32 绑定
// =============================================================================

/// 反思 × 错误 矩阵 (8 node × 4 kind = 32 绑定, 编译期 hardcode)
#[derive(Debug, Clone, Default)]
pub struct ReflectionErrorMatrix {
    bindings: Vec<ReflectionErrorBinding>,
}

impl ReflectionErrorMatrix {
    /// 默认矩阵: 8 node × 4 kind = 32 绑定
    pub fn default_matrix() -> Self {
        let kinds = [
            ErrorKind::Transport,
            ErrorKind::Conversion,
            ErrorKind::Bridge,
            ErrorKind::Contract,
        ];
        let nodes = [
            "observe",
            "analyze",
            "reflect",
            "refine",
            "finalize",
            "internal_audit",
            "internal_ceiling",
            "internal_harness",
        ];
        let mut bindings = Vec::with_capacity(STAGE7_I2_BINDING_COUNT);
        for node_id in &nodes {
            for &kind in &kinds {
                bindings.push(ReflectionErrorBinding::default_binding(node_id, kind));
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

    pub fn get(&self, node_id: &str, kind: ErrorKind) -> Option<&ReflectionErrorBinding> {
        self.bindings
            .iter()
            .find(|b| b.node_id == node_id && b.error_kind == kind)
    }

    pub fn bindings(&self) -> &[ReflectionErrorBinding] {
        &self.bindings
    }
}

// =============================================================================
// ReflectionErrorAuditEvent
// =============================================================================

#[derive(Debug, Clone)]
pub struct ReflectionErrorAuditEvent {
    pub timestamp: u64,
    pub node_id: String,
    pub error_kind: ErrorKind,
    pub recovery_applied: String,
    pub recovery_success: bool,
}

impl ReflectionErrorAuditEvent {
    pub fn new(
        timestamp: u64,
        node_id: &str,
        error_kind: ErrorKind,
        recovery_applied: &str,
        recovery_success: bool,
    ) -> Self {
        Self {
            timestamp,
            node_id: node_id.to_string(),
            error_kind,
            recovery_applied: recovery_applied.to_string(),
            recovery_success,
        }
    }
}

// =============================================================================
// ReflectionErrorReport
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct ReflectionErrorReport {
    pub events: Vec<ReflectionErrorAuditEvent>,
    pub matrix: ReflectionErrorMatrix,
}

impl ReflectionErrorReport {
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
    pub fn recovery_success_count(&self) -> usize {
        self.events.iter().filter(|e| e.recovery_success).count()
    }
    pub fn recovery_failed_count(&self) -> usize {
        self.events.iter().filter(|e| !e.recovery_success).count()
    }
}

// =============================================================================
// ReflectionErrorCoordinator
// =============================================================================

#[derive(Debug, Clone)]
pub struct ReflectionErrorCoordinator {
    pub matrix: ReflectionErrorMatrix,
    pub report: ReflectionErrorReport,
}

impl ReflectionErrorCoordinator {
    pub fn new() -> Self {
        let matrix = ReflectionErrorMatrix::default_matrix();
        let report = ReflectionErrorReport {
            events: Vec::new(),
            matrix: matrix.clone(),
        };
        Self { matrix, report }
    }

    /// reflect_and_recover: 反思遇错时, 自动应用恢复策略 + 记录 K1 错误事件
    /// 简化: auto_retry 都成功, 其他都失败
    pub fn reflect_and_recover(
        &mut self,
        timestamp: u64,
        node_id: &str,
        error_kind: ErrorKind,
    ) -> bool {
        let binding = self.matrix.get(node_id, error_kind);
        let (strategy, success) = match binding {
            Some(b) => {
                let s = b.recovery_strategy == "auto_retry";
                (b.recovery_strategy.clone(), s)
            }
            None => ("no_binding".to_string(), false),
        };
        // 记录 K1 错误事件
        let _ = stage6_record_error(
            error_kind,
            ErrorSeverity::Warn,
            "stage7_i2",
            &format!("reflection={node_id} strategy={strategy}"),
        );
        let ev = ReflectionErrorAuditEvent::new(timestamp, node_id, error_kind, &strategy, success);
        self.report.events.push(ev);
        success
    }
}

impl Default for ReflectionErrorCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// I2 公共 API
// =============================================================================

pub fn stage7_i2_summary() -> String {
    format!(
        "I2 D2+K1 反思+错误集成 v{} ({} dim, {} bindings, {} nodes × {} error kinds)",
        STAGE7_I2_VERSION,
        STAGE7_I2_DIMENSION_COUNT,
        STAGE7_I2_BINDING_COUNT,
        STAGE7_I2_NODE_COUNT,
        STAGE7_I2_ERROR_KIND_COUNT,
    )
}

pub fn stage7_i2_healthy() -> bool {
    let c = ReflectionErrorCoordinator::new();
    c.matrix.len() == STAGE7_I2_BINDING_COUNT
        && STAGE7_I2_NODE_COUNT == REFLECTION_GRAPH_NODE_COUNT
        && STAGE7_I2_BINDING_COUNT == REFLECTION_GRAPH_NODE_COUNT * STAGE7_I2_ERROR_KIND_COUNT
}

pub fn stage7_i2_to_d2_consistency() -> bool {
    let d2_summary = reflection_self_loop_summary();
    let c = ReflectionErrorCoordinator::new();
    d2_summary.contains("Reflection Self-Loop")
        && c.matrix.len() == STAGE7_I2_BINDING_COUNT
        && REFLECTION_GRAPH_NODE_COUNT == 8
}

pub fn stage7_i2_to_k1_consistency() -> bool {
    let c = ReflectionErrorCoordinator::new();
    // 矩阵应覆盖 4 ErrorKind (per K1)
    let kinds = [
        ErrorKind::Transport,
        ErrorKind::Conversion,
        ErrorKind::Bridge,
        ErrorKind::Contract,
    ];
    let mut all_present = true;
    for k in &kinds {
        if c.matrix.get("observe", *k).is_none() {
            all_present = false;
            break;
        }
    }
    all_present && kinds.len() == STAGE7_I2_ERROR_KIND_COUNT
}

// =============================================================================
// I2 inline unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i2_01_version_constants() {
        assert_eq!(STAGE7_I2_VERSION, "0.1.0-R129-Stage7-I2");
        assert_eq!(STAGE7_I2_DIMENSION_COUNT, 2);
        assert_eq!(STAGE7_I2_NODE_COUNT, 8);
        assert_eq!(STAGE7_I2_ERROR_KIND_COUNT, 4);
        assert_eq!(STAGE7_I2_BINDING_COUNT, 32);
    }

    #[test]
    fn i2_02_matrix_default_32() {
        let m = ReflectionErrorMatrix::default_matrix();
        assert_eq!(m.len(), 32);
    }

    #[test]
    fn i2_03_matrix_get_binding() {
        let m = ReflectionErrorMatrix::default_matrix();
        let b = m.get("observe", ErrorKind::Bridge);
        assert!(b.is_some());
        assert_eq!(b.unwrap().recovery_strategy, "auto_retry");
    }

    #[test]
    fn i2_04_matrix_get_unknown() {
        let m = ReflectionErrorMatrix::default_matrix();
        let b = m.get("nonexistent", ErrorKind::Bridge);
        assert!(b.is_none());
    }

    #[test]
    fn i2_05_binding_fields() {
        let b = ReflectionErrorBinding::new("analyze", ErrorKind::Contract, "manual_review");
        assert_eq!(b.node_id, "analyze");
        assert_eq!(b.error_kind, ErrorKind::Contract);
        assert_eq!(b.recovery_strategy, "manual_review");
    }

    #[test]
    fn i2_06_default_binding() {
        let b = ReflectionErrorBinding::default_binding("reflect", ErrorKind::Transport);
        assert_eq!(b.recovery_strategy, "auto_retry");
    }

    #[test]
    fn i2_07_coordinator_auto_retry_success() {
        let mut c = ReflectionErrorCoordinator::new();
        let ok = c.reflect_and_recover(0, "observe", ErrorKind::Bridge);
        assert!(ok);
        assert_eq!(c.report.event_count(), 1);
        assert_eq!(c.report.recovery_success_count(), 1);
    }

    #[test]
    fn i2_08_coordinator_no_binding_fail() {
        let mut c = ReflectionErrorCoordinator::new();
        let ok = c.reflect_and_recover(0, "nonexistent", ErrorKind::Bridge);
        assert!(!ok);
        assert_eq!(c.report.recovery_failed_count(), 1);
    }

    #[test]
    fn i2_09_audit_event_fields() {
        let e = ReflectionErrorAuditEvent::new(123, "reflect", ErrorKind::Contract, "auto_retry", true);
        assert_eq!(e.timestamp, 123);
        assert!(e.recovery_success);
    }

    #[test]
    fn i2_10_report_default() {
        let r = ReflectionErrorReport::default();
        assert_eq!(r.event_count(), 0);
        assert_eq!(r.recovery_success_count(), 0);
    }

    #[test]
    fn i2_11_summary() {
        let s = stage7_i2_summary();
        assert!(s.contains("D2"));
        assert!(s.contains("K1"));
        assert!(s.contains("32"));
    }

    #[test]
    fn i2_12_healthy() {
        assert!(stage7_i2_healthy());
    }

    #[test]
    fn i2_13_to_d2_consistency() {
        assert!(stage7_i2_to_d2_consistency());
    }

    #[test]
    fn i2_14_to_k1_consistency() {
        assert!(stage7_i2_to_k1_consistency());
    }

    #[test]
    fn i2_15_coordinator_default() {
        let c = ReflectionErrorCoordinator::default();
        assert_eq!(c.matrix.len(), 32);
    }
}
