//! **融合量纲探针**（2026-11-06）—— 量清「**融合到底把联想通道淹没了多少**」。
//!
//! # 这个例子回答什么
//!
//! 公平卷实测：**联想通道单独 recall@10 = 0.4760**，**融合后 = 0.3269**
//! ⇒ ⇒ **通道单独比融合强 15 个点** ⇒ ⇒ ⇒ **融合在丢召回，不是通道没用**。
//!
//! 本探针**不猜**，逐题量三件事：
//! 1. BM25 一路给了多少候选、各自多少分；
//! 2. 联想一路给了多少候选、各自多少分；
//! 3. **融合取 top-k 之后，联想候选还剩几个**。
//!
//! # 为什么必须量
//!
//! 两路的分数**量纲不同**：
//! - BM25 路：顶分 = `lexical(已按 max 归一化) + 覆盖加成` ⇒ **顶分约 1.0~1.2**；
//! - 联想路：分数 = `strength`（共现次数开方，100 封顶）⇒ **典型 0.2~0.5**。
//!
//! ⇒ ⇒ **合并后按分数重排 ⇒ ⇒ 联想候选系统性地排在 BM25 后面**
//! ⇒ ⇒ ⇒ **它只能填 BM25 剩下的空位**。
//!
//! # 输出
//!
//! 一行 JSON（机器读）：每个语料三组数字 + 一个**结论字段**。

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{
    build_fair_exam, CorpusAssociations, CorpusAssociativeSource, FairExamConfig,
};
use apeireth_memory::golden_run::{run_with_extra_sources, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::GoldenSet;
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

/// 评测的 top-k（与 `scripts/golden-baseline.json` 的 `k` 一致）。
const K: usize = 10;

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

/// 一题量出来的数字。
#[derive(Debug, Default)]
struct Probe {
    /// BM25 给了多少候选。
    bm25_count: usize,
    /// BM25 的最高分。
    bm25_top: f64,
    /// BM25 的平均分。
    bm25_mean: f64,
    /// 联想给了多少候选。
    assoc_count: usize,
    /// 联想的最高分。
    assoc_top: f64,
    /// 联想的平均分。
    assoc_mean: f64,
    /// 联想候选里**分数大于 BM25 顶分**的有几个（**能压过 BM25 的**）。
    assoc_above_bm25_top: usize,
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn probe_one(name: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let path = corpus_root().join(format!("{name}.txt"));
    if !path.exists() {
        return Err(format!("语料不在：{}", path.display()).into());
    }
    let chapters = novel_corpus::load_chapters(&path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();

    // 公平卷（零人工标注）
    let fair = GoldenSet::new(
        format!("fair-{name}"),
        build_fair_exam(&records, FairExamConfig::default()),
    );
    let associations = Arc::new(CorpusAssociations::build(&records));
    let channel = CorpusAssociativeSource {
        associations: associations.clone(),
        records: index.clone(),
    };

    // BM25 一路（与生产同源）
    let bm25 = Bm25LexicalCandidateSource::new(
        index
            .iter()
            .map(|record| MemoryCandidate {
                id: record.id.clone(),
                layer: "golden".to_string(),
                scope: MemoryScope::Global,
                content: record.text.clone(),
                score: 0.0,
                score_components: Default::default(),
                provenance: Default::default(),
            })
            .collect(),
    );

    let mut probes: Vec<Probe> = Vec::new();
    // ⭐ **修法验证**：每路按**自己的 max** 归一化后再合并（量纲对齐）
    let mut normalized_hits = 0usize;
    let mut normalized_cases = 0usize;
    let mut assoc_total = 0usize;
    let mut assoc_survived_total = 0usize;
    let mut hit_with_channel = 0usize;
    let mut cases = 0usize;

    for case in &fair.cases {
        let bm25_hits = bm25.candidates(&case.query, K)?;
        let assoc_hits = channel.candidates(&case.query, K)?;

        let bm25_scores: Vec<f64> = bm25_hits.iter().map(|c| c.score).collect();
        let assoc_scores: Vec<f64> = assoc_hits.iter().map(|c| c.score).collect();
        let bm25_top = bm25_scores.first().copied().unwrap_or(0.0);

        // 融合后的 top-k（与生产同结构：并集 ⇒ 按分重排 ⇒ 取 k ⇒ 丢零分）
        let mut merged: Vec<(String, f64)> =
            bm25_hits.iter().map(|c| (c.id.clone(), c.score)).collect();
        for candidate in &assoc_hits {
            match merged.iter_mut().find(|item| item.0 == candidate.id) {
                Some(existing) if candidate.score > existing.1 => existing.1 = candidate.score,
                Some(_) => {}
                None => merged.push((candidate.id.clone(), candidate.score)),
            }
        }
        merged.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        let topk: Vec<&(String, f64)> = merged.iter().filter(|item| item.1 > 0.0).take(K).collect();
        let assoc_ids: Vec<&str> = assoc_hits.iter().map(|c| c.id.as_str()).collect();
        let survived = topk
            .iter()
            .filter(|item| assoc_ids.contains(&item.0.as_str()))
            .count();
        let hit = !case.expect_ids.is_empty()
            && case
                .expect_ids
                .iter()
                .all(|id| topk.iter().any(|item| item.0 == *id));

        // ⭐ **修法**：两路各自按 max 归一化（各路的「顶分」都变成 1.0）⇒ ⇒ 量纲对齐
        let bm25_max = bm25_scores.iter().cloned().fold(0.0_f64, f64::max);
        let assoc_max = assoc_scores.iter().cloned().fold(0.0_f64, f64::max);
        let mut normalized: Vec<(String, f64)> = bm25_hits
            .iter()
            .map(|c| {
                (
                    c.id.clone(),
                    if bm25_max > 0.0 {
                        c.score / bm25_max
                    } else {
                        0.0
                    },
                )
            })
            .collect();
        for candidate in &assoc_hits {
            let score = if assoc_max > 0.0 {
                candidate.score / assoc_max
            } else {
                0.0
            };
            match normalized.iter_mut().find(|item| item.0 == candidate.id) {
                Some(existing) if score > existing.1 => existing.1 = score,
                Some(_) => {}
                None => normalized.push((candidate.id.clone(), score)),
            }
        }
        normalized.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        let normalized_topk: Vec<&(String, f64)> = normalized
            .iter()
            .filter(|item| item.1 > 0.0)
            .take(K)
            .collect();
        let normalized_hit = !case.expect_ids.is_empty()
            && case
                .expect_ids
                .iter()
                .all(|id| normalized_topk.iter().any(|item| item.0 == *id));
        normalized_cases += 1;
        if normalized_hit {
            normalized_hits += 1;
        }

        probes.push(Probe {
            bm25_count: bm25_hits.len(),
            bm25_top,
            bm25_mean: mean(&bm25_scores),
            assoc_count: assoc_hits.len(),
            assoc_top: assoc_scores.first().copied().unwrap_or(0.0),
            assoc_mean: mean(&assoc_scores),
            assoc_above_bm25_top: assoc_scores.iter().filter(|s| **s > bm25_top).count(),
        });
        assoc_total += assoc_hits.len();
        assoc_survived_total += survived;
        cases += 1;
        if hit {
            hit_with_channel += 1;
        }
    }

    // 官方 A/B（跑批口径）
    let ab = run_with_extra_sources(&index, &fair, K, InjectionMode::Production, &[&channel])?;
    let baseline = run_with_extra_sources(&index, &fair, K, InjectionMode::Production, &[])?;

    let n = probes.len().max(1);
    Ok(serde_json::json!({
        "name": name,
        "cases": cases,
        "bm25": {
            "候选数/题": probes.iter().map(|p| p.bm25_count).sum::<usize>() / n,
            "顶分均值": probes.iter().map(|p| p.bm25_top).sum::<f64>() / n as f64,
            "均分均值": probes.iter().map(|p| p.bm25_mean).sum::<f64>() / n as f64,
        },
        "联想": {
            "候选数/题": probes.iter().map(|p| p.assoc_count).sum::<usize>() / n,
            "顶分均值": probes.iter().map(|p| p.assoc_top).sum::<f64>() / n as f64,
            "均分均值": probes.iter().map(|p| p.assoc_mean).sum::<f64>() / n as f64,
            "压过BM25顶分的候选数/题": probes.iter().map(|p| p.assoc_above_bm25_top).sum::<usize>() / n,
        },
        "融合后": {
            "联想候选进入top-k的总数": assoc_survived_total,
            "联想候选总数": assoc_total,
            "存活率": if assoc_total == 0 { 0.0 } else { assoc_survived_total as f64 / assoc_total as f64 },
            "命中率(手算)": hit_with_channel as f64 / cases.max(1) as f64,
        },
        "⭐按路归一化融合": {
            "recall@10": normalized_hits as f64 / normalized_cases.max(1) as f64,
            "对比原融合": hit_with_channel as f64 / cases.max(1) as f64,
            "差": (normalized_hits as f64 / normalized_cases.max(1) as f64)
                - (hit_with_channel as f64 / cases.max(1) as f64),
        },
        "官方A/B": {
            "基线recall@10": baseline.metrics.recall_at_k,
            "加通道recall@10": ab.metrics.recall_at_k,
            "delta": ab.metrics.recall_at_k - baseline.metrics.recall_at_k,
        },
    }))
}

use std::sync::Arc;

fn main() -> ExitCode {
    let mut out: Vec<serde_json::Value> = Vec::new();
    for name in ["redchamber", "watershed"] {
        match probe_one(name) {
            Ok(value) => out.push(value),
            Err(error) => {
                eprintln!("· {name}：跳过（{error}）");
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "probe": out })).unwrap_or_default()
    );
    ExitCode::SUCCESS
}
