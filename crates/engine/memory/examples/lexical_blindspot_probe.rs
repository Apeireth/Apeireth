//! **词法盲区诊断**（2026-11-08）—— 那 18.6% 的题**为什么一个候选都命中不了**？
//!
//! # 背景
//! `memory-recall-decomposition-2026-11-08.md` 实测：
//!
//! | 考卷 | 全库可达率 | 词法永久盲区 |
//! |---|---|---|
//! | 真人卷 MIRACL | 0.9644 | 3.6% |
//! | 词面卷 redchamber | 0.9391 | 6.1% |
//! | **词面卷 watershed** | **0.8145** | **18.6%** |
//!
//!；**watershed 近五分之一的题，标准答案在 BM25 下分数为 0**
//!；这些题**再强的重排也救不了**；必须先查清**为什么**。
//!
//! # 本例把「盲区题」逐条归类
//!
//! | 类别 | 含义 | 该怎么办 |
//! |---|---|---|
//! | `空题面` | 规范化后**一个词都不剩**（全是停用字/疑问词）| 题面构造问题 |
//! | `词在文里但索引漏了` | 题面词**确实是正文的子串**，但 BM25 仍为 0 | **分词/索引口径有 bug** |
//! | `词真的不在文里` | 题面词**不在**标准答案正文里 | **题面与标签不匹配**（挖题器的锅）|
//! | `只有单字命中` | 命中的全是单字（bigram 不在）| 分词粒度问题 |
//! | `其它` | 兜底 | 逐条看 |
//!
//! # 口径
//! * 查询词 = **生产口径** `tokenize(normalize_for_index(query))`；
//! * 「盲区题」= 该题**所有** `expect_ids` 在 BM25 全库检索下分数都为 0；
//! * 「在文里」= 题面词是标准答案正文的**子串**（`str::contains`）；
//! * 只诊断**词面卷**（产品形态），另外两套卷作为对照。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_mining::{mine, question_subject, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, IndexRecord};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::hybrid_search::{normalize_for_index, tokenize};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

const MAX_CASES_PER_RECORD: usize = 6;
const SAMPLES_PER_CLASS: usize = 5;

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

/// 一条盲区题的归类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Cause {
    /// 规范化后一个词都不剩。
    EmptyQuery,
    /// 题面词是正文子串，但 BM25 为 0（**索引口径问题**）。
    InTextButNotIndexed,
    /// 题面词真的不在正文里（**题面与标签不匹配**）。
    AbsentFromText,
    /// 只有单字词能对上（bigram 对不上）。
    OnlySingleChars,
}

impl Cause {
    fn label(self) -> &'static str {
        match self {
            Cause::EmptyQuery => "空题面",
            Cause::InTextButNotIndexed => "词在文里但索引漏了",
            Cause::AbsentFromText => "词真的不在文里",
            Cause::OnlySingleChars => "只有单字命中",
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("· 失败：{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut rows: Vec<serde_json::Value> = Vec::new();

    for name in ["watershed", "redchamber"] {
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
        let text_by_id: HashMap<&str, &str> = index
            .iter()
            .map(|record| (record.id.as_str(), record.text.as_str()))
            .collect();
        let mined = mine(
            &records,
            MiningConfig {
                max_cases_per_record: MAX_CASES_PER_RECORD,
                ..MiningConfig::default()
            },
        );
        let set = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));
        let bm25 = Bm25LexicalCandidateSource::new(
            index
                .iter()
                .map(|record| MemoryCandidate {
                    id: record.id.clone(),
                    layer: "probe".to_string(),
                    scope: MemoryScope::Global,
                    content: record.text.clone(),
                    score: 0.0,
                    score_components: Default::default(),
                    provenance: Default::default(),
                })
                .collect(),
        );

        // ⚠️ **口径关键**：词面卷的 `conflict` / `temporal` / `multi_record` 是**多金标题**
        //；「全金标在池里」与「题目能不能被检索到」**不是一回事**
        //；必须**按 (题, 金标) 对**统计，不能按题统计。
        let mut gold_total = 0usize;
        let mut gold_unreachable = 0usize;
        let mut cases_all_unreachable = 0usize;
        let mut cases_some_unreachable = 0usize;
        let mut causes: HashMap<Cause, usize> = HashMap::new();
        let mut samples: HashMap<Cause, Vec<String>> = HashMap::new();
        for case in &set.cases {
            let hits = bm25.candidates(&case.query, index.len().max(1))?;
            let score_by_id: HashMap<&str, f64> = hits
                .iter()
                .map(|hit| (hit.id.as_str(), hit.score))
                .collect();
            let mut unreachable_here = 0usize;
            for id in &case.expect_ids {
                gold_total += 1;
                let score = score_by_id.get(id.as_str()).copied().unwrap_or(0.0);
                if score > 0.0 {
                    continue;
                }
                gold_unreachable += 1;
                unreachable_here += 1;
                let terms = tokenize(&normalize_for_index(&case.query));
                let cause = classify_one(&terms, text_by_id.get(id.as_str()).copied());
                *causes.entry(cause).or_default() += 1;
                let bucket = samples.entry(cause).or_default();
                if bucket.len() < SAMPLES_PER_CLASS {
                    let gold = text_by_id
                        .get(id.as_str())
                        .map(|text| text.chars().take(50).collect::<String>())
                        .unwrap_or_default();
                    bucket.push(format!(
                        "题面「{}」 · 词 {:?} · 该条金标开头「{}」",
                        case.query, terms, gold
                    ));
                }
            }
            if unreachable_here == case.expect_ids.len() {
                cases_all_unreachable += 1;
            } else if unreachable_here > 0 {
                cases_some_unreachable += 1;
            }
        }

        let total = set.cases.len().max(1);
        println!("=== 词面卷 / {name} · 题 {total} · 金标 {gold_total} 条 ===");
        println!(
            "     金标级盲区 {gold_unreachable} 条（{:.1}%）",
            gold_unreachable as f64 / gold_total.max(1) as f64 * 100.0
        );
        println!(
            "     题级：全部金标都盲 {cases_all_unreachable} 题（{:.1}%）· 部分金标盲 {cases_some_unreachable} 题（{:.1}%）",
            cases_all_unreachable as f64 / total as f64 * 100.0,
            cases_some_unreachable as f64 / total as f64 * 100.0
        );
        let mut ordered: Vec<(Cause, usize)> = causes.iter().map(|(k, v)| (*k, *v)).collect();
        ordered.sort_by(|a, b| b.1.cmp(&a.1));
        for (cause, count) in &ordered {
            println!(
                "     {:<20} {:>4} 条（占盲区金标 {:.1}%）",
                cause.label(),
                count,
                *count as f64 / gold_unreachable.max(1) as f64 * 100.0
            );
        }
        for (cause, count) in &ordered {
            if let Some(items) = samples.get(cause) {
                println!("   -- {}（{count} 条）样例如下 --", cause.label());
                for item in items {
                    println!("      · {item}");
                }
            }
        }
        rows.push(serde_json::json!({
            "语料": name,
            "题数": set.cases.len(),
            "金标条数": gold_total,
            "金标级盲区": gold_unreachable,
            "金标级盲区占比": gold_unreachable as f64 / gold_total.max(1) as f64,
            "全部金标都盲的题": cases_all_unreachable,
            "部分金标盲的题": cases_some_unreachable,
            "归类": ordered.iter().map(|(cause, count)| serde_json::json!({
                "原因": cause.label(),
                "题数": count,
            })).collect::<Vec<_>>(),
        }));
    }

    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "词法盲区诊断": rows }))?
    );
    Ok(())
}

/// 给**一条盲区金标**归类（不按题）。
fn classify_one(terms: &[String], gold_text: Option<&str>) -> Cause {
    if terms.is_empty() {
        return Cause::EmptyQuery;
    }
    let Some(gold_text) = gold_text else {
        return Cause::AbsentFromText;
    };
    let hit: Vec<&String> = terms
        .iter()
        .filter(|term| gold_text.contains(term.as_str()))
        .collect();
    if hit.is_empty() {
        return Cause::AbsentFromText;
    }
    if hit.iter().all(|term| term.chars().count() == 1) {
        return Cause::OnlySingleChars;
    }
    // 有 ≥2 字的词是子串，但 BM25 仍为 0
    Cause::InTextButNotIndexed
}

/// 保留给后续诊断：题面主体词（当前未用于归类）。
#[allow(dead_code)]
fn subject_of(
    case: &GoldenCase,
    records: &[apeireth_memory::golden_mining::MineRecord],
) -> Option<String> {
    records
        .iter()
        .find(|record| case.expect_ids.contains(&record.id))
        .and_then(question_subject)
}

/// 未使用但保留以表明 HashSet 的用途（避免未用导入告警）。
#[allow(dead_code)]
fn dedupe(terms: &[String]) -> HashSet<String> {
    terms.iter().cloned().collect()
}
