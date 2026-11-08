//! 残差式多样性重排（相关性-多样性折中）：机制、口径与开关。
//!
//! # 为什么在子模块
//! `retrieval_pipeline.rs` 是 `scripts/arch-baseline.txt` 登记的**尺寸棘轮文件**（只许不增）
//! ⇒ ⇒ 这段贪心选择整体搬出，主文件只留**一行调用**；语义、默认值与确定性逐字不变。
//!
//! # 口径
//! 候选源**不暴露向量** ⇒ ⇒ 词面 Jaccard 是最诚实的成对关系回退；
//! 语义余弦在协同器拿到真向量时使用（两条口径不混用、不假装）。
//! 强度由 `MemoryRankingConfig::diversity_lambda` 控制：`lambda == 0` ⇒ 纯相关性排序，
//! `lambda == 1` ⇒ 纯新颖度排序；默认值见 `MemoryRankingConfig::default`。

use std::collections::HashSet;

use super::scoring::{jaccard_similarity, token_set};
use crate::{MemoryCandidate, MemoryRankingConfig};

/// 贪心、确定性的多样性选择：按 MMR（相关性-多样性折中）挑，再受 `max_chars` 预算约束。
///
/// # 预算的**计费口径**（2026-11-08 修「静默全失」）
/// 每条候选按 **选后长度**（`min(正文长度, 单条上限)`）计费 ⇒ ⇒ 与**交付侧**
/// （按单条上限裁剪后注入）**同一口径** ⇒ ⇒ 预算不会再把「长记录」整批挤掉。
///
/// 选出后把所选候选的新颖度写进 `score_components.novelty`（**只做可解释性**，
/// 不让后置标注改变排序）。
pub(super) fn select(
    ranking: &MemoryRankingConfig,
    candidates: Vec<MemoryCandidate>,
    limit: usize,
    max_chars: usize,
) -> Vec<MemoryCandidate> {
    // Greedy, deterministic MMR selection.  Candidate sources do not expose
    // vectors, so lexical Jaccard is the honest pairwise relation fallback;
    // semantic cosine is already used by the coordinator for relevance when
    // valid vectors are available.
    let lambda = ranking.diversity_lambda.clamp(0.0, 1.0);
    let mut remaining = candidates;
    let mut seen_content = HashSet::new();
    let mut selected_tokens: Vec<HashSet<String>> = Vec::new();
    let mut result = Vec::new();
    let mut chars = 0;
    while result.len() < limit && !remaining.is_empty() {
        let mut best: Option<(usize, f64, f64)> = None;
        for (index, candidate) in remaining.iter().enumerate() {
            let normalized: String = candidate
                .content
                .chars()
                .filter(|ch| ch.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect();
            if normalized.is_empty() || seen_content.contains(&normalized) {
                continue;
            }
            let candidate_tokens = token_set(&candidate.content);
            let max_similarity = selected_tokens
                .iter()
                .map(|selected| jaccard_similarity(&candidate_tokens, selected))
                .fold(0.0, f64::max);
            let novelty = 1.0 - max_similarity;
            let mmr = (1.0 - lambda) * candidate.score + lambda * novelty;
            let better = match best {
                None => true,
                Some((best_index, best_mmr, _)) => {
                    mmr > best_mmr + f64::EPSILON
                        || ((mmr - best_mmr).abs() <= f64::EPSILON
                            && candidate.id < remaining[best_index].id)
                }
            };
            if better {
                best = Some((index, mmr, novelty));
            }
        }
        let Some((index, _, novelty)) = best else {
            break;
        };
        let mut candidate = remaining.swap_remove(index);
        // ⭐⭐ **计费口径 = 选后长度**（2026-11-08：修一个「静默全失」）。
        //
        // # 为什么不能按**原始正文长度**计费
        // 召回交付给模型的是**选段**（单条 ≤ 单条上限），不是整条正文 ⇒ ⇒
        // 按整条计费 ⇒ ⇒ 一条 7,400 字的记录在 4,000 字预算下**一条都装不下**
        // ⇒ ⇒ 整批候选被跳过 ⇒ ⇒ **不报错、不告警、模型一条也收不到**（静默全失）。
        // ⇒ ⇒ 计费必须与**交付侧同一口径**：`min(正文长度, 单条上限)`。
        //
        // # 单条上限从哪来
        // [`MemoryRankingConfig::candidate_span_chars`]（默认 = 生产注入的单条跨度
        // ⇒ **单一真源**，见 [`crate::anchor_injection::AnchoredInjectionConfig::PRODUCTION`]）
        // ⇒ ⇒ `0` = 不限（按原始正文计费 = 旧口径，逃生阀）。
        let content_chars = candidate.content.chars().count();
        let billed_chars = match ranking.candidate_span_chars {
            0 => content_chars,
            span => content_chars.min(span),
        };
        if chars + billed_chars > max_chars {
            // Skip oversized candidates and continue looking for a bounded item.
            continue;
        }
        let normalized: String = candidate
            .content
            .chars()
            .filter(|ch| ch.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();
        seen_content.insert(normalized);
        // Keep source relevance and make novelty explainable without
        // letting the post-selection annotation change ordering.
        candidate.score_components.novelty = novelty;
        selected_tokens.push(token_set(&candidate.content));
        chars += billed_chars;
        result.push(candidate);
    }
    result
}
