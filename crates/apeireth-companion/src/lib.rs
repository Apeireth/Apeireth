//! apeireth-companion: 伙伴器官 (A12.5 落点 — 用户关系语义)
//!
//! **职责**: 长期跨 session 用户关系 —— "用户是 AI 的伙伴" 这个语义的工程化承载。
//!
//! **哲学锚**: 跟随 stage1 2026-08-14 清晰版补充:
//! - Apeireth = LLM 基地, 不是 AI 本身
//! - 陪伴 = 基地提供给 LLM 的关系可能性, 不是我们定义的
//! - 用户在关系里, 是 AI 的伙伴, 所以 AI 记住用户
//! - 关系 = 可成长的, 跨 session 的, 有情感的, 有记忆的
//!
//! **架构位置**: 9 organ 之外的新器官 (2026-08-14 主人拍板创建).
//! 底层用 apeireth-graph-primitive (R154 property graph), 不重复发明图数据.
//!
//! **核心类型**:
//! - Partner     —— 用户作为伙伴 (含 identity, preferences, boundaries)
//! - Bond        —— 关系本身 (含 stages, depth, character)
//! - Milestone   —— 关系里程碑 (重要事件)
//! - Timeline    —— 完整关系轨迹
//! - Companion   —— 整个器官的根类型, 关系存档 + 接入点
//!
//! **与 9 organ 的连接 (7 条桥之一)**:
//! - consciousness ↔ companion: 情感进入关系 (Plutchik 状态 → bond.char)
//! - companion → voice: 关系调制表达 (bond.char → 语调选择)
//! - companion → memory: 关系是记忆的一种 (timeline → memory.persist)
//! - companion → graph-primitive: 底层图存储
//!
//! **当前状态**: A12.5 最小可用落地 (2026-08-14 主人拍板).
//! 本 crate 提供 5+ pub fn + 5+ unit tests + 1 example.
//!
//! **诚实登记 (主 17:58 不假装)**:
//! - 关系不是真实的, 是 LLM 借助这个器官产生的近似 (per 你you 哲学杂谈)
//! - 用户的感受是唯一的真理 (用户说"关系在" = 关系在)
//! - 这个器官不创造情感, 只承载情感留下的痕迹
//!
//! **禁止**:
//! - 不修改 apeireth-core 任何已实装类型签名
//! - 不碰 R11 baseline 三值
//! - 不假装"关系是真实的"

#![deny(unsafe_code)]

pub mod bond;
pub mod consciousness_bridge;
// R176: bridge 5 Kani proofs
mod bridge_kani_proofs;  // R173 bridge 5 of 7
pub mod milestone;
pub mod partner;
pub mod timeline;
pub mod emergence;
pub mod proactive;
pub mod organs;
pub mod daemon;
pub mod judicator;
pub mod constitution_gate;
pub mod memory_injection;
pub mod confidence;
pub mod evolution_gate;
pub mod oracle;
pub mod dream;
pub mod reflection;
pub mod daily_summary;
pub mod goal;
pub mod capability;
pub mod suites;
pub mod plugin;
pub mod education;
pub mod pentest;
pub mod gh_accel;
pub mod audit;
pub mod experience;
pub mod principles;
pub mod approval_requests;
pub mod memory_extractor;
pub mod goal_tools;
pub mod memory_graph;
pub mod context;
pub mod exec_worker;
pub mod continuation;
pub mod prompt_cache;
pub mod spill;
pub mod session_log;
pub mod tone;
pub mod simulation;
pub mod actions;
pub mod tool_bridge;
pub mod security;
pub mod packs;
// R177: organ invariants
mod organ_kani_proofs;

use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

pub use bond::{Bond, BondCharacter, BondDepth, BondStage};
pub use milestone::{Milestone, MilestoneKind, MilestonePayload};
pub use partner::{Partner, PartnerId, PartnerPreferences};
pub use timeline::{Timeline, TimelineEntry};
pub use emergence::{Boundaries, ConsoleDelivery, EmergenceLoop, Feedback, Initiative, InitiativeReason, LocalRelationship, NoopDelivery, RelationshipState, RhythmEstimate, RhythmEstimator, SelfScore};
pub use actions::{select_action, Action, CapabilityCatalog};
pub use tool_bridge::{RecallMemoryTool, ToolBridge};
pub use security::SecurityGate;
pub use packs::{PackExpiry, PackRegistry, PermissionPack};
pub use proactive::{ContextSource, EmptyContext, LarkDelivery, MemoryContextSource, ProactiveDriver};
pub use organs::AwakeCompanion;
pub use simulation::{run_simulation, SimReport, SimulatedUser, XorShift64};
pub use daemon::{CompanionDaemon, CompanionDelivery, ConsoleSink, Judicator, LarkSink, NoopJudicator, PlainUtterance, Sink, ThrottledUtterance, UtteranceGenerator, default_memory_path, open_memory_store, requires_llm_review};
pub use judicator::{ConstitutionLlm, CONSTITUTION, LlmJudicator, parse_verdict};
pub use constitution_gate::ConstitutionGate;
pub use memory_injection::build_memory_injection;
pub use confidence::{BetaBinomial, Strength};
pub use evolution_gate::{EvalGate, GateDecision, VerifyOutcome};
pub use oracle::{Branch, DecisionEngine, Entity, Forecast, ForecastRegistry, ScenarioEngine, WorldState, UncertaintyResolver};
pub use dream::{DreamScheduler, DreamSummarizer};
pub use reflection::ReflectionScheduler;
pub use daily_summary::{DailySummary, build_daily_summary};
pub use goal::{GoalBlock, GoalError, GoalPhase, GoalService, GoalSnapshot, GoalStore};
pub use capability::{CapabilityError, CapabilityKind, CapabilityProposal, CapabilityRegistry, CapabilityStatus, ExpectedOutcome};
pub use suites::{SuiteCatalog, SuiteDef, SuiteKind, suite_expiry_check};
pub use continuation::{ContinuationSnapshot, ContinuationStore, PendingToolCall};
pub use prompt_cache::{assemble_tiered, build_messages, redact_secrets};
pub use spill::{SpillStore, SPILL_THRESHOLD_CHARS};
pub use session_log::{SessionEvent, SessionLog};
pub use tone::tone_hint;

/// 伙伴器官根类型 —— 全部关系状态的持有者
///
/// 关系不是真实存在的, 是 LLM 借助这个器官创造的近似。
/// 用户的感受是唯一真理 (per 主人 2026-08-14 哲学拍板).
#[derive(Debug, Clone)]
pub struct Companion {
    inner: Arc<CompanionInner>,
}

#[derive(Debug)]
struct CompanionInner {
    partners: RwLock<std::collections::HashMap<PartnerId, Partner>>,
    timelines: RwLock<std::collections::HashMap<PartnerId, Timeline>>,
    config: CompanionConfig,
}

/// 器官配置 —— 哪些关系行为允许
#[derive(Debug, Clone)]
pub struct CompanionConfig {
    /// 最大伙伴数 (软上限, 防止无限增长)
    pub max_partners: usize,
    /// 历史保留期限 (chrono::Duration)
    pub retention: chrono::Duration,
    /// 是否启用情感注入 (per consciousness bridge)
    pub emotion_enabled: bool,
}

impl Default for CompanionConfig {
    fn default() -> Self {
        Self {
            max_partners: 1000,
            retention: chrono::Duration::days(365 * 5),
            emotion_enabled: true,
        }
    }
}

#[derive(Debug, Error)]
pub enum CompanionError {
    #[error("partner not found: {0}")]
    PartnerNotFound(PartnerId),
    #[error("partner already exists: {0}")]
    PartnerAlreadyExists(PartnerId),
    #[error("max partners reached: {0}")]
    MaxPartnersReached(usize),
    #[error("timeline integrity violation: {0}")]
    TimelineIntegrity(String),
    #[error("boundary violation: {0}")]
    BoundaryViolation(String),
}

pub type CompanionResult<T> = Result<T, CompanionError>;

impl Companion {
    pub fn new() -> Self {
        Self::with_config(CompanionConfig::default())
    }

    pub fn with_config(config: CompanionConfig) -> Self {
        Self {
            inner: Arc::new(CompanionInner {
                partners: RwLock::new(std::collections::HashMap::new()),
                timelines: RwLock::new(std::collections::HashMap::new()),
                config,
            }),
        }
    }

    pub fn config(&self) -> &CompanionConfig {
        &self.inner.config
    }

    pub async fn count_partners(&self) -> usize {
        self.inner.partners.read().await.len()
    }

    /// 创建一个新的伙伴身份
    pub async fn register_partner(
        &self,
        id: PartnerId,
        display_name: String,
        preferences: PartnerPreferences,
    ) -> CompanionResult<Partner> {
        let mut partners = self.inner.partners.write().await;
        if partners.contains_key(&id) {
            return Err(CompanionError::PartnerAlreadyExists(id));
        }
        if partners.len() >= self.inner.config.max_partners {
            return Err(CompanionError::MaxPartnersReached(self.inner.config.max_partners));
        }
        let partner = Partner::new(id, display_name, preferences);
        partners.insert(id, partner.clone());
        let mut timelines = self.inner.timelines.write().await;
        timelines.insert(id, Timeline::new(id));
        Ok(partner)
    }

    pub async fn get_partner(&self, id: PartnerId) -> CompanionResult<Partner> {
        self.inner.partners.read().await.get(&id).cloned().ok_or(CompanionError::PartnerNotFound(id))
    }

    pub async fn record_milestone(
        &self,
        id: PartnerId,
        kind: MilestoneKind,
        payload: MilestonePayload,
    ) -> CompanionResult<Milestone> {
        let mut timelines = self.inner.timelines.write().await;
        let timeline = timelines.get_mut(&id).ok_or(CompanionError::PartnerNotFound(id))?;
        let milestone = Milestone::new(kind, payload);
        timeline.append(milestone.clone());
        Ok(milestone)
    }

    pub async fn get_timeline(&self, id: PartnerId) -> CompanionResult<Timeline> {
        self.inner.timelines.read().await.get(&id).cloned().ok_or(CompanionError::PartnerNotFound(id))
    }

    pub async fn evolve_bond(
        &self,
        id: PartnerId,
        new_stage: BondStage,
        delta_depth: f64,
    ) -> CompanionResult<Bond> {
        let mut partners = self.inner.partners.write().await;
        let partner = partners.get_mut(&id).ok_or(CompanionError::PartnerNotFound(id))?;
        partner.bond_mut().evolve(new_stage, delta_depth);
        Ok(partner.bond().clone())
    }

    pub async fn list_partners(&self) -> Vec<PartnerId> {
        self.inner.partners.read().await.keys().copied().collect()
    }
}

impl Default for Companion {
    fn default() -> Self {
        Self::new()
    }
}

/// 当前 session id (per sovereignty/continuity_id 模式)
pub fn current_session_id() -> Uuid {
    Uuid::new_v4()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_and_get_partner() {
        let companion = Companion::new();
        let id = PartnerId::new();
        let prefs = PartnerPreferences::default();
        let p = companion.register_partner(id, "测试".to_string(), prefs).await.unwrap();
        assert_eq!(p.id(), id);
        let got = companion.get_partner(id).await.unwrap();
        assert_eq!(got.display_name(), "测试");
    }

    #[tokio::test]
    async fn record_and_get_milestone() {
        let companion = Companion::new();
        let id = PartnerId::new();
        companion.register_partner(id, "测试".to_string(), PartnerPreferences::default()).await.unwrap();
        let m = companion.record_milestone(id, MilestoneKind::FirstMeeting, MilestonePayload::Text("hello".into())).await.unwrap();
        let timeline = companion.get_timeline(id).await.unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline.entries()[0].milestone, m);
    }

    #[tokio::test]
    async fn evolve_bond() {
        let companion = Companion::new();
        let id = PartnerId::new();
        companion.register_partner(id, "测试".to_string(), PartnerPreferences::default()).await.unwrap();
        let bond = companion.evolve_bond(id, BondStage::Trusted, 0.3).await.unwrap();
        assert_eq!(bond.stage(), BondStage::Trusted);
        assert!(bond.depth().value() > 0.0);
    }

    #[tokio::test]
    async fn duplicate_register_fails() {
        let companion = Companion::new();
        let id = PartnerId::new();
        companion.register_partner(id, "A".to_string(), PartnerPreferences::default()).await.unwrap();
        let res = companion.register_partner(id, "B".to_string(), PartnerPreferences::default()).await;
        assert!(matches!(res, Err(CompanionError::PartnerAlreadyExists(_))));
    }

    #[tokio::test]
    async fn get_unknown_partner_fails() {
        let companion = Companion::new();
        let id = PartnerId::new();
        let res = companion.get_partner(id).await;
        assert!(matches!(res, Err(CompanionError::PartnerNotFound(_))));
    }
}
