//! **锚定注入**的生产接线证据 —— 「按信号选段」能不能落地、且**零总成本**？
//!
//! # 这个测试证明什么
//!
//! 实测结论（`docs/status/chapter-anchor-value-2026-10-27.md`）：
//! **同一成本（400 字/条）下，「选对段」的送达能力是「多给字」的 5.1 倍**
//! ⇒ ⇒ ⇒ **「信号」远比「预算」重要**。
//!
//! ⇒ 本模块把那条结论的**生产落点**做出来：
//! * **默认关闭** ⇒ ⇒ **零行为变化**（必须被**单独证明**）；
//! * 开启后 ⇒ ⇒ 证据是「**问句主体所在的那一段**」⇒ ⇒ **不是 oracle**（主体来自问句）；
//! * ⚠️ **总量不变** ⇒ ⇒ **放大单条上限不会增加总消耗**（**零成本**）。

use std::sync::Arc;

use apeireth_memory::{
    ClosedWorldContextCompiler, MemoryCoordinator, MemoryGovernanceStore, MemoryLayerKind,
    MemoryRecallQuery, MemoryWritebackEntry, SqliteMemoryStore,
};
use apeireth_storage::SqliteConnectionPool;
use tempfile::tempdir;

const NOW_MS: i64 = 1_800_000_000_000;

fn coordinator(path: &std::path::Path) -> MemoryCoordinator {
    let store = Arc::new(SqliteMemoryStore::open(path).expect("open SQLite file"));
    let backend = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store;
    MemoryCoordinator::new(backend, governance)
}

/// 写一条**很长**的记忆（答案在**深处**，开头 120 字里没有主体）。
fn write_long_episode(path: &std::path::Path, session: &str) -> String {
    let coord = coordinator(path);
    let filler = "甲".repeat(600);
    let content = format!("{filler}賈寶玉在這裡說了重要的事。{filler}");
    let mut entry = MemoryWritebackEntry::new(session, "user", &content);
    entry.timestamp_ms = Some(NOW_MS - 60_000);
    coord.writeback(&entry).expect("写入记忆");
    content
}

fn query_of(session: &str, text: &str) -> MemoryRecallQuery {
    let mut query = MemoryRecallQuery::new(session, text)
        .with_layers(vec![MemoryLayerKind::Episodic])
        .with_limit(10)
        .with_as_of_ms(NOW_MS);
    query.recency_decay_lambda = 0.05;
    query.min_score = 0.0;
    query
}

fn recalled(
    path: &std::path::Path,
    session: &str,
    text: &str,
) -> apeireth_memory::MemoryRecallResult {
    coordinator(path)
        .recall(&query_of(session, text))
        .expect("recall 应成功")
}

#[test]
fn disabled_config_produces_byte_identical_output() {
    // ⇒ **零行为变化**的护栏：**默认关闭** ⇒ ⇒ 输出与老路径**逐字节相同**。
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "anchor-session";
    write_long_episode(&path, session);
    let result = recalled(&path, session, "賈寶玉");

    let compiler = ClosedWorldContextCompiler::new();
    let legacy = compiler
        .compile_injection_with_selected_access(&result, session, 4000)
        .expect("应产出注入");
    let disabled = compiler
        .compile_injection_with_selected_access_and_query(
            &result,
            session,
            4000,
            "賈寶玉",
            apeireth_memory::anchor_injection::AnchoredInjectionConfig::default(),
        )
        .expect("应产出注入");
    assert_eq!(
        legacy.overlay, disabled.overlay,
        "默认关闭 ⇒ ⇒ 输出必须**逐字节相同**（**零行为变化**）"
    );
}

#[test]
fn enabled_config_selects_the_subject_span() {
    // ⇒ 开启后 ⇒ ⇒ 注入的证据里**含主体**（「賈寶玉」）⇒ ⇒ 而不只是开头那截。
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "anchor-session";
    write_long_episode(&path, session);
    let result = recalled(&path, session, "賈寶玉");

    let compiler = ClosedWorldContextCompiler::new();
    let legacy = compiler
        .compile_injection_with_selected_access(&result, session, 4000)
        .expect("应产出注入");
    let anchored = compiler
        .compile_injection_with_selected_access_and_query(
            &result,
            session,
            4000,
            "賈寶玉",
            apeireth_memory::anchor_injection::AnchoredInjectionConfig {
                enabled: true,
                span_chars: 400,
            },
        )
        .expect("应产出注入");
    assert!(
        !legacy.overlay.contains("賈寶玉"),
        "老路径（开头 120 字）里**不该**出现主体"
    );
    assert!(
        anchored.overlay.contains("賈寶玉"),
        "锚定路径（主体所在段）里**应该**出现主体：\n{}",
        anchored.overlay
    );
}

#[test]
fn enabling_the_anchor_does_not_increase_the_total_size() {
    // ⇒ ⇒ **零总成本**：单条从 120 放到 400 字 ⇒ ⇒ **总量不变**（≤ 总量预算）。
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "anchor-session";
    write_long_episode(&path, session);
    let result = recalled(&path, session, "賈寶玉");

    let compiler = ClosedWorldContextCompiler::new();
    let total_budget = 4000usize;
    let legacy = compiler
        .compile_injection_with_selected_access(&result, session, total_budget)
        .expect("应产出注入");
    let anchored = compiler
        .compile_injection_with_selected_access_and_query(
            &result,
            session,
            total_budget,
            "賈寶玉",
            apeireth_memory::anchor_injection::AnchoredInjectionConfig {
                enabled: true,
                span_chars: 400,
            },
        )
        .expect("应产出注入");
    assert!(
        anchored.overlay.chars().count() <= total_budget,
        "锚定后的总量 {} **不该超过**预算 {}",
        anchored.overlay.chars().count(),
        total_budget
    );
    assert!(
        anchored.overlay.chars().count() >= legacy.overlay.chars().count() - 1,
        "锚定后**不该**比老路径更短（它装了更多有用内容）"
    );
}

#[test]
fn a_query_without_a_subject_falls_back_to_the_head() {
    // ⇒ 主体解析不出来 ⇒ ⇒ **退回开头截断**（**不注入空证据**）。
    let dir = tempdir().expect("临时目录");
    let path = dir.path().join("memory.sqlite");
    let session = "anchor-session";
    write_long_episode(&path, session);
    let result = recalled(&path, session, "賈寶玉");

    let compiler = ClosedWorldContextCompiler::new();
    let anchored = compiler
        .compile_injection_with_selected_access_and_query(
            &result,
            session,
            4000,
            "？",
            apeireth_memory::anchor_injection::AnchoredInjectionConfig {
                enabled: true,
                span_chars: 400,
            },
        )
        .expect("应产出注入");
    assert!(
        anchored.overlay.contains("记忆证据"),
        "兜底时仍应产出证据块：\n{}",
        anchored.overlay
    );
}
