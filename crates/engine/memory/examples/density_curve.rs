//! **主体密度 → recall** 的关系曲线（第四个杠杆的第一个实测）。
//!
//! # 这个例子回答什么
//!
//! 上一批（`corpora_robustness`）发现：**语料减半 ⇒ `recall@10` 反而涨**
//! （《紅樓夢》0.2983 → 0.4578 / 0.3891；《水滸傳》0.6875 → 0.8508 / 0.6895）
//! ⇒ ⇒ **同主体竞争者变少 ⇒ 排序变容易** ⇒ ⇒ **「同主体候选有多少」是自变量**
//! ⇒ ⇒ 这与 W-20R 的机制**一致**（相对位置由密度决定），但**方向相反**：
//! W-20 是「**切细** ⇒ 候选变多 ⇒ 更难」；这里是「**语料变小** ⇒ 候选变少 ⇒ **更容易****。
//!
//! ⚠️ **本例子的推进**：不只是「两半」，而是**一条曲线**
//! （1/4、1/2、3/4、全量）⇒ ⇒ 看「**密度 ↔ recall**」是不是**单调**的。
//!
//! # 三条纪律
//!
//! 1. **同一套口径**：同 k、同一挖题配置、同一检索器、**同一批题**
//!    ⇒ ⇒ **题目不变，只换语料规模**（**唯一变量 = 语料规模**）
//!    ⇒ ⇒ 这比「切半」更干净（**题数不变** ⇒ **难度分布不变** ⇒ **补上了上一批的缺口**）；
//! 2. **题集固定**：用**全量语料**挖出的题，在**每个规模**下都跑同一批题
//!    ⇒ ⇒ **部分题的目标记录不在小语料里** ⇒ 那部分**必然答不中**
//!    ⇒ ⇒ ⚠️ 这会让小规模的 recall **被压低** ⇒ **必须**同时报「目标在库率」
//!    （**如实**，否则会低估规模的影响）；
//! 3. **密度口径**：**同主体竞争者中位数**（含该主体的记录数）—— 与 `chunking::competitor_density` 同口径。

use apeireth_memory::delivery_metrics::measure_delivery;
use apeireth_memory::golden_mining::{self, MiningConfig};
use apeireth_memory::golden_run::{run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::GoldenCase;
use apeireth_memory::novel_corpus::{self, Chapter};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const K: usize = 10;
/// 规模阶梯（**分数**；1.0 = 全量）。
const SCALES: [f64; 4] = [0.25, 0.5, 0.75, 1.0];

fn main() -> ExitCode {
    let paths = novels();
    if paths.is_empty() {
        eprintln!("· 语料不在（$APEIRETH_DATA_DIR/novels/ 或 ~/.apeireth/novels/）⇒ **没测到**");
        return ExitCode::FAILURE;
    }
    for (name, path) in paths {
        if let Err(error) = report_one(&name, &path) {
            eprintln!("· {name}：{error}");
        }
    }
    ExitCode::SUCCESS
}

fn novels() -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("APEIRETH_DATA_DIR") {
        roots.push(PathBuf::from(dir).join("novels"));
    }
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(home).join(".apeireth/novels"));
    }
    for root in roots {
        out.push(("redchamber".to_string(), root.join("redchamber.txt")));
        out.push(("watershed".to_string(), root.join("watershed.txt")));
    }
    out.retain(|(_, path)| path.exists());
    let mut seen: Vec<PathBuf> = Vec::new();
    out.retain(|(_, path)| {
        if seen.contains(path) {
            false
        } else {
            seen.push(path.clone());
            true
        }
    });
    out
}

fn report_one(name: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    if chapters.is_empty() {
        return Err("语料解析出 0 条记录".into());
    }
    let records = novel_corpus::to_mine_records(&chapters);
    let config = MiningConfig {
        max_cases_per_record: 6,
        ..MiningConfig::default()
    };
    let mined = golden_mining::mine(&records, config);
    let cases: Vec<GoldenCase> = mined
        .iter()
        .map(|case| GoldenCase {
            id: case.id.clone(),
            query: case.query.clone(),
            expect_ids: case.expect_ids.clone(),
            expect_fragments: case.expect_fragments.clone(),
            tags: vec![case.kind.clone()],
        })
        .collect();
    if cases.is_empty() {
        return Err("没挖出题".into());
    }

    println!("════════ {name} ════════");
    println!(
        "{} 回 · 题 {} 道（**全量语料挖的固定题集**，各规模下**同一批题**）· k={K}",
        chapters.len(),
        cases.len()
    );
    println!();
    println!(
        "{:<10} {:>8} {:>14} {:>14} {:>14} {:>12}",
        "规模", "回数", "目标在库率", "同主体中位", "recall@10", "送达率"
    );
    println!("{}", "-".repeat(76));

    let mut rows: Vec<(f64, f64, f64, f64)> = Vec::new();
    for scale in SCALES {
        let take = ((chapters.len() as f64) * scale).round() as usize;
        let take = take.max(1).min(chapters.len());
        let subset: Vec<Chapter> = chapters[..take].to_vec();
        let subset_records = novel_corpus::to_mine_records(&subset);
        let index: Vec<IndexRecord> = subset_records
            .iter()
            .map(|record| IndexRecord {
                id: record.id.clone(),
                text: record.content.clone(),
            })
            .collect();
        let ids: Vec<&str> = index.iter().map(|record| record.id.as_str()).collect();

        // ⚠️ **目标在库率**：这批题的目标记录有多少**在当前规模的语料里**
        // ⇒ **必须**报（否则小规模 recall 被压低会**误读**成「规模影响更大」）。
        let mut in_library = 0usize;
        let mut total_targets = 0usize;
        for case in &cases {
            for target in &case.expect_ids {
                total_targets += 1;
                if ids.iter().any(|id| *id == target.as_str()) {
                    in_library += 1;
                }
            }
        }
        let in_library_ratio = if total_targets == 0 {
            0.0
        } else {
            in_library as f64 / total_targets as f64
        };

        // 同主体竞争者中位数（与 `chunking::competitor_density` 同口径）。
        let densities: Vec<usize> = cases
            .iter()
            .map(|case| {
                apeireth_memory::query_signals::parse_signals(case)
                    .subject
                    .map(|subject| {
                        index
                            .iter()
                            .filter(|record| record.text.contains(&subject))
                            .count()
                    })
                    .unwrap_or(0)
            })
            .collect();
        let median_density = median(&densities);

        let golden =
            apeireth_memory::golden_set::GoldenSet::new("scale".to_string(), cases.clone());
        let report = run(&index, &golden, K, InjectionMode::Production)?;
        let delivery = measure_delivery(&report.metrics);
        let delivery_rate = delivery.delivery_rate.unwrap_or(0.0);

        println!(
            "{:<10} {:>8} {:>14.4} {:>14} {:>14.4} {:>12.4}",
            format!("{:.0}%", scale * 100.0),
            index.len(),
            in_library_ratio,
            median_density,
            report.metrics.recall_at_k,
            delivery_rate
        );
        rows.push((
            scale,
            in_library_ratio,
            median_density as f64,
            report.metrics.recall_at_k,
        ));
    }
    println!();

    // ---------- 关系（**如实**）----------
    println!("── 关系（**如实**）──");
    for (scale, in_library, density, recall) in &rows {
        println!(
            "  规模 {:>5.0}%：同主体中位 {:>4} 条 · 目标在库率 {:.4} · recall@10 {:.4}",
            scale * 100.0,
            density,
            in_library,
            recall
        );
    }
    if rows.len() >= 2 {
        let first = rows[0];
        let last = rows[rows.len() - 1];
        println!(
            "  ⇒ 从 {:.0}% 到 {:.0}%：同主体中位 {:.0} → {:.0} 条；recall {:.4} → {:.4}（Δ {:+.4}）",
            first.0 * 100.0,
            last.0 * 100.0,
            first.2,
            last.2,
            first.3,
            last.3,
            last.3 - first.3
        );
        println!(
            "  ⇒ ⚠️ **目标在库率**也变了（{:.4} → {:.4}）⇒ recall 的变化**混着**「目标不在库里」的成分",
            first.1, last.1
        );
        println!("  ⇒ ⇒ 只在**目标在库率相同**的规模之间比较 ⇒ 才是**干净的**「密度效应」");
    }
    println!();
    println!("  ⚠️ 口径：");
    println!("    · **题集固定**（全量语料挖的）⇒ **难度分布不变** ⇒ 比「切半」更干净；");
    println!("    · 但小规模下**部分题的目标不在库里** ⇒ 那部分**必然答不中**");
    println!("      ⇒ 故**必须**同时看「目标在库率」（**已报**）");
    println!("    · 「同主体中位」= 题面主体词在语料里出现的**记录数中位数**");
    Ok(())
}

fn median(values: &[usize]) -> usize {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[middle]
    } else {
        (sorted[middle - 1] + sorted[middle]) / 2
    }
}
