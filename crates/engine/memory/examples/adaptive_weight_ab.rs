//! **自适应权重 vs 常数权重**（2026-11-07）—— 证明「按头名间距自适应」打得过「一个常数」。
//!
//! # 背景（实测链条）
//! 1. 通道与底座**此消彼长**（底座越强，通道在「词面强的题」上越挤）；
//! 2. 四个候选「主通道信心」信号里，**只有「头名间距（margin）」有分辨力**
//!    （极差 0.1076；覆盖率 0.0297；命中条数 / 顶分绝对值 0.0000）；
//! 3. ⇒ ⇒ ⇒ 假设：**主通道头名间距小 ⇒ ⇒ 它自己也不确定 ⇒ ⇒ 联想通道该顶上**。
//!
//! # 本例子测什么
//! 同一批题、同一套检索器，**只改「辅助通道的权重怎么定」**：
//! - **常数档**：w ∈ {0.10, 0.20, 0.30, 0.45, 1.00}；
//! - **自适应档**：`w = base × (1 − 主通道头名间距)`（间距小 ⇒ ⇒ 权重大）。
//!
//! # 关键实现（**也是生产实现该有的样子**）
//! 辅助源**自己按题**去问主通道「你这一题的把握多大」，
//! 再据此缩放自己的分数 ⇒ ⇒ ⇒ **一次跑批**就能算出两档数字。
//!
//! # 输出
//! 一行 JSON（每档的 recall@10）。

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{
    build_fair_exam, CorpusAssociations, CorpusAssociativeSource, FairExamConfig,
};
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::{mined_to_golden, run_with_extra_sources, IndexRecord};
use apeireth_memory::golden_set::GoldenSet;
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

const K: usize = 10;
const MAX_CASES_PER_RECORD: usize = 6;

/// 一路候选的「头名间距」（第一名 − 第二名，**夹在 0..1**）。
fn margin(scores: &[f64]) -> f64 {
    let mut sorted = scores.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let top = sorted.first().copied().unwrap_or(0.0);
    let second = sorted.get(1).copied().unwrap_or(0.0);
    (top - second).clamp(0.0, 1.0)
}

/// ⭐ **按主通道头名间距缩放权重的辅助源**。
struct ScaledSource {
    inner: CorpusAssociativeSource,
    /// 主通道（用来问「这一题的把握」）。
    main: Bm25LexicalCandidateSource,
    /// 基础权重。
    base: f64,
    /// `true` = 自适应（按间距缩放）；`false` = 常数。
    adaptive: bool,
    /// ⭐ **准入阈值**：`Some(theta)` ⇒ ⇒ **主通道 margin ≥ theta 时，本路直接不上场**
    /// （⇒ ⇒ 「准入 + 权重」两段式）。
    admission: Option<f64>,
}

impl ScaledSource {
    /// ⭐ **本路这一题的权重**（**归一化之后**才乘 ⇒ ⇒ 见下面的注释）。
    fn weight_for(&self, query: &str) -> f64 {
        let hits = self.main.candidates(query, K).unwrap_or_default();
        let scores: Vec<f64> = hits.iter().map(|h| h.score).collect();
        let main_margin = margin(&scores);
        // ⭐⭐ **准入规则**（2026-11-07 实测出来的两段式）：
        // 主通道 margin 大 ⇒ ⇒ 它很确定 ⇒ ⇒ **本路不许上场**（直接 0）
        // ⇒ ⇒ 否则本路在「词面强的题」上会把正确答案挤掉（实测 -0.0651）
        if let Some(theta) = self.admission {
            if main_margin >= theta {
                return 0.0;
            }
        }
        if !self.adaptive {
            return self.base;
        }
        // ⭐ **间距小 ⇒ ⇒ 主通道虚 ⇒ ⇒ 权重大**（让联想顶上）
        self.base * (1.0 - main_margin)
    }
}

/// ⭐ **自己合并两路并算 recall@10**（**权重在归一化之后乘**）。
///
/// # 为什么不能直接用 `run_with_extra_sources`
/// 那条路会把**额外源的分数先归一化**（`÷ source_max`）⇒ ⇒
/// **若在 source 里预先乘权重 ⇒ ⇒ 会被完全抵消**（同比缩放）
/// ⇒ ⇒ ⇒ **权重必须在归一化之后乘**（生产管线 `retrieval_pipeline` 就是这么做的）。
fn fuse_recall(
    main: &Bm25LexicalCandidateSource,
    auxiliary: &ScaledSource,
    set: &GoldenSet,
) -> Result<f64, Box<dyn Error>> {
    use apeireth_memory::golden_set::{evaluate, EvalBatch, Retrieved};
    let mut batch = EvalBatch::default();
    for case in &set.cases {
        // 路 A：主通道
        let main_hits = main.candidates(&case.query, K)?;
        let main_max = main_hits.iter().map(|c| c.score).fold(0.0_f64, f64::max);
        let mut merged: Vec<(String, f64)> = main_hits
            .iter()
            .map(|c| {
                (
                    c.id.clone(),
                    if main_max > 0.0 {
                        c.score / main_max
                    } else {
                        0.0
                    },
                )
            })
            .collect();
        // 路 B：辅助通道（**先归一化，再乘权重**）
        let aux_hits = auxiliary.inner.candidates(&case.query, K)?;
        let aux_max = aux_hits.iter().map(|c| c.score).fold(0.0_f64, f64::max);
        let weight = auxiliary.weight_for(&case.query);
        for candidate in &aux_hits {
            let normalized = if aux_max > 0.0 {
                candidate.score / aux_max
            } else {
                0.0
            };
            let score = normalized * weight;
            match merged.iter_mut().find(|item| item.0 == candidate.id) {
                Some(existing) if score > existing.1 => existing.1 = score,
                Some(_) => {}
                None => merged.push((candidate.id.clone(), score)),
            }
        }
        merged.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        let ids: Vec<String> = merged
            .iter()
            .filter(|item| item.1 > 0.0)
            .take(K)
            .map(|item| item.0.clone())
            .collect();
        batch.cases.push(case.clone());
        batch.results.push(Retrieved {
            ids,
            fragments: Vec::new(),
        });
    }
    let metrics = evaluate(&batch, K)
        .map_err(|error| -> Box<dyn Error> { format!("评测失败：{error}").into() })?;
    Ok(metrics.recall_at_k)
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
        let associations = std::sync::Arc::new(CorpusAssociations::build(&records));
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
            let baseline = run_with_extra_sources(
                &index,
                set,
                K,
                apeireth_memory::golden_run::InjectionMode::Production,
                &[],
            )?;
            let mut constants = Vec::new();
            for weight in [0.10_f64, 0.20, 0.30, 0.45, 1.00] {
                let scaled = ScaledSource {
                    inner: CorpusAssociativeSource {
                        associations: associations.clone(),
                        records: index.clone(),
                    },
                    main: bm25.clone(),
                    base: weight,
                    adaptive: false,
                    admission: None,
                };
                constants.push((weight, fuse_recall(&bm25, &scaled, set)?));
            }
            let mut adaptive = Vec::new();
            for base in [1.00_f64, 2.00] {
                let scaled = ScaledSource {
                    inner: CorpusAssociativeSource {
                        associations: associations.clone(),
                        records: index.clone(),
                    },
                    main: bm25.clone(),
                    base,
                    adaptive: true,
                    admission: None,
                };
                adaptive.push((base, fuse_recall(&bm25, &scaled, set)?));
            }
            // **准入档**：只有主通道 margin < theta 时，本路才上场
            let mut admitted = Vec::new();
            for (theta, base) in [
                (0.10_f64, 2.00_f64),
                (0.20, 2.00),
                (0.30, 2.00),
                (0.50, 2.00),
                (0.20, 4.00),
            ] {
                let scaled = ScaledSource {
                    inner: CorpusAssociativeSource {
                        associations: associations.clone(),
                        records: index.clone(),
                    },
                    main: bm25.clone(),
                    base,
                    adaptive: true,
                    admission: Some(theta),
                };
                let recall = fuse_recall(&bm25, &scaled, set)?;
                admitted.push(serde_json::json!({
                    "theta": theta,
                    "base": base,
                    "recall": recall,
                    "delta": recall - baseline.metrics.recall_at_k,
                }));
            }
            rows.push(serde_json::json!({
                "语料": name,
                "考卷": exam,
                "题数": set.cases.len(),
                "基线": baseline.metrics.recall_at_k,
                "常数档": constants.iter().map(|(w, r)| serde_json::json!({"w": w, "recall": r, "delta": r - baseline.metrics.recall_at_k})).collect::<Vec<_>>(),
                "自适应档": adaptive.iter().map(|(w, r)| serde_json::json!({"base": w, "recall": r, "delta": r - baseline.metrics.recall_at_k})).collect::<Vec<_>>(),
                "准入档": admitted,
            }));
        }
    }
    for row in &rows {
        eprintln!(
            "· {} / {} · 题 {} · 基线 {:.4}",
            row["语料"].as_str().unwrap_or(""),
            row["考卷"].as_str().unwrap_or(""),
            row["题数"],
            row["基线"].as_f64().unwrap_or(0.0)
        );
        for c in row["常数档"].as_array().unwrap_or(&vec![]) {
            eprintln!(
                "    常数 w={:.2} ⇒ {:.4} ({:+.4})",
                c["w"].as_f64().unwrap_or(0.0),
                c["recall"].as_f64().unwrap_or(0.0),
                c["delta"].as_f64().unwrap_or(0.0)
            );
        }
        for c in row["自适应档"].as_array().unwrap_or(&vec![]) {
            eprintln!(
                "    自适应 base={:.2} ⇒ {:.4} ({:+.4})",
                c["base"].as_f64().unwrap_or(0.0),
                c["recall"].as_f64().unwrap_or(0.0),
                c["delta"].as_f64().unwrap_or(0.0)
            );
        }
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "自适应vs常数": rows })).unwrap_or_default()
    );
    Ok(())
}
