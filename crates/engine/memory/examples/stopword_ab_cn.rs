//! **停用字表 A/B**（2026-11-07 重写）—— 找出「既保住增益、又不削实词」的表。
//!
//! # 为什么重写
//! 第一版在**生产已开启规范化**的前提下比较三张表 ⇒ ⇒ **三组数字完全相同**
//! （基线本身就是 L1 状态）⇒ ⇒ **量不出差异**。
//!
//! # 本次口径（关键）
//! 所有臂都用 `Bm25Config { strip_cn_stopwords: false }` **关掉生产规范化**
//! ⇒ ⇒ 由实验**手动**应用「去疑问词 + 指定停用字表」
//! ⇒ ⇒ ⇒ 于是「换表」成为**唯一变量**。
//!
//! # 四臂
//! - **基线**：只去疑问词，**不去停用字**；
//! - **L1 宽表**：生产现用（41 字）；
//! - **L2 收窄**：20 字（去掉会做实词构件的字）；
//! - **L3 极简**：16 字。
//!
//! # 为什么关心「削实词」
//! 用真实查询（`scripts/fixtures/miracl-zh-dev-queries.txt`）实测：
//! `巴西的首都是哪里？` 经 L1 变成 `巴西首？` ⇒ ⇒ **「首都」被毁**
//! （因为 `都` 在 L1 表里）⇒ ⇒ 这类字是**实词构件**，不该无条件删。

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{build_fair_exam, FairExamConfig};
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::{
    mined_to_golden, run_with_extra_sources_and_config, IndexRecord,
};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::novel_corpus;

const K: usize = 10;
const MAX_CASES_PER_RECORD: usize = 6;

/// **L1 宽表**（生产现用）：含 `都/道/为/以/及/之/说/云/曰/一/上/下/中/着` 等**实词构件**。
const L1_BROAD: &[char] = &[
    '的', '了', '是', '在', '我', '他', '她', '它', '和', '也', '就', '不', '有', '这', '那', '你',
    '们', '之', '与', '而', '及', '以', '为', '于', '其', '但', '却', '并', '或', '很', '都', '说',
    '道', '云', '曰', '个', '一', '上', '下', '中', '着',
];

/// **L2 收窄**：只保留**几乎不做实词构件**的字。
const L2_NARROW: &[char] = &[
    '的', '了', '是', '在', '和', '也', '就', '不', '有', '这', '那', '我', '他', '她', '它', '你',
    '们', '与', '于', '但',
];

/// **L3 极简**：连 `与/于/但` 也去掉（它们常做构件：参与/关于/由于）。
const L3_MINIMAL: &[char] = &[
    '的', '了', '是', '在', '和', '也', '就', '不', '这', '那', '我', '他', '她', '它', '你', '们',
];

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

/// 按给定表 + 去疑问词生成索引与题集副本。
fn prepare(
    index: &[IndexRecord],
    set: &GoldenSet,
    stopwords: &[char],
) -> (Vec<IndexRecord>, GoldenSet) {
    let strip = |text: &str| -> String {
        apeireth_memory::hybrid_search::strip_cn_interrogatives(text)
            .chars()
            .filter(|c| !stopwords.contains(c))
            .collect()
    };
    let new_index: Vec<IndexRecord> = index
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: strip(&record.text),
        })
        .collect();
    let new_set = GoldenSet::new(
        format!("arm"),
        set.cases
            .iter()
            .map(|case| GoldenCase {
                id: case.id.clone(),
                query: strip(&case.query),
                expect_ids: case.expect_ids.clone(),
                expect_fragments: case.expect_fragments.clone(),
                tags: case.tags.clone(),
            })
            .collect(),
    );
    (new_index, new_set)
}

/// **关掉生产规范化**后跑批（否则基线本身就是 L1 状态 ⇒ 量不出差异）。
fn recall(index: &[IndexRecord], set: &GoldenSet) -> Result<f64, Box<dyn Error>> {
    let config = apeireth_memory::hybrid_search::Bm25Config {
        strip_cn_stopwords: false,
        ..Default::default()
    };
    let report = run_with_extra_sources_and_config(
        index,
        set,
        K,
        apeireth_memory::golden_run::InjectionMode::Production,
        &[],
        Some(config),
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

        for (exam, set) in [("词面卷", &lexical), ("公平卷", &fair)] {
            let (i0, s0) = prepare(&index, set, &[]);
            let base = recall(&i0, &s0)?;
            let (i1, s1) = prepare(&index, set, L1_BROAD);
            let l1 = recall(&i1, &s1)?;
            let (i2, s2) = prepare(&index, set, L2_NARROW);
            let l2 = recall(&i2, &s2)?;
            let (i3, s3) = prepare(&index, set, L3_MINIMAL);
            let l3 = recall(&i3, &s3)?;
            rows.push(serde_json::json!({
                "语料": name,
                "考卷": exam,
                "题数": set.cases.len(),
                "基线": base,
                "L1": l1,
                "L1的Δ": l1 - base,
                "L2": l2,
                "L2的Δ": l2 - base,
                "L3": l3,
                "L3的Δ": l3 - base,
            }));
        }
    }
    for row in &rows {
        println!(
            "· {} / {} · 题 {} · 基线 {:.4} | L1 {:.4} ({:+.4}) | L2 {:.4} ({:+.4}) | L3 {:.4} ({:+.4})",
            row["语料"].as_str().unwrap_or(""),
            row["考卷"].as_str().unwrap_or(""),
            row["题数"],
            row["基线"].as_f64().unwrap_or(0.0),
            row["L1"].as_f64().unwrap_or(0.0),
            row["L1的Δ"].as_f64().unwrap_or(0.0),
            row["L2"].as_f64().unwrap_or(0.0),
            row["L2的Δ"].as_f64().unwrap_or(0.0),
            row["L3"].as_f64().unwrap_or(0.0),
            row["L3的Δ"].as_f64().unwrap_or(0.0),
        );
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "停用字表A/B": rows })).unwrap_or_default()
    );
    Ok(())
}
