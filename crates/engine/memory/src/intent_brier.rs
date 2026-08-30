//! `apeireth-memory::intent_brier` — 意图理解准确率 Brier 自我诊断系统 (W6 / 元认知自校准).
//!
//! ## 哲学 (S-2 实事求是 + 价值内化从玄学变有数字)
//!
//! 认知模型不能只输出预测，而需要对自己“是否真正猜对用户意图”进行事后统计量化打分：
//! 每轮对话后，模型对“用户真实意图”的预测概率 $p$ vs 事后真实意图反馈 (Agree/Correct/Silent)
//! $\to$ Brier score 计算公式: $(p - 1)^2$ (命中) 或 $p^2$ (未命中).
//!
//! 0.0 表示完美预测，1.0 表示完全猜反，0.25 为业内未校准中位基线。
//!
//! ## 滚动窗口与领域诊断
//! - 默认多档滚动窗口 [30, 100, 300] 轮跟踪短期、中期与长期校准斜率趋势；
//! - 按 `domain` 聚合均值，识别出“在哪些具体话题或场景下容易猜偏”；
//! - 纯 Safe Rust 确定性数学计算，0 外部不可信 C-FFI 依赖，0 LLM 阻塞依赖。

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

// ============================================================
// 核心数据结构
// ============================================================

/// 模型对用户意图的预测与置信度.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentPrediction {
    /// 预测话题/意图类型 (例如 "code_refactor", "companion", "troubleshooting").
    pub topic: String,
    /// 模型自信度 (0.0..=1.0). f64 与 Brier 公式对齐以避免精度退化.
    pub confidence: f64,
}

impl IntentPrediction {
    pub fn new(topic: impl Into<String>, confidence: f64) -> Self {
        Self {
            topic: topic.into(),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

/// 用户反馈信号 (同意 / 纠正 / 沉默).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackOutcome {
    /// 用户明确同意/采纳 $\to$ 命中 (hit = true).
    Agree,
    /// 用户纠正/拒绝 $\to$ 未命中 (hit = false).
    Correct,
    /// 用户沉默/继续后续话题 $\to$ 保守按命中计 (hit = true).
    Silent,
}

impl FeedbackOutcome {
    /// 是否算“命中” (用于 Brier 得分计算).
    pub fn is_hit(self) -> bool {
        matches!(self, FeedbackOutcome::Agree | FeedbackOutcome::Silent)
    }
}

/// 一条意图对账记录.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentRecord {
    pub prediction: IntentPrediction,
    /// 用户真实意图/修正话题 (None = 尚未反馈).
    pub true_topic: Option<String>,
    /// 反馈结果 (None = 尚未反馈).
    pub outcome: Option<FeedbackOutcome>,
    /// 毫秒时间戳 (单机唯一递增 ID).
    pub timestamp_ms: i64,
    /// 话题领域 (诊断聚合分类用, 例如 "study", "coding", "companion").
    pub domain: Option<String>,
}

impl IntentRecord {
    pub fn new(prediction: IntentPrediction, timestamp_ms: i64) -> Self {
        Self {
            prediction,
            true_topic: None,
            outcome: None,
            timestamp_ms,
            domain: None,
        }
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// 链式追加反馈.
    pub fn feedback(mut self, outcome: FeedbackOutcome, true_topic: Option<String>) -> Self {
        self.outcome = Some(outcome);
        self.true_topic = true_topic;
        self
    }

    /// 原地设置反馈.
    pub fn apply_feedback(&mut self, outcome: FeedbackOutcome, true_topic: Option<String>) {
        self.outcome = Some(outcome);
        self.true_topic = true_topic;
    }

    /// 命中与否 (仅当有明确 outcome 时有效).
    pub fn hit(&self) -> Option<bool> {
        self.outcome.map(|o| o.is_hit())
    }

    /// Brier 单条得分 (无 outcome 时返回 None).
    pub fn brier(&self) -> Option<f64> {
        self.outcome
            .map(|o| brier_score(self.prediction.confidence, o.is_hit()))
    }
}

// ============================================================
// IntentLedger — 滚动记录簿
// ============================================================

/// 滑动记录簿 (按插入时间顺序管理，带最大容量上限防超支).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentLedger {
    records: VecDeque<IntentRecord>,
    max_capacity: usize,
}

impl Default for IntentLedger {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl IntentLedger {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            records: VecDeque::with_capacity(max_capacity.max(1)),
            max_capacity: max_capacity.max(1),
        }
    }

    /// 记录一次意图预测.
    pub fn record(&mut self, r: IntentRecord) {
        if self.records.len() >= self.max_capacity {
            self.records.pop_front();
        }
        self.records.push_back(r);
    }

    /// 按 timestamp_ms 查找记录并追加反馈 (已反馈则拒绝覆盖).
    pub fn feedback(
        &mut self,
        timestamp_ms: i64,
        outcome: FeedbackOutcome,
        true_topic: Option<String>,
    ) -> Result<(), String> {
        let pos = self
            .records
            .iter()
            .position(|r| r.timestamp_ms == timestamp_ms)
            .ok_or_else(|| format!("未找到意图记录: {timestamp_ms}"))?;
        let r = &mut self.records[pos];
        if r.outcome.is_some() {
            return Err("该记录已完成反馈，不可重复反馈".into());
        }
        r.apply_feedback(outcome, true_topic);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// 全部记录 (按插入顺序).
    pub fn records(&self) -> Vec<IntentRecord> {
        self.records.iter().cloned().collect()
    }

    /// 仅返回已完成反馈的记录.
    pub fn resolved_records(&self) -> Vec<IntentRecord> {
        self.records
            .iter()
            .filter(|r| r.outcome.is_some())
            .cloned()
            .collect()
    }
}

// ============================================================
// Brier 纯函数计算
// ============================================================

/// Brier 单条得分: $(p-1)^2$ if hit else $p^2$.
/// 范围 $\in [0, 1]$; 0 = 完美校准, 1 = 完全猜反.
pub fn brier_score(predicted_confidence: f64, hit: bool) -> f64 {
    let p = predicted_confidence.clamp(0.0, 1.0);
    if hit {
        (p - 1.0).powi(2)
    } else {
        p.powi(2)
    }
}

/// Brier 均值计算 (无样本时返回 0.0).
pub fn mean_brier(records: &[IntentRecord]) -> f64 {
    let resolved: Vec<f64> = records.iter().filter_map(|r| r.brier()).collect();
    if resolved.is_empty() {
        0.0
    } else {
        resolved.iter().sum::<f64>() / resolved.len() as f64
    }
}

// ============================================================
// 滚动窗口与趋势分析
// ============================================================

/// 单个滚动窗口的统计.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrierWindow {
    pub window_size: usize,
    pub mean_brier: f64,
    pub sample_count: usize,
}

impl BrierWindow {
    pub fn empty(window_size: usize) -> Self {
        Self {
            window_size,
            mean_brier: 0.0,
            sample_count: 0,
        }
    }
}

/// 校准趋势 (短期窗口 vs 长期窗口).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrierTrend {
    /// 短期 Brier < 长期 $\to$ 校准改善中.
    Improving,
    /// 短期 $\approx$ 长期 (差异 $< 5\%$).
    Stable,
    /// 短期 Brier > 长期 $\to$ 校准退化.
    Degrading,
}

/// 默认窗口档位 [30, 100, 300] 轮.
pub const DEFAULT_WINDOWS: &[usize] = &[30, 100, 300];

/// 趋势判定阈值 (差异占比; $< 5\%$ 判定为 Stable).
pub const TREND_DELTA_RATIO: f64 = 0.05;

/// 低校准领域阈值 (mean_brier 高于此值则标记为需重点关注). 业内基线为 0.25.
pub const DEFAULT_LOW_CALIBRATION_THRESHOLD: f64 = 0.25;

/// 计算指定窗口大小的 Brier 均值.
pub fn compute_window(records: &[IntentRecord], window_size: usize) -> BrierWindow {
    if window_size == 0 {
        return BrierWindow::empty(0);
    }
    let n = records.len();
    let start = n.saturating_sub(window_size);
    let slice = &records[start..];
    let resolved: Vec<&IntentRecord> = slice.iter().filter(|r| r.outcome.is_some()).collect();
    let sample_count = resolved.len();
    if sample_count == 0 {
        BrierWindow {
            window_size,
            mean_brier: 0.0,
            sample_count: 0,
        }
    } else {
        let sum: f64 = resolved.iter().filter_map(|r| r.brier()).sum();
        BrierWindow {
            window_size,
            mean_brier: sum / sample_count as f64,
            sample_count,
        }
    }
}

/// 趋势判定: 短期窗口 (30) vs 长期窗口 (300).
pub fn compute_trend(records: &[IntentRecord]) -> BrierTrend {
    let short = compute_window(records, 30);
    let long = compute_window(records, 300);
    match (short.sample_count, long.sample_count) {
        (0, _) | (_, 0) => BrierTrend::Stable,
        (_, _) => {
            let delta = (long.mean_brier - short.mean_brier) / long.mean_brier.max(1e-9);
            if delta > TREND_DELTA_RATIO {
                BrierTrend::Improving
            } else if delta < -TREND_DELTA_RATIO {
                BrierTrend::Degrading
            } else {
                BrierTrend::Stable
            }
        }
    }
}

// ============================================================
// 领域诊断 (识别低校准话题领域)
// ============================================================

/// 单个话题领域的诊断.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainDiagnostic {
    pub domain: String,
    pub mean_brier: f64,
    pub sample_count: usize,
    pub is_low_calibration: bool,
}

/// 按 domain 分组计算 mean_brier 并标定低校准领域.
pub fn domain_diagnostics(records: &[IntentRecord], threshold: f64) -> Vec<DomainDiagnostic> {
    let mut groups: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for r in records.iter().filter(|r| r.outcome.is_some()) {
        if let Some(d) = &r.domain {
            if let Some(b) = r.brier() {
                groups.entry(d.clone()).or_default().push(b);
            }
        }
    }
    let mut out: Vec<DomainDiagnostic> = groups
        .into_iter()
        .map(|(domain, scores)| {
            let sample_count = scores.len();
            let mean = scores.iter().sum::<f64>() / sample_count as f64;
            DomainDiagnostic {
                domain,
                mean_brier: mean,
                sample_count,
                is_low_calibration: mean > threshold,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.mean_brier
            .partial_cmp(&a.mean_brier)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

// ============================================================
// 总报告生成与 Markdown 渲染
// ============================================================

/// 完整意图校准诊断报告.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDiagnosticReport {
    /// 三档滚动窗口统计 [30, 100, 300].
    pub windows: Vec<BrierWindow>,
    /// 全部已反馈样本的总体 mean_brier.
    pub overall_mean_brier: f64,
    /// 短期 vs 长期趋势.
    pub trend: BrierTrend,
    /// 各领域诊断 (按 mean_brier 降序排列).
    pub domain_diagnostics: Vec<DomainDiagnostic>,
    /// 低校准领域清单 (mean_brier > threshold).
    pub low_calibration_domains: Vec<String>,
    /// 已反馈总样本数.
    pub sample_count: usize,
}

impl IntentDiagnosticReport {
    pub fn empty() -> Self {
        Self {
            windows: DEFAULT_WINDOWS
                .iter()
                .map(|&w| BrierWindow::empty(w))
                .collect(),
            overall_mean_brier: 0.0,
            trend: BrierTrend::Stable,
            domain_diagnostics: Vec::new(),
            low_calibration_domains: Vec::new(),
            sample_count: 0,
        }
    }
}

/// 主入口: 根据当前 Ledger 计算完整的意图诊断报告.
pub fn compute_report(
    ledger: &IntentLedger,
    low_calibration_threshold: f64,
) -> IntentDiagnosticReport {
    let records = ledger.resolved_records();
    if records.is_empty() {
        return IntentDiagnosticReport::empty();
    }
    let windows: Vec<BrierWindow> = DEFAULT_WINDOWS
        .iter()
        .map(|&w| compute_window(&records, w))
        .collect();
    let overall = mean_brier(&records);
    let trend = compute_trend(&records);
    let domain_diag = domain_diagnostics(&records, low_calibration_threshold);
    let low_calibration_domains: Vec<String> = domain_diag
        .iter()
        .filter(|d| d.is_low_calibration)
        .map(|d| d.domain.clone())
        .collect();
    IntentDiagnosticReport {
        windows,
        overall_mean_brier: overall,
        trend,
        domain_diagnostics: domain_diag,
        low_calibration_domains,
        sample_count: records.len(),
    }
}

/// 渲染诊断报告为可读文本 / Markdown (供注入 System Prompt 或日志对账).
pub fn render_report(report: &IntentDiagnosticReport) -> String {
    let mut s = String::from("[意图理解校准诊断]\n");
    s.push_str(&format!(
        "· 总样本 {} 条, 整体 Brier = {:.3} (0.000=完美, 1.000=全错)\n",
        report.sample_count, report.overall_mean_brier
    ));
    let trend_str = match report.trend {
        BrierTrend::Improving => "改善中 ↑",
        BrierTrend::Stable => "稳定 →",
        BrierTrend::Degrading => "退化 ↓",
    };
    s.push_str(&format!("· 趋势: {trend_str}\n"));
    for w in &report.windows {
        s.push_str(&format!(
            "· 窗口 {}: Brier = {:.3} (样本 {})\n",
            w.window_size, w.mean_brier, w.sample_count
        ));
    }
    if !report.domain_diagnostics.is_empty() {
        s.push_str("· 按领域:\n");
        for d in &report.domain_diagnostics {
            let flag = if d.is_low_calibration {
                " ⚠低校准"
            } else {
                ""
            };
            s.push_str(&format!(
                "  - {}: Brier = {:.3} (样本 {}){}\n",
                d.domain, d.mean_brier, d.sample_count, flag
            ));
        }
    }
    if !report.low_calibration_domains.is_empty() {
        s.push_str(&format!(
            "· 低校准领域需关注: {}\n",
            report.low_calibration_domains.join(", ")
        ));
    }
    s
}

// ============================================================
// 单元测试集 (1:1 继承 1.0 全部 31 测试 + 2.0 增强测试)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(topic: &str, conf: f64, ts: i64) -> IntentRecord {
        IntentRecord::new(IntentPrediction::new(topic, conf), ts)
    }

    fn rec_domain(topic: &str, conf: f64, ts: i64, domain: &str) -> IntentRecord {
        IntentRecord::new(IntentPrediction::new(topic, conf), ts).with_domain(domain)
    }

    // --- brier_score 纯函数测试 ---

    #[test]
    fn brier_perfect_prediction_zero() {
        // 命中 + 高自信 → 接近 0
        assert!((brier_score(0.99, true) - 0.0001).abs() < 1e-4);
    }

    #[test]
    fn brier_terrible_prediction_one() {
        // 未命中 + 高自信 → 接近 1
        assert!((brier_score(0.99, false) - 0.9801).abs() < 1e-4);
    }

    #[test]
    fn brier_unconfident_miss_small() {
        // 未命中 + 低自信 → 接近 0
        assert!((brier_score(0.05, false) - 0.0025).abs() < 1e-4);
    }

    #[test]
    fn brier_fifty_fifty_quarter() {
        // 50% 自信 → 无论命中与否均为 0.25
        assert!((brier_score(0.5, true) - 0.25).abs() < 1e-6);
        assert!((brier_score(0.5, false) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn brier_clamps_confidence() {
        // 边界保护
        assert_eq!(brier_score(1.5, true), 0.0);
        assert_eq!(brier_score(-0.5, false), 0.0);
    }

    // --- IntentRecord 行为测试 ---

    #[test]
    fn record_feedback_agree_hit() {
        let r = rec("exam", 0.9, 100).feedback(FeedbackOutcome::Agree, None);
        assert_eq!(r.hit(), Some(true));
        assert!((r.brier().unwrap() - 0.01).abs() < 1e-4);
    }

    #[test]
    fn record_feedback_correct_miss() {
        let r = rec("exam", 0.9, 100).feedback(FeedbackOutcome::Correct, Some("companion".into()));
        assert_eq!(r.hit(), Some(false));
        assert_eq!(r.true_topic.as_deref(), Some("companion"));
        assert!((r.brier().unwrap() - 0.81).abs() < 1e-4);
    }

    #[test]
    fn record_feedback_silent_hit() {
        let r = rec("exam", 0.8, 100).feedback(FeedbackOutcome::Silent, None);
        assert_eq!(r.hit(), Some(true));
    }

    #[test]
    fn record_unresolved_brier_none() {
        let r = rec("exam", 0.9, 100);
        assert_eq!(r.hit(), None);
        assert_eq!(r.brier(), None);
    }

    // --- mean_brier 测试 ---

    #[test]
    fn mean_brier_empty_records_zero() {
        assert_eq!(mean_brier(&[]), 0.0);
    }

    #[test]
    fn mean_brier_skips_unresolved() {
        let records = vec![
            rec("a", 0.9, 1).feedback(FeedbackOutcome::Agree, None),
            rec("b", 0.9, 2), // 未反馈
        ];
        assert!((mean_brier(&records) - 0.01).abs() < 1e-4);
    }

    #[test]
    fn mean_brier_computes_average() {
        let records = vec![
            rec("a", 0.9, 1).feedback(FeedbackOutcome::Agree, None), // 0.01
            rec("b", 0.9, 2).feedback(FeedbackOutcome::Correct, None), // 0.81
        ];
        assert!((mean_brier(&records) - 0.41).abs() < 1e-4);
    }

    // --- IntentLedger 滚动管理测试 ---

    #[test]
    fn ledger_records_and_caps() {
        let mut ledger = IntentLedger::new(2);
        ledger.record(rec("a", 0.5, 1));
        ledger.record(rec("b", 0.5, 2));
        ledger.record(rec("c", 0.5, 3));
        assert_eq!(ledger.len(), 2);
        assert_eq!(ledger.records()[0].timestamp_ms, 2);
        assert_eq!(ledger.records()[1].timestamp_ms, 3);
    }

    #[test]
    fn ledger_feedback_by_timestamp() {
        let mut ledger = IntentLedger::new(10);
        ledger.record(rec("a", 0.8, 100));
        assert!(ledger.feedback(100, FeedbackOutcome::Agree, None).is_ok());
        assert_eq!(ledger.resolved_records().len(), 1);
        // 重复反馈拒绝
        assert!(ledger
            .feedback(100, FeedbackOutcome::Correct, None)
            .is_err());
        // 未知时间戳
        assert!(ledger.feedback(999, FeedbackOutcome::Agree, None).is_err());
    }

    // --- 滚动窗口测试 ---

    #[test]
    fn compute_window_empty_ledger() {
        let w = compute_window(&[], 30);
        assert_eq!(w.mean_brier, 0.0);
        assert_eq!(w.sample_count, 0);
    }

    #[test]
    fn compute_window_takes_recent_tail() {
        let mut records = Vec::new();
        // 早期 10 条: 糟糕预测 (brier = 0.81)
        for i in 0..10 {
            records.push(rec("bad", 0.9, i).feedback(FeedbackOutcome::Correct, None));
        }
        // 近期 10 条: 完美预测 (brier = 0.01)
        for i in 10..20 {
            records.push(rec("good", 0.9, i).feedback(FeedbackOutcome::Agree, None));
        }
        let w10 = compute_window(&records, 10);
        assert_eq!(w10.sample_count, 10);
        assert!((w10.mean_brier - 0.01).abs() < 1e-4);

        let w20 = compute_window(&records, 20);
        assert_eq!(w20.sample_count, 20);
        assert!((w20.mean_brier - 0.41).abs() < 1e-4);
    }

    // --- 趋势判定测试 ---

    #[test]
    fn compute_trend_improving() {
        let mut records = Vec::new();
        // 早期 300 条: 0.81
        for i in 0..300 {
            records.push(rec("bad", 0.9, i).feedback(FeedbackOutcome::Correct, None));
        }
        // 近期 30 条: 0.01
        for i in 300..330 {
            records.push(rec("good", 0.9, i).feedback(FeedbackOutcome::Agree, None));
        }
        assert_eq!(compute_trend(&records), BrierTrend::Improving);
    }

    #[test]
    fn compute_trend_degrading() {
        let mut records = Vec::new();
        // 早期 300 条: 0.01
        for i in 0..300 {
            records.push(rec("good", 0.9, i).feedback(FeedbackOutcome::Agree, None));
        }
        // 近期 30 条: 0.81
        for i in 300..330 {
            records.push(rec("bad", 0.9, i).feedback(FeedbackOutcome::Correct, None));
        }
        assert_eq!(compute_trend(&records), BrierTrend::Degrading);
    }

    #[test]
    fn compute_trend_stable_when_similar() {
        let mut records = Vec::new();
        for i in 0..330 {
            records.push(rec("a", 0.5, i).feedback(FeedbackOutcome::Agree, None));
        }
        assert_eq!(compute_trend(&records), BrierTrend::Stable);
    }

    #[test]
    fn compute_trend_stable_when_insufficient_samples() {
        let records = vec![rec("a", 0.9, 1).feedback(FeedbackOutcome::Agree, None)];
        assert_eq!(compute_trend(&records), BrierTrend::Stable);
    }

    // --- 领域诊断测试 ---

    #[test]
    fn domain_diagnostics_groups_and_flags_threshold() {
        let records = vec![
            rec_domain("a", 0.9, 1, "study").feedback(FeedbackOutcome::Agree, None), // 0.01
            rec_domain("b", 0.9, 2, "study").feedback(FeedbackOutcome::Agree, None), // 0.01
            rec_domain("c", 0.9, 3, "invest").feedback(FeedbackOutcome::Correct, None), // 0.81
            rec_domain("d", 0.9, 4, "invest").feedback(FeedbackOutcome::Correct, None), // 0.81
        ];
        let diag = domain_diagnostics(&records, 0.25);
        assert_eq!(diag.len(), 2);
        // invest 排第一 (brier 降序)
        assert_eq!(diag[0].domain, "invest");
        assert!(diag[0].is_low_calibration);
        assert!((diag[0].mean_brier - 0.81).abs() < 1e-4);

        assert_eq!(diag[1].domain, "study");
        assert!(!diag[1].is_low_calibration);
        assert!((diag[1].mean_brier - 0.01).abs() < 1e-4);
    }

    #[test]
    fn domain_diagnostics_skips_unlabeled_domain() {
        let records = vec![rec("a", 0.9, 1).feedback(FeedbackOutcome::Agree, None)];
        let diag = domain_diagnostics(&records, 0.25);
        assert!(diag.is_empty());
    }

    // --- 完整报告与 Markdown 渲染测试 ---

    #[test]
    fn compute_report_empty_returns_empty_struct() {
        let ledger = IntentLedger::default();
        let report = compute_report(&ledger, 0.25);
        assert_eq!(report.sample_count, 0);
        assert_eq!(report.overall_mean_brier, 0.0);
        assert_eq!(report.windows.len(), 3);
    }

    #[test]
    fn compute_report_with_data() {
        let mut ledger = IntentLedger::new(100);
        ledger.record(rec_domain("a", 0.9, 1, "code").feedback(FeedbackOutcome::Agree, None));
        ledger.record(rec_domain("b", 0.9, 2, "invest").feedback(FeedbackOutcome::Correct, None));

        let report = compute_report(&ledger, 0.25);
        assert_eq!(report.sample_count, 2);
        assert!((report.overall_mean_brier - 0.41).abs() < 1e-4);
        assert_eq!(report.low_calibration_domains, vec!["invest".to_string()]);
    }

    #[test]
    fn render_report_produces_readable_text() {
        let mut ledger = IntentLedger::new(100);
        ledger.record(rec_domain("a", 0.9, 1, "code").feedback(FeedbackOutcome::Agree, None));
        ledger.record(rec_domain("b", 0.9, 2, "invest").feedback(FeedbackOutcome::Correct, None));

        let report = compute_report(&ledger, 0.25);
        let text = render_report(&report);
        assert!(text.contains("[意图理解校准诊断]"));
        assert!(text.contains("总样本 2 条"));
        assert!(text.contains("invest: Brier = 0.810 (样本 1) ⚠低校准"));
        assert!(text.contains("低校准领域需关注: invest"));
    }
}
