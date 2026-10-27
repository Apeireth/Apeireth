//! **结论稳健性**检验：把语料**切开**，看结论**是否稳定**。
//!
//! # 这个例子回答什么
//!
//! 今天的实测结论都来自**两部古典小说** ⇒ 而《紅樓夢》与《水滸傳》之间
//! **差异极大**（recall 0.2983 vs 0.6875）⇒ ⇒ 「数字**不可迁移**」是事实。
//!
//! ⚠️ **但真正要回答的是**：结论**本身**（「**信号缺失 ⇒ L1/L2 同时失效**」）
//! **是否依赖这两本书的具体内容**？
//!
//! ⇒ 办法：**把同一本书切成两半**（前半 / 后半）⇒ 得到**两部独立子语料**
//! ⇒ ⇒ 若结论在两个子语料上**一致** ⇒ ⇒ 结论**不是这两本书的巧合**。
//!
//! # 三条纪律
//!
//! 1. **同一套口径**：同一份代码、同一 k、同一挖题配置、同一检索器；
//! 2. **子语料是独立测量**：各自挖题、各自检索 ⇒ **不是**在同一份卷子上切；
//! 3. **如实**：若两个子语料**不一致** ⇒ **如实说不一致**（**不许**只报好看的那个）。
//!
//! # ⚠️ 这个检验的**边界**
//!
//! * 它检验的是「**同一本书的两半**」⇒ **不是**「换一本书」
//!   ⇒ ⇒ 能排除「**某本书特有的巧合**」，**不能**排除「**长篇叙事的共性**」；
//! * 真正的外部验证需要**第三部语料**（**本机没有** ⇒ 记为缺口）。

use apeireth_memory::delivery_metrics::measure_delivery;
use apeireth_memory::golden_mining::{self, MiningConfig};
use apeireth_memory::golden_run::{run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::GoldenCase;
use apeireth_memory::novel_corpus::{self, Chapter};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const K: usize = 10;

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
    let mid = chapters.len() / 2;
    let halves: Vec<(&str, Vec<Chapter>)> = vec![
        ("前半", chapters[..mid].to_vec()),
        ("后半", chapters[mid..].to_vec()),
    ];

    println!("════════ {name} ════════");
    println!(
        "{} 回 · 切成两半（各 {} / {} 回）· 各自挖题、各自检索 ⇒ **独立测量**",
        chapters.len(),
        mid,
        chapters.len() - mid
    );
    println!();
    println!(
        "{:<12} {:>8} {:>8} {:>12} {:>12} {:>12}",
        "子语料", "回数", "题数", "recall@10", "grounded", "送达率"
    );
    println!("{}", "-".repeat(70));

    let mut rows: Vec<(String, f64, f64, f64)> = Vec::new();
    for (label, subset) in halves {
        let (recall, grounded, delivery, cases) = measure_subset(&subset)?;
        println!(
            "{:<12} {:>8} {:>8} {:>12.4} {:>12.4} {:>12.4}",
            label,
            subset.len(),
            cases,
            recall,
            grounded,
            delivery
        );
        rows.push((label.to_string(), recall, grounded, delivery));
    }
    println!();

    // ---------- 一致性判定（**如实**）----------
    if rows.len() == 2 {
        let (label_a, recall_a, grounded_a, delivery_a) = &rows[0];
        let (label_b, recall_b, grounded_b, delivery_b) = &rows[1];
        println!("── 一致性（**如实**）──");
        println!(
            "  recall@10：{label_a} {:.4} vs {label_b} {:.4}（Δ {:+.4}）",
            recall_a,
            recall_b,
            recall_b - recall_a
        );
        println!(
            "  groundedness：{label_a} {:.4} vs {label_b} {:.4}（Δ {:+.4}）",
            grounded_a,
            grounded_b,
            grounded_b - grounded_a
        );
        println!(
            "  送达率：{label_a} {:.4} vs {label_b} {:.4}（Δ {:+.4}）",
            delivery_a,
            delivery_b,
            delivery_b - delivery_a
        );
        // ⚠️ **判据**：送达率**两边都低于 1** ⇒ 「**漏送**」这件事**与具体内容无关**。
        let both_miss = *delivery_a < 1.0 && *delivery_b < 1.0;
        println!(
            "  ⇒ 「**检索到了却没送到**」在两半上都成立：{}",
            if both_miss {
                "**是**（⇒ 不是这本书的巧合）"
            } else {
                "**否**（⇒ 要如实复核）"
            }
        );
        println!(
            "  ⇒ 「**L1 与 L2 同时偏低**」在两半上都成立：{}",
            if *recall_a < 0.7 && *recall_b < 0.7 {
                "**是**"
            } else {
                "**否**（有一半很高 ⇒ 结论要收窄）"
            }
        );
    }
    println!();
    println!("  ⚠️ 边界：这是「**同一本书的两半**」⇒ 能排除「**某本书特有的巧合**」，");
    println!("     **不能**排除「**长篇叙事的共性**」⇒ 真正的外部验证需要**第三部语料**。");
    Ok(())
}

/// 测一个子语料 ⇒ (recall, groundedness, 送达率, 题数)。
fn measure_subset(
    chapters: &[Chapter],
) -> Result<(f64, f64, f64, usize), Box<dyn std::error::Error>> {
    let records = novel_corpus::to_mine_records(chapters);
    if records.is_empty() {
        return Err("子语料解析出 0 条记录".into());
    }
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
        return Err("子语料没挖出题".into());
    }
    let golden = apeireth_memory::golden_set::GoldenSet::new("half".to_string(), cases);
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();
    let report = run(&index, &golden, K, InjectionMode::Production)?;
    let delivery = measure_delivery(&report.metrics);
    Ok((
        report.metrics.recall_at_k,
        report.metrics.groundedness,
        delivery.delivery_rate.unwrap_or(0.0),
        report.cases,
    ))
}
