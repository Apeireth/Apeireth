//! **时间锚点的生产接线证据** —— 「3年前」真的改变排序吗？
//!
//! # 这个测试证明什么
//!
//! 2026-10-27 的实测（`docs/status/chapter-anchor-value-2026-10-27.md`）表明：
//! **用户问句里的结构化信息**（回数 / 相对时间）**在生产检索里原本被浪费**。
//!
//! ⇒ 本模块把「**相对时间锚点**」接进了生产的 recency 计算：
//! ⇒ **问句说「3年前」⇒ recency 的参照时间改成「3年前」**（而不是「现在」）。
//!
//! # 三条纪律
//!
//! 1. **零行为变化**：**没有锚点** ⇒ 排序与接线前**逐字一致**
//!    ⇒ ⇒ 这条必须被**单独证明**（否则「顺手改坏了」没人知道）；
//! 2. **锚点真的生效**：说「3年前」⇒ **旧的那条排前面**；
//! 3. **确定性**：用 `as_of_ms` 固定「现在」⇒ ⇒ 不依赖真实时钟。

use std::sync::Arc;

use apeireth_memory::{
    MemoryCoordinator, MemoryGovernanceStore, MemoryLayerKind, MemoryRecallQuery,
    MemoryWritebackEntry, SqliteMemoryStore,
};
use apeireth_storage::SqliteConnectionPool;
use tempfile::tempdir;

/// 「现在」（固定 ⇒ 不依赖真实时钟）。
const NOW_MS: i64 = 1_800_000_000_000;
/// 三年前（以 365 天/年计）。
const THREE_YEARS_MS: i64 = 3 * 365 * 24 * 60 * 60 * 1000;

fn coordinator(path: &std::path::Path) -> MemoryCoordinator {
    let store = Arc::new(SqliteMemoryStore::open(path).expect("open SQLite file"));
    let backend = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store;
    MemoryCoordinator::new(backend, governance)
}

/// 写入**两条内容相同**、**时间不同**的记忆（⇒ 词面分打平 ⇒ 只剩 recency 能分胜负）。
fn write_pair(path: &std::path::Path, session: &str) {
    let coord = coordinator(path);
    // ⚠️ **两条的内容必须不同**（实测踩到：**内容完全相同会被去重**
    // ⇒ ⇒ 只剩一条 ⇒ ⇒ 「老的那条排前面」根本**测不到**）
    // ⇒ ⇒ 但**保留同一个判别关键词**（"共享主题"）⇒ ⇒ 词面分基本可比。
    let mut recent = MemoryWritebackEntry::new(session, "user", "共享主题的记忆内容甲");
    recent.timestamp_ms = Some(NOW_MS - 60_000);
    coord.writeback(&recent).expect("写入近期记忆");

    let mut old = MemoryWritebackEntry::new(session, "user", "共享主题的记忆内容乙");
    old.timestamp_ms = Some(NOW_MS - THREE_YEARS_MS);
    coord.writeback(&old).expect("写入三年前的记忆");
}

/// 造一个**确定性**的召回查询。
///
/// ⚠️ **两个参数必须显式设**（否则测试会「碰巧」通过）：
/// * `recency_decay_lambda` ⇒ 固定衰减率；
/// * `min_score = 0.0` ⇒ **两条都要能被召回**
///   ⇒ 否则 3 年前那条的 recency ≈ 0.1 ⇒ **低于默认 min_score 0.10**
///   ⇒ ⇒ 压根不返回 ⇒ ⇒ 「老的那条排前面」就**测不到**。
fn query_of(session: &str, text: &str) -> MemoryRecallQuery {
    let mut query = MemoryRecallQuery::new(session, text)
        .with_layers(vec![MemoryLayerKind::Episodic])
        .with_limit(10)
        .with_as_of_ms(NOW_MS);
    query.recency_decay_lambda = 0.05;
    query.min_score = 0.0;
    query
}

fn recall_ids(coord: &MemoryCoordinator, session: &str, query: &str) -> Vec<String> {
    coord
        .recall(&query_of(session, query))
        .expect("recall 应成功")
        .items
        .into_iter()
        .map(|item| item.id)
        .collect()
}

/// 找出**时间戳较旧**那条的 id。
fn older_id(coord: &MemoryCoordinator, session: &str) -> String {
    let result = coord
        .recall(&query_of(session, "共享主题"))
        .expect("recall 应成功");
    let mut items = result.items;
    items.sort_by_key(|item| item.timestamp_ms);
    items.first().map(|item| item.id.clone()).expect("应有记忆")
}

#[test]
fn without_anchor_the_newest_memory_ranks_first() {
    // ⇒ **基线**：没有锚点 ⇒ recency 以「现在」为参照 ⇒ 近期的那条在前。
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "anchor-session";
    write_pair(&path, session);
    let coord = coordinator(&path);
    let ids = recall_ids(&coord, session, "共享主题");
    assert!(ids.len() >= 2, "应召回两条：{ids:?}");
    let old = older_id(&coord, session);
    assert_eq!(
        ids.first().map(String::as_str),
        Some(ids[0].as_str()),
        "占位断言（保证 ids 非空）"
    );
    assert_ne!(
        ids.first().map(String::as_str),
        Some(old.as_str()),
        "无锚点时**近期**的那条应在前（老的不该在前）"
    );
}

#[test]
fn a_relative_time_anchor_moves_the_old_memory_to_the_front() {
    // ⇒ 🎯 **锚点真的生效**：说「3年前」⇒ recency 参照时间变成 3 年前
    // ⇒ ⇒ **旧的那条**排到前面。
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "anchor-session";
    write_pair(&path, session);
    let coord = coordinator(&path);
    let old = older_id(&coord, session);

    let ids = recall_ids(&coord, session, "3年前我們記了什麼？");
    assert!(ids.len() >= 2, "应召回两条：{ids:?}");
    assert_eq!(
        ids.first().map(String::as_str),
        Some(old.as_str()),
        "说「3年前」⇒ **旧的那条**应排到最前（实际 {:?}）",
        ids
    );
}

/// 召回结果（含**分数**）⇒ 用于**确定性断言**。
fn recall_items(coord: &MemoryCoordinator, session: &str, query: &str) -> Vec<(String, f64)> {
    coord
        .recall(&query_of(session, query))
        .expect("recall 应成功")
        .items
        .into_iter()
        .map(|item| (item.id, item.score))
        .collect()
}

#[test]
fn the_anchor_changes_the_scores_not_just_the_order() {
    // ⇒ **对照 + 修 flaky**（2026-10-27 实测踩到）：
    // ⇒ 「1年前」⇒ 两条记忆**离参照点都很远**（1 年 vs 2 年）
    // ⇒ ⇒ recency **几乎相同** ⇒ ⇒ 排序会落到 **id 顺序**上
    // ⇒ ⇒ **id 是随机 uuid** ⇒ ⇒ 断言「谁在前」= **天然 flaky**。
    //
    // ⇒ ⇒ 改成断言**分数**（**确定性**）：
    // ⇒ 「3年前」⇒ 老那条**离参照点 0** ⇒ 分数**高**；
    // ⇒ 「1年前」⇒ 老那条**离参照点 2 年** ⇒ 分数**低**。
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "anchor-session";
    write_pair(&path, session);
    let coord = coordinator(&path);
    let old = older_id(&coord, session);

    let at_three_years = recall_items(&coord, session, "3年前我們記了什麼？");
    let at_one_year = recall_items(&coord, session, "1年前我們記了什麼？");
    let score_of = |items: &[(String, f64)], id: &str| {
        items
            .iter()
            .find(|(item_id, _)| item_id == id)
            .map(|(_, score)| *score)
            .unwrap_or(f64::NAN)
    };

    let three_year_score = score_of(&at_three_years, &old);
    let one_year_score = score_of(&at_one_year, &old);
    assert!(
        three_year_score > one_year_score,
        "老那条的分数：说「3年前」⇒ {three_year_score} ⇒ 应**高于**说「1年前」⇒ {one_year_score}"
    );
    // ⇒ ⇒ 而且两条的**相对顺序**在「3年前」下是**确定的**（老的那条在前）
    assert_eq!(
        at_three_years.first().map(|(id, _)| id.as_str()),
        Some(old.as_str()),
        "说「3年前」⇒ 老的那条应排最前（实际 {:?}）",
        at_three_years
    );
}

#[test]
fn a_question_without_anchors_keeps_the_baseline_order() {
    // ⇒ **零行为变化**：问句里没有锚点 ⇒ 排序与「接线前」一致
    // ⇒ ⇒ 这条是「顺手改坏了」的**唯一护栏**。
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "anchor-session";
    write_pair(&path, session);
    let coord = coordinator(&path);
    let baseline = recall_ids(&coord, session, "共享主题");
    let with_noise = recall_ids(&coord, session, "共享主题 記憶 內容");
    assert_eq!(
        baseline, with_noise,
        "没有时间锚点的问句 ⇒ 排序应完全一致（**零行为变化**）"
    );
}
