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
//!
//! # ⚠️ 2026-11-07：**真根因**修正（**不是**放宽断言）
//!
//! 本模块原来有一条测试比的是 `recall()` 返回的**最终 `score`**：
//! 「同一条记忆在『3年前』下的最终分 **>** 在『1年前』下的最终分」。
//!
//! ⇒ ⇒ ⇒ **那条断言结构上恒假** ⇒ ⇒ **与日期无关**（⇒ ⇒ **不是 date-bomb**）：
//!
//! * 生产管线的最终 `score` 是**「按路归一化」后的相对量**
//!   （`crates/engine/memory/src/retrieval_pipeline.rs`：`normalized = raw / source_max`）
//!   ⇒ ⇒ **每个问句的榜首恒等于 1.0**；
//! * 「1年前」下老那条与新那条**同分**
//!   （recency 双双被 clamp 到 0.1、词面分打平）
//!   ⇒ ⇒ 老那条**也是榜首** ⇒ ⇒ 也是 1.0 ⇒ ⇒ ⇒ `1.0 > 1.0` ⇒ **恒假**。
//!
//! ⇒ ⇒ **正解**：断言**跨问句可比**的量 ——
//!
//! 1. **recency 分量**（绝对量纲 `[0.1, 1.0]`）⇒ 这就是锚点**直接移动**的那个分量；
//! 2. **最终分数的「两条之差」**（同一个问句**内部**的差 ⇒ 跨问句可比）。
//!
//! ⇒ ⇒ 两条都**严格** ⇒ ⇒ ⇒ **不是放宽**（比原来的断言更贴近产品语义）：
//! ⇒ ⇒ 锚点若被摘掉（`recency_reference_ms = None`）⇒ ⇒ recency 双双 0.1 ⇒ ⇒ **必红**。

use std::sync::Arc;

use apeireth_memory::anchor_injection::{
    anchor_reference_time_ms, parse_time_anchors, TimeAnchor, TimeUnit,
};
use apeireth_memory::{
    MemoryCoordinator, MemoryGovernanceStore, MemoryLayerKind, MemoryRecallQuery,
    MemoryWritebackEntry, SqliteMemoryStore,
};
use apeireth_storage::SqliteConnectionPool;
use tempfile::tempdir;

/// 「现在」（**固定** ⇒ 不依赖真实时钟）。
///
/// ⚠️ **必须取在「墙钟之前」**（2026-11-07 修正）：本模块写的两条记忆都以它为基准
/// ⇒ ⇒ 若基准落在**未来**（原值 2027-01-15），那么在真实时钟追上它之前
/// ⇒ ⇒ 这些记忆就是「**未来记录**」⇒ ⇒ 任何「按墙钟过滤未来记录」的实现
/// ⇒ ⇒ 都会把它们滤掉 ⇒ ⇒ ⇒ 那才是**真·date-bomb**。
/// ⇒ ⇒ 取一个**已经过去**的固定时刻 ⇒ ⇒ 两条记忆永远是「过去」
/// ⇒ ⇒ ⇒ 结构上无此风险（⇒ ⇒ 与真实时钟**彻底无关**）。
const NOW_MS: i64 = 1_700_000_000_000; // 2023-11-14T22:13:20Z
/// 三年前（以 365 天/年计 ⇒ 与 `anchor_injection::TimeUnit::Year` **同口径**）。
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

/// ⭐ **相对时间锚点**必须按**注入的基准时刻**解析 ⇒ ⇒ **不读墙钟**。
///
/// # 为什么这条必须**单独存在**
///
/// 「问句说『3年前』⇒ 参照点 = 现在往前 3 年」这句话里的「现在」有两种可能：
///
/// * **注入的基准时刻**（`as_of_ms`）⇒ ⇒ **可复现**（本模块的口径）；
/// * **墙钟**（`chrono::Utc::now()`）⇒ ⇒ ⇒ **随日期漂移**（真·date-bomb）。
///
/// ⇒ ⇒ 这里把两者**分开断言**：
/// * `anchor_reference_time_ms(NOW_MS, …)` ⇒ ⇒ **基准是注入值**（**逐字可算**）；
/// * 且**接线层**真的把参照点用上了（见
///   [`the_anchor_changes_the_scores_not_just_the_order`]：老那条的 recency 分量
///   **正好是 1.0** ⇒ ⇒ 它**落在参照点上** ⇒ ⇒ 参照点 = `NOW_MS - 3 年`
///   ⇒ ⇒ ⇒ **若哪天改成读墙钟，这条与那条会同时红** ⇒ ⇒ ⇒ 漂移被**当场抓住**）。
#[test]
fn the_relative_anchor_resolves_against_the_injected_base_instant() {
    let anchors = parse_time_anchors("3年前我們記了什麼？");
    assert_eq!(
        anchors,
        vec![TimeAnchor::Relative {
            amount: 3,
            unit: TimeUnit::Year
        }],
        "「3年前」应解析成**唯一**的相对锚点（相对量 3 · 单位 年）"
    );
    assert_eq!(
        anchor_reference_time_ms(NOW_MS, &anchors),
        Some(NOW_MS - THREE_YEARS_MS),
        "参照时间必须由**注入的基准时刻**推出（⇒ ⇒ 与墙钟无关 ⇒ ⇒ 不随日期漂移）"
    );
}

/// 一条召回结果里**这个测试要看**的三个量。
struct Scored {
    /// 记忆 id。
    id: String,
    /// **最终分**。
    ///
    /// ⚠️ 这是**「按路归一化」后**的**相对量**（`retrieval_pipeline.rs`：
    /// `normalized = raw / source_max`）⇒ ⇒ **单个问句内部**可比，
    /// ⇒ ⇒ **跨问句**不可比（⇒ ⇒ 每个问句的榜首恒为 1.0）。
    score: f64,
    /// **recency 分量**（绝对量纲 `[0.1, 1.0]`）⇒ ⇒ **跨问句可比**。
    recency: f64,
}

/// 召回结果（含**最终分**与 **recency 分量**）⇒ 用于**确定性断言**。
fn recall_items(coord: &MemoryCoordinator, session: &str, query: &str) -> Vec<Scored> {
    coord
        .recall(&query_of(session, query))
        .expect("recall 应成功")
        .items
        .into_iter()
        .map(|item| Scored {
            recency: item
                .score_components
                .map(|components| components.recency)
                .unwrap_or(f64::NAN),
            id: item.id,
            score: item.score,
        })
        .collect()
}

/// 「老那条」的 **recency 分量**（⇒ ⇒ **跨问句可比**）。
fn recency_of(items: &[Scored], old: &str) -> f64 {
    items
        .iter()
        .find(|item| item.id == old)
        .map(|item| item.recency)
        .unwrap_or(f64::NAN)
}

/// 「老那条」与「另一条」的**最终分之差**。
///
/// ⚠️ 取的是**同一个问句内部**的差 ⇒ ⇒ **跨问句可比**
/// ⇒ ⇒ （而**不是**拿「老那条的最终分」跨问句直接比 —— 那是**归一化后的相对量**
/// ⇒ ⇒ ⇒ 跨问句比它**结构上恒假** ⇒ ⇒ ⇒ 见模块文档的「真根因」段）。
fn score_gap(items: &[Scored], old: &str) -> f64 {
    let old_score = items
        .iter()
        .find(|item| item.id == old)
        .map(|item| item.score)
        .unwrap_or(f64::NAN);
    let other_score = items
        .iter()
        .find(|item| item.id != old)
        .map(|item| item.score)
        .unwrap_or(f64::NAN);
    old_score - other_score
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
    //
    // ⚠️ **2026-11-07 修正断言的观测量**（**不是放宽**）：
    // ⇒ 比的**不是**最终 `score`（**按路归一化后的相对量** ⇒ 每个问句榜首恒为 1.0
    // ⇒ ⇒ ⇒ 跨问句比它**结构上恒假** ⇒ ⇒ ⇒ 恒红），而是**跨问句可比**的两个量：
    // ⇒ ① **recency 分量**（绝对量纲 `[0.1, 1.0]`）；
    // ⇒ ② **最终分数的「两条之差」**（问句**内部**的差）。
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "anchor-session";
    write_pair(&path, session);
    let coord = coordinator(&path);
    let old = older_id(&coord, session);

    let at_three_years = recall_items(&coord, session, "3年前我們記了什麼？");
    let at_one_year = recall_items(&coord, session, "1年前我們記了什麼？");

    // ① ⭐ **recency 分量**（锚点**直接移动**的那个分量 ⇒ **跨问句可比**）：
    // ⇒ 「3年前」⇒ 老那条**离参照点 0** ⇒ 分量 = 1.0；
    // ⇒ 「1年前」⇒ 老那条**离参照点 2 年** ⇒ 分量被 clamp 到 0.1。
    let three_year_recency = recency_of(&at_three_years, &old);
    let one_year_recency = recency_of(&at_one_year, &old);
    assert!(
        three_year_recency > one_year_recency,
        "老那条的 recency 分量：说「3年前」⇒ {three_year_recency} ⇒ 应**高于**说「1年前」⇒ {one_year_recency}"
    );
    // ⇒ ⇒ 且「3年前」下老那条**正好落在参照点上** ⇒ ⇒ 分量 = 1.0
    // ⇒ ⇒ ⇒ **这同时证明参照点是由「注入的基准时刻」推出的**（不是墙钟）
    assert!(
        (three_year_recency - 1.0).abs() < 1e-12,
        "说「3年前」⇒ 老那条应**正好落在参照点**上 ⇒ recency 分量 = 1.0（实际 {three_year_recency}）"
    );

    // ② **最终分数的「两条之差」**（问句**内部**的差 ⇒ **跨问句可比**）：
    // ⇒ 「3年前」⇒ 老那条**明确压过**新那条（差 > 0）；
    // ⇒ 「1年前」⇒ 两条**同分**（差 = 0）⇒ ⇒ ⚠️ **不比较谁在前**
    // ⇒ ⇒ ⇒ （同分 ⇒ 顺序由 **uuid** 决定 ⇒ ⇒ 断言顺序 = **天然 flaky**）。
    let three_year_gap = score_gap(&at_three_years, &old);
    let one_year_gap = score_gap(&at_one_year, &old);
    assert!(
        three_year_gap > one_year_gap,
        "老那条与新那条的分数差：说「3年前」⇒ {three_year_gap} ⇒ 应**高于**说「1年前」⇒ {one_year_gap}"
    );
    assert!(
        three_year_gap > 0.0,
        "说「3年前」⇒ 老那条的最终分应**明确高于**新那条（实际差 {three_year_gap}）"
    );

    // ⇒ ⇒ 而且两条的**相对顺序**在「3年前」下是**确定的**（老的那条在前）
    // ⇒ ⇒ ⇒ （「3年前」下两条**不同分** ⇒ ⇒ ⇒ 顺序由分数决定 ⇒ ⇒ 与 uuid 无关）
    assert_eq!(
        at_three_years.first().map(|item| item.id.as_str()),
        Some(old.as_str()),
        "说「3年前」⇒ 老的那条应排最前（实际 {:?}）",
        at_three_years
            .iter()
            .map(|item| (item.id.as_str(), item.score))
            .collect::<Vec<_>>()
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
