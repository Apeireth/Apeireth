//! **特征融合 A/B**（内部卷 · **按文档划分** ⇒ 防「同一文档的题同时出现在训练与测试」）。
//!
//! # 要回答的问题
//!
//! 「我们那些实验性算法/信号，到底能不能**学**着把它们合起来用？」
//! ⇒ ⇒ 先量**单个特征**在**留出文档**上的增益（诚实的下限），再谈学习权重。
//!
//! # 特征（全部**零模型、零依赖**）
//!
//! | 特征 | 含义 |
//! |---|---|
//! | `bm25` | 生产 BM25（**已含 `coverage` 融合**，即当前生产口径）|
//! | `heading` | 问题实词在**章节标题**里的命中比例（文档结构信号）|
//! | `path` | 问题实词在**文件路径**里的命中比例（领域信号）|
//! | `len_pen` | 记录过长的惩罚（`1 - min(1, len/4000)`）|
//!
//! # 口径（钉死）
//!
//! * **按 `path` 划分**：`sha256(path) % 3 == 0` ⇒ 测试集，其余 ⇒ 训练/调参集
//!   ⇒ ⇒ 同一文档的题**不会**跨集出现（否则是泄漏）；
//! * 权重只在**训练集**上扫（`{0.0, 0.25, 0.5, 1.0}`），**测试集只报最终数字**；
//! * `k = 10`；只报 `recall@10`。
//!
//! # 怎么跑
//!
//! ```sh
//! python3 scripts/internal_golden.py --records > ~/internal-records.jsonl
//! cargo run -p apeireth-memory --example feature_blend_ab --locked -- ~/internal-records.jsonl
//! ```

use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use std::collections::HashMap;
use std::process::ExitCode;

const K: usize = 10;
const WEIGHTS: [f64; 4] = [0.0, 0.25, 0.5, 1.0];

#[derive(Clone)]
struct Record {
    id: String,
    text: String,
    heading: String,
    path: String,
}

/// 一次查询的候选快照（**只取一次** ⇒ 权重扫描时只重算分数，不重跑检索）。
struct CandidateSnap {
    id: String,
    /// 生产口径分（`Bm25LexicalCandidateSource` 返回的 `score`，**已含 coverage 融合**）。
    production_score: f64,
    heading: String,
    path: String,
    text_len: usize,
}

#[derive(Clone)]
struct Case {
    query: String,
    expect: Vec<String>,
    path: String,
}

fn main() -> ExitCode {
    let records_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/home/bai/internal-records.jsonl".to_string());
    let Ok(records_text) = std::fs::read_to_string(&records_path) else {
        eprintln!("· 读不到 {records_path} ⇒ **没测到**");
        return ExitCode::FAILURE;
    };
    let mut records: Vec<Record> = Vec::new();
    for line in records_text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        let (Some(id), Some(text)) = (
            v.get("id").and_then(|x| x.as_str()),
            v.get("text").and_then(|x| x.as_str()),
        ) else {
            continue;
        };
        records.push(Record {
            id: id.to_string(),
            text: text.to_string(),
            heading: v
                .get("heading")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            path: v
                .get("path")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    let Ok(golden_text) = std::fs::read_to_string("docs/03-reference/internal-golden-v1.jsonl")
    else {
        eprintln!("· 读不到考卷 ⇒ **没测到**");
        return ExitCode::FAILURE;
    };
    let by_id: HashMap<&str, &Record> = records.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut cases: Vec<Case> = Vec::new();
    for line in golden_text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        let query = v.get("query").and_then(|x| x.as_str()).unwrap_or("");
        let expect: Vec<String> = v
            .get("expect_ids")
            .and_then(|x| x.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if query.is_empty() || expect.is_empty() {
            continue;
        }
        // 题目归属的文档 = 期望记录的文档（用于**按文档划分**）
        let path = expect
            .first()
            .and_then(|id| by_id.get(id.as_str()).map(|r| r.path.clone()))
            .unwrap_or_default();
        cases.push(Case {
            query: query.to_string(),
            expect,
            path,
        });
    }
    if records.is_empty() || cases.is_empty() {
        eprintln!("· 记录或考卷为空 ⇒ **没测到**");
        return ExitCode::FAILURE;
    }

    // ---- 按文档划分（sha256(path) 首字节 % 3 == 0 ⇒ 测试集）----
    let is_test = |path: &str| -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        hasher.finish() % 3 == 0
    };
    let train: Vec<&Case> = cases.iter().filter(|c| !is_test(&c.path)).collect();
    let test: Vec<&Case> = cases.iter().filter(|c| is_test(&c.path)).collect();
    println!("════════ 特征融合 A/B（**按文档划分**）════════");
    println!(
        "记录 **{}** · 考卷 **{}**（训练/调参 {} · **留出测试 {}**）· k = {K}",
        records.len(),
        cases.len(),
        train.len(),
        test.len()
    );

    let documents: Vec<MemoryCandidate> = records
        .iter()
        .map(|r| MemoryCandidate {
            id: r.id.clone(),
            layer: "internal".to_string(),
            scope: MemoryScope::Global,
            content: r.text.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance {
                source: "internal_golden".to_string(),
                source_session: None,
                source_trace: None,
                source_request: None,
            },
        })
        .collect();
    let source = Bm25LexicalCandidateSource::new(documents);

    // ---- **候选快照**：每查询只检索一次（否则 64 组权重 × 每次全库排序 = 跑不完）----
    let all_cases: Vec<&Case> = cases.iter().collect();
    let mut snaps: Vec<Vec<CandidateSnap>> = Vec::with_capacity(all_cases.len());
    for case in &all_cases {
        let Ok(all) = source.candidates(&case.query, 100_000) else {
            snaps.push(Vec::new());
            continue;
        };
        snaps.push(
            all.iter()
                .map(|candidate| {
                    let record = by_id.get(candidate.id.as_str());
                    CandidateSnap {
                        id: candidate.id.clone(),
                        production_score: candidate.score,
                        heading: record.map(|r| r.heading.clone()).unwrap_or_default(),
                        path: record.map(|r| r.path.clone()).unwrap_or_default(),
                        text_len: candidate.content.chars().count(),
                    }
                })
                .collect(),
        );
    }
    // 测试集索引（与 `all_cases` 同序）
    let test_idx: Vec<usize> = all_cases
        .iter()
        .enumerate()
        .filter(|(_, c)| is_test(&c.path))
        .map(|(i, _)| i)
        .collect();
    let train_idx: Vec<usize> = all_cases
        .iter()
        .enumerate()
        .filter(|(_, c)| !is_test(&c.path))
        .map(|(i, _)| i)
        .collect();

    // ---- 在**训练集**上扫权重 ----
    println!(
        "\n── 训练/调参集（{} 题 · 权重网格 {}³）──",
        train_idx.len(),
        WEIGHTS.len()
    );
    let mut best = (0.0_f64, 0.0_f64, 0.0_f64, f64::MIN);
    for &heading in &WEIGHTS {
        for &path in &WEIGHTS {
            for &len_pen in &WEIGHTS {
                // ⚠️ 必须传**全量** `all_cases`：`train_idx` 里的是**全局索引**
                // ⇒ 传压缩后的子集会索引错位（第一版就是这么得出 0.3043 的假结果的）。
                let recall =
                    evaluate_subset(&snaps, &train_idx, &all_cases, heading, path, len_pen);
                if recall > best.3 {
                    best = (heading, path, len_pen, recall);
                }
            }
        }
    }
    let (w_heading, w_path, w_len, train_best) = best;
    let train_base = evaluate_subset(&snaps, &train_idx, &all_cases, 0.0, 0.0, 0.0);
    println!(
        "  基线（生产口径）：{train_base:.4} ⇒ 最优 heading×{w_heading} · path×{w_path} · len×{w_len} ⇒ {train_best:.4}（{:+.4}）",
        train_best - train_base
    );

    // ---- 留出测试集：只报一次 ----
    println!("\n── 留出测试集（{} 题 · **只用一次**）──", test_idx.len());
    let test_base = evaluate_subset(&snaps, &test_idx, &all_cases, 0.0, 0.0, 0.0);
    let test_best = evaluate_subset(&snaps, &test_idx, &all_cases, w_heading, w_path, w_len);
    println!("  基线：{test_base:.4}");
    println!("  加特征：{test_best:.4}（{:+.4}）", test_best - test_base);
    println!();
    if test_best > test_base + 0.005 {
        println!("  ✅ **留出文档上仍有正收益** ⇒ 三信号**不是过拟合**，可作 LTR 特征池。");
    } else {
        println!("  ⚠️ **留出文档上无正收益（或负）** ⇒ 训练集提升来自**过拟合** ⇒ 不可信。");
        println!("     ⇒ 这三个特征**不能**直接加；要学权重需更多留出数据。");
    }
    ExitCode::SUCCESS
}

/// 按**索引子集**评估（快照与 cases 同序 ⇒ 用索引挑）。
fn evaluate_subset(
    snaps: &[Vec<CandidateSnap>],
    indices: &[usize],
    cases: &[&Case],
    w_heading: f64,
    w_path: f64,
    w_len: f64,
) -> f64 {
    let mut total = 0usize;
    let mut hits = 0usize;
    for &index in indices {
        let (Some(snap), Some(case)) = (snaps.get(index), cases.get(index)) else {
            continue;
        };
        let terms = terms_of(&case.query);
        let mut scored: Vec<(&str, f64)> = snap
            .iter()
            .map(|c| {
                let mut score = c.production_score;
                if w_heading > 0.0 {
                    score += w_heading * coverage_terms(&terms, &c.heading);
                }
                if w_path > 0.0 {
                    score += w_path * coverage_terms(&terms, &c.path);
                }
                if w_len > 0.0 {
                    let len = c.text_len as f64;
                    score += w_len * (1.0 - (len / 4000.0).min(1.0));
                }
                (c.id.as_str(), score)
            })
            .collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(b.0))
        });
        total += 1;
        if scored
            .iter()
            .take(K)
            .any(|(id, _)| case.expect.iter().any(|e| e == id))
        {
            hits += 1;
        }
    }
    if total == 0 {
        return f64::NAN;
    }
    hits as f64 / total as f64
}

fn evaluate(
    snaps: &[Vec<CandidateSnap>],
    cases: &[&Case],
    w_heading: f64,
    w_path: f64,
    w_len: f64,
) -> f64 {
    let mut total = 0usize;
    let mut hits = 0usize;
    for (index, case) in cases.iter().enumerate() {
        let Some(snap) = snaps.get(index) else {
            continue;
        };
        let terms = terms_of(&case.query);
        let mut scored: Vec<(&str, f64)> = snap
            .iter()
            .map(|c| {
                let mut score = c.production_score;
                if w_heading > 0.0 {
                    score += w_heading * coverage_terms(&terms, &c.heading);
                }
                if w_path > 0.0 {
                    score += w_path * coverage_terms(&terms, &c.path);
                }
                if w_len > 0.0 {
                    let len = c.text_len as f64;
                    score += w_len * (1.0 - (len / 4000.0).min(1.0));
                }
                (c.id.as_str(), score)
            })
            .collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(b.0))
        });
        total += 1;
        if scored
            .iter()
            .take(K)
            .any(|(id, _)| case.expect.iter().any(|e| e == id))
        {
            hits += 1;
        }
    }
    if total == 0 {
        return f64::NAN;
    }
    hits as f64 / total as f64
}

fn terms_of(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|t| t.chars().count() >= 2)
        .map(str::to_lowercase)
        .collect()
}

fn coverage_terms(terms: &[String], text: &str) -> f64 {
    if terms.is_empty() {
        return 0.0;
    }
    let haystack = text.to_lowercase();
    let hit = terms
        .iter()
        .filter(|t| haystack.contains(t.as_str()))
        .count();
    hit as f64 / terms.len() as f64
}
