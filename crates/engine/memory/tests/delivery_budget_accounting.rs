//! **候选预算计费口径**的钉死测试（2026-11-08）—— 长记录**不许被整条静默丢掉**。
//!
//! # 缺陷（实测定位，2026-11-08）
//! 检索管线的字符预算原本按候选的**原始正文长度**计费
//!；一条 7,400 字的整章记录在 4,000 字预算下**一条都装不下**
//!；而当**所有**候选都这样时，**整批候选被静默丢弃**
//!；不报错、不告警、模型一条也收不到（实测送达率 0.0000）。
//!
//! # 正确口径
//! 生产注入走**锚定选段**（`AnchoredInjectionConfig::PRODUCTION.span_chars` = 200 字）
//!；**实际送达量 = `min(原始长度, 200)`**
//!；预算必须按**送达量**计费。
//!
//! # 本测试的性质：**反证**
//! 把计费改回「按原始长度」； **这些用例必须变红**
//!；它们量的是「缺陷还在不在」，不是「修复在不在」。

use apeireth_memory::anchor_injection::AnchoredInjectionConfig;
use apeireth_memory::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
};
use apeireth_memory::{MemoryCandidate, MemoryRankingConfig, MemoryScope};

/// 生产总预算（与 `coordinator` 的 `max_chars: 4_000` 一致）。
const MAX_CHARS: usize = 4_000;

fn candidate(id: &str, content: &str) -> MemoryCandidate {
    MemoryCandidate {
        id: id.to_string(),
        layer: "test".to_string(),
        scope: MemoryScope::Global,
        content: content.to_string(),
        score: 0.0,
        score_components: Default::default(),
        provenance: Default::default(),
    }
}

/// 一条**整章级**的长记录（远长于单条送达跨度）。
fn chapter(id: &str, seed: &str) -> MemoryCandidate {
    candidate(id, &format!("{seed}{}", "話說當時".repeat(1_000)))
}

/// 单条实际送达的字符数（= 预算计费口径，取**配置旋钮**这个单一真源）。
fn delivered(content: &str) -> usize {
    let span = MemoryRankingConfig::default().candidate_span_chars;
    content.chars().count().min(span)
}

/// ⭐⭐ **核心反证**：全是长记录时，**一条都不许丢**。
#[test]
fn long_records_are_not_silently_dropped() {
    let scope = MemoryScope::Global;
    let source = Bm25LexicalCandidateSource::new(vec![
        chapter("long-a", "甲"),
        chapter("long-b", "乙"),
        chapter("long-c", "丙"),
    ]);
    let pipeline = HybridRetrievalPipeline::default();
    let items = pipeline
        .retrieve("甲乙丙", &[scope], &[&source], 3, MAX_CHARS)
        .expect("retrieve must not fail");

    assert!(
        !items.is_empty(),
        "全是长记录时**一条都不许丢**（修复前这里返回空集，送达率 0.0000 且不报错）"
    );
    assert_eq!(
        items.len(),
        3,
        "三条都该装得下：每条只送达 {} 字，预算 {} 字，装得下 3 条",
        AnchoredInjectionConfig::PRODUCTION.span_chars,
        MAX_CHARS
    );
}

/// ⭐ **反证**：按送达量计费；总送达量**不超过**预算。
#[test]
fn delivered_total_stays_within_budget() {
    let scope = MemoryScope::Global;
    let source =
        Bm25LexicalCandidateSource::new(vec![chapter("long-a", "甲"), chapter("long-b", "乙")]);
    let pipeline = HybridRetrievalPipeline::default();
    let items = pipeline
        .retrieve("甲乙", &[scope], &[&source], 3, MAX_CHARS)
        .expect("retrieve must not fail");

    let spent: usize = items.iter().map(|item| delivered(&item.content)).sum();
    assert!(
        spent <= MAX_CHARS,
        "按送达量计费，总送达量 {spent} 不许超过预算 {MAX_CHARS}"
    );
}

/// ⭐ **反证**：预算很小时**保底一条**（不许返回空集）。
#[test]
fn a_tiny_budget_still_admits_one() {
    let scope = MemoryScope::Global;
    let source = Bm25LexicalCandidateSource::new(vec![chapter("long-a", "甲")]);
    let pipeline = HybridRetrievalPipeline::default();
    let items = pipeline
        .retrieve("甲", &[scope], &[&source], 3, 9)
        .expect("retrieve must not fail");
    assert!(
        !items.is_empty(),
        "预算只有 9 字时也要**保底装一条**（修复前：一条都装不下，空集）"
    );
}

/// ⭐ **不变式**：**短**记录的既有行为**逐字不变**（这是修复不能破坏的那一半）。
#[test]
fn short_records_keep_their_previous_behaviour() {
    let scope = MemoryScope::Global;
    let source = Bm25LexicalCandidateSource::new(vec![
        candidate("b", "alpha"),
        candidate("a", "beta"),
        candidate("c", "gamma"),
    ]);
    let pipeline = HybridRetrievalPipeline::default();
    let items = pipeline
        .retrieve("", &[scope], &[&source], 3, 9)
        .expect("retrieve must not fail");
    assert_eq!(
        items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"],
        "短记录的选择与顺序**逐字不变**"
    );
    assert_eq!(
        items
            .iter()
            .map(|item| item.content.chars().count())
            .sum::<usize>(),
        9,
        "预算仍然被严格执行"
    );
}
