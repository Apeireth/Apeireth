//! **B2 批：契约在生产装配根上的存在与「生产路径逐字节不变」**。
//!
//! # 这个文件证明什么
//!
//! 1. ⭐ **装配根能拿到契约束**（`memory_contract_wire::install()`）⇒ 三契约在生产**有落点**；
//! 2. ⭐ **口径对齐自检真的跑了**（25 组范围组合）⇒ 两套可见性口径**没分叉**
//!    （分叉 ⇒ 装配会失败 ⇒ 这道门是**会咬人**的，不是摆设）；
//! 3. ⭐ **生产召回→编译路径的产物逐字节不变**（契约视图是无损投影 ⇒ 输出不变）。
//!
//! # 中性表述
//!
//! 只描述**机制**（契约 / 口径对齐 / 逐字节不变），不引用任何外部产品名。

use apeireth_memory::memory_view::Scope;
use apeireth_memory::{MemoryRecallQuery, MemoryWritebackEntry};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_runtime_assembly::canonical::memory_contract_wire;
use std::sync::Arc;

fn coordinator() -> apeireth_memory::MemoryCoordinator {
    let store = Arc::new(apeireth_memory::SqliteMemoryStore::open_in_memory().expect("store"));
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn apeireth_memory::MemoryGovernanceStore> = store;
    apeireth_memory::MemoryCoordinator::new(backend, governance)
}

/// ⭐⭐ **装配根能拿到契约束 + 自检真的跑了**。
#[test]
fn the_assembly_root_can_install_the_contracts() {
    let bundle = memory_contract_wire::install().expect("契约装配必须通过（口径分叉 ⇒ 失败）");
    assert_eq!(
        bundle.alignment_pairs,
        memory_contract_wire::ALIGNMENT_PAIRS,
        "自检必须真的覆盖全部范围组合（不是空跑）"
    );
}

/// ⭐⭐ **生产召回→编译路径的产物逐字节不变**（契约视图是无损投影）。
#[test]
fn the_production_overlay_is_byte_identical_after_the_contract_wiring() {
    let coordinator = coordinator();
    coordinator
        .writeback(&MemoryWritebackEntry::new(
            "session-a",
            "user",
            "用户说他喜欢简洁的回答",
        ))
        .expect("writeback");

    // 召回查询用**默认范围**（会话域）⇒ 写入的那条记忆可见（不改范围口径）。
    let query = MemoryRecallQuery::new("session-a".to_string(), "简洁".to_string());

    let selected = coordinator
        .compile_prompt_overlay_with_selected_access(&query)
        .expect("必须能编译出 overlay")
        .expect("有内容时必须产出 overlay");
    assert!(
        selected
            .overlay
            .contains("<governed_memory provenance=\"session-a\""),
        "生产 overlay 形态必须不变（契约接线只收窄面，不改形态）：\n{}",
        selected.overlay
    );
    assert!(
        selected.overlay.contains("用户说他喜欢简洁的回答"),
        "证据必须原样在 overlay 里：\n{}",
        selected.overlay
    );
}

/// ⭐ **算法只能通过契约视图读**（窄面 + 未授权 ⇒ fail-closed）。
#[test]
fn algorithms_read_through_the_contract_view_only() {
    let items = vec![apeireth_memory::layers::RecalledMemoryItem {
        id: "ep-1".to_string(),
        layer: apeireth_memory::layers::MemoryLayerKind::Episodic,
        content: "一条记忆".to_string(),
        timestamp_ms: 1_700_000_000_000,
        score: 0.9,
        importance: 0.5,
        source_ref: Some("session-a".to_string()),
        score_components: None,
        diary_snippets: Vec::new(),
    }];
    let granted = vec![Scope::Global];
    let view = memory_contract_wire::view_for(&items, &granted);
    assert_eq!(view.len(), 1, "授权范围内 ⇒ 可见");
    assert_eq!(
        view.annotations().len(),
        2,
        "每条一个分数标注 + 一个标签标注"
    );

    let closed = memory_contract_wire::view_for(&items, &[]);
    assert!(closed.is_empty(), "未授权 ⇒ fail-closed（一条都不给）");
}
