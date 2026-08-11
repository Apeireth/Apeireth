//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I1 D1+G1 工具+资源集成
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **承接**: R129-4 Stage 4 自治 (D1 工具自循环) + R129-5 Stage 5 治理 (G1 资源治理) + R129-6 Stage 6 守护 (K1-K4 守护)
//! **维度**: I1 D1+G1 — 工具调用自循环 (D1) 跟资源治理 (G1) 跨 stage 集成
//! **目标**: ASI 工具调用遵守资源配额, 工具调用 + 资源限制互锁
//!
//! # I1 集成范围
//!
//! 1. **ToolResourceBinding** — 单个 ASI 工具跟 4 资源维度配额的绑定
//!    - 1 个工具 ID + 1 个资源维度 + 1 个 quota 档位
//! 2. **ToolResourceMatrix** — 工具 × 资源 矩阵
//!    - 5 default 工具 (per D1) × 4 资源维度 (per G1) = 20 绑定 (编译期 hardcode)
//! 3. **ToolResourceAuditEvent** — 集成审计事件
//!    - timestamp + tool_id + dimension + action + reason
//! 4. **ToolResourceReport** — 集成报告 (聚合 events)
//! 5. **ToolResourceCoordinator** — 顶层协调器
//!    - check_and_call(tool_id, dimension) — 调用工具时 verify 资源配额
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-18)
//!
//! - ✅ D1 工具调用自循环 (R129-4) cloned = 真借 D1 公共 API
//! - ✅ G1 资源治理 (R129-5) cloned = 真借 G1 ResourceQuota
//! - 默认 build: I1 集成 0 装 PASS 严守, 跑 0 体积 stub
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 3 值 0 改
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW)
//! - B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 / A3 13 键 0 改
//! - C1 0 主动 commit (Mavis 整合 #5 commit 时机拍板)
//! - C2 0 装 PASS 严守

use crate::resource_governance::{
    GovernanceAction, ResourceDimension, ResourceQuota, RESOURCE_GOVERNANCE_DIMENSION_COUNT,
    RESOURCE_GOVERNANCE_MODULE_COUNT,
};
use crate::tool_self_loop::{
    tool_self_loop_summary, ToolRegistry, DEFAULT_TOOL_COUNT, TOOL_SELF_LOOP_MAX_DEPTH,
};

// =============================================================================
// I1 集成版本 + 计数
// =============================================================================

/// I1 工具+资源集成版本 (per decision-61 §3.1 R129-18)
pub const STAGE7_I1_VERSION: &str = "0.1.0-R129-Stage7-I1";

/// I1 集成维度数 (2: D1 工具 + G1 资源)
pub const STAGE7_I1_DIMENSION_COUNT: usize = 2;

/// I1 工具×资源绑定数 (5 default tool × 4 resource dimension = 20 绑定, 编译期 hardcode)
pub const STAGE7_I1_BINDING_COUNT: usize = DEFAULT_TOOL_COUNT * RESOURCE_GOVERNANCE_DIMENSION_COUNT;

/// I1 默认 quota 档位 (Default, 1:1 跟 G1 ResourceQuota::default_const)
pub const STAGE7_I1_DEFAULT_QUOTA: &str = "default";

// =============================================================================
// ToolResourceBinding — 单个工具 × 资源绑定
// =============================================================================

/// 1 个工具 × 1 个资源维度的绑定
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolResourceBinding {
    /// 工具 ID (per D1 ToolRegistry)
    pub tool_id: String,
    /// 资源维度 (per G1 ResourceDimension)
    pub dimension: ResourceDimension,
    /// quota 档位 ("default" / "strict" / "relaxed" / "unlimited")
    pub quota_label: String,
}

impl ToolResourceBinding {
    /// 新建绑定
    pub fn new(tool_id: &str, dimension: ResourceDimension, quota_label: &str) -> Self {
        Self {
            tool_id: tool_id.to_string(),
            dimension,
            quota_label: quota_label.to_string(),
        }
    }

    /// 默认绑定 (D1 + G1 协同默认)
    pub fn default_binding(tool_id: &str, dimension: ResourceDimension) -> Self {
        Self::new(tool_id, dimension, STAGE7_I1_DEFAULT_QUOTA)
    }
}

// =============================================================================
// ToolResourceMatrix — 工具 × 资源 矩阵
// =============================================================================

/// 工具 × 资源 矩阵 (5 tool × 4 dim = 20 绑定, 编译期 hardcode)
#[derive(Debug, Clone, Default)]
pub struct ToolResourceMatrix {
    /// 绑定集合 (按 tool_id 字母序 + dim 字母序排列)
    bindings: Vec<ToolResourceBinding>,
}

impl ToolResourceMatrix {
    /// 默认矩阵: 5 default tool × 4 dim = 20 绑定, 全 default quota
    pub fn default_matrix() -> Self {
        let registry = ToolRegistry::with_default_tools();
        let mut bindings = Vec::with_capacity(STAGE7_I1_BINDING_COUNT);
        for tool_id in registry.ids() {
            for &dim in &ResourceDimension::ALL {
                bindings.push(ToolResourceBinding::default_binding(&tool_id, dim));
            }
        }
        Self { bindings }
    }

    /// 绑定数
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// 是否空
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// 查 tool_id + dim 的绑定
    pub fn get(&self, tool_id: &str, dim: ResourceDimension) -> Option<&ToolResourceBinding> {
        self.bindings
            .iter()
            .find(|b| b.tool_id == tool_id && b.dimension == dim)
    }

    /// 所有绑定
    pub fn bindings(&self) -> &[ToolResourceBinding] {
        &self.bindings
    }

    /// 唯一 tool_id 列表 (per D1 default 工具)
    pub fn tool_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .bindings
            .iter()
            .map(|b| b.tool_id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        ids.sort();
        ids
    }
}

// =============================================================================
// ToolResourceAuditEvent — 集成审计事件
// =============================================================================

/// 1 个集成审计事件
#[derive(Debug, Clone)]
pub struct ToolResourceAuditEvent {
    /// 时间戳 (ms since epoch 或单测用 0-based)
    pub timestamp: u64,
    /// 工具 ID
    pub tool_id: String,
    /// 资源维度
    pub dimension: ResourceDimension,
    /// 治理动作 (per G1)
    pub action: GovernanceAction,
    /// 决策原因 (人类可读)
    pub reason: String,
}

impl ToolResourceAuditEvent {
    /// 新建事件
    pub fn new(
        timestamp: u64,
        tool_id: &str,
        dimension: ResourceDimension,
        action: GovernanceAction,
        reason: &str,
    ) -> Self {
        Self {
            timestamp,
            tool_id: tool_id.to_string(),
            dimension,
            action,
            reason: reason.to_string(),
        }
    }
}

// =============================================================================
// ToolResourceReport — 集成报告
// =============================================================================

/// 集成报告 (聚合 events)
#[derive(Debug, Clone, Default)]
pub struct ToolResourceReport {
    /// 审计事件 (按 timestamp 升序)
    pub events: Vec<ToolResourceAuditEvent>,
    /// 绑定矩阵快照
    pub matrix: ToolResourceMatrix,
}

impl ToolResourceReport {
    /// 事件数
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Allow 数
    pub fn allow_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e.action, GovernanceAction::Allow))
            .count()
    }

    /// Throttle 数
    pub fn throttle_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e.action, GovernanceAction::Throttle))
            .count()
    }

    /// Reject 数
    pub fn reject_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e.action, GovernanceAction::Reject))
            .count()
    }
}

// =============================================================================
// ToolResourceCoordinator — 顶层协调器
// =============================================================================

/// I1 顶层协调器: 工具调用 + 资源治理 跨 stage 集成
#[derive(Debug, Clone)]
pub struct ToolResourceCoordinator {
    /// 绑定矩阵
    pub matrix: ToolResourceMatrix,
    /// 报告
    pub report: ToolResourceReport,
}

impl ToolResourceCoordinator {
    /// 新建协调器
    pub fn new() -> Self {
        let matrix = ToolResourceMatrix::default_matrix();
        let report = ToolResourceReport {
            events: Vec::new(),
            matrix: matrix.clone(),
        };
        Self { matrix, report }
    }

    /// check_and_call: 工具调用时 verify 资源配额
    /// 简化: 默认都 Allow, 实际场景可基于 G1 配额动态决策
    pub fn check_and_call(
        &mut self,
        timestamp: u64,
        tool_id: &str,
        dimension: ResourceDimension,
    ) -> GovernanceAction {
        let binding = self.matrix.get(tool_id, dimension);
        let (action, reason) = match binding {
            Some(b) if b.quota_label == "default" => {
                (GovernanceAction::Allow, "default quota 允许调用".to_string())
            }
            Some(b) if b.quota_label == "strict" => {
                (GovernanceAction::Throttle, "strict quota 限流".to_string())
            }
            Some(b) if b.quota_label == "unlimited" => {
                (GovernanceAction::Allow, "unlimited quota 允许".to_string())
            }
            Some(_) => (GovernanceAction::Allow, "relaxed quota 允许".to_string()),
            None => (
                GovernanceAction::Reject,
                format!("no binding for tool={tool_id} dim={dimension:?}"),
            ),
        };
        let ev = ToolResourceAuditEvent::new(timestamp, tool_id, dimension, action, &reason);
        self.report.events.push(ev);
        action
    }
}

impl Default for ToolResourceCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// I1 公共 API
// =============================================================================

/// I1 集成摘要
pub fn stage7_i1_summary() -> String {
    format!(
        "I1 D1+G1 工具+资源集成 v{} ({} dim, {} bindings, {} default tools × {} resource dims)",
        STAGE7_I1_VERSION,
        STAGE7_I1_DIMENSION_COUNT,
        STAGE7_I1_BINDING_COUNT,
        DEFAULT_TOOL_COUNT,
        RESOURCE_GOVERNANCE_DIMENSION_COUNT,
    )
}

/// I1 集成健康度
pub fn stage7_i1_healthy() -> bool {
    let coord = ToolResourceCoordinator::new();
    let matrix = &coord.matrix;
    matrix.len() == STAGE7_I1_BINDING_COUNT
        && matrix.tool_ids().len() == DEFAULT_TOOL_COUNT
        && STAGE7_I1_BINDING_COUNT == DEFAULT_TOOL_COUNT * RESOURCE_GOVERNANCE_DIMENSION_COUNT
        && RESOURCE_GOVERNANCE_MODULE_COUNT >= DEFAULT_TOOL_COUNT
}

/// I1 + D1 协同 verify (D1 公共 API + I1 矩阵)
pub fn stage7_i1_to_d1_consistency() -> bool {
    let d1_summary = tool_self_loop_summary();
    let i1_matrix = ToolResourceMatrix::default_matrix();
    // D1 公共 API 应提到 max_depth 守门
    d1_summary.contains("max_depth")
        && i1_matrix.len() == STAGE7_I1_BINDING_COUNT
        && TOOL_SELF_LOOP_MAX_DEPTH >= 1
}

/// I1 + G1 协同 verify (G1 ResourceQuota + I1 矩阵)
pub fn stage7_i1_to_g1_consistency() -> bool {
    let default_quota = ResourceQuota::default_const();
    let i1_matrix = ToolResourceMatrix::default_matrix();
    // G1 ResourceQuota::default_const() 4 维度 (per Stage 5: rate=100, memory=64MB, time=5s, count=8)
    let rate = default_quota.rate_per_sec;
    let memory = default_quota.memory_bytes;
    let time = default_quota.time_ms;
    let count = default_quota.count_max;
    // I1 矩阵 5 tool × 4 dim 完整
    i1_matrix.len() == STAGE7_I1_BINDING_COUNT
        && rate == 100
        && memory == 64 * 1024 * 1024
        && time == 5_000
        && count == 8
}

// =============================================================================
// I1 inline unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 1. I1 编译期常数
    #[test]
    fn i1_01_version_constant() {
        assert_eq!(STAGE7_I1_VERSION, "0.1.0-R129-Stage7-I1");
        assert_eq!(STAGE7_I1_DIMENSION_COUNT, 2);
        assert_eq!(STAGE7_I1_BINDING_COUNT, 20); // 5 × 4
        assert_eq!(STAGE7_I1_DEFAULT_QUOTA, "default");
    }

    // 2. I1 绑定 5 tool × 4 dim = 20
    #[test]
    fn i1_02_matrix_default_20_bindings() {
        let m = ToolResourceMatrix::default_matrix();
        assert_eq!(m.len(), 20);
        assert_eq!(m.tool_ids().len(), 5);
    }

    // 3. I1 查 tool_id + dim
    #[test]
    fn i1_03_matrix_get_binding() {
        let m = ToolResourceMatrix::default_matrix();
        let binding = m.get("executor", ResourceDimension::Rate);
        assert!(binding.is_some());
        assert_eq!(binding.unwrap().quota_label, "default");
    }

    // 4. I1 查不存在的 binding 返回 None
    #[test]
    fn i1_04_matrix_get_unknown_tool() {
        let m = ToolResourceMatrix::default_matrix();
        let binding = m.get("nonexistent", ResourceDimension::Rate);
        assert!(binding.is_none());
    }

    // 5. I1 ToolResourceBinding 字段
    #[test]
    fn i1_05_binding_fields() {
        let b = ToolResourceBinding::new("tool.test", ResourceDimension::Memory, "strict");
        assert_eq!(b.tool_id, "tool.test");
        assert_eq!(b.dimension, ResourceDimension::Memory);
        assert_eq!(b.quota_label, "strict");
    }

    // 6. I1 ToolResourceBinding::default_binding
    #[test]
    fn i1_06_default_binding() {
        let b = ToolResourceBinding::default_binding("tool.x", ResourceDimension::Time);
        assert_eq!(b.quota_label, "default");
    }

    // 7. I1 协调器 default quota Allow
    #[test]
    fn i1_07_coordinator_default_allow() {
        let mut c = ToolResourceCoordinator::new();
        let action = c.check_and_call(0, "executor", ResourceDimension::Rate);
        assert_eq!(action, GovernanceAction::Allow);
        assert_eq!(c.report.event_count(), 1);
        assert_eq!(c.report.allow_count(), 1);
    }

    // 8. I1 协调器 unknown tool Reject
    #[test]
    fn i1_08_coordinator_unknown_reject() {
        let mut c = ToolResourceCoordinator::new();
        let action = c.check_and_call(0, "nonexistent", ResourceDimension::Rate);
        assert_eq!(action, GovernanceAction::Reject);
        assert_eq!(c.report.reject_count(), 1);
    }

    // 9. I1 ToolResourceAuditEvent 字段
    #[test]
    fn i1_09_audit_event_fields() {
        let ev = ToolResourceAuditEvent::new(
            1234,
            "tool.test",
            ResourceDimension::Count,
            GovernanceAction::Throttle,
            "over count",
        );
        assert_eq!(ev.timestamp, 1234);
        assert_eq!(ev.tool_id, "tool.test");
        assert_eq!(ev.dimension, ResourceDimension::Count);
        assert_eq!(ev.action, GovernanceAction::Throttle);
    }

    // 10. I1 报告 0 事件默认状态
    #[test]
    fn i1_10_report_default() {
        let r = ToolResourceReport::default();
        assert_eq!(r.event_count(), 0);
        assert_eq!(r.allow_count(), 0);
        assert_eq!(r.throttle_count(), 0);
        assert_eq!(r.reject_count(), 0);
    }

    // 11. I1 公共 API summary
    #[test]
    fn i1_11_summary_mentions_d1_g1() {
        let s = stage7_i1_summary();
        assert!(s.contains("D1"));
        assert!(s.contains("G1"));
        assert!(s.contains("20"));
    }

    // 12. I1 公共 API healthy
    #[test]
    fn i1_12_healthy_true() {
        assert!(stage7_i1_healthy());
    }

    // 13. I1 跟 D1 协同
    #[test]
    fn i1_13_to_d1_consistency() {
        assert!(stage7_i1_to_d1_consistency());
    }

    // 14. I1 跟 G1 协同
    #[test]
    fn i1_14_to_g1_consistency() {
        assert!(stage7_i1_to_g1_consistency());
    }

    // 15. I1 协调器默认构造
    #[test]
    fn i1_15_coordinator_default() {
        let c = ToolResourceCoordinator::default();
        assert_eq!(c.matrix.len(), 20);
        assert_eq!(c.report.event_count(), 0);
    }
}
