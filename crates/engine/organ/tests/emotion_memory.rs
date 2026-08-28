//! F1 EmotionMemory 器官 集成测试 (per 任务 §3, 子代理 R1).
//!
//! 3 测试 (per task spec §3):
//! 1. `emotion_organ_record_updates_mood_state` (record → 改 current_mood, 1:1 v1)
//! 2. `emotion_organ_mood_trend_reflects_history` (mood_trend 路径 — 历史反映趋势)
//! 3. `emotion_organ_recall_by_mood_finds_similar` (recall_by_mood 路径, 1:1 v1)
//!
//! **0 装诚实** (per 任务 §3 + 子代理 R 同款):
//! - 真生产路径: `EmotionOrgan::new()` (无 LLM 注入 — v1 emotion_memory 是确定性无 LLM).
//! - dev 测试路径: 直接用 `EmotionMemoryEngine` (无 LLM 介入).
//! - `#[ignore]` 测试: trait 边界 + LlmFactory 注入 shape 验 (per 子代理 Q1/R2 同模式)
//!
//! **承接**:
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入), F1 与 E4/F4/F6
//!   共享 `LlmFactory` trait 边界
//! - 子代理 R2 F4 / R3 F6 并行写, 0 触碰

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_organ::emotion_memory::{EmotionMemoryEngine, EmotionOrgan, MoodSource};
use apeireth_organ::{EmotionTrend, OrganInput, OrganKind, OrganOutput, OrganTrait};
use apeireth_plugin::llm_factory::{LlmFactory, NoopLlmFactory};
use std::sync::Arc;

fn make_input(hints: Vec<String>) -> OrganInput {
    let ep = Episode {
        id: "integration-test-f1".into(),
        session_id: SessionId::new().to_string(),
        role: "user".into(),
        content: "主人刚提到他今天很烦".into(),
        timestamp: 1_700_000_000,
    };
    OrganInput::new(ep, hints)
}

// ============================================
// Test 1: record → 改 current_mood (1:1 v1 emotion_memory::record / current_mood)
// ============================================

#[tokio::test]
async fn emotion_organ_record_updates_mood_state() {
    // 0 装诚实: 直接用 EmotionMemoryEngine (无 LLM 介入, v1 是确定性无 LLM).
    // 验证 record → current_mood 路径 1:1 v1.
    let mut engine = EmotionMemoryEngine::new(Default::default());
    assert_eq!(engine.current_mood().sample_count, 0, "无数据 → 空快照");

    engine.record(apeireth_organ::emotion_memory::MoodRecord::new(
        0.5,
        0.3,
        MoodSource::TextSignal,
        "主人聊得开心",
    ));
    engine.record(apeireth_organ::emotion_memory::MoodRecord::new(
        -0.8,
        0.6,
        MoodSource::ExplicitFeedback,
        "主人说今天很烦",
    ));

    // 同时验 EmotionOrgan trait 路径: process() 走 record + 当前快照
    let organ = EmotionOrgan::new();
    let output = organ
        .process(make_input(vec![
            "valence=-0.7".into(),
            "arousal=0.6".into(),
            "source=explicit_feedback".into(),
        ]))
        .await
        .expect("process ok");

    match output {
        OrganOutput::Emotion {
            pleasure,
            arousal,
            dominance,
            trend,
        } => {
            // process() 解析 hints → 记录一条 record (valence=-0.7)
            // current_mood = 该 record 加权 → pleasure ≈ -0.7
            assert!(
                (pleasure - (-0.7)).abs() < 1e-3,
                "pleasure 应≈ -0.7 (解析 hint)"
            );
            assert!(
                (arousal - 0.6).abs() < 1e-3,
                "arousal 应≈ 0.6 (解析 hint)"
            );
            assert_eq!(
                dominance, 0.0,
                "0 装诚实: v1 emotion_memory 无 dominance 概念, schema 字段填 0.0"
            );
            // 单条 record → trend = None → Stable (trend 阈值 0.05)
            assert!(
                matches!(trend, EmotionTrend::Stable),
                "单条 record trend=None → Stable"
            );
        }
        other => panic!("expected Emotion output, got {other:?}"),
    }

    // 1) llm_factory() 返 None (0 装诚实 — v1 emotion_memory 确定性无 LLM)
    assert!(
        organ.llm_factory().is_none(),
        "v1 emotion_memory 是确定性无 LLM, trait 必须返 None (0 装诚实)"
    );

    // 2) organ_id + name 锁定 F1
    assert_eq!(organ.organ_id(), OrganKind::F1);
    assert_eq!(organ.name(), "F1 Emotion Memory");

    // 3) engine 状态: 1 条 record (process() 记录)
    assert_eq!(organ.engine().len(), 1, "process() 记录 1 条 MoodRecord");
}

// ============================================
// Test 2: mood_trend 路径 (per v1 mood_trend — 历史反映趋势)
// ============================================

#[tokio::test]
async fn emotion_organ_mood_trend_reflects_history() {
    // 0 装诚实: 直接用 EmotionMemoryEngine (无 LLM), 用固定时间戳验 trend 路径.
    let mut engine = EmotionMemoryEngine::new(Default::default());
    let now = 1_700_000_000_000_i64;

    engine.record(apeireth_organ::emotion_memory::MoodRecord::with_timestamp(
        -0.7,
        0.5,
        MoodSource::TextSignal,
        "早 (烦)",
        now - 8 * 3600 * 1000,
    ));
    engine.record(apeireth_organ::emotion_memory::MoodRecord::with_timestamp(
        0.4,
        0.3,
        MoodSource::TextSignal,
        "晚 (好转)",
        now,
    ));

    // mood_trend_at: 窗口内首尾 valence 差
    let trend = engine.mood_trend_at(now, 24 * 3600 * 1000).unwrap();
    assert!(
        trend > 0.0,
        "情绪在变好: trend={trend} (val 末-首 = 0.4-(-0.7) = 1.1)"
    );

    // 窗口太短 → None
    assert!(engine.mood_trend_at(now, 60 * 1000).is_none());

    // EmotionOrgan 路径: 验 dry_run 也走 mood_trend (process 内的 trend_to_enum)
    let organ = EmotionOrgan::new();
    let output = organ
        .process(OrganInput {
            episode: Episode {
                id: "test-f1-trend".into(),
                session_id: SessionId::new().to_string(),
                role: "user".into(),
                content: "".into(),
                timestamp: now / 1000,
            },
            session_id: SessionId::new().to_string(),
            context_hints: vec![],
            dry_run: true,
        })
        .await
        .expect("process ok");

    match output {
        OrganOutput::Emotion { trend, .. } => {
            // 空快照 → trend = None → Stable
            assert!(
                matches!(trend, EmotionTrend::Stable),
                "空快照 trend=None → Stable, got {trend:?}"
            );
        }
        other => panic!("expected Emotion output, got {other:?}"),
    }
}

// ============================================
// Test 3: recall_by_mood 路径 (per v1 recall_by_mood — 找相似时段)
// ============================================

#[tokio::test]
async fn emotion_organ_recall_by_mood_finds_similar() {
    // 0 装诚实: 用 EmotionMemoryEngine (无 LLM) 验 recall_by_mood 1:1 v1.
    let mut engine = EmotionMemoryEngine::new(Default::default());
    let now = 1_700_000_000_000_i64;

    engine.record(apeireth_organ::emotion_memory::MoodRecord::with_timestamp(
        -0.9,
        0.7,
        MoodSource::ExplicitFeedback,
        "上次项目黄了",
        now - 3 * 24 * 3600 * 1000,
    ));
    engine.record(apeireth_organ::emotion_memory::MoodRecord::with_timestamp(
        0.8,
        0.2,
        MoodSource::TextSignal,
        "拿到投资那天",
        now - 5 * 24 * 3600 * 1000,
    ));

    // 低落检索 → 找到"项目黄了"
    let low = engine.recall_by_mood_at(now, -0.8, 0.2, 5);
    assert_eq!(low.len(), 1, "应找到 1 条低落时段");
    assert!(
        low[0].note.contains("项目黄了"),
        "应找回低落时段: {:?}",
        low[0].note
    );

    // 高昂检索 → 找到"拿到投资那天"
    let high = engine.recall_by_mood_at(now, 0.8, 0.2, 5);
    assert_eq!(high.len(), 1, "应找到 1 条高昂时段");
    assert!(
        high[0].note.contains("投资"),
        "应找回高昂时段: {:?}",
        high[0].note
    );

    // 90 天前记录应被 recall_window_ms (30 天) 排除
    engine.record(apeireth_organ::emotion_memory::MoodRecord::with_timestamp(
        -0.8,
        0.5,
        MoodSource::TextSignal,
        "很久以前",
        now - 90 * 24 * 3600 * 1000,
    ));
    let low2 = engine.recall_by_mood_at(now, -0.8, 0.2, 5);
    assert_eq!(low2.len(), 1, "90 天前记录应被窗口排除");
}

// ============================================
// Test 4 (#[ignore]): trait 边界 + LlmFactory 注入 shape (per 子代理 Q1/R2/R3 同模式)
// ============================================

/// **0 装诚实**: trait 边界在 LlmFactory 注入下能编译/构造.
///
/// v1 emotion_memory 是确定性无 LLM, 当前 trait `process()` 不调 LLM. 此 #[ignore]
/// test 的目的: 验证 trait shape 在 NoopLlmFactory 注入下不变.
#[tokio::test]
#[ignore = "manual run: cargo test -p apeireth-organ --test emotion_memory -- --ignored; 验 trait shape with NoopLlmFactory"]
async fn emotion_organ_trait_shape_with_noop_llm_factory() {
    let _noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    // EmotionOrgan 不需要 LLM 注入 (v1 确定性); 验 trait shape 完整
    let organ = EmotionOrgan::new();
    let output = organ
        .process(make_input(vec!["valence=0.1".into()]))
        .await
        .expect("process ok");
    match output {
        OrganOutput::Emotion { .. } => {
            // OK: trait 边界 + factory 不依赖路径都 work
        }
        other => panic!("expected Emotion output, got {other:?}"),
    }
    // 0 装诚实: llm_factory() 返 None (trait 默认)
    assert!(
        organ.llm_factory().is_none(),
        "v1 emotion_memory 是确定性无 LLM, trait 默认 None (0 装诚实)"
    );
}