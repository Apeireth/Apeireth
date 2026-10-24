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
    build_constant_injection, consolidate_persona, BoundedMemoryInput, MemoryCoordinator,
    MemoryExtractor, MemoryGovernanceStatus, MemoryGovernanceStore, MemoryMaterializer,
    MemoryMaterializerPort, MemoryRecallQuery, MemoryScope, MemoryTypedMaterializationSink,
    PersonaMemoryStore, ProactiveRecallPolicy, ProactiveRecallService, RuleMemoryExtractor,
    SelectedMemoryAccess, SqliteAccessHistoryStore,
};
use apeireth_orchestration::{
    Advisor, AdvisorDecision, AdvisorVerdict, Council, CouncilCallError, CouncilDecision,
    CouncilInvoker, Proposal,
};
use apeireth_organ::context_assembly::{
    AssemblyPolicy, ContextAssembler, ContextAssemblerPolicyExt, ContextBlock,
};
use apeireth_organ::homeostasis::{
    HomeostasisRegulator, RegulationPlan, VitalSign, VitalSignReading, VitalSignSet,
};
use apeireth_organ::prompt_assembly::{
    repair_guidance, AssemblyGuard, AssemblyRole, PromptAssembler, REPAIR_DIRECTIVE_PLACEHOLDER,
};
use apeireth_organ::social_repair::RepairPlan;
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
    BehaviorModule, HookPoint, ModuleContext, ModuleDirective, ModuleError,
    ModuleInvocationRequest, ModuleManifest, ModuleOutcome, PromptOverlay,
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

/// poison 容错锁 (L 组, cognitive.rs 15+ 处 expect)。
///
/// 认知模块的 telemetry / 指标 / 访问记录锁都在模块热路径上: 持锁线程一旦
/// panic (例如一次 telemetry sink 的 panic), 旧实现的 `.expect(...)` 会把
/// 之后每一次 hook 调用都变成 panic —— 一个模块的失误升级为整个运行时的
/// 级联崩溃。锁保护的都是可重建的非权威状态, 取回内部数据继续运行。
fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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
        let mut events = lock_or_recover(&self.events);
        if events.len() == MAX_TELEMETRY_EVENTS {
            events.remove(0);
        }
        events.push(event);
    }

    /// Snapshot and clear no state; callers receive a stable copy.
    pub fn events(&self) -> Vec<CognitiveModuleEvent> {
        lock_or_recover(&self.events).clone()
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
        lock_or_recover(&self.selected_by_session).remove(session_id);
    }

    fn record_selected(&self, session_id: &str, access: &SelectedMemoryAccess) {
        lock_or_recover(&self.selected_by_session)
            .insert(session_id.to_owned(), access.selected_candidate_ids.clone());
    }

    /// Return the selected candidate IDs for the latest recall of a session.
    pub fn selected_candidate_ids(&self, session_id: &str) -> Vec<String> {
        lock_or_recover(&self.selected_by_session)
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
        *lock_or_recover(&self.telemetry) = Some(telemetry);
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
        *lock_or_recover(&self.last_hook) = Some(format!("{hook:?}"));
        *lock_or_recover(&self.last_directive) = Some(directive_name(directive).to_string());
        if let Some(telemetry) = lock_or_recover(&self.telemetry).as_ref() {
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
            last_hook: lock_or_recover(&self.last_hook).clone(),
            last_directive: lock_or_recover(&self.last_directive).clone(),
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
/// **存储**: 默认生产装配走 `SqlitePartnerStore`（2026-10-24 接上，
/// 此前是 `InMemoryPartnerStore` = 进程内、重启即散 —— 羁绊本义是**跨会话**的，
/// 那种存法等于每次重启都要重新「认识」一遍）。
/// `SqlitePartnerStore` 自带幂等建表（`partner_records`），与认知库同一份
/// `cognitive.sqlite3` ⇒ 羁绊与记忆同生共死。
/// 本模块只依赖 `PartnerStore` trait，具体后端由装配层决定。
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

/// 互动连续性注入文本 (纯函数, 供 TurnStart overlay 与测试)。
///
/// **ADR-0014 §1.1 改名**（2026-10-24）：注入给模型的措辞从「羁绊阶段」改为
/// 「互动连续性阶段」。理由（外部顾问原判）：
///
/// > 它把系统内部的交互统计，翻译成了**关系发展的事实**。用户看到的不是连续性本身，
/// > 而是产品对连续性的解释。
///
/// 「羁绊」是一个**关系断言**（暗示双方已形成某种羁绊），而系统实际拥有的只是
/// 「我们交互了多少次、演化到第几档」这一串**可验证的统计**。改成「互动连续性」
/// 之后，措辞与它真正掌握的事实对齐 —— 这是 ADR-0014 §6「最小诚实自述集」
/// （每句自述须对应可验证观察）在**注入面**的落实。
///
/// 注：**内部类型名不改**（`Bond` / `BondStage` / `bond_overlay_text`）——
/// 那是代码标识符，不是对用户或模型的表述；改它要动 schema 与迁移，
/// 收益为零。诚实纪律管的是**表述面**，不是标识符。
fn bond_overlay_text(partner: &apeireth_memory::partner::Partner) -> String {
    format!(
        "【互动连续性】阶段: {}, 深度: {:.2}, 演化次数: {} (工程化的交互统计, 供语气与信任校准参考; 不代表关系性质)",
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
impl BehaviorModule for PartnerBondModule {
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

/// 恒定小注入 (V4-a 人格记忆层回取面 ①) 的模块 id.
pub const PERSONA_INJECTION_MODULE_ID: &str = "cognitive.persona_injection";

/// **V4-a 恒定小注入模块**: `TurnStart` 把自我叙事摘要注入 prompt overlay
/// (`PromptOverlay::system`), 供每一轮提示词组装取用。
///
/// - **恒预算**: 摘要走 [`build_constant_injection`] 的 400 字符恒预算
///   (`CONSTANT_INJECTION_BUDGET_CHARS`), 超长截断 + 显式省略注记;
/// - **与查询无关**: 身份不靠检索命中, 它一直在;
/// - **缺人格层 = 零注入零变化**: 本模块只在人格层落库面在场时注册 (装配点见
///   [`super::production::ProductionModules::build`]); 人格层无自我叙事沉淀或
///   读取失败时同样不发 overlay —— 提示词逐字不变 (0 装诚实: 未沉淀就是未沉淀,
///   读失败走 fail-open 零注入, 不编造身份)。
pub struct PersonaConstantInjectionModule {
    manifest: ModuleManifest,
    persona: Arc<dyn PersonaMemoryStore>,
    metrics: ModuleMetrics,
}

impl PersonaConstantInjectionModule {
    /// 构造 (人格层落库面 = 恒定注入的回取源, 与人格提取步同一分表同账)。
    pub fn new(persona: Arc<dyn PersonaMemoryStore>) -> Self {
        Self {
            manifest: ModuleManifest::new(
                PERSONA_INJECTION_MODULE_ID,
                "Persona constant injection",
            ),
            persona,
            metrics: ModuleMetrics::default(),
        }
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        self.metrics.attach_telemetry(telemetry);
        self
    }
}

#[async_trait::async_trait]
impl BehaviorModule for PersonaConstantInjectionModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook != HookPoint::TurnStart {
            return Ok(ModuleOutcome::continue_());
        }
        let outcome = match self.persona.load() {
            // 空自述 → 空串 → 零注入零变化 (未沉淀就是未沉淀)。
            Ok(memory) => {
                let text = build_constant_injection(&memory);
                if text.is_empty() {
                    ModuleOutcome::continue_()
                } else {
                    ModuleOutcome::continue_().with_prompt_overlay(PromptOverlay::system(text))
                }
            }
            // 读失败 fail-open 零注入: 警告计数留痕, 不编造身份文本。
            Err(_) => {
                self.metrics.warning();
                ModuleOutcome::continue_()
            }
        };
        Ok(outcome)
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
        assert!(text.contains("互动连续性】阶段: 初识"), "{text}");
        assert!(text.contains("深度: 0.00"), "{text}");
        // ADR-0014 §1.1: 注入面不得再出现关系断言词「羁绊」
        assert!(!text.contains("羁绊"), "注入文本不得再断言关系: {text}");
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
impl BehaviorModule for AbsorptionInsightModule {
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
    /// 「性格养成」第一铲 (默认无 = 自学习关): 自校准接线层 —— 本模块的召回
    /// 结果是**真接**的检索命中/未命中信号源 (每轮记一次, 凑批由接线层聚合)。
    self_tuning: Option<Arc<crate::canonical::self_tuning_wire::SelfTuningWire>>,
    /// V4-c 检索维度自进化 (默认无 = 关): 注入回执喂证据账, 调权由接线层
    /// 裁决落账 —— 本模块的**注入回执**是它的信号面 (无回执 = 显式不进化)。
    retrieval_tuning: Option<Arc<crate::canonical::retrieval_tuning_wire::RetrievalTuningWire>>,
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
            self_tuning: None,
            retrieval_tuning: None,
        }
    }

    /// V4-c 检索维度自进化: 接入自进化接线层 (默认无 = 调权关)。
    /// 本模块每次召回把注入回执喂给接线层 (真接信号面)。
    #[must_use]
    pub fn with_retrieval_tuning(
        mut self,
        wire: Arc<crate::canonical::retrieval_tuning_wire::RetrievalTuningWire>,
    ) -> Self {
        self.retrieval_tuning = Some(wire);
        self
    }

    /// 「性格养成」第一铲: 接入自校准接线层 (默认无 = 自学习关)。
    /// 本模块每次召回把命中/未命中喂给接线层 (真接信号)。
    #[must_use]
    pub fn with_self_tuning(
        mut self,
        wire: Arc<crate::canonical::self_tuning_wire::SelfTuningWire>,
    ) -> Self {
        self.self_tuning = Some(wire);
        self
    }

    /// 回执/信号时间戳: 显式时钟优先, 缺时钟回系统时间 (缺系统时间回 0)。
    fn now_epoch_ms(&self) -> i64 {
        self.clock
            .as_ref()
            .map(|clock| clock.now().timestamp_millis())
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_millis() as i64)
                    .unwrap_or(0)
            })
    }

    /// 真接信号: 把一轮召回结果 (选中 ≥1 候选 = 命中, 空 = 未命中) 记进
    /// 自校准接线层; 无接线层 = 自学习关, 什么都不做。
    fn feed_retrieval_signal(&self, hit: bool) {
        if let Some(wire) = &self.self_tuning {
            wire.record_retrieval_round(hit, self.now_epoch_ms());
        }
    }

    /// V4-c 真接信号面: 把一轮召回的**注入回执**喂给自进化接线层 (回执 =
    /// 后续使用证据的唯一真值面; 无接线层 = 调权关, 什么都不做)。
    fn feed_retrieval_tuning(&self, selected: &apeireth_memory::SelectedMemoryAccess) {
        if let Some(wire) = &self.retrieval_tuning {
            wire.record_injection_round(&selected.selected_attribution, self.now_epoch_ms());
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
impl BehaviorModule for MemoryRecallModule {
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
                        // 真接信号: 本轮召回选中了候选 = 命中, 全空 = 未命中。
                        self.feed_retrieval_signal(!selected.selected_candidate_ids.is_empty());
                        // V4-c: 注入回执喂证据账 (选中集为空的轮按空轮报)。
                        self.feed_retrieval_tuning(&selected);
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
                        // Recalled memory content is cross-source input: the
                        // provider-request funnel discloses it through the
                        // untrusted reference envelope.
                        ModuleOutcome::continue_().with_prompt_overlay(
                            PromptOverlay::system_cross_source("memory_recall", overlay),
                        )
                    }
                    Ok(None) => {
                        // 真接信号: 无候选可召回 = 未命中轮。
                        self.feed_retrieval_signal(false);
                        // V4-c: 空轮如实报给接线层 (非机会轮, 不裁决证据)。
                        if let Some(wire) = &self.retrieval_tuning {
                            wire.record_empty_round(self.now_epoch_ms());
                        }
                        if let Some(recorder) = &self.access_recorder {
                            recorder.clear(&session);
                        }
                        ModuleOutcome::continue_()
                    }
                    Err(_) => {
                        // 检索失败是故障不是使用证据, 不喂信号 (0 装诚实)。
                        self.metrics.warning();
                        ModuleOutcome::continue_()
                    }
                }
            } else {
                let mut context = match self.memory.recent_episodes(&session, self.limit) {
                    Ok(episodes) => {
                        // 真接信号 (旧路径): 近期记忆非空 = 命中轮。
                        self.feed_retrieval_signal(!episodes.is_empty());
                        episode_context(&episodes, self.max_context_chars)
                    }
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
                    // Recalled memory content is cross-source input: the
                    // provider-request funnel discloses it through the
                    // untrusted reference envelope.
                    ModuleOutcome::continue_().with_prompt_overlay(
                        PromptOverlay::system_cross_source("memory_recall", overlay),
                    )
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
    /// V4-a 人格记忆层: 人格提取步的落库面 (未配置 = 显式不跑, 默认未接线).
    persona: Option<Arc<dyn PersonaMemoryStore>>,
    /// V4-b 保鲜回路: 散列增量门 (整编素材同源同散列 = 跳过重复整编, 省算力防
    /// 重复记忆; 未配置 = 门显式不跑, 默认未接线).
    ingest_gate: Option<Arc<dyn apeireth_memory::IngestGate>>,
    /// 「性格养成」第一铲: 整合节奏计数 (每会话回合数)。触发阈值 =
    /// 体验旋钮 ConsolidationCadence (每 N 回合一次, 1 = 每回合 = 现行为)。
    consolidation_turns: Mutex<std::collections::BTreeMap<String, u64>>,
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
            persona: None,
            ingest_gate: None,
            consolidation_turns: Mutex::new(std::collections::BTreeMap::new()),
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

    /// 接人格记忆层落库面 (V4-a): 整编触发点上加**人格提取步** (夜间整编与
    /// 整编浅接面同一条管线)。默认未接线 —— 不配置即显式不跑 (0 装诚实)。
    #[must_use]
    pub fn with_persona_layer(mut self, persona: Arc<dyn PersonaMemoryStore>) -> Self {
        self.persona = Some(persona);
        self
    }

    /// 接散列增量门 (V4-b 保鲜回路第①件): 整编触发点上先查门, 整编素材
    /// **同源同散列** = 跳过重复整编 (省算力防重复记忆); 变内容照常重摄。
    /// 默认未接线 —— 不配置即门显式不跑 (0 装诚实)。
    #[must_use]
    pub fn with_ingest_gate(mut self, gate: Arc<dyn apeireth_memory::IngestGate>) -> Self {
        self.ingest_gate = Some(gate);
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

    /// 整合节奏判定 (体验旋钮 ConsolidationCadence): 每会话每 N 回合返回 true
    /// 一次; N = 1 (未设/非法) = 每回合 = 现行为, 零变化。
    fn consolidation_due(&self, session: &str) -> bool {
        let cadence = crate::canonical::self_tuning_wire::consolidation_cadence_turns();
        let mut turns = lock_or_recover(&self.consolidation_turns);
        let counter = turns.entry(session.to_string()).or_insert(0);
        *counter = counter.saturating_add(1);
        *counter % cadence == 0
    }

    /// 散列增量门的整编素材口径: 治理可见近期情节的**原始证据角色**
    /// (`user`/`assistant`/`tool`) —— 派生记忆 (整理洞察/人格沉淀) 不入料,
    /// 与整编提炼素材同口径 (W2: 派生记忆不再入料, 防自我增殖)。
    ///
    /// 素材分段 (`role\ncontent`) **去重后按字典序规范拼接** —— 同一素材集合
    /// 恒得同一散列 (重复行 / 取回顺序都不误判重摄)。无原始证据 = `None`
    /// (无料可去重, 照常整编, 不查门)。
    fn consolidation_gate_material(&self, session: &str) -> Option<String> {
        let episodes = self.persona_source_episodes(session).ok()?;
        let mut segments: Vec<String> = episodes
            .iter()
            .filter(|episode| matches!(episode.role.as_str(), "user" | "assistant" | "tool"))
            .map(|episode| format!("{}\n{}", episode.role, episode.content))
            .collect();
        if segments.is_empty() {
            return None;
        }
        segments.sort();
        segments.dedup();
        Some(segments.join("\n\n"))
    }

    /// 人格提取步的素材口径: 治理可见的近期情节 (已遗忘情节不入料)。
    /// 协调器缺席时退化为后端近况 —— 显式降级, 不是假素材。
    fn persona_source_episodes(&self, session: &str) -> Result<Vec<Episode>, String> {
        if let Some(coord) = &self.coordinator {
            let governed = coord
                .governance()
                .governed_recent_episodes(session, 50)
                .map_err(|error| error.to_string())?;
            return Ok(governed
                .into_iter()
                .filter(|item| item.status != MemoryGovernanceStatus::Forgotten)
                .map(|item| item.episode)
                .collect());
        }
        self.memory
            .recent_episodes(session, 50)
            .map_err(|error| error.to_string())
    }
}

#[async_trait::async_trait]
impl BehaviorModule for MemoryWritebackModule {
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
                // 「性格养成」第一铲: 触发阈值 = 整合节奏旋钮
                // (APEIRETH_TUNE_CONSOLIDATION_CADENCE: 每 N 回合一次, 1 = 每回合
                // = 现行为, 零变化; 非法值回 1)。
                if self.consolidation && self.consolidation_due(&session) {
                    // V4-b 散列增量门 (保鲜三件套第①件): 整编素材内容散列查门 ——
                    // 同源同散列 = 跳过重复整编 (省算力防重复记忆), 变内容照常
                    // 重摄。记账语义: 跳过即记去重计数; 摄取记账在整编步**全部
                    // 成功后** (失败不污染账, 下次照常重摄); 门自身故障 fail-open
                    // 照常整编 (真因进 warning 计数, 不静默降级也不挡记忆固化)。
                    let gate_source = format!("consolidation:{session}");
                    let gate_material = self.consolidation_gate_material(&session);
                    let now_ms = now.saturating_mul(1_000);
                    let mut skip_round = false;
                    if let (Some(gate), Some(material)) = (&self.ingest_gate, &gate_material) {
                        match gate.check(&gate_source, material) {
                            Ok(apeireth_memory::GateDecision::SkipDuplicate { .. }) => {
                                skip_round = true;
                                if gate.mark_skipped(&gate_source, material, now_ms).is_err() {
                                    self.metrics.warning();
                                }
                            }
                            Ok(apeireth_memory::GateDecision::Ingest { .. }) => {}
                            Err(_) => self.metrics.warning(),
                        }
                    }
                    let mut ran_any = false;
                    let mut round_ok = true;
                    if !skip_round {
                        if let Some(coord) = &self.coordinator {
                            ran_any = true;
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
                                            round_ok = false;
                                        }
                                    }
                                }
                                Err(_) => {
                                    self.metrics.warning();
                                    round_ok = false;
                                }
                            }
                        }
                        // V4-a 人格提取步: 从近期情节提炼人格增量 → 合并策略 (同类合并 /
                        // 矛盾标注「人格张力」, 不静默覆盖) → 人格层落库。与夜间整编共用
                        // 同一条管线; persona 存储未配置 = 显式不跑 (默认未接线)。
                        if let Some(persona) = &self.persona {
                            ran_any = true;
                            match self.persona_source_episodes(&session) {
                                Ok(episodes) => {
                                    if consolidate_persona(persona.as_ref(), &episodes, now)
                                        .is_err()
                                    {
                                        self.metrics.warning();
                                        round_ok = false;
                                    }
                                }
                                Err(_) => {
                                    self.metrics.warning();
                                    round_ok = false;
                                }
                            }
                        }
                        // 整编成功才记摄入账 (失败不污染账 = 同素材下次仍可重摄)。
                        if ran_any && round_ok {
                            if let (Some(gate), Some(material)) =
                                (&self.ingest_gate, &gate_material)
                            {
                                if gate.mark_ingested(&gate_source, material, now_ms).is_err() {
                                    self.metrics.warning();
                                }
                            }
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
impl BehaviorModule for PreferenceRecallModule {
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

/// 失败闭环模块 (`apeireth-companion` 口头强化反思, 2026-10-06 W2 接线).
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
impl BehaviorModule for ReflexionModule {
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
impl BehaviorModule for JudgeModule {
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
            lock_or_recover(&self.retries).remove(ctx.session_id);
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
                            let mut retries = lock_or_recover(&self.retries);
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
impl BehaviorModule for SelfAssessmentModule {
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
impl BehaviorModule for CouncilModule {
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
/// Perception remains an input adapter, not an BehaviorModule.  Only the text
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

// ============================================================================
// W3.2 装配面: 修复指令 + 上下文策略 (两个默认关的开关)
// ============================================================================
//
// 能力级别 (四级口径): IMPLEMENTED (crate 内接线) + 默认关. **不是**
// DEFAULT ENABLED、**不是** PRODUCTION WIRED: `production.rs` 未注册
// [`BioAssemblyModule`], 装配根接线属后续批次; 本模块只装配显式交来的输入.
//
// - `APEIRETH_ENABLE_BIO_REPAIR_GUIDANCE=1` + 显式注入 [`RepairPlan`]:
//   [`PromptAssembler::with_repair_plan`] 注册 `RepairDirectiveSource`, 把
//   [`REPAIR_DIRECTIVE_PLACEHOLDER`] 放进 system 角色的独立 core 块;
// - `APEIRETH_ENABLE_BIO_CONTEXT_POLICY=1` + 显式预算/已用量: 由实测
//   ContextPressure (其余 4 维取健康基线中性值) → `HomeostasisRegulator::regulate`
//   → `AssemblyPolicy::from_plan` → `ContextAssembler::with_policy` 参与装配;
// - 两开关都关 / 无数据: 既有输出逐字节不变 (不构造体征、不注册 source、不发
//   overlay; 模块钩子直接 `ModuleOutcome::continue_()`).
//
// **未接线 (如实标注)**: cognitive 侧没有会话纠正/矛盾事件台账, 所以
// `UncertaintySignal` 采集与 `social_repair::plan_repair` 调用**不在本批**;
// [`RepairPlan`] 只能由调用方显式注入, 本模块**不**从任意文本臆造信号.

/// 修复指令装配开关 env 名 (仅显式 `1` 开, 默认关 fail-closed).
pub const BIO_REPAIR_GUIDANCE_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_REPAIR_GUIDANCE";

/// 上下文策略装配开关 env 名 (仅显式 `1` 开, 默认关 fail-closed).
pub const BIO_CONTEXT_POLICY_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_CONTEXT_POLICY";

/// 装配面模块 id (slot ledger 键; 改 id = 兼容性变更).
pub const BIO_ASSEMBLY_MODULE_ID: &str = "cognitive.bio_assembly";

/// 修复指令块名 (core 块: 预算回收不截断该段).
const BIO_REPAIR_BLOCK_NAME: &str = "bio_repair_directive";

/// 非核心上下文块名 (预算报告可读).
const BIO_CONTEXT_BLOCK_NAME: &str = "bio_context";

/// 无预算数据时的装配上限: 不限额 (不发明预算数字, 等于不裁剪).
const BIO_UNBOUNDED_BUDGET_CHARS: usize = usize::MAX;

/// 开关解析 (纯函数): 仅显式 `1` 开, 其余 (未设 / 空串 / `0` / `true` /
/// `yes` / `on`) 一律关 (默认关 fail-closed).
pub fn bio_switch_enabled_from_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

/// 两个默认关的装配开关 (逐开关独立; 全关 = 今日行为逐字节不变).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BioAssemblySwitches {
    repair_guidance: bool,
    context_policy: bool,
}

impl BioAssemblySwitches {
    /// 全关 (默认; 等价于本批之前的行为).
    pub const fn disabled() -> Self {
        Self {
            repair_guidance: false,
            context_policy: false,
        }
    }

    /// 从两个原始值构造 (纯函数, 便于 fail-closed 表测试而不动进程 env).
    pub fn from_values(repair_guidance: Option<&str>, context_policy: Option<&str>) -> Self {
        Self {
            repair_guidance: bio_switch_enabled_from_env_value(repair_guidance),
            context_policy: bio_switch_enabled_from_env_value(context_policy),
        }
    }

    /// 从装配根已解析的布尔开关构造 (R1b-1b: env 读取收敛到 `NervousSwitchboard`).
    ///
    /// 本构造器**不读 env**: 两个开关值由组合根一次性解析后显式传入, 避免同一开关
    /// 在库内出现第二个读取点.
    pub const fn from_flags(repair_guidance: bool, context_policy: bool) -> Self {
        Self {
            repair_guidance,
            context_policy,
        }
    }

    /// 设置修复指令开关 (链式).
    pub fn with_repair_guidance(mut self, enabled: bool) -> Self {
        self.repair_guidance = enabled;
        self
    }

    /// 设置上下文策略开关 (链式).
    pub fn with_context_policy(mut self, enabled: bool) -> Self {
        self.context_policy = enabled;
        self
    }

    /// 修复指令开关现况.
    pub const fn repair_guidance(self) -> bool {
        self.repair_guidance
    }

    /// 上下文策略开关现况.
    pub const fn context_policy(self) -> bool {
        self.context_policy
    }

    /// 两个开关是否都关 (关 = 零行为变化, 钩子不做任何装配).
    pub const fn is_disabled(self) -> bool {
        !self.repair_guidance && !self.context_policy
    }
}

/// 显式交来的非核心上下文块 (策略只折叠 / 裁剪非核心块; 核心块只有修复段).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BioContextBlock {
    /// 块正文 (调用方自有文本, 通常来自召回 / 投影面).
    pub content: String,
    /// 单块字符上限 (`None` = 只受总预算约束).
    pub cap_chars: Option<usize>,
}

impl BioContextBlock {
    /// 构造一个非核心上下文块.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            cap_chars: None,
        }
    }

    /// 设置单块字符上限 (链式).
    pub fn with_cap(mut self, cap_chars: usize) -> Self {
        self.cap_chars = Some(cap_chars);
        self
    }
}

/// 一次装配的显式输入: 每个字段都由调用方给出, 本模块不从任意文本臆造.
#[derive(Debug, Clone, Default)]
pub struct BioTurnInput {
    /// 本回合可用的修复计划 (来自 `social_repair::plan_repair` 或调用方).
    pub repair_plan: Option<RepairPlan>,
    /// 本回合要注入的非核心上下文块.
    pub context_blocks: Vec<BioContextBlock>,
    /// 本回合上下文预算 (字符); `None` = 无预算数据 (不构造体征 → 默认策略).
    pub context_budget_chars: Option<usize>,
    /// 本回合上下文已用量 (字符); 无预算数据时不参与判定.
    pub context_used_chars: Option<usize>,
}

impl BioTurnInput {
    /// 空输入 (= 什么都不装配).
    pub fn new() -> Self {
        Self::default()
    }

    /// 注入修复计划 (链式).
    pub fn with_repair_plan(mut self, plan: RepairPlan) -> Self {
        self.repair_plan = Some(plan);
        self
    }

    /// 注入上下文预算与已用量 (链式; 两者都给才参与 ContextPressure 判定).
    pub fn with_context_usage(mut self, used_chars: usize, budget_chars: usize) -> Self {
        self.context_used_chars = Some(used_chars);
        self.context_budget_chars = Some(budget_chars);
        self
    }

    /// 追加一个非核心上下文块 (链式).
    pub fn with_context_block(mut self, block: BioContextBlock) -> Self {
        self.context_blocks.push(block);
        self
    }
}

/// 一次装配的可观测结果 (全部字段来自本次装配, 不含用户原文).
#[derive(Debug, Clone, PartialEq)]
pub struct BioTurnAssembly {
    /// 装配后的 system 角色 overlay (顺序: 修复段在前, 非核心块在后).
    pub overlays: Vec<PromptOverlay>,
    /// 本次实际使用的装配策略 (无数据 / 开关关 = `AssemblyPolicy::default()`).
    pub policy: AssemblyPolicy,
    /// 本次的调节计划 (`None` = 没有体征数据, 未构造调节).
    pub regulation: Option<RegulationPlan>,
    /// 是否真的注册了修复指令 source (false = 组装输出与未接线一致).
    pub repair_registered: bool,
    /// 进入修复段的动作数 (≤ [`apeireth_organ::social_repair::MAX_REPAIR_MOVES`]).
    pub repair_moves: usize,
    /// 策略生效后的装配预算 (只有「开关开 + 预算与已用量都给」时给出;
    /// `None` = 没构造体征: 无策略收紧 / 无数据 / 开关关).
    pub effective_budget_chars: Option<usize>,
    /// 装配中没有定义、被原样保留的占位符 (正常应为空; 空 = 全部解析).
    pub undefined_vars: Vec<String>,
}

/// 上下文压力: 已用量 / 预算, 夹到 `[0, 1]` (预算 `0` 视为无压力).
fn bio_context_pressure(used_chars: usize, budget_chars: usize) -> f64 {
    if budget_chars == 0 {
        return 0.0;
    }
    (used_chars.min(budget_chars) as f64) / (budget_chars as f64)
}

/// 上下文策略映射结果 (策略 + 调节计划 + 生效预算).
struct BioContextPolicy {
    policy: AssemblyPolicy,
    regulation: Option<RegulationPlan>,
    effective_budget_chars: Option<usize>,
}

/// 体征 → 策略映射 (纯函数, 无 IO / 无时钟).
///
/// 只在「策略开关开 + 预算与已用量都给 + 预算 > 0」时构造 [`VitalSignSet`];
/// 其余情况一律返回默认策略 (行为不变). 只有 ContextPressure 是实测值,
/// 其余 4 维沿用 [`VitalSignSet::healthy`] 的中性基线 (设定值附近, 不越限).
///
/// 用 [`HomeostasisRegulator::regulate`] 而非 `regulate_with_trend`: cognitive
/// 侧没有上一窗口的体征快照, 不做趋势外推 (不编造第二窗口数据 —— 0 装诚实).
fn bio_context_policy(switches: BioAssemblySwitches, input: &BioTurnInput) -> BioContextPolicy {
    let inactive = BioContextPolicy {
        policy: AssemblyPolicy::default(),
        regulation: None,
        effective_budget_chars: None,
    };
    if !switches.context_policy() {
        return inactive;
    }
    let Some(budget_chars) = input.context_budget_chars.filter(|budget| *budget > 0) else {
        // 无预算数据: 不发明预算 → 默认策略 (行为不变).
        return inactive;
    };
    let Some(used_chars) = input.context_used_chars else {
        // 有预算但无已用量: 不猜占用率 → 默认策略 (行为不变).
        return inactive;
    };
    let pressure = bio_context_pressure(used_chars, budget_chars);
    let vitals = VitalSignSet::healthy().with(
        VitalSign::ContextPressure,
        VitalSignReading::defaults_for(VitalSign::ContextPressure, pressure),
    );
    let regulation = HomeostasisRegulator::new().regulate(&vitals);
    let policy = AssemblyPolicy::from_plan(&regulation);
    BioContextPolicy {
        policy,
        effective_budget_chars: Some(policy.effective_budget_chars(budget_chars)),
        regulation: Some(regulation),
    }
}

/// 装配一次: 修复指令段 (system 独立 core 块) + 策略化非核心上下文块.
///
/// 两开关都关时整条装配面停用: 返回空 `overlays` + 默认策略 + 不构造体征
/// (与未接线逐字节一致, 即使输入全给); 开关开但输入为空 (空 plan / 无预算)
/// 时同样不注册、不改策略.
pub fn assemble_bio_turn(switches: BioAssemblySwitches, input: &BioTurnInput) -> BioTurnAssembly {
    if switches.is_disabled() {
        return BioTurnAssembly {
            overlays: Vec::new(),
            policy: AssemblyPolicy::default(),
            regulation: None,
            repair_registered: false,
            repair_moves: 0,
            effective_budget_chars: None,
            undefined_vars: Vec::new(),
        };
    }
    let context_policy = bio_context_policy(switches, input);
    let configured_budget = input
        .context_budget_chars
        .filter(|budget| *budget > 0)
        .unwrap_or(BIO_UNBOUNDED_BUDGET_CHARS);
    let mut prompt = PromptAssembler::new();
    let mut context = ContextAssembler::new(configured_budget).with_policy(context_policy.policy);

    // 修复指令段: 只有「开关开 + 有可渲染动作」才注册 source 并放置独立块.
    let mut repair_registered = false;
    let mut repair_moves = 0_usize;
    if switches.repair_guidance() {
        if let Some(plan) = &input.repair_plan {
            if let Some(section) = repair_guidance(plan) {
                repair_moves = section.moves().len();
                repair_registered = true;
                prompt = prompt.with_repair_plan(plan);
                context = context.push(
                    ContextBlock::new(BIO_REPAIR_BLOCK_NAME, REPAIR_DIRECTIVE_PLACEHOLDER)
                        .core(true),
                );
            }
        }
    }

    for block in &input.context_blocks {
        let mut injected = ContextBlock::new(BIO_CONTEXT_BLOCK_NAME, block.content.clone());
        if let Some(cap_chars) = block.cap_chars {
            injected = injected.with_cap(cap_chars);
        }
        context = context.push(injected);
    }

    let mut guard = AssemblyGuard::new();
    let (blocks, report) = prompt.assemble(&context, AssemblyRole::System, &mut guard);
    let overlays = blocks
        .into_iter()
        .filter(|block| !block.content.trim().is_empty())
        .map(|block| PromptOverlay::system(block.content))
        .collect();
    BioTurnAssembly {
        overlays,
        policy: context_policy.policy,
        regulation: context_policy.regulation,
        repair_registered,
        repair_moves,
        effective_budget_chars: context_policy.effective_budget_chars,
        undefined_vars: report.undefined,
    }
}

/// 可见 transcript 的字符数 (只取长度: 文本本身不进入装配输入).
fn transcript_chars(messages: &[NormalizedMessage]) -> usize {
    messages
        .iter()
        .map(|message| ContentPart::join_text(&message.content).chars().count())
        .sum()
}

/// W3.2 装配面模块: `TurnStart` 装配修复指令段 + 策略化上下文块.
///
/// 默认关 (两个开关都关或 env 未设) 时钩子直接返回
/// `ModuleOutcome::continue_()`: 不注册 source、不构造体征、不发 overlay.
/// 开关开时也只装配**显式注入**的 [`RepairPlan`] / 上下文块 / 预算 ——
/// 没有输入就是零变化, 不从任意文本臆造信号 (0 装诚实).
///
/// **未接线 (如实标注)**: `production.rs` 尚未注册本模块, 无会话事件源驱动
/// `UncertaintySignal`; 计划与预算需装配根显式注入.
#[derive(Debug)]
pub struct BioAssemblyModule {
    manifest: ModuleManifest,
    switches: BioAssemblySwitches,
    repair_plan: Option<RepairPlan>,
    context_blocks: Vec<BioContextBlock>,
    context_budget_chars: Option<usize>,
}

impl BioAssemblyModule {
    /// 按给定开关构造 (默认关 = 行为不变).
    pub fn new(switches: BioAssemblySwitches) -> Self {
        Self {
            manifest: ModuleManifest::new(BIO_ASSEMBLY_MODULE_ID, "Cognitive Bio Assembly"),
            switches,
            repair_plan: None,
            context_blocks: Vec::new(),
            context_budget_chars: None,
        }
    }

    /// 从装配根已解析的布尔开关构造 (R1b-1b: 本模块不再自行读 env).
    pub fn from_flags(repair_guidance: bool, context_policy: bool) -> Self {
        Self::new(BioAssemblySwitches::from_flags(
            repair_guidance,
            context_policy,
        ))
    }

    /// 注入本回合修复计划 (链式; 仅修复指令开关开时生效).
    pub fn with_repair_plan(mut self, plan: Option<RepairPlan>) -> Self {
        self.repair_plan = plan;
        self
    }

    /// 注入本回合非核心上下文块 (链式; 仅上下文策略开关开时受策略折叠).
    pub fn with_context_blocks(mut self, blocks: Vec<BioContextBlock>) -> Self {
        self.context_blocks = blocks;
        self
    }

    /// 注入本回合上下文预算 (链式; 无预算 = 默认策略, 行为不变).
    pub fn with_context_budget(mut self, budget_chars: usize) -> Self {
        self.context_budget_chars = Some(budget_chars);
        self
    }

    /// 开关现况 (观测口).
    pub const fn switches(&self) -> BioAssemblySwitches {
        self.switches
    }

    /// 装配一次 (`used_chars` = 本回合已用量; 无预算数据时被忽略).
    pub fn assemble(&self, used_chars: Option<usize>) -> BioTurnAssembly {
        let input = BioTurnInput {
            repair_plan: self.repair_plan.clone(),
            context_blocks: self.context_blocks.clone(),
            context_budget_chars: self.context_budget_chars,
            context_used_chars: used_chars,
        };
        assemble_bio_turn(self.switches, &input)
    }
}

#[async_trait::async_trait]
impl BehaviorModule for BioAssemblyModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook != HookPoint::TurnStart || self.switches.is_disabled() {
            return Ok(ModuleOutcome::continue_());
        }
        // 已用量只取 transcript 长度 (数字), 原文不进入装配输入.
        let used_chars = self
            .switches
            .context_policy()
            .then(|| transcript_chars(ctx.messages));
        let assembly = self.assemble(used_chars);
        let mut outcome = ModuleOutcome::continue_();
        for overlay in assembly.overlays {
            outcome = outcome.with_prompt_overlay(overlay);
        }
        Ok(outcome)
    }
}

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

    /// 脚本化散列门 (测试用): 按预设判定回答, 记录两笔记账动作。
    struct ScriptedGate {
        decision: std::sync::Mutex<Result<apeireth_memory::GateDecision, String>>,
        calls: std::sync::Mutex<Vec<&'static str>>,
    }

    impl ScriptedGate {
        fn new(decision: Result<apeireth_memory::GateDecision, String>) -> Arc<Self> {
            Arc::new(Self {
                decision: std::sync::Mutex::new(decision),
                calls: std::sync::Mutex::new(Vec::new()),
            })
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl apeireth_memory::IngestGate for ScriptedGate {
        fn check(
            &self,
            _source: &str,
            content: &str,
        ) -> Result<apeireth_memory::GateDecision, String> {
            assert!(!content.trim().is_empty(), "门素材不得为空");
            self.calls.lock().unwrap().push("check");
            self.decision.lock().unwrap().clone()
        }

        fn mark_ingested(&self, _source: &str, _content: &str, _now_ms: i64) -> Result<(), String> {
            self.calls.lock().unwrap().push("ingested");
            Ok(())
        }

        fn mark_skipped(&self, _source: &str, _content: &str, _now_ms: i64) -> Result<u64, String> {
            self.calls.lock().unwrap().push("skipped");
            Ok(1)
        }
    }

    /// V4-b 散列增量门: 同源同散列 = 跳过**整轮**重复整编 (省算力防重复记忆);
    /// 跳过记去重计数, 不记摄入账。
    #[tokio::test]
    async fn hash_gate_skips_a_duplicate_consolidation_round() {
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
        let gate = ScriptedGate::new(Ok(apeireth_memory::GateDecision::SkipDuplicate {
            content_hash: "0".repeat(16),
            first_seen_ms: 1,
            skip_count: 0,
        }));

        let module = MemoryWritebackModule::new(memory.clone(), Arc::clone(&clock))
            .with_coordinator(Arc::clone(&coordinator))
            .with_consolidation()
            .with_ingest_gate(gate.clone());
        module
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

        assert!(
            insight_ids(&memory, &session.to_string()).is_empty(),
            "跳过重复整编 = 整轮不跑 (不产任何洞察)"
        );
        assert_eq!(
            gate.calls(),
            vec!["check", "skipped"],
            "只查门 + 记去重计数; 摄入记账不得出现"
        );
    }

    /// V4-b 散列增量门 fail-open: 门自身故障不挡记忆固化 —— 照常整编, 记账
    /// 按**整编结局**走 (整编成功 = 如实记一笔摄入, 不因查门失败漏记/伪造)。
    #[tokio::test]
    async fn hash_gate_failures_do_not_block_consolidation() {
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
        let gate = ScriptedGate::new(Err("gate store down".to_string()));

        let module = MemoryWritebackModule::new(memory.clone(), Arc::clone(&clock))
            .with_coordinator(Arc::clone(&coordinator))
            .with_consolidation()
            .with_ingest_gate(gate.clone());
        module
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

        assert_eq!(
            insight_ids(&memory, &session.to_string()).len(),
            2,
            "门故障 fail-open: 整编照常跑"
        );
        assert_eq!(
            gate.calls(),
            vec!["check", "ingested"],
            "查门失败不挡记账: 整编成功照常记摄入 (账按结局, 不按查门结果)"
        );
    }

    /// V4-b 记账语义: 整编成功后才记摄入账 (一次成功整编 = 恰好一笔)。
    #[tokio::test]
    async fn hash_gate_marks_ingested_once_after_a_successful_round() {
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
        let gate = ScriptedGate::new(Ok(apeireth_memory::GateDecision::Ingest {
            content_hash: "0".repeat(16),
        }));

        let module = MemoryWritebackModule::new(memory.clone(), Arc::clone(&clock))
            .with_coordinator(Arc::clone(&coordinator))
            .with_consolidation()
            .with_ingest_gate(gate.clone());
        module
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

        assert_eq!(
            gate.calls(),
            vec!["check", "ingested"],
            "放行 → 整编 → 成功后恰一笔摄入记账"
        );
    }

    /// V4-b 三件套端到端 (真散列门): 同素材第二轮跳过 (去重计数 +1, 不产重复
    /// 记忆); 变素材照常重摄 (去重计数不再涨)。
    #[tokio::test]
    async fn hash_gate_dedups_identical_material_and_reingests_changes() {
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
        let gate_store = Arc::new(apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap());
        let module = MemoryWritebackModule::new(memory.clone(), Arc::clone(&clock))
            .with_coordinator(Arc::clone(&coordinator))
            .with_consolidation()
            .with_ingest_gate(gate_store.clone());

        // 第一轮: 素材首见 = 放行整编。
        let first = NormalizedResponse::text("answer-1", "fake", "resolved the issue");
        module
            .on_hook(
                HookPoint::AfterTurn,
                &context(
                    &session,
                    &messages,
                    Some(&first),
                    &invoker,
                    MEMORY_WRITEBACK_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        let after_first = insight_ids(&memory, &session.to_string());
        assert!(!after_first.is_empty(), "第一轮必须整编");
        assert_eq!(
            gate_store.freshness_status().unwrap().dedup_skipped,
            0,
            "首见不计去重"
        );

        // 第二轮: 同素材 (同回合内容重复到会) = 跳过重复整编。
        module
            .on_hook(
                HookPoint::AfterTurn,
                &context(
                    &session,
                    &messages,
                    Some(&first),
                    &invoker,
                    MEMORY_WRITEBACK_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        assert_eq!(
            insight_ids(&memory, &session.to_string()),
            after_first,
            "跳过重复整编 = 不产重复记忆"
        );
        assert_eq!(
            gate_store.freshness_status().unwrap().dedup_skipped,
            1,
            "去重计数真值 +1"
        );

        // 第三轮: 素材有变 = 变内容重摄 (去重计数不再涨, 新洞察落库)。
        let second = NormalizedResponse::text("answer-2", "fake", "fixed the second issue");
        module
            .on_hook(
                HookPoint::AfterTurn,
                &context(
                    &session,
                    &messages,
                    Some(&second),
                    &invoker,
                    MEMORY_WRITEBACK_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        let after_third = insight_ids(&memory, &session.to_string());
        assert!(
            after_third.len() > after_first.len(),
            "变内容必须重摄整编: {after_first:?} -> {after_third:?}"
        );
        assert_eq!(
            gate_store.freshness_status().unwrap().dedup_skipped,
            1,
            "重摄不去重"
        );
    }

    /// V4-a 人格提取步: 整编触发点上从近期情节提炼人格增量 → 人格层落库;
    /// 用户事实不入人格层 (分账); 重复回合幂等; 整编关 = 显式不跑.
    #[tokio::test]
    async fn persona_extraction_step_runs_with_the_consolidation_trigger() {
        let session = SessionId::new();
        let memory = Arc::new(FakeMemory::default());
        memory
            .put_episode(&Episode {
                id: "e1".into(),
                timestamp: 1,
                role: "user".into(),
                content: "主人喜欢乌龙茶".into(),
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
        // 本轮回复是自述情节: 一句信念 + 一句底线.
        let candidate = NormalizedResponse::text(
            "answer-p",
            "fake",
            "我相信先验证再叙事。我绝不在未告知的情况下改用户数据。",
        );

        // 整编关 (默认): persona 面在场也不跑 —— 触发点就是整编触发点.
        let silent_persona = Arc::new(apeireth_memory::InMemoryPersonaMemoryStore::new());
        let off = MemoryWritebackModule::new(memory.clone(), Arc::clone(&clock))
            .with_coordinator(Arc::clone(&coordinator))
            .with_persona_layer(silent_persona.clone());
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
        assert!(
            silent_persona.load().unwrap().all_entries().is_empty(),
            "默认关: 不落人格沉淀 (0 装诚实)"
        );

        // 整编开 + persona 面配置: 人格提取步跑起来.
        let persona = Arc::new(apeireth_memory::InMemoryPersonaMemoryStore::new());
        let on = MemoryWritebackModule::new(memory.clone(), Arc::clone(&clock))
            .with_coordinator(Arc::clone(&coordinator))
            .with_consolidation()
            .with_persona_layer(persona.clone());
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
        let loaded = persona.load().unwrap();
        assert_eq!(loaded.core_beliefs.len(), 1, "自述信念入人格层");
        assert_eq!(loaded.boundaries.len(), 1, "自述底线入人格层");
        assert!(
            loaded.preferences.is_empty(),
            "用户事实 (主人喜欢乌龙茶) 不入人格层"
        );
        assert!(loaded.last_consolidation_at.is_some(), "最后整编真值已推进");

        // 再跑一轮 (同一批素材): 幂等, 条目不增殖.
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
        let again = persona.load().unwrap();
        assert_eq!(again.core_beliefs.len(), 1);
        assert_eq!(again.boundaries.len(), 1);
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

        // TurnStart 注入教训块 (反刍格式).
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
                // 生物机制族 W3.2: BioAssemblyModule 为懒注册（两个 env 全关 =
                // 不构造 = 不注册，见 startup_instrumentation 的懒加载语义）；
                // 因此默认配置下不占槽位，开关开时注册在 self_assessment 与
                // memory_writeback 之间（顺序由 production 注册点固定）。
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

    /// V4-a 恒定小注入: TurnStart 注入自我叙事摘要, 与 [`build_constant_injection`]
    /// 同一渲染同一恒预算 (超长截断 + 显式省略注记, 不静默裁剪)。
    #[tokio::test]
    async fn persona_constant_injection_is_wired_at_turn_start_with_the_fixed_budget() {
        let persona = Arc::new(apeireth_memory::InMemoryPersonaMemoryStore::new());
        let mut memory = apeireth_memory::PersonaMemory::default();
        memory.apply_extract(
            &apeireth_memory::PersonaExtract {
                narrative_lines: vec![apeireth_memory::PersonaStatement {
                    text: "我是一个先验证再叙事的记忆体。".to_string(),
                    source_episode_id: "e-n".to_string(),
                }],
                ..Default::default()
            },
            1_728_400_000,
        );
        persona.save(&memory).unwrap();

        let module = PersonaConstantInjectionModule::new(persona.clone());
        let session = SessionId::new();
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = Arc::new(FixedInvoker {
            response: NormalizedResponse::text("x", "fake", "ok"),
            calls: AtomicU64::new(0),
        });
        let messages = vec![NormalizedMessage::user("hi")];
        let outcome = module
            .on_hook(
                HookPoint::TurnStart,
                &context(
                    &session,
                    &messages,
                    None,
                    &invoker,
                    PERSONA_INJECTION_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        assert_eq!(outcome.prompt_overlays.len(), 1);
        let text = ContentPart::join_text(&outcome.prompt_overlays[0].message().content);
        assert_eq!(
            text,
            apeireth_memory::build_constant_injection(&persona.load().unwrap()),
            "注入文本必须与回取面纯渲染逐字一致"
        );
        assert!(text.contains("【我是谁 · 恒定自述】"), "{text}");
        assert!(text.contains("我是一个先验证再叙事的记忆体。"), "{text}");

        // 恒预算钉死: 超长自述进注入面 = 400 字符头部 + 显式省略注记。
        let mut long = apeireth_memory::PersonaMemory::default();
        long.self_narrative = "认".repeat(600);
        persona.save(&long).unwrap();
        let outcome = module
            .on_hook(
                HookPoint::TurnStart,
                &context(
                    &session,
                    &messages,
                    None,
                    &invoker,
                    PERSONA_INJECTION_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        let text = ContentPart::join_text(&outcome.prompt_overlays[0].message().content);
        let body = text.trim_start_matches("【我是谁 · 恒定自述】\n");
        let head: String = body
            .chars()
            .take(apeireth_memory::CONSTANT_INJECTION_BUDGET_CHARS)
            .collect();
        assert!(body.starts_with(&head), "恒预算头部必须逐字保留");
        assert!(
            body.contains("已省略"),
            "超长截断必须留显式省略注记: {body}"
        );
        assert!(
            body.chars().count() <= apeireth_memory::CONSTANT_INJECTION_BUDGET_CHARS + 32,
            "恒预算溢出: {} 字符",
            body.chars().count()
        );
    }

    /// 无人格沉淀 = 零注入零变化: 空自述不发 overlay (未沉淀就是未沉淀, 不编)。
    #[tokio::test]
    async fn without_persona_sediment_the_turn_start_outcome_carries_no_overlay() {
        let persona = Arc::new(apeireth_memory::InMemoryPersonaMemoryStore::new());
        let module = PersonaConstantInjectionModule::new(persona);
        let session = SessionId::new();
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = Arc::new(FixedInvoker {
            response: NormalizedResponse::text("x", "fake", "ok"),
            calls: AtomicU64::new(0),
        });
        let messages = vec![NormalizedMessage::user("hi")];
        let outcome = module
            .on_hook(
                HookPoint::TurnStart,
                &context(
                    &session,
                    &messages,
                    None,
                    &invoker,
                    PERSONA_INJECTION_MODULE_ID,
                ),
            )
            .await
            .unwrap();
        assert!(
            outcome.prompt_overlays.is_empty(),
            "无沉淀必须零注入 (提示词逐字不变)"
        );
    }

    /// 装配缝 (V4-a): 恒定注入模块只在人格层落库面在场时注册 —— 缺人格层 =
    /// 不注册 = 零注入零变化。
    #[tokio::test]
    async fn the_injection_module_registers_only_with_a_persona_layer() {
        let clock: Arc<dyn Clock> = Arc::new(VirtualClock::new(
            apeireth_core::kernel::Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ));
        let mut config = CognitiveModuleConfig::default();
        config.judge.enabled = false;
        config.self_assessment = false;
        let mem = Arc::new(FakeMemory::default());
        let base = || CognitiveBackends {
            memory: Some(mem.clone()),
            memory_governance: Some(mem.clone()),
            preferences: Some(Arc::new(FakePreferences)),
            ..CognitiveBackends::default()
        };

        // 缺人格层: 模块不进装配 (零注入零变化)。
        let absent = super::super::production::ProductionCognitiveModules::build(
            config.clone(),
            base(),
            clock.clone(),
        )
        .unwrap();
        assert!(
            !absent
                .ids()
                .contains(&PERSONA_INJECTION_MODULE_ID.to_string()),
            "缺人格层不得注册恒定注入模块: {:?}",
            absent.ids()
        );

        // 有人格层: TurnStart 装配点收进恒定注入模块。
        let present = super::super::production::ProductionCognitiveModules::build(
            config,
            CognitiveBackends {
                persona_layer: Some(Arc::new(apeireth_memory::InMemoryPersonaMemoryStore::new())),
                ..base()
            },
            clock,
        )
        .unwrap();
        assert!(
            present
                .ids()
                .contains(&PERSONA_INJECTION_MODULE_ID.to_string()),
            "有人格层必须注册恒定注入模块: {:?}",
            present.ids()
        );
    }

    /// V4-c 检索面真接线: MemoryRecallModule 的注入回执喂自进化接线层 ——
    /// 首轮注入回执待决 (信号面在场, 无证据不调权), 次轮再命中 = 正信号 →
    /// 调权真落账 (`retrieval_tuning.jsonl`) 且真落生效值。
    #[tokio::test]
    async fn recall_round_receipts_feed_the_retrieval_tuning_wire() {
        use super::super::retrieval_tuning_wire::{
            read_retrieval_tuning_log, RetrievalTuningWire, SignalSurface,
        };

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
        let coordinator = Arc::new(
            MemoryCoordinator::new(memory.clone(), memory.clone()).with_scoped_backend(scoped),
        );
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = Arc::new(FixedInvoker {
            response: NormalizedResponse::text("x", "fake", "ok"),
            calls: AtomicU64::new(0),
        });
        let messages = vec![NormalizedMessage::user("主人明天要交线代作业吗")];
        let log = std::env::temp_dir().join(format!(
            "apeireth-recall-tuning-{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&log);
        let wire = Arc::new(RetrievalTuningWire::new(log.clone()));
        let module = MemoryRecallModule::new(memory.clone())
            .with_coordinator(Arc::clone(&coordinator))
            .with_retrieval_tuning(Arc::clone(&wire));

        // 第一轮: e1 注入 → 回执待决 (无证据不调权), 信号面在场。
        module
            .on_hook(
                HookPoint::TurnStart,
                &context(&session, &messages, None, &invoker, MEMORY_RECALL_MODULE_ID),
            )
            .await
            .unwrap();
        assert_eq!(wire.evolution_count(), 0, "首轮回执待决不调权");
        assert_eq!(
            wire.signal_surface(),
            SignalSurface::Present {
                selective_rounds: 1
            }
        );
        assert_eq!(wire.pending_receipts()[0].id, "e1");

        // 第二轮: e1 再命中 = 正信号 → 条目 + 归因维度各动一步, 落账。
        module
            .on_hook(
                HookPoint::TurnStart,
                &context(&session, &messages, None, &invoker, MEMORY_RECALL_MODULE_ID),
            )
            .await
            .unwrap();
        assert_eq!(wire.evolution_count(), 2, "{:?}", wire.last_move());
        let summary = read_retrieval_tuning_log(&log).expect("自进化账已落");
        assert_eq!(summary.evolution_count(), 2);
        assert!(
            summary
                .last_move()
                .unwrap()
                .evidence
                .contains("注入后命中回执"),
            "{:?}",
            summary.last_move()
        );
        let _ = std::fs::remove_file(&log);
    }
}
