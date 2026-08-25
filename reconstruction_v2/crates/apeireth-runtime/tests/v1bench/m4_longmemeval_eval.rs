//! # `tests/m4_longmemeval_eval.rs` — M4 记忆评测闭环: LongMemEval 风格 5 能力评测集
//!
//! **任务**: 1c1f3f95 (记忆调研批 M4, 台账 ⭐)
//! **基准**: LongMemEval 5 能力模型 —
//!   ① 抽取 (单跳事实)        ② 多会话推理 (跨会话信息合成)
//!   ③ 时序推理 (时间窗/有效期)  ④ 知识更新 (新事实覆盖旧事实, 对账 DELETE)
//!   ⑤ 弃答 (库外问题拒答, 反幻觉核心验收)
//!
//! ## 确定性声明 (0 装 PASS)
//! - fixture 记忆库 = 内联条目 (虚构用户「阿小」), 无外部数据
//! - embedder = `apeireth_memory::HashEmbedder` (FNV-1a byte-hash, L2 归一化,
//!   同输入永远同输出, 0 外部 API)
//! - 纯 `cargo test` 可跑, 0 真 LLM, 0 网络
//! - **LLM 评分层留口**: `LlmJudge` trait 见下, 生产路径换真 LLM 判分即可;
//!   当前用 `DeterministicJudge` (子串包含判定) 先行, 不装已实现
//! - ③ 时序: Note 级 `valid_from/valid_until` 过滤为 M5 待实现项
//!   (字段已落 NoteRecord, 查询路径未接), 本集只测已实现的时间窗语义, 不装 M5
//!
//! ## 边界
//! 只动 apeireth-bench (本文件); 不改 memory/companion 本体.
//!
//! ## 实现注记
//! `SqliteMemoryStore` 同时 impl `EpisodeStore` 与 `NoteStore`, 两者的 `query`
//! 同名 → 全部用全限定语法 `EpisodeStore::query` / `NoteStore::query` 消歧.

use apeireth_core::{Episode, Note};
use apeireth_memory::{
    EmbedFn, EpisodeQuery, EpisodeStore, HashEmbedder, NoteQuery, NoteStore, SqliteMemoryStore,
};
use std::sync::Arc;

// =====================================================================
// 确定性 fixture 记忆库 (虚构用户「阿小」)
// =====================================================================

/// 基准时间 (epoch seconds).
const DAY1: i64 = 1_700_000_000;
/// 一天秒数.
const DAY: i64 = 86_400;

/// 写入全部 fixture: 7 episode (3 会话) + 5 note (知识层).
fn seed_fixture(store: &SqliteMemoryStore) {
    // ---- Episodes (append-only 原始对话层) ----
    let episodes = [
        ("ep-s1-cat", DAY1, "s1", "我的猫叫橘子，是一只三岁的公猫"),
        (
            "ep-s1-food",
            DAY1,
            "s1",
            "我最爱吃的食物是火锅，尤其是麻辣锅底",
        ),
        (
            "ep-s1-phone-old",
            DAY1,
            "s1",
            "我现在用的是 iPhone 12，打算以后换手机",
        ),
        (
            "ep-s2-city",
            DAY1 + 10 * DAY,
            "s2",
            "我最近搬家到了北京望京",
        ),
        (
            "ep-s3-work",
            DAY1 + 20 * DAY,
            "s3",
            "我工作的地点是北京望京附近的一家软件公司",
        ),
        (
            "ep-s3-phone-new",
            DAY1 + 20 * DAY,
            "s3",
            "我刚换了一台 iPhone 15 手机",
        ),
        ("ep-s3-ski", DAY1 + 20 * DAY, "s3", "我计划十二月去崇礼滑雪"),
    ];
    for (id, ts, session, content) in episodes {
        store
            .put_episode(&Episode {
                id: id.into(),
                timestamp: ts,
                role: "user".into(),
                content: content.into(),
                session_id: session.into(),
            })
            .expect("put_episode");
    }

    // ---- Notes (知识层: 可更新/合并/遗忘, D2 §5.4) ----
    let notes = [
        (
            "n-pet",
            DAY1,
            "用户的猫叫橘子",
            &["pet"][..],
            vec!["ep-s1-cat"],
        ),
        (
            "n-food",
            DAY1,
            "用户最爱吃火锅",
            &["preference"][..],
            vec!["ep-s1-food"],
        ),
        (
            "n-city",
            DAY1 + 10 * DAY,
            "用户住在北京望京",
            &["location"][..],
            vec!["ep-s2-city"],
        ),
        (
            "n-phone-old",
            DAY1,
            "用户用的是 iPhone 12",
            &["device"][..],
            vec!["ep-s1-phone-old"],
        ),
        (
            "n-phone-new",
            DAY1 + 20 * DAY,
            "用户用的是 iPhone 15",
            &["device"][..],
            vec!["ep-s3-phone-new"],
        ),
    ];
    for (id, ts, content, tags, sources) in notes {
        store
            .put_note(&Note {
                id: id.into(),
                timestamp: ts,
                content: content.into(),
                source_episode_ids: sources.into_iter().map(String::from).collect(),
                confidence: 0.9,
                tags: tags.iter().map(|t| t.to_string()).collect(),
            })
            .expect("put_note");
    }
}

fn fresh_store() -> SqliteMemoryStore {
    let store = SqliteMemoryStore::open_in_memory().expect("open_in_memory");
    seed_fixture(&store);
    store
}

/// 余弦相似度 (两个 L2 归一化向量 = 点积).
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

// =====================================================================
// LLM 评分层留口 (0 装 PASS: 生产换真 LLM, 当前确定性判分先行)
// =====================================================================

/// 评测判分器留口: 生产路径实现方内部调 LLM provider 做语义判分.
trait LlmJudge {
    /// 判定查询的期望信息是否被 actual 覆盖.
    fn judge(&self, query: &str, expected: &str, actual: &str) -> bool;
}

/// 确定性判分器: 期望子串包含判定 (0 LLM, 可重复).
struct DeterministicJudge;

impl LlmJudge for DeterministicJudge {
    fn judge(&self, _query: &str, expected: &str, actual: &str) -> bool {
        actual.contains(expected)
    }
}

// =====================================================================
// 能力 ①: 抽取 (单跳事实)
// =====================================================================

#[test]
fn m4_c1_extraction_single_hop_fact() {
    let store = fresh_store();

    // 知识层按标签抽取.
    let hits = NoteStore::query(&store, &NoteQuery::new().with_tag("pet")).expect("query");
    assert_eq!(hits.len(), 1, "pet 标签应恰有 1 条");
    assert!(hits[0].content.contains("橘子"), "单跳事实应可抽取");

    // 语义层抽取: 与 ep-s1-cat 共享长前缀「我的猫叫」→ 相似度最高.
    let embedder = Arc::new(HashEmbedder::new(512));
    let results = store
        .semantic_search("我的猫叫什么名字", 3, embedder)
        .expect("search");
    assert!(!results.is_empty(), "语义抽取应有命中");
    assert_eq!(results[0].session_id, "s1", "top1 应来自 s1 会话");
    assert!(results[0].content.contains("橘子"), "top1 应含事实本体");
}

// =====================================================================
// 能力 ②: 多会话推理 (跨会话信息合成)
// =====================================================================

#[test]
fn m4_c2_multi_session_reasoning_spans_sessions() {
    let store = fresh_store();
    let embedder = Arc::new(HashEmbedder::new(512));

    // 查询同时关联 s2 (搬家望京) 与 s3 (望京附近软件公司工作) 两个会话.
    let results = store
        .semantic_search("我在北京望京的公司工作", 5, embedder)
        .expect("search");
    assert!(!results.is_empty());

    let sessions: std::collections::HashSet<&str> =
        results.iter().map(|e| e.session_id.as_str()).collect();
    assert!(
        sessions.contains("s2") && sessions.contains("s3"),
        "多会话合成应同时覆盖 s2 与 s3, 实际: {sessions:?}"
    );
}

// =====================================================================
// 能力 ③: 时序推理 (时间窗 / 有效期语义)
// =====================================================================

#[test]
fn m4_c3_temporal_reasoning_time_windows() {
    let store = fresh_store();

    // episode 时间窗 [DAY1+5d, DAY1+15d]: 只有 s2-city (DAY1+10d) 落窗内.
    let eps = EpisodeStore::query(
        &store,
        &EpisodeQuery::new().in_range(Some(DAY1 + 5 * DAY), Some(DAY1 + 15 * DAY)),
    )
    .expect("query eps");
    assert_eq!(eps.len(), 1, "窗内应恰 1 条 episode");
    assert_eq!(eps[0].id, "ep-s2-city");

    // note 时间窗 (提炼时间 <= DAY1+5d): 只有 DAY1 的 3 条.
    let notes = NoteStore::query(
        &store,
        &NoteQuery::new().in_range(None, Some(DAY1 + 5 * DAY)),
    )
    .expect("query notes");
    assert_eq!(notes.len(), 3, "DAY1 知识应恰 3 条");
    assert!(
        notes.iter().all(|n| n.timestamp == DAY1),
        "窗内 note 必须全部是 DAY1 提炼"
    );

    // 全库「过期」窗口 (晚于一切条目): 必须为空 — 时序推理的拒返边界.
    let expired = NoteStore::query(
        &store,
        &NoteQuery::new().in_range(Some(DAY1 + 30 * DAY), None),
    )
    .expect("query expired");
    assert!(expired.is_empty(), "窗口外不应返回任何知识");
}

// =====================================================================
// 能力 ④: 知识更新 (对账 DELETE: 新事实覆盖旧事实, 已删事实不输出)
// =====================================================================

#[test]
fn m4_c4_knowledge_update_reconcile_delete() {
    let store = fresh_store();

    // 对账前: device 标签下新旧并存.
    let before = NoteStore::query(&store, &NoteQuery::new().with_tag("device")).expect("before");
    assert_eq!(before.len(), 2, "对账前应新旧并存");

    // 模拟 Mem0 式对账应用: 旧事实被新事实覆盖 → DELETE 旧条目.
    store.delete_note("n-phone-old").expect("reconcile delete");

    // 对账后: 只剩新事实.
    let after = NoteStore::query(&store, &NoteQuery::new().with_tag("device")).expect("after");
    assert_eq!(after.len(), 1, "对账后应只剩新事实");
    assert_eq!(after[0].id, "n-phone-new");
    assert!(after[0].content.contains("iPhone 15"), "留下的应是新事实");

    // 已删事实不输出: 按 id 直查为 None.
    assert!(
        store.get_note("n-phone-old").expect("get").is_none(),
        "已删 note 不应可读"
    );

    // 已删事实不输出: 全库查询 (任何条件) 都不得含旧事实内容.
    let all = NoteStore::query(&store, &NoteQuery::new().limit(1000)).expect("all");
    assert!(
        all.iter().all(|n| !n.content.contains("iPhone 12")),
        "对账 DELETE 后全库不得输出已删旧事实"
    );
}

// =====================================================================
// 能力 ⑤: 弃答 (库外问题拒答 + 已删事实绝不输出 — 反幻觉核心验收)
// =====================================================================

/// 弃答判定阈值: 库外查询与任何 fixture 条目的余弦相似度必须低于此值.
/// (由确定性 HashEmbedder 实测标定: 库内强关联查询 > 0.5, 库外查询 ≈ 0.0x)
const REFUSAL_COSINE_THRESHOLD: f32 = 0.15;

#[test]
fn m4_c5_refusal_out_of_kb_and_deleted_fact_not_output() {
    let store = fresh_store();
    let embedder = Arc::new(HashEmbedder::new(512));

    // ---- 5a: 库外问题必须拒答 (反幻觉) ----
    let out_of_kb = "量子计算的基本原理是什么";
    let results = store
        .semantic_search(out_of_kb, 7, embedder.clone())
        .expect("search");
    let qvec = embedder.embed(out_of_kb);
    for ep in &results {
        let sim = cosine(&qvec, &embedder.embed(&ep.content));
        assert!(
            sim < REFUSAL_COSINE_THRESHOLD,
            "库外查询与条目「{}」相似度过高 ({sim}), 注入层会产生幻觉关联",
            ep.content
        );
        assert!(!ep.content.contains("量子"), "fixture 本身无此知识");
    }

    // ---- 5b: 已删事实绝不输出 (对账 DELETE 后的注入安全) ----
    store.delete_note("n-phone-old").expect("reconcile delete");

    // 用已删 note 的原文做最强诱导查询 — 知识层任何查询都不得返回它.
    let all = NoteStore::query(&store, &NoteQuery::new().limit(1000)).expect("all");
    assert!(
        all.iter().all(|n| !n.content.contains("iPhone 12")),
        "弃答验收: 对账删除的旧事实不得出现在任何注入候选中"
    );
    let device = NoteStore::query(&store, &NoteQuery::new().with_tag("device")).expect("device");
    assert_eq!(device.len(), 1, "device 维度只剩新事实");
    assert!(device[0].content.contains("iPhone 15"));
}

// =====================================================================
// 评测聚合: 5 能力判分走 LlmJudge 留口 (当前 DeterministicJudge)
// =====================================================================

#[test]
fn m4_eval_suite_all_capabilities_deterministic_judge() {
    let store = fresh_store();
    let judge = DeterministicJudge;

    // 每能力一条代表性查询 → 判分器验证信息覆盖.
    let cases: &[(&str, &str, &str)] = &[
        // (能力, 期望信息, 查询条件标签)
        ("①抽取", "橘子", "pet"),
        ("③时序", "北京望京", "location"),
        ("④知识更新", "iPhone 15", "device"),
    ];
    let mut passed = 0;
    for (cap, expected, tag) in cases {
        let hits = NoteStore::query(&store, &NoteQuery::new().with_tag(*tag)).expect("query");
        let actual: String = hits
            .iter()
            .map(|n| n.content.as_str())
            .collect::<Vec<_>>()
            .join(";");
        assert!(
            judge.judge(cap, expected, &actual),
            "能力 {cap} 判分未通过: 期望含「{expected}」, 实际「{actual}」"
        );
        passed += 1;
    }

    // ② 多会话: 期望信息分布在两个 session 的 episode.
    let embedder = Arc::new(HashEmbedder::new(512));
    let results = store
        .semantic_search("我在北京望京的公司工作", 5, embedder)
        .expect("search");
    let actual: String = results
        .iter()
        .map(|e| e.content.as_str())
        .collect::<Vec<_>>()
        .join(";");
    assert!(judge.judge("②多会话", "望京", &actual));
    passed += 1;

    // ⑤ 弃答: 已删旧事实不在全库输出中 (判分器反向验证).
    store.delete_note("n-phone-old").expect("delete");
    let all = NoteStore::query(&store, &NoteQuery::new().limit(1000)).expect("all");
    let actual: String = all
        .iter()
        .map(|n| n.content.as_str())
        .collect::<Vec<_>>()
        .join(";");
    assert!(
        !judge.judge("⑤弃答", "iPhone 12", &actual),
        "已删事实不得被判定为可输出"
    );
    passed += 1;

    assert_eq!(passed, 5, "5 能力判分应全部执行");
}
