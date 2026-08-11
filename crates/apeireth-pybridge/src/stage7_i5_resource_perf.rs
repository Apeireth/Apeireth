//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I5 G1+K2 资源+性能集成
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **承接**: R129-5 Stage 5 治理 (G1 资源治理) + R129-6 Stage 6 守护 (K2 性能守护)
//! **维度**: I5 G1+K2 — 资源治理 (G1) 跟性能守护 (K2) 跨 stage 集成
//! **目标**: ASI 资源使用性能监控, 4 资源维度 × 5 perf kind = 20 绑定
//!
//! # I5 集成范围
//!
//! 1. **ResourcePerfBinding** — 1 资源维度 × 1 perf kind 绑定
//! 2. **ResourcePerfMatrix** — 4 dim × 5 kind = 20 绑定
//! 3. **ResourcePerfAuditEvent** — 集成审计事件
//! 4. **ResourcePerfReport** — 集成报告
//! 5. **ResourcePerfCoordinator** — 顶层协调器
//!
//! # 0 装 PASS 严守
//!
//! - ✅ G1 资源治理 (R129-5) cloned = 真借 G1 ResourceDimension
//! - ✅ K2 性能守护 (R129-6) cloned = 真借 K2 PerfKind
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改 / A1 R11 baseline 0 改 / B1 24 LOCKED 入口签名 0 改
//! - B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 / A3 13 键 0 改
//! - C1 0 主动 commit / C2 0 装 PASS 严守 / 0 主动 push

use crate::perf_guardianship::{
    stage6_perf_healthy, stage6_perf_summary, PerfKind,
};
use crate::resource_governance::{
    resource_governance_summary, ResourceDimension, RESOURCE_GOVERNANCE_DIMENSION_COUNT,
};

// =============================================================================
// I5 集成版本 + 计数
// =============================================================================

/// I5 资源+性能集成版本
pub const STAGE7_I5_VERSION: &str = "0.1.0-R129-Stage7-I5";

/// I5 集成维度数 (2: G1 资源 + K2 性能)
pub const STAGE7_I5_DIMENSION_COUNT: usize = 2;

/// I5 perf kind 数 (per K2 PerfKind 5 变体)
pub const STAGE7_I5_PERF_KIND_COUNT: usize = 5;

/// I5 资源维度数 (per G1, 4)
pub const STAGE7_I5_RESOURCE_DIM_COUNT: usize = RESOURCE_GOVERNANCE_DIMENSION_COUNT;

/// I5 资源 × 性能 绑定数 (4 × 5 = 20, 编译期 hardcode)
pub const STAGE7_I5_BINDING_COUNT: usize =
    RESOURCE_GOVERNANCE_DIMENSION_COUNT * STAGE7_I5_PERF_KIND_COUNT;

// =============================================================================
// ResourcePerfBinding — 1 资源维度 × 1 perf kind
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourcePerfBinding {
    pub dimension: ResourceDimension,
    pub perf_kind: PerfKind,
    pub threshold_us: u64, // 性能阈值 (微秒)
}

impl ResourcePerfBinding {
    pub fn new(dimension: ResourceDimension, perf_kind: PerfKind, threshold_us: u64) -> Self {
        Self {
            dimension,
            perf_kind,
            threshold_us,
        }
    }

    /// 默认 binding: 各 dim × kind 默认阈值
    pub fn default_binding(dimension: ResourceDimension, perf_kind: PerfKind) -> Self {
        let threshold = match (dimension, perf_kind) {
            // Rate → Bridge threshold 500us
            (ResourceDimension::Rate, PerfKind::Bridge) => 500,
            // Memory → Eval 1000us
            (ResourceDimension::Memory, PerfKind::Eval) => 1000,
            // Time → Import 5000us
            (ResourceDimension::Time, PerfKind::Import) => 5000,
            // Count → Call 800us
            (ResourceDimension::Count, PerfKind::Call) => 800,
            _ => 1000,
        };
        Self::new(dimension, perf_kind, threshold)
    }
}

// =============================================================================
// ResourcePerfMatrix — 4 dim × 5 kind = 20 绑定
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct ResourcePerfMatrix {
    bindings: Vec<ResourcePerfBinding>,
}

impl ResourcePerfMatrix {
    pub fn default_matrix() -> Self {
        let kinds = [
            PerfKind::Bridge,
            PerfKind::Eval,
            PerfKind::Import,
            PerfKind::Convert,
            PerfKind::Call,
        ];
        let mut bindings = Vec::with_capacity(STAGE7_I5_BINDING_COUNT);
        for &dim in &ResourceDimension::ALL {
            for &kind in &kinds {
                bindings.push(ResourcePerfBinding::default_binding(dim, kind));
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
        dim: ResourceDimension,
        kind: PerfKind,
    ) -> Option<&ResourcePerfBinding> {
        self.bindings
            .iter()
            .find(|b| b.dimension == dim && b.perf_kind == kind)
    }

    pub fn bindings(&self) -> &[ResourcePerfBinding] {
        &self.bindings
    }
}

// =============================================================================
// ResourcePerfAuditEvent
// =============================================================================

#[derive(Debug, Clone)]
pub struct ResourcePerfAuditEvent {
    pub timestamp: u64,
    pub dimension: ResourceDimension,
    pub perf_kind: PerfKind,
    pub threshold_us: u64,
    pub observed_us: u64,
    pub over_threshold: bool,
}

impl ResourcePerfAuditEvent {
    pub fn new(
        timestamp: u64,
        dimension: ResourceDimension,
        perf_kind: PerfKind,
        threshold_us: u64,
        observed_us: u64,
        over_threshold: bool,
    ) -> Self {
        Self {
            timestamp,
            dimension,
            perf_kind,
            threshold_us,
            observed_us,
            over_threshold,
        }
    }
}

// =============================================================================
// ResourcePerfReport
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct ResourcePerfReport {
    pub events: Vec<ResourcePerfAuditEvent>,
    pub matrix: ResourcePerfMatrix,
}

impl ResourcePerfReport {
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
    pub fn over_threshold_count(&self) -> usize {
        self.events.iter().filter(|e| e.over_threshold).count()
    }
}

// =============================================================================
// ResourcePerfCoordinator
// =============================================================================

#[derive(Debug, Clone)]
pub struct ResourcePerfCoordinator {
    pub matrix: ResourcePerfMatrix,
    pub report: ResourcePerfReport,
}

impl ResourcePerfCoordinator {
    pub fn new() -> Self {
        let matrix = ResourcePerfMatrix::default_matrix();
        let report = ResourcePerfReport {
            events: Vec::new(),
            matrix: matrix.clone(),
        };
        Self { matrix, report }
    }

    /// observe: 记录 1 个观察, verify 是否超阈值
    pub fn observe(
        &mut self,
        timestamp: u64,
        dim: ResourceDimension,
        kind: PerfKind,
        observed_us: u64,
    ) -> bool {
        let binding = self.matrix.get(dim, kind);
        let threshold = binding.map(|b| b.threshold_us).unwrap_or(1000);
        let over = observed_us > threshold;
        self.report.events.push(ResourcePerfAuditEvent::new(
            timestamp,
            dim,
            kind,
            threshold,
            observed_us,
            over,
        ));
        over
    }
}

impl Default for ResourcePerfCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// I5 公共 API
// =============================================================================

pub fn stage7_i5_summary() -> String {
    format!(
        "I5 G1+K2 资源+性能集成 v{} ({} dim, {} bindings, {} dims × {} kinds)",
        STAGE7_I5_VERSION,
        STAGE7_I5_DIMENSION_COUNT,
        STAGE7_I5_BINDING_COUNT,
        STAGE7_I5_RESOURCE_DIM_COUNT,
        STAGE7_I5_PERF_KIND_COUNT,
    )
}

pub fn stage7_i5_healthy() -> bool {
    let c = ResourcePerfCoordinator::new();
    let g1_report = resource_governance_summary();
    let g1_ok = format!("{:?}", g1_report).contains("ResourceReport");
    let k2_ok = stage6_perf_healthy();
    c.matrix.len() == STAGE7_I5_BINDING_COUNT
        && g1_ok
        && k2_ok
        && STAGE7_I5_BINDING_COUNT == RESOURCE_GOVERNANCE_DIMENSION_COUNT * STAGE7_I5_PERF_KIND_COUNT
}

pub fn stage7_i5_to_g1_consistency() -> bool {
    let c = ResourcePerfCoordinator::new();
    let summary = resource_governance_summary();
    let summary_dbg = format!("{:?}", summary);
    summary_dbg.contains("ResourceReport")
        && c.matrix.len() == STAGE7_I5_BINDING_COUNT
        && RESOURCE_GOVERNANCE_DIMENSION_COUNT == 4
}

pub fn stage7_i5_to_k2_consistency() -> bool {
    let c = ResourcePerfCoordinator::new();
    let summary = stage6_perf_summary();
    summary.contains("PerfMonitor")
        && c.matrix.len() == STAGE7_I5_BINDING_COUNT
        && STAGE7_I5_PERF_KIND_COUNT == 5
}

// =============================================================================
// I5 inline unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i5_01_version_constants() {
        assert_eq!(STAGE7_I5_VERSION, "0.1.0-R129-Stage7-I5");
        assert_eq!(STAGE7_I5_DIMENSION_COUNT, 2);
        assert_eq!(STAGE7_I5_PERF_KIND_COUNT, 5);
        assert_eq!(STAGE7_I5_RESOURCE_DIM_COUNT, 4);
        assert_eq!(STAGE7_I5_BINDING_COUNT, 20);
    }

    #[test]
    fn i5_02_matrix_default_20() {
        let m = ResourcePerfMatrix::default_matrix();
        assert_eq!(m.len(), 20);
    }

    #[test]
    fn i5_03_matrix_get_binding_rate_bridge() {
        let m = ResourcePerfMatrix::default_matrix();
        let b = m.get(ResourceDimension::Rate, PerfKind::Bridge);
        assert!(b.is_some());
        assert_eq!(b.unwrap().threshold_us, 500);
    }

    #[test]
    fn i5_04_matrix_get_binding_memory_eval() {
        let m = ResourcePerfMatrix::default_matrix();
        let b = m.get(ResourceDimension::Memory, PerfKind::Eval);
        assert!(b.is_some());
        assert_eq!(b.unwrap().threshold_us, 1000);
    }

    #[test]
    fn i5_05_binding_fields() {
        let b = ResourcePerfBinding::new(ResourceDimension::Time, PerfKind::Import, 5000);
        assert_eq!(b.dimension, ResourceDimension::Time);
        assert_eq!(b.perf_kind, PerfKind::Import);
        assert_eq!(b.threshold_us, 5000);
    }

    #[test]
    fn i5_06_default_binding_default_thresholds() {
        // Rate → Bridge = 500
        let b = ResourcePerfBinding::default_binding(ResourceDimension::Rate, PerfKind::Bridge);
        assert_eq!(b.threshold_us, 500);
        // Time → Import = 5000
        let b = ResourcePerfBinding::default_binding(ResourceDimension::Time, PerfKind::Import);
        assert_eq!(b.threshold_us, 5000);
    }

    #[test]
    fn i5_07_coordinator_observe_under() {
        let mut c = ResourcePerfCoordinator::new();
        let over = c.observe(0, ResourceDimension::Rate, PerfKind::Bridge, 100);
        assert!(!over);
        assert_eq!(c.report.event_count(), 1);
        assert_eq!(c.report.over_threshold_count(), 0);
    }

    #[test]
    fn i5_08_coordinator_observe_over() {
        let mut c = ResourcePerfCoordinator::new();
        let over = c.observe(0, ResourceDimension::Rate, PerfKind::Bridge, 1000);
        assert!(over);
        assert_eq!(c.report.over_threshold_count(), 1);
    }

    #[test]
    fn i5_09_audit_event_fields() {
        let e = ResourcePerfAuditEvent::new(100, ResourceDimension::Count, PerfKind::Call, 800, 1500, true);
        assert_eq!(e.threshold_us, 800);
        assert_eq!(e.observed_us, 1500);
        assert!(e.over_threshold);
    }

    #[test]
    fn i5_10_report_default() {
        let r = ResourcePerfReport::default();
        assert_eq!(r.event_count(), 0);
    }

    #[test]
    fn i5_11_summary() {
        let s = stage7_i5_summary();
        assert!(s.contains("G1"));
        assert!(s.contains("K2"));
        assert!(s.contains("20"));
    }

    #[test]
    fn i5_12_healthy() {
        assert!(stage7_i5_healthy());
    }

    #[test]
    fn i5_13_to_g1_consistency() {
        assert!(stage7_i5_to_g1_consistency());
    }

    #[test]
    fn i5_14_to_k2_consistency() {
        assert!(stage7_i5_to_k2_consistency());
    }

    #[test]
    fn i5_15_coordinator_default() {
        let c = ResourcePerfCoordinator::default();
        assert_eq!(c.matrix.len(), 20);
    }
}
