//! 混合检索融合的三条**实测修正**（按路归一化 / 两阶段顺序 / 填充带）。
//!
//! # 为什么这个文件存在
//!
//! 2026-11-07 起，`retrieval_pipeline.rs` 是 `scripts/arch-baseline.txt` 登记的
//! 尺寸文件（只许不变或变小）。融合规则又连着**实测数字**，写进主文件会不断顶高
//! 它 ⇒ ⇒ 把「规则 + 为什么这么定 + 实测数字」整体搬到本子模块，
//! 主文件只留调用 ⇒ ⇒ **语义与阈值逐字不变，行数回到基线内**。
//!
//! # 一、按路归一化（2026-11-06 实测修正）
//!
//! **各路的分数量纲不同**：词法路的分数按**自己的 max** 归一化（顶分恒 1.0），
//! 而联想路的信号只落在**图维度**（权重 0.05）⇒ 量级差 4 倍 ⇒ 不对齐就合并
//! ⇒ **一路系统性压过另一路**。修法 = 每路先按**自己的 max** 归一化（各路顶分都变成 1.0）。
//! 实测（公平卷，零人工标注）：联想通道收益从 **delta 0.0000** 变成
//! **redchamber +0.1394 / watershed +0.0645** ⇒ 恢复到「通道单独」上限的 **98% / 97.8%**。
//! ⚠️ **单路时是恒等变换**：只有一路 ⇒ max 就是它自己的顶分 ⇒ 除以它不改变任何顺序
//! ⇒ **单路行为逐字不变**。
//!
//! # 二、两阶段顺序（2026-11-07 实测修正）
//!
//! **主力源先跑、填充源后跑** ⇒ 填充源处理时，主力候选**已经全部在 `by_id` 里**
//! ⇒ 于是能算出「主力正分候选的最小分」当作**填充带上限**。
//!
//! # 三、填充带（2026-11-07 实测修正）
//!
//! 底座连续变强后（停用字 +52.5% ⇒ 疑问词 +7.6%），联想通道在内部词面卷上变成
//! **−0.0032**，而 **w = 0.05 / 0.10 / 0.20 三个数字完全相同** ⇒ 不是「权重太大」。
//! 机制：辅助候选归一化后**恒有约 `w` 分**，而主力候选的**尾巴分数可能远小于 `w`**
//! ⇒ 辅助候选**必然顶掉主力最弱的那几条命中**（约 1.5 道题）⇒ 换更小的权重救不了。
//! 口径：`is_filler() == true` 的源，其候选分数被**映射到「主力所有正分候选的最小分」
//! 之下** ⇒ **结构上不可能顶掉任何主力命中** ⇒ 只在主力**填不满** `limit` 个名额时补位。

use std::collections::HashMap;

use super::{MemoryCandidate, MemoryCandidateSource, MemoryRankingConfig};

/// **两阶段顺序**：主力源在前、填充源在后（各自的相对顺序不变）。
pub(super) fn order_sources<'a>(
    sources: &[&'a dyn MemoryCandidateSource],
) -> Vec<&'a dyn MemoryCandidateSource> {
    let mut ordered: Vec<&dyn MemoryCandidateSource> = sources
        .iter()
        .copied()
        .filter(|source| !source.is_filler())
        .collect();
    ordered.extend(sources.iter().copied().filter(|source| source.is_filler()));
    ordered
}

/// 本路候选的**顶分**（用于按路归一化；空集 ⇒ 0.0 ⇒ 恒等变换）。
pub(super) fn route_max(candidates: &[MemoryCandidate], ranking: &MemoryRankingConfig) -> f64 {
    candidates
        .iter()
        .map(|candidate| candidate.score_components.weighted(ranking))
        .fold(0.0_f64, f64::max)
}

/// **填充带**：填充源的分数被压到「主力正分候选最小分」之下（乘 0.999 保证严格靠后）。
///
/// 主力**一个正分候选都没有** ⇒ 填充源正常上（它才是唯一来源）；
/// 非填充源 ⇒ 原值返回（逐字不变）。
pub(super) fn apply_filler_band(
    normalized: f64,
    is_filler: bool,
    by_id: &HashMap<String, MemoryCandidate>,
) -> f64 {
    if !is_filler {
        return normalized;
    }
    let ceiling = by_id
        .values()
        .map(|existing| existing.score)
        .filter(|score| *score > 0.0)
        .fold(f64::INFINITY, f64::min);
    if ceiling.is_finite() {
        normalized * ceiling * 0.999
    } else {
        normalized
    }
}
