//! **候选池上限曲线**（2026-11-08）—— 池子砍到多少，recall@10 就开始掉？
//!
//! # 为什么要测这个
//! 生产管线每路取的候选数上限是 `HybridRetrievalPipeline::candidate_cap`（**默认 64**）。
//! 而从没有人量过：**这个 64 砍掉了多少召回？**
//!
//! 两个既有结论**看起来矛盾**，其实是因为**量的是两把尺子**：
//!
//! | 结论 | 出处 | 它实际量的是 |
//! |---|---|---|
//! | 「**`NotInPool = 0` 题**，瓶颈在排序层」 | `recall_gap_report` | **候选池 = 全部记录**，目标当然都在池里（**空洞统计**）|
//! | 「模板卷 **26%~27%** 的题答案不在**前 50** 候选里」 | `first_pos_mechanism` | **前 50 候选** |
//!
//!；后者说明：**池子上限本身就是一道召回天花板**，
//! 而任何「重排」都只能在这道天花板以下做功。
//!
//! # 本例测什么
//! 对每个池上限 `P`，量 `recall@10`（**闸门口径**：逐题「命中比例」再平均）：
//!
//! * `P = 10`：等于「不重排」的极限（拿不到第 11 名及以后）；
//! * `P = 64`：**生产现状**；
//! * `P` 继续变大：看**天花板在哪**；
//! * `P = 全库`：**理论极限**（池子不再是瓶颈）。
//!
//! 同时报 **oracle 可达率**（标准答案在池里就算）； 两者一减就是「排序层还能捞多少」。
//!
//! # 口径（与生产逐字一致）
//! * 检索 = 生产 BM25 源；**一次搜全量，再按前缀切片**
//!   （源内部按「分数降序 + id 升序」排好；前缀与直接 `candidates(query, P)` 等价）；
//! * 分数按池内 max 归一；**归一化基准与 P 无关**（全量 max），所以切片不改变排序；
//! * 只保留分数 > 0 的候选（与生产同口径）；
//! * k = 10。
//!
//! # 边界
//! * **不测 CPU 成本**（本例只回答「加池子能捞回多少召回」，不回答「值不值」）；
//! * 只在**三套考卷**上测，未覆盖其它语料。

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{build_fair_exam, FairExamConfig};
use apeireth_memory::golden_mining::{mine, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, IndexRecord};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

const K: usize = 10;
const MAX_CASES_PER_RECORD: usize = 6;
/// 生产默认候选上限（`HybridRetrievalPipeline::candidate_cap`）。
const PRODUCTION_CAP: usize = 64;
/// 要扫的池上限（`usize::MAX` 表示全库）。
const POOLS: [usize; 9] = [10, 16, 32, 50, 64, 100, 200, 500, usize::MAX];

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

fn exam_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("APEIRETH_MIRACL_EXAM") {
        return PathBuf::from(dir);
    }
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("miracl-exam"),
        None => {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            home.join(".apeireth").join("miracl-exam")
        }
    }
}

struct Exam {
    label: String,
    role: &'static str,
    cases: Vec<GoldenCase>,
    index: Vec<IndexRecord>,
}

#[derive(Default, Clone)]
struct Column {
    /// 闸门口径的分子（逐题命中比例之和）。
    frac_sum: f64,
    /// 二值「全部命中」题数。
    all_hits: usize,
    /// 池内可达题数（标准答案**全部**在池里）。
    reachable: usize,
    /// ⭐ **闸门口径下的可达上限**：逐题「可达金标数 / 金标总数」之和。
    ///
    /// **为什么必须有这个**：闸门的指标是 `|gold ∩ top-k| / |gold|` 的**平均**
    ///；一道 2 金标题若只有 1 条可达，这题的**上限就是 0.5**
    ///；用「全金标可达」（二值）当上限会**低估**可达性、**高估**余量。
    frac_oracle_sum: f64,
    cases: usize,
}

impl Column {
    fn frac(&self) -> f64 {
        self.frac_sum / self.cases.max(1) as f64
    }
    fn all(&self) -> f64 {
        self.all_hits as f64 / self.cases.max(1) as f64
    }
    fn reach(&self) -> f64 {
        self.reachable as f64 / self.cases.max(1) as f64
    }
    /// 闸门口径下的可达上限（逐题可达金标比例的平均）。
    fn frac_oracle(&self) -> f64 {
        self.frac_oracle_sum / self.cases.max(1) as f64
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
    let mut exams: Vec<Exam> = Vec::new();

    // 真人卷（MIRACL 官方标注）
    let root = exam_root();
    if (root.join("corpus.jsonl")).exists() && (root.join("exam.jsonl")).exists() {
        let mut index: Vec<IndexRecord> = Vec::new();
        for line in std::fs::read_to_string(root.join("corpus.jsonl"))?.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line)?;
            let id = value["id"].as_str().unwrap_or_default().to_string();
            if id.is_empty() {
                continue;
            }
            let joined = format!(
                "{}\n{}",
                value["title"].as_str().unwrap_or_default(),
                value["text"].as_str().unwrap_or_default()
            );
            index.push(IndexRecord { id, text: joined });
        }
        let mut cases: Vec<GoldenCase> = Vec::new();
        for line in std::fs::read_to_string(root.join("exam.jsonl"))?.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line)?;
            let query = value["query"].as_str().unwrap_or_default().to_string();
            let gold: Vec<String> = value["gold"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            if query.is_empty() || gold.is_empty() {
                continue;
            }
            cases.push(GoldenCase {
                id: value["id"].as_str().unwrap_or_default().to_string(),
                query,
                expect_ids: gold,
                expect_fragments: Vec::new(),
                tags: vec!["miracl".to_string()],
            });
        }
        exams.push(Exam {
            label: "真人卷 / MIRACL（官方人工标注）".to_string(),
            role: "真实形态 + 独立标签",
            cases,
            index,
        });
    } else {
        eprintln!("· 真人卷考卷不在，跳过（这不是通过）");
    }

    // 我们的两套卷
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
        let mined = mine(
            &records,
            MiningConfig {
                max_cases_per_record: MAX_CASES_PER_RECORD,
                ..MiningConfig::default()
            },
        );
        exams.push(Exam {
            label: format!("词面卷 / {name}（金标=主体本身）"),
            role: "产品形态",
            cases: GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined)).cases,
            index: index.clone(),
        });
        exams.push(Exam {
            label: format!("公平卷 / {name}（金标=顺带提及）"),
            role: "诊断用（非产品形态）",
            cases: build_fair_exam(
                &records,
                FairExamConfig {
                    max_cases_per_record: MAX_CASES_PER_RECORD,
                    ..FairExamConfig::default()
                },
            ),
            index,
        });
    }

    if exams.is_empty() {
        eprintln!("· 没有可测的考卷");
        return Ok(());
    }

    let mut summary: Vec<serde_json::Value> = Vec::new();
    for exam in &exams {
        let columns = measure(exam)?;
        println!(
            "=== {} · {} · 语料 {} 段 · 题 {} ===",
            exam.label,
            exam.role,
            exam.index.len(),
            columns[0].cases
        );
        println!("     池上限     闸门口径recall   全部命中     全可达率    **闸门口径可达上限**");
        for (index, pool) in POOLS.iter().enumerate() {
            let label = if *pool == usize::MAX {
                "全库".to_string()
            } else {
                pool.to_string()
            };
            let marker = if *pool == PRODUCTION_CAP || *pool == usize::MAX {
                "  ←"
            } else {
                ""
            };
            println!(
                "     {:<10} {:.4}          {:.4}       {:.4}      {:.4}{}",
                label,
                columns[index].frac(),
                columns[index].all(),
                columns[index].reach(),
                columns[index].frac_oracle(),
                if *pool == PRODUCTION_CAP {
                    "  ← 生产默认 64"
                } else {
                    marker
                },
            );
        }
        // 关键读数：从生产上限加到全库，还能捞回多少
        let production = columns
            .iter()
            .zip(POOLS.iter())
            .find(|(_, pool)| **pool == PRODUCTION_CAP)
            .map(|(column, _)| column.clone())
            .unwrap_or_default();
        let full = columns.last().cloned().unwrap_or_default();
        println!(
            "     上限从 {} 放到全库：闸门口径 {:+.4} · 全部命中 {:+.4} · 池内可达率 {:+.4}",
            PRODUCTION_CAP,
            full.frac() - production.frac(),
            full.all() - production.all(),
            full.reach() - production.reach(),
        );
        // ⭐ 排序层余量（**必须用闸门口径的上限**才算得对）
        println!(
            "     全库池下的排序余量（闸门口径）：上限 {:.4} − recall {:.4} = {:+.4}",
            full.frac_oracle(),
            full.frac(),
            full.frac_oracle() - full.frac()
        );
        println!(
            "     其中「不可能被检索到的金标」占 {:.2}%（上限 1 − {:.4}）",
            (1.0 - full.frac_oracle()) * 100.0,
            full.frac_oracle()
        );

        summary.push(serde_json::json!({
            "考卷": exam.label,
            "角色": exam.role,
            "语料段数": exam.index.len(),
            "题数": columns[0].cases,
            "各池上限": POOLS.iter().zip(columns.iter()).map(|(pool, column)| serde_json::json!({
                "池上限": if *pool == usize::MAX { "全库".to_string() } else { pool.to_string() },
                "闸门口径recall": column.frac(),
                "全部命中": column.all(),
                "池内可达率": column.reach(),
                "闸门口径可达上限": column.frac_oracle(),
            })).collect::<Vec<_>>(),
            "闸门口径可达上限": full.frac_oracle(),
            "不可能达到的金标占比": 1.0 - full.frac_oracle(),
            "生产上限到全库的增量": {
                "闸门口径": full.frac() - production.frac(),
                "全部命中": full.all() - production.all(),
                "池内可达率": full.reach() - production.reach(),
            },
        }));
    }

    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "候选池曲线": summary }))?
    );
    Ok(())
}

fn measure(exam: &Exam) -> Result<Vec<Column>, Box<dyn Error>> {
    let mut columns = vec![Column::default(); POOLS.len()];
    let bm25 = Bm25LexicalCandidateSource::new(
        exam.index
            .iter()
            .map(|record| MemoryCandidate {
                id: record.id.clone(),
                layer: "pool".to_string(),
                scope: MemoryScope::Global,
                content: record.text.clone(),
                score: 0.0,
                score_components: Default::default(),
                provenance: Default::default(),
            })
            .collect(),
    );
    let _ = HashMap::<String, String>::new();
    for case in &exam.cases {
        // 一次搜全量，再按前缀切片（源内部已按「分数降序 + id 升序」排好）
        let all = bm25.candidates(&case.query, exam.index.len().max(1))?;
        let ranked: Vec<&str> = all
            .iter()
            .filter(|hit| hit.score > 0.0)
            .map(|hit| hit.id.as_str())
            .collect();
        if ranked.is_empty() {
            continue;
        }
        let gold: std::collections::HashSet<&str> =
            case.expect_ids.iter().map(String::as_str).collect();
        for (index, pool) in POOLS.iter().enumerate() {
            let limit = if *pool == usize::MAX {
                ranked.len()
            } else {
                (*pool).min(ranked.len())
            };
            let ids: Vec<&str> = ranked.iter().take(limit).copied().collect();
            let column = &mut columns[index];
            column.cases += 1;
            // 池内可达（标准答案全在池里）
            if gold.iter().all(|id| ids.contains(id)) {
                column.reachable += 1;
            }
            // 闸门口径下的可达上限（逐题「可达金标数 / 金标总数」）
            column.frac_oracle_sum += fraction(&gold, &ids);
            // 前 k（k 与池上限取小）
            let top: Vec<&str> = ids.iter().take(K).copied().collect();
            column.frac_sum += fraction(&gold, &top);
            if gold.iter().all(|id| top.contains(id)) {
                column.all_hits += 1;
            }
        }
    }
    Ok(columns)
}

fn fraction(gold: &std::collections::HashSet<&str>, ids: &[&str]) -> f64 {
    if gold.is_empty() {
        return 0.0;
    }
    gold.iter().filter(|id| ids.contains(*id)).count() as f64 / gold.len() as f64
}
