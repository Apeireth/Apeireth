//! **两批查询的形态分布诊断**（2026-11-07）—— 先确认「能不能分开」，再谈判据。
//!
//! # 背景
//! `rerank_headroom_ab` 实测：`first_pos` 在**词面卷 −0.008/−0.024**、
//! 在**公平卷 +0.048/+0.032** ⇒ ⇒ 需要「查询形态准入」。
//! 但**两个判据都失败了**：
//! 1. 查询词在语料里的覆盖率 ⇒ 中文 bigram 几乎全在语料里 ⇒ 两边都判成重叠；
//! 2. 原始查询是否含疑问词 ⇒ 判据表含「吗/呢/谁」这类小说里到处都是的单字 ⇒ 全命中。
//!
//! # 这个例子做什么
//! **不跑检索**，只统计两批查询自身的分布 ⇒ ⇒ 回答「有任何一个统计量能分开吗」。
//!
//! | 统计量 | 为什么可能有效 |
//! |---|---|
//! | 字符数 / 词数 | 自然语言问句更长 |
//! | 是否以「？」结尾 | 问句特征 |
//! | **只在句尾**的疑问词 | 排除小说对话里的「吗/呢」 |
//! | 多字疑问词（什么/怎么/为什么/哪里/如何）| 不会被单字污染 |
//! | 虚词占比（的/了/是/在）| 自然语言更口语化 |
//!
//! # 输出
//! 一行 JSON（每个统计量：两批的 均值/中位/分位）+ 各 3 条样例。

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{build_fair_exam, FairExamConfig};
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::mined_to_golden;
use apeireth_memory::golden_set::GoldenSet;
use apeireth_memory::novel_corpus;

const MAX_CASES_PER_RECORD: usize = 6;

/// **多字**疑问词（**不含**吗/呢/谁/吧这类单字 ⇒ 那些在小说里到处都是）。
const MULTI_INTERROGATIVES: &[&str] = &[
    "为什么",
    "什么样",
    "怎么样",
    "怎么",
    "什么",
    "哪里",
    "哪儿",
    "哪个",
    "哪些",
    "何时",
    "多少",
    "如何",
    "为何",
];

/// 虚词（自然语言更常出现）。
const FUNCTION_WORDS: &[char] = &['的', '了', '是', '在', '和', '也', '就', '不'];

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

/// 一维分布统计。
#[derive(Default)]
struct Dist {
    values: Vec<f64>,
}

impl Dist {
    fn push(&mut self, value: f64) {
        self.values.push(value);
    }
    fn summary(&mut self) -> serde_json::Value {
        self.values
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = self.values.len();
        let at = |q: f64| -> f64 {
            if n == 0 {
                0.0
            } else {
                self.values[((q * (n - 1) as f64).round() as usize).min(n - 1)]
            }
        };
        let mean = if n == 0 {
            0.0
        } else {
            self.values.iter().sum::<f64>() / n as f64
        };
        serde_json::json!({
            "n": n,
            "均值": mean,
            "p10": at(0.10),
            "p50": at(0.50),
            "p90": at(0.90),
        })
    }
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
            continue;
        }
        let Ok(chapters) = novel_corpus::load_chapters(&path) else {
            continue;
        };
        let records = novel_corpus::to_mine_records(&chapters);
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
            let mut chars = Dist::default();
            let mut terms = Dist::default();
            let mut multi = Dist::default();
            let mut tail = Dist::default();
            let mut question_mark = Dist::default();
            let mut function_ratio = Dist::default();
            for case in &set.cases {
                chars.push(case.query.chars().count() as f64);
                terms.push(
                    case.query
                        .split(|c: char| !c.is_alphanumeric())
                        .filter(|t| !t.is_empty())
                        .count() as f64,
                );
                multi.push(
                    MULTI_INTERROGATIVES
                        .iter()
                        .filter(|word| case.query.contains(*word))
                        .count() as f64,
                );
                // 疑问词**只在句尾**（最后 4 个字符内）
                let tail_text: String = case
                    .query
                    .chars()
                    .rev()
                    .take(4)
                    .collect::<Vec<char>>()
                    .into_iter()
                    .rev()
                    .collect();
                tail.push(
                    if MULTI_INTERROGATIVES.iter().any(|w| tail_text.contains(w)) {
                        1.0
                    } else {
                        0.0
                    },
                );
                question_mark.push(if case.query.contains('？') || case.query.contains('?') {
                    1.0
                } else {
                    0.0
                });
                let total = case.query.chars().count().max(1) as f64;
                let functions = case
                    .query
                    .chars()
                    .filter(|c| FUNCTION_WORDS.contains(c))
                    .count() as f64;
                function_ratio.push(functions / total);
            }
            let samples: Vec<&str> = set
                .cases
                .iter()
                .take(4)
                .map(|case| case.query.as_str())
                .collect();
            rows.push(serde_json::json!({
                "语料": name,
                "考卷": exam,
                "题数": set.cases.len(),
                "字符数": chars.summary(),
                "词数": terms.summary(),
                "多字疑问词数": multi.summary(),
                "疑问词在句尾": tail.summary(),
                "含问号": question_mark.summary(),
                "虚词占比": function_ratio.summary(),
                "样例": samples,
            }));
        }
    }
    for row in &rows {
        println!(
            "· {} / {} · 题 {}",
            row["语料"].as_str().unwrap_or(""),
            row["考卷"].as_str().unwrap_or(""),
            row["题数"]
        );
        for key in [
            "字符数",
            "词数",
            "多字疑问词数",
            "疑问词在句尾",
            "含问号",
            "虚词占比",
        ] {
            let s = &row[key];
            println!(
                "     {:<8} 均值 {:.3} · p10 {:.3} · p50 {:.3} · p90 {:.3}",
                key,
                s["均值"].as_f64().unwrap_or(0.0),
                s["p10"].as_f64().unwrap_or(0.0),
                s["p50"].as_f64().unwrap_or(0.0),
                s["p90"].as_f64().unwrap_or(0.0),
            );
        }
        for sample in row["样例"].as_array().unwrap_or(&vec![]) {
            println!("     样例: {}", sample.as_str().unwrap_or(""));
        }
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "查询形态分布": rows })).unwrap_or_default()
    );
    Ok(())
}
