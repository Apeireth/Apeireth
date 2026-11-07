//! 检索「相关性-多样性折中」重排的 **λ 敏感性**离线测量（**内部卷 + 真实查询卷 + 去冗余收益**）。
//!
//! # 为什么需要这个测例
//!
//! 生产混合检索管线在融合排序之后有一道**残差式多样性重排**（贪心选择：
//! `(1-λ)·相关度 + λ·新颖度`，新颖度 = 1 − 与已选项的最大**词面**相似度），
//! 旋钮是 `MemoryRankingConfig::diversity_lambda`（**默认 0.0 = 关闭**）。
//!
//! 但**既有两道闸门都量不到它**：
//! * `scripts/check_golden_baseline.py` 走**单路 BM25**（`golden_run::run`），不经过管线；
//! * `scripts/check_fusion_channels.py` 量**通道贡献**（多路并联 delta），不单独量重排。
//!
//! ⇒ 本测例用**同一份内部卷、同一批题、同一个生产管线**，只改 λ，把「重排把分数
//! 推高还是压低、λ 取多少合理」变成可复现数字。
//!
//! # 本测例回答**两个**问题（不是只报一半）
//!
//! 1. **相关度侧**：λ 变大 ⇒ recall@10 / nDCG@10 怎么变？
//! 2. **去冗余侧**：λ 变大 ⇒ 结果里的**近似重复对**少了多少？
//!    （只看第 1 项会把「多样性」误读成「纯损失」；只看第 2 项会把「去重」误读成「纯收益」。）
//!
//! # 两把尺子都要报（**结论相反也必须照报**）
//!
//! * **内部卷**（模板题）：`$APEIRETH_DATA_DIR/novels/` 两本；
//! * **真实查询卷**（真人问句 + 官方标注）：`$APEIRETH_MIRACL_EXAM/`（`corpus.jsonl` + `exam.jsonl`）。
//!
//! ⚠️ 已知事实：**同一个特性在两套卷上结论可能相反**（例：首命中位置在真人卷涨、模板卷跌）
//! ⇒ ⇒ 本测例**不允许**只报一套卷。
//!
//! # 口径
//!
//! * 内部卷挖题 = 与基线同口径（`MiningConfig::default` + 每记录 6 题）；`CASE_SAMPLE` 仅**截前 N 题**；
//! * 检索 = 生产管线 `HybridRetrievalPipeline::retrieve`（单路 BM25 源、`Global` 可见域、
//!   `k = 10`、预算足够大 ⇒ 只量排序不量注入）；
//! * 指标 = `recall@10` / `nDCG@10`（内部卷用 `golden_set::evaluate` 同口径；
//!   真人卷按「top-10 命中任一金标」计）；
//! * **近似重复对** = 同一结果内两两**词面** Jaccard ≥ `DUP_THRESHOLD` 的对数
//!   （本地同口径副本：分词 = `unicode_tokens`，与管线内部同一套）。
//!
//! # 中性表述
//!
//! 只描述**机制**（相关性-多样性折中 / 贪心重排 / 词面相似度），不引用任何外部产品名、
//! 文档或代码。
//!
//! # 输出
//!
//! stdout 一行 JSON；stderr 人读摘要 + 语料缺失如实说明（跳过 ≠ 跑过）。

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::mined_to_golden;
use apeireth_memory::golden_set::{evaluate, EvalBatch, GoldenSet, Retrieved};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use apeireth_memory::{unicode_tokens, MemoryRankingConfig};
use serde_json::json;

/// 与基线一致的 top-k。
const K: usize = 10;
/// 与基线一致的挖题预算（《紅樓夢》476 题 / 《水滸傳》248 题）。
const MAX_CASES_PER_RECORD: usize = 6;
/// 每题最多取前 N 题做扫描（`0` = 全量）。同一个子集横向比 λ 仍然可比。
const CASE_SAMPLE: usize = 96;
/// 字符预算：本测例只量排序，给足够预算 ⇒ 不因预算截断丢条目。
const MAX_CHARS: usize = 4_000_000;

const LAMBDAS: [f64; 5] = [0.0, 0.2, 0.5, 0.8, 1.0];
/// 真人卷只扫小档位（λ 越大越容易把相关项挤掉 ⇒ 大档位在真人卷上意义不大）。
const REAL_QUERY_LAMBDAS: [f64; 3] = [0.0, 0.1, 0.2];
const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];
/// **近似重复**的判定阈值（词面 Jaccard）。
const DUP_THRESHOLD: f64 = 0.8;

fn novels_dir() -> PathBuf {
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("novels"),
        None => std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".apeireth")
            .join("novels"),
    }
}

fn exam_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("APEIRETH_MIRACL_EXAM") {
        return PathBuf::from(dir);
    }
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("miracl-exam"),
        None => std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".apeireth")
            .join("miracl-exam"),
    }
}

/// 词面 token 集合（与管线内部同口径：`unicode_tokens`）。
fn token_set(text: &str) -> HashSet<String> {
    unicode_tokens(text).into_iter().collect()
}

/// 两集合的 Jaccard 相似度（与管线内部同口径）。
fn jaccard(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    let intersection = left.intersection(right).count() as f64;
    let union = left.union(right).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// 结果里的**近似重复对数**（两两 Jaccard ≥ 阈值）。
fn near_duplicate_pairs(items: &[MemoryCandidate], threshold: f64) -> usize {
    let sets: Vec<HashSet<String>> = items.iter().map(|item| token_set(&item.content)).collect();
    let mut pairs = 0usize;
    for i in 0..sets.len() {
        for j in (i + 1)..sets.len() {
            if jaccard(&sets[i], &sets[j]) >= threshold {
                pairs += 1;
            }
        }
    }
    pairs
}

fn main() -> ExitCode {
    let mut corpora = Vec::new();
    let mut loaded = 0usize;
    let dir = novels_dir();
    for (name, filename) in CORPORA {
        let path = dir.join(filename);
        if !path.exists() {
            eprintln!(
                "· {name}：语料不在（{}）⇒ **跳过**（不是「跑过了」）",
                path.display()
            );
            continue;
        }
        match sweep_internal(name, &path) {
            Ok(entry) => {
                loaded += 1;
                corpora.push(entry);
            }
            Err(error) => {
                eprintln!("· {name}：**跑失败**（{}）：{error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }
    let real_query = match sweep_real_query() {
        Ok(entry) => entry,
        Err(error) => {
            eprintln!("· 真实查询卷：**跑失败**：{error}");
            return ExitCode::FAILURE;
        }
    };
    if loaded == 0 && real_query.is_none() {
        eprintln!(
            "一份卷都没跑到（内部卷目录 {} · 真人卷目录 {}）⇒ **不能出数字**（不许把「没跑」当「没问题」）",
            dir.display(),
            exam_dir().display()
        );
        return ExitCode::FAILURE;
    }
    println!(
        "{}",
        json!({
            "k": K,
            "case_sample": CASE_SAMPLE,
            "dup_threshold": DUP_THRESHOLD,
            "internal_corpora": corpora,
            "real_query": real_query,
        })
    );
    ExitCode::SUCCESS
}

/// 内部卷（模板题）：同语料 / 同挖题 / 同生产管线，只改 λ。
fn sweep_internal(
    name: &str,
    path: &std::path::Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    let config = MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        ..MiningConfig::default()
    };
    let mined = apeireth_memory::golden_mining::mine(&records, config);
    let golden = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));

    let candidates: Vec<MemoryCandidate> = records
        .iter()
        .map(|record| MemoryCandidate {
            id: record.id.clone(),
            layer: "golden".to_string(),
            scope: MemoryScope::Global,
            content: record.content.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance {
                source: "internal_corpus".to_string(),
                source_session: None,
                source_trace: None,
                source_request: None,
            },
        })
        .collect();
    let source = Bm25LexicalCandidateSource::new(candidates);

    let cases: Vec<_> = if CASE_SAMPLE == 0 {
        golden.cases.clone()
    } else {
        golden.cases.iter().take(CASE_SAMPLE).cloned().collect()
    };

    let mut rows = Vec::new();
    for lambda in LAMBDAS {
        let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig {
            diversity_lambda: lambda,
            ..MemoryRankingConfig::default()
        });
        let mut batch = EvalBatch::default();
        let mut duplicate_pairs = 0usize;
        for case in &cases {
            let items = pipeline.retrieve(
                &case.query,
                &[MemoryScope::Global],
                &[&source],
                K,
                MAX_CHARS,
            )?;
            duplicate_pairs += near_duplicate_pairs(&items, DUP_THRESHOLD);
            batch.cases.push(case.clone());
            batch.results.push(Retrieved {
                ids: items.into_iter().map(|item| item.id).collect(),
                fragments: Vec::new(),
            });
        }
        let metrics = evaluate(&batch, K)?;
        let per_case = if metrics.total_cases == 0 {
            0.0
        } else {
            duplicate_pairs as f64 / metrics.total_cases as f64
        };
        eprintln!(
            "· {name} · λ={lambda:.2}：{} 题 · recall@{K}={:.4} · nDCG@{K}={:.4} · 近似重复对/题={:.4}",
            metrics.total_cases, metrics.recall_at_k, metrics.ndcg_at_k, per_case
        );
        rows.push(json!({
            "lambda": lambda,
            "cases": metrics.total_cases,
            "recall_at_k": metrics.recall_at_k,
            "ndcg_at_k": metrics.ndcg_at_k,
            "near_duplicate_pairs_per_case": per_case,
        }));
    }
    Ok(json!({ "name": name, "cases_total": golden.cases.len(), "lambda": rows }))
}

/// 真实查询卷（真人问句 + 官方标注）：同生产管线，只改 λ。
fn sweep_real_query() -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
    let root = exam_dir();
    let corpus_path = root.join("corpus.jsonl");
    let exam_path = root.join("exam.jsonl");
    if !corpus_path.exists() || !exam_path.exists() {
        eprintln!(
            "· 真实查询卷：考卷不在（{}）⇒ **跳过**（不是「跑过了」）",
            root.display()
        );
        return Ok(None);
    }
    let corpus_text = std::fs::read_to_string(&corpus_path)?;
    let exam_text = std::fs::read_to_string(&exam_path)?;
    let mut documents: Vec<MemoryCandidate> = Vec::new();
    for line in corpus_text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        let id = value["id"].as_str().unwrap_or_default().to_string();
        if id.is_empty() {
            continue;
        }
        let content = format!(
            "{}\n{}",
            value["title"].as_str().unwrap_or_default(),
            value["text"].as_str().unwrap_or_default()
        );
        documents.push(MemoryCandidate {
            id,
            layer: "real_query".to_string(),
            scope: MemoryScope::Global,
            content,
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance::default(),
        });
    }
    let document_count = documents.len();
    let source = Bm25LexicalCandidateSource::new(documents);

    let mut cases: Vec<(String, HashSet<String>)> = Vec::new();
    for line in exam_text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        let query = value["query"].as_str().unwrap_or_default().to_string();
        let gold: HashSet<String> = value["gold"]
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
        cases.push((query, gold));
    }
    if cases.is_empty() {
        eprintln!("· 真实查询卷：题集为空 ⇒ **没测到**");
        return Ok(None);
    }
    eprintln!(
        "· 真实查询卷：语料 {document_count} 段 · 题 {} 道 · k={K}",
        cases.len()
    );

    let mut rows = Vec::new();
    for lambda in REAL_QUERY_LAMBDAS {
        let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig {
            diversity_lambda: lambda,
            ..MemoryRankingConfig::default()
        });
        let mut hits = 0usize;
        let mut scored = 0usize;
        let mut duplicate_pairs = 0usize;
        for (query, gold) in &cases {
            let items =
                pipeline.retrieve(query, &[MemoryScope::Global], &[&source], K, MAX_CHARS)?;
            if items.is_empty() {
                continue;
            }
            scored += 1;
            duplicate_pairs += near_duplicate_pairs(&items, DUP_THRESHOLD);
            if items.iter().any(|item| gold.contains(&item.id)) {
                hits += 1;
            }
        }
        let denominator = scored.max(1) as f64;
        let recall = hits as f64 / denominator;
        let per_case = if scored == 0 {
            0.0
        } else {
            duplicate_pairs as f64 / scored as f64
        };
        eprintln!(
            "· 真实查询 · λ={lambda:.2}：有效 {} 题 · recall@{K}={:.4} · 近似重复对/题={:.4}",
            scored, recall, per_case
        );
        rows.push(json!({
            "lambda": lambda,
            "cases": scored,
            "recall_at_k": recall,
            "near_duplicate_pairs_per_case": per_case,
        }));
    }
    Ok(Some(json!({ "name": "real_query", "lambda": rows })))
}
