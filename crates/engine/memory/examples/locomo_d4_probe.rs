//! **B0-c 探针**：把 D4 候选池里**能上检索卷子**的算法逐个开一遍，各出一行数字。
//!
//! # 设计稿 §8 B0-c 的验收
//!
//! > **36 个模块进候选池，并各有一行数字**
//!
//! ⇒ 候选池结构在 `scripts/d4-ledger.json`（41 模块 / 20 个可测候选）；
//! ⇒ 本例子负责给**能在检索卷子上测的那几个**回填数字。
//!
//! # 本卷上**真正可测**的（其余在 `d4-ledger.json` 里标了原因）
//!
//! | 算法 | 怎么测 |
//! |---|---|
//! | `memory_rank` | 它的 `memory_score` = 重要度×3 + 访问×0.3 + 分组加成 + **新旧×2** ⇒ 在本卷上等价于「**按新旧重排**」 |
//! | `dedup` | 按 `normalize_for_dedup` + Jaccard **去近似重复** ⇒ 看能否**腾出 top-k 名额** |
//!
//! ⚠️ **负结果也是结果**：候选池的意义就是「**量出来，再决定**」——
//! 若某个算法在本卷上掉分，就如实记「**本卷上无正收益**」，不粉饰。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo run -p apeireth-memory --example locomo_d4_probe --locked
//! ```

use apeireth_memory::golden_run::{run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::locomo_corpus::{self, LocomoSample};
use apeireth_memory::{dedup, memory_rank};
use std::collections::BTreeMap;
use std::process::ExitCode;

const K: usize = 10;

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

fn main() -> ExitCode {
    let Some(path) = locomo_corpus::default_path() else {
        eprintln!("· 找不到 locomo10.json ⇒ **没测到**");
        return ExitCode::FAILURE;
    };
    let samples = match locomo_corpus::load(&path) {
        Ok(samples) => samples,
        Err(error) => {
            eprintln!("· 加载失败：{error}");
            return ExitCode::FAILURE;
        }
    };
    println!("════════ B0-c 探针：D4 候选池 · 检索卷子可测项 ════════");
    println!("单证据题 · k = {K} · 其它一律不动（唯一变量 = 算法开关）");
    println!();
    println!(
        "{:<30} {:>6} {:>11} {:>12} {:>10}",
        "臂", "题数", "recall@10", "Δ vs 基线", "结论"
    );
    println!("{}", "-".repeat(74));

    let arms: [(&str, Arm); 3] = [
        ("基线（BM25）", Arm::Baseline),
        ("+ memory_rank 重排", Arm::MemoryRank),
        ("+ dedup 去近似重复", Arm::Dedup),
    ];
    let mut baseline = f64::NAN;
    let mut rows: Vec<(String, f64, bool)> = Vec::new();
    for (label, arm) in arms {
        let mut total = 0usize;
        let mut hits = 0usize;
        for sample in &samples {
            let cases: Vec<GoldenCase> = sample
                .cases
                .iter()
                .filter(|case| case.expect_ids.len() == 1)
                .cloned()
                .collect();
            if cases.is_empty() {
                continue;
            }
            let (index, remap) = build_index(sample, arm);
            if index.is_empty() {
                continue;
            }
            // ⚠️ `dedup` 会把近似重复的**后一条**并到**前一条**
            // ⇒ ⇒ 若题目期望的正是被并掉的那条 ⇒ **必须重映射**到留下的那条
            // ⇒ ⇒ 否则会把它记成「检索不到」⇒ **假掉分**（早先 −0.2692 就是这么来的）。
            let cases: Vec<GoldenCase> = cases
                .into_iter()
                .map(|mut case| {
                    case.expect_ids = case
                        .expect_ids
                        .iter()
                        .map(|id| remap.get(id).cloned().unwrap_or_else(|| id.clone()))
                        .collect();
                    case
                })
                .collect();
            let golden = GoldenSet::new(sample.sample_id.clone(), cases);
            let Ok(report) = run(&index, &golden, K, InjectionMode::Production) else {
                continue;
            };
            total += report.cases;
            hits += report.per_case_hit.iter().filter(|(_, hit)| *hit).count();
        }
        let recall = if total > 0 {
            hits as f64 / total as f64
        } else {
            f64::NAN
        };
        let is_baseline = label.starts_with("基线");
        if is_baseline {
            baseline = recall;
        }
        let delta = recall - baseline;
        let verdict = if is_baseline {
            "—"
        } else if delta > 0.005 {
            "**有正收益**"
        } else if delta < -0.005 {
            "**掉分** ⇒ 不采用"
        } else {
            "无显著差异"
        };
        println!(
            "{:<30} {:>6} {:>11.4} {:>12} {:>10}",
            label,
            total,
            recall,
            if is_baseline {
                "—".to_string()
            } else {
                format!("{delta:+.4}")
            },
            verdict
        );
        rows.push((label.to_string(), recall, !is_baseline));
    }
    println!();
    println!("── 结论（**如实**）──");
    println!("  基线 recall@10 = {baseline:.4}");
    let mut any_win = false;
    for (label, recall, _) in &rows {
        if label.starts_with("基线") {
            continue;
        }
        let delta = recall - baseline;
        if delta > 0.005 {
            any_win = true;
            println!("  ✅ **{label}** 有正收益：{delta:+.4} ⇒ 可进下一批（考虑接生产）");
        } else {
            println!("  ❌ **{label}**：{delta:+.4} ⇒ **本卷上无正收益**（如实记录，不采用）");
        }
    }
    if !any_win {
        println!();
        println!("  ⚠️ 本卷上**没有一个 D4 候选带来正收益**");
        println!("     ⇒ ⇒ 这正是候选池的价值：**先量再决定**，而不是「看起来聪明就接上去」");
    }
    println!();
    println!("  ⚠️ 边界：");
    println!("    ① `memory_rank` 在本卷上等价于「**按新旧重排**」");
    println!(
        "       （重要度全默认 5 · 分组加成 0 ⇒ 只剩 recency 项）⇒ 不代表它在真实库上的表现；"
    );
    println!("    ② `dedup` 只测「去重腾名额」这一面；它更主要的作用是**写入门禁**，不是重排；");
    println!("    ③ 其余 18 个候选**本卷测不了**（缺标签/缺图/缺预测）⇒ 见 `scripts/d4-ledger.json` 的 `measurable_by`。");
    ExitCode::SUCCESS
}

#[derive(Clone, Copy)]
enum Arm {
    Baseline,
    MemoryRank,
    Dedup,
}

/// 按臂构造索引。
fn build_index(sample: &LocomoSample, arm: Arm) -> (Vec<IndexRecord>, BTreeMap<String, String>) {
    // ① 解析会话日期 ⇒ epoch 秒（`memory_rank` 要时间戳）。
    let mut session_epoch: BTreeMap<usize, i64> = BTreeMap::new();
    for (index, raw) in &sample.session_dates {
        session_epoch.insert(*index, parse_epoch_seconds(raw));
    }
    let epoch_of = |record_id: &str| -> i64 {
        record_id
            .rsplit("::")
            .next()
            .and_then(|dia| dia.trim_start_matches('D').split(':').next())
            .and_then(|number| number.parse::<usize>().ok())
            .and_then(|index| session_epoch.get(&index).copied())
            .unwrap_or(0)
    };

    match arm {
        Arm::Baseline => (sample.records.clone(), BTreeMap::new()),
        Arm::Dedup => dedup_records(sample),
        Arm::MemoryRank => {
            // ③ 用 `memory_score` 排序 ⇒ 但本卷上没有重要度/访问/分组信号
            // ⇒ 等价于「按新旧重排」⇒ **故意照实做**（不偷偷加别的信号）。
            let now_unix = session_epoch.values().copied().max().unwrap_or(0);
            // ⚠️ 排序必须用 **f64**：`memory_score` 的 recency 项是 0~2 的**小数**
            // ⇒ ⇒ 早先写成 `score as i64` ⇒ 全被截成 0 ⇒ **假「无差异」**（已修）。
            let mut rankable: Vec<(f64, IndexRecord)> = sample
                .records
                .iter()
                .map(|record| {
                    let epoch = epoch_of(&record.id);
                    let item = memory_rank::RankableMemory::new(
                        record.id.clone(),
                        record.text.clone(),
                        epoch,
                    );
                    let score = memory_rank::memory_score(&item, 0, now_unix);
                    (score, record.clone())
                })
                .collect();
            // 分数降序；同分保持原序（`sort_by` 是稳定的）。
            rankable.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            (
                rankable.into_iter().map(|(_, record)| record).collect(),
                BTreeMap::new(),
            )
        }
    }
}

/// 解析 `"1:56 pm on 8 May, 2023"` ⇒ epoch 秒（粗解析，够排序用）。
fn parse_epoch_seconds(raw: &str) -> i64 {
    let lowered = raw.to_lowercase();
    let year = lowered
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|piece| piece.len() == 4)
        .find_map(|piece| piece.parse::<i32>().ok())
        .unwrap_or(1970);
    let Some(month) = MONTHS
        .iter()
        .position(|name| lowered.contains(name))
        .map(|index| index as i32 + 1)
    else {
        return 0;
    };
    let before = lowered
        .split(MONTHS[(month - 1) as usize])
        .next()
        .unwrap_or("");
    let day = before
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|piece| !piece.is_empty() && piece.len() <= 2)
        .filter_map(|piece| piece.parse::<i32>().ok())
        .filter(|value| (1..=31).contains(value))
        .next_back()
        .unwrap_or(1);
    days_from_civil(year, month, day) * 86_400
}

/// Howard Hinnant 的 `days_from_civil`（公历 ⇒ 1970-01-01 起的天数）。
fn days_from_civil(year: i32, month: i32, day: i32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era) * 146_097 + i64::from(day_of_era) - 719_468
}

/// **去近似重复**：把 Jaccard 过高的后一条并到前一条 ⇒ 返回**重映射表**。
///
/// ⚠️ 关键：被并掉的记录若是某题的**答案** ⇒ 必须把它重映射到留下的那条
/// ⇒ ⇒ 否则题目会变成「检索不到」⇒ **假掉分**。
fn dedup_records(sample: &LocomoSample) -> (Vec<IndexRecord>, BTreeMap<String, String>) {
    let mut kept: Vec<IndexRecord> = Vec::new();
    let mut normalized: Vec<String> = Vec::new();
    let mut remap: BTreeMap<String, String> = BTreeMap::new();
    for record in &sample.records {
        let fingerprint = dedup::normalize_for_dedup(&record.text);
        let mut merged_into: Option<String> = None;
        for (position, existing) in normalized.iter().enumerate() {
            if dedup::overlap_ratio(existing, &fingerprint) > 0.95 {
                merged_into = Some(kept[position].id.clone());
                break;
            }
        }
        match merged_into {
            Some(target) => {
                remap.insert(record.id.clone(), target);
            }
            None => {
                normalized.push(fingerprint);
                kept.push(record.clone());
            }
        }
    }
    (kept, remap)
}
