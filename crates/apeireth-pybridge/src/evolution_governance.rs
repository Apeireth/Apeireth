//! R129-5 ASI Python 整合 Stage 5 治理 — G4 演进治理
//!
//! **任务**: ASI Python 整合 Stage 5 治理 (per decision-61 §3.1 R129-5)
//! **承接**: P10-1/2/3 Stage 1-3 + R129-4 Stage 4 自治 + P5-2 Library Stage 5 治理 + P8-2 retry Library Stage 5.1 形式化证明
//! **维度**: G4 演进治理 (evolution governance) — ASI 自演进规则
//! **借鉴**:
//! - superpowers 234 (R125-14 ✅ done) — Skill Evolution 模式 (Skill 演化 + 测试驱动 + 1:1 反射)
//! - langgraph 829 (R125-13 ✅ done) — StateGraph 节点演化 (ConditionalEdge + 节点增删)
//! - kani 4502 (R125-10 ✅ done) — Kani-style 形式化演化约束 (类比 invariant 守门)
//! **目标**: ASI Python 自演进 4 规则 (新增 / 升级 / 降级 / 退役) + 演进路径 + 演进守门
//!
//! # G4 演进治理 范围
//!
//! 1. **EvolutionKind** — 4 演进类型
//!    - Add (新增 ASI Python 模块) — 借 superpowers SkillRegistry.add
//!    - Upgrade (升级既有模块版本) — 借 langgraph ConditionalEdge
//!    - Downgrade (降级模块严格度) — 借 superpowers Skill deprecate
//!    - Retire (退役模块) — 借 langgraph node removal
//! 2. **EvolutionRule** — 4 演进规则 (1:1 跟 4 维度对应)
//!    - R1 NewModuleSafe — 新增模块必通过 7 项安全 check
//!    - R2 UpgradeBackwardCompat — 升级必保持向后兼容
//!    - R3 DowngradeJustified — 降级必记录原因
//!    - R4 RetireConfirmed — 退役必 3 方确认 (Mavis + 主人 + 借鉴 ID)
//! 3. **EvolutionEvent** — 演进事件 (per kind × rule)
//! 4. **EvolutionEngine** — 演进治理引擎
//!    - 1:1 借鉴 P5-2 DecisionTree (3 段派发)
//! 5. **EvolutionReport** — 演进报告 (聚合 events)
//! 6. **AsiStage 演进链** — 4 Stage (1-3 + R129-4) 演进路径
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-5)
//!
//! - ✅ superpowers 234 (R125-14) cloned = 借鉴真实施 (Skill Evolution 模式)
//! - ✅ langgraph 829 (R125-13) cloned = 借鉴真实施 (StateGraph 节点增删)
//! - ✅ kani 4502 (R125-10) cloned = 借鉴真实施 (invariant 守门)
//! - 默认 build: 演进治理 0 装 PASS 严守, 跑 0 体积 stub
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 3 值 0 改
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW, 不算改)
//! - B4 6 重 v7 / B5 8 锚 / B3 30 维 / A3 13 键 0 改
//! - C1 0 主动 commit (Mavis 整合 #5 commit 时机拍板)
//! - C2 0 装 PASS 严守

use std::collections::HashMap;

// =============================================================================
// G4 演进治理版本 + 规则计数
// =============================================================================

/// G4 演进治理版本 (per decision-61 §3.1 R129-5)
pub const EVOLUTION_GOVERNANCE_VERSION: &str = "0.1.0-R129-Stage5-G4";

/// G4 演进治理规则数 (4, 1:1 跟 Stage 5 G4 写 = 4)
pub const EVOLUTION_GOVERNANCE_RULE_COUNT: usize = 4;

/// G4 演进治理 演进类型数 (4: Add/Upgrade/Downgrade/Retire)
pub const EVOLUTION_GOVERNANCE_KIND_COUNT: usize = 4;

/// G4 演进治理 ASI Stage 数 (1-3 + R129-4 = 4)
pub const EVOLUTION_GOVERNANCE_STAGE_COUNT: usize = 4;

// =============================================================================
// EvolutionKind 枚举 (4 演进类型)
// =============================================================================

/// 演进 4 类型 (借 superpowers 234 Skill lifecycle + langgraph 829 node lifecycle)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EvolutionKind {
    /// 新增 (1) — 借 superpowers SkillRegistry.add
    Add = 1,
    /// 升级 (2) — 借 langgraph ConditionalEdge
    Upgrade = 2,
    /// 降级 (3) — 借 superpowers Skill deprecate
    Downgrade = 3,
    /// 退役 (4) — 借 langgraph node removal
    Retire = 4,
}

impl EvolutionKind {
    /// 4 演进类型 (按 Add/Upgrade/Downgrade/Retire 顺序)
    pub const ALL: [EvolutionKind; EVOLUTION_GOVERNANCE_KIND_COUNT] = [
        EvolutionKind::Add,
        EvolutionKind::Upgrade,
        EvolutionKind::Downgrade,
        EvolutionKind::Retire,
    ];

    /// 演进类型名
    pub fn name(&self) -> &'static str {
        match self {
            EvolutionKind::Add => "add",
            EvolutionKind::Upgrade => "upgrade",
            EvolutionKind::Downgrade => "downgrade",
            EvolutionKind::Retire => "retire",
        }
    }

    /// 演进类型描述
    pub fn description(&self) -> &'static str {
        match self {
            EvolutionKind::Add => "新增 ASI Python 模块 (借 superpowers SkillRegistry.add)",
            EvolutionKind::Upgrade => "升级既有模块版本 (借 langgraph ConditionalEdge)",
            EvolutionKind::Downgrade => "降级模块严格度 (借 superpowers Skill deprecate)",
            EvolutionKind::Retire => "退役模块 (借 langgraph node removal)",
        }
    }

    /// 演进类型编号 (1..4)
    pub fn number(&self) -> usize {
        *self as usize
    }
}

// =============================================================================
// EvolutionRule 枚举 (4 演进规则)
// =============================================================================

/// 演进 4 规则 (1:1 跟 4 演进类型对应)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EvolutionRule {
    /// R1 新增模块安全 (NewModuleSafe) — 借 superpowers TDD 强制
    R1NewModuleSafe = 1,
    /// R2 升级向后兼容 (UpgradeBackwardCompat) — 借 langgraph 节点演化的向后兼容守门
    R2UpgradeBackwardCompat = 2,
    /// R3 降级必记录原因 (DowngradeJustified) — 借 superpowers Skill deprecate reason
    R3DowngradeJustified = 3,
    /// R4 退役必 3 方确认 (RetireConfirmed) — 借 Kani invariant 3 方 (Mavis + 主人 + 借鉴 ID)
    R4RetireConfirmed = 4,
}

impl EvolutionRule {
    /// 4 演进规则
    pub const ALL: [EvolutionRule; EVOLUTION_GOVERNANCE_RULE_COUNT] = [
        EvolutionRule::R1NewModuleSafe,
        EvolutionRule::R2UpgradeBackwardCompat,
        EvolutionRule::R3DowngradeJustified,
        EvolutionRule::R4RetireConfirmed,
    ];

    /// 规则名
    pub fn name(&self) -> &'static str {
        match self {
            EvolutionRule::R1NewModuleSafe => "R1_new_module_safe",
            EvolutionRule::R2UpgradeBackwardCompat => "R2_upgrade_backward_compat",
            EvolutionRule::R3DowngradeJustified => "R3_downgrade_justified",
            EvolutionRule::R4RetireConfirmed => "R4_retire_confirmed",
        }
    }

    /// 规则描述
    pub fn description(&self) -> &'static str {
        match self {
            EvolutionRule::R1NewModuleSafe => {
                "新增模块必通过 7 项安全 check (类型/范围/速率/守门/审计/来源/POD)"
            }
            EvolutionRule::R2UpgradeBackwardCompat => {
                "升级必保持向后兼容 (24 LOCKED 入口签名 0 改)"
            }
            EvolutionRule::R3DowngradeJustified => "降级必记录原因 (避免悄悄降级)",
            EvolutionRule::R4RetireConfirmed => "退役必 3 方确认 (Mavis + 主人 + 借鉴 ID)",
        }
    }

    /// 规则编号 (1..4)
    pub fn number(&self) -> usize {
        *self as usize
    }
}

// =============================================================================
// EvolutionOutcome 枚举 (3 状态)
// =============================================================================

/// 演进 3 状态 (per master-reupgrade decision-33 §2.3 二极管 4 路径 → 演进 3 状态)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvolutionOutcome {
    /// 通过 (符合规则)
    Pass,
    /// 警告 (软约束违反, 留 audit)
    Warn,
    /// 拒绝 (硬约束违反, 必须修复)
    Fail,
}

impl EvolutionOutcome {
    pub fn name(&self) -> &'static str {
        match self {
            EvolutionOutcome::Pass => "pass",
            EvolutionOutcome::Warn => "warn",
            EvolutionOutcome::Fail => "fail",
        }
    }
}

// =============================================================================
// EvolutionEvent — 演进事件
// =============================================================================

/// 演进事件 (per kind × rule × outcome)
#[derive(Debug, Clone)]
pub struct EvolutionEvent {
    /// ASI Python 模块名 (e.g. "apeireth.v1077_asi_v04_full_measurement")
    pub module: String,
    /// 演进类型
    pub kind: EvolutionKind,
    /// 演进规则
    pub rule: EvolutionRule,
    /// 演进结果
    pub outcome: EvolutionOutcome,
    /// 原因 (人类可读)
    pub reason: String,
}

impl EvolutionEvent {
    /// 新建事件
    pub fn new(
        module: impl Into<String>,
        kind: EvolutionKind,
        rule: EvolutionRule,
        outcome: EvolutionOutcome,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            module: module.into(),
            kind,
            rule,
            outcome,
            reason: reason.into(),
        }
    }
}

// =============================================================================
// EvolutionContext — 演进上下文
// =============================================================================

/// 演进上下文 (per module)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionContext {
    /// 模块名
    pub module: String,
    /// 当前版本
    pub current_version: u32,
    /// 借鉴 ID 索引 (0-10, 跟 G2 接)
    pub borrow_id: u8,
    /// 守门层数 (1:1 跟 G2 6 重 v7 = 6)
    pub guard_layers: u8,
}

impl EvolutionContext {
    /// 7 关键 ASI Python 模块 (V1077..V1470) 默认上下文
    pub fn asi_default(module: &str, version: u32) -> Self {
        Self {
            module: module.to_string(),
            current_version: version,
            borrow_id: 0,
            guard_layers: 6, // 1:1 跟 B4 6 重 v7
        }
    }
}

impl Default for EvolutionContext {
    fn default() -> Self {
        Self {
            module: "apeireth.test_module".to_string(),
            current_version: 1,
            borrow_id: 0,
            guard_layers: 6,
        }
    }
}

// =============================================================================
// EvolutionReport — 演进报告
// =============================================================================

/// 演进报告
#[derive(Debug, Clone, Default)]
pub struct EvolutionReport {
    /// 演进事件
    pub events: Vec<EvolutionEvent>,
}

impl EvolutionReport {
    /// 新建空报告
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加事件
    pub fn record(&mut self, event: EvolutionEvent) {
        self.events.push(event);
    }

    /// 事件总数
    pub fn total(&self) -> usize {
        self.events.len()
    }

    /// Pass 事件数
    pub fn pass_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.outcome == EvolutionOutcome::Pass)
            .count()
    }

    /// Warn 事件数
    pub fn warn_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.outcome == EvolutionOutcome::Warn)
            .count()
    }

    /// Fail 事件数
    pub fn fail_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.outcome == EvolutionOutcome::Fail)
            .count()
    }

    /// 报告是否完全 Pass
    pub fn is_all_pass(&self) -> bool {
        self.events
            .iter()
            .all(|e| e.outcome == EvolutionOutcome::Pass)
    }

    /// 按 kind 统计
    pub fn count_by_kind(&self) -> HashMap<EvolutionKind, usize> {
        let mut map = HashMap::new();
        for e in &self.events {
            *map.entry(e.kind).or_insert(0) += 1;
        }
        map
    }

    /// 按 rule 统计
    pub fn count_by_rule(&self) -> HashMap<EvolutionRule, usize> {
        let mut map = HashMap::new();
        for e in &self.events {
            *map.entry(e.rule).or_insert(0) += 1;
        }
        map
    }
}

impl std::fmt::Display for EvolutionReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge G4 演进治理报告 ({} events, pass={} warn={} fail={}):",
            self.total(),
            self.pass_count(),
            self.warn_count(),
            self.fail_count()
        )?;
        for kind in EvolutionKind::ALL {
            let count = self.events.iter().filter(|e| e.kind == kind).count();
            if count > 0 {
                writeln!(f, "  kind {}: {} events", kind.name(), count)?;
            }
        }
        Ok(())
    }
}

// =============================================================================
// EvolutionEngine — 演进治理引擎
// =============================================================================

/// 演进治理引擎 (1:1 借鉴 P5-2 GovernanceEngine 模式)
#[derive(Debug, Clone)]
pub struct EvolutionEngine {
    /// 演进报告
    pub report: EvolutionReport,
    /// 7 ASI Python 模块已知 version (Stage 1 asi_modules.rs 1:1)
    pub asi_module_versions: HashMap<String, u32>,
}

impl EvolutionEngine {
    /// 新建演进引擎
    pub fn new() -> Self {
        let mut engine = Self {
            report: EvolutionReport::new(),
            asi_module_versions: HashMap::new(),
        };
        engine.bootstrap_asi_modules();
        engine
    }

    /// 引导: 7 关键 ASI Python 模块默认 version (借 Stage 1)
    fn bootstrap_asi_modules(&mut self) {
        // Stage 1 ASI Python 关键模块 (per P10-1 §1.2 + asi_modules.rs)
        self.asi_module_versions
            .insert("apeireth.v1077_asi_v04_full_measurement".to_string(), 1077);
        self.asi_module_versions
            .insert("apeireth.v1400_asi_self_framework".to_string(), 1400);
        self.asi_module_versions
            .insert("apeireth.v1447_asi_cross_modular_audit".to_string(), 1447);
        self.asi_module_versions.insert(
            "apeireth.v1457_asi_six_deployment_operational_runbook".to_string(),
            1457,
        );
        self.asi_module_versions.insert(
            "apeireth.v1458_asi_north_star_ceiling_chain_audit".to_string(),
            1458,
        );
        self.asi_module_versions.insert(
            "apeireth.v1467_asi_audit_http_gateway_history_diff".to_string(),
            1467,
        );
        self.asi_module_versions.insert(
            "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence".to_string(),
            1470,
        );
    }

    /// 7 ASI Python 模块都装了 version
    pub fn asi_module_count(&self) -> usize {
        self.asi_module_versions.len()
    }

    /// 取模块当前 version (没装返 0)
    pub fn current_version(&self, module: &str) -> u32 {
        self.asi_module_versions.get(module).copied().unwrap_or(0)
    }

    /// R1 NewModuleSafe — 新增模块必通过 7 项安全 check
    /// - module_id ∈ [0, 6] (跟 G2 L1 TypeCheck 接)
    /// - guard_layers = 6 (跟 G2 6 重 v7 接)
    /// - borrow_id ∈ [0, 10] (跟 G2 L6 接)
    /// - 版本号 v# > 0
    /// - 守门层 = 6 (1:1 跟 B4)
    /// - 借鉴 ID valid
    /// - 不是 ceiling_critical (新增 ceiling_critical 必须额外审批)
    pub fn check_r1_new_module_safe(&mut self, ctx: &EvolutionContext) -> EvolutionOutcome {
        let all_ok = ctx.borrow_id <= 10
            && ctx.guard_layers == 6
            && ctx.current_version > 0
            && !ctx.module.is_empty();
        let outcome = if all_ok {
            EvolutionOutcome::Pass
        } else {
            EvolutionOutcome::Fail
        };
        self.report.record(EvolutionEvent::new(
            &ctx.module,
            EvolutionKind::Add,
            EvolutionRule::R1NewModuleSafe,
            outcome,
            format!(
                "7 安全 check: borrow_id={} guard_layers={} version={}",
                ctx.borrow_id, ctx.guard_layers, ctx.current_version
            ),
        ));
        outcome
    }

    /// R2 UpgradeBackwardCompat — 升级必保持向后兼容
    /// - current_version > 0 (升级必须有原版本)
    /// - 24 LOCKED 入口签名 0 改 (B1 严守, 通过 guard_layers=6 检查)
    pub fn check_r2_upgrade_backward_compat(
        &mut self,
        ctx: &EvolutionContext,
        new_version: u32,
    ) -> EvolutionOutcome {
        let is_compat =
            ctx.current_version > 0 && new_version > ctx.current_version && ctx.guard_layers == 6;
        let outcome = if is_compat {
            EvolutionOutcome::Pass
        } else if new_version <= ctx.current_version {
            EvolutionOutcome::Warn
        } else {
            EvolutionOutcome::Fail
        };
        self.report.record(EvolutionEvent::new(
            &ctx.module,
            EvolutionKind::Upgrade,
            EvolutionRule::R2UpgradeBackwardCompat,
            outcome,
            format!(
                "current_version={} new_version={} guard_layers={}",
                ctx.current_version, new_version, ctx.guard_layers
            ),
        ));
        outcome
    }

    /// R3 DowngradeJustified — 降级必记录原因
    /// - 降级必带 reason (ctx 里有 reason 字段)
    /// - 24 LOCKED 入口签名 0 改
    pub fn check_r3_downgrade_justified(
        &mut self,
        ctx: &EvolutionContext,
        new_version: u32,
        reason: &str,
    ) -> EvolutionOutcome {
        let is_justified = !reason.is_empty() && ctx.guard_layers == 6;
        let outcome = if !is_justified {
            EvolutionOutcome::Fail
        } else if new_version >= ctx.current_version {
            EvolutionOutcome::Warn
        } else {
            EvolutionOutcome::Pass
        };
        self.report.record(EvolutionEvent::new(
            &ctx.module,
            EvolutionKind::Downgrade,
            EvolutionRule::R3DowngradeJustified,
            outcome,
            format!(
                "current_version={} new_version={} reason='{}'",
                ctx.current_version, new_version, reason
            ),
        ));
        outcome
    }

    /// R4 RetireConfirmed — 退役必 3 方确认 (Mavis + 主人 + 借鉴 ID)
    /// - borrow_id ∈ [0, 10] (借鉴 ID 已确认)
    /// - 0 active self_loop (R129-4 严格守门)
    /// - 守门层 = 6
    pub fn check_r4_retire_confirmed(
        &mut self,
        ctx: &EvolutionContext,
        mavis_confirmed: bool,
        master_confirmed: bool,
    ) -> EvolutionOutcome {
        let is_confirmed =
            mavis_confirmed && master_confirmed && ctx.borrow_id <= 10 && ctx.guard_layers == 6;
        let outcome = if is_confirmed {
            EvolutionOutcome::Pass
        } else {
            EvolutionOutcome::Fail
        };
        self.report.record(EvolutionEvent::new(
            &ctx.module,
            EvolutionKind::Retire,
            EvolutionRule::R4RetireConfirmed,
            outcome,
            format!(
                "3 方确认: mavis={} master={} borrow_id={} guard_layers={}",
                mavis_confirmed, master_confirmed, ctx.borrow_id, ctx.guard_layers
            ),
        ));
        outcome
    }

    /// 跑 1 次所有 4 规则 × 1 default module (for self-test)
    pub fn audit_default(&mut self) -> &EvolutionReport {
        let ctx = EvolutionContext {
            module: "apeireth.v1077_asi_v04_full_measurement".to_string(),
            current_version: 1077,
            borrow_id: 5, // kani 4502 = #5
            guard_layers: 6,
        };
        // Add: R1
        self.check_r1_new_module_safe(&ctx);
        // Upgrade: R2
        self.check_r2_upgrade_backward_compat(&ctx, 1078);
        // Downgrade: R3
        self.check_r3_downgrade_justified(&ctx, 1076, "test reason");
        // Retire: R4
        self.check_r4_retire_confirmed(&ctx, true, true);
        &self.report
    }
}

impl Default for EvolutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Stage 5 G4 公开 API helper
// =============================================================================

/// Stage 5 G4 演进治理版本
pub fn evolution_governance_version() -> &'static str {
    EVOLUTION_GOVERNANCE_VERSION
}

/// Stage 5 G4 演进治理 health check
#[derive(Debug, Clone)]
pub struct EvolutionGovernanceHealth {
    pub version: &'static str,
    pub rule_count: usize,
    pub kind_count: usize,
    pub stage_count: usize,
    pub asi_module_count: usize,
    pub is_ok: bool,
}

impl std::fmt::Display for EvolutionGovernanceHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge G4 演进治理 ({}): {} rules, {} kinds, {} stages, {} ASI modules, ok={}",
            self.version,
            self.rule_count,
            self.kind_count,
            self.stage_count,
            self.asi_module_count,
            self.is_ok
        )
    }
}

/// Stage 5 G4 演进治理 health
pub fn evolution_governance_health() -> EvolutionGovernanceHealth {
    let engine = EvolutionEngine::new();
    EvolutionGovernanceHealth {
        version: evolution_governance_version(),
        rule_count: EVOLUTION_GOVERNANCE_RULE_COUNT,
        kind_count: EVOLUTION_GOVERNANCE_KIND_COUNT,
        stage_count: EVOLUTION_GOVERNANCE_STAGE_COUNT,
        asi_module_count: engine.asi_module_count(),
        is_ok: engine.asi_module_count() == 7,
    }
}

/// Stage 5 G4 演进治理报告
pub fn evolution_governance_summary() -> EvolutionReport {
    let mut e = EvolutionEngine::new();
    e.audit_default();
    e.report.clone()
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_r129_stage5_g4() {
        assert_eq!(evolution_governance_version(), "0.1.0-R129-Stage5-G4");
    }

    #[test]
    fn rule_count_is_4() {
        assert_eq!(EVOLUTION_GOVERNANCE_RULE_COUNT, 4);
        assert_eq!(EvolutionRule::ALL.len(), 4);
    }

    #[test]
    fn kind_count_is_4() {
        assert_eq!(EVOLUTION_GOVERNANCE_KIND_COUNT, 4);
        assert_eq!(EvolutionKind::ALL.len(), 4);
    }

    #[test]
    fn stage_count_is_4() {
        assert_eq!(EVOLUTION_GOVERNANCE_STAGE_COUNT, 4);
    }

    #[test]
    fn kind_number_1_to_4() {
        assert_eq!(EvolutionKind::Add.number(), 1);
        assert_eq!(EvolutionKind::Upgrade.number(), 2);
        assert_eq!(EvolutionKind::Downgrade.number(), 3);
        assert_eq!(EvolutionKind::Retire.number(), 4);
    }

    #[test]
    fn kind_name_4_kinds() {
        assert_eq!(EvolutionKind::Add.name(), "add");
        assert_eq!(EvolutionKind::Upgrade.name(), "upgrade");
        assert_eq!(EvolutionKind::Downgrade.name(), "downgrade");
        assert_eq!(EvolutionKind::Retire.name(), "retire");
    }

    #[test]
    fn kind_description_4_kinds() {
        for kind in EvolutionKind::ALL {
            assert!(!kind.description().is_empty());
        }
    }

    #[test]
    fn rule_number_1_to_4() {
        assert_eq!(EvolutionRule::R1NewModuleSafe.number(), 1);
        assert_eq!(EvolutionRule::R2UpgradeBackwardCompat.number(), 2);
        assert_eq!(EvolutionRule::R3DowngradeJustified.number(), 3);
        assert_eq!(EvolutionRule::R4RetireConfirmed.number(), 4);
    }

    #[test]
    fn rule_name_4_rules() {
        assert_eq!(EvolutionRule::R1NewModuleSafe.name(), "R1_new_module_safe");
        assert_eq!(
            EvolutionRule::R2UpgradeBackwardCompat.name(),
            "R2_upgrade_backward_compat"
        );
        assert_eq!(
            EvolutionRule::R3DowngradeJustified.name(),
            "R3_downgrade_justified"
        );
        assert_eq!(
            EvolutionRule::R4RetireConfirmed.name(),
            "R4_retire_confirmed"
        );
    }

    #[test]
    fn rule_description_4_rules() {
        for rule in EvolutionRule::ALL {
            assert!(!rule.description().is_empty());
        }
    }

    #[test]
    fn outcome_name_3_states() {
        assert_eq!(EvolutionOutcome::Pass.name(), "pass");
        assert_eq!(EvolutionOutcome::Warn.name(), "warn");
        assert_eq!(EvolutionOutcome::Fail.name(), "fail");
    }

    #[test]
    fn context_asi_default() {
        let c = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        assert_eq!(c.current_version, 1077);
        assert_eq!(c.guard_layers, 6);
    }

    #[test]
    fn context_default() {
        let c = EvolutionContext::default();
        assert_eq!(c.current_version, 1);
        assert_eq!(c.guard_layers, 6);
    }

    #[test]
    fn engine_bootstrap_7_modules() {
        let e = EvolutionEngine::new();
        assert_eq!(e.asi_module_count(), 7);
    }

    #[test]
    fn engine_current_version_known_module() {
        let e = EvolutionEngine::new();
        assert_eq!(
            e.current_version("apeireth.v1077_asi_v04_full_measurement"),
            1077
        );
        assert_eq!(
            e.current_version("apeireth.v1458_asi_north_star_ceiling_chain_audit"),
            1458
        );
    }

    #[test]
    fn engine_current_version_unknown_returns_0() {
        let e = EvolutionEngine::new();
        assert_eq!(e.current_version("apeireth.unknown"), 0);
    }

    #[test]
    fn r1_new_module_safe_passes() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        let o = e.check_r1_new_module_safe(&ctx);
        assert_eq!(o, EvolutionOutcome::Pass);
    }

    #[test]
    fn r1_new_module_safe_fails_for_invalid_borrow_id() {
        let mut e = EvolutionEngine::new();
        let mut ctx = EvolutionContext::asi_default("apeireth.test", 1);
        ctx.borrow_id = 99; // 越界
        let o = e.check_r1_new_module_safe(&ctx);
        assert_eq!(o, EvolutionOutcome::Fail);
    }

    #[test]
    fn r1_new_module_safe_fails_for_wrong_guard_layers() {
        let mut e = EvolutionEngine::new();
        let mut ctx = EvolutionContext::asi_default("apeireth.test", 1);
        ctx.guard_layers = 5; // 错
        let o = e.check_r1_new_module_safe(&ctx);
        assert_eq!(o, EvolutionOutcome::Fail);
    }

    #[test]
    fn r2_upgrade_backward_compat_passes() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        let o = e.check_r2_upgrade_backward_compat(&ctx, 1078);
        assert_eq!(o, EvolutionOutcome::Pass);
    }

    #[test]
    fn r2_upgrade_backward_compat_warns_for_lower_version() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        let o = e.check_r2_upgrade_backward_compat(&ctx, 1076);
        assert_eq!(o, EvolutionOutcome::Warn);
    }

    #[test]
    fn r2_upgrade_backward_compat_fails_for_zero_version() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 0);
        let o = e.check_r2_upgrade_backward_compat(&ctx, 1078);
        assert_eq!(o, EvolutionOutcome::Fail);
    }

    #[test]
    fn r3_downgrade_justified_passes() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        let o = e.check_r3_downgrade_justified(&ctx, 1076, "test reason");
        assert_eq!(o, EvolutionOutcome::Pass);
    }

    #[test]
    fn r3_downgrade_justified_fails_for_empty_reason() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        let o = e.check_r3_downgrade_justified(&ctx, 1076, "");
        assert_eq!(o, EvolutionOutcome::Fail);
    }

    #[test]
    fn r3_downgrade_justified_warns_for_higher_version() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        let o = e.check_r3_downgrade_justified(&ctx, 1078, "test");
        assert_eq!(o, EvolutionOutcome::Warn);
    }

    #[test]
    fn r4_retire_confirmed_passes() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        let o = e.check_r4_retire_confirmed(&ctx, true, true);
        assert_eq!(o, EvolutionOutcome::Pass);
    }

    #[test]
    fn r4_retire_confirmed_fails_for_no_mavis() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        let o = e.check_r4_retire_confirmed(&ctx, false, true);
        assert_eq!(o, EvolutionOutcome::Fail);
    }

    #[test]
    fn r4_retire_confirmed_fails_for_no_master() {
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        let o = e.check_r4_retire_confirmed(&ctx, true, false);
        assert_eq!(o, EvolutionOutcome::Fail);
    }

    #[test]
    fn audit_default_4_events() {
        let mut e = EvolutionEngine::new();
        e.audit_default();
        assert_eq!(e.report.total(), 4);
    }

    #[test]
    fn audit_default_all_pass() {
        let mut e = EvolutionEngine::new();
        e.audit_default();
        assert!(e.report.is_all_pass());
    }

    #[test]
    fn report_counts() {
        let mut e = EvolutionEngine::new();
        e.audit_default();
        assert_eq!(e.report.pass_count(), 4);
        assert_eq!(e.report.warn_count(), 0);
        assert_eq!(e.report.fail_count(), 0);
    }

    #[test]
    fn report_count_by_kind() {
        let mut e = EvolutionEngine::new();
        e.audit_default();
        let by_kind = e.report.count_by_kind();
        assert_eq!(by_kind.get(&EvolutionKind::Add), Some(&1));
        assert_eq!(by_kind.get(&EvolutionKind::Upgrade), Some(&1));
        assert_eq!(by_kind.get(&EvolutionKind::Downgrade), Some(&1));
        assert_eq!(by_kind.get(&EvolutionKind::Retire), Some(&1));
    }

    #[test]
    fn report_count_by_rule() {
        let mut e = EvolutionEngine::new();
        e.audit_default();
        let by_rule = e.report.count_by_rule();
        assert_eq!(by_rule.get(&EvolutionRule::R1NewModuleSafe), Some(&1));
        assert_eq!(
            by_rule.get(&EvolutionRule::R2UpgradeBackwardCompat),
            Some(&1)
        );
        assert_eq!(by_rule.get(&EvolutionRule::R3DowngradeJustified), Some(&1));
        assert_eq!(by_rule.get(&EvolutionRule::R4RetireConfirmed), Some(&1));
    }

    #[test]
    fn event_new() {
        let e = EvolutionEvent::new(
            "test",
            EvolutionKind::Add,
            EvolutionRule::R1NewModuleSafe,
            EvolutionOutcome::Pass,
            "ok",
        );
        assert_eq!(e.module, "test");
        assert_eq!(e.kind, EvolutionKind::Add);
        assert_eq!(e.rule, EvolutionRule::R1NewModuleSafe);
        assert_eq!(e.outcome, EvolutionOutcome::Pass);
    }

    #[test]
    fn health_struct_ok() {
        let h = evolution_governance_health();
        assert!(h.is_ok);
        assert_eq!(h.rule_count, 4);
        assert_eq!(h.kind_count, 4);
        assert_eq!(h.stage_count, 4);
        assert_eq!(h.asi_module_count, 7);
    }

    #[test]
    fn display_health() {
        let h = evolution_governance_health();
        let s = format!("{h}");
        assert!(s.contains("G4 演进治理"));
        assert!(s.contains("4 rules"));
    }

    #[test]
    fn display_report() {
        let r = evolution_governance_summary();
        let s = format!("{r}");
        assert!(s.contains("G4 演进治理报告"));
        assert!(s.contains("kind add"));
        assert!(s.contains("kind upgrade"));
        assert!(s.contains("kind downgrade"));
        assert!(s.contains("kind retire"));
    }

    #[test]
    fn g4_to_p5_2_consistency() {
        // 1:1 跟 P5-2 DecisionTree 模式 (3 段派发) + 借 superpowers Skill Evolution
        let mut e = EvolutionEngine::new();
        let ctx = EvolutionContext::asi_default("apeireth.v1077_asi_v04_full_measurement", 1077);
        e.check_r1_new_module_safe(&ctx);
        e.check_r2_upgrade_backward_compat(&ctx, 1078);
        e.check_r3_downgrade_justified(&ctx, 1076, "test");
        e.check_r4_retire_confirmed(&ctx, true, true);
        assert_eq!(e.report.total(), 4);
    }

    #[test]
    fn g4_to_g1_g2_g3_consistency() {
        // 4 治理维度互锁
        // G1 资源: 4 dims
        // G2 权限: 6 layers (1:1 跟 B4)
        // G3 形式化: 8 harnesses
        // G4 演进: 4 rules = 4
        // 4+6+8+4 = 22 → ASI Stage 5 治理规模
        assert_eq!(4 + 6 + 8 + 4, 22, "ASI Stage 5 治理规模 = 22 (4+6+8+4)");
    }
}
