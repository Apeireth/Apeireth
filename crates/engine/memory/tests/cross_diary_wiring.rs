//! 跨日记检索接线 (`cross_diary_wiring`) 在检索主路径上的语义.
//!
//! # 这条接线补的是哪个缺口
//!
//! 日记的**写入面**早就有了（`apeireth dream` 按日 JSON 落盘到 `<data>/diary`），
//! 但**读取方**缺席 ⇒ 用户写下的东西永远检索不到。本接线给检索路径注入一个
//! **只读**日记面：命中事实时附上**关联日记片段**（限量、确定性排序）。
//!
//! 三条纪律（下面每条都有测试钉住）：
//! - **缺省零变化**：不注入日记面 ⇒ 召回结果与接线前逐字节一致；
//! - **只读**：检索路径只 `list_days` / `read_day`，绝不写、绝不删日记；
//! - **优雅降级**：日记目录不存在 / 日记文件损坏 ⇒ 不报错、不编造关联。

use std::sync::Arc;

use apeireth_memory::{
    DiaryEntry, DiaryStore, FileDiaryStore, MemoryCoordinator, MemoryGovernanceStore,
    MemoryLayerKind, MemoryRecallQuery, MemoryRecallResult, MemoryWritebackEntry,
    SqliteMemoryStore,
};
use tempfile::tempdir;

const SESSION: &str = "cross-diary-session";
const FACT: &str = "Rust 所有权 编译 安全";
const DIARY_DATE: &str = "2026-08-28";
const DIARY_BODY: &str = "今天主人教我写 Rust 所有权";

fn coordinator(path: &std::path::Path) -> MemoryCoordinator {
    let store = Arc::new(SqliteMemoryStore::open(path).expect("open SQLite file"));
    let backend = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store;
    MemoryCoordinator::new(backend, governance)
}

fn recall_query(session: &str, query: &str) -> MemoryRecallQuery {
    let mut request = MemoryRecallQuery::new(session, query)
        .with_layers(vec![MemoryLayerKind::Episodic])
        .with_limit(5)
        .with_as_of_ms(1_800_000_000_000);
    // 下限归零：本测试关心的是**召回集合与附加片段**，不是打分阈值
    request.min_score = 0.0;
    request
}

fn recall(coordinator: &MemoryCoordinator, query: &str) -> MemoryRecallResult {
    coordinator
        .recall(&recall_query(SESSION, query))
        .expect("recall should succeed")
}

/// 写入一条记忆 + 一份**真实日记文件**（`apeireth dream` 的落盘格式：按日一个 JSON）。
fn seed_memory_and_diary(root: &std::path::Path) -> FileDiaryStore {
    let coordinator = coordinator(&root.join("cross-diary.sqlite"));
    coordinator
        .writeback(&MemoryWritebackEntry::new(SESSION, "user", FACT))
        .expect("writeback");

    let diary = FileDiaryStore::new(root.join("diary"));
    diary
        .append(DIARY_DATE, DiaryEntry::new("dream", DIARY_BODY, 1_000))
        .expect("append diary entry");
    diary
}

/// 注入日记面 ⇒ 命中事实带上**关联日记片段**（含可解释的共享词证据）。
#[test]
fn matched_hit_carries_related_diary_snippet() {
    let dir = tempdir().expect("temporary directory");
    let diary = seed_memory_and_diary(dir.path());

    let wired =
        coordinator(&dir.path().join("cross-diary.sqlite")).with_diary_store(Arc::new(diary));
    assert!(
        wired.diary_store().is_some(),
        "注入后 = 检索路径带日记面 (只读)"
    );
    let recalled = recall(&wired, "Rust 所有权");

    let item = recalled
        .items
        .first()
        .expect("the written memory must be recalled");
    assert_eq!(
        item.diary_snippets.len(),
        1,
        "命中事实应带关联日记片段: {item:?}"
    );
    let snippet = &item.diary_snippets[0];
    assert_eq!(snippet.diary_date, DIARY_DATE);
    assert_eq!(snippet.fact_id, item.id);
    assert!(
        snippet.snippet.contains("Rust"),
        "片段应带日记正文摘要: {snippet:?}"
    );
    assert!(
        !snippet.shared_tokens.is_empty(),
        "关联必须带可解释的共享词证据: {snippet:?}"
    );
}

/// **不注入**日记面 ⇒ 召回结果与接线前一致（逐条无日记片段）。
///
/// 同一目录里**已经有**日记文件 —— 只是检索路径压根不碰它。
#[test]
fn recall_without_diary_injection_carries_no_snippet() {
    let dir = tempdir().expect("temporary directory");
    let _diary = seed_memory_and_diary(dir.path());

    let plain_coordinator = coordinator(&dir.path().join("cross-diary.sqlite"));
    assert!(
        plain_coordinator.diary_store().is_none(),
        "缺省 = 检索路径没有日记面"
    );
    let plain = recall(&plain_coordinator, "Rust 所有权");
    assert_eq!(plain.items.len(), 1);
    assert!(
        plain.items[0].diary_snippets.is_empty(),
        "未注入日记面 ⇒ 命中事实不带日记片段: {:?}",
        plain.items[0]
    );
}

/// 日记面**在场但没有日记** ⇒ 优雅降级：召回结果与未注入**逐字节一致**。
#[test]
fn missing_diary_directory_degrades_to_the_unchanged_recall() {
    let dir = tempdir().expect("temporary directory");
    let _diary = seed_memory_and_diary(dir.path());
    let sqlite = dir.path().join("cross-diary.sqlite");

    let plain = recall(&coordinator(&sqlite), "Rust 所有权");

    // 指向一个**不存在**的目录（文件日记面读不到任何一天）。
    let missing = FileDiaryStore::new(dir.path().join("no-such-diary"));
    let degraded = recall(
        &coordinator(&sqlite).with_diary_store(Arc::new(missing)),
        "Rust 所有权",
    );

    assert_eq!(
        serde_json::to_string(&plain).expect("serialize plain"),
        serde_json::to_string(&degraded).expect("serialize degraded"),
        "日记面读不到东西 ⇒ 召回结果逐字节不变"
    );
    assert!(degraded.items[0].diary_snippets.is_empty());
}

/// 日记文件**损坏** ⇒ 优雅降级：不报错、不编造关联。
#[test]
fn corrupted_diary_file_degrades_without_error() {
    let dir = tempdir().expect("temporary directory");
    let coordinator = coordinator(&dir.path().join("cross-diary.sqlite"));
    coordinator
        .writeback(&MemoryWritebackEntry::new(SESSION, "user", FACT))
        .expect("writeback");

    let diary_root = dir.path().join("diary");
    std::fs::create_dir_all(&diary_root).expect("create diary dir");
    std::fs::write(
        diary_root.join(format!("{DIARY_DATE}.json")),
        b"{ this is not json",
    )
    .expect("write broken diary file");

    let wired = coordinator.with_diary_store(Arc::new(FileDiaryStore::new(&diary_root)));
    let recalled = recall(&wired, "Rust 所有权");

    assert_eq!(recalled.items.len(), 1, "召回本身必须照常成功");
    assert!(
        recalled.items[0].diary_snippets.is_empty(),
        "坏日记不得被编造成关联: {:?}",
        recalled.items[0]
    );
}

/// **只读**：召回过程不得改写、不得新增日记文件。
#[test]
fn retrieval_path_never_writes_the_diary() {
    let dir = tempdir().expect("temporary directory");
    let diary = seed_memory_and_diary(dir.path());
    let diary_file = dir.path().join("diary").join(format!("{DIARY_DATE}.json"));
    let before = std::fs::read_to_string(&diary_file).expect("read diary file");

    let wired =
        coordinator(&dir.path().join("cross-diary.sqlite")).with_diary_store(Arc::new(diary));
    let _ = recall(&wired, "Rust 所有权");

    let after = std::fs::read_to_string(&diary_file).expect("read diary file");
    assert_eq!(before, after, "检索路径不得改写日记");
    let files = std::fs::read_dir(dir.path().join("diary"))
        .expect("read diary dir")
        .count();
    assert_eq!(files, 1, "检索路径不得新增日记文件");
}

/// 提示词 overlay（封闭世界 XML 格式）带上日记片段行。
#[test]
fn prompt_overlay_renders_the_diary_excerpt() {
    let dir = tempdir().expect("temporary directory");
    let diary = seed_memory_and_diary(dir.path());

    let wired =
        coordinator(&dir.path().join("cross-diary.sqlite")).with_diary_store(Arc::new(diary));
    let selected = wired
        .compile_prompt_overlay_with_selected_access(&recall_query(SESSION, "Rust 所有权"))
        .expect("overlay compiles")
        .expect("overlay is not empty");

    assert!(
        selected.overlay.contains(&format!("[diary:{DIARY_DATE}]")),
        "overlay 应带日记片段行: {}",
        selected.overlay
    );
    assert!(
        selected.overlay.contains("Rust"),
        "overlay 应带日记正文: {}",
        selected.overlay
    );
    assert_eq!(
        selected.selected_candidate_ids.len(),
        1,
        "日记片段行不额外贡献候选回执: {:?}",
        selected.selected_candidate_ids
    );
}

/// 提示词 overlay（反幻觉编号证据格式）带上日记片段行。
#[test]
fn injection_overlay_renders_the_diary_excerpt() {
    let dir = tempdir().expect("temporary directory");
    let diary = seed_memory_and_diary(dir.path());

    let wired = coordinator(&dir.path().join("cross-diary.sqlite"))
        .with_diary_store(Arc::new(diary))
        .with_memory_injection_format();
    let selected = wired
        .compile_prompt_overlay_with_selected_access(&recall_query(SESSION, "Rust 所有权"))
        .expect("overlay compiles")
        .expect("overlay is not empty");

    assert!(
        selected.overlay.contains(&format!("[日记 {DIARY_DATE}]")),
        "编号证据清单应含日记条目: {}",
        selected.overlay
    );
    assert_eq!(
        selected.selected_candidate_ids.len(),
        1,
        "日记证据行不额外贡献候选回执: {:?}",
        selected.selected_candidate_ids
    );
}
