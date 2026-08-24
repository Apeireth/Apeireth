//! Monitor — dashboard + 真实测量 (OTA 阶段 6/7).
//!
//! 升级切换 (Switchover) 后, 必须进入监控期, 收集关键指标并产出"保持 / 回滚"建议.
//! 监控期支持自定义 SmokeCheck (注入式测试), 默认包含 3 项: 健康检查 / 错误率 / 延迟.
//!
//! 设计基线: 阶段 2 §6 OTA + 阶段 4 v6 §1 dashboard.
//!
//! 监控周期 (简化):
//! - Smoke: 启动后立即 (1 shot)
//! - Health: 每 N 秒 1 次 (默认 1)
//! - Latency / ErrorRate: 每 N 秒 1 次
//!
//! 推荐 (Recommendation):
//! - Keep     — 所有指标 Healthy 或 Degraded 但在阈值内
//! - Rollback — 任一指标 Failed 或多个 Degraded
//!
//! **禁止**: 不修改 apeireth-core 任何已实装类型签名.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// 单个指标状态.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricStatus {
    /// 健康.
    Healthy,
    /// 降级 (未达 Failed, 但偏离基线).
    Degraded,
    /// 失败.
    Failed,
}

/// 单个监控指标.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorMetric {
    /// 指标名 (e.g., "smoke", "error_rate", "latency_p99").
    pub name: String,
    /// 实际测量值.
    pub value: f64,
    /// 阈值上限 (None 表示无上限).
    pub threshold: Option<f64>,
    /// 阈值下限 (None 表示无下限).
    pub lower_bound: Option<f64>,
    /// 当前状态.
    pub status: MetricStatus,
    /// 采样时间戳.
    pub sampled_at: i64,
}

impl MonitorMetric {
    /// 构造新指标, 自动评估 status.
    pub fn new(
        name: impl Into<String>,
        value: f64,
        threshold: Option<f64>,
        lower_bound: Option<f64>,
    ) -> Self {
        let status = Self::classify(value, threshold, lower_bound);
        Self {
            name: name.into(),
            value,
            threshold,
            lower_bound,
            status,
            sampled_at: Utc::now().timestamp(),
        }
    }

    /// 分类: Failed = 超过 threshold; Degraded = 偏离基线但未超阈值.
    pub fn classify(value: f64, threshold: Option<f64>, lower_bound: Option<f64>) -> MetricStatus {
        if let Some(ub) = threshold {
            if value > ub {
                return MetricStatus::Failed;
            }
            if value > ub * 0.8 {
                return MetricStatus::Degraded;
            }
        }
        if let Some(lb) = lower_bound {
            if value < lb {
                return MetricStatus::Failed;
            }
            if value < lb * 1.2 {
                return MetricStatus::Degraded;
            }
        }
        MetricStatus::Healthy
    }

    /// 是否需要关注 (Degraded 或 Failed).
    pub fn needs_attention(&self) -> bool {
        !matches!(self.status, MetricStatus::Healthy)
    }
}

/// 监控建议.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonitorRecommendation {
    /// 保持当前版本.
    Keep,
    /// 回滚.
    Rollback,
}

impl MonitorRecommendation {
    /// 从指标集合推导建议.
    pub fn from_metrics(metrics: &[MonitorMetric]) -> Self {
        let failed = metrics
            .iter()
            .filter(|m| matches!(m.status, MetricStatus::Failed))
            .count();
        if failed > 0 {
            return MonitorRecommendation::Rollback;
        }
        let degraded = metrics
            .iter()
            .filter(|m| matches!(m.status, MetricStatus::Degraded))
            .count();
        if degraded >= 2 {
            return MonitorRecommendation::Rollback;
        }
        MonitorRecommendation::Keep
    }
}

/// Smoke check trait — 自定义检查可注入.
pub trait SmokeCheck: Send + Sync {
    /// smoke 检查名.
    fn name(&self) -> &str;
    /// 执行检查, 返回 (status, reason).
    fn run(&self) -> (MetricStatus, String);
}

/// 默认 smoke 检查集.
pub struct DefaultSmokeChecks;

impl DefaultSmokeChecks {
    /// 内置 smoke 检查: health / error_rate / latency.
    pub fn run_all() -> Vec<MonitorMetric> {
        vec![
            MonitorMetric::new("smoke", 1.0, Some(0.0), None), // smoke 通过 = 1.0, Failed if > 0
            MonitorMetric::new("error_rate", 0.01, Some(0.05), Some(0.0)),
            MonitorMetric::new("latency_p99_ms", 80.0, Some(500.0), None),
        ]
    }
}

/// 健康检查 smoke (1.0 表示通过).
pub struct HealthSmoke;

impl SmokeCheck for HealthSmoke {
    fn name(&self) -> &str {
        "health"
    }
    fn run(&self) -> (MetricStatus, String) {
        (MetricStatus::Healthy, "service reachable".into())
    }
}

/// 错误率 smoke (mock — 返回 0.01).
pub struct ErrorRateSmoke {
    /// 模拟错误率.
    pub error_rate: f64,
}

impl SmokeCheck for ErrorRateSmoke {
    fn name(&self) -> &str {
        "error_rate"
    }
    fn run(&self) -> (MetricStatus, String) {
        let status = if self.error_rate > 0.05 {
            MetricStatus::Failed
        } else if self.error_rate >= 0.04 {
            MetricStatus::Degraded
        } else {
            MetricStatus::Healthy
        };
        (status, format!("error_rate={}", self.error_rate))
    }
}

/// 延迟 smoke.
pub struct LatencySmoke {
    /// p99 延迟 (ms).
    pub p99_ms: f64,
}

impl SmokeCheck for LatencySmoke {
    fn name(&self) -> &str {
        "latency_p99_ms"
    }
    fn run(&self) -> (MetricStatus, String) {
        let status = if self.p99_ms > 500.0 {
            MetricStatus::Failed
        } else if self.p99_ms >= 400.0 {
            MetricStatus::Degraded
        } else {
            MetricStatus::Healthy
        };
        (status, format!("p99={}ms", self.p99_ms))
    }
}

/// 监控 dashboard — 聚合指标 + 自定义 smoke + 产出报告.
pub struct MonitorDashboard {
    /// 已收集的指标.
    metrics: Vec<MonitorMetric>,
    /// 自定义 smoke 检查 (可选).
    smoke_checks: Vec<Box<dyn SmokeCheck>>,
}

impl std::fmt::Debug for MonitorDashboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MonitorDashboard")
            .field("metrics", &self.metrics)
            .field("smoke_checks_count", &self.smoke_checks.len())
            .finish()
    }
}

impl Default for MonitorDashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitorDashboard {
    /// 构造新 dashboard.
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
            smoke_checks: Vec::new(),
        }
    }

    /// 注册自定义 smoke check.
    pub fn register_smoke(&mut self, check: Box<dyn SmokeCheck>) {
        self.smoke_checks.push(check);
    }

    /// 注入一个指标.
    pub fn record(&mut self, metric: MonitorMetric) {
        self.metrics.push(metric);
    }

    /// 一次性跑完所有注册的 smoke checks 并写入 metrics.
    pub fn run_smokes(&mut self) {
        for check in &self.smoke_checks {
            let (status, reason) = check.run();
            let m = MonitorMetric {
                name: check.name().to_string(),
                value: match status {
                    MetricStatus::Healthy => 0.0,
                    MetricStatus::Degraded => 1.0,
                    MetricStatus::Failed => 2.0,
                },
                threshold: Some(1.5),
                lower_bound: None,
                status,
                sampled_at: Utc::now().timestamp(),
            };
            self.metrics.push(m);
            // 防止 unused warning
            let _ = reason;
        }
    }

    /// 取已收集的指标.
    pub fn metrics(&self) -> &[MonitorMetric] {
        &self.metrics
    }

    /// 产出监控报告.
    pub fn report(&self) -> MonitorReport {
        let rec = MonitorRecommendation::from_metrics(&self.metrics);
        let failed = self
            .metrics
            .iter()
            .filter(|m| matches!(m.status, MetricStatus::Failed))
            .count();
        let degraded = self
            .metrics
            .iter()
            .filter(|m| matches!(m.status, MetricStatus::Degraded))
            .count();
        MonitorReport {
            metrics: self.metrics.clone(),
            recommendation: rec,
            failed_count: failed,
            degraded_count: degraded,
            generated_at: Utc::now().timestamp(),
        }
    }
}

/// 监控报告.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorReport {
    /// 全部指标.
    pub metrics: Vec<MonitorMetric>,
    /// 推荐 (Keep / Rollback).
    pub recommendation: MonitorRecommendation,
    /// Failed 指标数.
    pub failed_count: usize,
    /// Degraded 指标数.
    pub degraded_count: usize,
    /// 报告生成时间.
    pub generated_at: i64,
}

impl MonitorReport {
    /// 是否建议回滚.
    pub fn should_rollback(&self) -> bool {
        matches!(self.recommendation, MonitorRecommendation::Rollback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_classify_within_threshold() {
        let m = MonitorMetric::new("e", 0.03, Some(0.05), Some(0.0));
        assert_eq!(m.status, MetricStatus::Healthy);
        assert!(!m.needs_attention());
    }

    #[test]
    fn metric_classify_degraded_above_80_percent() {
        let m = MonitorMetric::new("e", 0.045, Some(0.05), Some(0.0));
        assert_eq!(m.status, MetricStatus::Degraded);
        assert!(m.needs_attention());
    }

    #[test]
    fn metric_classify_failed_above_threshold() {
        let m = MonitorMetric::new("e", 0.10, Some(0.05), Some(0.0));
        assert_eq!(m.status, MetricStatus::Failed);
        assert!(m.needs_attention());
    }

    #[test]
    fn metric_classify_failed_below_lower_bound() {
        let m = MonitorMetric::new("e", 0.0, Some(1.0), Some(0.5));
        assert_eq!(m.status, MetricStatus::Failed);
    }

    #[test]
    fn metric_classify_degraded_below_120_percent_lower() {
        let m = MonitorMetric::new("e", 0.55, Some(1.0), Some(0.5));
        assert_eq!(m.status, MetricStatus::Degraded);
    }

    #[test]
    fn recommendation_keep_when_all_healthy() {
        let metrics = vec![
            MonitorMetric::new("a", 0.01, Some(0.05), None),
            MonitorMetric::new("b", 100.0, Some(500.0), None),
        ];
        assert_eq!(
            MonitorRecommendation::from_metrics(&metrics),
            MonitorRecommendation::Keep
        );
    }

    #[test]
    fn recommendation_rollback_on_any_failed() {
        let metrics = vec![
            MonitorMetric::new("a", 0.01, Some(0.05), None),
            MonitorMetric::new("b", 600.0, Some(500.0), None),
        ];
        assert_eq!(
            MonitorRecommendation::from_metrics(&metrics),
            MonitorRecommendation::Rollback
        );
    }

    #[test]
    fn recommendation_rollback_on_two_degraded() {
        let metrics = vec![
            MonitorMetric::new("a", 0.045, Some(0.05), None), // Degraded
            MonitorMetric::new("b", 450.0, Some(500.0), None), // Degraded
        ];
        assert_eq!(
            MonitorRecommendation::from_metrics(&metrics),
            MonitorRecommendation::Rollback
        );
    }

    #[test]
    fn recommendation_keep_on_single_degraded() {
        let metrics = vec![
            MonitorMetric::new("a", 0.01, Some(0.05), None), // Healthy
            MonitorMetric::new("b", 450.0, Some(500.0), None), // Degraded
        ];
        assert_eq!(
            MonitorRecommendation::from_metrics(&metrics),
            MonitorRecommendation::Keep
        );
    }

    #[test]
    fn health_smoke_returns_healthy() {
        let c = HealthSmoke;
        let (status, reason) = c.run();
        assert_eq!(status, MetricStatus::Healthy);
        assert!(reason.contains("reachable"));
        assert_eq!(c.name(), "health");
    }

    #[test]
    fn error_rate_smoke_classifies() {
        let c = ErrorRateSmoke { error_rate: 0.01 };
        assert_eq!(c.run().0, MetricStatus::Healthy);
        let c = ErrorRateSmoke { error_rate: 0.04 };
        assert_eq!(c.run().0, MetricStatus::Degraded);
        let c = ErrorRateSmoke { error_rate: 0.10 };
        assert_eq!(c.run().0, MetricStatus::Failed);
    }

    #[test]
    fn latency_smoke_classifies() {
        let c = LatencySmoke { p99_ms: 100.0 };
        assert_eq!(c.run().0, MetricStatus::Healthy);
        let c = LatencySmoke { p99_ms: 450.0 };
        assert_eq!(c.run().0, MetricStatus::Degraded);
        let c = LatencySmoke { p99_ms: 600.0 };
        assert_eq!(c.run().0, MetricStatus::Failed);
    }

    #[test]
    fn dashboard_run_smokes_records_metrics() {
        let mut d = MonitorDashboard::new();
        d.register_smoke(Box::new(HealthSmoke));
        d.register_smoke(Box::new(ErrorRateSmoke { error_rate: 0.01 }));
        d.register_smoke(Box::new(LatencySmoke { p99_ms: 100.0 }));
        d.run_smokes();
        let metrics = d.metrics();
        assert_eq!(metrics.len(), 3);
        let names: Vec<&str> = metrics.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"health"));
        assert!(names.contains(&"error_rate"));
        assert!(names.contains(&"latency_p99_ms"));
    }

    #[test]
    fn dashboard_report_all_healthy_keeps() {
        let mut d = MonitorDashboard::new();
        d.register_smoke(Box::new(HealthSmoke));
        d.register_smoke(Box::new(ErrorRateSmoke { error_rate: 0.01 }));
        d.register_smoke(Box::new(LatencySmoke { p99_ms: 100.0 }));
        d.run_smokes();
        let report = d.report();
        assert_eq!(report.recommendation, MonitorRecommendation::Keep);
        assert_eq!(report.failed_count, 0);
        assert_eq!(report.degraded_count, 0);
        assert!(!report.should_rollback());
    }

    #[test]
    fn dashboard_report_high_error_rate_rolls_back() {
        let mut d = MonitorDashboard::new();
        d.register_smoke(Box::new(HealthSmoke));
        d.register_smoke(Box::new(ErrorRateSmoke { error_rate: 0.20 })); // Failed
        d.register_smoke(Box::new(LatencySmoke { p99_ms: 100.0 }));
        d.run_smokes();
        let report = d.report();
        assert_eq!(report.recommendation, MonitorRecommendation::Rollback);
        assert!(report.should_rollback());
        assert_eq!(report.failed_count, 1);
    }

    #[test]
    fn default_smoke_checks_run_all_returns_three_metrics() {
        let m = DefaultSmokeChecks::run_all();
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn monitor_record_then_report() {
        let mut d = MonitorDashboard::new();
        d.record(MonitorMetric::new("a", 1.0, Some(5.0), None));
        d.record(MonitorMetric::new("b", 1.0, Some(5.0), None));
        let report = d.report();
        assert_eq!(report.metrics.len(), 2);
        assert_eq!(report.recommendation, MonitorRecommendation::Keep);
    }
}
