//! **LoCoMo 真实对话记忆基准上的送达率**（⇒ 把「区间」收成「一个点」）。
//!
//! # 这个例子回答什么（**D-13 的方法论缺口**）
//!
//! 此前送达率只有**区间**（4.6× ~ 9.4×），因为：
//! * 语料是**两部古典小说**（长篇叙事，与「对话记忆」不是一回事）；
//! * 正确答案是**我们自己**按「含主体的首句」挖的 ⇒ ⇒ **字面重合红利不可控**。
//!
//! ⇒ ⇒ LoCoMo 的 `evidence` 是**人工标注**的**具体对话轮次**（数据公开、可外部复核）
//! ⇒ ⇒ 本次给出**真实语料上的点估计**。
//!
//! # ⚠️ 一个**必须先讲清的口径问题**（否则会得出 >1 的荒谬送达率）
//!
//! 现行送达率定义是 `groundedness / recall_at_k`（见 `measure_delivery`）。
//! 这个定义在**每题恰好 1 条标准答案**时是干净的（两个量都是 0/1）
//! ⇒ ⇒ 小说语料正是这种形态。
//!
//! 但 LoCoMo **约 27% 的题有多条证据轮** ⇒ ⇒ 此时：
//! * `recall_at_k` = 「命中证据轮 / 证据轮总数」的**平均值**（可以是 0.5）；
//! * `groundedness` = 「**至少一条**证据被送达」的**二值**题比例（可以是 1.0）；
//! ⇒ ⇒ 两者相除 = **1.11**（>1）⇒ ⇒ **不是「超量送达」，是口径混用**。
//!
//! ⇒ ⇒ 所以本例子**分两段**报：
//! * **A. 全量**（约 1,527 题）：只报 `recall@10` 与 `groundedness`（各自定义清楚，
//!   **不**相除 ⇒ 不相除就不会撒谎）；
//! * **B. 单证据子集**（约 1,127 题，**73.2%**）：**两种量都是 0/1** ⇒ 送达率**有定义**、
//!   且与小说语料**同形态** ⇒ ⇒ ⇒ **这一段的送达率才是可比的点估计**。
//!
//! # 其它口径
//!
//! * **一条记忆 = 一个对话轮次**；`k = 10`；`InjectionMode::Production`（与生产逐字一致）；
//! * **对抗类（`category = 5`，答案 `None`）不进考卷** ⇒ 如实单独报**题数**（缺口）；
//! * 每题 `expect_fragments` = 证据轮原文（**≤100 字**，仍是正文子串，有 assert 钉着）；
//! * **逐样本**跑再**逐题加权汇总**（不让大样本吞掉小样本）；任何样本失败 ⇒ **打印出处**。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo run -p apeireth-memory --example locomo_delivery --locked
//! ```

use apeireth_memory::golden_run::{run, InjectionMode};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet, Metrics};
use apeireth_memory::locomo_corpus::{self, LocomoSample};
use std::process::ExitCode;

/// 检索深度（与其它实验**同一口径** ⇒ 可比）。
const K: usize = 10;

/// 一段汇总（**逐题加权**）。
#[derive(Default)]
struct Aggregate {
    cases: usize,
    sum_recall: f64,
    sum_grounded: f64,
    sum_delivery: f64,
    delivery_cases: usize,
}

impl Aggregate {
    fn recall(&self) -> f64 {
        if self.cases == 0 {
            f64::NAN
        } else {
            self.sum_recall / self.cases as f64
        }
    }

    fn grounded(&self) -> f64 {
        if self.cases == 0 {
            f64::NAN
        } else {
            self.sum_grounded / self.cases as f64
        }
    }

    fn delivery(&self) -> f64 {
        if self.delivery_cases == 0 {
            f64::NAN
        } else {
            self.sum_delivery / self.delivery_cases as f64
        }
    }
}

fn main() -> ExitCode {
    let Some(path) = locomo_corpus::default_path() else {
        eprintln!(
            "· 找不到 locomo10.json（$APEIRETH_DATA_DIR/real-corpus/ 或 ~/.apeireth/real-corpus/）"
        );
        eprintln!("  ⇒ ⇒ **没测到**（不是「测了结果是 0」）");
        return ExitCode::FAILURE;
    };
    println!("════════ LoCoMo 真实对话记忆基准 ════════");
    println!("数据：{}", path.display());
    let samples = match locomo_corpus::load(&path) {
        Ok(samples) => samples,
        Err(error) => {
            eprintln!("· 加载失败：{error}");
            return ExitCode::FAILURE;
        }
    };
    let turns: usize = samples.iter().map(|sample| sample.turn_count()).sum();
    let cases: usize = samples.iter().map(|sample| sample.cases.len()).sum();
    let adversarial: usize = samples.iter().map(|sample| sample.adversarial_cases).sum();
    let skipped: usize = samples.iter().map(|sample| sample.skipped_cases).sum();
    let single: usize = samples.iter().map(single_evidence_cases).sum();
    println!(
        "{} 个长期对话 · {} 轮真实对话 ⇒ {} 条记忆 · 考卷 {} 题（k={}）",
        samples.len(),
        turns,
        turns,
        cases,
        K
    );
    println!(
        "⚠️ 未进考卷：对抗类（答案=None）**{}** 题 · 缺证据/轮次不存在 {} 题",
        adversarial, skipped
    );
    println!();

    // ---------- A. 全量 ----------
    println!(
        "── A. 全量（{} 题）：只报两个**各自定义清楚**的量，**不相除** ──",
        cases
    );
    println!(
        "{:<12} {:>7} {:>7} {:>11} {:>11}",
        "样本", "轮次", "题数", "recall@10", "grounded"
    );
    println!("{}", "-".repeat(52));
    let mut all = Aggregate::default();
    let mut failures = 0usize;
    for sample in &samples {
        match run_one(sample, &sample.cases) {
            Some((count, recall, grounded)) => {
                println!(
                    "{:<12} {:>7} {:>7} {:>11.4} {:>11.4}",
                    sample.sample_id,
                    sample.turn_count(),
                    count,
                    recall,
                    grounded
                );
                all.cases += count;
                all.sum_recall += recall * count as f64;
                all.sum_grounded += grounded * count as f64;
            }
            None => {
                println!("{:<12} **跑不动**", sample.sample_id);
                failures += 1;
            }
        }
    }
    println!("{}", "-".repeat(52));
    println!(
        "{:<12} {:>7} {:>7} {:>11.4} {:>11.4}",
        "**汇总**",
        turns,
        all.cases,
        all.recall(),
        all.grounded()
    );
    println!();

    // ---------- B. 单证据子集（**送达率有定义**）----------
    println!(
        "── B. **单证据子集**（{} 题 · 占 {:.1}%）：两种量都是 0/1 ⇒ **送达率有定义** ──",
        single,
        if cases > 0 {
            single as f64 / cases as f64 * 100.0
        } else {
            0.0
        }
    );
    println!(
        "{:<12} {:>7} {:>11} {:>11} {:>11}",
        "样本", "题数", "recall@10", "grounded", "**送达率**"
    );
    println!("{}", "-".repeat(54));
    let mut one = Aggregate::default();
    for sample in &samples {
        let subset: Vec<GoldenCase> = sample
            .cases
            .iter()
            .filter(|case| case.expect_ids.len() == 1)
            .cloned()
            .collect();
        if subset.is_empty() {
            continue;
        }
        match run_one(sample, &subset) {
            Some((count, recall, grounded)) => {
                let delivery = if recall > 0.0 {
                    grounded / recall
                } else {
                    f64::NAN
                };
                println!(
                    "{:<12} {:>7} {:>11.4} {:>11.4} {:>11.4}",
                    sample.sample_id, count, recall, grounded, delivery
                );
                one.cases += count;
                one.sum_recall += recall * count as f64;
                one.sum_grounded += grounded * count as f64;
                if delivery.is_finite() {
                    one.sum_delivery += delivery * count as f64;
                    one.delivery_cases += count;
                }
            }
            None => println!("{:<12} **跑不动**", sample.sample_id),
        }
    }
    println!("{}", "-".repeat(54));
    println!(
        "{:<12} {:>7} {:>11.4} {:>11.4} {:>11.4}",
        "**汇总**",
        one.cases,
        one.recall(),
        one.grounded(),
        one.delivery()
    );
    println!();

    // ---------- 结论 ----------
    if one.cases == 0 {
        eprintln!("· **单证据子集一题都没跑** ⇒ 没测到（不是「结果是 0」）");
        return ExitCode::FAILURE;
    }
    let metrics = Metrics {
        recall_at_k: one.recall(),
        groundedness: one.grounded(),
        ..Metrics::default()
    };
    let verdict = apeireth_memory::delivery_metrics::measure_delivery(&metrics);
    println!("── 结论 ──");
    println!(
        "  🎯 **真实语料（单证据子集 {} 题）上的送达率 = {:.4}**",
        one.cases,
        one.delivery()
    );
    println!(
        "     ⇒ 「检索到了却没送到模型面前」的比例 = **{:.4}**（= 1 − 送达率）",
        verdict.missed_ratio.unwrap_or(f64::NAN)
    );
    println!(
        "  · recall@10 = {:.4} · groundedness = {:.4}",
        one.recall(),
        one.grounded()
    );
    println!(
        "  · 全量 {} 题：recall@10 = {:.4} · groundedness = {:.4}（**不相除** ⇒ 多证据题口径不同）",
        all.cases,
        all.recall(),
        all.grounded()
    );
    println!();
    println!("  ⚠️ 边界（**如实**）：");
    println!(
        "    ① **对抗类（答案=None，{adversarial} 题）没测** ⇒ 「该拒答时拒没拒」**未覆盖**；"
    );
    println!("    ② 一条记忆 = 一个**对话轮次**（短）⇒ 与「长篇叙事按回切」**不同形态**");
    println!("       ⇒ 结论只对「对话轮次级记忆」成立，**不能**直接外推到「按回切」；");
    println!("    ③ `k = 10` 固定 ⇒ 未扫 k 曲线；");
    println!(
        "    ④ **多证据题（{} 题）的送达率仍未定义** ⇒ 需要「逐题送达」新口径（缺口）。",
        all.cases.saturating_sub(one.cases)
    );
    if failures > 0 {
        println!("    ⑤ ⚠️ **{failures} 个样本跑不动** ⇒ 汇总未含它们（**如实标出**）。");
    }
    ExitCode::SUCCESS
}

/// 单证据题数。
fn single_evidence_cases(sample: &LocomoSample) -> usize {
    sample
        .cases
        .iter()
        .filter(|case| case.expect_ids.len() == 1)
        .count()
}

/// 跑一组题 ⇒ `(题数, recall, groundedness)`；失败给 `None`（**不静默**）。
fn run_one(sample: &LocomoSample, cases: &[GoldenCase]) -> Option<(usize, f64, f64)> {
    if cases.is_empty() {
        return None;
    }
    let golden = GoldenSet::new(sample.sample_id.clone(), cases.to_vec());
    let report = run(&sample.records, &golden, K, InjectionMode::Production).ok()?;
    Some((
        report.cases,
        report.metrics.recall_at_k,
        report.metrics.groundedness,
    ))
}
