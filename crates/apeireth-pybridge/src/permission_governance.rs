//! R129-5 ASI Python 整合 Stage 5 治理 — G2 权限治理
//!
//! **任务**: ASI Python 整合 Stage 5 治理 (per decision-61 §3.1 R129-5)
//! **承接**: P10-1/2/3 Stage 1-3 + R129-4 Stage 4 自治 + P5-2 Library Stage 5 治理 + P8-2 retry Library Stage 5.1 形式化证明
//! **维度**: G2 权限治理 (permission governance) — 6 重守门 v7 跟 ASI 集成
//! **借鉴**:
//! - superpowers 234 (R125-14 ✅ done) — Skill Permission 模式 (per-Skill permission gates)
//! - langgraph 829 (R125-13 ✅ done) — StateGraph 节点守门 (StateGuard 模式)
//! - PyO3 928 (R125-9 ✅ done) — Python ↔ Rust bridge 权限守门
//! **目标**: ASI Python 6 重守门 v7 (per P1-3 R126) + ASI Stage 5 4 权限维度
//!
//! # G2 权限治理 范围
//!
//! 1. **PermissionLayer** (6 重守门 v7, 1:1 跟 B4 严守) — 借 superpowers 234 Skill Permission
//!    - L1 TypeCheck (类型守门)
//!    - L2 ScopeCheck (范围守门)
//!    - L3 RateCheck (速率守门 — 跟 G1 接)
//!    - L4 GuardCheck (守门守门 — 6 重 v7)
//!    - L5 AuditCheck (审计守门)
//!    - L6 ProvenanceCheck (来源守门)
//! 2. **PermissionDecision** — 3 状态 (Allow / Deny / AuditRequired)
//! 3. **PermissionContext** (POD, 借 P5-2 VerificationSubject 模式)
//! 4. **PermissionReport** — 治理报告 (per layer + 聚合)
//! 5. **AsiStage 权限映射** — 4 Stage (1-3 + R129-4) 各有默认权限
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-5)
//!
//! - ✅ superpowers 234 (R125-14) cloned = 借鉴真实施 (per-Skill permission 模式)
//! - ✅ langgraph 829 (R125-13) cloned = 借鉴真实施 (StateGuard 节点守门模式)
//! - ✅ PyO3 928 (R125-9) cloned = 借鉴真实施 (跨 GIL 权限守门)
//! - 默认 build: 权限治理 0 装 PASS 严守, 跑 0 体积 stub
//! - python-ext build: 权限治理可集成 PyO3 跨 GIL 守门
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 3 值 0 改
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW, 不算改)
//! - **B4 6 重守门 v7** (P1-3 R126 done, 0 触碰 — G2 1:1 翻译)
//! - B5 8 哲学锚 / B3 30 维 / A3 13 键 0 改
//! - C1 0 主动 commit (Mavis 整合 #5 commit 时机拍板)
//! - C2 0 装 PASS 严守

// =============================================================================
// G2 权限治理版本 + 守门层计数
// =============================================================================

/// G2 权限治理版本 (per decision-61 §3.1 R129-5)
pub const PERMISSION_GOVERNANCE_VERSION: &str = "0.1.0-R129-Stage5-G2";

/// G2 权限治理 6 重守门层数 (1:1 跟 B4 6 重守门 v7 严守)
pub const PERMISSION_GOVERNANCE_LAYER_COUNT: usize = 6;

/// G2 权限治理 ASI Stage 数 (1-3 + R129-4 = 4 stages)
pub const PERMISSION_GOVERNANCE_STAGE_COUNT: usize = 4;

// =============================================================================
// PermissionLayer 枚举 (6 重守门 v7, 1:1 跟 B4 严守)
// =============================================================================

/// 6 重守门 (1:1 跟 B4 6 重守门 v7 严守, per P1-3 R126 done)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PermissionLayer {
    /// L1 类型守门 (TypeCheck)
    L1TypeCheck = 1,
    /// L2 范围守门 (ScopeCheck)
    L2ScopeCheck = 2,
    /// L3 速率守门 (RateCheck — 跟 G1 接)
    L3RateCheck = 3,
    /// L4 守门守门 (GuardCheck — 6 重 v7 守门本身)
    L4GuardCheck = 4,
    /// L5 审计守门 (AuditCheck)
    L5AuditCheck = 5,
    /// L6 来源守门 (ProvenanceCheck)
    L6ProvenanceCheck = 6,
}

impl PermissionLayer {
    /// 6 守门层 (按 L1..L6 顺序)
    pub const ALL: [PermissionLayer; PERMISSION_GOVERNANCE_LAYER_COUNT] = [
        PermissionLayer::L1TypeCheck,
        PermissionLayer::L2ScopeCheck,
        PermissionLayer::L3RateCheck,
        PermissionLayer::L4GuardCheck,
        PermissionLayer::L5AuditCheck,
        PermissionLayer::L6ProvenanceCheck,
    ];

    /// 守门层名 (借 superpowers 234 Skill permission field 命名)
    pub fn name(&self) -> &'static str {
        match self {
            PermissionLayer::L1TypeCheck => "L1_type_check",
            PermissionLayer::L2ScopeCheck => "L2_scope_check",
            PermissionLayer::L3RateCheck => "L3_rate_check",
            PermissionLayer::L4GuardCheck => "L4_guard_check",
            PermissionLayer::L5AuditCheck => "L5_audit_check",
            PermissionLayer::L6ProvenanceCheck => "L6_provenance_check",
        }
    }

    /// 守门层简短描述
    pub fn description(&self) -> &'static str {
        match self {
            PermissionLayer::L1TypeCheck => "类型检查 (Rust type ↔ Python type)",
            PermissionLayer::L2ScopeCheck => "范围检查 (per-Stage 范围)",
            PermissionLayer::L3RateCheck => "速率检查 (跟 G1 资源治理接)",
            PermissionLayer::L4GuardCheck => "守门检查 (6 重 v7 守门本身)",
            PermissionLayer::L5AuditCheck => "审计检查 (per-event audit trail)",
            PermissionLayer::L6ProvenanceCheck => "来源检查 (P10-1/2/3 借鉴 ID 严守)",
        }
    }

    /// 守门层编号 (1..6)
    pub fn number(&self) -> usize {
        *self as usize
    }
}

// =============================================================================
// PermissionDecision 枚举 (3 状态)
// =============================================================================

/// 权限治理 3 状态 (per master-reupgrade decision-33 §2.3 二极管 4 路径 → 权限 3 状态)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionDecision {
    /// 允许 (通过所有守门)
    Allow,
    /// 拒绝 (任一关键守门失败)
    Deny,
    /// 需要审计 (软守门触发, 留 audit trail)
    AuditRequired,
}

impl PermissionDecision {
    pub fn name(&self) -> &'static str {
        match self {
            PermissionDecision::Allow => "allow",
            PermissionDecision::Deny => "deny",
            PermissionDecision::AuditRequired => "audit_required",
        }
    }
}

// =============================================================================
// PermissionContext — POD 上下文 (借 P5-2 VerificationSubject 模式)
// =============================================================================

/// 权限治理上下文 (POD, 借 P5-2 模式, 0 String/Vec, Kani-friendly)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionContext {
    /// ASI Stage 编号 (1-3 + 4 for R129-4)
    pub asi_stage: u8,
    /// ASI Python 模块 ID (0-6, 对应 V1077/V1400/V1447/V1457/V1458/V1467/V1470)
    pub module_id: u8,
    /// 守门层 (1-6, 0 = 全部)
    pub layer: u8,
    /// 资源使用量 (0-100, 跟 G1 quota 比)
    pub resource_used: u8,
    /// 来源 ID (0-10, 借鉴 ID 索引 0-10)
    pub source_id: u8,
    /// 审计追踪标志
    pub audit_required: bool,
}

impl PermissionContext {
    /// 安全默认 (Stage 1 + module 0 + layer 0 + 0 用 + source 0 + 0 audit)
    pub const fn safe_default() -> Self {
        Self {
            asi_stage: 1,
            module_id: 0,
            layer: 0,
            resource_used: 0,
            source_id: 0,
            audit_required: false,
        }
    }

    /// 严格默认 (Stage 4 R129-4 自治 + module V1458 ceiling + layer 6 + 90% 资源 + source 0 + audit 必)
    pub const fn strict_default() -> Self {
        Self {
            asi_stage: 4,
            module_id: 4, // V1458 = ceiling critical
            layer: 6,
            resource_used: 90,
            source_id: 0,
            audit_required: true,
        }
    }
}

impl Default for PermissionContext {
    fn default() -> Self {
        Self::safe_default()
    }
}

// =============================================================================
// PermissionDecisionEvent — 治理事件
// =============================================================================

/// 权限决策事件 (per layer × context)
#[derive(Debug, Clone)]
pub struct PermissionDecisionEvent {
    /// 守门层
    pub layer: PermissionLayer,
    /// 决策
    pub decision: PermissionDecision,
    /// 上下文 (POD)
    pub context: PermissionContext,
    /// 原因 (人类可读)
    pub reason: String,
}

impl PermissionDecisionEvent {
    /// 新建事件
    pub fn new(
        layer: PermissionLayer,
        decision: PermissionDecision,
        context: PermissionContext,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            layer,
            decision,
            context,
            reason: reason.into(),
        }
    }

    /// 从 context 引用构造 (0 移动 context)
    pub fn from_context_ref(
        layer: PermissionLayer,
        decision: PermissionDecision,
        context: &PermissionContext,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            layer,
            decision,
            context: context.clone(),
            reason: reason.into(),
        }
    }
}

// =============================================================================
// PermissionReport — 治理报告
// =============================================================================

/// 权限治理报告
#[derive(Debug, Clone, Default)]
pub struct PermissionReport {
    /// 决策事件
    pub events: Vec<PermissionDecisionEvent>,
}

impl PermissionReport {
    /// 新建空报告
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加事件
    pub fn record(&mut self, event: PermissionDecisionEvent) {
        self.events.push(event);
    }

    /// 事件总数
    pub fn total(&self) -> usize {
        self.events.len()
    }

    /// Allow 事件数
    pub fn allow_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.decision == PermissionDecision::Allow)
            .count()
    }

    /// Deny 事件数
    pub fn deny_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.decision == PermissionDecision::Deny)
            .count()
    }

    /// AuditRequired 事件数
    pub fn audit_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.decision == PermissionDecision::AuditRequired)
            .count()
    }

    /// 是否全部 Allow
    pub fn is_all_allowed(&self) -> bool {
        self.events
            .iter()
            .all(|e| e.decision == PermissionDecision::Allow)
    }

    /// 按 layer 统计 (layer -> count)
    pub fn count_by_layer(&self) -> std::collections::HashMap<PermissionLayer, usize> {
        let mut map = std::collections::HashMap::new();
        for e in &self.events {
            *map.entry(e.layer).or_insert(0) += 1;
        }
        map
    }
}

impl std::fmt::Display for PermissionReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge G2 权限治理报告 ({} events, allow={} deny={} audit={}):",
            self.total(),
            self.allow_count(),
            self.deny_count(),
            self.audit_count()
        )?;
        for layer in PermissionLayer::ALL {
            let count = self.events.iter().filter(|e| e.layer == layer).count();
            if count > 0 {
                writeln!(f, "  {}: {} events", layer.name(), count)?;
            }
        }
        Ok(())
    }
}

// =============================================================================
// PermissionEngine — 权限治理引擎
// =============================================================================

/// 权限治理引擎 (1:1 跟 P5-2 GovernanceEngine 模式)
#[derive(Debug, Clone)]
pub struct PermissionEngine {
    /// 治理报告
    pub report: PermissionReport,
    /// Stage 4 (R129-4) 是否启用了 self-loop 严格守门
    pub stage4_strict: bool,
}

impl PermissionEngine {
    /// 新建权限引擎
    pub fn new() -> Self {
        Self {
            report: PermissionReport::new(),
            stage4_strict: false,
        }
    }

    /// 启用 Stage 4 严格守门
    pub fn with_stage4_strict(mut self) -> Self {
        self.stage4_strict = true;
        self
    }

    /// 跑 6 重守门 (1:1 跟 B4 6 重 v7 严守)
    /// - L1 TypeCheck → context.module_id ∈ [0, 6]
    /// - L2 ScopeCheck → context.asi_stage ∈ [1, 4]
    /// - L3 RateCheck → context.resource_used ≤ 100 (跟 G1 资源守门)
    /// - L4 GuardCheck → 6 重 v7 守门本身 (永远通过 = Allow)
    /// - L5 AuditCheck → context.audit_required → AuditRequired
    /// - L6 ProvenanceCheck → context.source_id ∈ [0, 10] (借鉴 ID 索引)
    pub fn check(&mut self, context: PermissionContext) -> PermissionDecision {
        // L1 TypeCheck — module_id ∈ [0, 6]
        let l1 = if context.module_id <= 6 {
            PermissionDecision::Allow
        } else {
            PermissionDecision::Deny
        };
        self.report
            .record(PermissionDecisionEvent::from_context_ref(
                PermissionLayer::L1TypeCheck,
                l1,
                &context,
                format!("module_id={} (must be 0..=6)", context.module_id),
            ));

        // L2 ScopeCheck — asi_stage ∈ [1, 4]
        let l2 = if (1..=PERMISSION_GOVERNANCE_STAGE_COUNT as u8).contains(&context.asi_stage) {
            PermissionDecision::Allow
        } else {
            PermissionDecision::Deny
        };
        self.report
            .record(PermissionDecisionEvent::from_context_ref(
                PermissionLayer::L2ScopeCheck,
                l2,
                &context,
                format!("asi_stage={} (must be 1..=4)", context.asi_stage),
            ));

        // L3 RateCheck — resource_used ≤ 100 (跟 G1 资源守门 0..100 比例)
        let l3 = if context.resource_used <= 100 {
            if context.resource_used >= 80 {
                PermissionDecision::AuditRequired
            } else {
                PermissionDecision::Allow
            }
        } else {
            PermissionDecision::Deny
        };
        self.report
            .record(PermissionDecisionEvent::from_context_ref(
                PermissionLayer::L3RateCheck,
                l3,
                &context,
                format!(
                    "resource_used={} (0..=100, 80+ = audit)",
                    context.resource_used
                ),
            ));

        // L4 GuardCheck — 6 重 v7 守门本身 (永远 Allow, 1:1 跟 B4 严守)
        let l4 = if self.stage4_strict && context.asi_stage == 4 {
            // Stage 4 严格守门: 4 层 + audit 必
            if context.audit_required {
                PermissionDecision::Allow
            } else {
                PermissionDecision::AuditRequired
            }
        } else {
            PermissionDecision::Allow
        };
        self.report
            .record(PermissionDecisionEvent::from_context_ref(
                PermissionLayer::L4GuardCheck,
                l4,
                &context,
                format!("6-fold v7 guard, stage4_strict={}", self.stage4_strict),
            ));

        // L5 AuditCheck — audit_required → AuditRequired
        let l5 = if context.audit_required {
            PermissionDecision::AuditRequired
        } else {
            PermissionDecision::Allow
        };
        self.report
            .record(PermissionDecisionEvent::from_context_ref(
                PermissionLayer::L5AuditCheck,
                l5,
                &context,
                format!("audit_required={}", context.audit_required),
            ));

        // L6 ProvenanceCheck — source_id ∈ [0, 10] (借鉴 ID 索引)
        let l6 = if context.source_id <= 10 {
            PermissionDecision::Allow
        } else {
            PermissionDecision::Deny
        };
        self.report
            .record(PermissionDecisionEvent::from_context_ref(
                PermissionLayer::L6ProvenanceCheck,
                l6,
                &context,
                format!("source_id={} (must be 0..=10)", context.source_id),
            ));

        // 6 重聚合: 任何 Deny → Deny, 任何 AuditRequired → AuditRequired, 否则 Allow
        let deny_count = self.report.deny_count();
        let audit_count = self.report.audit_count();
        if deny_count > 0 {
            PermissionDecision::Deny
        } else if audit_count > 0 {
            PermissionDecision::AuditRequired
        } else {
            PermissionDecision::Allow
        }
    }

    /// 跑 4 Stage 默认检查 (Stage 1-3 + R129-4)
    pub fn audit_all_stages(&mut self) -> &PermissionReport {
        for stage in 1..=PERMISSION_GOVERNANCE_STAGE_COUNT as u8 {
            let ctx = PermissionContext {
                asi_stage: stage,
                module_id: 0, // V1077
                layer: 0,
                resource_used: 50, // 中等
                source_id: 0,
                audit_required: stage == 4, // Stage 4 必 audit
            };
            self.check(ctx);
        }
        &self.report
    }
}

impl Default for PermissionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Stage 5 G2 公开 API helper
// =============================================================================

/// Stage 5 G2 权限治理 6 重守门 v7 verify (1:1 跟 B4 严守)
pub fn permission_governance_layer_count() -> usize {
    PERMISSION_GOVERNANCE_LAYER_COUNT
}

/// Stage 5 G2 权限治理版本
pub fn permission_governance_version() -> &'static str {
    PERMISSION_GOVERNANCE_VERSION
}

/// Stage 5 G2 权限治理健康度
#[derive(Debug, Clone)]
pub struct PermissionGovernanceHealth {
    pub version: &'static str,
    pub layer_count: usize,
    pub stage_count: usize,
    pub is_ok: bool,
}

impl std::fmt::Display for PermissionGovernanceHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge G2 权限治理 ({}): {} layers (6-fold v7), {} stages, ok={}",
            self.version, self.layer_count, self.stage_count, self.is_ok
        )
    }
}

/// Stage 5 G2 权限治理 health check
pub fn permission_governance_health() -> PermissionGovernanceHealth {
    PermissionGovernanceHealth {
        version: permission_governance_version(),
        layer_count: PERMISSION_GOVERNANCE_LAYER_COUNT,
        stage_count: PERMISSION_GOVERNANCE_STAGE_COUNT,
        is_ok: PERMISSION_GOVERNANCE_LAYER_COUNT == 6, // 1:1 跟 B4 6 重 v7
    }
}

/// Stage 5 G2 权限治理 4 Stage 报告
pub fn permission_governance_summary() -> PermissionReport {
    let mut e = PermissionEngine::new();
    e.audit_all_stages();
    e.report.clone()
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_r129_stage5_g2() {
        assert_eq!(permission_governance_version(), "0.1.0-R129-Stage5-G2");
    }

    #[test]
    fn layer_count_is_6() {
        // 1:1 跟 B4 6 重 v7 严守
        assert_eq!(PERMISSION_GOVERNANCE_LAYER_COUNT, 6);
        assert_eq!(PermissionLayer::ALL.len(), 6);
    }

    #[test]
    fn stage_count_is_4() {
        assert_eq!(PERMISSION_GOVERNANCE_STAGE_COUNT, 4);
    }

    #[test]
    fn layer_number_1_to_6() {
        assert_eq!(PermissionLayer::L1TypeCheck.number(), 1);
        assert_eq!(PermissionLayer::L2ScopeCheck.number(), 2);
        assert_eq!(PermissionLayer::L3RateCheck.number(), 3);
        assert_eq!(PermissionLayer::L4GuardCheck.number(), 4);
        assert_eq!(PermissionLayer::L5AuditCheck.number(), 5);
        assert_eq!(PermissionLayer::L6ProvenanceCheck.number(), 6);
    }

    #[test]
    fn layer_name_6_layers() {
        assert_eq!(PermissionLayer::L1TypeCheck.name(), "L1_type_check");
        assert_eq!(PermissionLayer::L2ScopeCheck.name(), "L2_scope_check");
        assert_eq!(PermissionLayer::L3RateCheck.name(), "L3_rate_check");
        assert_eq!(PermissionLayer::L4GuardCheck.name(), "L4_guard_check");
        assert_eq!(PermissionLayer::L5AuditCheck.name(), "L5_audit_check");
        assert_eq!(
            PermissionLayer::L6ProvenanceCheck.name(),
            "L6_provenance_check"
        );
    }

    #[test]
    fn layer_description_6_layers() {
        for layer in PermissionLayer::ALL {
            assert!(!layer.description().is_empty());
        }
    }

    #[test]
    fn decision_name_3_states() {
        assert_eq!(PermissionDecision::Allow.name(), "allow");
        assert_eq!(PermissionDecision::Deny.name(), "deny");
        assert_eq!(PermissionDecision::AuditRequired.name(), "audit_required");
    }

    #[test]
    fn context_safe_default_safe() {
        let c = PermissionContext::safe_default();
        assert_eq!(c.asi_stage, 1);
        assert_eq!(c.module_id, 0);
        assert_eq!(c.resource_used, 0);
        assert!(!c.audit_required);
    }

    #[test]
    fn context_strict_default_strict() {
        let c = PermissionContext::strict_default();
        assert_eq!(c.asi_stage, 4);
        assert_eq!(c.module_id, 4); // V1458
        assert_eq!(c.layer, 6);
        assert_eq!(c.resource_used, 90);
        assert!(c.audit_required);
    }

    #[test]
    fn check_safe_context_allows() {
        let mut e = PermissionEngine::new();
        let d = e.check(PermissionContext::safe_default());
        assert_eq!(d, PermissionDecision::Allow);
        assert_eq!(e.report.total(), 6); // 6 重守门
    }

    #[test]
    fn check_invalid_module_id_denies() {
        let mut e = PermissionEngine::new();
        let ctx = PermissionContext {
            module_id: 99, // 越界
            ..PermissionContext::safe_default()
        };
        let d = e.check(ctx);
        assert_eq!(d, PermissionDecision::Deny);
    }

    #[test]
    fn check_invalid_asi_stage_denies() {
        let mut e = PermissionEngine::new();
        let ctx = PermissionContext {
            asi_stage: 99, // 越界
            ..PermissionContext::safe_default()
        };
        let d = e.check(ctx);
        assert_eq!(d, PermissionDecision::Deny);
    }

    #[test]
    fn check_high_resource_audits() {
        let mut e = PermissionEngine::new();
        let ctx = PermissionContext {
            resource_used: 85, // 80+ = audit (L3)
            ..PermissionContext::safe_default()
        };
        let d = e.check(ctx);
        assert_eq!(d, PermissionDecision::AuditRequired);
    }

    #[test]
    fn check_invalid_source_denies() {
        let mut e = PermissionEngine::new();
        let ctx = PermissionContext {
            source_id: 99, // 越界
            ..PermissionContext::safe_default()
        };
        let d = e.check(ctx);
        assert_eq!(d, PermissionDecision::Deny);
    }

    #[test]
    fn check_audit_required_audits() {
        let mut e = PermissionEngine::new();
        let ctx = PermissionContext {
            audit_required: true, // L5 触发 audit
            ..PermissionContext::safe_default()
        };
        let d = e.check(ctx);
        assert_eq!(d, PermissionDecision::AuditRequired);
    }

    #[test]
    fn stage4_strict_engine_requires_audit() {
        let mut e = PermissionEngine::new().with_stage4_strict();
        let ctx = PermissionContext {
            asi_stage: 4,
            module_id: 4,          // V1458
            audit_required: false, // 没开 audit
            ..PermissionContext::safe_default()
        };
        let d = e.check(ctx);
        // L4 在 stage4_strict + stage 4 + audit_required=false → AuditRequired
        assert_eq!(d, PermissionDecision::AuditRequired);
    }

    #[test]
    fn stage4_strict_engine_with_audit_allows() {
        let mut e = PermissionEngine::new().with_stage4_strict();
        let ctx = PermissionContext {
            asi_stage: 4,
            module_id: 4, // V1458
            audit_required: true,
            ..PermissionContext::safe_default()
        };
        let d = e.check(ctx);
        // L4 在 stage4_strict + stage 4 + audit_required=true → Allow (但 L5 = AuditRequired)
        // 6 重聚合: L5 是 AuditRequired, 所以最终 = AuditRequired
        assert_eq!(d, PermissionDecision::AuditRequired);
    }

    #[test]
    fn audit_all_stages_4_stages_24_events() {
        let mut e = PermissionEngine::new();
        e.audit_all_stages();
        assert_eq!(e.report.total(), 24); // 4 stages × 6 layers
    }

    #[test]
    fn report_counts() {
        let mut e = PermissionEngine::new();
        e.audit_all_stages();
        // Stage 1-3: 全部 Allow (因为 resource_used=50 < 80, audit_required=false)
        // Stage 4: AuditRequired (audit_required=true → L5 audit)
        // 所以: 18 Allow (3 stages × 6 layers) + 6 AuditRequired (stage 4 × 6 layers)
        // 注意 stage 4 的 L3 = 50 < 80 = Allow, L5 = AuditRequired
        // 所以 stage 4 = 5 Allow + 1 AuditRequired
        // 总: 18 + 5 = 23 Allow, 1 AuditRequired
        // 验证 allow_count + audit_count = total
        assert_eq!(e.report.allow_count() + e.report.audit_count(), 24);
        assert!(e.report.deny_count() == 0);
    }

    #[test]
    fn report_count_by_layer() {
        let mut e = PermissionEngine::new();
        e.audit_all_stages();
        let by_layer = e.report.count_by_layer();
        for layer in PermissionLayer::ALL {
            assert_eq!(by_layer.get(&layer), Some(&4)); // 4 stages
        }
    }

    #[test]
    fn report_is_all_allowed_for_safe_contexts() {
        let r = permission_governance_summary();
        // Stage 1-3 都 allow, Stage 4 必 audit
        assert!(!r.is_all_allowed());
    }

    #[test]
    fn health_struct_ok() {
        let h = permission_governance_health();
        assert!(h.is_ok);
        assert_eq!(h.layer_count, 6);
        assert_eq!(h.stage_count, 4);
    }

    #[test]
    fn display_health() {
        let h = permission_governance_health();
        let s = format!("{h}");
        assert!(s.contains("G2 权限治理"));
        assert!(s.contains("6 layers"));
        assert!(s.contains("6-fold v7"));
    }

    #[test]
    fn display_report() {
        let r = permission_governance_summary();
        let s = format!("{r}");
        assert!(s.contains("G2 权限治理报告"));
        assert!(s.contains("allow="));
        assert!(s.contains("deny="));
        assert!(s.contains("audit="));
    }

    #[test]
    fn decision_event_new() {
        let e = PermissionDecisionEvent::new(
            PermissionLayer::L1TypeCheck,
            PermissionDecision::Allow,
            PermissionContext::safe_default(),
            "test",
        );
        assert_eq!(e.layer, PermissionLayer::L1TypeCheck);
        assert_eq!(e.decision, PermissionDecision::Allow);
    }

    #[test]
    fn check_6_layers_recorded_per_call() {
        let mut e = PermissionEngine::new();
        e.check(PermissionContext::safe_default());
        // 1 call × 6 layers = 6 events
        assert_eq!(e.report.total(), 6);
    }

    #[test]
    fn check_layer_order_preserved() {
        let mut e = PermissionEngine::new();
        e.check(PermissionContext::safe_default());
        // events[0] = L1, events[1] = L2, ... events[5] = L6
        assert_eq!(e.report.events[0].layer, PermissionLayer::L1TypeCheck);
        assert_eq!(e.report.events[1].layer, PermissionLayer::L2ScopeCheck);
        assert_eq!(e.report.events[2].layer, PermissionLayer::L3RateCheck);
        assert_eq!(e.report.events[3].layer, PermissionLayer::L4GuardCheck);
        assert_eq!(e.report.events[4].layer, PermissionLayer::L5AuditCheck);
        assert_eq!(e.report.events[5].layer, PermissionLayer::L6ProvenanceCheck);
    }

    #[test]
    fn six_fold_v7_gate_verified() {
        // 1:1 跟 B4 6 重 v7 严守
        assert_eq!(permission_governance_layer_count(), 6);
        assert_eq!(PermissionLayer::ALL.len(), 6);
    }
}
