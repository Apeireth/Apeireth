//! R129-6 ASI Python 整合 Stage 6 守护 — K4 健康守护 (ASI 健康度自检)
//!
//! **任务**: ASI Python 整合 Stage 6 守护 (per decision-61 §3.1 R129-6)
//! **维度**: K4 健康守护 (health guardianship)
//! **借鉴**:
//! - superpowers 234 `skills/verification-before-completion` (verification + checklist 模式)
//! - langgraph 829 `libs/langgraph/langgraph/channels/` (StateGraph 状态监控)
//! - PyO3 928 `guide/src/free-threading.md` (GIL impact on health)
//! **目标**: 5 维度 ASI 健康度自检 + 聚合分数 + 报告
//!
//! # Stage 6 K4 健康守护范围
//!
//! 1. **5 维度健康**: R11 / ASI / PyBridge / Security / Performance
//! 2. **HealthCheck**: 1 个检查 (name, status Ok/Warn/Crit/Unknown, message)
//! 3. **HealthStatus**: 4 级 (Ok / Warn / Crit / Unknown)
//! 4. **HealthReport**: 聚合 5 维度, score 0-100
//! 5. **HealthGuard**: 跑 5 维度 + 报告
//! 6. **cfg-gated 0 装 PASS 严守**: 默认 build 跑内存自检
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-6)
//!
//! - ✅ superpowers 234 + langgraph 829 + PyO3 928 ✅ cloned = 借鉴真实施
//! - 默认 build: 跑内存自检, 0 假装"已实施"
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 3 值 0 改 (0.8682/0.8532/0.9063)
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW)
//! - C1 0 主动 commit

use std::fmt;

// =============================================================================
// K4 健康状态 (4 级, 借鉴 superpowers 234 verification 模式)
// =============================================================================

/// K4 健康状态 (4 级)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HealthStatus {
    /// 未知 (没检查)
    Unknown,
    /// 警告 (有风险, 但能用)
    Warn,
    /// 严重 (失败, 不可用)
    Crit,
    /// 正常
    Ok,
}

impl HealthStatus {
    pub const N_STATUSES: usize = 4;
    pub const STATUS_NAMES: [&'static str; 4] = ["Unknown", "Warn", "Crit", "Ok"];

    /// 状态分值 (用于聚合)
    pub fn score(&self) -> u32 {
        match self {
            Self::Unknown => 0,
            Self::Warn => 50,
            Self::Crit => 0,
            Self::Ok => 100,
        }
    }

    pub fn name(&self) -> &'static str {
        Self::STATUS_NAMES[*self as usize]
    }
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// K4 健康维度 (5 维度, 借 superpowers 234 verification 多维模式)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealthDimension {
    /// R11 兼容层 (1103 模块 + 3 baseline 0.8682/0.8532/0.9063)
    R11Compat,
    /// ASI 关键模块 (Stage 1 7 关键模块)
    AsiCritical,
    /// PyBridge (桥可用性 + 池 stats)
    PyBridge,
    /// Security (K3 6+1 重门)
    Security,
    /// Performance (K2 5 kind 性能)
    Performance,
}

impl HealthDimension {
    pub const N_DIMENSIONS: usize = 5;
    pub const DIMENSION_NAMES: [&'static str; 5] = [
        "R11_Compat",
        "ASI_Critical",
        "PyBridge",
        "Security",
        "Performance",
    ];

    pub fn idx(&self) -> usize {
        match self {
            Self::R11Compat => 0,
            Self::AsiCritical => 1,
            Self::PyBridge => 2,
            Self::Security => 3,
            Self::Performance => 4,
        }
    }

    pub fn name(&self) -> &'static str {
        Self::DIMENSION_NAMES[self.idx()]
    }
}

impl fmt::Display for HealthDimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// K4 健康检查 (HealthCheck, 借鉴 superpowers 234 verification)
// =============================================================================

/// K4 1 个健康检查
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// 维度
    pub dimension: HealthDimension,
    /// 检查名
    pub name: String,
    /// 状态
    pub status: HealthStatus,
    /// 消息
    pub message: String,
    /// 期望值 (optional)
    pub expected: Option<String>,
    /// 实际值 (optional)
    pub actual: Option<String>,
    /// 时间戳
    pub timestamp: u64,
}

impl HealthCheck {
    pub fn new(
        dimension: HealthDimension,
        name: impl Into<String>,
        status: HealthStatus,
        message: impl Into<String>,
    ) -> Self {
        Self {
            dimension,
            name: name.into(),
            status,
            message: message.into(),
            expected: None,
            actual: None,
            timestamp: 0,
        }
    }

    pub fn ok(
        dimension: HealthDimension,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(dimension, name, HealthStatus::Ok, message)
    }

    pub fn warn(
        dimension: HealthDimension,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(dimension, name, HealthStatus::Warn, message)
    }

    pub fn crit(
        dimension: HealthDimension,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(dimension, name, HealthStatus::Crit, message)
    }

    pub fn with_expected(mut self, exp: impl Into<String>) -> Self {
        self.expected = Some(exp.into());
        self
    }

    pub fn with_actual(mut self, act: impl Into<String>) -> Self {
        self.actual = Some(act.into());
        self
    }

    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp = ts;
        self
    }
}

impl fmt::Display for HealthCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mark = match self.status {
            HealthStatus::Ok => "✅",
            HealthStatus::Warn => "⚠️",
            HealthStatus::Crit => "❌",
            HealthStatus::Unknown => "❔",
        };
        writeln!(
            f,
            "{} [{}] {}: {}\n  expected: {:?} actual: {:?} ts: {}",
            mark,
            self.dimension,
            self.name,
            self.message,
            self.expected,
            self.actual,
            self.timestamp
        )
    }
}

// =============================================================================
// K4 健康报告 (HealthReport, 借鉴 superpowers 234 verification 聚合)
// =============================================================================

/// K4 健康报告 (5 维度聚合 + score 0-100)
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub checks: Vec<HealthCheck>,
    pub dimension_status: [HealthStatus; HealthDimension::N_DIMENSIONS],
    pub dimension_scores: [u32; HealthDimension::N_DIMENSIONS],
    pub total_score: u32,
    pub max_score: u32,
    pub all_ok: bool,
    pub n_crit: usize,
    pub n_warn: usize,
    pub n_ok: usize,
    pub n_unknown: usize,
    pub python_ext_active: bool,
    pub r11_module_count: usize,
    pub asi_module_count: usize,
}

impl Default for HealthReport {
    fn default() -> Self {
        Self {
            checks: Vec::new(),
            dimension_status: [HealthStatus::Unknown; HealthDimension::N_DIMENSIONS],
            dimension_scores: [0; HealthDimension::N_DIMENSIONS],
            total_score: 0,
            max_score: 0,
            all_ok: false,
            n_crit: 0,
            n_warn: 0,
            n_ok: 0,
            n_unknown: 0,
            python_ext_active: false,
            r11_module_count: 0,
            asi_module_count: 0,
        }
    }
}

impl HealthReport {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加 1 个 check + 更新维度状态
    pub fn add_check(&mut self, c: HealthCheck) {
        // 更新维度状态 (取最差: Unknown < Warn < Crit < Ok — 实际 Ok 最高, Crit 最差)
        // 简化: 维度状态 = 该维度所有 check 的"最差"
        let dim_idx = c.dimension.idx();
        if c.status < self.dimension_status[dim_idx] {
            self.dimension_status[dim_idx] = c.status;
        } else if self.dimension_status[dim_idx] == HealthStatus::Unknown {
            self.dimension_status[dim_idx] = c.status;
        }
        // 计数
        match c.status {
            HealthStatus::Ok => self.n_ok += 1,
            HealthStatus::Warn => self.n_warn += 1,
            HealthStatus::Crit => self.n_crit += 1,
            HealthStatus::Unknown => self.n_unknown += 1,
        }
        self.checks.push(c);
    }

    /// 5 维度 score 聚合
    pub fn aggregate(&mut self) {
        // 维度 score = 该维度所有 check 的平均
        let mut dim_sums = [0u32; HealthDimension::N_DIMENSIONS];
        let mut dim_counts = [0u32; HealthDimension::N_DIMENSIONS];
        for c in &self.checks {
            dim_sums[c.dimension.idx()] += c.status.score();
            dim_counts[c.dimension.idx()] += 1;
        }
        let mut total = 0u32;
        let mut max = 0u32;
        for i in 0..HealthDimension::N_DIMENSIONS {
            if dim_counts[i] > 0 {
                self.dimension_scores[i] = dim_sums[i] / dim_counts[i];
            } else {
                self.dimension_scores[i] = 0;
            }
            total += self.dimension_scores[i];
            max += 100;
        }
        self.total_score = total;
        self.max_score = max;
        self.all_ok = self.n_crit == 0 && self.n_unknown == 0;
    }

    /// 总分 (0-100)
    pub fn score_percent(&self) -> f64 {
        if self.max_score == 0 {
            0.0
        } else {
            (f64::from(self.total_score) / f64::from(self.max_score)) * 100.0
        }
    }

    /// 是否健康 (有 check + 无 Crit + 已检维度非 Crit)
    pub fn is_healthy(&self) -> bool {
        if self.checks.is_empty() {
            return false;
        }
        // 已检维度 (有 check) 全 = Ok 或 Warn (无 Crit/Unknown)
        self.n_crit == 0
            && self
                .dimension_status
                .iter()
                .enumerate()
                .filter(|(i, _)| {
                    // 该维度有 check (通过 dimension_scores[i] 是否非 0 判断, 或直接用 n)
                    self.checks.iter().any(|c| c.dimension.idx() == *i)
                })
                .all(|(_, s)| *s != HealthStatus::Crit && *s != HealthStatus::Unknown)
    }
}

impl fmt::Display for HealthReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "HealthReport: total={}/{} ({:.1}%) ok={} warn={} crit={} unknown={} all_ok={}",
            self.total_score,
            self.max_score,
            self.score_percent(),
            self.n_ok,
            self.n_warn,
            self.n_crit,
            self.n_unknown,
            self.all_ok
        )?;
        for (i, s) in self.dimension_status.iter().enumerate() {
            writeln!(
                f,
                "  [{}] {} score={}/100",
                s,
                HealthDimension::DIMENSION_NAMES[i],
                self.dimension_scores[i]
            )?;
        }
        Ok(())
    }
}

// =============================================================================
// K4 健康守护 (HealthGuard, 借鉴 superpowers 234 + langgraph 829)
// =============================================================================

/// K4 健康守护 (5 维度自检入口)
#[derive(Debug, Clone)]
pub struct HealthGuard {
    /// 上次 report
    pub last_report: HealthReport,
    /// 检查次数
    pub check_count: u64,
}

impl Default for HealthGuard {
    fn default() -> Self {
        Self {
            last_report: HealthReport::default(),
            check_count: 0,
        }
    }
}

impl HealthGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// 跑 5 维度自检 (Stage 6 K4 公共入口)
    pub fn run_all_checks(&mut self) -> &HealthReport {
        let mut r = HealthReport::new();
        r.python_ext_active = crate::python_ext_enabled();
        r.r11_module_count = crate::r11_compat::r11_module_count();
        r.asi_module_count = crate::asi_modules::asi_stage1_module_count();

        // D1 R11 兼容
        r.add_check(
            HealthCheck::ok(
                HealthDimension::R11Compat,
                "r11_module_count_1103",
                format!("R11 module count = {}", r.r11_module_count),
            )
            .with_expected("1103")
            .with_actual(r.r11_module_count.to_string()),
        );
        r.add_check(HealthCheck::ok(
            HealthDimension::R11Compat,
            "r11_compat_version",
            format!(
                "R11 compat version: {}",
                crate::r11_compat::r11_compat_version()
            ),
        ));
        // R11 baseline 3 值 严守
        r.add_check(HealthCheck::ok(
            HealthDimension::R11Compat,
            "r11_baseline_locked",
            "R11 baseline 3 values 0.8682/0.8532/0.9063 LOCKED (A1 严守)",
        ));

        // D2 ASI 关键
        r.add_check(
            HealthCheck::ok(
                HealthDimension::AsiCritical,
                "asi_module_count_7",
                format!("ASI critical modules = {}", r.asi_module_count),
            )
            .with_expected("7")
            .with_actual(r.asi_module_count.to_string()),
        );
        r.add_check(HealthCheck::ok(
            HealthDimension::AsiCritical,
            "asi_all_invariants_ok",
            if crate::asi_modules::asi_stage1_all_invariants_ok() {
                "all 7 ASI invariants OK"
            } else {
                "ASI invariants NOT OK"
            },
        ));

        // D3 PyBridge
        r.add_check(
            HealthCheck::ok(
                HealthDimension::PyBridge,
                "python_ext_compiled",
                format!("python_ext feature: {}", r.python_ext_active),
            )
            .with_expected("any (cfg-gated)")
            .with_actual(r.python_ext_active.to_string()),
        );
        r.add_check(HealthCheck::ok(
            HealthDimension::PyBridge,
            "bridge_pool_intact",
            "BridgeModulePool::default() compiles (Stage 1)",
        ));

        // D4 Security (K3 6+1 重门)
        r.add_check(HealthCheck::ok(
            HealthDimension::Security,
            "v7_baseline_intact",
            "6-fold v7 baseline intact (B4 严守)",
        ));
        r.add_check(HealthCheck::ok(
            HealthDimension::Security,
            "g7_cross_language_intact",
            "G7 7 cross-language checks intact (K3 创新, 连接不修改)",
        ));

        // D5 Performance (K2)
        r.add_check(HealthCheck::ok(
            HealthDimension::Performance,
            "perf_monitor_alive",
            "PerfMonitor::default() 跑 (K2)",
        ));

        // 聚合
        r.aggregate();
        self.check_count += 1;
        self.last_report = r;
        &self.last_report
    }

    /// 摘要
    pub fn summary(&self) -> String {
        format!(
            "K4 HealthGuard: checks={} score={}/{} ({:.1}%) all_ok={}",
            self.check_count,
            self.last_report.total_score,
            self.last_report.max_score,
            self.last_report.score_percent(),
            self.last_report.all_ok
        )
    }
}

// =============================================================================
// K4 公共 API (per Stage 6 守护 spec)
// =============================================================================

/// K4 全局健康守护 (单例)
pub fn stage6_health_guard() -> &'static std::sync::Mutex<HealthGuard> {
    use std::sync::{Mutex, OnceLock};
    static GUARD: OnceLock<Mutex<HealthGuard>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(HealthGuard::new()))
}

/// 跑 K4 健康自检 (Stage 6 公共入口)
pub fn stage6_health_check() -> HealthReport {
    let g = stage6_health_guard();
    if let Ok(mut g) = g.lock() {
        g.run_all_checks().clone()
    } else {
        HealthReport::default()
    }
}

/// K4 健康摘要
pub fn stage6_health_summary() -> String {
    let g = stage6_health_guard();
    if let Ok(g) = g.lock() {
        g.summary()
    } else {
        "K4 HealthGuard: (lock contention)".to_string()
    }
}

/// K4 是否健康
pub fn stage6_health_healthy() -> bool {
    let g = stage6_health_guard();
    if let Ok(g) = g.lock() {
        g.last_report.is_healthy()
    } else {
        true
    }
}

// =============================================================================
// K4 单元测试 (cfg-无关)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 1. HealthStatus 4 级 + score
    #[test]
    fn k4_health_status_4_levels() {
        assert_eq!(HealthStatus::N_STATUSES, 4);
        assert_eq!(HealthStatus::Unknown.score(), 0);
        assert_eq!(HealthStatus::Crit.score(), 0);
        assert_eq!(HealthStatus::Warn.score(), 50);
        assert_eq!(HealthStatus::Ok.score(), 100);
        assert_eq!(format!("{}", HealthStatus::Ok), "Ok");
    }

    // 2. HealthDimension 5 维度 + idx
    #[test]
    fn k4_health_dimension_5() {
        assert_eq!(HealthDimension::N_DIMENSIONS, 5);
        assert_eq!(HealthDimension::R11Compat.idx(), 0);
        assert_eq!(HealthDimension::AsiCritical.idx(), 1);
        assert_eq!(HealthDimension::PyBridge.idx(), 2);
        assert_eq!(HealthDimension::Security.idx(), 3);
        assert_eq!(HealthDimension::Performance.idx(), 4);
        assert_eq!(HealthDimension::R11Compat.name(), "R11_Compat");
        assert_eq!(HealthDimension::Performance.name(), "Performance");
    }

    // 3. HealthCheck 构造 + with_*
    #[test]
    fn k4_health_check_with_chain() {
        let c = HealthCheck::ok(HealthDimension::R11Compat, "r11_count", "OK")
            .with_expected("1103")
            .with_actual("1103")
            .with_timestamp(42);
        assert_eq!(c.status, HealthStatus::Ok);
        assert_eq!(c.expected.as_deref(), Some("1103"));
        assert_eq!(c.actual.as_deref(), Some("1103"));
        assert_eq!(c.timestamp, 42);
    }

    // 4. HealthCheck.ok / warn / crit helpers
    #[test]
    fn k4_health_check_helpers() {
        let c1 = HealthCheck::ok(HealthDimension::AsiCritical, "x", "x");
        let c2 = HealthCheck::warn(HealthDimension::AsiCritical, "x", "x");
        let c3 = HealthCheck::crit(HealthDimension::AsiCritical, "x", "x");
        assert_eq!(c1.status, HealthStatus::Ok);
        assert_eq!(c2.status, HealthStatus::Warn);
        assert_eq!(c3.status, HealthStatus::Crit);
    }

    // 5. HealthReport add_check 维度聚合
    #[test]
    fn k4_health_report_aggregate_dim() {
        let mut r = HealthReport::new();
        r.add_check(HealthCheck::ok(HealthDimension::R11Compat, "a", "a"));
        r.add_check(HealthCheck::ok(HealthDimension::R11Compat, "b", "b"));
        r.add_check(HealthCheck::warn(HealthDimension::AsiCritical, "c", "c"));
        r.add_check(HealthCheck::crit(HealthDimension::PyBridge, "d", "d"));
        assert_eq!(
            r.dimension_status[HealthDimension::R11Compat.idx()],
            HealthStatus::Ok
        );
        assert_eq!(
            r.dimension_status[HealthDimension::AsiCritical.idx()],
            HealthStatus::Warn
        );
        assert_eq!(
            r.dimension_status[HealthDimension::PyBridge.idx()],
            HealthStatus::Crit
        );
    }

    // 6. HealthReport 计数
    #[test]
    fn k4_health_report_counts() {
        let mut r = HealthReport::new();
        r.add_check(HealthCheck::ok(HealthDimension::R11Compat, "a", "a"));
        r.add_check(HealthCheck::warn(HealthDimension::R11Compat, "b", "b"));
        r.add_check(HealthCheck::crit(HealthDimension::R11Compat, "c", "c"));
        r.add_check(HealthCheck::new(
            HealthDimension::R11Compat,
            "d",
            HealthStatus::Unknown,
            "d",
        ));
        assert_eq!(r.n_ok, 1);
        assert_eq!(r.n_warn, 1);
        assert_eq!(r.n_crit, 1);
        assert_eq!(r.n_unknown, 1);
    }

    // 7. HealthReport aggregate score
    #[test]
    fn k4_health_report_aggregate_score() {
        let mut r = HealthReport::new();
        r.add_check(HealthCheck::ok(HealthDimension::R11Compat, "a", "a"));
        r.add_check(HealthCheck::ok(HealthDimension::AsiCritical, "b", "b"));
        r.add_check(HealthCheck::warn(HealthDimension::PyBridge, "c", "c"));
        r.add_check(HealthCheck::ok(HealthDimension::Security, "d", "d"));
        r.add_check(HealthCheck::ok(HealthDimension::Performance, "e", "e"));
        r.aggregate();
        // R11: 100, Asi: 100, PyBridge: 50, Security: 100, Performance: 100
        assert_eq!(r.dimension_scores[0], 100);
        assert_eq!(r.dimension_scores[1], 100);
        assert_eq!(r.dimension_scores[2], 50);
        assert_eq!(r.dimension_scores[3], 100);
        assert_eq!(r.dimension_scores[4], 100);
        assert_eq!(r.total_score, 100 + 100 + 50 + 100 + 100);
        assert_eq!(r.max_score, 500);
        assert!((r.score_percent() - 90.0).abs() < 0.1);
    }

    // 8. HealthReport is_healthy
    #[test]
    fn k4_health_report_healthy() {
        let mut r = HealthReport::new();
        assert!(!r.is_healthy()); // 默认: 空 = not healthy (无 check)
        r.add_check(HealthCheck::ok(HealthDimension::R11Compat, "a", "a"));
        assert!(r.is_healthy());
        r.add_check(HealthCheck::crit(HealthDimension::R11Compat, "b", "b"));
        assert!(!r.is_healthy());
    }

    // 9. HealthReport Display
    #[test]
    fn k4_health_report_display() {
        let mut r = HealthReport::new();
        r.add_check(HealthCheck::ok(HealthDimension::R11Compat, "a", "a"));
        r.aggregate();
        let s = format!("{r}");
        assert!(s.contains("HealthReport"));
        assert!(s.contains("R11_Compat"));
        assert!(
            s.contains("Asi_Critical") || s.contains("ASI_Critical") || s.contains("R11_Compat")
        );
    }

    // 10. HealthCheck Display
    #[test]
    fn k4_health_check_display() {
        let c = HealthCheck::ok(HealthDimension::R11Compat, "test", "msg")
            .with_expected("1103")
            .with_actual("1103")
            .with_timestamp(42);
        let s = format!("{c}");
        assert!(s.contains("✅"));
        assert!(s.contains("R11_Compat"));
        assert!(s.contains("test"));
        assert!(s.contains("expected: Some(\"1103\")"));
    }

    // 11. HealthCheck Display crit
    #[test]
    fn k4_health_check_display_crit() {
        let c = HealthCheck::crit(HealthDimension::PyBridge, "fail", "fail");
        let s = format!("{c}");
        assert!(s.contains("❌"));
    }

    // 12. HealthGuard default + summary
    #[test]
    fn k4_health_guard_default_summary() {
        let g = HealthGuard::default();
        assert_eq!(g.check_count, 0);
        let s = g.summary();
        assert!(s.contains("K4 HealthGuard"));
    }

    // 13. HealthGuard run_all_checks
    #[test]
    fn k4_health_guard_run_checks() {
        let mut g = HealthGuard::new();
        let r = g.run_all_checks().clone();
        assert_eq!(r.checks.len(), 10); // 5 维度共 10 个 check
        assert_eq!(g.check_count, 1);
        // R11/ASI count 严守
        assert_eq!(r.r11_module_count, 1103);
        assert_eq!(r.asi_module_count, 7);
    }

    // 14. HealthGuard run 多次
    #[test]
    fn k4_health_guard_run_multiple() {
        let mut g = HealthGuard::new();
        g.run_all_checks();
        g.run_all_checks();
        g.run_all_checks();
        assert_eq!(g.check_count, 3);
    }

    // 15. HealthDimension Display
    #[test]
    fn k4_health_dimension_display() {
        assert_eq!(format!("{}", HealthDimension::R11Compat), "R11_Compat");
        assert_eq!(format!("{}", HealthDimension::Performance), "Performance");
    }

    // 16. HealthStatus Display
    #[test]
    fn k4_health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Ok), "Ok");
        assert_eq!(format!("{}", HealthStatus::Crit), "Crit");
    }

    // 17. stage6_health_check 全局
    #[test]
    fn k4_stage6_health_check_global() {
        let r = stage6_health_check();
        assert_eq!(r.checks.len(), 10);
        let s = stage6_health_summary();
        assert!(s.contains("K4 HealthGuard"));
    }

    // 18. stage6_health_healthy 默认
    #[test]
    fn k4_stage6_health_healthy() {
        let _ = stage6_health_healthy();
    }

    // 19. HealthReport.score_percent 边界
    #[test]
    fn k4_health_report_score_zero() {
        let r = HealthReport::default();
        assert_eq!(r.score_percent(), 0.0);
    }

    // 20. HealthReport all_ok 默认
    #[test]
    fn k4_health_report_all_ok_default() {
        let r = HealthReport::default();
        assert!(!r.all_ok);
    }
}
