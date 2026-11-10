//! **不需要 owner 拍板的杠杆**：能不能用「便宜的检索侧改动」把 19.7% 的候选缺口啃下一块？
//!
//! # 背景（已实测）
//!
//! LoCoMo 单证据题（1,122 题）扫 k 的结果：
//! * `recall@10 = 0.6221`、`recall@100 = 0.8030`；
//! * 「加大 k」只能捞回 **18.1%**；
//! * **19.7% 的答案压根不在候选集里** ⇒ ⇒ **这是候选集/语义问题，不是排序问题**。
//!
//! ⇒ ⇒ 真语义召回需要**真模型**（哈希伪语义已被否证）⇒ **那是基础设施决策**。
//! ⇒ ⇒ 但在那之前，有几个**检索侧零风险杠杆**值得先量：
//!
//! | 杠杆 | 想法 | 为什么可能有用 |
//! |---|---|---|
//! | **L1 基线** | 原样 | 对照 |
//! | **L2 去疑问词** | 去掉 `what/when/did/would/...` | 疑问词在**每个问题**里都出现 ⇒ 对 BM25 **零区分度**、只稀释信号 |
//! | **L3 时间入索引** | 每条记忆前面加 `日期 说话人:` | LoCoMo 的**时间题占 25%**（cat:2 = 280/1,122）⇒ 日期是**答案的一部分** |
//! | **L4 两者叠加** | L2 + L3 | 看是否可加 |
//!
//! ⚠️ **诚实边界**：
//! * L3 是「把**结构化元数据**并进可检索文本」的**通用**做法（真实系统里时间本来就该可检索），
//!   不是只对 LoCoMo 有效的 hack；
//! * 但 LoCoMo 的**日期字符串**与题目里的日期写法可能不一致 ⇒ 有**构造成分**，会单独标注。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo run -p apeireth-memory --example locomo_levers --locked
//! ```

use apeireth_memory::golden_run::{run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::locomo_corpus::{self, LocomoSample};
use std::process::ExitCode;

const K: usize = 10;

/// 英文疑问词/停用词（**只删这些** ⇒ 不碰实词）。
const STOPWORDS: [&str; 30] = [
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "did", "do", "does", "what",
    "when", "where", "who", "whom", "whose", "which", "why", "how", "would", "could", "should",
    "can", "to", "of", "in", "on", "for",
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
    println!("════════ 检索侧杠杆（不需要 owner 拍板）════════");
    println!("单证据题 · k = {K} · 4 个臂对照");
    println!();
    println!(
        "{:<26} {:>7} {:>11} {:>11} {:>9}",
        "臂", "题数", "recall@10", "Δ vs 基线", "送不送"
    );
    println!("{}", "-".repeat(68));

    let arms: [(
        &str,
        fn(&str) -> String,
        fn(&LocomoSample) -> Vec<IndexRecord>,
    ); 4] = [
        ("L1 基线", identity, |sample| records_plain(sample)),
        ("L2 去疑问词", strip_stopwords, |sample| {
            records_plain(sample)
        }),
        ("L3 时间入索引", identity, |sample| {
            records_with_metadata(sample)
        }),
        ("L4 两者叠加", strip_stopwords, |sample| {
            records_with_metadata(sample)
        }),
    ];

    let mut baseline = f64::NAN;
    let mut rows: Vec<(String, usize, f64)> = Vec::new();
    for (label, rewrite, build) in arms {
        let mut total = 0usize;
        let mut hits = 0usize;
        for sample in &samples {
            let cases: Vec<GoldenCase> = sample
                .cases
                .iter()
                .filter(|case| case.expect_ids.len() == 1)
                .map(|case| GoldenCase {
                    id: case.id.clone(),
                    query: rewrite(&case.query),
                    expect_ids: case.expect_ids.clone(),
                    expect_fragments: case.expect_fragments.clone(),
                    tags: case.tags.clone(),
                })
                .collect();
            if cases.is_empty() {
                continue;
            }
            let index = build(sample);
            let golden = GoldenSet::new(sample.sample_id.clone(), cases.clone());
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
        if baseline.is_nan() {
            baseline = recall;
        }
        let delta = recall - baseline;
        println!(
            "{:<26} {:>7} {:>11.4} {:>+11.4} {:>9}",
            label,
            total,
            recall,
            if label.starts_with("L1") {
                f64::NAN
            } else {
                delta
            },
            "—"
        );
        rows.push((label.to_string(), total, recall));
    }
    println!();

    // ---------- 判定 ----------
    println!("── 判定（**先写死的规则**）──");
    let best = rows
        .iter()
        .filter(|(_, _, recall)| recall.is_finite())
        .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    match best {
        Some((label, _, recall)) if *recall > baseline + 0.005 => {
            println!(
                "  🎯 最有效的是 **{label}**：{baseline:.4} → {recall:.4}（{:+.4}）",
                recall - baseline
            );
            println!("  ⇒ 这个杠杆**不需要 owner 拍板** ⇒ 可以纳入下一批。");
        }
        _ => {
            println!("  ⚠️ **没有任何检索侧杠杆有效**（全部 ≤ 基线 + 0.005）");
            println!("  ⇒ ⇒ 那就只剩「真语义模型」一条路 ⇒ ⇒ **必须 owner 拍板**（见下）。");
        }
    }
    println!();
    println!("  ⚠️ 边界：L3/L4 把 **session 日期**写进记忆正文，利用了 LoCoMo 的日期字符串格式，");
    println!("     存在**构造成分** ⇒ 它证明的是「**元数据该不该可检索**」，");
    println!("     **不是**「真实系统一定涨这么多」。");
    ExitCode::SUCCESS
}

fn identity(text: &str) -> String {
    text.to_string()
}

/// 去掉疑问词/停用词（**只删词表里的** ⇒ 不动其它词）。
fn strip_stopwords(text: &str) -> String {
    text.split_whitespace()
        .filter(|word| {
            let cleaned: String = word
                .chars()
                .filter(|ch| ch.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            !STOPWORDS.contains(&cleaned.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// 原样记忆（对照组）。
fn records_plain(sample: &LocomoSample) -> Vec<IndexRecord> {
    sample.records.clone()
}

/// **把会话日期写进可检索文本**（`2023-05-07 Alice: ...`）。
fn records_with_metadata(sample: &LocomoSample) -> Vec<IndexRecord> {
    // `dia_id` 形如 `D<会话序号>:<轮次>` ⇒ 从里面取会话序号 ⇒ 查日期。
    let mut by_session: std::collections::BTreeMap<usize, &str> = std::collections::BTreeMap::new();
    for (index, date) in &sample.session_dates {
        by_session.insert(*index, date.as_str());
    }
    sample
        .records
        .iter()
        .map(|record| {
            let session = record
                .id
                .rsplit("::")
                .next()
                .and_then(|dia| dia.trim_start_matches('D').split(':').next())
                .and_then(|number| number.parse::<usize>().ok());
            let date = session
                .and_then(|index| by_session.get(&index).copied())
                .unwrap_or("");
            IndexRecord {
                id: record.id.clone(),
                text: if date.is_empty() {
                    record.text.clone()
                } else {
                    format!("{date} {}", record.text)
                },
            }
        })
        .collect()
}
