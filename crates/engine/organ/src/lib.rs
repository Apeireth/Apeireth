//! P-arch (2026-08-28): 9-organ 真移植 (v2 形态, per `v2-unabsorbed-features.md` 9-organ).
//!
//! v1 `apeireth-companion` 9 organ 散落在 `lib.rs` 顶层 mod (curiosity / emotion_memory /
//! hypothesis / value_cases / world_model / causal_world_model / emergence / 等), 内部
//! if-else 散落. v2 形态:
//!
//! - `OrganTrait` 在 `apeireth-plugin` (foundation) — trait 边界
//! - impl 在本 crate (`apeireth-organ`) — engine 层真接
//! - 单向依赖: `apeireth-organ` → `apeireth-plugin` → `apeireth-core` (与 plugin 体系一致)
//!
//! **v2.0-rc.1 真实现进度** (per 任务"真生产前阻塞 #1: 9 organ 至少 1 真移植"):
//! - ✅ **E4 Curiosity** (`curiosity::CuriosityOrgan`) — 1:1 翻译 v1 真实现 (子代理 Q1)
//! - ✅ **F4 Hypothesis** (`hypothesis::HypothesisOrgan`) — 1:1 翻译 v1 真实现 (子代理 R2, 2026-08-28)
//! - ✅ **F6 Value Cases** (`value_cases::ValueCasesOrgan`) — 1:1 翻译 v1 真实现 (子代理 R3, 2026-08-28)
//! - ✅ **F1 Emotion Memory** (`emotion_memory::EmotionOrgan`) — 1:1 翻译 v1 真实现 (子代理 R1, 2026-08-28)
//! - ✅ **W1 World Model** (`world_model::WorldModelOrgan`) — 1:1 翻译 v1 真实现, **真接 LLM** (子代理 R4, 2026-08-28)
//! - ✅ **W2 Causal World Model** (`causal_world_model::CausalWorldModelOrgan`) — 1:1 翻译 v1 真实现, **真接 LLM MCTS** (子代理 R5, 2026-08-28)
//! - ✅ **W3 Causal Edge Mining** (`causal_world_model_edges::EdgeMinerOrgan`)
//!   — 1:1 翻译 v1 `MineCausalEdges` 被动路径, 确定性无 LLM (子代理 R6, 2026-08-28)
//! - ✅ **E7 Emergence** (`emergence::EmergenceOrgan`) — 1:1 翻译 v1 `EmergenceLoop`
//!   rhythm+boundary 8 重门控真实现; **子代理 R7 独立判断**: 任务说明里的"5 状态机
//!   Idle/Draft/Proposed/Ratified/Active"实际来自 v1 `apeireth-evolution::state`, **不**是
//!   `apeireth-companion::emergence` 内部状态机. 本模块**不发明**v1 没有的状态机;
//!   `PolicyStage` 是 v2 前向声明 (future apeireth-evolution 接入预留). 0 装诚实:
//!   `llm_factory()` 返 None, 决策路径严格确定性, **不假装"E7 always speak"** (子代理 R7, 2026-08-28)
//! - ✅ **Memory Merger** (`memory::MemoryMergerOrgan`) — 跨 8 organ 记忆合并抽象
//!   (子代理 R8, 2026-08-28). **0 装诚实标**: v1 `runtime_brain.rs` 没有 `MemoryMerger`
//!   模块; v2 是新抽象, 借鉴 v1 `MemoryExtractionService` dedup/weight/persist 算法骨架 1:1 翻译.
//!
//! **0 装 PASS (per task + 子代理 R 同步)**:
//! - 本 crate E4 / F4 / F6 / F1 / W1 / **W2** / W3 / E7 / Memory 真实现; **9 organ 全实装**.
//! - 不假装"9 organ 全实装" 已是事实 (per O-5 不假装锚, R11 LOCKED body.rs 0 字节占位同样纪律).
//!
//! **承接 (per 任务 §5)**:
//! - 子代理 D actionable #1 真兑现 (Experience 保守版是真接 LLM 真接路线)
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入)
//! - Curiosity 与 Council 共享 `LlmFactory` 接口 (E4 trait 默认 `llm_factory()` 返 None,
//!   因 v1 curiosity 真实现是确定性无 LLM, 不假装能调 — 0 装诚实)
//!
//! **3 阶审查** (O-6 锚 9):
//! 1. 总体: 与 LlmFactory / PerceptionInput 同位, 9 organ 走统一 trait 边界.
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致).
//! 3. 架构: runtime 拿 `Arc<dyn OrganTrait>`, 9 organ trait 抽象统一入口语义.
//!
//! **v1 compat**: trait 是新增, 0 破现有 consumer. v1 `apeireth-companion::curiosity`
//! 仍在 `legacy/donor/` (workspace exclude), v2 真生产路径走本 crate.

// Re-export plugin 层 trait + 类型 (per `apeireth-perception` 同模式)
pub use apeireth_plugin::organ::{
    CausalEdge, CuriosityDepth, CuriosityTarget, EmotionTrend, OrganError, OrganInput, OrganKind,
    OrganOutput, OrganTrait, ValueVerdict,
};

pub mod causal_world_model;
pub mod causal_world_model_edges;
pub mod curiosity;
pub mod emergence;
pub mod emotion_memory;
pub mod hypothesis;
pub mod memory;
pub mod morphology;
pub mod tone;
pub mod value_cases;
pub mod world_model;

pub use morphology::{
    classify as classify_query_morphology, crawl_budget, sanitize_temperature, MorphologyVerdict,
    RetrievalMode,
};
pub use tone::{
    deliberation_intensity, emotion_tone, organ_tone, tone_hint, BondCharacterSnapshot,
    DeliberationEcho, EmotionToneStyle, ToneError,
};

// ============================================
// 0 装 Noop stub (无 organ 占位: 9 organ 全实装)
// ============================================

/// 0 装 PASS: 占位 organ (无 — 9 organ 全实装).
///
/// **为何**: v2.0-rc.1 E4/F4/F6/F1/W1/W2/W3/E7/Memory 全实装. NoopOrgan 保留作为兜底
/// (runtime 启动时如果配置了未注册的 organ, 注入此 stub), 调用 `process` 时显式返
/// `NotImplemented`, 不假装 organ 在工作.
pub struct NoopOrgan {
    kind: OrganKind,
}

impl NoopOrgan {
    /// 构造占位 organ
    pub fn new(kind: OrganKind) -> Self {
        Self { kind }
    }
}

#[async_trait::async_trait]
impl OrganTrait for NoopOrgan {
    fn name(&self) -> &'static str {
        match self.kind {
            OrganKind::W1 => "W1 World Model (should use WorldModelOrgan, not Noop)",
            OrganKind::W2 => "W2 Causal World Model (should use CausalWorldModelOrgan, not Noop)",
            OrganKind::W3 => "W3 Causal Edge Mining (should use EdgeMinerOrgan, not Noop)",
            OrganKind::E4 => "E4 Curiosity (should use CuriosityOrgan, not Noop)",
            OrganKind::F4 => "F4 Hypothesis (should use HypothesisOrgan, not Noop)",
            OrganKind::F1 => "F1 Emotion Memory (should use EmotionOrgan, not Noop)",
            OrganKind::F6 => "F6 Value Cases (should use ValueCasesOrgan, not Noop)",
            OrganKind::E7 => "E7 Emergence (should use EmergenceOrgan, not Noop)",
            OrganKind::Memory => "Memory Merger (should use MemoryMergerOrgan, not Noop)",
        }
    }

    fn organ_id(&self) -> OrganKind {
        self.kind
    }

    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 0 装 PASS: 显式标缺, 不假装 organ 在工作.
        Err(OrganError::NotImplemented(self.kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;

    fn empty_input() -> OrganInput {
        use apeireth_core::kernel::memory::Episode;
        let ep = Episode {
            id: "test-episode-0".into(),
            session_id: SessionId::new().to_string(),
            role: "user".into(),
            content: "".into(),
            timestamp: 0,
        };
        OrganInput::new(ep, vec![])
    }

    /// 0 装 PASS: 9 organ IDs 走 NoopOrgan 都返 NotImplemented (除 E4 — 警示而已, 也返 NotImplemented)
    #[tokio::test]
    async fn noop_organ_all_kinds_return_not_implemented() {
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
            let organ = NoopOrgan::new(*k);
            let result = organ.process(empty_input()).await;
            match result {
                Err(OrganError::NotImplemented(returned_kind)) => {
                    assert_eq!(returned_kind, *k, "organ kind must round-trip");
                }
                other => panic!("{k:?} must return NotImplemented, got {other:?}"),
            }
        }
    }

    /// v1 compat: re-export 9 organ + OrganTrait 完整 (编译通过 = trait 边界完整)
    #[test]
    fn re_exports_complete() {
        fn _check_trait<T: OrganTrait>() {}
        _check_trait::<NoopOrgan>();
        // OrganInput / OrganOutput / OrganKind 9 variant 全可达 (编译通过)
        let _kinds = [
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
    }
}
