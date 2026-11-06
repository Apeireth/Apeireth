//! **中文停用词 A/B**（2026-11-06）—— 在**中文真实语料**上重测「去停用词」这个杠杆。
//!
//! # 为什么测这个
//! `locomo_levers2` 在 **LoCoMo（英文）**上测出「去停用词/疑问词」**+0.0544**，
//! 但生产分词器（`hybrid_search::tokenize`）**不过滤停用词**
//! ⇒ ⇒ 这个杠杆**在生产里不存在** ⇒ ⇒ 要不要接，**必须先在中文语料上量**。
//!
//! # 三个臂（唯一变量 = 文档/题面是否去停用词）
//! - **A 基线**：原样；
//! - **B 文档去停用词**：只改**文档**（模拟「写入侧规范化」）；
//! - **C 两边都去**：文档 + 题面（模拟「分词器过滤」）。
//!
//! # 口径
//! 同一批题、同一个 k、同一套检索器；只换索引里的文本。
//! ⇒ ⇒ 两套考卷（内部词面卷 + 公平卷）各报一遍。

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{build_fair_exam, FairExamConfig};
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::{mined_to_golden, run_with_extra_sources, IndexRecord};
use apeireth_memory::golden_set::GoldenSet;
use apeireth_memory::novel_corpus;

const K: usize = 10;
const MAX_CASES_PER_RECORD: usize = 6;

/// 中文停用词（**高频功能词 + 疑问词**）。
const CN_STOPWORDS: &[&str] = &[
    "的", "了", "是", "在", "我", "他", "她", "它", "和", "也", "就", "不", "有", "这", "那", "你",
    "们", "之", "与", "而", "及", "以", "为", "于", "其", "但", "却", "并", "或", "很", "都", "说",
    "道", "云", "曰", "个", "一", "上", "下", "中", "着",
];

/// 去掉中文停用词（**按字去**，因为中文分词器出的是 bigram）。
fn strip_cn(text: &str) -> String {
    text.chars()
        .filter(|c| !CN_STOPWORDS.contains(&c.to_string().as_str()))
        .collect()
}

fn corpus_root() -> PathBuf {
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("novels"),
        None => {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            home.join(".apeireth").join("novels")
        }
    }
}

fn recall(index: &[IndexRecord], set: &GoldenSet) -> Result<f64, Box<dyn Error>> {
    let report = run_with_extra_sources(
        index,
        set,
        K,
        apeireth_memory::golden_run::InjectionMode::Production,
        &[],
    )?;
    Ok(report.metrics.recall_at_k)
}

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("· 失败：{error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut rows: Vec<serde_json::Value> = Vec::new();
    for name in ["redchamber", "watershed"] {
        let path = corpus_root().join(format!("{name}.txt"));
        if !path.exists() {
            eprintln!("· {name}：语料不在，跳过");
            continue;
        }
        let Ok(chapters) = novel_corpus::load_chapters(&path) else {
            eprintln!("· {name}：语料读不了，跳过");
            continue;
        };
        let records = novel_corpus::to_mine_records(&chapters);
        let index: Vec<IndexRecord> = records
            .iter()
            .map(|record| IndexRecord {
                id: record.id.clone(),
                text: record.content.clone(),
            })
            .collect();
        let config = MiningConfig {
            max_cases_per_record: MAX_CASES_PER_RECORD,
            ..MiningConfig::default()
        };
        let mined = apeireth_memory::golden_mining::mine(&records, config);
        let lexical = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));
        let fair = GoldenSet::new(
            format!("fair-{name}"),
            build_fair_exam(&records, FairExamConfig::default()),
        );

        // 臂 B：只改文档
        let index_b: Vec<IndexRecord> = index
            .iter()
            .map(|record| IndexRecord {
                id: record.id.clone(),
                text: strip_cn(&record.text),
            })
            .collect();
        // 臂 C：文档 + 题面都改
        let index_c: Vec<IndexRecord> = index_b.clone();
        let lexical_c = GoldenSet::new(
            format!("internal-{name}-c"),
            lexical
                .cases
                .iter()
                .map(|case| apeireth_memory::golden_set::GoldenCase {
                    id: case.id.clone(),
                    query: strip_cn(&case.query),
                    expect_ids: case.expect_ids.clone(),
                    expect_fragments: case.expect_fragments.clone(),
                    tags: case.tags.clone(),
                })
                .collect(),
        );
        let fair_c = GoldenSet::new(
            format!("fair-{name}-c"),
            fair.cases
                .iter()
                .map(|case| apeireth_memory::golden_set::GoldenCase {
                    id: case.id.clone(),
                    query: strip_cn(&case.query),
                    expect_ids: case.expect_ids.clone(),
                    expect_fragments: case.expect_fragments.clone(),
                    tags: case.tags.clone(),
                })
                .collect(),
        );

        for (exam, set, set_c) in [("词面卷", &lexical, &lexical_c), ("公平卷", &fair, &fair_c)]
        {
            let base = recall(&index, set)?;
            let arm_b = recall(&index_b, set)?;
            let arm_c = recall(&index_c, set_c)?;
            rows.push(serde_json::json!({
                "语料": name,
                "考卷": exam,
                "题数": set.cases.len(),
                "A基线": base,
                "B文档去停用词": arm_b,
                "B的Δ": arm_b - base,
                "C两边都去": arm_c,
                "C的Δ": arm_c - base,
            }));
        }
    }
    for row in &rows {
        println!(
            "· {} / {} · 题 {} · A {:.4} · B {:.4} ({:+.4}) · C {:.4} ({:+.4})",
            row["语料"].as_str().unwrap_or(""),
            row["考卷"].as_str().unwrap_or(""),
            row["题数"],
            row["A基线"].as_f64().unwrap_or(0.0),
            row["B文档去停用词"].as_f64().unwrap_or(0.0),
            row["B的Δ"].as_f64().unwrap_or(0.0),
            row["C两边都去"].as_f64().unwrap_or(0.0),
            row["C的Δ"].as_f64().unwrap_or(0.0),
        );
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "中文停用词A/B": rows })).unwrap_or_default()
    );
    Ok(())
}
