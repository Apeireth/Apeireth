//! 检索融合用的**朴素评分工具**：分词集合 / 集合相似度 / 查询词覆盖率。
//!
//! 从 `retrieval_pipeline.rs` 拆出（2026-11-07）：`arch_guard --strict` 报
//! size-grow（1318 -> 1328 行）⇒ 做**真实拆分**（不是删注释/空行装样）。
//! ⚠️ 本段上方的**实测记录**（覆盖率两语料扫描数字、权重取 0.5 的理由）随代码
//! 一起搬走，语义与阈值逐字不变；父模块只保留管线主体与装配。
//!
//! 可见性：父模块（`super`）内部使用 ⇒ `pub(super)`，不进 crate 公共面。

use std::collections::HashSet;

use super::unicode_tokens;

pub(super) fn token_set(text: &str) -> HashSet<String> {
    unicode_tokens(text).into_iter().collect()
}

pub(super) fn jaccard_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
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

/// **查询词覆盖率**：query 的实词有多少**出现在** document 文本里（0..1）。
///
/// # 为什么需要它（2026-10-28 实测）
///
/// BM25 是**加权**的（IDF + 长度归一）⇒ 它可能把「命中长词一次」排在
/// 「命中所有短词」前面。覆盖率是**朴素计数**，与 BM25 **不共线**
/// ⇒ ⇒ 融合后两语料**同向正收益**：
/// * LoCoMo（真实对话 1,122 题）：`recall@10` 0.6221 → **0.6346**（**+0.0125**）；
/// * 内部卷（工程文档 301 题）：0.8738 → **0.9003**（**+0.0266**）。
///
/// # 口径
///
/// * 只算**长度 ≥ 2** 的实词（去掉单字噪声）；
/// * 按**子串包含**判定（与生产分词口径一致：CJK 二字组 / ASCII 词）；
/// * `query` 无实词 ⇒ 返回 0（**不加分**，不制造偏向）。
pub(super) fn query_coverage(query: &str, document_text: &str) -> f64 {
    let terms: Vec<String> = query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|term| term.chars().count() >= 2)
        .map(str::to_lowercase)
        .collect();
    if terms.is_empty() {
        return 0.0;
    }
    let haystack = document_text.to_lowercase();
    let hit = terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count();
    hit as f64 / terms.len() as f64
}

/// 覆盖率在最终分数里的权重（**实测值**，不是拍的）。
///
/// 两语料扫描 {0.1, 0.5, 1.0} 后取**两语料都正收益**的档位：
/// 内部卷在 ×0.5 与 ×1.0 同为 0.9003，LoCoMo 在 ×1.0 最高（0.6346）
/// ⇒ 取 **0.5**：两头都有正收益、且对长记录的长度偏向更小（更保守）。
pub(super) const COVERAGE_WEIGHT: f64 = 0.5;
