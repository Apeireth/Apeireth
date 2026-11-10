//! **生产排序口径 == 质量闸门口径**的钉死测试（单一词法通道）。
//!
//! # 为什么需要这一份
//!
//! 词法源自己排序用的是 `lexical + 0.5 × coverage`（**覆盖率融合**）；
//! 而生产走的多路管线为了「各路可比」，会用 `score_components.weighted(...)`
//! **重算一遍**分数 —— 那次重算**只带分量、不带 coverage**
//! ⇒ ⇒ **coverage 被丢掉** ⇒ ⇒ 后果是：
//!
//! * **闸门**（内部卷基线 / 真人卷闸门）量的是**源自己的分数** ⇒ 含 coverage；
//! * **生产**（管线）用的是重算后的分量分 ⇒ **不含** coverage。
//!
//! ⇒ ⇒ 「闸门量的」与「生产做的」**不是同一件事**：闸门变绿**不能**保证生产也变好。
//! 实测（2026-11-07）：真人卷上管线臂比单路词面臂低 **0.0013** recall。
//!
//! # 本测试钉住的口径
//!
//! **单一词法通道**时，管线的排序必须与**源自己的排序逐条同序**
//! （两者只差一个正比例因子）⇒ ⇒ 「闸门量的就是生产做的」。
//!
//! ⚠️ 这是**同序**断言，不是「分数相等」断言 —— 管线会把分数**按路归一化**
//! （量纲对齐），绝对值本来就不该相等。
//!
//! # 中性表述
//!
//! 只描述**机制**（覆盖率融合 / 按路归一化 / 多路融合），不引用任何外部产品名。

use apeireth_memory::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};

fn candidate(id: &str, content: &str) -> MemoryCandidate {
    MemoryCandidate {
        id: id.to_string(),
        layer: "episodic".to_string(),
        scope: MemoryScope::Global,
        content: content.to_string(),
        score: 0.0,
        score_components: Default::default(),
        provenance: MemoryProvenance::default(),
    }
}

/// ⭐⭐ **单词法通道时：管线排序必须与源排序逐条同序**。
///
/// 语料刻意构造成「**BM25 高但覆盖率低**」与「**BM25 略低但覆盖率全**」并存
/// ⇒ ⇒ 没有 coverage 时两者顺序会**分岔** ⇒ ⇒ 这条断言才有鉴别力
/// （不是恒绿断言）。
#[test]
fn the_pipeline_ranks_one_lexical_route_exactly_like_the_source_does() {
    let documents = vec![
        // 同一个词重复很多次 ⇒ BM25（词频饱和后）不低，但**覆盖率只有 1/3**。
        candidate("repeat", "alpha alpha alpha alpha alpha alpha alpha alpha"),
        // 三个查询词各一次 ⇒ BM25 是三个词之和（最高），覆盖率 3/3。
        candidate("all", "alpha beta gamma"),
        // 覆盖 2/3。
        candidate("two", "beta beta gamma gamma"),
        // 覆盖 1/3，但词频高。
        candidate("beta_repeat", "beta beta beta beta beta beta beta beta"),
        // 覆盖 1/3。
        candidate("gamma_only", "gamma"),
    ];
    let source = Bm25LexicalCandidateSource::new(documents);
    let query = "alpha beta gamma";

    let source_order: Vec<String> = source
        .candidates(query, 64)
        .expect("source candidates")
        .into_iter()
        .map(|item| item.id)
        .collect();

    let pipeline = HybridRetrievalPipeline::default();
    let pipeline_order: Vec<String> = pipeline
        .retrieve(
            query,
            &[MemoryScope::Global],
            &[&source],
            source_order.len(),
            1_000_000,
        )
        .expect("retrieve")
        .into_iter()
        .map(|item| item.id)
        .collect();

    assert_eq!(
        pipeline_order, source_order,
        "单词法通道时，管线排序必须与源排序同序：\n  源   = {source_order:?}\n  管线 = {pipeline_order:?}\n（不一致 ⇒ 覆盖率融合在管线里被丢了 ⇒ 闸门量的与生产做的不是一回事）"
    );
}
