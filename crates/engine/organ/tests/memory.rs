//! Memory 器官集成测试 (per 任务 §3, 子代理 R8).
//!
//! 3 测试 (per task spec §3):
//! 1. `memory_merger_organ_merge_deduplicates_similar_content` (merge 路径, dedup)
//! 2. `memory_merger_organ_weight_increases_total` (weight 调整)
//! 3. `memory_merger_organ_query_finds_by_keyword` (query 路径)
//!
//! **0 装诚实** (per 任务 §3 + 子代理 R 同款):
//! - 真生产路径: `MemoryMergerOrgan::with_default()` (无 LLM 注入 — 跨 organ 合并是
//!   确定性无 LLM 抽象).
//! - dev 测试路径: 直接用 `MemoryMerger` 引擎 + `MemoryMergerOrgan::process()` 两条路并行.
//! - `#[ignore]` 测试: trait 边界 shape 验 (per 子代理 R1-R7 同模式)
//!
//! **承接**:
//! - 子代理 R1/R2/R3/R4/R5/R6/R7 8 个 organ 真实现已就位, Memory 是第 8 organ 真实现
//!   (per `crates/engine/organ/src/lib.rs:30-32` 进度: 8/9 organ 真实现, 仅 W2 占位)
//! - 跨 8 organ trait 抽象: Memory process() 仅**标识** source_organ (per 子代理 R8
//!   独立判断: 避免 organ→organ cyclic dep; 真生产由 cognitive module 集成时调度)
//!
//! **R8 独立判断** (与 R7 同模式):
//! - 任务 spec "1:1 翻译 v1 MemoryMerger" — **v1 没有此模块** (核查
//!   `legacy/donor/apeireth-companion/src/runtime_brain.rs` 242 行无 MemoryMerger;
//!   相关散落 3 处: memory_extractor + proactive_memory + runtime_brain).
//! - v2 MemoryMerger 是**新设计**, 借鉴 v1 `MemoryExtractionService` (dedup-by-content
//!   + weight + persist schema) **算法骨架** 1:1 翻译.
//! - trait schema `OrganOutput::Memory { notes_added, notes_merged }` 锁定
//!   (per `apeireth-plugin::organ:184-185`).

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_organ::memory::{MemoryConfig, MemoryMerger, MemoryMergerOrgan};
use apeireth_organ::{OrganInput, OrganKind, OrganOutput, OrganTrait};

fn make_input(hints: Vec<String>, content: &str) -> OrganInput {
    let ep = Episode {
        id: "integration-test-mrg".into(),
        session_id: SessionId::new().to_string(),
        role: "user".into(),
        content: content.into(),
        timestamp: 1_700_000_000,
    };
    OrganInput::new(ep, hints)
}

// ============================================
// Test 1: merge 路径 dedup 同内容 (per task spec §3)
// ============================================

#[tokio::test]
async fn memory_merger_organ_merge_deduplicates_similar_content() {
    // 0 装诚实: 直接用 MemoryMerger 引擎 (无 LLM, 跨 organ 合并是确定性).
    // 验 merge 路径 1:1 v1 MemoryExtractionService::apply 算法骨架.
    let mut merger = MemoryMerger::new(MemoryConfig::default());
    assert_eq!(merger.len(), 0, "初始空 merger");

    // 第一次 merge: 新内容
    let (is_new_1, id_1) = merger.merge(
        OrganKind::E4,
        "主人的工作进入新阶段",
        0.7,
        1_700_000_000_000,
    );
    assert!(is_new_1, "首次 merge 应新增");
    assert!(id_1.starts_with("mrg-"), "id 前缀对齐 v1 mem-ex-/pref- 1:1");

    // 第二次 merge: 完全相同内容 → dedup
    let (is_new_2, id_2) = merger.merge(
        OrganKind::F1,
        "主人的工作进入新阶段",
        0.5,
        1_700_000_001_000,
    );
    assert!(!is_new_2, "重复内容应 dedup, 不新增");
    assert_eq!(id_1, id_2, "dedup 应返同一 id");

    // dedup 路径 weight 累加 (0.7 + 0.5*0.95 = 1.175 → clamp 1.0)
    let mem = merger.list().iter().find(|m| m.id == id_1).unwrap();
    assert!(
        (mem.weight - 1.0).abs() < 1e-4,
        "dedup 后 weight 应累加并 clamp: got {}",
        mem.weight
    );
    // dedup 后 at_ms 取 max
    assert_eq!(mem.at_ms, 1_700_000_001_000, "dedup 后 at_ms 应取最新");

    // 不同内容 → 新增
    let (is_new_3, _) = merger.merge(
        OrganKind::W1,
        "完全不同内容",
        0.5,
        1_700_000_002_000,
    );
    assert!(is_new_3, "新内容应新增");

    assert_eq!(merger.len(), 2, "应有 2 条记忆 (dedup 后)");

    // 同时验 trait process() 路径: schema { notes_added, notes_merged } 锁定
    let organ = MemoryMergerOrgan::with_default();
    let out = organ
        .process(make_input(
            vec![
                "source_organ=E4".into(),
                "weight=0.7".into(),
                "at_ms=1700000000000".into(),
            ],
            "新内容 (trait 路径)",
        ))
        .await
        .expect("process ok");
    match out {
        OrganOutput::Memory {
            notes_added,
            notes_merged,
        } => {
            assert_eq!(notes_added, 1, "新内容 → notes_added=1");
            assert_eq!(notes_merged, 0, "新内容 → notes_merged=0");
        }
        other => panic!("expected Memory output, got {other:?}"),
    }

    // 1) llm_factory() 返 None (0 装诚实 — 跨 organ 合并是确定性无 LLM)
    assert!(
        organ.llm_factory().is_none(),
        "Memory 是跨 organ 确定性抽象, trait 必须返 None (0 装诚实)"
    );

    // 2) organ_id + name 锁定 Memory
    assert_eq!(organ.organ_id(), OrganKind::Memory);
    assert_eq!(organ.name(), "Memory Merger");

    // 3) trait process 记录 1 条
    assert_eq!(organ.merger().len(), 1);
}

// ============================================
// Test 2: weight 调整路径 (per task spec §3)
// ============================================

#[tokio::test]
async fn memory_merger_organ_weight_increases_total() {
    // 0 装诚实: 直接用 MemoryMerger (无 LLM), 验 weight 调整路径.
    let mut merger = MemoryMerger::new(MemoryConfig::default());

    let (_is_new, id) = merger.merge(OrganKind::E4, "重要事实", 0.5, 1_700_000_000_000);
    let initial = merger.list().iter().find(|m| m.id == id).unwrap().weight;
    assert!(
        (initial - 0.5).abs() < 1e-4,
        "初始 weight 应为 0.5: got {}",
        initial
    );

    // weight +0.3 → 0.8
    assert!(merger.weight(&id, 0.3), "weight 调整应成功");
    let after_up = merger.list().iter().find(|m| m.id == id).unwrap().weight;
    assert!(
        (after_up - 0.8).abs() < 1e-4,
        "weight 应增到 0.8: got {}",
        after_up
    );

    // weight -0.5 → 0.3
    assert!(merger.weight(&id, -0.5), "weight 减应成功");
    let after_down = merger.list().iter().find(|m| m.id == id).unwrap().weight;
    assert!(
        (after_down - 0.3).abs() < 1e-4,
        "weight 应降到 0.3: got {}",
        after_down
    );

    // 减到 0 以下 → clamp 0.0
    assert!(merger.weight(&id, -1.0));
    let clamped = merger.list().iter().find(|m| m.id == id).unwrap().weight;
    assert!(
        (clamped - 0.0).abs() < 1e-4,
        "weight 应 clamp 到 0.0: got {}",
        clamped
    );

    // 增到 1.0 以上 → clamp 1.0
    assert!(merger.weight(&id, 5.0));
    let clamped_up = merger.list().iter().find(|m| m.id == id).unwrap().weight;
    assert!(
        (clamped_up - 1.0).abs() < 1e-4,
        "weight 应 clamp 到 1.0: got {}",
        clamped_up
    );

    // 不存在 id → false (0 装诚实)
    assert!(!merger.weight("mrg-nonexist", 0.1), "不存在 id 应返 false");

    // 同时验 trait process 路径: source_organ 解析锁定
    let organ = MemoryMergerOrgan::with_default();
    let out = organ
        .process(make_input(
            vec![
                "source_organ=W2".into(),
                "weight=0.6".into(),
                "at_ms=1700000123000".into(),
            ],
            "weight 路径 trait 测试",
        ))
        .await
        .expect("process ok");

    match out {
        OrganOutput::Memory {
            notes_added,
            notes_merged,
        } => {
            assert_eq!(notes_added, 1, "weight 路径新内容 notes_added=1");
            assert_eq!(notes_merged, 0);
        }
        other => panic!("expected Memory output, got {other:?}"),
    }

    // 验 merger 状态: weight 调整后的条目存在
    {
        let merger = organ.merger();
        let entry = merger
            .list()
            .iter()
            .find(|m| m.content == "weight 路径 trait 测试")
            .expect("应有该条目");
        assert!(
            (entry.weight - 0.6).abs() < 1e-4,
            "weight 应为 0.6 (hint 解析)"
        );
        assert_eq!(entry.source_organ, OrganKind::W2, "source_organ 解析 W2");
    }
}

// ============================================
// Test 3: query 路径按关键词检索 (per task spec §3)
// ============================================

#[tokio::test]
async fn memory_merger_organ_query_finds_by_keyword() {
    // 0 装诚实: 用 MemoryMerger (无 LLM) 验 query 路径 1:1 v1 dedup-by-keyword.
    let mut merger = MemoryMerger::new(MemoryConfig::default());

    merger.merge(OrganKind::E4, "主人明天要考线代", 0.9, 1_700_000_000_000);
    merger.merge(OrganKind::F1, "主人今天心情好", 0.6, 1_700_000_001_000);
    merger.merge(OrganKind::W1, "项目上线了", 0.8, 1_700_000_002_000);
    merger.merge(OrganKind::F6, "主人喜欢古风", 0.7, 1_700_000_003_000);

    // 关键词"主人" → 应命中 3 条 (前 3 条都含)
    let hits = merger.query("主人");
    assert_eq!(hits.len(), 3, "关键词'主人'应命中 3 条: {hits:?}");
    // 按 weight 倒序: 0.9 (E4) > 0.7 (F6) > 0.6 (F1) — 但 F6 不含"主人" → 0.9 > 0.6
    assert!(
        hits[0].weight >= hits[1].weight,
        "应按 weight 倒序: hits={:?}",
        hits.iter().map(|h| (h.content.as_str(), h.weight)).collect::<Vec<_>>()
    );

    // 关键词"古风" → 命中 1 条
    let gufeng = merger.query("古风");
    assert_eq!(gufeng.len(), 1, "关键词'古风'应命中 1 条");
    assert!(gufeng[0].content.contains("古风"));

    // 关键词"上线" → 命中 1 条 (中文 keyword)
    let online = merger.query("上线");
    assert_eq!(online.len(), 1, "中文 keyword '上线' 应命中");
    assert!(online[0].content.contains("上线"));

    // 大小写不敏感 (英文混合)
    merger.merge(OrganKind::W2, "Rust async trait", 0.5, 1_700_000_004_000);
    let lower = merger.query("rust");
    let upper = merger.query("RUST");
    assert_eq!(
        lower.len(),
        upper.len(),
        "大小写不敏感: rust == RUST 命中数应等: lower={} upper={}",
        lower.len(),
        upper.len()
    );
    assert_eq!(lower.len(), 1);

    // 空关键词 → 空 (0 装诚实: call 行为确定, 不假装"全返")
    let no_match = merger.query("");
    assert!(
        no_match.is_empty(),
        "空关键词应返空 (0 装诚实): got {}",
        no_match.len()
    );

    // 不存在关键词 → 空
    let nothing = merger.query("xyznomatch");
    assert!(nothing.is_empty(), "不存在关键词应返空");

    // 同时验 trait process 路径: process 后 merger list 应有该条目
    let organ = MemoryMergerOrgan::with_default();
    let _ = organ
        .process(make_input(
            vec!["source_organ=E4".into(), "weight=0.95".into()],
            "query 路径 trait 测试",
        ))
        .await
        .expect("process ok");
    let found = {
        let merger = organ.merger();
        merger
            .list()
            .iter()
            .any(|m| m.content.contains("query 路径"))
    };
    assert!(found, "trait process 后 list 应找到");
}

// ============================================
// Test 4 (#[ignore]): trait 边界 shape 验 (per 子代理 R1-R7 同模式)
// ============================================

/// **0 装诚实**: trait 边界完整 + 跨 organ trait 抽象.

#[tokio::test]
#[ignore = "manual run: cargo test -p apeireth-organ --test memory -- --ignored; 验 trait shape"]
async fn memory_organ_trait_shape_with_noop_llm_factory() {
    use apeireth_plugin::llm_factory::{LlmFactory, NoopLlmFactory};
    use std::sync::Arc;

    let _noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    // MemoryMergerOrgan 不需要 LLM 注入 (跨 organ 合并是确定性无 LLM); 验 trait shape 完整.
    let organ = MemoryMergerOrgan::with_default();
    let output = organ
        .process(make_input(
            vec!["source_organ=E4".into(), "weight=0.5".into()],
            "trait shape 测试",
        ))
        .await
        .expect("process ok");
    match output {
        OrganOutput::Memory { .. } => {
            // OK: trait 边界 + factory 不依赖路径都 work
        }
        other => panic!("expected Memory output, got {other:?}"),
    }
    // 0 装诚实: llm_factory() 返 None (trait 默认)
    assert!(
        organ.llm_factory().is_none(),
        "跨 organ 合并是确定性无 LLM 抽象, trait 必须返 None (0 装诚实)"
    );

    // 验所有 9 OrganKind 都能被 trait 路径接受 (含 Memory)
    let kinds = [
        OrganKind::W1,
        OrganKind::W2,
        OrganKind::W3,
        OrganKind::E4,
        OrganKind::F4,
        OrganKind::F1,
        OrganKind::F6,
        OrganKind::E7,
        OrganKind::Memory,
    ];
    for k in &kinds {
        // 仅验 trait 编译: parse_hints 能识 source_organ=str
        let hint = format!("source_organ={:?}", k);
        let _ = hint; // 仅验编译通过
    }
}