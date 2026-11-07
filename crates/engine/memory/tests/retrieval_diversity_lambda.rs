//! 「相关性-多样性折中」（MMR 重排）的 **λ 语义**集成测试。
//!
//! # 为什么需要这一份
//!
//! 生产检索管线在融合排序之后有一道**残差式多样性重排**：逐轮按
//! `(1-λ)·相关度 + λ·新颖度` 贪心挑选，新颖度 = `1 − 与已选项的最大词面相似度`。
//! 旋钮是 `MemoryRankingConfig::diversity_lambda`。
//!
//! ⭐ **默认是 0.0（关闭）**，而且这是**实测决定的**（不是默认值随手选的）：
//! 内部两卷的 λ 扫描是**单调下降**的（`λ=0.2` 相对 `λ=0` 的 recall@10
//! 掉 **−5.19% / −2.89%**；`λ=1` 灾难性）⇒ ⇒ 默认不能自损；
//! 需要「去冗余展示」的调用方**显式**给 `λ > 0`。
//!
//! ⇒ 这一份测试钉的是**语义**（λ 到底怎么改行为），不是「跑不跑得过」。
//! 覆盖边界：λ=0 只相关度且**保留正文去重** · λ=1 首条确定、次条取最不相似 ·
//! 默认 λ 不做新颖度压制 · 空/单条/全同边界不 panic · 测试源默认**非填充源**。
//!
//! # 中性表述
//!
//! 只描述**机制**（相关性-多样性折中 / 贪心重排 / 词面相似度），
//! 不引用任何外部产品名、文档或代码。

use apeireth_memory::{
    BasicLexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidate, MemoryCandidateSource,
    MemoryProvenance, MemoryRankingConfig, MemoryScope, ScoreComponents,
};

/// 构造一个候选（分数由源自己算，这里只给内容与 id）。
fn candidate(id: &str, content: &str) -> MemoryCandidate {
    MemoryCandidate {
        id: id.to_string(),
        layer: "episodic".to_string(),
        scope: MemoryScope::Global,
        content: content.to_string(),
        score: 0.0,
        score_components: ScoreComponents::default(),
        provenance: MemoryProvenance::default(),
    }
}

/// 跑一次检索 ⇒ 返回 id 序列。
fn ids_for(lambda: f64, query: &str, documents: Vec<MemoryCandidate>, limit: usize) -> Vec<String> {
    let scope = MemoryScope::Global;
    let source = BasicLexicalCandidateSource::new(documents);
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig {
        diversity_lambda: lambda,
        ..MemoryRankingConfig::default()
    });
    pipeline
        .retrieve(query, &[scope], &[&source], limit, 100_000)
        .expect("retrieve")
        .into_iter()
        .map(|item| item.id)
        .collect()
}

/// ⭐⭐ **λ = 0 ⇒ 只按相关度选，但「正文完全相同」的去重仍然保留**。
///
/// 口径：`diversity_lambda = 0` 表示**不做相关性-多样性折中**，
/// **不等于**「允许重复条目」——正文归一化后完全相同的候选**只进一条**。
/// ⇒ 这是两件不同的事（去重 vs 折中），测试要把它们分开钉住。
#[test]
fn lambda_zero_keeps_relevance_only_but_still_dedupes_identical_content() {
    let documents = vec![
        candidate("a", "alpha beta"),
        candidate("d", "alpha beta"), // 与 a 正文完全相同
        candidate("c", "alpha gamma"),
    ];
    let ids = ids_for(0.0, "alpha beta", documents, 3);
    assert_eq!(
        ids,
        vec!["a".to_string(), "c".to_string()],
        "λ=0：正文完全相同的候选必须只进一条（去重保留），其余按相关度"
    );
}

/// ⭐⭐ **λ = 1 ⇒ 首条按确定性取，第二条取「与首条最不相似」的**。
///
/// 这是折中的极端侧：完全不看相关度，只看新颖度。
/// ⇒ 断言它与 λ=0 的结果**不同**，且**不同在预期的那一条**上
/// （近似重复的 b 被压后，不同的 c 提前）。
#[test]
fn lambda_one_picks_least_similar_second_entry() {
    let documents = vec![
        candidate("a", "alpha beta"),
        candidate("b", "alpha beta extra"), // 与 a 近似重复
        candidate("c", "alpha gamma"),      // 与 a 差别大
    ];
    let relevance_order = ids_for(0.0, "alpha beta", documents.clone(), 3);
    assert_eq!(
        relevance_order,
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        "λ=0：纯相关度排序（a/b 同分按 id 稳定）"
    );

    let diverse_order = ids_for(1.0, "alpha beta", documents, 3);
    assert_eq!(
        diverse_order,
        vec!["a".to_string(), "c".to_string(), "b".to_string()],
        "λ=1：首条仍按确定性取 a，第二条必须取与 a 最不相似的 c（近似重复的 b 被压后）"
    );
}

/// ⭐ **默认 λ 必须不做新颖度压制**（默认 0.0 的行为面）。
///
/// 为什么要这条：默认值一旦被无声改回 0.20，检索结果会**静默变样**
/// （近似重复被压后），而这正是 2026-11-07 实测认定为**净损失**的档位。
#[test]
fn default_lambda_does_not_suppress_near_duplicates() {
    let documents = vec![
        candidate("a", "alpha beta"),
        candidate("b", "alpha beta extra"),
        candidate("c", "alpha gamma"),
    ];
    let ids = ids_for(
        MemoryRankingConfig::default().diversity_lambda,
        "alpha beta",
        documents,
        3,
    );
    assert_eq!(
        ids,
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        "默认 λ（0.0）⇒ 不做新颖度压制：结果与纯相关度一致"
    );
}

/// ⭐ **边界：空源 / 单条 / 全同正文** ⇒ 都不 panic，且结果可预期。
#[test]
fn degenerate_inputs_behave_predictably() {
    // 空源 ⇒ 空结果。
    assert!(
        ids_for(0.5, "alpha", Vec::new(), 5).is_empty(),
        "空源必须返回空结果，不能 panic"
    );

    // 单条 ⇒ 照常返回。
    assert_eq!(
        ids_for(0.5, "alpha", vec![candidate("solo", "alpha beta")], 5),
        vec!["solo".to_string()],
        "单条候选必须原样返回"
    );

    // 全同正文 ⇒ 去重后只剩一条。
    let same = vec![
        candidate("s1", "alpha beta"),
        candidate("s2", "alpha beta"),
        candidate("s3", "alpha beta"),
    ];
    assert_eq!(
        ids_for(0.5, "alpha beta", same, 5),
        vec!["s1".to_string()],
        "正文全同 ⇒ 去重后只进一条"
    );
}

/// ⭐ **测试源默认**非填充源**、权重默认 1.0**。
///
/// 这两条是 trait 的**默认方法** ⇒ ⇒ ⇒ 既有实现零改动。
/// 这里钉住它们，是因为「填充源」会在融合时被映射到主力正分候选最小分之下
/// ⇒ ⇒ 误标成填充源会**静默改变**该路的排序地位。
#[test]
fn a_plain_test_source_is_not_a_filler_route() {
    let source = BasicLexicalCandidateSource::new(vec![candidate("x", "alpha")]);
    assert!(!source.is_filler(), "普通源默认不是填充通道（默认 false）");
    assert_eq!(source.fusion_weight(), 1.0, "普通源默认融合权重是 1.0");
}
