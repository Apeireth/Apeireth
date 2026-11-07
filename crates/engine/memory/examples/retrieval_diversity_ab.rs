//! 检索「相关性-多样性折中」重排的 **λ 敏感性**离线测量。
//!
//! # 为什么需要这个测例
//!
//! 生产混合检索管线在融合排序之后有一道**残差式多样性重排**（贪心选择：
//! `(1-λ)·相关度 + λ·新颖度`，新颖度 = 1 − 与已选项的最大**词面**相似度），
//! 旋钮是 `MemoryRankingConfig::diversity_lambda`（默认 0.20）。
//!
//! 但**既有两道闸门都量不到它**：
//! * `scripts/check_golden_baseline.py` 走**单路 BM25**（`golden_run::run`），不经过管线；
//! * `scripts/check_fusion_channels.py` 量**通道贡献**（多路并联 delta），不单独量重排。
//!
//! ⇒ 本测例用**同一份内部卷、同一批题、同一个生产管线**，只改 λ，把「重排把分数
//! 推高还是压低、λ 取多少合理」变成可复现数字。
//!
//! # 口径
//!
//! * 语料 = `$APEIRETH_DATA_DIR/novels/`（默认 `~/.apeireth/novels/`）两本内部卷；
//! * 挖题 = 与基线同口径（`MiningConfig::default` + 每记录 6 题）；`CASE_SAMPLE` 仅**截前 N 题**；
//! * 检索 = 生产管线 `HybridRetrievalPipeline::retrieve`（单路 BM25 源、`Global` 可见域、
//!   `k = 10`、预算足够大 ⇒ 只量排序不量注入）；
//! * 指标 = `recall@10` / `nDCG@10`（`golden_set::evaluate` 同口径）；
//! * λ 网格 = `[0.0, 0.2(默认), 0.5, 0.8, 1.0]`。
//!
//! # 中性表述
//!
//! 只描述**机制**（相关性-多样性折中 / 贪心重排 / 词面相似度），不引用任何外部产品名、
//! 文档或代码。
//!
//! # 输出
//!
//! stdout 一行 JSON；stderr 人读摘要 + 语料缺失如实说明（跳过 ≠ 跑过）。

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
use apeireth_memory::MemoryRankingConfig;
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
const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

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

fn main() -> ExitCode {
    let dir = novels_dir();
    let mut corpora = Vec::new();
    let mut loaded = 0usize;
    for (name, filename) in CORPORA {
        let path = dir.join(filename);
        if !path.exists() {
            eprintln!(
                "· {name}：语料不在（{}）⇒ **跳过**（不是「跑过了」）",
                path.display()
            );
            continue;
        }
        match sweep_one(name, &path) {
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
    if loaded == 0 {
        eprintln!(
            "一份语料都没跑到（目录 {}）⇒ **不能出数字**（不许把「没跑」当「没问题」）",
            dir.display()
        );
        return ExitCode::FAILURE;
    }
    println!(
        "{}",
        json!({ "k": K, "case_sample": CASE_SAMPLE, "corpora": corpora })
    );
    ExitCode::SUCCESS
}

fn sweep_one(
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
        for case in &cases {
            let items = pipeline.retrieve(
                &case.query,
                &[MemoryScope::Global],
                &[&source],
                K,
                MAX_CHARS,
            )?;
            batch.cases.push(case.clone());
            batch.results.push(Retrieved {
                ids: items.into_iter().map(|item| item.id).collect(),
                fragments: Vec::new(),
            });
        }
        let metrics = evaluate(&batch, K)?;
        eprintln!(
            "· {name} · λ={lambda:.2}：{} 题 · recall@{K}={:.4} · nDCG@{K}={:.4}",
            metrics.total_cases, metrics.recall_at_k, metrics.ndcg_at_k
        );
        rows.push(json!({
            "lambda": lambda,
            "cases": metrics.total_cases,
            "recall_at_k": metrics.recall_at_k,
            "ndcg_at_k": metrics.ndcg_at_k,
        }));
    }
    Ok(json!({ "name": name, "cases_total": golden.cases.len(), "lambda": rows }))
}
