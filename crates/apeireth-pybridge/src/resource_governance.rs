//! R129-5 ASI Python 整合 Stage 5 治理 — G1 资源治理
//!
//! **任务**: ASI Python 整合 Stage 5 治理 (per decision-61 §3.1 R129-5)
//! **承接**: P10-1/2/3 Stage 1-3 + R129-4 Stage 4 自治 + P5-2 Library Stage 5 治理 (per decision-55 §2.3) + P8-2 retry Library Stage 5.1 形式化证明 (per decision-56)
//! **维度**: G1 资源治理 (resource governance) — 限流 + 配额 + 超时
//! **借鉴**:
//! - PyO3 928 (R125-9 ✅ done) — Python ↔ Rust bridge + GIL 资源限制
//! - hyper 80 (R125-3 ✅ done) — 连接池限流模式 (借 Stage 1 bridge_pool.rs 复用)
//! - superpowers 234 (R125-14 ✅ done) — Skill 限流模式 (SkillQuota)
//! **目标**: ASI Python 资源 4 维度配额 (rate / memory / time / count) + 3 路径 (allow / throttle / reject) + 治理报告
//!
//! # G1 资源治理 范围
//!
//! 1. **ResourceQuota** — 4 维度配额 (per resource dimension)
//!    - RateLimit (请求/秒) — 借鉴 superpowers 234 SkillQuota
//!    - MemoryBudget (字节) — 借鉴 PyO3 928 GIL 内存限制
//!    - TimeBudget (毫秒) — 借鉴 hyper 80 connection timeout
//!    - CountLimit (并发数) — 借鉴 Stage 1 BridgeModulePool max_idle
//! 2. **ResourceGovernor** — 资源治理引擎, 4 维度配额守门
//!    - 3 路径: Allow / Throttle(降级) / Reject(失败)
//!    - cfg-无关 (默认 build 也能跑, 0 装 PASS 严守)
//! 3. **ResourceAuditEvent** — 治理事件 (per audit hook)
//!    - timestamp + dimension + action + hint
//! 4. **ResourceReport** — 治理报告 (聚合 events)
//! 5. **AsiModule 资源映射** — 7 关键 ASI Python 模块 (V1077/V1400/V1447/V1457/V1458/V1467/V1470) 各有默认配额
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-5)
//!
//! - ✅ PyO3 928 (R125-9) cloned = 借鉴真实施 (memory budget 模式)
//! - ✅ hyper 80 (R125-3) cloned = 借鉴真实施 (count limit / pool 模式)
//! - ✅ superpowers 234 (R125-14) cloned = 借鉴真实施 (skill quota 模式)
//! - 默认 build: 资源治理 0 装 PASS 严守, 跑 0 体积 stub
//! - python-ext build: 资源治理可集成 PyO3 GIL 内存限制
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 3 值 0 改
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW, 不算改)
//! - B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 / A3 13 键 0 改
//! - C1 0 主动 commit (Mavis 整合 #5 commit 时机拍板)
//! - C2 0 装 PASS 严守

use std::collections::HashMap;
use std::time::{Duration, Instant};

// =============================================================================
// G1 资源治理版本 + 维度计数
// =============================================================================

/// G1 资源治理版本 (per decision-61 §3.1 R129-5)
pub const RESOURCE_GOVERNANCE_VERSION: &str = "0.1.0-R129-Stage5-G1";

/// G1 资源治理维度数 (4: rate / memory / time / count)
pub const RESOURCE_GOVERNANCE_DIMENSION_COUNT: usize = 4;

/// G1 资源治理 ASI Python 模块数 (7 关键模块, 跟 Stage 1 一致)
pub const RESOURCE_GOVERNANCE_MODULE_COUNT: usize = 7;

// =============================================================================
// ResourceDimension 枚举 (4 维度)
// =============================================================================

/// 4 资源维度 (rate / memory / time / count)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceDimension {
    /// 速率 (请求/秒) — 借 superpowers 234 SkillQuota
    Rate,
    /// 内存 (字节) — 借 PyO3 928 GIL 限制
    Memory,
    /// 时间 (毫秒) — 借 hyper 80 connection timeout
    Time,
    /// 并发数 — 借 Stage 1 BridgeModulePool max_idle
    Count,
}

impl ResourceDimension {
    /// 4 维度列表 (按 Rate/Memory/Time/Count 顺序)
    pub const ALL: [ResourceDimension; RESOURCE_GOVERNANCE_DIMENSION_COUNT] = [
        ResourceDimension::Rate,
        ResourceDimension::Memory,
        ResourceDimension::Time,
        ResourceDimension::Count,
    ];

    /// 维度名 (人类可读, 跟 superpowers SkillQuota 字段对齐)
    pub fn name(&self) -> &'static str {
        match self {
            ResourceDimension::Rate => "rate",
            ResourceDimension::Memory => "memory",
            ResourceDimension::Time => "time",
            ResourceDimension::Count => "count",
        }
    }

    /// 维度单位 (报告里显示)
    pub fn unit(&self) -> &'static str {
        match self {
            ResourceDimension::Rate => "req/s",
            ResourceDimension::Memory => "bytes",
            ResourceDimension::Time => "ms",
            ResourceDimension::Count => "concurrent",
        }
    }
}

// =============================================================================
// ResourceQuota — 4 维度配额
// =============================================================================

/// 4 维度配额 (借 superpowers 234 SkillQuota 模式)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceQuota {
    /// 请求速率上限 (req/s, 0 = unlimited)
    pub rate_per_sec: u32,
    /// 内存预算 (bytes, 0 = unlimited)
    pub memory_bytes: u64,
    /// 时间预算 (ms, 0 = unlimited)
    pub time_ms: u64,
    /// 并发数上限 (0 = unlimited)
    pub count_max: u32,
}

impl ResourceQuota {
    /// 默认配额 (保守, 适合大多数 ASI Python 模块)
    /// - rate: 100 req/s
    /// - memory: 64 MB
    /// - time: 5000 ms
    /// - count: 8 concurrent
    pub const fn default_const() -> Self {
        Self {
            rate_per_sec: 100,
            memory_bytes: 64 * 1024 * 1024,
            time_ms: 5_000,
            count_max: 8,
        }
    }

    /// 严格配额 (V1458 ceiling-critical 模块 — 北极星天花板链 audit, 必须严格)
    /// - rate: 10 req/s
    /// - memory: 32 MB
    /// - time: 2000 ms
    /// - count: 2 concurrent
    pub const fn strict_const() -> Self {
        Self {
            rate_per_sec: 10,
            memory_bytes: 32 * 1024 * 1024,
            time_ms: 2_000,
            count_max: 2,
        }
    }

    /// 宽松配额 (V1077 measurement 维度 — 全 17 维测量, 允许大流量)
    /// - rate: 1000 req/s
    /// - memory: 256 MB
    /// - time: 30000 ms
    /// - count: 32 concurrent
    pub const fn relaxed_const() -> Self {
        Self {
            rate_per_sec: 1_000,
            memory_bytes: 256 * 1024 * 1024,
            time_ms: 30_000,
            count_max: 32,
        }
    }

    /// 0 装 (unlimited) 配额
    pub const fn unlimited_const() -> Self {
        Self {
            rate_per_sec: 0,
            memory_bytes: 0,
            time_ms: 0,
            count_max: 0,
        }
    }

    /// 该维度的配额值
    pub fn value_for(&self, dim: ResourceDimension) -> u64 {
        match dim {
            ResourceDimension::Rate => self.rate_per_sec as u64,
            ResourceDimension::Memory => self.memory_bytes,
            ResourceDimension::Time => self.time_ms,
            ResourceDimension::Count => self.count_max as u64,
        }
    }
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self::default_const()
    }
}

// =============================================================================
// GovernanceAction — 3 路径
// =============================================================================

/// 资源治理 3 路径 (per master-reupgrade decision-33 §2.3 二极管 4 路径 → 治理 3 路径)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GovernanceAction {
    /// 允许 (in budget)
    Allow,
    /// 节流 (over soft limit, 降级)
    Throttle,
    /// 拒绝 (over hard limit, 失败)
    Reject,
}

impl GovernanceAction {
    /// 4 路径名 (主 11:51 不要二极管 4 路径: 成功/重试/降级/失败)
    pub fn name(&self) -> &'static str {
        match self {
            GovernanceAction::Allow => "allow",
            GovernanceAction::Throttle => "throttle",
            GovernanceAction::Reject => "reject",
        }
    }
}

// =============================================================================
// ResourceAuditEvent — 治理事件
// =============================================================================

/// 资源治理事件 (每次 check 调用 1 个)
#[derive(Debug, Clone)]
pub struct ResourceAuditEvent {
    /// 事件时间 (Instant, 借 std::time)
    pub timestamp: Instant,
    /// ASI Python 模块名 (e.g. "apeireth.v1458_...")
    pub module: String,
    /// 检查的维度
    pub dimension: ResourceDimension,
    /// 实际使用量
    pub used: u64,
    /// 配额上限
    pub quota: u64,
    /// 治理行动
    pub action: GovernanceAction,
    /// 提示 (e.g. "rate exceeded 100/100 req/s")
    pub hint: String,
}

impl ResourceAuditEvent {
    /// 构造新事件
    pub fn new(
        module: impl Into<String>,
        dimension: ResourceDimension,
        used: u64,
        quota: u64,
        action: GovernanceAction,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: Instant::now(),
            module: module.into(),
            dimension,
            used,
            quota,
            action,
            hint: hint.into(),
        }
    }

    /// 该事件是否 over budget (used > quota, 且 quota > 0)
    pub fn is_over_budget(&self) -> bool {
        self.quota > 0 && self.used > self.quota
    }
}

// =============================================================================
// ResourceReport — 治理报告 (聚合 events)
// =============================================================================

/// 资源治理报告
#[derive(Debug, Clone, Default)]
pub struct ResourceReport {
    /// 治理事件
    pub events: Vec<ResourceAuditEvent>,
}

impl ResourceReport {
    /// 新建空报告
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加事件
    pub fn record(&mut self, event: ResourceAuditEvent) {
        self.events.push(event);
    }

    /// 事件总数
    pub fn total(&self) -> usize {
        self.events.len()
    }

    /// 允许事件数
    pub fn allow_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.action == GovernanceAction::Allow)
            .count()
    }

    /// 节流事件数
    pub fn throttle_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.action == GovernanceAction::Throttle)
            .count()
    }

    /// 拒绝事件数
    pub fn reject_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.action == GovernanceAction::Reject)
            .count()
    }

    /// 报告是否完全允许 (无 Throttle / Reject)
    pub fn is_all_allowed(&self) -> bool {
        self.events.iter().all(|e| e.action == GovernanceAction::Allow)
    }

    /// 按维度统计 (dimension -> count)
    pub fn count_by_dimension(&self) -> HashMap<ResourceDimension, usize> {
        let mut map = HashMap::new();
        for e in &self.events {
            *map.entry(e.dimension).or_insert(0) += 1;
        }
        map
    }

    /// 按模块统计 (module -> count)
    pub fn count_by_module(&self) -> HashMap<String, usize> {
        let mut map: HashMap<String, usize> = HashMap::new();
        for e in &self.events {
            *map.entry(e.module.clone()).or_insert(0) += 1;
        }
        map
    }
}

impl std::fmt::Display for ResourceReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge G1 资源治理报告 ({} events, allow={} throttle={} reject={}):",
            self.total(),
            self.allow_count(),
            self.throttle_count(),
            self.reject_count()
        )?;
        let by_dim = self.count_by_dimension();
        for dim in ResourceDimension::ALL {
            if let Some(count) = by_dim.get(&dim) {
                writeln!(f, "  dim {}: {} events", dim.name(), count)?;
            }
        }
        Ok(())
    }
}

// =============================================================================
// ResourceGovernor — 资源治理引擎
// =============================================================================

/// 资源治理引擎 (per module 配额表 + 累计 events)
#[derive(Debug, Clone)]
pub struct ResourceGovernor {
    /// 模块名 -> 配额 (借 Stage 1 ASI 7 模块 + default fallback)
    quotas: HashMap<String, ResourceQuota>,
    /// 治理报告
    pub report: ResourceReport,
    /// 软阈值比例 (默认 0.8, used/quota >= 0.8 = Throttle)
    soft_ratio: f64,
    /// 硬阈值比例 (默认 1.0, used/quota >= 1.0 = Reject)
    hard_ratio: f64,
}

impl ResourceGovernor {
    /// 新建治理引擎
    pub fn new() -> Self {
        let mut governor = Self {
            quotas: HashMap::new(),
            report: ResourceReport::new(),
            soft_ratio: 0.8,
            hard_ratio: 1.0,
        };
        governor.bootstrap_asi_modules();
        governor
    }

    /// 引导: 给 7 关键 ASI Python 模块装默认配额 (借 Stage 1 asi_modules.rs)
    fn bootstrap_asi_modules(&mut self) {
        // V1077 测量 — relaxed (17 维测量, 允许大流量)
        self.quotas.insert(
            "apeireth.v1077_asi_v04_full_measurement".to_string(),
            ResourceQuota::relaxed_const(),
        );
        // V1400 Self framework — default
        self.quotas.insert(
            "apeireth.v1400_asi_self_framework".to_string(),
            ResourceQuota::default_const(),
        );
        // V1447 Cross modular audit — strict (35 audit pairs × 5 closure = 175 probes, 严守)
        self.quotas.insert(
            "apeireth.v1447_asi_cross_modular_audit".to_string(),
            ResourceQuota::strict_const(),
        );
        // V1457 6 deployment operational runbook — default
        self.quotas.insert(
            "apeireth.v1457_asi_six_deployment_operational_runbook".to_string(),
            ResourceQuota::default_const(),
        );
        // V1458 ceiling critical — strict (北极星天花板链 audit, ceiling_critical = true)
        self.quotas.insert(
            "apeireth.v1458_asi_north_star_ceiling_chain_audit".to_string(),
            ResourceQuota::strict_const(),
        );
        // V1467 HTTP gateway — default
        self.quotas.insert(
            "apeireth.v1467_asi_audit_http_gateway_history_diff".to_string(),
            ResourceQuota::default_const(),
        );
        // V1470 batch harness — relaxed (3 runs default × 12 cross-checks = 36 total, 允许大流量)
        self.quotas.insert(
            "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence".to_string(),
            ResourceQuota::relaxed_const(),
        );
    }

    /// 7 ASI Python 模块都装了配额
    pub fn asi_module_count(&self) -> usize {
        self.quotas.len()
    }

    /// 取某模块配额 (没装返 default)
    pub fn quota_for(&self, module: &str) -> ResourceQuota {
        self.quotas
            .get(module)
            .cloned()
            .unwrap_or_else(ResourceQuota::default)
    }

    /// 设置模块配额 (覆盖默认)
    pub fn set_quota(&mut self, module: impl Into<String>, quota: ResourceQuota) {
        self.quotas.insert(module.into(), quota);
    }

    /// 治理一次资源使用 (used vs quota → 3 路径)
    /// - quota = 0 = unlimited → Allow
    /// - used / quota >= hard_ratio → Reject
    /// - used / quota >= soft_ratio → Throttle
    /// - else → Allow
    pub fn check(
        &mut self,
        module: &str,
        dimension: ResourceDimension,
        used: u64,
    ) -> GovernanceAction {
        let quota = self.quota_for(module);
        let quota_value = quota.value_for(dimension);

        // quota = 0 = unlimited → 永远 Allow
        let action = if quota_value == 0 {
            GovernanceAction::Allow
        } else {
            let ratio = used as f64 / quota_value as f64;
            if ratio >= self.hard_ratio {
                GovernanceAction::Reject
            } else if ratio >= self.soft_ratio {
                GovernanceAction::Throttle
            } else {
                GovernanceAction::Allow
            }
        };

        let hint = if quota_value == 0 {
            format!(
                "unlimited (module={}, dim={}, used={})",
                module,
                dimension.name(),
                used
            )
        } else {
            format!(
                "{} used={}/{} (ratio={:.2}, action={})",
                dimension.name(),
                used,
                quota_value,
                used as f64 / quota_value as f64,
                action.name()
            )
        };

        let event = ResourceAuditEvent::new(module, dimension, used, quota_value, action, hint);
        self.report.record(event);
        action
    }

    /// 跑 1 次所有模块 × 4 维度基础 check (for self-test)
    /// - rate: 50 req/s
    /// - memory: 32 MB
    /// - time: 2500 ms
    /// - count: 4 concurrent
    pub fn audit_all(&mut self) -> &ResourceReport {
        let test_used: [(ResourceDimension, u64); 4] = [
            (ResourceDimension::Rate, 50),
            (ResourceDimension::Memory, 32 * 1024 * 1024),
            (ResourceDimension::Time, 2_500),
            (ResourceDimension::Count, 4),
        ];
        // 先 clone 模块名, 避免 borrow conflict
        let modules: Vec<String> = self.quotas.keys().cloned().collect();
        for module in &modules {
            for (dim, used) in &test_used {
                self.check(module, *dim, *used);
            }
        }
        &self.report
    }
}

impl Default for ResourceGovernor {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Stage 5 G1 公开 API helper
// =============================================================================

/// Stage 5 G1 资源治理: 7 关键模块都装了配额 verify
pub fn resource_governance_bootstrap_ok() -> bool {
    let mut g = ResourceGovernor::new();
    g.asi_module_count() == RESOURCE_GOVERNANCE_MODULE_COUNT
}

/// Stage 5 G1 资源治理版本
pub fn resource_governance_version() -> &'static str {
    RESOURCE_GOVERNANCE_VERSION
}

/// Stage 5 G1 资源治理健康度
#[derive(Debug, Clone)]
pub struct ResourceGovernanceHealth {
    pub version: &'static str,
    pub dimension_count: usize,
    pub asi_module_count: usize,
    pub is_ok: bool,
}

impl std::fmt::Display for ResourceGovernanceHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge G1 资源治理 ({}): {} dims, {} ASI modules, ok={}",
            self.version, self.dimension_count, self.asi_module_count, self.is_ok
        )
    }
}

/// Stage 5 G1 资源治理 health check
pub fn resource_governance_health() -> ResourceGovernanceHealth {
    ResourceGovernanceHealth {
        version: resource_governance_version(),
        dimension_count: RESOURCE_GOVERNANCE_DIMENSION_COUNT,
        asi_module_count: RESOURCE_GOVERNANCE_MODULE_COUNT,
        is_ok: resource_governance_bootstrap_ok(),
    }
}

/// Stage 5 G1 资源治理报告 (per module → action map)
pub fn resource_governance_summary() -> ResourceReport {
    let mut g = ResourceGovernor::new();
    g.audit_all();
    g.report.clone()
}

// =============================================================================
// 单元测试 (cfg-无关, 默认 + python-ext build 都跑)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_r129_stage5_g1() {
        assert_eq!(resource_governance_version(), "0.1.0-R129-Stage5-G1");
    }

    #[test]
    fn dimension_count_is_4() {
        assert_eq!(RESOURCE_GOVERNANCE_DIMENSION_COUNT, 4);
        assert_eq!(ResourceDimension::ALL.len(), 4);
    }

    #[test]
    fn asi_module_count_is_7() {
        assert_eq!(RESOURCE_GOVERNANCE_MODULE_COUNT, 7);
    }

    #[test]
    fn default_quota_has_all_4_dims_set() {
        let q = ResourceQuota::default_const();
        assert!(q.rate_per_sec > 0);
        assert!(q.memory_bytes > 0);
        assert!(q.time_ms > 0);
        assert!(q.count_max > 0);
    }

    #[test]
    fn strict_quota_stricter_than_default() {
        let s = ResourceQuota::strict_const();
        let d = ResourceQuota::default_const();
        assert!(s.rate_per_sec < d.rate_per_sec);
        assert!(s.memory_bytes < d.memory_bytes);
        assert!(s.time_ms < d.time_ms);
        assert!(s.count_max < d.count_max);
    }

    #[test]
    fn relaxed_quota_more_loose_than_default() {
        let r = ResourceQuota::relaxed_const();
        let d = ResourceQuota::default_const();
        assert!(r.rate_per_sec > d.rate_per_sec);
        assert!(r.memory_bytes > d.memory_bytes);
        assert!(r.time_ms > d.time_ms);
        assert!(r.count_max > d.count_max);
    }

    #[test]
    fn unlimited_quota_all_zero() {
        let u = ResourceQuota::unlimited_const();
        assert_eq!(u.value_for(ResourceDimension::Rate), 0);
        assert_eq!(u.value_for(ResourceDimension::Memory), 0);
        assert_eq!(u.value_for(ResourceDimension::Time), 0);
        assert_eq!(u.value_for(ResourceDimension::Count), 0);
    }

    #[test]
    fn dimension_name_and_unit() {
        assert_eq!(ResourceDimension::Rate.name(), "rate");
        assert_eq!(ResourceDimension::Rate.unit(), "req/s");
        assert_eq!(ResourceDimension::Memory.name(), "memory");
        assert_eq!(ResourceDimension::Memory.unit(), "bytes");
        assert_eq!(ResourceDimension::Time.name(), "time");
        assert_eq!(ResourceDimension::Time.unit(), "ms");
        assert_eq!(ResourceDimension::Count.name(), "count");
        assert_eq!(ResourceDimension::Count.unit(), "concurrent");
    }

    #[test]
    fn quota_value_for_4_dims() {
        let q = ResourceQuota {
            rate_per_sec: 50,
            memory_bytes: 1024,
            time_ms: 100,
            count_max: 5,
        };
        assert_eq!(q.value_for(ResourceDimension::Rate), 50);
        assert_eq!(q.value_for(ResourceDimension::Memory), 1024);
        assert_eq!(q.value_for(ResourceDimension::Time), 100);
        assert_eq!(q.value_for(ResourceDimension::Count), 5);
    }

    #[test]
    fn action_name_3_paths() {
        assert_eq!(GovernanceAction::Allow.name(), "allow");
        assert_eq!(GovernanceAction::Throttle.name(), "throttle");
        assert_eq!(GovernanceAction::Reject.name(), "reject");
    }

    #[test]
    fn audit_event_is_over_budget() {
        let e = ResourceAuditEvent::new(
            "test",
            ResourceDimension::Rate,
            101,
            100,
            GovernanceAction::Reject,
            "test",
        );
        assert!(e.is_over_budget());
    }

    #[test]
    fn audit_event_unlimited_never_over_budget() {
        let e = ResourceAuditEvent::new(
            "test",
            ResourceDimension::Rate,
            999_999,
            0,
            GovernanceAction::Allow,
            "unlimited",
        );
        assert!(!e.is_over_budget());
    }

    #[test]
    fn report_record_and_counts() {
        let mut r = ResourceReport::new();
        r.record(ResourceAuditEvent::new(
            "m",
            ResourceDimension::Rate,
            50,
            100,
            GovernanceAction::Allow,
            "ok",
        ));
        r.record(ResourceAuditEvent::new(
            "m",
            ResourceDimension::Time,
            90,
            100,
            GovernanceAction::Throttle,
            "soft",
        ));
        r.record(ResourceAuditEvent::new(
            "m",
            ResourceDimension::Count,
            9,
            8,
            GovernanceAction::Reject,
            "hard",
        ));
        assert_eq!(r.total(), 3);
        assert_eq!(r.allow_count(), 1);
        assert_eq!(r.throttle_count(), 1);
        assert_eq!(r.reject_count(), 1);
        assert!(!r.is_all_allowed());
    }

    #[test]
    fn report_count_by_dimension_and_module() {
        let mut r = ResourceReport::new();
        r.record(ResourceAuditEvent::new(
            "a",
            ResourceDimension::Rate,
            1,
            100,
            GovernanceAction::Allow,
            "x",
        ));
        r.record(ResourceAuditEvent::new(
            "b",
            ResourceDimension::Rate,
            1,
            100,
            GovernanceAction::Allow,
            "x",
        ));
        r.record(ResourceAuditEvent::new(
            "a",
            ResourceDimension::Time,
            1,
            100,
            GovernanceAction::Allow,
            "x",
        ));
        let by_dim = r.count_by_dimension();
        let by_mod = r.count_by_module();
        assert_eq!(by_dim.get(&ResourceDimension::Rate), Some(&2));
        assert_eq!(by_dim.get(&ResourceDimension::Time), Some(&1));
        assert_eq!(by_mod.get("a"), Some(&2));
        assert_eq!(by_mod.get("b"), Some(&1));
    }

    #[test]
    fn governor_bootstrap_7_modules() {
        let g = ResourceGovernor::new();
        assert_eq!(g.asi_module_count(), 7);
    }

    #[test]
    fn governor_quota_for_known_module() {
        let g = ResourceGovernor::new();
        // V1458 = strict (ceiling critical)
        let q = g.quota_for("apeireth.v1458_asi_north_star_ceiling_chain_audit");
        assert_eq!(q.rate_per_sec, ResourceQuota::strict_const().rate_per_sec);
        // V1077 = relaxed (full measurement)
        let q = g.quota_for("apeireth.v1077_asi_v04_full_measurement");
        assert_eq!(
            q.rate_per_sec,
            ResourceQuota::relaxed_const().rate_per_sec
        );
    }

    #[test]
    fn governor_quota_for_unknown_module_returns_default() {
        let g = ResourceGovernor::new();
        let q = g.quota_for("apeireth.unknown_module");
        assert_eq!(q.rate_per_sec, ResourceQuota::default_const().rate_per_sec);
    }

    #[test]
    fn governor_set_quota_override() {
        let mut g = ResourceGovernor::new();
        let custom = ResourceQuota::unlimited_const();
        g.set_quota("custom_module", custom.clone());
        let q = g.quota_for("custom_module");
        assert_eq!(q.rate_per_sec, 0);
    }

    #[test]
    fn check_under_soft_returns_allow() {
        let mut g = ResourceGovernor::new();
        let action = g.check("test", ResourceDimension::Rate, 50);
        assert_eq!(action, GovernanceAction::Allow);
    }

    #[test]
    fn check_at_soft_returns_throttle() {
        let mut g = ResourceGovernor::new();
        // default quota rate = 100, soft = 0.8, so 80 = soft threshold
        let action = g.check("test", ResourceDimension::Rate, 80);
        assert_eq!(action, GovernanceAction::Throttle);
    }

    #[test]
    fn check_over_hard_returns_reject() {
        let mut g = ResourceGovernor::new();
        // default quota rate = 100, hard = 1.0, so 100+ = hard threshold
        let action = g.check("test", ResourceDimension::Rate, 110);
        assert_eq!(action, GovernanceAction::Reject);
    }

    #[test]
    fn check_unlimited_quota_always_allow() {
        let mut g = ResourceGovernor::new();
        g.set_quota("unlimited_module", ResourceQuota::unlimited_const());
        let action = g.check("unlimited_module", ResourceDimension::Rate, 999_999);
        assert_eq!(action, GovernanceAction::Allow);
    }

    #[test]
    fn v1458_ceiling_critical_strict_under_default_allow() {
        let mut g = ResourceGovernor::new();
        // V1458 strict quota rate = 10, 5 < 10 = under hard
        let action = g.check(
            "apeireth.v1458_asi_north_star_ceiling_chain_audit",
            ResourceDimension::Rate,
            5,
        );
        assert_eq!(action, GovernanceAction::Allow);
    }

    #[test]
    fn v1458_ceiling_critical_strict_at_soft_throttle() {
        let mut g = ResourceGovernor::new();
        // V1458 strict quota rate = 10, 0.8 * 10 = 8
        let action = g.check(
            "apeireth.v1458_asi_north_star_ceiling_chain_audit",
            ResourceDimension::Rate,
            8,
        );
        assert_eq!(action, GovernanceAction::Throttle);
    }

    #[test]
    fn v1458_ceiling_critical_strict_over_hard_reject() {
        let mut g = ResourceGovernor::new();
        // V1458 strict quota rate = 10, 11 = over hard
        let action = g.check(
            "apeireth.v1458_asi_north_star_ceiling_chain_audit",
            ResourceDimension::Rate,
            11,
        );
        assert_eq!(action, GovernanceAction::Reject);
    }

    #[test]
    fn audit_all_records_28_events() {
        // 7 modules × 4 dims = 28
        let mut g = ResourceGovernor::new();
        g.audit_all();
        assert_eq!(g.report.total(), 28);
    }

    #[test]
    fn health_struct_ok() {
        let h = resource_governance_health();
        assert!(h.is_ok);
        assert_eq!(h.dimension_count, 4);
        assert_eq!(h.asi_module_count, 7);
    }

    #[test]
    fn summary_returns_report() {
        let r = resource_governance_summary();
        assert_eq!(r.total(), 28);
    }

    #[test]
    fn display_resource_report() {
        let r = resource_governance_summary();
        let s = format!("{r}");
        assert!(s.contains("G1 资源治理报告"));
        assert!(s.contains("allow="));
        assert!(s.contains("throttle="));
        assert!(s.contains("reject="));
    }

    #[test]
    fn display_resource_dimension() {
        // No Display impl — verify name() works
        assert_eq!(ResourceDimension::Rate.name(), "rate");
    }

    #[test]
    fn check_4_dims_consistent() {
        let mut g = ResourceGovernor::new();
        // rate
        assert_eq!(
            g.check("m", ResourceDimension::Rate, 50),
            GovernanceAction::Allow
        );
        // memory — 32MB under default 64MB
        assert_eq!(
            g.check("m", ResourceDimension::Memory, 32 * 1024 * 1024),
            GovernanceAction::Allow
        );
        // time — 2500ms under default 5000ms
        assert_eq!(
            g.check("m", ResourceDimension::Time, 2_500),
            GovernanceAction::Allow
        );
        // count — 4 under default 8
        assert_eq!(
            g.check("m", ResourceDimension::Count, 4),
            GovernanceAction::Allow
        );
    }

    #[test]
    fn check_records_event_with_correct_dim() {
        let mut g = ResourceGovernor::new();
        g.check("test", ResourceDimension::Memory, 1024);
        assert_eq!(g.report.total(), 1);
        let by_dim = g.report.count_by_dimension();
        assert_eq!(by_dim.get(&ResourceDimension::Memory), Some(&1));
    }

    #[test]
    fn report_display_after_audit_all() {
        let r = resource_governance_summary();
        let displayed = format!("{r}");
        assert!(displayed.contains("dim rate"));
        assert!(displayed.contains("dim memory"));
        assert!(displayed.contains("dim time"));
        assert!(displayed.contains("dim count"));
    }
}
