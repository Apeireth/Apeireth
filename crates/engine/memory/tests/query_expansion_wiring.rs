//! 查询扩展（`query_expand`）在检索主路径上的接线语义。
//!
//! # 这条接线为什么长这样
//!
//! `query_expand` 的文档说「调用方把 `ExpandedQuery::rewritten` 传给混合检索」。
//! 但检索打分是**各项之和**（BM25 逐词累加，无查询长度归一化）——
//! 若把扩展后的整句直接当查询，「匹配到的同义/聚类词多」的文档会累出更高分，
//! 反而压过精确匹配那一篇。**加词本意是提召回，不该降精度。**
//!
//! 所以接线做成**召回增强**：只在主查询召回不足 `limit` 时补跑一遍扩展查询，
//! 再按 id 并集去重（同 id 保留更高分）。于是：
//!
//! - 主查询够用 ⇒ 完全走原路径，零行为变化；
//! - 主查询稀疏 ⇒ 扩展补召回，精确匹配那篇的分数不会被压低（并集只增不减）。
//!
//! 下面三条测试分别钉住这三件事。

use std::sync::Arc;

use apeireth_memory::{
    MemoryCoordinator, MemoryGovernanceStore, MemoryLayerKind, MemoryRecallQuery,
    MemoryWritebackEntry, SqliteMemoryStore,
};
use tempfile::tempdir;

fn coordinator(path: &std::path::Path) -> MemoryCoordinator {
    let store = Arc::new(SqliteMemoryStore::open(path).expect("open SQLite file"));
    let backend = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store;
    MemoryCoordinator::new(backend, governance)
}

fn recall(
    coordinator: &MemoryCoordinator,
    session: &str,
    query: &str,
    limit: usize,
) -> apeireth_memory::MemoryRecallResult {
    let mut request = MemoryRecallQuery::new(session, query)
        .with_layers(vec![MemoryLayerKind::Episodic])
        .with_limit(limit)
        .with_as_of_ms(1_800_000_000_000);
    // 下限归零：本测试关心的是**召回集合**，不是打分阈值
    request.min_score = 0.0;
    coordinator.recall(&request).expect("recall should succeed")
}

/// 主查询召回不足时，扩展把**同义词簇**里的记忆也捞回来。
#[test]
fn sparse_primary_recall_is_augmented_by_expansion() {
    let dir = tempdir().expect("temporary directory");
    let path = dir.path().join("query-expand-augment.sqlite");
    let session = "expand-session";
    let coordinator = coordinator(&path);

    // 两篇记忆分属同一个扩展簇（「考试 / 备考 / 复习 / 线代 / 高数」）
    coordinator
        .writeback(&MemoryWritebackEntry::new(
            session,
            "user",
            "明天要考试了有点紧张",
        ))
        .expect("writeback");
    coordinator
        .writeback(&MemoryWritebackEntry::new(
            session,
            "user",
            "线代第三章还没复习完",
        ))
        .expect("writeback");

    // 主查询「考试」只会命中第一篇的精确词；limit 给 5 ⇒ 召回不足 5 篇
    // ⇒ 触发扩展补召回，第二篇（只含「线代/复习」）应被捞回来。
    let expanded = apeireth_memory::query_expand::expand_query("考试");
    assert!(
        expanded.extra_terms.iter().any(|term| term == "线代"),
        "触发条件：短查询必须能产出同簇词: {expanded:?}"
    );
    let recalled = recall(&coordinator, session, "考试", 5);
    let texts: Vec<&str> = recalled
        .items
        .iter()
        .map(|item| item.content.as_str())
        .collect();
    assert!(
        texts.iter().any(|text| text.contains("考试")),
        "精确匹配那篇必须在: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.contains("线代")),
        "扩展应把同簇的「线代/复习」那篇也捞回来（召回增强）: {texts:?}"
    );
}

/// 主查询召回已满 `limit` 时**不走扩展** —— 零行为变化。
///
/// 证明方式：造 3 篇都含精确词「考试」的记忆，`limit = 1`。
/// 若扩展照跑并参与合并，簇内其它词（线代/复习）也会带进候选；
/// 这里断言结果恰好只回 1 篇（limit 生效）且是精确匹配那篇。
#[test]
fn saturated_primary_recall_is_unchanged() {
    let dir = tempdir().expect("temporary directory");
    let path = dir.path().join("query-expand-saturated.sqlite");
    let session = "saturated-session";
    let coordinator = coordinator(&path);

    for text in ["考试焦虑怎么办", "考试前要早睡", "考试当天带准考证"] {
        coordinator
            .writeback(&MemoryWritebackEntry::new(session, "user", text))
            .expect("writeback");
    }

    let recalled = recall(&coordinator, session, "考试", 1);
    assert_eq!(recalled.items.len(), 1, "limit 必须生效: {recalled:?}");
    assert!(
        recalled.items[0].content.contains("考试"),
        "唯一一条也必须是精确匹配那篇"
    );
}

/// 查询里**没有**任何扩展簇成员时，扩展不产出额外词 ⇒ 走原路径。
#[test]
fn query_without_cluster_members_is_left_alone() {
    let dir = tempdir().expect("temporary directory");
    let path = dir.path().join("query-expand-none.sqlite");
    let session = "no-cluster-session";
    let coordinator = coordinator(&path);

    coordinator
        .writeback(&MemoryWritebackEntry::new(
            session,
            "user",
            "数据库迁移脚本已经跑通",
        ))
        .expect("writeback");

    // 「迁移脚本」不在任何扩展簇里 ⇒ 扩展产出零额外词，路径与接线前一致。
    let recalled = recall(&coordinator, session, "数据库 迁移 脚本", 5);
    assert!(
        recalled
            .items
            .iter()
            .any(|item| item.content.contains("迁移")),
        "原查询本身应命中: {recalled:?}"
    );

    let expanded = apeireth_memory::query_expand::expand_query("数据库 迁移 脚本");
    assert!(
        expanded.extra_terms.is_empty(),
        "不在任何簇里就不该造词: {expanded:?}"
    );
    assert_eq!(
        expanded.rewritten, "数据库 迁移 脚本",
        "无扩展时重写串等于原串"
    );
}
