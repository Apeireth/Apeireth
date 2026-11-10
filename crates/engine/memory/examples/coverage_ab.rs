//! **`coverage` 特征的 A/B**（**内部卷**上验，因为生产门禁看内部卷）。
//!
//! # 背景
//!
//! `d4_fusion_probe` 在 LoCoMo 上测出「查询词覆盖率（coverage）」**剂量响应正收益**
//! （+0.0045 / +0.0053 / **+0.0125**）⇒ 但 **LoCoMo 不是生产门禁**。
//! ⇒ ⇒ 要落地必须先在我们**自己的卷子**（`internal-golden-v1`，301 题）上验。
//!
//! # 口径
//!
//! * 记录 = 文档章节（`scripts/internal_golden.py --records`）；
//! * 只改**分数融合**（`score = bm25_norm + w·coverage`），语料/题集/k 全不动；
//! * `w` 固定扫描 {0.1, 0.5, 1.0} ⇒ **不调参到最优**。
//!
//! # 怎么跑
//!
//! ```sh
//! python3 scripts/internal_golden.py --records > ~/internal-records.jsonl
//! cargo run -p apeireth-memory --example coverage_ab --locked -- ~/internal-records.jsonl
//! ```

use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use std::process::ExitCode;

const K: usize = 10;
const WEIGHTS: [f64; 3] = [0.1, 0.5, 1.0];

fn main() -> ExitCode {
    let records_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/home/bai/internal-records.jsonl".to_string());
    let Ok(records_text) = std::fs::read_to_string(&records_path) else {
        eprintln!("· 读不到 {records_path} ⇒ **没测到**");
        return ExitCode::FAILURE;
    };
    // ① 记录
    let mut index: Vec<(String, String)> = Vec::new();
    for line in records_text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        let (Some(id), Some(text)) = (
            value.get("id").and_then(|v| v.as_str()),
            value.get("text").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        index.push((id.to_string(), text.to_string()));
    }
    if index.is_empty() {
        eprintln!("· 记录为空 ⇒ **没测到**");
        return ExitCode::FAILURE;
    }
    // ② 考卷
    let Ok(golden_text) = std::fs::read_to_string("docs/03-reference/internal-golden-v1.jsonl")
    else {
        eprintln!("· 读不到考卷 ⇒ **没测到**");
        return ExitCode::FAILURE;
    };
    let mut cases: Vec<(String, Vec<String>)> = Vec::new();
    for line in golden_text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        let query = value.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let expect: Vec<String> = value
            .get("expect_ids")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if query.is_empty() || expect.is_empty() {
            continue;
        }
        cases.push((query.to_string(), expect));
    }
    if cases.is_empty() {
        eprintln!("· 考卷为空 ⇒ **没测到**");
        return ExitCode::FAILURE;
    }
    // ③ 建 BM25 源（取全部候选，排序自己算 ⇒ 融合才可能生效）
    let documents: Vec<MemoryCandidate> = index
        .iter()
        .map(|(id, text)| MemoryCandidate {
            id: id.clone(),
            layer: "internal".to_string(),
            scope: MemoryScope::Global,
            content: text.clone(),
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

    println!("════════ coverage A/B（**内部卷**）════════");
    println!(
        "记录 **{}** 条 · 考卷 **{}** 题 · k = {K} · 只改分数融合",
        index.len(),
        cases.len()
    );
    println!();
    println!("{:<24} {:>11} {:>12}", "臂", "recall@10", "Δ vs 基线");
    println!("{}", "-".repeat(50));

    let baseline = evaluate(&source, &cases, &index, 0.0);
    println!("{:<24} {:>11.4} {:>12}", "基线（纯 BM25）", baseline, "—");
    let mut best = baseline;
    let mut best_label = String::new();
    for weight in WEIGHTS {
        let recall = evaluate(&source, &cases, &index, weight);
        let label = format!("+ coverage ×{weight}");
        println!(
            "{:<24} {:>11.4} {:>12}",
            label,
            recall,
            format!("{:+.4}", recall - baseline)
        );
        if recall > best {
            best = recall;
            best_label = label;
        }
    }
    println!();
    if best > baseline + 0.005 {
        println!(
            "  ✅ **{best_label}** 有正收益：{baseline:.4} → {best:.4}（{:+.4}）",
            best - baseline
        );
        println!("     ⇒ 内部卷与 LoCoMo **同向** ⇒ 可以考虑接进生产排序（需过基线闸门）。");
    } else {
        println!("  ❌ **内部卷上无正收益**（{baseline:.4} → {best:.4}）⇒ **不接生产**");
        println!(
            "     ⇒ 结论收窄：`coverage` 只在 LoCoMo（真实对话）有效，在**工程文档卷**上不成立。"
        );
    }
    ExitCode::SUCCESS
}

/// 跑一遍（`weight = 0` ⇒ 纯 BM25 基线）。
fn evaluate(
    source: &Bm25LexicalCandidateSource,
    cases: &[(String, Vec<String>)],
    index: &[(String, String)],
    weight: f64,
) -> f64 {
    let mut total = 0usize;
    let mut hits = 0usize;
    for (query, expect) in cases {
        let Ok(all) = source.candidates(query, index.len().max(1)) else {
            continue;
        };
        let mut scored: Vec<(String, f64)> = all
            .iter()
            .map(|candidate| {
                let base = candidate.score_components.lexical;
                let fused = if weight > 0.0 {
                    base + weight * coverage(query, &candidate.content)
                } else {
                    base
                };
                (candidate.id.clone(), fused)
            })
            .collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        total += 1;
        if scored.iter().take(K).any(|(id, _)| expect.contains(id)) {
            hits += 1;
        }
    }
    if total == 0 {
        return f64::NAN;
    }
    hits as f64 / total as f64
}

/// 查询词覆盖率（实词命中比例）。
fn coverage(query: &str, text: &str) -> f64 {
    let terms: Vec<String> = query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|term| term.chars().count() > 1)
        .map(str::to_lowercase)
        .collect();
    if terms.is_empty() {
        return 0.0;
    }
    let haystack = text.to_lowercase();
    let hit = terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count();
    hit as f64 / terms.len() as f64
}
