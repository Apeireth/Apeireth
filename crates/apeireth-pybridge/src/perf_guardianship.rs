//! R129-6 ASI Python 整合 Stage 6 守护 — K2 性能守护 (跨语言性能监控)
//!
//! **任务**: ASI Python 整合 Stage 6 守护 (per decision-61 §3.1 R129-6)
//! **维度**: K2 性能守护 (performance guardianship)
//! **借鉴**:
//! - PyO3 928 `guide/src/performance.md` (GIL release 模式 + `Python::allow_threads` + 性能微基准)
//! - PyO3 928 `guide/src/free-threading.md` (3.13 free-threaded build + GIL impact on perf)
//! - superpowers 234 `skills/verification-before-completion` (perf 测量 + 阈值 verify)
//! **目标**: 跨语言性能监控 + 阈值告警 + 性能聚合
//!
//! # Stage 6 K2 性能守护范围
//!
//! 1. **性能采样**: PerfSample (latency_us + kind + success + timestamp)
//! 2. **性能窗口**: PerfWindow (rolling 256 samples, LRU)
//! 3. **性能聚合**: PerfStats (count, mean, p50, p95, p99, min, max, throughput)
//! 4. **性能分类**: 5 类 (Bridge / Eval / Import / Convert / Call)
//! 5. **阈值告警**: p95 > threshold_per_kind
//! 6. **cfg-gated 0 装 PASS 严守**: 默认 build 跑内存 ring buffer
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-6)
//!
//! - ✅ PyO3 928 + superpowers 234 ✅ cloned = 借鉴真实施
//! - 默认 build: 内存 ring buffer 跑, 0 假装"已实施"
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 3 值 0 改
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW)
//! - C1 0 主动 commit

use std::fmt;
use std::time::{Duration, Instant};

// =============================================================================
// K2 性能分类 (5 类, 借鉴 PyO3 928 performance.md + free-threading.md)
// =============================================================================

/// K2 性能分类 (5 类, 跨语言性能监控维度)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerfKind {
    /// 桥接 (Python ↔ Rust 跨 GIL 桥, 含 GIL acquire/release 开销)
    Bridge,
    /// 求值 (eval 表达式, 借 PyO3 py.eval)
    Eval,
    /// 导入 (import Python 模块, 借 PyO3 PyImport)
    Import,
    /// 转换 (类型转换, 借 PyO3 conversions + type_convert.rs)
    Convert,
    /// 调用 (call Python 函数, 借 PyO3 call_function)
    Call,
}

impl PerfKind {
    /// 类别数 (5 类, 编译期 hardcode)
    pub const N_KINDS: usize = 5;
    /// 类别名
    pub const KIND_NAMES: [&'static str; 5] = ["Bridge", "Eval", "Import", "Convert", "Call"];

    /// 默认阈值 (μs, p95 上限; 借 superpowers 234 verification-before-completion 性能规范)
    pub const DEFAULT_THRESHOLDS_US: [u128; 5] = [
        500,  // Bridge: 跨 GIL 桥, p95 < 500μs
        1000, // Eval: 表达式求值, p95 < 1000μs
        5000, // Import: 模块导入, p95 < 5000μs (含首次 import)
        100,  // Convert: 类型转换, p95 < 100μs (轻量)
        800,  // Call: 函数调用, p95 < 800μs
    ];

    pub fn idx(&self) -> usize {
        match self {
            Self::Bridge => 0,
            Self::Eval => 1,
            Self::Import => 2,
            Self::Convert => 3,
            Self::Call => 4,
        }
    }

    pub fn name(&self) -> &'static str {
        Self::KIND_NAMES[self.idx()]
    }

    /// 默认阈值 (p95 上限, μs)
    pub fn default_threshold_us(&self) -> u128 {
        Self::DEFAULT_THRESHOLDS_US[self.idx()]
    }
}

impl fmt::Display for PerfKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// K2 性能采样 (PerfSample, 借鉴 PyO3 928 performance.md + superpowers 234)
// =============================================================================

/// K2 1 个性能采样
#[derive(Debug, Clone)]
pub struct PerfSample {
    /// 性能分类
    pub kind: PerfKind,
    /// 延迟 (μs, 微秒)
    pub latency_us: u128,
    /// 是否成功
    pub success: bool,
    /// 错误消息 (失败时, optional)
    pub error: Option<String>,
    /// 阈值 (μs, 当前 kind 的 p95 上限)
    pub threshold_us: u128,
    /// 是否超阈值
    pub over_threshold: bool,
}

impl PerfSample {
    /// 构造 (从 Duration)
    pub fn from_duration(kind: PerfKind, elapsed: Duration, success: bool) -> Self {
        let latency_us = elapsed.as_micros();
        let threshold_us = kind.default_threshold_us();
        Self {
            kind,
            latency_us,
            success,
            error: None,
            threshold_us,
            over_threshold: latency_us > threshold_us,
        }
    }

    /// 构造 (从 μs)
    pub fn from_us(kind: PerfKind, latency_us: u128, success: bool) -> Self {
        let threshold_us = kind.default_threshold_us();
        Self {
            kind,
            latency_us,
            success,
            error: None,
            threshold_us,
            over_threshold: latency_us > threshold_us,
        }
    }

    /// 设 error
    pub fn with_error(mut self, err: impl Into<String>) -> Self {
        self.error = Some(err.into());
        self
    }

    /// 设 自定义 threshold (覆盖默认)
    pub fn with_threshold(mut self, threshold_us: u128) -> Self {
        self.threshold_us = threshold_us;
        self.over_threshold = self.latency_us > threshold_us;
        self
    }
}

impl fmt::Display for PerfSample {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mark = if self.over_threshold { "⚠️" } else { "✅" };
        let status = if self.success { "ok" } else { "fail" };
        writeln!(
            f,
            "{} [{}{}] {}: {}μs (threshold={}μs)",
            mark,
            self.kind,
            if self.success { "" } else { "|fail" },
            status,
            self.latency_us,
            self.threshold_us
        )
    }
}

// =============================================================================
// K2 性能统计 (PerfStats, 借鉴 superpowers 234 BenchStats 模式)
// =============================================================================

/// K2 性能统计 (1 个 kind 的聚合)
#[derive(Debug, Clone, Copy)]
pub struct PerfStats {
    pub count: u64,
    pub mean_us: f64,
    pub p50_us: u128,
    pub p95_us: u128,
    pub p99_us: u128,
    pub min_us: u128,
    pub max_us: u128,
    pub success_count: u64,
    pub failure_count: u64,
    pub over_threshold_count: u64,
    /// 吞吐 (samples/sec, 估; 借 stage3_bench 5 target × 100 iter 模式)
    pub throughput_per_sec: f64,
}

impl PerfStats {
    /// 空 stats
    pub fn empty() -> Self {
        Self {
            count: 0,
            mean_us: 0.0,
            p50_us: 0,
            p95_us: 0,
            p99_us: 0,
            min_us: 0,
            max_us: 0,
            success_count: 0,
            failure_count: 0,
            over_threshold_count: 0,
            throughput_per_sec: 0.0,
        }
    }

    /// 从样本计算
    pub fn from_samples(samples: &[PerfSample]) -> Self {
        if samples.is_empty() {
            return Self::empty();
        }
        let mut sorted: Vec<u128> = samples.iter().map(|s| s.latency_us).collect();
        sorted.sort();
        let count = samples.len() as u64;
        let sum: u128 = sorted.iter().sum();
        let mean_us = sum as f64 / count as f64;
        let p50 = sorted[sorted.len() / 2];
        let p95_idx = ((sorted.len() as f64 * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1);
        let p99_idx = ((sorted.len() as f64 * 0.99).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1);
        let p95 = sorted[p95_idx];
        let p99 = sorted[p99_idx];
        let min = *sorted.first().unwrap();
        let max = *sorted.last().unwrap();
        let success = samples.iter().filter(|s| s.success).count() as u64;
        let over = samples.iter().filter(|s| s.over_threshold).count() as u64;
        Self {
            count,
            mean_us,
            p50_us: p50,
            p95_us: p95,
            p99_us: p99,
            min_us: min,
            max_us: max,
            success_count: success,
            failure_count: count - success,
            over_threshold_count: over,
            throughput_per_sec: count as f64 / (mean_us * count as f64 / 1_000_000.0).max(1.0),
        }
    }

    /// 失败率
    pub fn failure_rate(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.failure_count as f64 / self.count as f64
        }
    }

    /// 超阈率
    pub fn over_threshold_rate(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.over_threshold_count as f64 / self.count as f64
        }
    }
}

impl fmt::Display for PerfStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "PerfStats: n={} mean={:.2}μs p50={}μs p95={}μs p99={}μs min={}μs max={}μs",
            self.count,
            self.mean_us,
            self.p50_us,
            self.p95_us,
            self.p99_us,
            self.min_us,
            self.max_us
        )?;
        writeln!(
            f,
            "  success={}/{} failure_rate={:.2} over_threshold={}/{} over_rate={:.2} throughput={:.0}/s",
            self.success_count, self.count, self.failure_rate(),
            self.over_threshold_count, self.count, self.over_threshold_rate(),
            self.throughput_per_sec
        )
    }
}

// =============================================================================
// K2 性能监控 (PerfMonitor, 借鉴 superpowers 234 SkillRegistry 模式)
// =============================================================================

/// K2 性能监控 (5 kind × 256 sample ring buffer, cfg-无关)
#[derive(Debug, Clone)]
pub struct PerfMonitor {
    /// 5 kind 的样本列表 (按 kind idx)
    pub samples: [Vec<PerfSample>; PerfKind::N_KINDS],
    /// 5 kind 的最大样本数
    pub max_per_kind: usize,
    /// 总记录数
    pub total_recorded: u64,
    /// 总丢弃数 (LRU overflow)
    pub total_dropped: u64,
}

impl Default for PerfMonitor {
    fn default() -> Self {
        Self::new(256)
    }
}

impl PerfMonitor {
    pub fn new(max_per_kind: usize) -> Self {
        Self {
            samples: [
                Vec::with_capacity(max_per_kind),
                Vec::with_capacity(max_per_kind),
                Vec::with_capacity(max_per_kind),
                Vec::with_capacity(max_per_kind),
                Vec::with_capacity(max_per_kind),
            ],
            max_per_kind,
            total_recorded: 0,
            total_dropped: 0,
        }
    }

    /// 记录样本
    pub fn record(&mut self, sample: PerfSample) {
        let idx = sample.kind.idx();
        if self.samples[idx].len() >= self.max_per_kind {
            self.samples[idx].remove(0);
            self.total_dropped += 1;
        }
        self.samples[idx].push(sample);
        self.total_recorded += 1;
    }

    /// 记录 (从 Duration, 简化入口)
    pub fn record_duration(&mut self, kind: PerfKind, elapsed: Duration, success: bool) {
        self.record(PerfSample::from_duration(kind, elapsed, success));
    }

    /// 1 个 kind 的 stats
    pub fn stats(&self, kind: PerfKind) -> PerfStats {
        PerfStats::from_samples(&self.samples[kind.idx()])
    }

    /// 全部 5 kind 的 stats
    pub fn all_stats(&self) -> [PerfStats; PerfKind::N_KINDS] {
        let mut out = [PerfStats::empty(); PerfKind::N_KINDS];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = PerfStats::from_samples(&self.samples[i]);
        }
        out
    }

    /// 阈值告警 (p95 > threshold)
    pub fn alerts(&self) -> Vec<(PerfKind, u128, u128)> {
        // (kind, p95_us, threshold_us)
        self.all_stats()
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let kind = match i {
                    0 => PerfKind::Bridge,
                    1 => PerfKind::Eval,
                    2 => PerfKind::Import,
                    3 => PerfKind::Convert,
                    _ => PerfKind::Call,
                };
                let th = kind.default_threshold_us();
                if s.p95_us > th && s.count > 0 {
                    Some((kind, s.p95_us, th))
                } else {
                    None
                }
            })
            .collect()
    }

    /// 是否健康 (无阈值告警)
    pub fn is_healthy(&self) -> bool {
        self.alerts().is_empty()
    }

    /// 摘要
    pub fn summary(&self) -> String {
        let mut parts = vec![format!(
            "K2 PerfMonitor: recorded={} dropped={}",
            self.total_recorded, self.total_dropped
        )];
        for (i, s) in self.all_stats().iter().enumerate() {
            if s.count > 0 {
                let kind = match i {
                    0 => "Bridge",
                    1 => "Eval",
                    2 => "Import",
                    3 => "Convert",
                    _ => "Call",
                };
                parts.push(format!("{}[n={} p95={}μs]", kind, s.count, s.p95_us));
            }
        }
        parts.join(" ")
    }
}

// =============================================================================
// K2 公共 API (per Stage 6 守护 spec)
// =============================================================================

/// K2 全局性能监控 (单例)
pub fn stage6_perf_monitor() -> &'static std::sync::Mutex<PerfMonitor> {
    use std::sync::{Mutex, OnceLock};
    static MON: OnceLock<Mutex<PerfMonitor>> = OnceLock::new();
    MON.get_or_init(|| Mutex::new(PerfMonitor::default()))
}

/// 记录 K2 性能 (Stage 6 公共入口, 从 Duration)
pub fn stage6_record_perf(kind: PerfKind, elapsed: Duration, success: bool) -> PerfSample {
    let s = PerfSample::from_duration(kind, elapsed, success);
    let m = stage6_perf_monitor();
    if let Ok(mut m) = m.lock() {
        m.record(s.clone());
    }
    s
}

/// K2 摘要
pub fn stage6_perf_summary() -> String {
    let m = stage6_perf_monitor();
    if let Ok(m) = m.lock() {
        m.summary()
    } else {
        "K2 PerfMonitor: (lock contention)".to_string()
    }
}

/// K2 健康检查 (无阈值告警)
pub fn stage6_perf_healthy() -> bool {
    let m = stage6_perf_monitor();
    if let Ok(m) = m.lock() {
        m.is_healthy()
    } else {
        true
    }
}

/// K2 阈值告警列表
pub fn stage6_perf_alerts() -> Vec<(PerfKind, u128, u128)> {
    let m = stage6_perf_monitor();
    if let Ok(m) = m.lock() {
        m.alerts()
    } else {
        Vec::new()
    }
}

// =============================================================================
// K2 单元测试 (cfg-无关)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 1. PerfKind 5 类 idx + threshold 严守
    #[test]
    fn k2_perf_kind_5_kinds() {
        assert_eq!(PerfKind::N_KINDS, 5);
        assert_eq!(PerfKind::KIND_NAMES.len(), 5);
        assert_eq!(PerfKind::Bridge.idx(), 0);
        assert_eq!(PerfKind::Eval.idx(), 1);
        assert_eq!(PerfKind::Import.idx(), 2);
        assert_eq!(PerfKind::Convert.idx(), 3);
        assert_eq!(PerfKind::Call.idx(), 4);
        assert_eq!(PerfKind::Bridge.name(), "Bridge");
        assert_eq!(PerfKind::Call.name(), "Call");
    }

    // 2. PerfKind 默认阈值
    #[test]
    fn k2_perf_kind_thresholds() {
        assert_eq!(PerfKind::Bridge.default_threshold_us(), 500);
        assert_eq!(PerfKind::Eval.default_threshold_us(), 1000);
        assert_eq!(PerfKind::Import.default_threshold_us(), 5000);
        assert_eq!(PerfKind::Convert.default_threshold_us(), 100);
        assert_eq!(PerfKind::Call.default_threshold_us(), 800);
    }

    // 3. PerfSample from_us
    #[test]
    fn k2_perf_sample_from_us() {
        let s = PerfSample::from_us(PerfKind::Bridge, 200, true);
        assert_eq!(s.latency_us, 200);
        assert!(s.success);
        assert!(!s.over_threshold);
    }

    // 4. PerfSample 超阈值
    #[test]
    fn k2_perf_sample_over_threshold() {
        let s = PerfSample::from_us(PerfKind::Convert, 200, true);
        // Convert 阈值 100μs, 200 > 100 → 超阈
        assert!(s.over_threshold);
    }

    // 5. PerfSample with_threshold 自定义
    #[test]
    fn k2_perf_sample_with_threshold() {
        let s = PerfSample::from_us(PerfKind::Bridge, 200, true).with_threshold(50);
        assert!(s.over_threshold);
        assert_eq!(s.threshold_us, 50);
    }

    // 6. PerfSample with_error
    #[test]
    fn k2_perf_sample_with_error() {
        let s = PerfSample::from_us(PerfKind::Call, 100, false).with_error("timeout");
        assert!(!s.success);
        assert_eq!(s.error.as_deref(), Some("timeout"));
    }

    // 7. PerfStats empty
    #[test]
    fn k2_perf_stats_empty() {
        let s = PerfStats::empty();
        assert_eq!(s.count, 0);
        assert_eq!(s.mean_us, 0.0);
        assert_eq!(s.p95_us, 0);
    }

    // 8. PerfStats from_samples 聚合
    #[test]
    fn k2_perf_stats_aggregate() {
        let samples: Vec<PerfSample> = (0..100)
            .map(|i| PerfSample::from_us(PerfKind::Bridge, i as u128, true))
            .collect();
        let s = PerfStats::from_samples(&samples);
        assert_eq!(s.count, 100);
        assert_eq!(s.min_us, 0);
        assert_eq!(s.max_us, 99);
        assert_eq!(s.success_count, 100);
        assert_eq!(s.failure_count, 0);
        // mean = (0+1+...+99)/100 = 49.5
        assert!((s.mean_us - 49.5).abs() < 0.5);
    }

    // 9. PerfStats failure_rate
    #[test]
    fn k2_perf_stats_failure_rate() {
        let samples: Vec<PerfSample> = (0..10)
            .map(|i| PerfSample::from_us(PerfKind::Bridge, i as u128, i % 2 == 0))
            .collect();
        let s = PerfStats::from_samples(&samples);
        assert_eq!(s.failure_count, 5);
        assert!((s.failure_rate() - 0.5).abs() < 0.01);
    }

    // 10. PerfMonitor record + 5 kind 隔离
    #[test]
    fn k2_perf_monitor_5_kind_isolation() {
        let mut m = PerfMonitor::default();
        m.record(PerfSample::from_us(PerfKind::Bridge, 100, true));
        m.record(PerfSample::from_us(PerfKind::Eval, 200, true));
        m.record(PerfSample::from_us(PerfKind::Import, 300, true));
        m.record(PerfSample::from_us(PerfKind::Convert, 50, true));
        m.record(PerfSample::from_us(PerfKind::Call, 150, true));
        assert_eq!(m.samples[PerfKind::Bridge.idx()].len(), 1);
        assert_eq!(m.samples[PerfKind::Eval.idx()].len(), 1);
        assert_eq!(m.samples[PerfKind::Import.idx()].len(), 1);
        assert_eq!(m.samples[PerfKind::Convert.idx()].len(), 1);
        assert_eq!(m.samples[PerfKind::Call.idx()].len(), 1);
        assert_eq!(m.total_recorded, 5);
    }

    // 11. PerfMonitor LRU 滚动
    #[test]
    fn k2_perf_monitor_lru_overflow() {
        let mut m = PerfMonitor::new(3);
        for i in 0..5 {
            m.record(PerfSample::from_us(PerfKind::Bridge, i, true));
        }
        assert_eq!(m.samples[PerfKind::Bridge.idx()].len(), 3);
        assert_eq!(m.total_dropped, 2);
    }

    // 12. PerfMonitor is_healthy 无超阈
    #[test]
    fn k2_perf_monitor_healthy_no_alert() {
        let mut m = PerfMonitor::default();
        m.record(PerfSample::from_us(PerfKind::Convert, 50, true)); // < 100
        m.record(PerfSample::from_us(PerfKind::Bridge, 200, true)); // < 500
        assert!(m.is_healthy());
        assert!(m.alerts().is_empty());
    }

    // 13. PerfMonitor alerts 超阈
    #[test]
    fn k2_perf_monitor_alerts_over_threshold() {
        let mut m = PerfMonitor::default();
        // 多个 Convert 样本, p95 > 100
        for i in 1..=20 {
            m.record(PerfSample::from_us(PerfKind::Convert, i * 20, true));
        }
        let alerts = m.alerts();
        // Convert p95 应 > 100 (max = 400)
        assert!(!alerts.is_empty());
        let (kind, _p95, th) = &alerts[0];
        assert_eq!(*kind, PerfKind::Convert);
        assert_eq!(*th, 100);
    }

    // 14. stage6_record_perf 全局
    #[test]
    fn k2_stage6_record_perf_global() {
        let s = stage6_record_perf(PerfKind::Call, Duration::from_micros(100), true);
        assert_eq!(s.kind, PerfKind::Call);
        assert!(stage6_perf_summary().contains("K2 PerfMonitor"));
    }

    // 15. stage6_perf_alerts 公共 API
    #[test]
    fn k2_stage6_perf_alerts() {
        let _ = stage6_perf_alerts();
    }

    // 16. PerfSample Display
    #[test]
    fn k2_perf_sample_display() {
        let s = PerfSample::from_us(PerfKind::Bridge, 200, true);
        let out = format!("{s}");
        assert!(out.contains("Bridge"));
        assert!(out.contains("200μs"));
        assert!(out.contains("✅") || out.contains("⚠️"));
    }

    // 17. PerfKind Display
    #[test]
    fn k2_perf_kind_display() {
        assert_eq!(format!("{}", PerfKind::Bridge), "Bridge");
        assert_eq!(format!("{}", PerfKind::Call), "Call");
    }

    // 18. PerfStats over_threshold_rate
    #[test]
    fn k2_perf_stats_over_threshold_rate() {
        let samples: Vec<PerfSample> = (1..=10)
            .map(|i| PerfSample::from_us(PerfKind::Convert, i * 50, true))
            .collect();
        let s = PerfStats::from_samples(&samples);
        // Convert 阈值 100, 1*50=50, 2*50=100, 3*50=150, ...
        // 超阈 = i*50 > 100 即 i >= 3, 共 8 个
        assert_eq!(s.over_threshold_count, 8);
        assert!((s.over_threshold_rate() - 0.8).abs() < 0.01);
    }

    // 19. PerfMonitor summary 含 5 kind
    #[test]
    fn k2_perf_monitor_summary() {
        let mut m = PerfMonitor::default();
        m.record(PerfSample::from_us(PerfKind::Bridge, 100, true));
        let s = m.summary();
        assert!(s.contains("K2 PerfMonitor"));
        assert!(s.contains("recorded=1"));
        assert!(s.contains("Bridge"));
    }

    // 20. PerfMonitor stats 1 kind
    #[test]
    fn k2_perf_monitor_stats_single_kind() {
        let mut m = PerfMonitor::default();
        for i in 1..=10 {
            m.record(PerfSample::from_us(PerfKind::Eval, i * 100, true));
        }
        let s = m.stats(PerfKind::Eval);
        assert_eq!(s.count, 10);
        assert_eq!(s.max_us, 1000);
        assert_eq!(s.min_us, 100);
    }
}
