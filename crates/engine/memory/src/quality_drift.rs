//! 在线质量漂移监控：**EMA 基线 + 2σ 连击告警**（记忆线的「不许悄悄劣化」）。
//!
//! ## 它与 `golden_baseline.rs` 的分工（**互补，不重复**）
//! | | 参照物 | 抓什么 | 在哪跑 |
//! |---|---|---|---|
//! | [`crate::golden_baseline`]（**静态基线**） | **一次冻结**的数值 | 「相对那次掉了没」 | CI 闸门（`check_golden_baseline.py`） |
//! | 本模块（**在线漂移**） | **自己最近的常态**（EMA） | 「**慢慢**劣化没」（冻结阈值看不出） | 运行时 / 每次评测 |
//!
//! ⇒ ⇒ 静态闸门是「不许掉到某条线以下」，本模块是「**别在慢慢下滑**」—— 两者都需要。
//!
//! ## 它接线了什么
//! [`AdaptiveBaseline`] 与 [`DriftDetector`] 这两个类型此前**库外零消费**
//! （台账 `unwired-ledger.md` §5.5q 记为 A 类缺口）⇒ ⇒ 本模块给它们一个**真实的消费者**：
//! 记忆线自己的**检索质量指标**（`recall@k` / `ndcg@k`，来自 [`crate::golden_baseline`] 的语料基线）。
//!
//! ## ⚠️ 判定顺序（**正确性要害**，别改反）
//! [`AdaptiveBaseline::observe`] 会**先把新样本并进 EMA 均值**；若「先更新、再判漂移」，
//! 则 z 分母是**含本次异常值**的均值 ⇒ ⇒ **信号被自己抹平**（大偏差把均值拉向自己 ⇒ z 变小）。
//! ⇒ ⇒ **必须先判（[`DriftDetector::observe`] 对**旧**基线算 z）、后更新（[`AdaptiveBaseline::observe`]）**。
//! 本模块的 `record` 固定这个顺序，并有测试钉住（见 `record_judges_against_the_pre_update_baseline`）。
//!
//! ## 工程边界
//! - **时间由调用方给**（`at_sec`，**epoch 秒**，与 `Episode.timestamp` 一致）⇒ **不读系统时钟**；
//! - **不做模型调用**、**不碰 IO**（历史只在内存里）⇒ ⇒ 同一输入必得同一输出；
//! - **非有限值（NaN/Inf）被忽略**（沿用 `AdaptiveBaseline` 的既有口径：缺失值按当前均值处理）。
//!
//! ## ⚠️ 为什么**没有** `baseline_std` 访问器
//! 孤儿 API 棘轮（`scripts/unwired-baseline.txt`）**只减不增** ⇒ ⇒ **新增公开符号**就是新增债
//! ⇒ ⇒ 本模块**不提供** `baseline_std`（标准差可由 `z_score` + `baseline_mean` 推出，
//! 且 `AdaptiveBaseline::std` 本身公开）⇒ ⇒ **不新增孤儿**。

use crate::golden_baseline::GoldenBaseline;
use crate::online_calibration::{AdaptiveBaseline, DriftAlarm, DriftDetector};

/// 一次质量采样。
#[derive(Debug, Clone, PartialEq)]
pub struct QualitySample {
    /// 采样时刻（**epoch 秒**）。
    pub at_sec: i64,
    /// 各指标的取值（顺序与指标的声明顺序一致）。
    pub values: Vec<f64>,
}

/// 在线漂移监控：记住「最近表现的样子」，并在**持续**偏离时告警。
#[derive(Debug, Clone)]
pub struct QualityDriftMonitor {
    metrics: Vec<String>,
    baseline: AdaptiveBaseline,
    detector: DriftDetector,
    samples: Vec<QualitySample>,
    max_history: usize,
    alarms: Vec<DriftAlarm>,
}

impl QualityDriftMonitor {
    /// 构造监控器。
    ///
    /// - `metrics`：指标名（**顺序即向量顺序**）；
    /// - `alpha`：EMA 平滑系数（`(0, 1]`；越小 ⇒ 基线越稳、越迟钝）；
    /// - `z_threshold`：告警阈值（**|z|** 超过它算一次偏离；默认 2.0）；
    /// - `window_threshold`：**连续**偏离多少次才告警（默认 3）。
    #[must_use]
    pub fn new(
        metrics: &[impl AsRef<str>],
        alpha: f64,
        z_threshold: f64,
        window_threshold: usize,
    ) -> Self {
        let names: Vec<String> = metrics.iter().map(|m| m.as_ref().to_string()).collect();
        let count = names.len();
        Self {
            metrics: names.clone(),
            baseline: AdaptiveBaseline::new(names, alpha),
            detector: DriftDetector::new(count, z_threshold, window_threshold),
            samples: Vec::new(),
            max_history: 64,
            alarms: Vec::new(),
        }
    }

    /// 用**静态基线**（[`GoldenBaseline`]）做种子：每个语料的 `recall@k` 与 `ndcg@k` 各成一个指标。
    ///
    /// ⇒ ⇒ 于是「第一次上线」就有参照物（否则第一条样本无从判漂移）。
    #[must_use]
    pub fn from_golden_baseline(
        baseline: &GoldenBaseline,
        alpha: f64,
        z_threshold: f64,
        window_threshold: usize,
    ) -> Self {
        let mut metrics: Vec<String> = Vec::new();
        let mut values: Vec<f64> = Vec::new();
        for corpus in &baseline.corpora {
            metrics.push(format!("{}/recall@{}", corpus.name, baseline.k));
            values.push(corpus.metrics.recall_at_k);
            metrics.push(format!("{}/ndcg@{}", corpus.name, baseline.k));
            values.push(corpus.metrics.ndcg_at_k);
        }
        let mut monitor = Self::new(&metrics, alpha, z_threshold, window_threshold);
        // ⚠️ 种子值本身**不算一次采样**（它是「参照物」，不是「一次观测」）
        // ⇒ ⇒ 记为基线的初始状态（`AdaptiveBaseline` 的首帧即初始化）。
        monitor.baseline.observe(&values);
        monitor
    }

    /// 记录一次采样，并返回**本次**触发的告警（可能为空）。
    ///
    /// ⭐ **顺序固定为「先判后更」**（见模块头「判定顺序」）。
    pub fn record(&mut self, at_sec: i64, values: &[f64]) -> &[DriftAlarm] {
        // ① 先判：对**旧**基线算 z（此刻基线还不含本次样本）
        //    ⚠️ **但没有「旧基线」时不判**：`AdaptiveBaseline` 未初始化时 `mean` 取默认 0.5、
        //    `std` 取下限 1e-6 ⇒ ⇒ 首条样本的 |z| 恒为巨大 ⇒ ⇒ **会被误记成一次偏离**
        //    ⇒ ⇒ 首条样本只用来**建立**基线，不参与判定
        //    （`from_golden_baseline` 的种子则不同：它已把参照物喂进基线 ⇒ ⇒ 之后每条都参与判定）。
        let alarms = if self.baseline.seen() > 0 {
            self.detector.observe(values, &self.baseline)
        } else {
            Vec::new()
        };
        // ② 后更：把本次样本并进 EMA
        self.baseline.observe(values);
        self.alarms = alarms.clone();
        self.samples.push(QualitySample {
            at_sec,
            values: values.to_vec(),
        });
        if self.samples.len() > self.max_history {
            let excess = self.samples.len() - self.max_history;
            self.samples.drain(0..excess);
        }
        &self.alarms
    }

    /// 指标名列表（顺序即向量顺序）。
    #[must_use]
    pub fn metrics(&self) -> &[String] {
        &self.metrics
    }

    /// 某指标当前的基线均值（`None` = 指标名不存在）。
    #[must_use]
    pub fn baseline_mean(&self, metric: &str) -> Option<f64> {
        self.baseline
            .index_of(metric)
            .map(|i| self.baseline.mean(i))
    }

    /// 某指标的 z 分（**相对当前基线**；`None` = 指标名不存在）。
    #[must_use]
    pub fn z_score(&self, metric: &str, value: f64) -> Option<f64> {
        self.baseline
            .index_of(metric)
            .map(|i| self.baseline.z(i, value))
    }

    /// 各指标当前的**连续偏离次数**（未达阈值为 0）。
    ///
    /// ⚠️ **为什么叫 `consecutive_deviations` 而不是 `streaks`**：见
    /// `consolidation_trigger` 里同名撞名的同一条理由（避免**假的债减少**）。
    #[must_use]
    pub fn consecutive_deviations(&self) -> &[usize] {
        self.detector.streaks()
    }

    /// 最近一次采样触发的告警。
    #[must_use]
    pub fn alarms(&self) -> &[DriftAlarm] {
        &self.alarms
    }

    /// 最近的采样历史（最多 `max_history` 条）。
    #[must_use]
    pub fn samples(&self) -> &[QualitySample] {
        &self.samples
    }

    /// 已见过的采样次数（`AdaptiveBaseline` 的计数）。
    #[must_use]
    pub fn observed(&self) -> usize {
        self.baseline.seen()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::golden_baseline::CorpusBaseline;

    /// ⚠️ **`alpha` 取 0.05（基线很稳）**：α 越大 ⇒ 基线越快吸收偏离 ⇒ ⇒ z 越小
    /// ⇒ ⇒ 「连续 3 次偏离才告警」这件事**依赖足够小的 α**（否则第二次偏离就被拉回带内）。
    fn monitor() -> QualityDriftMonitor {
        QualityDriftMonitor::new(&["recall@10", "ndcg@10"], 0.05, 2.0, 3)
    }

    #[test]
    fn the_first_sample_is_not_judged() {
        // ⚠️ **实测抓到的真 bug**：首条样本没有参照物，若参与判定 ⇒
        //   `mean` 取默认 0.5、`std` 取下限 1e-6 ⇒ |z| 巨大 ⇒ **被误记成一次偏离**。
        let mut m = monitor();
        let alarms = m.record(1_000, &[0.90, 0.80]);
        assert!(alarms.is_empty(), "首条不该告警：{alarms:?}");
        assert_eq!(m.consecutive_deviations(), &[0, 0], "首条不该记偏离");
        assert_eq!(m.observed(), 1, "但它确实建立了基线");
    }

    #[test]
    fn a_normal_sample_does_not_alarm() {
        let mut m = monitor();
        for at in [1_000, 2_000, 3_000] {
            let alarms = m.record(at, &[0.90, 0.80]);
            assert!(alarms.is_empty(), "正常波动不该告警：{alarms:?}");
        }
        assert_eq!(m.consecutive_deviations(), &[0, 0]);
    }

    #[test]
    fn three_consecutive_deviations_alarm() {
        let mut m = monitor();
        for at in [1_000, 2_000, 3_000] {
            m.record(at, &[0.90, 0.80]);
        }
        // 连续大幅偏低 ⇒ 第 1、2 次不告警（未达连击阈值），第 3 次告警
        assert!(m.record(4_000, &[0.50, 0.80]).is_empty());
        assert!(m.record(5_000, &[0.50, 0.80]).is_empty());
        let alarms = m.record(6_000, &[0.50, 0.80]);
        assert_eq!(alarms.len(), 1, "只该有一个指标告警：{alarms:?}");
        assert_eq!(alarms[0].name, "recall@10");
        assert!(alarms[0].streak >= 3);
    }

    #[test]
    fn a_single_deviation_does_not_alarm() {
        let mut m = monitor();
        for at in [1_000, 2_000, 3_000] {
            m.record(at, &[0.90, 0.80]);
        }
        let alarms = m.record(4_000, &[0.50, 0.80]);
        assert!(alarms.is_empty(), "单次偏离不该告警：{alarms:?}");
        assert_eq!(m.consecutive_deviations()[0], 1);
    }

    #[test]
    fn streak_resets_when_the_value_returns_to_band() {
        let mut m = monitor();
        for at in [1_000, 2_000, 3_000] {
            m.record(at, &[0.90, 0.80]);
        }
        m.record(4_000, &[0.50, 0.80]);
        assert_eq!(m.consecutive_deviations()[0], 1);
        m.record(5_000, &[0.90, 0.80]);
        assert_eq!(m.consecutive_deviations()[0], 0, "回到带内 ⇒ 连续计数清零");
    }

    #[test]
    fn record_judges_against_the_pre_update_baseline() {
        // ⚠️ **顺序保证**（模块头「判定顺序」）：判定必须用**更新前**的均值。
        // 若实现反了（先更新再判），第一次偏离的 |z| 会被「均值已被拉低」抹平。
        let mut m = monitor();
        m.record(1_000, &[0.90, 0.80]);
        let mean_before = m.baseline_mean("recall@10").expect("mean");
        assert!((mean_before - 0.90).abs() < 1e-9);
        m.record(2_000, &[0.50, 0.80]);
        // 若判定用的是**更新后**的均值，z 会被显著压低；断言「偏离仍被记为 1 次」
        assert_eq!(
            m.consecutive_deviations()[0],
            1,
            "第一次偏离必须被记上（说明判定用的是旧基线）"
        );
        // 且均值**随后**才被本次样本拉低（EMA）
        let mean_after = m.baseline_mean("recall@10").expect("mean");
        assert!(mean_after < mean_before, "本次样本应并入 EMA");
    }

    #[test]
    fn non_finite_values_are_ignored() {
        let mut m = monitor();
        let alarms = m.record(1_000, &[f64::NAN, 0.80]);
        assert!(alarms.is_empty());
        assert!(m.baseline_mean("recall@10").expect("mean").is_finite());
    }

    #[test]
    fn golden_baseline_seeds_the_monitor_with_one_metric_per_corpus_and_measure() {
        let baseline = GoldenBaseline::new(
            "internal-test",
            10,
            vec![
                CorpusBaseline::new("redchamber", 0.2983, 0.1615, 476),
                CorpusBaseline::new("watershed", 0.6875, 0.4817, 248),
            ],
        );
        let mut m = QualityDriftMonitor::from_golden_baseline(&baseline, 0.05, 2.0, 3);
        assert_eq!(m.metrics().len(), 4);
        assert!(m
            .metrics()
            .iter()
            .any(|name| name == "redchamber/recall@10"));
        assert!(m.metrics().iter().any(|name| name == "watershed/ndcg@10"));
        let mean = m.baseline_mean("redchamber/recall@10").expect("mean");
        assert!((mean - 0.2983).abs() < 1e-9, "种子值应进入基线：{mean}");
        // 种子已建立基线 ⇒ ⇒ 之后每条采样都参与判定
        assert!(m
            .record(1_000, &[0.2983, 0.1615, 0.6875, 0.4817])
            .is_empty());
    }

    #[test]
    fn history_is_bounded() {
        let mut m = monitor();
        for index in 0_i64..200 {
            m.record(index, &[0.9, 0.8]);
        }
        assert!(m.samples().len() <= 64, "历史不该无限增长");
    }

    #[test]
    fn unknown_metric_has_no_mean() {
        let m = monitor();
        assert!(m.baseline_mean("nope").is_none());
        assert!(m.z_score("nope", 0.5).is_none());
    }
}
