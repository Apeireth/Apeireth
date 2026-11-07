//! **中文查询侧杠杆 A/B**（2026-11-07）—— 在**中文真实语料**上重测「查询侧规范化」这一族杠杆。
//!
//! # 为什么测这个
//! 2026-11-06 在 **LoCoMo（英文）**上测出「去疑问词」**+0.0544**、「日期多形态」**+0.0258**，
//! 而 2026-11-07 实测发现：**召回瓶颈已从「召回」移到「排序」**
//! （`recall_gap_report`：NotInPool = **0 题**，42.6% 排在 k=10 之外）
//! ⇒ ⇒ ⇒ **排序侧的杠杆才是现在该攻的方向** ⇒ ⇒ ⇒ **查询侧规范化是同一族**。
//!
//! # 本例子测什么（每组数字都跑两套考卷）
//! - **A 基线**：当前生产（索引与查询侧都已去中文停用字）；
//! - **B 去疑问词**：查询侧去掉疑问词（「什么/为什么/怎么/谁/吗…」）；
//! - **C 日期多形态**：查询侧 + 索引侧把中文数字年份归一（「二〇二一年」→「2021年」）；
//! - **D B+C 叠加**：**故意测叠加** ⇒ ⇒ 已知 LoCoMo 上「叠加反而不如单独」（G +0.0027 < B +0.0544）。
//!
//! # 口径
//! 同一批题、同一个 k、同一套检索器；**只换文本**。
//! ⇒ ⇒ 两套考卷（内部词面卷 + 公平卷）各报一遍。

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{build_fair_exam, FairExamConfig};
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::{mined_to_golden, run_with_extra_sources, IndexRecord};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::novel_corpus;

const K: usize = 10;
const MAX_CASES_PER_RECORD: usize = 6;

/// 中文疑问词（**多字词级**删除 ⇒ ⇒ 与「去停用字」不同，保留词间结构）。
const CN_INTERROGATIVES: &[&str] = &[
    "为什么",
    "什么样",
    "怎么样",
    "怎么",
    "什么",
    "为什么",
    "哪里",
    "哪儿",
    "哪位",
    "哪个",
    "哪一些",
    "哪些",
    "哪个",
    "何时",
    "多少",
    "谁的",
    "谁",
    "吗",
    "呢",
    "吧",
    "请问",
    "如何",
    "为何",
    "何曾",
    "何况",
];

/// 去疑问词（词级）。
fn strip_interrogatives(text: &str) -> String {
    let mut out = text.to_string();
    for word in CN_INTERROGATIVES {
        if out.contains(word) {
            out = out.replace(word, "");
        }
    }
    out
}

/// 中文**单字**疑问词（与多字疑问词分开测 ⇒ ⇒ 看是哪一类在起作用）。
const CN_INTERROGATIVE_CHARS: &[char] = &['谁', '吗', '呢', '吧', '哪', '何', '啥', '嘛'];

/// 去单字疑问词。
fn strip_interrogative_chars(text: &str) -> String {
    text.chars()
        .filter(|c| !CN_INTERROGATIVE_CHARS.contains(c))
        .collect()
}

/// 中文数字年份 → 阿拉伯数字（「二〇二一年」→「2021年」）。
///
/// ⚠️ **只做绝对年份**（不需要基准日期）⇒ ⇒ 避开了 LoCoMo 那条「相对日期要写基准日」的构造成分。
fn normalize_dates(text: &str) -> String {
    const DIGITS: [&str; 10] = ["〇", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while i < chars.len() {
        // 「二〇二一年」= 4 个数字位 + 「年」
        if i + 5 <= chars.len() && chars[i + 4] == '年' {
            let mut year = String::new();
            let mut ok = true;
            for k in 0..4 {
                let c = chars[i + k];
                if c == '零' {
                    year.push('0');
                } else if let Some(pos) = DIGITS.iter().position(|d| d.chars().next() == Some(c)) {
                    year.push(char::from_digit(pos as u32, 10).unwrap_or('0'));
                } else {
                    ok = false;
                    break;
                }
            }
            if ok {
                out.push_str(&year);
                out.push('年');
                i += 5;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
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

fn remap(set: &GoldenSet, name: &str, transform: &dyn Fn(&str) -> String) -> GoldenSet {
    GoldenSet::new(
        name.to_string(),
        set.cases
            .iter()
            .map(|case| GoldenCase {
                id: case.id.clone(),
                query: transform(&case.query),
                expect_ids: case.expect_ids.clone(),
                expect_fragments: case.expect_fragments.clone(),
                tags: case.tags.clone(),
            })
            .collect(),
    )
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
        // **C 臂的索引侧**：日期归一
        let index_c: Vec<IndexRecord> = index
            .iter()
            .map(|record| IndexRecord {
                id: record.id.clone(),
                text: normalize_dates(&record.text),
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
            let base = recall(&index, set)?;

            // B：只去疑问词（查询侧）
            let set_b = remap(set, "b", &|q| strip_interrogatives(q));
            let b = recall(&index, &set_b)?;

            // C：只日期多形态（查询侧 + 索引侧）
            let set_c = remap(set, "c", &|q| normalize_dates(q));
            let c = recall(&index_c, &set_c)?;

            // D：B + C 叠加
            let set_d = remap(set, "d", &|q| normalize_dates(&strip_interrogatives(q)));
            let d = recall(&index_c, &set_d)?;

            // E：去单字疑问词（查询侧）
            let set_e = remap(set, "e", &|q| strip_interrogative_chars(q));
            let e = recall(&index, &set_e)?;

            // F：**索引侧 + 查询侧都去疑问词**（生产是同一个函数 ⇒ ⇒ 必须量这一臂）
            let index_f: Vec<IndexRecord> = index
                .iter()
                .map(|record| IndexRecord {
                    id: record.id.clone(),
                    text: strip_interrogatives(&record.text),
                })
                .collect();
            let set_f = remap(set, "f", &|q| strip_interrogatives(q));
            let f = recall(&index_f, &set_f)?;

            rows.push(serde_json::json!({
                "语料": name,
                "考卷": exam,
                "题数": set.cases.len(),
                "A基线": base,
                "B去疑问词": b,
                "B的Δ": b - base,
                "C日期多形态": c,
                "C的Δ": c - base,
                "D叠加": d,
                "D的Δ": d - base,
                "E去单字疑问词": e,
                "E的Δ": e - base,
                "F两侧都去": f,
                "F的Δ": f - base,
            }));
        }
    }
    for row in &rows {
        println!(
            "· {} / {} · 题 {} · A {:.4} · B(多字) {:.4} ({:+.4}) · C(日期) {:.4} ({:+.4}) · D(叠加) {:.4} ({:+.4}) · E(单字) {:.4} ({:+.4}) · F(两侧) {:.4} ({:+.4})",
            row["语料"].as_str().unwrap_or(""),
            row["考卷"].as_str().unwrap_or(""),
            row["题数"],
            row["A基线"].as_f64().unwrap_or(0.0),
            row["B去疑问词"].as_f64().unwrap_or(0.0),
            row["B的Δ"].as_f64().unwrap_or(0.0),
            row["C日期多形态"].as_f64().unwrap_or(0.0),
            row["C的Δ"].as_f64().unwrap_or(0.0),
            row["D叠加"].as_f64().unwrap_or(0.0),
            row["D的Δ"].as_f64().unwrap_or(0.0),
            row["E去单字疑问词"].as_f64().unwrap_or(0.0),
            row["E的Δ"].as_f64().unwrap_or(0.0),
            row["F两侧都去"].as_f64().unwrap_or(0.0),
            row["F的Δ"].as_f64().unwrap_or(0.0),
        );
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "查询侧杠杆A/B": rows })).unwrap_or_default()
    );
    Ok(())
}
