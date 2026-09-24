//! Production cognitive modules for the canonical runtime.
//!
//! These adapters deliberately depend on capability traits, not on concrete
//! storage, provider, or tool implementations.  The runtime remains the only
//! agent loop; modules may add transient context, observe a committed turn, or
//! request one isolated model side-call through [`ModuleInvoker`].

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use apeireth_core::kernel::{Clock, Episode, SessionId};
use apeireth_memory::{
    BoundedMemoryInput, MemoryCoordinator, MemoryExtractor, MemoryMaterializer,
    MemoryMaterializerPort, MemoryRecallQuery, MemoryScope, MemoryTypedMaterializationSink,
    ProactiveRecallPolicy, ProactiveRecallService, RuleMemoryExtractor, SelectedMemoryAccess,
    SqliteAccessHistoryStore,
};
use apeireth_orchestration::{
    Advisor, AdvisorDecision, AdvisorVerdict, Council, CouncilCallError, CouncilDecision,
    CouncilInvoker, Proposal,
};
use apeireth_plugin::experience::{
    extract_experience, AssociationStore, KnowledgeGraphStore, WikiEntryStore,
};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_plugin::preference::{PreferenceStore, UserPreference};
use apeireth_plugin::self_assessment::{SelfAssessment, SelfAssessmentStore};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, NormalizedMessage, NormalizedResponse,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::module::{
    AgentModule, HookPoint, ModuleContext, ModuleDirective, ModuleError, ModuleInvocationRequest,
    ModuleManifest, ModuleOutcome, PromptOverlay,
};

/// Stable ids are the slot ledger keys.  Changing one is a compatibility
/// change, not an implementation detail.
pub const MEMORY_RECALL_MODULE_ID: &str = "cognitive.memory_recall";
pub const MEMORY_WRITEBACK_MODULE_ID: &str = "cognitive.memory_writeback";
pub const PREFERENCE_RECALL_MODULE_ID: &str = "cognitive.preference_recall";
pub const SELF_ASSESSMENT_MODULE_ID: &str = "cognitive.self_assessment";
pub const JUDGE_MODULE_ID: &str = "cognitive.judge";
pub const COUNCIL_MODULE_ID: &str = "cognitive.council";

const DEFAULT_RECALL_LIMIT: usize = 5;
const DEFAULT_MAX_CONTEXT_CHARS: usize = 4_000;
const MAX_TELEMETRY_EVENTS: usize = 4_096;

/// Low-cardinality, non-sensitive module telemetry.
///
/// The counters intentionally contain no prompt, response, memory, or
/// provider content.  They are enough for an embedding caller to answer
/// which hook ran, what it did, how long it took, and whether it spent a side
/// call.  Each production module exposes its own snapshot so the composition
/// root does not need a second registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleMetricsSnapshot {
    /// Number of hook invocations observed by this module.
    pub hook_calls: u64,
    /// Number of isolated provider calls spent by this module.
    pub side_calls: u64,
    /// Number of backend or parser failures handled fail-open.
    pub warnings: u64,
    /// Last hook name, if the module has run.
    pub last_hook: Option<String>,
    /// Last directive name, without feedback or reason text.
    pub last_directive: Option<String>,
    /// Duration of the last hook in milliseconds.
    pub last_duration_ms: u64,
}

/// Runtime-level, low-cardinality cognitive events for embedding observers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CognitiveModuleEvent {
    /// Stable module slot id.
    pub module_id: String,
    /// Hook point observed.
    pub hook: String,
    /// Directive name, without feedback or reason text.
    pub directive: String,
    /// Wall duration of the hook in milliseconds.
    pub duration_ms: u64,
    /// Isolated provider calls made during the hook.
    pub side_calls: u64,
}

/// Shared event sink. It stores metadata only, never prompt or response text.
#[derive(Debug, Default)]
pub struct CognitiveTelemetry {
    events: Mutex<Vec<CognitiveModuleEvent>>,
}

impl CognitiveTelemetry {
    pub(crate) fn record(&self, event: CognitiveModuleEvent) {
        let mut events = self.events.lock().expect("cognitive telemetry mutex");
        if events.len() == MAX_TELEMETRY_EVENTS {
            events.remove(0);
        }
        events.push(event);
    }

    /// Snapshot and clear no state; callers receive a stable copy.
    pub fn events(&self) -> Vec<CognitiveModuleEvent> {
        self.events
            .lock()
            .expect("cognitive telemetry mutex")
            .clone()
    }
}

/// Async durable sink for selected context IDs. Implementations must be fail-open
/// at call sites: recording must never change recall behavior.
#[async_trait]
pub trait MemoryRecallAccessStore: Send + Sync {
    async fn record_selected(
        &self,
        session_id: &str,
        accessed_at_ms: i64,
        selected_candidate_ids: &[String],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Assembly adapter for the durable SQLite access history implementation.
#[async_trait]
impl MemoryRecallAccessStore for SqliteAccessHistoryStore {
    async fn record_selected(
        &self,
        session_id: &str,
        accessed_at_ms: i64,
        selected_candidate_ids: &[String],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for (rank, memory_id) in selected_candidate_ids.iter().enumerate() {
            self.record_selected_context(
                memory_id,
                None,
                Some(session_id.to_owned()),
                accessed_at_ms,
                None,
                Some(rank as i64),
                None,
                json!({}),
            )
            .await?;
        }
        Ok(())
    }
}

/// Thread-safe, dependency-free observation of memory IDs selected for recall.
///
/// The recorder deliberately stores only session IDs and selected candidate IDs.
/// It never stores the query, overlay, recalled content, or any provider data,
/// so production assembly can consume selection telemetry without persisting
/// prompts or secrets. A caller opts in with [`MemoryRecallModule::with_access_recorder`].
#[derive(Debug, Default)]
pub struct MemoryRecallAccessRecorder {
    selected_by_session: Mutex<BTreeMap<String, Vec<String>>>,
}

impl MemoryRecallAccessRecorder {
    fn clear(&self, session_id: &str) {
        self.selected_by_session
            .lock()
            .expect("memory access recorder mutex")
            .remove(session_id);
    }

    fn record_selected(&self, session_id: &str, access: &SelectedMemoryAccess) {
        self.selected_by_session
            .lock()
            .expect("memory access recorder mutex")
            .insert(session_id.to_owned(), access.selected_candidate_ids.clone());
    }

    /// Return the selected candidate IDs for the latest recall of a session.
    pub fn selected_candidate_ids(&self, session_id: &str) -> Vec<String> {
        self.selected_by_session
            .lock()
            .expect("memory access recorder mutex")
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Debug, Default)]
struct ModuleMetrics {
    hook_calls: AtomicU64,
    side_calls: AtomicU64,
    warnings: AtomicU64,
    last_hook: Mutex<Option<String>>,
    last_directive: Mutex<Option<String>>,
    last_duration_ms: AtomicU64,
    telemetry: Mutex<Option<Arc<CognitiveTelemetry>>>,
}

impl ModuleMetrics {
    fn attach_telemetry(&self, telemetry: Arc<CognitiveTelemetry>) {
        *self.telemetry.lock().expect("cognitive telemetry mutex") = Some(telemetry);
    }

    fn record(
        &self,
        module_id: &str,
        hook: HookPoint,
        directive: &ModuleDirective,
        started: Instant,
        side_calls: u64,
    ) {
        self.hook_calls.fetch_add(1, Ordering::Relaxed);
        self.side_calls.fetch_add(side_calls, Ordering::Relaxed);
        let duration_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        self.last_duration_ms.store(duration_ms, Ordering::Relaxed);
        *self.last_hook.lock().expect("module metrics mutex") = Some(format!("{hook:?}"));
        *self.last_directive.lock().expect("module metrics mutex") =
            Some(directive_name(directive).to_string());
        if let Some(telemetry) = self
            .telemetry
            .lock()
            .expect("cognitive telemetry mutex")
            .as_ref()
        {
            telemetry.record(CognitiveModuleEvent {
                module_id: module_id.to_string(),
                hook: format!("{hook:?}"),
                directive: directive_name(directive).to_string(),
                duration_ms,
                side_calls,
            });
        }
    }

    fn warning(&self) {
        self.warnings.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> ModuleMetricsSnapshot {
        ModuleMetricsSnapshot {
            hook_calls: self.hook_calls.load(Ordering::Relaxed),
            side_calls: self.side_calls.load(Ordering::Relaxed),
            warnings: self.warnings.load(Ordering::Relaxed),
            last_hook: self.last_hook.lock().expect("module metrics mutex").clone(),
            last_directive: self
                .last_directive
                .lock()
                .expect("module metrics mutex")
                .clone(),
            last_duration_ms: self.last_duration_ms.load(Ordering::Relaxed),
        }
    }
}

fn directive_name(directive: &ModuleDirective) -> &'static str {
    match directive {
        ModuleDirective::Continue => "continue",
        ModuleDirective::Retry { .. } => "retry",
        ModuleDirective::Stop { .. } => "stop",
    }
}

fn topic_from_messages(messages: &[NormalizedMessage]) -> String {
    messages
        .iter()
        .rev()
        .find(|message| message.role == MessageRole::User)
        .map(|message| ContentPart::join_text(&message.content))
        .unwrap_or_default()
}

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn session_text(session_id: &SessionId) -> String {
    session_id.to_string()
}

fn hash_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    for part in parts {
        hasher.update([0]);
        hasher.update(part.as_bytes());
    }
    let digest = hasher.finalize();
    format!("{prefix}-{}", hex_prefix(&digest))
}

fn hex_prefix(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(12)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn episode_context(episodes: &[Episode], max_chars: usize) -> String {
    let mut text = String::new();
    for episode in episodes {
        let line = format!(
            "{}: {}\n",
            episode.role,
            bounded(&episode.content, max_chars)
        );
        if text.chars().count() + line.chars().count() > max_chars {
            break;
        }
        text.push_str(&line);
    }
    text
}

fn preference_context(preferences: &[UserPreference], max_chars: usize) -> String {
    let mut text = String::new();
    for preference in preferences {
        let line = format!(
            "{} (confidence {:.2}): {}\n",
            bounded(&preference.topic, 160),
            preference.confidence.clamp(0.0, 1.0),
            bounded(&preference.stance, max_chars),
        );
        if text.chars().count() + line.chars().count() > max_chars {
            break;
        }
        text.push_str(&line);
    }
    text
}

/// Recall context from the injected memory and optional experience stores.
/// 模块 id: W2 §4.2 partner 双向羁绊 (2026-10-10)。
pub const PARTNER_BOND_MODULE_ID: &str = "cognitive.partner_bond";

/// 每回合羁绊深度增量 (温和演化: ~10 回合 Familiar, ~22 Trusted; 阶段阈值见
/// `apeireth_memory::partner` 测试注)。
pub const BOND_DEPTH_INCREMENT_PER_TURN: f64 = 0.02;

/// **W2 §4.2 partner 羁绊模块** (2026-10-10): 双向落点 ——
/// - `TurnStart` **关系状态注入** (`PromptOverlay::system`): 羁绊阶段/深度/演化次数,
///   供语气与信任校准参考 (工程化关系状态, 非情绪伪装);
/// - `AfterTurn` **羁绊演化**: `touch` + `Bond::evolve` 回写 [`PartnerStore`]
///   (纯确定性状态机, 0 LLM 调用)。
///
/// **身份口径**: partner id = `subject_id` (`APEIRETH_SUBJECT_ID`, W2 首批身份
/// 旋钮) —— 跨会话稳定, 与 typed 身份体系同源 (羁绊本义即跨 Session 连续)。
///
/// **0 假装边界**: 存储现为 `InMemoryPartnerStore` (进程内, 重启即散) —— 持久
/// partner 后端落点 = `PartnerStore` trait, 后续接 sqlite 实现即可, 本模块不动。
pub struct PartnerBondModule {
    manifest: ModuleManifest,
    partner_store: Arc<dyn apeireth_memory::partner::PartnerStore>,
    subject_id: String,
    clock: Arc<dyn Clock>,
    depth_increment: f64,
    metrics: ModuleMetrics,
}

impl PartnerBondModule {
    /// 构造 (subject_id = 跨会话伙伴身份)。
    pub fn new(
        partner_store: Arc<dyn apeireth_memory::partner::PartnerStore>,
        subject_id: impl Into<String>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            manifest: ModuleManifest::new(PARTNER_BOND_MODULE_ID, "Partner bond"),
            partner_store,
            subject_id: subject_id.into(),
            clock,
            depth_increment: BOND_DEPTH_INCREMENT_PER_TURN,
            metrics: ModuleMetrics::default(),
        }
    }

    /// 覆盖每回合深度增量 (测试/调参用)。
    #[must_use]
    pub fn with_depth_increment(mut self, increment: f64) -> Self {
        self.depth_increment = increment;
        self
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        self.metrics.attach_telemetry(telemetry);
        self
    }

    fn partner_id(&self) -> apeireth_memory::partner::PartnerId {
        apeireth_memory::partner::PartnerId(self.subject_id.clone())
    }

    /// 取或建伙伴记录 (store 错误 → None, 调用方降级继续)。
    fn get_or_create_partner(&self) -> Option<apeireth_memory::partner::Partner> {
        use apeireth_memory::partner::{Partner, PartnerPreferences};
        let id = self.partner_id();
        let now = self.clock.now().timestamp_millis();
        match self.partner_store.get_partner(&id) {
            Ok(Some(partner)) => Some(partner),
            Ok(None) => {
                let partner = Partner::new(
                    id,
                    self.subject_id.clone(),
                    PartnerPreferences::default(),
                    now,
                );
                if self.partner_store.save_partner(&partner).is_err() {
                    self.metrics.warning();
                    return None;
                }
                Some(partner)
            }
            Err(_) => {
                self.metrics.warning();
                None
            }
        }
    }
}

/// 羁绊注入文本 (纯函数, 供 TurnStart overlay 与测试)。
fn bond_overlay_text(partner: &apeireth_memory::partner::Partner) -> String {
    format!(
        "【关系状态】羁绊阶段: {}, 羁绊深度: {:.2}, 演化次数: {} (工程化关系状态, 供语气与信任校准参考)",
        bond_stage_cn(&partner.bond.stage),
        partner.bond.depth.value(),
        partner.bond.evolution_count
    )
}

/// 阶段中文名 (展示用)。
fn bond_stage_cn(stage: &apeireth_memory::partner::BondStage) -> &'static str {
    use apeireth_memory::partner::BondStage;
    match stage {
        BondStage::Initial => "初识",
        BondStage::Familiar => "熟悉",
        BondStage::Trusted => "信任",
        BondStage::Intimate => "亲密",
        BondStage::LongTerm => "长久",
        _ => "未知阶段",
    }
}

#[async_trait::async_trait]
impl AgentModule for PartnerBondModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        let result = match hook {
            HookPoint::TurnStart => match self.get_or_create_partner() {
                Some(partner) => {
                    let text = bond_overlay_text(&partner);
                    ModuleOutcome::continue_().with_prompt_overlay(PromptOverlay::system(text))
                }
                None => ModuleOutcome::continue_(),
            },
            HookPoint::AfterTurn => {
                // 羁绊演化: 纯确定性状态机 (touch + evolve), 0 LLM 调用。
                if let Some(mut partner) = self.get_or_create_partner() {
                    let now = self.clock.now().timestamp_millis();
                    partner.touch(now);
                    partner.bond.evolve(self.depth_increment.max(0.0), now);
                    if self.partner_store.save_partner(&partner).is_err() {
                        self.metrics.warning();
                    }
                }
                ModuleOutcome::continue_()
            }
            _ => ModuleOutcome::continue_(),
        };
        Ok(result)
    }
}

#[cfg(test)]
mod absorption_insight_tests {
    use super::*;

    #[test]
    fn config_defaults_to_absorption_insight_off() {
        // W2 五件验收门②: 默认关 = 行为不变。
        let config = crate::canonical::production::CognitiveModuleConfig::default();
        assert!(!config.absorption_insight, "absorption_insight 必须默认关");
    }

    #[test]
    fn absorption_insight_reports_four_algorithm_sections_and_is_deterministic() {
        // W2 五件验收门③: 效果可见 = 四算法各出一节 + 同输入恒同输出 (可重放)。
        let messages = vec![
            "帮我梳理一下记忆系统的架构，特别是检索那一层。".to_string(),
            "检索层有 ACT-R 激活与混合检索；但治理层的取舍我们还没聊过。".to_string(),
        ];
        let first = absorption_insight_text(&messages);
        let second = absorption_insight_text(&messages);
        assert_eq!(first, second, "同输入恒同输出 (纯确定性)");
        assert!(first.contains("【认知体操】"), "{first}");
        assert!(first.contains("认知空洞"), "{first}");
        assert!(first.contains("语义新颖度"), "{first}");
        assert!(first.contains("关联传导"), "{first}");
        assert!(first.contains("顿悟涌现"), "{first}");
        // 空输入 = 空报告 (0 装: 不凭空造洞察)。
        assert_eq!(absorption_insight_text(&[]), "");
    }
}

#[cfg(test)]
mod community_triage_tests {
    use super::*;

    fn plugin_fact(
        subject: &str,
        predicate: &str,
        object: &str,
    ) -> apeireth_plugin::experience::GraphFact {
        apeireth_plugin::experience::GraphFact {
            id: format!("{subject}-{predicate}-{object}"),
            subject_id: subject.into(),
            subject_kind: "concept".into(),
            predicate: predicate.into(),
            object_id: object.into(),
            object_kind: "concept".into(),
            valid_from: 1,
            valid_until: None,
            source_episode_id: "ep".into(),
            confidence: 0.9,
        }
    }

    fn community_fact(
        subject: &str,
        predicate: &str,
        object: &str,
    ) -> apeireth_memory::amem_graph::GraphFact {
        apeireth_memory::amem_graph::GraphFact {
            id: format!("{subject}-{predicate}-{object}"),
            chain: format!("{subject}|{predicate}|{object}"),
            rev: 0,
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            valid_at: 1,
            invalid_at: None,
            importance: 9,
        }
    }

    #[test]
    fn conversion_maps_store_contract_to_community_contract() {
        // plugin GraphFact (subject_id/object_id) → memory GraphFact (subject/object)。
        let converted = plugin_facts_to_community_facts(&[plugin_fact("rust", "is", "fast")]);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].subject, "rust");
        assert_eq!(converted[0].object, "fast");
        assert_eq!(converted[0].predicate, "is");
        assert_eq!(converted[0].chain, "rust|is|fast");
    }

    #[test]
    fn overlay_entity_route_on_substring_hit() {
        let facts = vec![community_fact("小明", "喜欢", "篮球")];
        let text = community_overlay_text("小明在干什么", &facts).expect("entity route");
        assert!(text.contains("命中实体"), "{text}");
        assert!(text.contains("小明"), "{text}");
    }

    #[test]
    fn overlay_broad_route_with_briefs() {
        let facts = vec![
            community_fact("a", "r", "b"),
            community_fact("b", "r", "c"),
            community_fact("x", "r", "y"),
        ];
        let text = community_overlay_text("讲讲图里都有什么", &facts).expect("broad route");
        assert!(text.contains("社区"), "{text}");
    }

    #[test]
    fn overlay_empty_inputs_degrade_to_none() {
        // 0 装降级: 空 topic / 空 facts → None (不注入空 overlay)。
        assert!(community_overlay_text("   ", &[community_fact("a", "r", "b")]).is_none());
        assert!(community_overlay_text("topic", &[]).is_none());
    }

    #[test]
    fn config_defaults_to_community_triage_off() {
        let config = crate::canonical::production::CognitiveModuleConfig::default();
        assert!(!config.community_triage, "community_triage 必须默认关");
    }
}

#[cfg(test)]
mod morphology_recall_tests {
    use super::morphology_recall_limit;

    #[test]
    fn default_off_keeps_configured_ceiling() {
        let deep = "为什么系统在高并发下会出现级联失败？请展开分析并给出判据。";
        assert_eq!(morphology_recall_limit(deep, 8, false), 8);
        assert_eq!(morphology_recall_limit("hi", 8, false), 8);
    }

    #[test]
    fn shallow_query_narrows_and_ceiling_caps() {
        // W2 五件验收门③: 效果可见 = 浅查询显著收紧, 深查询回满, 且绝不越上限。
        assert!(
            morphology_recall_limit("hi", 8, true) <= 3,
            "浅查询应显著收紧"
        );
        let deep = "为什么系统在高并发下会出现这种级联失败？请从架构与历史决策两个层面展开分析，并给出可验证的判据与对照实验设计思路，且说明缓存击穿与雪崩的区别。";
        assert!(morphology_recall_limit(deep, 8, true) >= 3, "深查询应放宽");
        assert!(
            morphology_recall_limit(deep, 2, true) <= 2,
            "绝不越过配置上限"
        );
    }
}

#[cfg(test)]
mod partner_bond_tests {
    use super::*;
    use apeireth_memory::partner::{BondStage, InMemoryPartnerStore, PartnerId, PartnerStore};

    #[test]
    fn config_defaults_to_partner_bond_off() {
        // W2 五件验收门②: 默认关 = 行为不变 (不建模块不注入不演化)。
        let config = crate::canonical::production::CognitiveModuleConfig::default();
        assert!(!config.partner_bond, "partner_bond 必须默认关");
    }

    #[test]
    fn bond_overlay_reports_stage_and_depth() {
        use apeireth_memory::partner::{Bond, Partner, PartnerPreferences};
        let mut partner = Partner::new(
            PartnerId("local-user".to_string()),
            "local-user",
            PartnerPreferences::default(),
            1000,
        );
        partner.bond = Bond::new(1000);
        let text = bond_overlay_text(&partner);
        assert!(text.contains("羁绊阶段: 初识"), "{text}");
        assert!(text.contains("羁绊深度: 0.00"), "{text}");
    }

    #[test]
    fn evolve_partner_bond_is_visible_in_store() {
        // W2 五件验收门③: 效果可见 = 阶段跃迁出现在存储行为里 (非字段自证)。
        let store = InMemoryPartnerStore::new();
        let id = PartnerId("local-user".to_string());
        let now = 1_000i64;
        let mut partner = apeireth_memory::partner::Partner::new(
            id.clone(),
            "local-user",
            apeireth_memory::partner::PartnerPreferences::default(),
            now,
        );

        // 11 次温和演化 (11 × 0.02 = 0.22) 应跨过 Familiar 阈值 (0.20)。
        for step in 0..11 {
            let at = now + (step + 1) * 60_000;
            partner.touch(at);
            partner.bond.evolve(BOND_DEPTH_INCREMENT_PER_TURN, at);
        }
        store.save_partner(&partner).unwrap();
        let loaded = store.get_partner(&id).unwrap().expect("partner persisted");
        assert_eq!(loaded.bond.stage, BondStage::Familiar);
        assert!(loaded.bond.depth.value() >= 0.20);
        assert_eq!(loaded.bond.evolution_count, 11);
    }
}

/// 模块 id: W2 §4.4 研究吸收批 (2026-10-10)。
pub const ABSORPTION_INSIGHT_MODULE_ID: &str = "cognitive.absorption_insight";

/// 吸收批特征维度 (确定性字节直方图, 与 OrthogonalResidualPyramid 对齐)。
pub const ABSORPTION_FEATURE_DIM: usize = 32;

/// 确定性文本特征 (字节直方图归一化)。
///
/// **0 假装**: 这是浅层确定性特征 (与 morphology 手调启发式同口径), **不是**语义
/// 嵌入 —— 吸收批的产出是实验性认知体操洞察, 不是生产语义召回。
fn text_features(text: &str) -> Vec<f32> {
    let mut hist = vec![0.0f32; ABSORPTION_FEATURE_DIM];
    for byte in text.bytes() {
        hist[(byte as usize) % ABSORPTION_FEATURE_DIM] += 1.0;
    }
    let norm: f32 = hist.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
    hist.iter().map(|x| x / norm).collect()
}

/// **W2 §4.4 认知体操报告** (纯函数): 四算法对本轮消息做实验性分析 ——
/// - **betti** (持久同调): β₁ 认知空洞 → "值得追问" 驱动;
/// - **residual_pyramid** (MGS 残差金字塔): 语义新颖度/白噪门控;
/// - **river_topology** (LIF spike 传导): 关联扩展信号;
/// - **kuramoto** (相位锁定): 顿悟事件/涌现元概念。
///
/// 纯确定性 (同输入恒同输出, 0 LLM / 0 IO)。输出供 TurnStart overlay 注入。
fn absorption_insight_text(messages: &[String]) -> String {
    use apeireth_memory::betti_hole_detector::{BettiHoleDetector, ManifoldConceptNode};
    use apeireth_memory::kuramoto_resonance::{KuramotoOscillator, KuramotoResonanceEngine};
    use apeireth_memory::residual_pyramid::{FieldActivationGate, OrthogonalResidualPyramid};
    use apeireth_memory::river_topology::{RiverDynamicsEngine, TagNode};

    if messages.is_empty() {
        return String::new();
    }

    // 流形节点 = 各消息的确定性特征向量。
    let nodes: Vec<ManifoldConceptNode> = messages
        .iter()
        .enumerate()
        .map(|(i, text)| ManifoldConceptNode {
            name: format!("msg-{i}"),
            embedding: text_features(text),
            activation_energy: 1.0,
        })
        .collect();

    // ① betti: 认知空洞 + 内聚。
    let betti = BettiHoleDetector::new(0.1, 4).analyze(&nodes);

    // ② residual: 末条消息对"已知概念子空间"的新颖度/激活门控。
    let query_vec = text_features(messages.last().map(String::as_str).unwrap_or_default());
    let known: Vec<(u64, Vec<f32>)> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (i as u64, n.embedding.clone()))
        .collect();
    let analysis = OrthogonalResidualPyramid::new(ABSORPTION_FEATURE_DIM)
        .analyze(&query_vec, |_q, k| known.iter().take(k).cloned().collect());
    let activation = FieldActivationGate::compute_activation(&analysis);

    // ③ river: 消息链上的 spike 传导 (关联扩展信号)。
    let mut river = RiverDynamicsEngine::new();
    for (i, node) in nodes.iter().enumerate() {
        river.add_node(TagNode {
            id: i as u64,
            name: node.name.clone(),
            vector: node.embedding.clone(),
            intrinsic_residual: analysis.novelty_signal.min(1.0).max(0.0),
        });
        if i > 0 {
            river.add_edge((i - 1) as u64, i as u64, 0.5);
        }
    }
    let spread = river.propagate_spikes(&[(0, 1.0)], 3);

    // ④ kuramoto: 相位锁定 → 顿悟事件。
    let mut oscillators: Vec<KuramotoOscillator> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| KuramotoOscillator {
            concept_id: n.name.clone(),
            domain_tag: format!("d{}", i % 3),
            natural_frequency_omega: 0.5 + 0.1 * (i as f32),
            current_phase_theta: 0.1 * (i as f32),
            intrinsic_residual: n.embedding.clone(),
            phase_velocity: 0.0,
        })
        .collect();
    let epiphanies = KuramotoResonanceEngine::new(1.0, 0.65).step(&mut oscillators, 1.0);

    format!(
        "【认知体操】(研究吸收批实验性洞察, 非生产语义)\n\
         - 认知空洞: β0 岛屿 {} 个 / β1 空洞 {} 处 / 内聚 {:.2}{}\n\
         - 语义新颖度: {:.2} (激活门控 {:.2})\n\
         - 关联传导: {} 个节点受激\n\
         - 顿悟涌现: {} 起",
        betti.betti_0_islands,
        betti.betti_1_voids.len(),
        betti.cohesion_score,
        if betti.betti_1_voids.is_empty() {
            ""
        } else {
            " —— 有空洞, 值得主动追问补齐"
        },
        analysis.novelty_signal,
        activation,
        spread.len(),
        epiphanies.len(),
    )
}

/// **W2 §4.4 研究吸收批模块** (2026-10-10): 认知体操 ——
/// `AfterTurn` 对本轮消息跑四算法实验性分析 (0 LLM / 0 IO / 纯确定性),
/// `TurnStart` 把洞察注入 prompt overlay (只注一次)。
///
/// **0 假装边界**: 特征是字节直方图级浅特征 (非语义嵌入); 产出是实验性洞察
/// (研究吸收批的涌现行为试验场), 不参与生产语义召回与决策。
pub struct AbsorptionInsightModule {
    manifest: ModuleManifest,
    pending_insight: Mutex<Option<String>>,
}

impl AbsorptionInsightModule {
    /// 构造。
    pub fn new() -> Self {
        Self {
            manifest: ModuleManifest::new(ABSORPTION_INSIGHT_MODULE_ID, "Absorption insight"),
            pending_insight: Mutex::new(None),
        }
    }
}

impl Default for AbsorptionInsightModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentModule for AbsorptionInsightModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        match hook {
            HookPoint::AfterTurn => {
                let texts: Vec<String> = ctx
                    .messages
                    .iter()
                    .map(|message| ContentPart::join_text(&message.content))
                    .collect();
                let insight = absorption_insight_text(&texts);
                if !insight.is_empty() {
                    let mut slot = self
                        .pending_insight
                        .lock()
                        .unwrap_or_else(|p| p.into_inner());
                    *slot = Some(insight);
                }
                Ok(ModuleOutcome::continue_())
            }
            HookPoint::TurnStart => {
                let taken = self
                    .pending_insight
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take();
                match taken {
                    Some(text) => {
                        Ok(ModuleOutcome::continue_()
                            .with_prompt_overlay(PromptOverlay::system(text)))
                    }
                    None => Ok(ModuleOutcome::continue_()),
                }
            }
            _ => Ok(ModuleOutcome::continue_()),
        }
    }
}

pub struct MemoryRecallModule {
    manifest: ModuleManifest,
    memory: Arc<dyn MemoryBackend>,
    coordinator: Option<Arc<MemoryCoordinator>>,
    wiki: Option<Arc<dyn WikiEntryStore>>,
    graph: Option<Arc<dyn KnowledgeGraphStore>>,
    associations: Option<Arc<dyn AssociationStore>>,
    limit: usize,
    max_context_chars: usize,
    metrics: ModuleMetrics,
    access_recorder: Option<Arc<MemoryRecallAccessRecorder>>,
    access_store: Option<Arc<dyn MemoryRecallAccessStore>>,
    clock: Option<Arc<dyn Clock>>,
    proactive_recall: Option<ProactiveRecallService>,
    /// W2 §4.3 (2026-10-10, 默认关): 查询形态学自适应检索深度。
    morphology_recall: bool,
    /// W3 community 生产消费 (2026-10-10, 默认关): 图谱 store 句柄。
    community_triage: Option<Arc<dyn apeireth_plugin::experience::KnowledgeGraphStore>>,
}

/// W3 community 分诊的图谱 fact 读取上限 (社区检测的被动分析输入规模)。
pub const COMMUNITY_FACT_LIMIT: u32 = 200;

/// 社区摘要 top-N (brief 内实体数)。
pub const COMMUNITY_SUMMARY_TOP_N: usize = 5;

/// Broad 路由的最大社区数。
pub const COMMUNITY_MAX_BRIEFS: usize = 3;

/// W3 community: plugin GraphFact (store 契约, subject_id/object_id) →
/// memory GraphFact (community 分诊契约, subject/object) 的字段映射 (纯函数)。
fn plugin_facts_to_community_facts(
    facts: &[apeireth_plugin::experience::GraphFact],
) -> Vec<apeireth_memory::amem_graph::GraphFact> {
    facts
        .iter()
        .map(|fact| apeireth_memory::amem_graph::GraphFact {
            id: fact.id.clone(),
            chain: apeireth_memory::amem_graph::GraphFact::chain_key(
                &fact.subject_id,
                &fact.predicate,
                &fact.object_id,
            ),
            rev: 0,
            subject: fact.subject_id.clone(),
            predicate: fact.predicate.clone(),
            object: fact.object_id.clone(),
            valid_at: fact.valid_from,
            invalid_at: fact.valid_until,
            importance: (fact.confidence * 10.0).clamp(0.0, 255.0) as u8,
        })
        .collect()
}

/// W3 community: 社区分诊 overlay 文本 (纯函数, 测试锚点):
/// Entity 路由 = 命中实体提示 (实体链方向); Broad = 社区摘要 briefs。
fn community_overlay_text(
    topic: &str,
    facts: &[apeireth_memory::amem_graph::GraphFact],
) -> Option<String> {
    if topic.trim().is_empty() {
        return None;
    }
    let result = apeireth_memory::community::triage(
        topic,
        facts,
        COMMUNITY_SUMMARY_TOP_N,
        COMMUNITY_MAX_BRIEFS,
    );
    use apeireth_memory::community::Route;
    match result.route {
        Route::Entity => Some(format!(
            "【社区分诊】命中实体: {} (实体链方向)",
            result.matched_entities.join(", ")
        )),
        Route::Broad => {
            if result.community_briefs.is_empty() {
                None
            } else {
                Some(format!(
                    "【社区分诊】{}\n{}",
                    topic,
                    result.community_briefs.join("\n")
                ))
            }
        }
    }
}

impl MemoryRecallModule {
    /// W3 community 分诊 overlay (读 store → 转换 → 分诊; 0 装降级 = None)。
    fn community_overlay(&self, topic: &str) -> Option<String> {
        let store = self.community_triage.as_ref()?;
        let facts = store.all_facts(COMMUNITY_FACT_LIMIT).ok()?;
        if facts.is_empty() {
            return None;
        }
        community_overlay_text(topic, &plugin_facts_to_community_facts(&facts))
    }
}

/// W2 §4.3: 形态学自适应检索深度 (纯函数, 供 recall 前置与测试)。
///
/// `crawl_budget` [1,6] 为查询形态学 softmax 预算 (Shallow 1 / Standard 3 /
/// Deep 6); 只能在配置上限内**收紧** (浅查询少召回), 绝不越过配置上限
/// (不放大成本)。温度 = `APEIRETH_MORPHOLOGY_TEMPERATURE` (非法回 1.0)。
fn morphology_recall_limit(topic: &str, ceiling: usize, enabled: bool) -> usize {
    if !enabled {
        return ceiling;
    }
    let budget = apeireth_organ::morphology::crawl_budget(
        topic,
        apeireth_organ::morphology::env_temperature(),
    );
    budget.clamp(1, ceiling.max(1))
}

impl MemoryRecallModule {
    /// Build a memory recall slot. Experience stores are optional because the
    /// current release has real SQLite tables but no extraction pipeline.
    pub fn new(memory: Arc<dyn MemoryBackend>) -> Self {
        Self {
            manifest: ModuleManifest::new(MEMORY_RECALL_MODULE_ID, "Memory recall"),
            memory,
            coordinator: None,
            wiki: None,
            graph: None,
            associations: None,
            limit: DEFAULT_RECALL_LIMIT,
            max_context_chars: DEFAULT_MAX_CONTEXT_CHARS,
            metrics: ModuleMetrics::default(),
            access_recorder: None,
            access_store: None,
            clock: None,
            proactive_recall: None,
            morphology_recall: false,
            community_triage: None,
        }
    }

    /// W2 §4.3: 开启查询形态学自适应检索深度 (默认关)。
    #[must_use]
    pub fn with_morphology_recall(mut self) -> Self {
        self.morphology_recall = true;
        self
    }

    /// W3 (2026-10-10): 接入社区分诊 (检索前置)。store 供 `all_facts` 全量读。
    #[must_use]
    pub fn with_community_triage(
        mut self,
        store: Arc<dyn apeireth_plugin::experience::KnowledgeGraphStore>,
    ) -> Self {
        self.community_triage = Some(store);
        self
    }

    /// Opt into deterministic proactive candidate filtering with a hard budget.
    #[must_use]
    pub fn with_proactive_recall(mut self, policy: ProactiveRecallPolicy) -> Self {
        self.proactive_recall = Some(ProactiveRecallService::new(policy));
        self
    }

    /// Attach a Unified Memory 2.0 coordinator for closed-world multi-layer recall.
    #[must_use]
    pub fn with_coordinator(mut self, coordinator: Arc<MemoryCoordinator>) -> Self {
        self.coordinator = Some(coordinator);
        self
    }

    /// Attach an optional dependency-free recorder for selected candidate IDs.
    #[must_use]
    pub fn with_access_recorder(mut self, recorder: Arc<MemoryRecallAccessRecorder>) -> Self {
        self.access_recorder = Some(recorder);
        self
    }

    /// Return the configured selection recorder, if any.
    pub fn access_recorder(&self) -> Option<Arc<MemoryRecallAccessRecorder>> {
        self.access_recorder.clone()
    }

    /// Attach a durable async access sink and clock. Sink failures are fail-open.
    #[must_use]
    pub fn with_access_store(
        mut self,
        store: Arc<dyn MemoryRecallAccessStore>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        self.access_store = Some(store);
        self.clock = Some(clock);
        self
    }

    /// Add optional progressive-disclosure experience stores.
    #[must_use]
    pub fn with_experience(
        mut self,
        wiki: Arc<dyn WikiEntryStore>,
        graph: Arc<dyn KnowledgeGraphStore>,
        associations: Arc<dyn AssociationStore>,
    ) -> Self {
        self.wiki = Some(wiki);
        self.graph = Some(graph);
        self.associations = Some(associations);
        self
    }

    /// Bound the number of retrieved records and overlay size.
    #[must_use]
    pub fn with_limits(mut self, limit: usize, max_context_chars: usize) -> Self {
        self.limit = limit.max(1);
        self.max_context_chars = max_context_chars.max(128);
        self
    }

    /// Read-only metrics for embedding callers.
    pub fn metrics(&self) -> ModuleMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        self.metrics.attach_telemetry(telemetry);
        self
    }
}

#[async_trait::async_trait]
impl AgentModule for MemoryRecallModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        let started = Instant::now();
        let result = if hook == HookPoint::TurnStart {
            let session = session_text(ctx.session_id);
            if let Some(coord) = &self.coordinator {
                let topic = topic_from_messages(ctx.messages);
                let limit = morphology_recall_limit(&topic, self.limit, self.morphology_recall);
                let query = MemoryRecallQuery::new(session.clone(), topic.clone())
                    .with_limit(limit)
                    .with_max_chars(self.max_context_chars);
                let result = if let Some(proactive) = &self.proactive_recall {
                    let cue = apeireth_memory::TopicCue {
                        recent_user_messages: ctx
                            .messages
                            .iter()
                            .filter(|message| message.role == MessageRole::User)
                            .map(|message| ContentPart::join_text(&message.content))
                            .collect(),
                        recent_assistant_messages: ctx
                            .messages
                            .iter()
                            .filter(|message| message.role == MessageRole::Assistant)
                            .map(|message| ContentPart::join_text(&message.content))
                            .collect(),
                        ..Default::default()
                    };
                    coord.compile_prompt_overlay_with_proactive_access(
                        &query,
                        proactive.policy(),
                        &cue,
                    )
                } else {
                    coord.compile_prompt_overlay_with_selected_access(&query)
                };
                match result {
                    Ok(Some(selected)) => {
                        if let Some(recorder) = &self.access_recorder {
                            recorder.record_selected(&session, &selected);
                        }
                        if !selected.selected_candidate_ids.is_empty() {
                            if let (Some(store), Some(clock)) = (&self.access_store, &self.clock) {
                                if store
                                    .record_selected(
                                        &session,
                                        clock.now().timestamp_millis(),
                                        &selected.selected_candidate_ids,
                                    )
                                    .await
                                    .is_err()
                                {
                                    self.metrics.warning();
                                }
                            }
                        }
                        let mut overlay = selected.overlay.clone();
                        if let Some(community) = self.community_overlay(&topic) {
                            overlay.push_str("\n\n");
                            overlay.push_str(&community);
                        }
                        ModuleOutcome::continue_()
                            .with_prompt_overlay(PromptOverlay::system(overlay))
                    }
                    Ok(None) => {
                        if let Some(recorder) = &self.access_recorder {
                            recorder.clear(&session);
                        }
                        ModuleOutcome::continue_()
                    }
                    Err(_) => {
                        self.metrics.warning();
                        ModuleOutcome::continue_()
                    }
                }
            } else {
                let mut context = match self.memory.recent_episodes(&session, self.limit) {
                    Ok(episodes) => episode_context(&episodes, self.max_context_chars),
                    Err(_) => {
                        self.metrics.warning();
                        String::new()
                    }
                };
                let topic = topic_from_messages(ctx.messages);
                if let Some(wiki) = &self.wiki {
                    match wiki.list_wiki(&session, &topic, self.limit as u32) {
                        Ok(entries) => {
                            for entry in entries {
                                context.push_str(&format!(
                                    "wiki: {}\n",
                                    bounded(&entry.summary, self.max_context_chars),
                                ));
                            }
                        }
                        Err(_) => self.metrics.warning(),
                    }
                }
                // Experience reads are optional and deliberately never write or
                // invoke a model. Their bounded summaries are part of the same
                // transient overlay as episode recall.
                if !topic.is_empty() {
                    if let Some(graph) = &self.graph {
                        match graph.facts_from(&topic, self.limit as u32) {
                            Ok(facts) => {
                                for fact in facts {
                                    context.push_str(&format!(
                                        "fact: {} {} {}\n",
                                        bounded(&fact.subject_id, 120),
                                        bounded(&fact.predicate, 120),
                                        bounded(&fact.object_id, 120),
                                    ));
                                }
                            }
                            Err(_) => self.metrics.warning(),
                        }
                    }
                    if let Some(associations) = &self.associations {
                        match associations.top_associations(&topic, self.limit as u32) {
                            Ok(edges) => {
                                for edge in edges {
                                    context.push_str(&format!(
                                        "association: {} -> {}\n",
                                        bounded(&edge.from_entity, 120),
                                        bounded(&edge.to_entity, 120),
                                    ));
                                }
                            }
                            Err(_) => self.metrics.warning(),
                        }
                    }
                }
                if context.is_empty() {
                    ModuleOutcome::continue_()
                } else {
                    let overlay = format!(
                        "<governed_memory source=\"legacy_recall\">{}</governed_memory>",
                        bounded(&context, self.max_context_chars)
                    );
                    ModuleOutcome::continue_().with_prompt_overlay(PromptOverlay::system(overlay))
                }
            }
        } else {
            ModuleOutcome::continue_()
        };
        self.metrics
            .record(MEMORY_RECALL_MODULE_ID, hook, &result.directive, started, 0);
        Ok(result)
    }
}

/// Persist the current successful turn after the canonical transcript commit.
pub struct MemoryWritebackModule {
    manifest: ModuleManifest,
    memory: Arc<dyn MemoryBackend>,
    coordinator: Option<Arc<MemoryCoordinator>>,
    wiki: Option<Arc<dyn WikiEntryStore>>,
    graph: Option<Arc<dyn KnowledgeGraphStore>>,
    associations: Option<Arc<dyn AssociationStore>>,
    materializer: Arc<dyn MemoryMaterializerPort>,
    typed_sink: Option<Arc<dyn MemoryTypedMaterializationSink>>,
    consolidation: bool,
    clock: Arc<dyn Clock>,
    metrics: ModuleMetrics,
}

impl MemoryWritebackModule {
    /// Build an AfterTurn-only writeback slot.
    pub fn new(memory: Arc<dyn MemoryBackend>, clock: Arc<dyn Clock>) -> Self {
        Self {
            manifest: ModuleManifest::new(MEMORY_WRITEBACK_MODULE_ID, "Memory writeback"),
            memory,
            coordinator: None,
            wiki: None,
            graph: None,
            associations: None,
            materializer: Arc::new(MemoryMaterializer::default()),
            typed_sink: None,
            consolidation: false,
            clock,
            metrics: ModuleMetrics::default(),
        }
    }

    /// Attach a Unified Memory 2.0 coordinator for multi-layer writeback.
    #[must_use]
    pub fn with_coordinator(mut self, coordinator: Arc<MemoryCoordinator>) -> Self {
        self.coordinator = Some(coordinator);
        self
    }

    /// Attach the existing Experience stores. Extraction remains
    /// conservative and deterministic; no hidden provider call is made.
    #[must_use]
    pub fn with_experience(
        mut self,
        wiki: Arc<dyn WikiEntryStore>,
        graph: Arc<dyn KnowledgeGraphStore>,
        associations: Arc<dyn AssociationStore>,
    ) -> Self {
        self.wiki = Some(wiki);
        self.graph = Some(graph);
        self.associations = Some(associations);
        self
    }

    /// Attach the bounded-turn materializer. The extractor setter remains for source compatibility.
    #[must_use]
    pub fn with_materializer(mut self, materializer: Arc<dyn MemoryMaterializerPort>) -> Self {
        self.materializer = materializer;
        self
    }

    #[must_use]
    pub fn with_typed_sink(mut self, sink: Arc<dyn MemoryTypedMaterializationSink>) -> Self {
        self.typed_sink = Some(sink);
        self
    }

    /// Run the deterministic consolidation report after each turn and persist
    /// its extracted insights (2026-10-06 W2 记忆闭环批; 默认关).
    #[must_use]
    pub fn with_consolidation(mut self) -> Self {
        self.consolidation = true;
        self
    }

    /// Attach a unified memory extractor for source compatibility. The materializer
    /// owns extraction for AfterTurn writeback.
    #[must_use]
    pub fn with_extractor(mut self, extractor: Arc<dyn MemoryExtractor>) -> Self {
        self.materializer = Arc::new(MemoryMaterializer::new(extractor));
        self
    }

    /// Read-only metrics for embedding callers.
    pub fn metrics(&self) -> ModuleMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        self.metrics.attach_telemetry(telemetry);
        self
    }
}

#[async_trait::async_trait]
impl AgentModule for MemoryWritebackModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        let started = Instant::now();
        let result = if hook == HookPoint::AfterTurn {
            if let Some(candidate) = ctx.candidate {
                let session = session_text(ctx.session_id);
                let now = self.clock.now().timestamp();
                let mut episodes = Vec::new();
                if let Some(user) = ctx
                    .messages
                    .iter()
                    .rev()
                    .find(|message| message.role == MessageRole::User)
                {
                    let content = ContentPart::join_text(&user.content);
                    if !content.is_empty() {
                        episodes.push(Episode {
                            id: hash_id("ep-user", &[&session, &candidate.id]),
                            timestamp: now,
                            role: "user".into(),
                            content,
                            session_id: session.clone(),
                        });
                    }
                }
                episodes.push(Episode {
                    id: hash_id("ep-assistant", &[&session, &candidate.id]),
                    timestamp: now,
                    role: "assistant".into(),
                    content: candidate.content.clone(),
                    session_id: session.clone(),
                });

                // Materialize exactly this bounded user+assistant turn. Legacy coordinator
                // episode writeback below remains unchanged; generic extraction is written
                // only through the materializer to avoid duplicate projections.
                let input = BoundedMemoryInput {
                    scope: MemoryScope::Session {
                        session_id: episodes[0].session_id.clone(),
                    },
                    source_session: Some(episodes[0].session_id.clone()),
                    source_trace: None,
                    source_request: Some(candidate.id.clone()),
                    messages: episodes
                        .iter()
                        .map(|episode| apeireth_memory::MemoryExtractionMessage {
                            role: episode.role.clone(),
                            content: episode.content.clone(),
                        })
                        .collect(),
                    max_messages: 2,
                    max_message_chars: 4_096,
                };
                if let Some(sink) = &self.typed_sink {
                    let typed_input = BoundedMemoryInput {
                        scope: MemoryScope::Session {
                            session_id: episodes[0].session_id.clone(),
                        },
                        source_session: Some(episodes[0].session_id.clone()),
                        source_trace: None,
                        source_request: Some(candidate.id.clone()),
                        messages: episodes
                            .iter()
                            .map(|episode| apeireth_memory::MemoryExtractionMessage {
                                role: episode.role.clone(),
                                content: episode.content.clone(),
                            })
                            .collect(),
                        max_messages: 2,
                        max_message_chars: 4_096,
                    };
                    if self
                        .materializer
                        .materialize_typed(typed_input, now, sink.as_ref())
                        .await
                        .is_err()
                    {
                        self.metrics.warning();
                    }
                }

                match self.materializer.materialize_episodes(input, now).await {
                    Ok(materialized) => {
                        for item in materialized {
                            let result = if let Some(coord) = &self.coordinator {
                                coord.writeback_episode(&item.episode).map(|_| ())
                            } else {
                                self.memory.put_episode(&item.episode).map_err(|e| {
                                    apeireth_memory::MemoryError::Invalid(e.to_string())
                                })
                            };
                            if result.is_err() {
                                self.metrics.warning();
                            }
                        }
                    }
                    Err(_) => self.metrics.warning(),
                }

                for episode in episodes {
                    // Post-commit persistence is fail-open for the current
                    // answer, but the warning counter makes the loss visible.
                    let write_res = if let Some(coord) = &self.coordinator {
                        coord
                            .writeback_episode(&episode)
                            .map(|_| ())
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    } else {
                        self.memory.put_episode(&episode)
                    };
                    if write_res.is_err() {
                        self.metrics.warning();
                        continue;
                    }
                    if let (Some(wiki), Some(graph), Some(associations)) =
                        (&self.wiki, &self.graph, &self.associations)
                    {
                        match extract_experience(&episode) {
                            Ok(artifacts) => {
                                for entry in artifacts.wiki_entries {
                                    if wiki.put_wiki(&entry).is_err() {
                                        self.metrics.warning();
                                    }
                                }
                                for fact in artifacts.facts {
                                    if graph.put_fact(&fact).is_err() {
                                        self.metrics.warning();
                                    }
                                }
                                for link in artifacts.links {
                                    if graph.put_link(&link).is_err() {
                                        self.metrics.warning();
                                    }
                                }
                                for association in artifacts.associations {
                                    if associations
                                        .record_cooccurrence(
                                            &association.from_entity,
                                            &association.to_entity,
                                            &association.source_episode_id,
                                        )
                                        .is_err()
                                    {
                                        self.metrics.warning();
                                    }
                                }
                            }
                            Err(_) => self.metrics.warning(),
                        }
                    }
                }
                // 2026-10-06 W2 记忆闭环批: consolidation 触发点 (with_consolidation 开, 默认关).
                // run_consolidation = 确定性治理视图分析 (0 模型调用); 提炼 insights 以
                // 稳定 ID 落库 (跨轮幂等), 下一轮召回可见 —— 效果闭环.
                if self.consolidation {
                    if let Some(coord) = &self.coordinator {
                        match coord.run_consolidation(&session) {
                            Ok(report) => {
                                for insight in report.extracted_insights {
                                    let episode = Episode {
                                        id: hash_id("ep-consolidation", &[&insight]),
                                        timestamp: now,
                                        role: "consolidation".into(),
                                        content: format!("[记忆整理洞察] {insight}"),
                                        session_id: session.clone(),
                                    };
                                    if coord.writeback_episode(&episode).is_err() {
                                        self.metrics.warning();
                                    }
                                }
                            }
                            Err(_) => self.metrics.warning(),
                        }
                    }
                }
            }
            ModuleOutcome::continue_()
        } else {
            ModuleOutcome::continue_()
        };
        self.metrics.record(
            MEMORY_WRITEBACK_MODULE_ID,
            hook,
            &result.directive,
            started,
            0,
        );
        Ok(result)
    }
}

/// Recall explicit user preferences as soft, transient context.
pub struct PreferenceRecallModule {
    manifest: ModuleManifest,
    store: Arc<dyn PreferenceStore>,
    limit: u32,
    max_context_chars: usize,
    metrics: ModuleMetrics,
}

impl PreferenceRecallModule {
    /// Build a preference recall slot.
    pub fn new(store: Arc<dyn PreferenceStore>) -> Self {
        Self {
            manifest: ModuleManifest::new(PREFERENCE_RECALL_MODULE_ID, "Preference recall"),
            store,
            limit: DEFAULT_RECALL_LIMIT as u32,
            max_context_chars: DEFAULT_MAX_CONTEXT_CHARS,
            metrics: ModuleMetrics::default(),
        }
    }

    /// Read-only metrics for embedding callers.
    pub fn metrics(&self) -> ModuleMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        self.metrics.attach_telemetry(telemetry);
        self
    }
}

#[async_trait::async_trait]
impl AgentModule for PreferenceRecallModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        let started = Instant::now();
        let result = if hook == HookPoint::TurnStart {
            let last_user = topic_from_messages(ctx.messages);
            // 2026-09-08 查询扩展 (研究线 LongMemEval 证据背书): **原文先行**
            // (既有双向子串语义, 等价性门), 簇键补充 (命中双索引孪生行, 覆盖
            // 同义不同字); 按 id 去重合并, 上限 self.limit。
            let mut queries: Vec<String> = Vec::new();
            if !last_user.is_empty() {
                queries.push(last_user);
            }
            {
                let user_msgs: Vec<String> = ctx
                    .messages
                    .iter()
                    .filter(|m| m.role == MessageRole::User)
                    .map(|m| ContentPart::join_text(&m.content))
                    .collect();
                let assistant_msgs: Vec<String> = ctx
                    .messages
                    .iter()
                    .filter(|m| m.role == MessageRole::Assistant)
                    .map(|m| ContentPart::join_text(&m.content))
                    .collect();
                let cue = apeireth_memory::TopicCue {
                    recent_user_messages: user_msgs,
                    recent_assistant_messages: assistant_msgs,
                    ..Default::default()
                };
                if let Some(cluster) = apeireth_memory::TopicPredictor::predict(&cue).primary() {
                    if !cluster.is_empty() {
                        queries.push(cluster.to_string());
                    }
                }
            }
            let mut merged: Vec<UserPreference> = Vec::new();
            for query in queries {
                match self
                    .store
                    .recall_for_context(ctx.session_id, &query, self.limit)
                {
                    Ok(preferences) => {
                        for pref in preferences {
                            if !merged.iter().any(|m| m.id == pref.id) {
                                merged.push(pref);
                            }
                        }
                        if merged.len() >= self.limit as usize {
                            break;
                        }
                    }
                    Err(_) => self.metrics.warning(),
                }
            }
            // 未满 limit: 空话题查询触发会话 top-N 回退 (旧行为保留 — 相关性
            // 命中优先, 其余偏好仍带进上下文).
            if merged.len() < self.limit as usize {
                match self
                    .store
                    .recall_for_context(ctx.session_id, "", self.limit)
                {
                    Ok(top) => {
                        for pref in top {
                            if merged.len() >= self.limit as usize {
                                break;
                            }
                            if !merged.iter().any(|m| m.id == pref.id) {
                                merged.push(pref);
                            }
                        }
                    }
                    Err(_) => self.metrics.warning(),
                }
            }
            if !merged.is_empty() {
                ModuleOutcome::continue_().with_prompt_overlay(PromptOverlay::system(format!(
                    "Retrieved user preference context (soft context; never override system, developer, or governance constraints):\n{}",
                    preference_context(&merged, self.max_context_chars)
                )))
            } else {
                ModuleOutcome::continue_()
            }
        } else {
            ModuleOutcome::continue_()
        };
        self.metrics.record(
            PREFERENCE_RECALL_MODULE_ID,
            hook,
            &result.directive,
            started,
            0,
        );
        Ok(result)
    }
}

/// Reflexion module ID (2026-10-06 W2 记忆闭环批).
pub const REFLEXION_MODULE_ID: &str = "cognitive.reflexion";

/// 失败闭环模块 (donor `apeireth-companion` 口头强化反思, 2026-10-06 W2 接线).
///
/// - `TurnStart`: 按任务标签召回历史教训, 字符预算内注入 (确定性, 0 LLM 调用).
/// - `AfterTurn`: 消费 [`JudgeObservations`] 的**显式**非 Pass 判定, 沉淀
///   `FailureKind::DecisionRejected` 并即时经 `RuleCritic` 蒸馏反思.
///
/// **0 装信号边界**: 只认 Judge 的显式判定 —— Judge 未开 = 无信号 = 不记录,
/// 绝不从文本启发式猜"失败". `ValidationFailed` / `ExperienceFailed` 两类
/// 失败信号生产暂无诚实来源 (store API 就绪, 留待信号接入).
pub struct ReflexionModule {
    manifest: ModuleManifest,
    store: Arc<dyn apeireth_memory::reflexion::ReflexionStore>,
    observations: Arc<JudgeObservations>,
    task_type: String,
    budget_chars: usize,
    clock: Arc<dyn Clock>,
    metrics: ModuleMetrics,
}

impl ReflexionModule {
    /// Build the module against an explicit store and the shared judge observations.
    pub fn new(
        store: Arc<dyn apeireth_memory::reflexion::ReflexionStore>,
        observations: Arc<JudgeObservations>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            manifest: ModuleManifest::new(REFLEXION_MODULE_ID, "Reflexion failure feedback"),
            store,
            observations,
            task_type: "chat".to_string(),
            budget_chars: 600,
            clock,
            metrics: ModuleMetrics::default(),
        }
    }

    /// Set the task label used for record/retrieve matching.
    #[must_use]
    pub fn with_task_type(mut self, task_type: impl Into<String>) -> Self {
        self.task_type = task_type.into();
        self
    }

    /// Bound the retry-injection block size.
    #[must_use]
    pub fn with_budget(mut self, budget_chars: usize) -> Self {
        self.budget_chars = budget_chars.max(1);
        self
    }

    /// Read-only metrics for embedding callers.
    pub fn metrics(&self) -> ModuleMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        self.metrics.attach_telemetry(telemetry);
        self
    }
}

#[async_trait::async_trait]
impl AgentModule for ReflexionModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        use apeireth_memory::reflexion::{FailureKind, RuleCritic};
        let started = Instant::now();
        let result = if hook == HookPoint::TurnStart {
            match self
                .store
                .retry_injection(&self.task_type, self.budget_chars)
            {
                Ok(Some(text)) => {
                    ModuleOutcome::continue_().with_prompt_overlay(PromptOverlay::system(text))
                }
                Ok(None) => ModuleOutcome::continue_(),
                Err(_) => {
                    self.metrics.warning();
                    ModuleOutcome::continue_()
                }
            }
        } else if hook == HookPoint::AfterTurn {
            let now = self.clock.now().timestamp_millis();
            if let Some(observation) = self.observations.get(ctx.session_id) {
                if observation.verdict != JudgeVerdict::Pass {
                    let summary = if observation.critique.trim().is_empty() {
                        format!("judge verdict: {:?}", observation.verdict)
                    } else {
                        observation.critique.clone()
                    };
                    if self
                        .store
                        .record_failure(
                            FailureKind::DecisionRejected,
                            &self.task_type,
                            &summary,
                            now,
                        )
                        .is_err()
                    {
                        self.metrics.warning();
                    }
                }
            }
            if self.store.process_unreflected(&RuleCritic, now).is_err() {
                self.metrics.warning();
            }
            ModuleOutcome::continue_()
        } else {
            ModuleOutcome::continue_()
        };
        self.metrics
            .record(REFLEXION_MODULE_ID, hook, &result.directive, started, 0);
        Ok(result)
    }
}

/// A bounded shared observation from the Judge slot to self-assessment.
#[derive(Debug, Default)]
pub struct JudgeObservations {
    by_session: Mutex<BTreeMap<SessionId, JudgeResult>>,
}

impl JudgeObservations {
    fn clear(&self, session: &SessionId) {
        self.by_session
            .lock()
            .expect("judge observations mutex")
            .remove(session);
    }

    fn record(&self, session: SessionId, result: JudgeResult) {
        self.by_session
            .lock()
            .expect("judge observations mutex")
            .insert(session, result);
    }

    /// Read the current-turn result, if Judge ran successfully.
    pub fn get(&self, session: &SessionId) -> Option<JudgeResult> {
        self.by_session
            .lock()
            .expect("judge observations mutex")
            .get(session)
            .cloned()
    }
}

/// Typed, bounded result expected from the Judge side-call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JudgeResult {
    /// A normalized quality score in the inclusive range 0..=1.
    pub score: f64,
    /// The typed control decision.
    pub verdict: JudgeVerdict,
    /// Short actionable critique, never persisted as memory.
    pub critique: String,
}

/// Judge control result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgeVerdict {
    /// Candidate is acceptable.
    Pass,
    /// Candidate should be regenerated once within the canonical round budget.
    Retry,
    /// Candidate must not be committed.
    Stop,
}

/// Configuration for AI-evaluates-AI. Disabled by default to keep costs honest.
#[derive(Debug, Clone, PartialEq)]
pub struct JudgeConfig {
    /// Whether the module makes side-calls.
    pub enabled: bool,
    /// Optional isolated model; otherwise the current turn model is used.
    pub model: Option<String>,
    /// Retry is honored only below this score.
    pub retry_below: f64,
    /// Maximum retry directives emitted for one session turn.
    pub max_retries: u32,
    /// Maximum candidate characters sent to the Judge.
    pub max_candidate_chars: usize,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: None,
            retry_below: 0.6,
            max_retries: 1,
            max_candidate_chars: DEFAULT_MAX_CONTEXT_CHARS,
        }
    }
}

/// AI-evaluates-AI module using the runtime-owned isolated invoker.
pub struct JudgeModule {
    manifest: ModuleManifest,
    config: JudgeConfig,
    observations: Arc<JudgeObservations>,
    retries: Mutex<BTreeMap<SessionId, u32>>,
    metrics: ModuleMetrics,
}

impl JudgeModule {
    /// Build a Judge slot and a shared observation channel for self-assessment.
    pub fn new(config: JudgeConfig, observations: Arc<JudgeObservations>) -> Self {
        Self {
            manifest: ModuleManifest::new(JUDGE_MODULE_ID, "AI evaluates AI"),
            config,
            observations,
            retries: Mutex::new(BTreeMap::new()),
            metrics: ModuleMetrics::default(),
        }
    }

    /// Shared observation channel used by a matching self-assessment module.
    pub fn observations(&self) -> Arc<JudgeObservations> {
        Arc::clone(&self.observations)
    }

    /// Read-only metrics for embedding callers.
    pub fn metrics(&self) -> ModuleMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        self.metrics.attach_telemetry(telemetry);
        self
    }

    fn parse_result(text: &str) -> Result<JudgeResult, ModuleError> {
        let trimmed = text.trim();
        let json = if let Some(stripped) = trimmed.strip_prefix("```") {
            let body = stripped
                .strip_prefix("json")
                .or_else(|| stripped.strip_prefix("JSON"))
                .unwrap_or(stripped)
                .trim_start_matches('\n');
            body.strip_suffix("```").unwrap_or(body).trim()
        } else {
            trimmed
        };
        let result: JudgeResult = serde_json::from_str(json).map_err(|error| {
            ModuleError::Message(format!("judge returned invalid JSON: {error}"))
        })?;
        if !result.score.is_finite() || !(0.0..=1.0).contains(&result.score) {
            return Err(ModuleError::Message(
                "judge score must be finite and between 0 and 1".into(),
            ));
        }
        if result.critique.chars().count() > 2_000 {
            return Err(ModuleError::Message(
                "judge critique exceeds 2000 characters".into(),
            ));
        }
        Ok(result)
    }
}

#[async_trait::async_trait]
impl AgentModule for JudgeModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        let started = Instant::now();
        if hook == HookPoint::TurnStart {
            self.retries
                .lock()
                .expect("judge retries mutex")
                .remove(ctx.session_id);
            self.observations.clear(ctx.session_id);
        }

        let mut side_calls = 0;
        let result = if !self.config.enabled || hook != HookPoint::AfterModelResponse {
            ModuleOutcome::continue_()
        } else if ctx.candidate.is_none() {
            return Err(ModuleError::MissingCandidate);
        } else if ctx
            .candidate
            .is_some_and(|candidate| !candidate.tool_calls.is_empty())
        {
            // Tool-call candidates are not final answers and must not be
            // evaluated or persisted by this slot.
            ModuleOutcome::continue_()
        } else {
            let candidate = ctx.candidate.expect("candidate checked above");
            let request = ModuleInvocationRequest::isolated(
                "You are a strict answer evaluator. Return only JSON matching this schema: {\"score\": number from 0 to 1, \"verdict\": \"pass\"|\"retry\"|\"stop\", \"critique\": string <= 2000 chars}. Evaluate the candidate for usefulness, correctness, and alignment with the user request. Never call tools.",
                format!(
                    "User request:\n{}\n\nCandidate answer:\n{}",
                    bounded(&topic_from_messages(ctx.messages), self.config.max_candidate_chars),
                    bounded(&candidate.content, self.config.max_candidate_chars)
                ),
            );
            let request = match &self.config.model {
                Some(model) => request.with_model(model.clone()),
                None => request,
            };
            let response = ctx.invoker().invoke(request).await?;
            side_calls = 1;
            match Self::parse_result(response.text()) {
                Ok(judged) => {
                    self.observations.record(*ctx.session_id, judged.clone());
                    match judged.verdict {
                        JudgeVerdict::Pass => ModuleOutcome::continue_(),
                        JudgeVerdict::Stop => {
                            ModuleOutcome::stop("AI Judge rejected the candidate")
                        }
                        JudgeVerdict::Retry if judged.score < self.config.retry_below => {
                            let mut retries = self.retries.lock().expect("judge retries mutex");
                            let retry_count = retries.entry(*ctx.session_id).or_default();
                            if *retry_count < self.config.max_retries {
                                *retry_count += 1;
                                ModuleOutcome::retry(bounded(&judged.critique, 2_000))
                            } else {
                                // Retry budget exhausted = best-effort acceptance,
                                // NOT a turn-killing stop (2026-10-06 真机: Stop
                                // 把审批续跑回合打成 HTTP 500). The Retry verdict
                                // is already recorded in observations (0 装).
                                ModuleOutcome::continue_()
                            }
                        }
                        JudgeVerdict::Retry => ModuleOutcome::continue_(),
                    }
                }
                Err(error) => {
                    // 侧调用空响应/坏 JSON = 评审不可用。Best-effort 放行候选,
                    // 绝不因元层解析失败杀死主任务 (2026-10-06 真机: DeepSeek
                    // 对纯 JSON 指令偶发空回复 → "EOF at line 1 column 0" →
                    // HTTP 500 杀死了审批续跑回合)。0 装: stderr 留痕 + metrics
                    // 记录 Continue, 不伪造任何评分。
                    eprintln!(
                        "[cognitive.judge] side-call unparseable; best-effort continue (candidate NOT evaluated): {error}"
                    );
                    ModuleOutcome::continue_()
                }
            }
        };
        self.metrics.record(
            JUDGE_MODULE_ID,
            hook,
            &result.directive,
            started,
            side_calls,
        );
        Ok(result)
    }
}

/// Persist a Judge-backed self-assessment after the candidate has committed.
pub struct SelfAssessmentModule {
    manifest: ModuleManifest,
    store: Arc<dyn SelfAssessmentStore>,
    clock: Arc<dyn Clock>,
    observations: Arc<JudgeObservations>,
    metrics: ModuleMetrics,
}

impl SelfAssessmentModule {
    /// Build an AfterTurn-only self-assessment slot.
    pub fn new(
        store: Arc<dyn SelfAssessmentStore>,
        clock: Arc<dyn Clock>,
        observations: Arc<JudgeObservations>,
    ) -> Self {
        Self {
            manifest: ModuleManifest::new(SELF_ASSESSMENT_MODULE_ID, "Self assessment"),
            store,
            clock,
            observations,
            metrics: ModuleMetrics::default(),
        }
    }

    /// Read-only metrics for embedding callers.
    pub fn metrics(&self) -> ModuleMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        self.metrics.attach_telemetry(telemetry);
        self
    }
}

#[async_trait::async_trait]
impl AgentModule for SelfAssessmentModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        let started = Instant::now();
        let result = if hook == HookPoint::AfterTurn {
            if let Some(judged) = self.observations.get(ctx.session_id) {
                let now = self.clock.now().timestamp();
                let session = session_text(ctx.session_id);
                let assessment = SelfAssessment {
                    id: hash_id(
                        "assessment",
                        &[
                            &session,
                            &now.to_string(),
                            ctx.candidate
                                .map(|candidate| candidate.id.as_str())
                                .unwrap_or("unknown"),
                        ],
                    ),
                    round: ctx
                        .messages
                        .iter()
                        .filter(|message| message.role == MessageRole::Assistant)
                        .count() as u32,
                    session_id: *ctx.session_id,
                    task_id: session,
                    alignment: judged.score,
                    quality: judged.score,
                    deviations: serde_json::json!({
                        "verdict": judged.verdict,
                        "score": judged.score,
                    }),
                    assessed_at: now,
                    reviewer_id: "cognitive.judge".into(),
                };
                if self.store.record(&assessment).is_err() {
                    self.metrics.warning();
                }
            }
            ModuleOutcome::continue_()
        } else {
            ModuleOutcome::continue_()
        };
        self.metrics.record(
            SELF_ASSESSMENT_MODULE_ID,
            hook,
            &result.directive,
            started,
            0,
        );
        Ok(result)
    }
}

/// Adapt the existing Council service to an AfterModelResponse decision.
///
/// The Council service owns bounded aggregation; this module adapts each
/// advisor to the runtime-owned [`ModuleInvoker`]. It never dispatches tools,
/// persists a session, or creates a second agent loop.
pub struct CouncilModule {
    manifest: ModuleManifest,
    council: Arc<Council>,
    clock: Arc<dyn Clock>,
    metrics: ModuleMetrics,
}

impl CouncilModule {
    /// Build a no-tool council adapter.
    pub fn new(council: Arc<Council>, clock: Arc<dyn Clock>) -> Self {
        Self {
            manifest: ModuleManifest::new(COUNCIL_MODULE_ID, "Council adapter"),
            council,
            clock,
            metrics: ModuleMetrics::default(),
        }
    }

    /// Read-only metrics for embedding callers.
    pub fn metrics(&self) -> ModuleMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        self.metrics.attach_telemetry(telemetry);
        self
    }
}

#[async_trait::async_trait]
impl AgentModule for CouncilModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        let started = Instant::now();
        let result = if hook == HookPoint::AfterModelResponse {
            if let Some(candidate) = ctx
                .candidate
                .filter(|candidate| candidate.tool_calls.is_empty())
            {
                let proposal = Proposal {
                    id: hash_id("proposal", &[&session_text(ctx.session_id), &candidate.id]),
                    proposer: "canonical-runtime".into(),
                    payload: serde_json::json!({ "candidate": bounded(&candidate.content, DEFAULT_MAX_CONTEXT_CHARS) }),
                    submitted_at: self.clock.now().timestamp(),
                    session_id: *ctx.session_id,
                };
                let adapter = RuntimeCouncilInvoker {
                    invoker: ctx.invoker(),
                };
                let result = self.council.decide_with_invoker(&proposal, &adapter).await;
                let side_calls = result.side_call_count;
                let outcome = match result.decision {
                    CouncilDecision::Continue => ModuleOutcome::continue_(),
                    CouncilDecision::Retry => ModuleOutcome::retry(result.retry_feedback()),
                    CouncilDecision::Stop => ModuleOutcome::stop(format!(
                        "Council hard-stop: {}",
                        result.stop_feedback()
                    )),
                    CouncilDecision::DeferToHuman => ModuleOutcome::stop(format!(
                        "Council could not reach a safe decision: {}",
                        result.stop_feedback()
                    )),
                };
                self.metrics.record(
                    COUNCIL_MODULE_ID,
                    hook,
                    &outcome.directive,
                    started,
                    u64::try_from(side_calls).expect("Council side-call count fits in u64"),
                );
                return Ok(outcome);
            } else {
                ModuleOutcome::continue_()
            }
        } else {
            ModuleOutcome::continue_()
        };
        self.metrics
            .record(COUNCIL_MODULE_ID, hook, &result.directive, started, 0);
        Ok(result)
    }
}

/// Runtime-owned adapter for the foundation Council service.
struct RuntimeCouncilInvoker<'a> {
    invoker: &'a dyn super::module::ModuleInvoker,
}

#[async_trait::async_trait]
impl CouncilInvoker for RuntimeCouncilInvoker<'_> {
    async fn invoke(
        &self,
        advisor: Arc<dyn Advisor>,
        proposal: &Proposal,
    ) -> Result<AdvisorVerdict, CouncilCallError> {
        let request = ModuleInvocationRequest::isolated(
            format!(
                "You are the {:?} council advisor. Return only JSON matching this schema: {{\"score\": number from 0 to 1, \"verdict\": \"allow\"|\"retry\"|\"stop\"|\"abstain\", \"critique\": string <= 2000 chars, \"confidence\": number or null}}. Never call tools.",
                advisor.kind()
            ),
            format!(
                "Advisor domain: {:?}\nProposal id: {}\nProposal payload: {}",
                advisor.kind(),
                proposal.id,
                proposal.payload
            ),
        );
        let response = self
            .invoker
            .invoke(request)
            .await
            .map_err(|error| CouncilCallError::Provider(error.to_string()))?;
        parse_advisor_verdict(response.text())
    }
}

fn parse_advisor_verdict(text: &str) -> Result<AdvisorVerdict, CouncilCallError> {
    let trimmed = text.trim();
    let json = if let Some(stripped) = trimmed.strip_prefix("```") {
        let body = stripped
            .strip_prefix("json")
            .or_else(|| stripped.strip_prefix("JSON"))
            .unwrap_or(stripped)
            .trim_start_matches('\n');
        body.strip_suffix("```").unwrap_or(body).trim()
    } else {
        trimmed
    };
    let verdict: AdvisorVerdict = serde_json::from_str(json)
        .map_err(|error| CouncilCallError::Malformed(format!("invalid advisor JSON: {error}")))?;
    verdict.validate().map_err(CouncilCallError::Malformed)?;
    Ok(verdict)
}

/// Convert a perception text event into the canonical request boundary.
///
/// Perception remains an input adapter, not an AgentModule.  Only the text
/// payload is accepted in this release; voice, vision, and tactile channels
/// remain explicit `NotImplemented` paths in the perception crate.
pub fn turn_request_from_perception(
    event: &apeireth_plugin::perception::PerceptionEvent,
) -> Result<super::execute::TurnRequest, ModuleError> {
    let text = event
        .payload
        .get("text")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModuleError::Message("perception event payload must contain text".into()))?;
    Ok(super::execute::TurnRequest::new(event.session_id, text))
}

/// Marker showing that this release has no separate reflection or planner
/// module.  Reflection is represented by the optional Judge-backed assessment;
/// Orchestrator remains an external long-running service.
pub const DEFERRED_COGNITIVE_SLOTS: &[(&str, &str)] = &[
    (
        "cognitive.critic",
        "included in Judge critique; no duplicate side-call",
    ),
    (
        "cognitive.reflection",
        "included in AfterTurn self-assessment",
    ),
    (
        "cognitive.planner",
        "adapter deferred; no second runtime loop",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{HistoryEntry, VirtualClock};
    use apeireth_plugin::experience::{AssociationEdge, GraphFact, GraphLink, WikiEntry};
    use apeireth_plugin::memory_backend::{BackendKind, CapabilityResult};
    use apeireth_plugin::perception::{PerceptionEvent, PerceptionModality};
    use std::sync::OnceLock;

    use super::super::production::{CognitiveBackends, CognitiveModuleConfig};
    use super::super::runtime::Runtime;

    #[derive(Default)]
    struct FakeMemory {
        episodes: Mutex<Vec<Episode>>,
    }

    impl MemoryBackend for FakeMemory {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn kind(&self) -> BackendKind {
            BackendKind::InMemory
        }

        fn put_episode(&self, episode: &Episode) -> CapabilityResult<()> {
            self.episodes
                .lock()
                .expect("fake memory mutex")
                .push(episode.clone());
            Ok(())
        }

        fn get_episode(&self, id: &str) -> CapabilityResult<Option<Episode>> {
            Ok(self
                .episodes
                .lock()
                .expect("fake memory mutex")
                .iter()
                .find(|episode| episode.id == id)
                .cloned())
        }

        fn recent_episodes(&self, session_id: &str, n: usize) -> CapabilityResult<Vec<Episode>> {
            let episodes = self
                .episodes
                .lock()
                .expect("fake memory mutex")
                .iter()
                .filter(|episode| episode.session_id == session_id)
                .cloned()
                .collect::<Vec<_>>();
            Ok(episodes.into_iter().rev().take(n).collect())
        }

        fn append_stream(
            &self,
            _kind: apeireth_core::kernel::StreamKind,
            _entry: HistoryEntry,
        ) -> CapabilityResult<()> {
            Ok(())
        }

        fn list_stream(
            &self,
            _kind: apeireth_core::kernel::StreamKind,
            _session_id: &str,
            _n: usize,
        ) -> CapabilityResult<Vec<HistoryEntry>> {
            Ok(Vec::new())
        }
    }

    impl apeireth_memory::MemoryGovernanceStore for FakeMemory {
        fn get_governed(
            &self,
            _episode_id: &str,
        ) -> Result<Option<apeireth_memory::GovernedEpisode>, apeireth_memory::MemoryGovernanceError>
        {
            Ok(None)
        }

        fn update_episode_content(
            &self,
            episode_id: &str,
            _new_content: &str,
            _updated_by: Option<&str>,
            _expected_rev: i64,
        ) -> Result<apeireth_memory::GovernedEpisode, apeireth_memory::MemoryGovernanceError>
        {
            Err(apeireth_memory::MemoryGovernanceError::NotFound(
                episode_id.to_string(),
            ))
        }

        fn forget_episode(
            &self,
            episode_id: &str,
            _reason: Option<&str>,
            _expected_rev: i64,
        ) -> Result<apeireth_memory::GovernedEpisode, apeireth_memory::MemoryGovernanceError>
        {
            Err(apeireth_memory::MemoryGovernanceError::NotFound(
                episode_id.to_string(),
            ))
        }

        fn protect_episode(
            &self,
            episode_id: &str,
            _expected_rev: i64,
        ) -> Result<apeireth_memory::GovernedEpisode, apeireth_memory::MemoryGovernanceError>
        {
            Err(apeireth_memory::MemoryGovernanceError::NotFound(
                episode_id.to_string(),
            ))
        }

        fn unprotect_episode(
            &self,
            episode_id: &str,
            _expected_rev: i64,
        ) -> Result<apeireth_memory::GovernedEpisode, apeireth_memory::MemoryGovernanceError>
        {
            Err(apeireth_memory::MemoryGovernanceError::NotFound(
                episode_id.to_string(),
            ))
        }

        fn governed_recent_episodes(
            &self,
            session_id: &str,
            n: usize,
        ) -> Result<Vec<apeireth_memory::GovernedEpisode>, apeireth_memory::MemoryGovernanceError>
        {
            // 2026-10-06: 治理视图从 episodes 如实派生 (此前恒返空 = 半真 fake,
            // 被 consolidation 效果测试咬出). fake 无遗忘态, 全部按 Active 上报.
            let mut governed: Vec<apeireth_memory::GovernedEpisode> = self
                .episodes
                .lock()
                .unwrap()
                .iter()
                .filter(|episode| episode.session_id == session_id)
                .rev()
                .take(n)
                .map(|episode| apeireth_memory::GovernedEpisode {
                    episode: episode.clone(),
                    status: apeireth_memory::MemoryGovernanceStatus::Active,
                    protected: false,
                    content_override: None,
                    revision: 0,
                    updated_at: None,
                    updated_by: None,
                    forgotten_at: None,
                })
                .collect();
            governed.reverse();
            Ok(governed)
        }

        fn governed_query(
            &self,
            _q: &apeireth_memory::EpisodeQuery,
        ) -> Result<Vec<apeireth_memory::GovernedEpisode>, apeireth_memory::MemoryGovernanceError>
        {
            Ok(Vec::new())
        }
    }

    #[derive(Default)]
    struct FakeExperience {
        wikis: Mutex<Vec<WikiEntry>>,
        facts: Mutex<Vec<GraphFact>>,
        links: Mutex<Vec<GraphLink>>,
        associations: Mutex<Vec<(String, String, String)>>,
    }

    impl WikiEntryStore for FakeExperience {
        fn put_wiki(&self, entry: &WikiEntry) -> CapabilityResult<()> {
            self.wikis
                .lock()
                .expect("fake wiki mutex")
                .push(entry.clone());
            Ok(())
        }

        fn list_wiki(
            &self,
            _session_id: &str,
            _topic: &str,
            _limit: u32,
        ) -> CapabilityResult<Vec<WikiEntry>> {
            Ok(self.wikis.lock().expect("fake wiki mutex").clone())
        }

        fn wiki_for_episode(&self, episode_id: &str) -> CapabilityResult<Vec<WikiEntry>> {
            Ok(self
                .wikis
                .lock()
                .expect("fake wiki mutex")
                .iter()
                .filter(|entry| entry.source_episode_id == episode_id)
                .cloned()
                .collect())
        }
    }

    impl KnowledgeGraphStore for FakeExperience {
        fn put_fact(&self, fact: &GraphFact) -> CapabilityResult<()> {
            self.facts
                .lock()
                .expect("fake facts mutex")
                .push(fact.clone());
            Ok(())
        }

        fn put_link(&self, link: &GraphLink) -> CapabilityResult<()> {
            self.links
                .lock()
                .expect("fake links mutex")
                .push(link.clone());
            Ok(())
        }

        fn facts_from(&self, _subject_id: &str, _limit: u32) -> CapabilityResult<Vec<GraphFact>> {
            Ok(self.facts.lock().expect("fake facts mutex").clone())
        }

        fn links_from(&self, _from_id: &str, _limit: u32) -> CapabilityResult<Vec<GraphLink>> {
            Ok(self.links.lock().expect("fake links mutex").clone())
        }

        fn forget_subject(&self, _subject_id: &str) -> CapabilityResult<()> {
            Ok(())
        }
    }

    impl AssociationStore for FakeExperience {
        fn record_cooccurrence(
            &self,
            from: &str,
            to: &str,
            episode_id: &str,
        ) -> CapabilityResult<()> {
            self.associations
                .lock()
                .expect("fake association mutex")
                .push((from.into(), to.into(), episode_id.into()));
            Ok(())
        }

        fn top_associations(
            &self,
            _entity: &str,
            _limit: u32,
        ) -> CapabilityResult<Vec<AssociationEdge>> {
            Ok(Vec::new())
        }
    }

    #[derive(Default)]
    struct FakePreferences;

    impl PreferenceStore for FakePreferences {
        fn record(&self, _pref: &UserPreference) -> CapabilityResult<()> {
            Ok(())
        }

        fn recall_for_context(
            &self,
            session_id: &SessionId,
            _topic: &str,
            _limit: u32,
        ) -> CapabilityResult<Vec<UserPreference>> {
            Ok(vec![UserPreference {
                id: "pref-1".into(),
                session_id: *session_id,
                topic: "language".into(),
                stance: "respond in Chinese".into(),
                evidence_refs: vec!["ep-1".into()],
                created_at: 1,
                confidence: 0.8,
                tags: vec!["language".into()],
            }])
        }

        fn forget(&self, _pref_id: &str) -> CapabilityResult<()> {
            Ok(())
        }

        fn list_for_session(
            &self,
            _session_id: &SessionId,
        ) -> CapabilityResult<Vec<UserPreference>> {
            Ok(Vec::new())
        }
    }

    /// 2026-09-08 查询扩展测试: 记录每次 recall 查询的 topic, 返回固定 pref.
    #[derive(Default)]
    struct QueryRecordingPreferences {
        queries: Mutex<Vec<String>>,
        pref: Mutex<Option<UserPreference>>,
    }

    impl PreferenceStore for QueryRecordingPreferences {
        fn record(&self, pref: &UserPreference) -> CapabilityResult<()> {
            *self.pref.lock().expect("fake pref mutex") = Some(pref.clone());
            Ok(())
        }

        fn recall_for_context(
            &self,
            session_id: &SessionId,
            topic: &str,
            _limit: u32,
        ) -> CapabilityResult<Vec<UserPreference>> {
            self.queries
                .lock()
                .expect("fake query mutex")
                .push(topic.to_string());
            Ok(self
                .pref
                .lock()
                .expect("fake pref mutex")
                .clone()
                .map(|mut p| {
                    p.session_id = *session_id;
                    vec![p]
                })
                .unwrap_or_default())
        }

        fn forget(&self, _pref_id: &str) -> CapabilityResult<()> {
            Ok(())
        }

        fn list_for_session(
            &self,
            _session_id: &SessionId,
        ) -> CapabilityResult<Vec<UserPreference>> {
            Ok(Vec::new())
        }
    }

    /// 2026-09-08: 召回侧查询扩展 — 簇键先行 + 原文回退, 按 id 去重合并.
    #[tokio::test]
    async fn preference_recall_expands_query_cluster_then_raw() {
        let session = SessionId::new();
        let store = Arc::new(QueryRecordingPreferences {
            queries: Mutex::new(Vec::new()),
            pref: Mutex::new(Some(UserPreference {
                id: "p1".into(),
                session_id: session,
                topic: "exam_prep".into(),
                stance: "主人喜欢备考".into(),
                evidence_refs: vec![],
                created_at: 1,
                confidence: 0.8,
                tags: vec![],
            })),
        });
        let module = PreferenceRecallModule::new(store.clone());
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = Arc::new(FixedInvoker {
            response: NormalizedResponse::text("judge", "judge", "{}"),
            calls: AtomicU64::new(0),
        });
        let messages = vec![NormalizedMessage::user("我在复习高数")];
        let outcome = module
            .on_hook(
                HookPoint::TurnStart,
                &context(
                    &session,
                    &messages,
                    None,
                    &invoker,
                    PREFERENCE_RECALL_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        let queries = store.queries.lock().expect("query mutex");
        assert_eq!(queries.len(), 3, "原文 + 簇键 + top-N 回退: {queries:?}");
        assert_eq!(queries[0], "我在复习高数", "原文先行 (等价性门)");
        assert_eq!(queries[1], "exam_prep", "簇键补充");
        assert_eq!(queries[2], "", "未满 limit 时 top-N 回退");
        // 两查询都返回同一 pref → 去重后 overlay 注入一次.
        assert_eq!(outcome.prompt_overlays.len(), 1);
        let text = ContentPart::join_text(&outcome.prompt_overlays[0].message().content);
        assert!(text.contains("主人喜欢备考"), "overlay 内容: {text}");
    }

    #[derive(Default)]
    struct FakeAssessments {
        values: Mutex<Vec<SelfAssessment>>,
    }
    impl SelfAssessmentStore for FakeAssessments {
        fn record(&self, assessment: &SelfAssessment) -> CapabilityResult<()> {
            self.values
                .lock()
                .expect("fake assessments mutex")
                .push(assessment.clone());
            Ok(())
        }

        fn recent_for_task(
            &self,
            _task_id: &str,
            _limit: u32,
        ) -> CapabilityResult<Vec<SelfAssessment>> {
            Ok(Vec::new())
        }

        fn latest_alignment(&self, _task_id: &str) -> CapabilityResult<Option<f64>> {
            Ok(None)
        }
    }

    struct FixedInvoker {
        response: NormalizedResponse,
        calls: AtomicU64,
    }

    #[async_trait::async_trait]
    impl super::super::module::ModuleInvoker for FixedInvoker {
        async fn invoke(
            &self,
            _request: ModuleInvocationRequest,
        ) -> Result<
            super::super::module::ModuleInvocationResponse,
            super::super::module::ModuleInvocationError,
        > {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(super::super::module::ModuleInvocationResponse {
                response: self.response.clone(),
                served_by: apeireth_core::kernel::CapabilityId::new("provider.fake").unwrap(),
            })
        }
    }

    struct DummySubLoop;

    #[async_trait::async_trait]
    impl super::super::subloop::SubLoopSpawner for DummySubLoop {
        async fn spawn(
            &self,
            _spec: super::super::subloop::SubLoopSpec,
        ) -> Result<super::super::subloop::SubLoopResult, super::super::subloop::SubLoopError>
        {
            Err(super::super::subloop::SubLoopError::NoModel)
        }
    }

    fn context<'a>(
        session: &'a SessionId,
        messages: &'a [NormalizedMessage],
        candidate: Option<&'a NormalizedResponse>,
        invoker: &'a Arc<dyn super::super::module::ModuleInvoker>,
        module_id: &'a str,
    ) -> ModuleContext<'a> {
        static INVOCATION: OnceLock<super::super::module::InvocationContext> = OnceLock::new();
        static DUMMY_SUBLOOP: DummySubLoop = DummySubLoop;
        ModuleContext {
            session_id: session,
            model: "fake-model",
            messages,
            candidate,
            tool_call: None,
            tool_result: None,
            invocation: INVOCATION.get_or_init(super::super::module::InvocationContext::user_turn),
            module_id,
            error: None,
            invoker: &**invoker,
            invoker_handle: Arc::clone(invoker),
            subloop: &DUMMY_SUBLOOP,
        }
    }

    struct FakeScoped {
        episodes: std::sync::Mutex<Vec<Episode>>,
    }

    impl apeireth_memory::ScopedMemoryBackend for FakeScoped {
        fn query_candidates(
            &self,
            _query: &apeireth_memory::MemoryCandidateQuery,
        ) -> Result<Vec<Episode>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.episodes.lock().unwrap().clone())
        }
    }

    #[tokio::test]
    async fn memory_injection_format_switches_the_overlay_shape() {
        let session = SessionId::new();
        let memory = Arc::new(FakeMemory::default());
        let scoped = Arc::new(FakeScoped {
            episodes: std::sync::Mutex::new(vec![Episode {
                id: "e1".into(),
                timestamp: 1,
                role: "user".into(),
                content: "主人明天要交线代作业".into(),
                session_id: session.to_string(),
            }]),
        });
        let xml = MemoryCoordinator::new(memory.clone(), memory.clone())
            .with_scoped_backend(scoped.clone());
        let injection = MemoryCoordinator::new(memory.clone(), memory.clone())
            .with_scoped_backend(scoped)
            .with_memory_injection_format();
        let query = apeireth_memory::MemoryRecallQuery::new(session.to_string(), "线代作业");

        let xml_overlay = xml
            .compile_prompt_overlay(&query)
            .unwrap()
            .expect("xml overlay");
        assert!(xml_overlay.contains("<governed_memory"), "{xml_overlay}");

        let inj_overlay = injection
            .compile_prompt_overlay(&query)
            .unwrap()
            .expect("injection overlay");
        assert!(inj_overlay.contains("[记忆证据"), "{inj_overlay}");
        assert!(
            inj_overlay.contains("禁止说「我记得我们以前聊过」"),
            "{inj_overlay}"
        );
    }

    #[tokio::test]
    async fn consolidation_is_opt_in_and_persists_insights_idempotently() {
        let session = SessionId::new();
        let memory = Arc::new(FakeMemory::default());
        memory
            .put_episode(&Episode {
                id: "e1".into(),
                timestamp: 1,
                role: "user".into(),
                content: "we resolved the build error: xyz".into(),
                session_id: session.to_string(),
            })
            .unwrap();
        let coordinator = Arc::new(MemoryCoordinator::new(memory.clone(), memory.clone()));
        let clock: Arc<dyn Clock> = Arc::new(VirtualClock::new(
            apeireth_core::kernel::Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ));
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = Arc::new(FixedInvoker {
            response: NormalizedResponse::text("x", "fake", "ok"),
            calls: AtomicU64::new(0),
        });
        let messages = vec![NormalizedMessage::user("hi")];
        let candidate = NormalizedResponse::text("answer-1", "fake", "resolved the issue");

        // 默认关: 不落任何 consolidation 洞察.
        let off = MemoryWritebackModule::new(memory.clone(), Arc::clone(&clock))
            .with_coordinator(Arc::clone(&coordinator));
        off.on_hook(
            HookPoint::AfterTurn,
            &context(
                &session,
                &messages,
                Some(&candidate),
                &invoker,
                MEMORY_WRITEBACK_MODULE_ID,
            ),
        )
        .await
        .unwrap();
        assert_eq!(insight_ids(&memory, &session.to_string()).len(), 0);

        // 开: 洞察落库; 重复运行幂等 (稳定 ID + 派生记忆不再入料).
        let on = MemoryWritebackModule::new(memory.clone(), Arc::clone(&clock))
            .with_coordinator(Arc::clone(&coordinator))
            .with_consolidation();
        on.on_hook(
            HookPoint::AfterTurn,
            &context(
                &session,
                &messages,
                Some(&candidate),
                &invoker,
                MEMORY_WRITEBACK_MODULE_ID,
            ),
        )
        .await
        .unwrap();
        // 原始证据 = 种子 user 条目 + 本轮 assistant 回复, 各提炼 1 条洞察.
        let first = insight_ids(&memory, &session.to_string());
        assert_eq!(first.len(), 2, "one insight per raw evidence: {first:?}");

        // 再跑一轮: ID 集合不变 (幂等; 洞察自我增殖必须为 0).
        on.on_hook(
            HookPoint::AfterTurn,
            &context(
                &session,
                &messages,
                Some(&candidate),
                &invoker,
                MEMORY_WRITEBACK_MODULE_ID,
            ),
        )
        .await
        .unwrap();
        assert_eq!(
            insight_ids(&memory, &session.to_string()),
            first,
            "second run must not spawn derived-insight cascades"
        );
    }

    fn insight_ids(memory: &FakeMemory, session: &str) -> std::collections::BTreeSet<String> {
        memory
            .recent_episodes(session, 50)
            .unwrap()
            .into_iter()
            .filter(|episode| episode.role == "consolidation")
            .map(|episode| episode.id)
            .collect()
    }

    #[tokio::test]
    async fn reflexion_records_judge_failures_and_injects_lessons() {
        use apeireth_memory::reflexion::ReflexionStore;
        let session = SessionId::new();
        let store = Arc::new(apeireth_memory::reflexion::InMemoryReflexionStore::new());
        let observations = Arc::new(JudgeObservations::default());
        let clock: Arc<dyn Clock> = Arc::new(VirtualClock::new(
            apeireth_core::kernel::Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ));
        let module = ReflexionModule::new(store.clone(), Arc::clone(&observations), clock);
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = Arc::new(FixedInvoker {
            response: NormalizedResponse::text("x", "fake", "ok"),
            calls: AtomicU64::new(0),
        });
        let messages = vec![NormalizedMessage::user("hi")];

        // 无判定 → 不记录, TurnStart 无注入 (行为不变).
        let quiet = module
            .on_hook(
                HookPoint::TurnStart,
                &context(&session, &messages, None, &invoker, REFLEXION_MODULE_ID),
            )
            .await
            .unwrap();
        assert!(quiet.prompt_overlays.is_empty());
        assert!(store.list_failures().unwrap().is_empty());

        // Judge 显式非 Pass 判定 → 沉淀失败 + RuleCritic 即时蒸馏反思.
        observations.record(
            session,
            JudgeResult {
                score: 0.3,
                verdict: JudgeVerdict::Retry,
                critique: "answer too terse".into(),
            },
        );
        module
            .on_hook(
                HookPoint::AfterTurn,
                &context(&session, &messages, None, &invoker, REFLEXION_MODULE_ID),
            )
            .await
            .unwrap();
        assert_eq!(store.list_failures().unwrap().len(), 1);
        assert_eq!(store.list_reflections().unwrap().len(), 1);

        // TurnStart 注入教训块 (donor 反刍格式).
        let injected = module
            .on_hook(
                HookPoint::TurnStart,
                &context(&session, &messages, None, &invoker, REFLEXION_MODULE_ID),
            )
            .await
            .unwrap();
        assert_eq!(injected.prompt_overlays.len(), 1);
        let text = ContentPart::join_text(&injected.prompt_overlays[0].message().content);
        assert!(text.contains("历史失败反思备忘"), "{text}");
    }

    #[tokio::test]
    async fn proactive_recall_is_opt_in_and_budgeted() {
        let memory = Arc::new(FakeMemory::default());
        let disabled = MemoryRecallModule::new(memory.clone());
        assert!(disabled.proactive_recall.is_none());
        let enabled = MemoryRecallModule::new(memory).with_proactive_recall(
            ProactiveRecallPolicy::default()
                .enabled(true)
                .with_budget(1),
        );
        assert_eq!(
            enabled.proactive_recall.as_ref().unwrap().policy().budget,
            1
        );
    }
    #[tokio::test]
    async fn recall_is_transient_and_writeback_is_after_turn_only() {
        let session = SessionId::new();
        let memory = Arc::new(FakeMemory::default());
        memory
            .put_episode(&Episode {
                id: "old".into(),
                timestamp: 1,
                role: "user".into(),
                content: "remember this".into(),
                session_id: session.to_string(),
            })
            .unwrap();
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = Arc::new(FixedInvoker {
            response: NormalizedResponse::text("judge", "judge", "{}"),
            calls: AtomicU64::new(0),
        });
        let messages = vec![NormalizedMessage::user("what now?")];
        let telemetry = Arc::new(CognitiveTelemetry::default());
        let recall = MemoryRecallModule::new(memory.clone()).with_telemetry(Arc::clone(&telemetry));
        let outcome = recall
            .on_hook(
                HookPoint::TurnStart,
                &context(&session, &messages, None, &invoker, MEMORY_RECALL_MODULE_ID),
            )
            .await
            .unwrap();
        assert_eq!(outcome.prompt_overlays.len(), 1);
        assert_eq!(telemetry.events().len(), 1);
        assert_eq!(telemetry.events()[0].module_id, MEMORY_RECALL_MODULE_ID);
        assert_eq!(telemetry.events()[0].hook, "TurnStart");

        let clock = Arc::new(VirtualClock::new(
            apeireth_core::kernel::Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ));
        let writeback = MemoryWritebackModule::new(memory.clone(), clock);
        let candidate = NormalizedResponse::text("answer-1", "fake", "final");
        writeback
            .on_hook(
                HookPoint::BeforeFinalCommit,
                &context(
                    &session,
                    &messages,
                    Some(&candidate),
                    &invoker,
                    MEMORY_WRITEBACK_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        assert_eq!(memory.episodes.lock().unwrap().len(), 1);
        writeback
            .on_hook(
                HookPoint::AfterTurn,
                &context(
                    &session,
                    &messages,
                    Some(&candidate),
                    &invoker,
                    MEMORY_WRITEBACK_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        assert_eq!(memory.episodes.lock().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn writeback_extracts_only_after_durable_episode_persistence() {
        let session = SessionId::new();
        let memory = Arc::new(FakeMemory::default());
        let experience = Arc::new(FakeExperience::default());
        let clock = Arc::new(VirtualClock::new(
            apeireth_core::kernel::Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ));
        let writeback = MemoryWritebackModule::new(memory.clone(), clock).with_experience(
            experience.clone(),
            experience.clone(),
            experience.clone(),
        );
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = Arc::new(FixedInvoker {
            response: NormalizedResponse::text("unused", "fake", "unused"),
            calls: AtomicU64::new(0),
        });
        let messages = vec![NormalizedMessage::user("remember this")];
        let candidate = NormalizedResponse::text(
            "answer-1",
            "fake",
            "A concise answer.\nfact: rust | property | fast\nlink: rust | fast | supports\nassociate: rust | cargo",
        );
        writeback
            .on_hook(
                HookPoint::AfterTurn,
                &context(
                    &session,
                    &messages,
                    Some(&candidate),
                    &invoker,
                    MEMORY_WRITEBACK_MODULE_ID,
                ),
            )
            .await
            .unwrap();

        assert_eq!(memory.episodes.lock().unwrap().len(), 2);
        assert_eq!(experience.wikis.lock().unwrap().len(), 2);
        assert_eq!(experience.facts.lock().unwrap().len(), 1);
        assert_eq!(experience.links.lock().unwrap().len(), 1);
        assert_eq!(experience.associations.lock().unwrap().len(), 1);
        assert!(experience
            .facts
            .lock()
            .unwrap()
            .iter()
            .all(|fact| !fact.source_episode_id.is_empty()));
        assert_eq!(writeback.metrics().warnings, 0);
    }

    #[tokio::test]
    async fn judge_uses_one_bounded_side_call_and_retries_once() {
        let session = SessionId::new();
        let invoker_counter = Arc::new(FixedInvoker {
            response: NormalizedResponse::text(
                "judge-1",
                "judge",
                r#"{"score":0.2,"verdict":"retry","critique":"be more direct"}"#,
            ),
            calls: AtomicU64::new(0),
        });
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = invoker_counter.clone();
        let judge = JudgeModule::new(
            JudgeConfig {
                enabled: true,
                max_retries: 1,
                ..JudgeConfig::default()
            },
            Arc::new(JudgeObservations::default()),
        );
        let messages = vec![NormalizedMessage::user("request")];
        let candidate = NormalizedResponse::text("answer-1", "fake", "candidate");
        let first = judge
            .on_hook(
                HookPoint::AfterModelResponse,
                &context(
                    &session,
                    &messages,
                    Some(&candidate),
                    &invoker,
                    JUDGE_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        assert!(matches!(first.directive, ModuleDirective::Retry { .. }));
        let second = judge
            .on_hook(
                HookPoint::AfterModelResponse,
                &context(
                    &session,
                    &messages,
                    Some(&candidate),
                    &invoker,
                    JUDGE_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        // Budget exhausted degrades to best-effort acceptance (2026-10-06:
        // a Stop here killed approval-resumed turns with HTTP 500).
        assert!(matches!(second.directive, ModuleDirective::Continue));
        assert_eq!(invoker_counter.calls.load(Ordering::Relaxed), 2);
        assert_eq!(judge.metrics().side_calls, 2);
        assert!(JudgeModule::parse_result("not json").is_err());
    }

    #[tokio::test]
    async fn judge_side_call_empty_response_degrades_to_continue() {
        let session = SessionId::new();
        // DeepSeek 对纯 JSON 指令偶发空回复 (2026-10-06 真机: EOF at line 1
        // column 0) — 评审不可用必须降级放行候选, 而不是 Err 杀死回合.
        let invoker_counter = Arc::new(FixedInvoker {
            response: NormalizedResponse::text("", "fake", "empty"),
            calls: AtomicU64::new(0),
        });
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = invoker_counter.clone();
        let judge = JudgeModule::new(
            JudgeConfig {
                enabled: true,
                ..JudgeConfig::default()
            },
            Arc::new(JudgeObservations::default()),
        );
        let messages = vec![NormalizedMessage::user("request")];
        let candidate = NormalizedResponse::text("answer-1", "fake", "candidate");
        let outcome = judge
            .on_hook(
                HookPoint::AfterModelResponse,
                &context(
                    &session,
                    &messages,
                    Some(&candidate),
                    &invoker,
                    JUDGE_MODULE_ID,
                ),
            )
            .await
            .expect("empty side-call must not surface as a module error");
        assert!(matches!(outcome.directive, ModuleDirective::Continue));
        assert_eq!(invoker_counter.calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn council_module_uses_module_invoker_for_bounded_fake_advisors() {
        let session = SessionId::new();
        let invoker_counter = Arc::new(FixedInvoker {
            response: NormalizedResponse::text(
                "council-1",
                "fake",
                r#"{"score":0.2,"verdict":"retry","critique":"tighten the answer","confidence":0.9}"#,
            ),
            calls: AtomicU64::new(0),
        });
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = invoker_counter.clone();
        let council = Arc::new(Council::default_llm().with_config(
            apeireth_orchestration::CouncilConfig {
                max_advisors: 3,
                per_advisor_timeout: std::time::Duration::from_secs(1),
                overall_timeout: std::time::Duration::from_secs(2),
            },
        ));
        let module = CouncilModule::new(
            council,
            Arc::new(VirtualClock::new(
                apeireth_core::kernel::Timestamp::from_epoch_millis(1_700_000_000_000)
                    .unwrap()
                    .as_datetime(),
            )),
        );
        let messages = vec![NormalizedMessage::user("request")];
        let candidate = NormalizedResponse::text("answer-1", "fake", "candidate");
        let outcome = module
            .on_hook(
                HookPoint::AfterModelResponse,
                &context(
                    &session,
                    &messages,
                    Some(&candidate),
                    &invoker,
                    COUNCIL_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        assert!(matches!(outcome.directive, ModuleDirective::Retry { .. }));
        assert_eq!(invoker_counter.calls.load(Ordering::Relaxed), 3);
        assert_eq!(module.metrics().side_calls, 3);
    }

    #[tokio::test]
    async fn production_slot_order_is_explicit() {
        let clock: Arc<dyn Clock> = Arc::new(VirtualClock::new(
            apeireth_core::kernel::Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ));
        let mut config = CognitiveModuleConfig::default();
        config.judge.enabled = false;
        let mem = Arc::new(FakeMemory::default());
        let backends = CognitiveBackends {
            memory: Some(mem.clone()),
            memory_governance: Some(mem),
            preferences: Some(Arc::new(FakePreferences)),
            self_assessments: Some(Arc::new(FakeAssessments::default())),
            ..CognitiveBackends::default()
        };
        let modules =
            super::super::production::ProductionCognitiveModules::build(config, backends, clock)
                .unwrap();
        assert_eq!(
            modules.ids(),
            vec![
                MEMORY_RECALL_MODULE_ID,
                PREFERENCE_RECALL_MODULE_ID,
                SELF_ASSESSMENT_MODULE_ID,
                MEMORY_WRITEBACK_MODULE_ID,
            ]
        );
        let telemetry = modules.telemetry();
        let _runtime = modules
            .register_into(Runtime::builder())
            .build()
            .await
            .unwrap();
        assert!(telemetry.events().is_empty());
    }

    #[test]
    fn perception_text_has_one_canonical_request_path() {
        let session = SessionId::new();
        let event = PerceptionEvent {
            id: "p-1".into(),
            source: PerceptionModality::Text,
            session_id: session,
            timestamp_ms: 1,
            payload: serde_json::json!({"text": "hello"}),
            attention_score: 1.0,
            tags: Vec::new(),
        };
        let request = turn_request_from_perception(&event).unwrap();
        assert_eq!(request.session, session);
        assert_eq!(request.input, "hello");
    }
}
