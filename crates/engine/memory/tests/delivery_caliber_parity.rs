//! **送达口径一致性** —— 「闸门量的」与「生产做的」**不是同一件事**。
//!
//! # 这个测试证明什么
//!
//! 2026-11-08 的送达漏送分解（`crates/engine/memory/examples/delivery_loss_decomposition.rs`，
//! 证据：`docs/04-internal/01-handoffs/memory-delivery-loss-decomposition-2026-11-08.md`）测出：
//!
//! | 注入口径 | 内部卷·红楼梦 | 内部卷·水浒传 | 真实查询卷（393 问句） |
//! |---|---|---|---|
//! | **闸门头截**（每条正文**前 120 字**） | 0.3121 | 0.5822 | 0.7477 |
//! | **注入 + 锚定选段**（主体出现处，400 字） | 0.8949 | 0.9867 | **1.0000** |
//! | **XML 形态**（每条放整条正文） | 0.0000 | 0.0000 | 0.9362 |
//!
//! ⇒ ⇒ **「送达率 0.3881 / 0.7218、漏送 61% / 28%」量的是第一行**（闸门口径）
//! ⇒ ⇒ **不是生产形态** ⇒ ⇒ **拿它当生产结论就是自欺**。
//!
//! # 本测试钉住的四件事
//!
//! 1. **同一总预算下，「选对段」送达的东西 ≥ 「头截」**（机制护栏，不是口径之争）；
//! 2. **长候选会把「名额」吃光**（候选预算按**原始正文**计费 ⇒ 名额被浪费）—— 钉住现状；
//! 3. **生产单条跨度 = 400**（两把尺子结论**相反** ⇒ 不许单边改 ⇒ 见证据文档）；
//! 4. **锚定路径不突破总预算**（总量纪律）。
//!
//! # 中性表述
//!
//! 只描述机制（召回 / 选段 / 注入 / 预算 / 送达），不引用任何外部产品名、文档或代码。

use std::sync::Arc;

use apeireth_memory::anchor_injection::AnchoredInjectionConfig;
use apeireth_memory::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
};
use apeireth_memory::{
    ClosedWorldContextCompiler, MemoryCoordinator, MemoryGovernanceStore, MemoryLayerKind,
    MemoryRankingConfig, MemoryRecallQuery, MemoryWritebackEntry, SqliteMemoryStore,
};
use apeireth_storage::SqliteConnectionPool;
use tempfile::tempdir;

const NOW_MS: i64 = 1_800_000_000_000;
/// 与生产同口径的注入**总**预算（`MemoryRecallQuery::max_chars`）。
const MAX_CHARS: usize = 4_000;
/// 闸门口径的单条上限（正文**前 120 字**）。
const HEAD_CHARS: usize = 120;
/// 答案标记（放在**深处** ⇒ 开头 120 字里没有）。
const ANSWER_MARK: &str = "重要的事";
/// 问句里的主体（与 `query_signals` 解析口径一致）。
const SUBJECT: &str = "賈寶玉";

fn coordinator(path: &std::path::Path) -> MemoryCoordinator {
    let store = Arc::new(SqliteMemoryStore::open(path).expect("open SQLite file"));
    let backend = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store;
    MemoryCoordinator::new(backend, governance)
}

/// 写一条**长**记忆：主体与答案都在**深处**（开头 120 字里两者都没有）。
fn write_deep_episode(path: &std::path::Path, session: &str, filler: &str) -> String {
    let coord = coordinator(path);
    let content = format!("{filler}{SUBJECT}在這裡說了{ANSWER_MARK}。{filler}");
    let mut entry = MemoryWritebackEntry::new(session, "user", &content);
    entry.timestamp_ms = Some(NOW_MS - 60_000);
    coord.writeback(&entry).expect("写入记忆");
    content
}

fn recalled(path: &std::path::Path, session: &str) -> apeireth_memory::MemoryRecallResult {
    let mut query = MemoryRecallQuery::new(session, SUBJECT)
        .with_layers(vec![MemoryLayerKind::Episodic])
        .with_limit(10)
        .with_as_of_ms(NOW_MS);
    query.recency_decay_lambda = 0.05;
    query.min_score = 0.0;
    coordinator(path).recall(&query).expect("recall 应成功")
}

/// ⭐ **机制护栏**：同一总预算下，「选对段」送达的东西**不少于**「头截」。
#[test]
fn anchored_path_delivers_what_head_truncation_loses() {
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "delivery-caliber";
    write_deep_episode(&path, session, &"甲".repeat(600));
    let result = recalled(&path, session);

    let compiler = ClosedWorldContextCompiler::new();
    let head = compiler
        .compile_injection_with_selected_access_budget(&result, session, MAX_CHARS, HEAD_CHARS)
        .expect("应产出注入");
    let anchored = compiler
        .compile_injection_with_selected_access_and_query(
            &result,
            session,
            MAX_CHARS,
            SUBJECT,
            AnchoredInjectionConfig::PRODUCTION,
        )
        .expect("应产出注入");

    assert!(
        !head.overlay.contains(ANSWER_MARK),
        "闸门口径（每条前 120 字）**不该**送达深处的答案"
    );
    assert!(
        anchored.overlay.contains(ANSWER_MARK),
        "锚定口径（主体出现处）**必须**送达深处的答案 ⇒ ⇒ 这是两种口径的**机制差**"
    );
}

/// ⭐ **长候选不再吃光名额**（2026-11-09 翻转：计费已从「原始正文」改成「选后长度」）。
///
/// # 这条用例原来是**绊线**
/// 原版钉的是改造前现状：「按**原始正文**计费，一条 1,000 字的候选占掉 1,000 字
/// ，4,000 字预算只装得下 4 条，排在第 5～10 位的候选根本没机会进注入」，
/// 并在注释里写明「**改成按选后长度计费时必红一次**」。
///
/// # 现在已红过，按约定翻转成**新不变量**
/// 计费改成 `min(正文长度, 单条跨度 400)` 之后：1,000 字/条只算 **400 字**，
/// ；4,000 字预算**装得下全部 10 条**（10 × 400 = 4,000）。
/// ；；而这正是生产要的形态：注入端每条只送 400 字（锚定选段），预算不再被浪费。
///
/// 翻转后的守护面：**长正文不得再挤掉后面的候选**（谁把计费改回原始长度，这条就红）。
#[test]
fn long_candidates_no_longer_eat_the_candidate_slots() {
    let documents: Vec<apeireth_memory::MemoryCandidate> = (0..10)
        .map(|index| apeireth_memory::MemoryCandidate {
            id: format!("doc-{index:02}"),
            layer: "test".to_string(),
            scope: apeireth_memory::MemoryScope::Global,
            content: format!("{SUBJECT}{index}{}", "甲".repeat(1_000)),
            score: 0.0,
            score_components: Default::default(),
            provenance: Default::default(),
        })
        .collect();
    let source = Bm25LexicalCandidateSource::new(documents);
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig::default());

    let admitted = pipeline
        .retrieve(
            SUBJECT,
            &[apeireth_memory::MemoryScope::Global],
            &[&source],
            10,
            MAX_CHARS,
        )
        .expect("检索应成功");
    assert_eq!(
        admitted.len(),
        10,
        "按**选后长度**（min(正文, 400)）计费，1,000 字/条只算 400 字，4,000 字预算装得下全部 10 条"
    );
    // 计费总量 = 10 × 400 = 4,000 ≤ MAX_CHARS，预算刚好用满而**不超**。
    let billed: usize = admitted
        .iter()
        .map(|item| {
            item.content
                .chars()
                .count()
                .min(AnchoredInjectionConfig::PRODUCTION.span_chars)
        })
        .sum();
    assert_eq!(billed, MAX_CHARS, "计费总量应恰好用满预算：{billed}");

    let relaxed = pipeline
        .retrieve(
            SUBJECT,
            &[apeireth_memory::MemoryScope::Global],
            &[&source],
            10,
            100_000,
        )
        .expect("检索应成功");
    assert_eq!(
        relaxed.len(),
        10,
        "放宽字符预算 ⇒ ⇒ 名额应当用满（这说明被吃光的不是候选本身，是**计费口径**）"
    );
}

/// ⭐ **两把尺子结论相反** ⇒ 单条跨度**不许单边改**。
///
/// * 内部卷（模板题，章节长）偏好 **300**（红楼梦 281 → 291 题）；
/// * 真实查询卷（真人问句，段落长）偏好 **400**（329 → 325 题，改 300 会掉）。
///
/// ⇒ ⇒ 冲突 ⇒ ⇒ **保持现值 400** ⇒ ⇒ 改它必须**两把尺子一起重跑**。
#[test]
fn production_span_stays_400_until_both_rulers_agree() {
    assert_eq!(
        AnchoredInjectionConfig::PRODUCTION.span_chars,
        400,
        "单条跨度 = 400：内部卷偏 300、真实查询卷偏 400 ⇒ 冲突 ⇒ 保持现值 ⇒ 改它要两把尺子一起重跑"
    );
    assert!(
        AnchoredInjectionConfig::PRODUCTION.enabled,
        "锚定注入在生产**是开的**（D-11）⇒ ⇒ 注入形态那一路的生产口径就是「锚定选段」"
    );
}

/// ⭐ **总量纪律**：锚定路径**不得**突破总预算。
#[test]
fn anchored_path_respects_the_total_budget() {
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "delivery-caliber";
    write_deep_episode(&path, session, &"丙".repeat(600));
    let result = recalled(&path, session);

    let compiler = ClosedWorldContextCompiler::new();
    let anchored = compiler
        .compile_injection_with_selected_access_and_query(
            &result,
            session,
            MAX_CHARS,
            SUBJECT,
            AnchoredInjectionConfig::PRODUCTION,
        )
        .expect("应产出注入");
    assert!(
        anchored.overlay.chars().count() <= MAX_CHARS,
        "锚定路径不得突破总预算（实测 {} 字 > {} 字）",
        anchored.overlay.chars().count(),
        MAX_CHARS
    );
}
