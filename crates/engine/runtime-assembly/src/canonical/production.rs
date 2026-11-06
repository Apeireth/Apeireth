//! The single production module composition root.
//!
//! Adapters supply concrete backends, tools, and the optional Council. This module
//! owns ordering and slot validation, so CLI, gateway, and future
//! embedding callers cannot accidentally register a second cognitive or tool spine.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use apeireth_core::kernel::Clock;
use apeireth_guard::hook::SharedImmuneMemory;
use apeireth_memory::{
    AccessHistoryActivationSource, ConsolidationPolicy, ConsolidationTrigger, ContextWindowManager,
    EmbeddingProvider, FileConsolidationWatermark, MemoryCoordinator, MemoryExtractor,
    MemoryGovernanceStore, MemoryMaterializer, MemoryMaterializerPort,
    MemoryTypedMaterializationSink, ScopedMemoryBackend, SqliteAccessHistoryStore,
    TypedMemoryRecallSource, TypedRecallIdentity,
};
use apeireth_orchestration::Council;
use apeireth_plugin::experience::{AssociationStore, KnowledgeGraphStore, WikiEntryStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_plugin::preference::PreferenceStore;
use apeireth_plugin::self_assessment::SelfAssessmentStore;
use apeireth_plugin::{Plugin, ToolCapability, ToolModule};
use apeireth_protocol::canonical::NormalizedMessage;
use apeireth_runtime::{ContextProjectionError, ContextProjector, RuntimeBuilder};
use apeireth_tools_canonical::mcp_bridge::{McpBridgeOptions, McpServerConfig};
use apeireth_tools_canonical::{CapabilitySwitch, MemoryLedgerStats, ObservedGate, StatusProbe};

use super::bio_approval_social::BioApprovalSocialSource;
use super::bio_extra_probes::CircadianProbeConfig;
use super::capability::CapabilityProvider;
use super::care_heartbeat::bio_lifecycle::BioLifecycleState;
use super::care_heartbeat::bio_tone::ToneState;
use super::cognitive::{
    CognitiveTelemetry, CouncilModule, JudgeModule, JudgeObservations, MemoryRecallAccessStore,
    MemoryRecallModule, MemoryWritebackModule, PreferenceRecallModule, SelfAssessmentModule,
};
use super::error::{RuntimeError, RuntimeResult};
use super::memory_typed_sink::CanonicalMemoryTypedSink;
use super::module::Module;
use super::nervous_bridge::NervousBridgeModule;
use super::nervous_bus::NervousBus;
use super::nervous_switchboard::{BioSwitch, NervousSwitchboard};
use super::organ_module::OrganModule;
use super::preference_learning::PreferenceLearningModule;
use super::prompt_injection_guard::{self, PromptInjectionReport};
use super::self_status_roster::roster_from_config;
use super::self_status_source::{
    doc_ops_status_probe, image_ops_status_probe, proposal_status_probe, sheet_ops_status_probe,
    task_board_status_probe, ProductionSelfStatusSource,
};
use super::tool_modules::{
    ApplyPatchModule, BackupExportModule, CareScheduleModule, ContentFeedModule, DocOpsModule,
    EducationModule, ExposureAuditModule, ExternalToolsModule, FetchModule, FilesystemModule,
    GitWriteModule, ImageOpsModule, McpModule, ProcessModule, ProposalModule, ReconModule,
    RepoMapModule, RepoModule, SearchModule, SelfStatusModule, SheetOpsModule, ShellModule,
    SkillDistillModule, SkillSearchModule, TaskBoardModule, TraitReportModule, VoiceSpeakModule,
    WorktreeApplyModule, WorktreePlanModule,
};
use super::trait_growth::{trait_growth_probe, trait_status_probe};
use crate::consequence_backfill::{default_data_root, ConsequenceBackfillModule};
use crate::startup::{StartupPhase, StartupTimeline};

// 生物机制族装配块（W3.2/W3.3/W3.5/W3.6）搬至子模块；`production.rs` 只留调用与字段。
#[path = "bio_production_wiring.rs"]
mod bio_production_wiring;
pub use bio_production_wiring::BioProbeSlots;

// 配置面（装什么、默认什么）搬至子模块；`production.rs` 只留装配流程（怎么装）。
// `pub use` 原样再导出 ⇒ `canonical::production::ProductionModulesConfig` 与
// crate 根 `apeireth_runtime_assembly::ProductionModulesConfig` 路径逐字不变
// ⇒ 外部调用点零改动。
#[path = "production_config.rs"]
mod production_config;
pub use production_config::{CognitiveModuleConfig, ProductionModulesConfig};

/// Adapter that exposes memory's context-window implementation through the
/// runtime-owned projection port. It is kept in assembly so the runtime kernel
/// does not depend on memory or storage.
pub struct MemoryContextProjector {
    manager: ContextWindowManager,
}

impl MemoryContextProjector {
    pub fn new(manager: ContextWindowManager) -> Self {
        Self { manager }
    }
}

impl ContextProjector for MemoryContextProjector {
    fn project(
        &self,
        transcript: &[NormalizedMessage],
        model_context_tokens: Option<u32>,
    ) -> Result<Vec<NormalizedMessage>, ContextProjectionError> {
        let projection = match model_context_tokens {
            // An unknown provider limit must not be treated as a tiny/default
            // budget: preserve the transcript rather than compacting eagerly.
            None => transcript.to_vec(),
            Some(context_tokens) => {
                self.manager
                    .project(transcript, context_tokens as usize)
                    .messages
            }
        };
        Ok(projection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::ContentPart;

    #[test]
    fn unknown_context_limit_preserves_transcript() {
        let projector = MemoryContextProjector::new(ContextWindowManager::default());
        let transcript = vec![
            NormalizedMessage::user("older context ".repeat(200)),
            NormalizedMessage::assistant("recent response"),
            NormalizedMessage::user("latest request"),
            NormalizedMessage::assistant("latest answer"),
            NormalizedMessage::user("follow-up"),
        ];
        let projected = projector.project(&transcript, None).unwrap();
        assert_eq!(projected, transcript);
        assert!(ContentPart::join_text(&projected[0].content).contains("older context"));
    }
}

/// Attach the default memory-backed context projector to a runtime builder.
#[must_use]
pub fn with_memory_context_projection(
    builder: RuntimeBuilder,
    manager: ContextWindowManager,
) -> RuntimeBuilder {
    builder.with_context_projector(Arc::new(MemoryContextProjector::new(manager)))
}

/// Concrete capability handles supplied by an adapter composition root.
///
/// Every field is optional so an embedding caller can choose a deliberate
/// subset.  A requested slot without its backend is a boot-time configuration
/// error, never a silently inert production registration.
#[derive(Default)]
pub struct ProductionBackends {
    /// Episode and history-stream backend.
    pub memory: Option<Arc<dyn MemoryBackend>>,
    /// Memory governance store (active/forgotten status, protection, overrides).
    pub memory_governance: Option<Arc<dyn MemoryGovernanceStore>>,
    /// Optional progressive-disclosure wiki store.
    pub wiki: Option<Arc<dyn WikiEntryStore>>,
    /// Optional knowledge graph store.
    pub graph: Option<Arc<dyn KnowledgeGraphStore>>,
    /// Optional association store.
    pub associations: Option<Arc<dyn AssociationStore>>,
    /// User preference store.
    pub preferences: Option<Arc<dyn PreferenceStore>>,
    /// Self-assessment store.
    pub self_assessments: Option<Arc<dyn SelfAssessmentStore>>,
    /// Council service, supplied only when the adapter explicitly enables it.
    pub council: Option<Arc<Council>>,
    /// Workspace root directory for local file tools.
    /// 关系里程碑存储（初见 / 阶段跃迁）。
    ///
    /// 缺席 = 不记里程碑（零行为变化）。生产装配走
    /// `SqliteMilestoneStore`（与认知库同一份库，关系史跨重启存活）。
    pub milestone_store: Option<Arc<dyn apeireth_memory::milestone::MilestoneStore>>,
    pub workspace_root: Option<PathBuf>,
    /// Scoped memory backend for cross-session storage queries.
    pub scoped_memory: Option<Arc<dyn ScopedMemoryBackend>>,
    /// Embedding provider for semantic memory retrieval.
    pub embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    /// Optional durable selected-context access history.
    pub access_history: Option<Arc<SqliteAccessHistoryStore>>,
    /// Optional **read-only** diary store (cross-diary retrieval wiring).
    ///
    /// 缺席 = 检索路径不碰日记 (零行为变化); 在场 = 命中事实可附**关联日记片段**
    /// (「你上周写过…」). 只读面: 检索路径只 `list_days` / `read_day`, 绝不写日记.
    pub diary: Option<Arc<dyn apeireth_memory::diary::DiaryStore>>,
    /// Optional unified materializer for bounded AfterTurn memory writeback.
    pub memory_materializer: Option<Arc<dyn MemoryMaterializerPort>>,
    /// Optional unified extractor used to build a default materializer.
    pub memory_extractor: Option<Arc<dyn MemoryExtractor>>,
    /// Optional typed durable candidate source for unified recall.
    pub typed_recall: Option<Arc<dyn TypedMemoryRecallSource>>,
    /// Explicit identity paired with the typed recall source.
    pub typed_recall_identity: Option<TypedRecallIdentity>,
    /// Optional concrete typed durable sink for commitment/persona/relation projections.
    pub typed_sink: Option<Arc<dyn MemoryTypedMaterializationSink>>,
    /// V4-a 人格记忆层落库面 (人格提取步的 `persona_memory_*` 分表).
    /// 缺位 = 人格提取步显式不跑 (默认未接线)。
    pub persona_layer: Option<Arc<dyn apeireth_memory::PersonaMemoryStore>>,
    /// V4-b 保鲜回路散列增量门落库面 (散列账 `source_hash` 与记忆库同库).
    /// 缺位 = 整编散列门显式不跑 (默认未接线)。
    pub ingest_gate: Option<Arc<dyn apeireth_memory::IngestGate>>,
    /// Reflexion failure-feedback store (2026-10-06 W2 记忆闭环批).
    pub reflexion_store: Option<Arc<dyn apeireth_memory::reflexion::ReflexionStore>>,
    /// W2 §4.2: partner 羁绊存储（默认生产装配走 `SqlitePartnerStore`，
    /// 2026-10-24 接上；trait 落点不绑后端）。
    pub partner_store: Option<Arc<dyn apeireth_memory::partner::PartnerStore>>,
    /// 「性格养成」第一铲 (默认无 = 自学习关): 自校准接线层, 接上后
    /// MemoryRecallModule 的召回结果作为真接信号喂给引擎。
    pub self_tuning: Option<Arc<crate::canonical::self_tuning_wire::SelfTuningWire>>,
    /// V4-c 检索维度自进化 (默认无 = 调权关): 自进化接线层, 接上后
    /// MemoryRecallModule 的注入回执作为真接信号面喂给引擎 (证据裁决 +
    /// `retrieval_tuning.jsonl` 自进化账落盘 + 调权生效值落地)。
    pub retrieval_tuning: Option<Arc<crate::canonical::retrieval_tuning_wire::RetrievalTuningWire>>,
    /// 自省通道: 记忆账本计数探测口 (会话/记忆/保护/教训计数, 只回计数)。
    pub self_status_ledger: Option<StatusProbe<MemoryLedgerStats>>,
    /// 自省通道: **问责计数**探测口 (影响声明 / 后果三态 / 待补救, 只回计数)。
    ///
    /// ADR-0014 §4 指定的「自述面新增只读子对象」。`None` = 不挂载 ⇒
    /// `capabilities.accountability` 子对象省略（旧帧形状不变）。
    pub self_status_accountability:
        Option<StatusProbe<apeireth_tools_canonical::AccountabilityStats>>,
    /// 自省通道: 凭据存在性探测口 (只回布尔, 不回显凭据本体)。
    pub self_status_credentials: Option<StatusProbe<bool>>,
    /// 自省通道: 组装根补充的能力名册行 (例如授权层的 `local_read_tools` 旋钮)。
    pub self_status_extras: Vec<CapabilitySwitch>,
    /// 自省通道: 生效中的预算一节 (可配置旋钮取值; 缺位回退编译期常量)。
    pub self_status_budget: Option<apeireth_tools_canonical::BudgetStatus>,
    /// 自省通道: 会话配额计量探测口 (按会话 id 读已耗真值, 只回数字)。
    pub self_status_quota: Option<apeireth_tools_canonical::SessionQuotaProbe>,
    /// 自省通道: 生效模型的画像 (H-P1 模型画像表解析结果; 未知模型传显式
    /// unknown 行; 缺位时模型节显式 null + 原因)。
    pub self_status_model_profile: Option<apeireth_core::model_profile::ModelProfile>,
    /// 自省通道: 成本分级路由节探测口 (H-P2: 主开关 + 最近一次路由决策真值,
    /// 与任务选型同一台路由器同源现读; 缺位时路由节显式 null + 原因)。
    pub self_status_cost_routing:
        Option<StatusProbe<apeireth_core::cost_router::CostRoutingStatus>>,
    /// 自省通道: 用户域节探测口 (W3-①: 当前域 id/角色/域内计数, 现探测如实)。
    pub self_status_domain: Option<StatusProbe<apeireth_tools_canonical::DomainStatus>>,
    /// 自省通道: 内容流面探测口 (J1: 订阅数/最近拉取, 现探测如实)。
    pub self_status_content_feed: Option<StatusProbe<apeireth_tools_canonical::ContentFeedStatus>>,
    /// 自省通道: 双引擎关怀节探测口 (V3-a: 规则数/下次触发/最近关怀真值, 现探测如实)。
    pub self_status_care: Option<StatusProbe<apeireth_tools_canonical::CareStatus>>,
    /// 自省通道: 备份节探测口 (G5: 最近备份时间/清单件数真值或 null, 现探测如实)。
    pub self_status_backup: Option<StatusProbe<apeireth_tools_canonical::BackupStatus>>,
    /// 自省通道: 语音节探测口 (V3-b: 账本数/最近说话/嗓名, 现探测如实;
    /// 与发声工具同一本语句账本 `spoken.jsonl`)。
    pub self_status_voice: Option<StatusProbe<apeireth_tools_canonical::VoiceStatus>>,
    /// 自省通道: 人格节探测口 (`self_status_persona`, V4-a: 信念/边界/偏好/
    /// 张力计数 + 最后整编真值, 现探测如实; 与人格层落库面同账, 只计数不回
    /// 内容)。
    pub self_status_persona: Option<StatusProbe<apeireth_tools_canonical::PersonaStatusSection>>,
}
/// Compatibility alias for [`ProductionBackends`].
pub type CognitiveBackends = ProductionBackends;

fn collect_tool_module(
    tool_modules: &mut Vec<Arc<dyn Plugin>>,
    capabilities: &mut Vec<Arc<dyn ToolCapability>>,
    provider: &dyn CapabilityProvider,
) -> RuntimeResult<()> {
    let module = provider
        .tool_module()
        .map_err(RuntimeError::misconfigured)?;
    capabilities.extend(module.tools());
    if !module.tools().is_empty() {
        tool_modules.push(Arc::new(module));
    }
    Ok(())
}

/// The validated, ordered module set to pass to [`RuntimeBuilder::with_module`].
pub struct ProductionModules {
    modules: Vec<Arc<dyn Module>>,
    tool_modules: Vec<Arc<dyn Plugin>>,
    capabilities: Vec<Arc<dyn ToolCapability>>,
    telemetry: Arc<CognitiveTelemetry>,
    mcp_definition: Option<Arc<apeireth_tools_canonical::mcp_bridge::McpDefinition>>,
    mcp_module: Option<Arc<McpModule>>,
    /// 器官模块句柄 (`config.organs` 开才 `Some`), 供组合根迟绑定 guard 免疫记忆.
    organ_module: Option<Arc<OrganModule>>,
    /// 生物机制族 W3.3: 调质状态句柄 (开关开才 `Some`, 与注册模块共享同一 `Arc`)。
    bio_tone_state: Option<Arc<ToneState>>,
    /// 生物机制族 W3.5 (SG-18): 生命周期状态句柄 (开关开才 `Some`, 与注册模块同一 `Arc`)。
    bio_lifecycle_state: Option<Arc<BioLifecycleState>>,
    /// 生物机制族 W3.6: 自述 probe 迟绑定共享槽 (开关关 = 槽不武装, 零读取).
    bio_probe_slots: BioProbeSlots,
    /// Wave 4 / ADR-0009 R1b-1a: 神经信号总线句柄 (适配器只 publish; 与注册的桥
    /// 共享同一 `Arc`)。全关时照常构造, 但没有桥/订阅者 = 零投递。
    nervous_bus: Arc<NervousBus>,
    /// Wave 4 / ADR-0009 R1b-1a: 本次装配生效的开关快照 (一次解析, 诊断读).
    nervous_switchboard: NervousSwitchboard,
    /// 回合层提示注入告警账句柄 (fetch 开才 `Some`, 与注册的告警模块同一 `Arc`)。
    /// `None` = fetch 面没开 = 没有任何输出侧标记可读 ⇒ 零告警 (不是「读了但没命中」)。
    pub(super) prompt_injection_report: Option<Arc<PromptInjectionReport>>,
}

/// Compatibility alias for [`ProductionModules`].
pub use ProductionModules as ProductionCognitiveModules;

/// 模块族枚举 (`canonical::ModuleFamily`) 的兼容再导出: 族枚举本体住在装配面
/// 这一层 (装配根 / 生物机制族装配块 / 回合层提示注入告警面三处共用) ⇒ ⇒ 旧路径
/// `production::ModuleFamily` 保留 ⇒ ⇒ 外部调用点零改动。
pub use super::ModuleFamily;

/// 构造探针 (B-P1 懒加载断言口): 族构造点上报一次。
///
/// 「开关关 = 不构造」由构造点上报钉住: 未启用族的构造被短路时探针零上报,
/// 测试即可断言**零构造** (构造点上报, 不是注册点上报)。上报在构造调用紧前:
/// 组装成功的语义下上报 = 构造发生 (个别族构造含前置校验, 校验失败整机组装
/// 拒绝, 计数不成账)。
pub trait ModuleBuildProbe: Send + Sync {
    /// 某族的构造发生了一次。
    fn constructed(&self, family: ModuleFamily);
}

/// 计数构造探针 (测试/诊断用): 按族计构造次数。
#[derive(Default)]
pub struct CountingBuildProbe {
    counts: Mutex<BTreeMap<ModuleFamily, u32>>,
}

impl CountingBuildProbe {
    /// 构造计数探针。
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// 某族的构造次数 (未构造 = 0)。
    pub fn count(&self, family: ModuleFamily) -> u32 {
        *self
            .counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&family)
            .unwrap_or(&0)
    }

    /// 全部族的构造总次数。
    pub fn total(&self) -> u32 {
        self.counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .sum()
    }
}

impl ModuleBuildProbe for CountingBuildProbe {
    fn constructed(&self, family: ModuleFamily) {
        *self
            .counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(family)
            .or_insert(0) += 1;
    }
}

/// 组装期插桩 (B-P1): 构造探针 + 分段计时, 生产传 [`BuildInstrumentation::default`]
/// = 全部无操作, 装配行为与不插桩完全一致。
#[derive(Default)]
pub struct BuildInstrumentation {
    /// 构造探针 (懒加载零构造断言口)。
    pub probe: Option<Arc<dyn ModuleBuildProbe>>,
    /// 分段计时记录面 (模块构造段落账)。
    pub timeline: Option<Arc<StartupTimeline>>,
}

impl BuildInstrumentation {
    /// 组装期插桩: 构造探针。
    #[must_use]
    pub fn with_probe(mut self, probe: Arc<dyn ModuleBuildProbe>) -> Self {
        self.probe = Some(probe);
        self
    }

    /// 组装期插桩: 分段计时记录面。
    #[must_use]
    pub fn with_timeline(mut self, timeline: Arc<StartupTimeline>) -> Self {
        self.timeline = Some(timeline);
        self
    }

    pub(super) fn constructed(&self, family: ModuleFamily) {
        if let Some(probe) = &self.probe {
            probe.constructed(family);
        }
    }
}

impl ProductionModules {
    /// Build the canonical registration order.
    pub fn build(
        config: ProductionModulesConfig,
        backends: ProductionBackends,
        clock: Arc<dyn Clock>,
    ) -> RuntimeResult<Self> {
        Self::build_instrumented(config, backends, clock, &BuildInstrumentation::default())
    }

    /// [`Self::build`] 的可插桩形态 (B-P1): 同一装配, 额外接受构造探针与
    /// 分段计时 (不插桩时与 [`Self::build`] 逐字节同行为)。
    pub fn build_instrumented(
        config: ProductionModulesConfig,
        backends: ProductionBackends,
        clock: Arc<dyn Clock>,
        instrument: &BuildInstrumentation,
    ) -> RuntimeResult<Self> {
        match &instrument.timeline {
            Some(timeline) => timeline.measure(StartupPhase::ModulesBuild, || {
                Self::build_inner(config, backends, clock, instrument)
            }),
            None => Self::build_inner(config, backends, clock, instrument),
        }
    }

    fn build_inner(
        mut config: ProductionModulesConfig,
        backends: ProductionBackends,
        clock: Arc<dyn Clock>,
        instrument: &BuildInstrumentation,
    ) -> RuntimeResult<Self> {
        let mut modules: Vec<Arc<dyn Module>> = Vec::new();
        // Wave 4 / ADR-0009 R1b-1a: 神经信号总线 + 开关表一次解析 (9 开关 + 1 参数,
        // fail-closed)。全关 = 不注册桥 = 零行为变化; 任一开关开 = 桥在模块表首位注册。
        let nervous_bus = Arc::new(NervousBus::new());
        let nervous_switchboard = NervousSwitchboard::from_env().map_err(|error| {
            RuntimeError::misconfigured(format!("nervous switchboard refused to load: {error}"))
        })?;
        // 两族生物总闸折进 config (env 读取收敛到装配根单入口): 值取自同一张
        // `NervousSwitchboard` —— 装配面与自述名册同源, 名册不再读 env (纯函数)。
        config.bio_play = nervous_switchboard.switch_enabled(BioSwitch::Play);
        config.bio_care_candidates = nervous_switchboard.switch_enabled(BioSwitch::CareCandidates);
        if nervous_switchboard.any_enabled() {
            modules.push(
                Arc::new(NervousBridgeModule::new(nervous_bus.as_ref().clone())) as Arc<dyn Module>,
            );
        }
        // 问责闭环第 ③ 步 · 自动回填面（ADR-0014 §5）：工具调用失败 ⇒ 把**对应的那条**
        // 已声明影响回填 `failed`。归属（action_id ↔ tool_call_id）**由调用方显式登记**
        // （`apeireth egress bind`），登记不到 ⇒ 不动 —— 系统不推断归属。
        //
        // **默认关**：与问责面共用同一个开关（`APEIRETH_ENABLE_EGRESS_GATE`）——
        // 开闸才装（闸 = 影响声明 + 后果账 + 自动回填这一整套）；关时**零注册**、
        // 零行为变化（未启用 = 不注册）。
        // `confirmed` **绝不**自动填（本地成功 ≠ 影响已确认 = 替外部背书）。
        if crate::egress_gate::egress_gate_enabled_from_env() {
            modules.push(Arc::new(ConsequenceBackfillModule::from_data_root(
                &default_data_root(),
            )) as Arc<dyn Module>);
        }
        let mut capabilities: Vec<Arc<dyn ToolCapability>> = Vec::new();
        let mut tool_modules: Vec<Arc<dyn Plugin>> = Vec::new();
        // 回合层提示注入告警账 (fetch 开时才构造 = 才有输出侧标记可读);
        // 组合根取回同一句柄, 让宿主能读「本进程见过几次注入命中」。
        let mut prompt_injection_report: Option<Arc<PromptInjectionReport>> = None;
        // W3.3: 器官模块句柄留给组合根迟绑定 guard 免疫记忆 (默认 None = 未注册).
        let mut organ_module: Option<Arc<OrganModule>> = None;
        // W3.6: 自述 probe 迟绑定槽 (安装点按开关武装; cli/组合根后续填句柄).
        let bio_probe_slots = BioProbeSlots::new();
        let observations = Arc::new(JudgeObservations::default());
        let telemetry = Arc::new(CognitiveTelemetry::default());

        // External tool bridge (MCP) server list is loaded BEFORE the roster
        // projection: the self-report must read the same facts the
        // registration uses. Fail-closed semantics unchanged: a defective
        // configuration is a boot error, never a silent fallback.
        let mcp_server_config = if config.mcp {
            Some(
                McpServerConfig::load(config.mcp_data_dir.as_deref()).map_err(|error| {
                    RuntimeError::misconfigured(format!(
                        "mcp server configuration refused to load: {error}"
                    ))
                })?,
            )
        } else {
            None
        };
        // 外部工具袋 (登记账): 早建 ⇒ 自述面可晚绑定「实际登记件数」; 生产里
        // 启动期接线把发现的工具登记进这只袋 ⇒ 袋里件数 = 实际登记件数。
        let mcp_tool_bag = mcp_server_config
            .as_ref()
            .map(|_| Arc::new(McpModule::new()));

        // 自省通道: 能力名册在任何字段移动之前投影 —— 名册即**实际用于装配的**
        // config 生效值; 注入槽级的注册条件 (图谱槽 / 外部工具桥面) 先折进
        // 生效视图, 名册不复述 "配置说开"而注册没成的事实。
        let mut roster_config = config.clone();
        roster_config.community_triage = config.community_triage && backends.graph.is_some();
        // 外部工具面两行口径分离: `mcp` = 实际登记件数 (启动期接线后才有数 ⇒ 现投影)。
        roster_config.mcp_bridge = mcp_server_config.is_some();
        // 自学习 / 类型化召回 / 检索调权的生效值 = 接线事实 AND 上游槽 (自学习
        // 信号只走记忆召回模块, 类型化召回候选只走记忆协调器, 检索调权挂记忆召回)。
        let self_tuning_effective = backends.self_tuning.is_some() && config.memory_recall;
        let retrieval_tuning_effective =
            backends.retrieval_tuning.is_some() && config.memory_recall;
        let typed_recall_effective =
            backends.typed_recall.is_some() && (config.memory_recall || config.memory_writeback);
        // V4-a 人格提取步的生效值 = 开关 AND 人格层落库面 (面缺席 = 显式不跑).
        roster_config.persona_consolidation =
            config.persona_consolidation && backends.persona_layer.is_some();
        // V4-b 散列增量门的生效值 = 开关 AND 散列门落库面 (面缺席 = 显式不跑)。
        roster_config.consolidation_hash_gate =
            config.consolidation_hash_gate && backends.ingest_gate.is_some();

        let experience_count = [
            backends.wiki.is_some(),
            backends.graph.is_some(),
            backends.associations.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();
        if experience_count != 0 && experience_count != 3 {
            return Err(RuntimeError::misconfigured(
                "Experience wiring must supply Wiki, knowledge graph, and association stores together",
            ));
        }

        // Register tool capabilities independently of behavior modules.
        // 读前观测门禁一张表共享: 读工具记录的观测就是写入端「已读」的证据
        // (跨工具同一会话观测, 未读不得覆盖写)。
        let observed_gate = Arc::new(ObservedGate::new());
        if config.filesystem {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::Filesystem);
                let provider =
                    FilesystemModule::new_with_gate(root.clone(), Arc::clone(&observed_gate));
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        if config.search {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::Search);
                let provider = SearchModule::new(root.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        if config.repo {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::Repo);
                let provider = RepoModule::new(root.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 代码地图 (`tool.repo_map`, 2026-10-24): 跨语言符号提取 + 跨文件
        // 依赖图 + Personalized PageRank, token 预算内输出紧凑大纲。
        // **只读**、工作区限域、三重预算封顶 (文件数 / 单文件字节 / token)。
        // 扫描根与 `tool.repo` 同源 (同一个 `backends.workspace_root`)。
        if config.repo_map {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::Repo);
                let provider = RepoMapModule::new(root.clone());
                capabilities.extend(provider.capabilities());
            }
        }

        // 受控文件写入 (`tool.apply_patch`, 第七件生产工具, opt-in): 补丁式
        // 创建/修改/删除显式声明 + 读前观测门禁; 写入风险档位默认 require-approval
        // 级 (每次写入都要人批), 授权层按同一开关接线。git 写边界 (分级语义):
        // 仓库写操作走 `tool.git_write` 的分级词表 (L1 默认人批 / L2 永远人批 /
        // L3 毁灭无工具面)。
        if config.file_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::ApplyPatch);
                let provider = ApplyPatchModule::new(root.clone(), Arc::clone(&observed_gate));
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 干活能力轴 P2 (「计划→改→验证→落」四拍受控工作树写, opt-in): 注册条件 =
        // 开关 + `file_write` 唯一写总闸 + 工作区根 (注册即闸)。plan 只读改单面
        // (零落盘, 产物是 apply 的输入合同); apply 走**共享**读前观测门禁 (未读
        // 不得覆盖/删除), 每次落盘人批 (无自动放行档), plan 只读零审批。
        if config.worktree_plan && config.file_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::WorktreePlan);
                let provider = WorktreePlanModule::new(root.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        if config.worktree_apply && config.file_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::WorktreeApply);
                let provider = WorktreeApplyModule::new(root.clone(), Arc::clone(&observed_gate));
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // git 写分级 (`tool.git_write`, 干活能力轴 P1, opt-in): 分级词表 +
        // 词法分级执行面 + 受控通道 (工作区限域/超时/输出归一)。L1 默认人批
        // (设置可开「自动放行 L1」档), L2 永远人批, L3 拒绝即帧无工具面。
        if config.git_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::GitWrite);
                let provider = GitWriteModule::new(root.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 书→技能沉淀 (P 轴吸收自动化 P1, 默认关): 两级工具面。`tool.skill_distill`
        // 是写文件面 —— 注册条件 = 开关 + `file_write` 唯一写总闸 + 工作区根 +
        // 技能库目录 (注册即闸), 每次沉淀人批; `tool.skill_search` 是只读档
        // (零审批), 只要技能库目录 (`skills/` 数据目录, 每卡一 StoredDoc 拒开)。
        if config.skill_distill && config.file_write {
            if let (Some(root), Some(skills_dir)) = (&backends.workspace_root, &config.skills_dir) {
                instrument.constructed(ModuleFamily::SkillDistill);
                let provider = SkillDistillModule::new(root.clone(), skills_dir.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        if config.skill_search {
            if let Some(skills_dir) = &config.skills_dir {
                instrument.constructed(ModuleFamily::SkillSearch);
                let provider = SkillSearchModule::new(
                    skills_dir.clone(),
                    apeireth_tools_canonical::skill::DEFAULT_SKILL_DISCLOSURE_BUDGET_CHARS,
                );
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // W2 §4.3 (2026-10-10, 默认关): education Dx-Check 工具 (纯确定性, 0 副作用)。
        if config.education {
            instrument.constructed(ModuleFamily::Education);
            let provider = EducationModule::new();
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 文件写入总闸 (宪法级, 内测整改): `file_write` 是**唯一写总闸** ——
        // 组装根把同一开关值注入 shell 配置, shell 写命令与 apply_patch 同受此闸。
        // 关 = 写意图拒绝即帧 (`pipeline.pre_deny`); 开 = shell 行为不变。
        if let Some(shell_config) = config.shell {
            instrument.constructed(ModuleFamily::Shell);
            let provider = ShellModule::new(shell_config.with_file_write(config.file_write));
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        if let Some(fetch_config) = config.fetch {
            instrument.constructed(ModuleFamily::Fetch);
            let provider = FetchModule::new(fetch_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;

            // 回合层提示注入告警面 (2026-11-05): 装在抓取面旁边 ⇒ ⇒ 抓取面一开,
            // 它的输出侧标记就**一定**有人读 (恒不阻断, 显式化不静默)。
            // ⚠️ 构造条件 = fetch 工具开着 (fetch 关 = 根本没有这个输出面)。
            // 装点、告警措辞与告警账的说明见 `prompt_injection_guard::install`。
            prompt_injection_report =
                Some(prompt_injection_guard::install(&mut modules, instrument));
        }

        // 内容流接入 (`tool.content_feed`, J1, opt-in): 订阅源管理 + 受控拉取 +
        // 结构化摘要 + 入记忆流 (L2 封套披露)。治理档位在治理链
        // (`ContentFeedApprovalHook`): RequireApproval 级 (拉取与入库都经审批档)。
        if let Some(feed_config) = config.content_feed {
            instrument.constructed(ModuleFamily::ContentFeed);
            let provider = ContentFeedModule::new(feed_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 双引擎主动关怀 (`tool.care_schedule`, V3-a, opt-in): 关怀规则面
        // (增删列, 四类触发器词表)。治理档位在治理链 (`CareScheduleApprovalHook`):
        // RequireApproval 级 —— 改规则 = 写文件 (write_intent 同族), 每次调用都停
        // 人工审批。W3.6: 关怀数据目录同时作为生命周期账本持久化目录 (缺省 = 纯内存)。
        let bio_lifecycle_store_dir = config.care_schedule.clone();
        if let Some(care_dir) = config.care_schedule {
            instrument.constructed(ModuleFamily::CareSchedule);
            let provider = CareScheduleModule::new(care_dir);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 备份导出 (`tool.backup_export`, G5 备份恢复, opt-in): 全量打包 +
        // manifest.json 逐文件摘要 (损坏可检出), 凭据不随行。治理档位在授权层
        // 同源接线 (RequireApproval 级: 每次导出都停人工审批)。
        if let Some(backup_config) = config.backup {
            instrument.constructed(ModuleFamily::BackupExport);
            let provider = BackupExportModule::new(backup_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 通用嗓发声 (`tool.voice_speak`, V3-b, opt-in): 文本 → 系统语音引擎
        // 合成即播 + 语句账本 (`spoken.jsonl`) + 安静纪律 (勿扰窗口, 与
        // care_rules 同源)。治理 = 发声面档 (注册即闸, grant 即发声免逐次
        // 人批, 与 shell 的执行档分立)。合成执行面 = 系统引擎桥 (可注入)。
        if let Some(voice_config) = config.voice_speak {
            instrument.constructed(ModuleFamily::VoiceSpeak);
            let provider = VoiceSpeakModule::new(
                voice_config,
                apeireth_tools_canonical::system_synth_engine(),
            );
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 任务簿 (`tool.task_board`, V6-a 任务编排持久队列, opt-in): 增/列/改派/
        // 完成 over 持久任务队列 (与会话账本同库的任务表; 崩溃恢复/幂等唤醒/环路
        // 熔断/结算接账在 orchestration 队列核)。治理档位在治理链
        // (`TaskBoardApprovalHook`): 写面 = write_intent 同族 (改任务簿 = 写文件)
        // 每次人批, `task_list` 只读零审批; **无 failed/retry 工具面**。
        if let Some(queue) = config.task_board.clone() {
            instrument.constructed(ModuleFamily::TaskBoard);
            let provider = TaskBoardModule::new(queue);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 性格周报 (`tool.trait_report`, 裁定 ⑤): 近 7 天四杆移动量 / 移动原因
        // top / 锁定态, 数据 = 成长计既有账 (不新造账)。**只读档零审批**
        // (与 `tool.self_status` 同档); 注册即闸 (成长账落位注入才注册), 名册
        // 按实际注册条件取值。自述性格节与工具面同账 (trait_status_probe)。
        if let Some(paths) = &config.trait_report {
            instrument.constructed(ModuleFamily::TraitReport);
            let provider = TraitReportModule::new(trait_growth_probe(paths.clone()));
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 暴露面体检 (`tool.exposure_audit`, V5-b 安全守望, 默认关): 本地只读
        // 自查 (只读零写, 报告是唯一产出), 与 CLI `apeireth exposure report`
        // 同一内核两个出口。治理 = 只读档 (注册即闸, grant 即用零审批)。
        if let Some(audit_config) = config.exposure_audit {
            instrument.constructed(ModuleFamily::ExposureAudit);
            let provider = ExposureAuditModule::new(audit_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 自主度第一小步 · 边角料提案面 (`tool.proposal_draft`, 默认关): 它提案,
        // 人点头 —— 提案只入提案账 (`proposals.jsonl` append-only), 不落任何真面;
        // 通过 = 账面标记 accepted, 提案不直接执行 (执行仍走既有工具面的正常审批
        // 链)。治理 = 低风险档 (读面 + append 账本, 注册即闸, grant 即用零审批)。
        if let Some(proposal_config) = &config.proposals {
            let provider = ProposalModule::new(proposal_config.clone());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 创作工具面 W1 · 图像操作 (`tool.image_ops`, 默认关): 闭合操作词表管道 +
        // 工作区限域读写 + 敏感面拒 + 炸弹图上限 + 格式白名单 + 原子写。注册条件 =
        // 开关 + `file_write` 唯一写总闸 + 工作区根 + 共享读前观测门禁; 输出写
        // 归档文件写入族。计数器句柄留给自述创作节 (同源同账)。
        let mut image_ops_activity = None;
        if config.image_ops && config.file_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::ImageOps);
                let provider = ImageOpsModule::new(root.clone(), Arc::clone(&observed_gate));
                image_ops_activity = Some(provider.activity());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 创作工具面 W2 · 文档操作 (`tool.doc_ops`, 默认关): 受控读写族 (PDF/DOCX 读
        // + PDF 基础写批量管道) + 工作区限域 + 敏感面拒 + 大小/页数上限 + 创建类
        // 原子写 (只创建不覆盖)。注册条件 = 开关 + 配置落位; **写面双重约束** =
        // `file_write` 唯一写总闸 + 审批档每次人批。
        let mut doc_ops_counters = None;
        if let Some(doc_ops_config) = &config.doc_ops {
            instrument.constructed(ModuleFamily::DocOps);
            let provider =
                DocOpsModule::new(doc_ops_config.clone().with_write_gate(config.file_write));
            doc_ops_counters = Some(provider.counters());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 创作工具面 K3 · 表格操作 (`tool.sheet_ops`, 默认关): 受控读写族 (CSV 读 +
        // 筛排聚合 + 计算列 + 导出) + 工作区限域 + 敏感面拒 + 行列/字节上限 (防炸弹表)
        // + 创建类原子写 + 数值精度如实 (f64 不静默取整, NaN/inf 零产出)。注册条件
        // = 开关 + 配置落位; **写面双重约束** = `file_write` 唯一写总闸 (关 = 拒写
        // 不拒读) + 审批档每次人批。
        let mut sheet_ops_counters = None;
        if let Some(sheet_ops_config) = &config.sheet_ops {
            instrument.constructed(ModuleFamily::SheetOps);
            let provider =
                SheetOpsModule::new(sheet_ops_config.clone().with_write_gate(config.file_write));
            sheet_ops_counters = Some(provider.counters());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // K 轴 L1 在场感知 (`tool.device_sense` + `tool.weather`, 默认关): 白名单制
        // 默认零设备 (devices.json StoredDoc 拒开) + mock 适配器 (真实协议插槽留位
        // 不接) + 摄像头类敏感读面 (显式授权 + 审计) + 天气 mock 档如实标注数据源
        // + 提醒信号只交信号。注册条件 = 配置注入 (默认关 fail-closed)。
        let mut device_sense_activity = None;
        if let Some(device_config) = &config.device_sense {
            instrument.constructed(ModuleFamily::DeviceSense);
            let provider = super::tool_modules::DeviceSenseModule::new(device_config.clone());
            device_sense_activity = Some(provider.activity());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }
        if let Some(weather_config) = &config.weather {
            instrument.constructed(ModuleFamily::Weather);
            let provider = super::tool_modules::WeatherModule::new(weather_config.clone());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // K 轴 L3/L4 受控执行骨架 (`tool.device_control`, 默认关): 三拍执行
        // (plan/preview 零副作用 → execute 真执行 + 状态账 append-only) + 三档
        // 分级审批 (reversible 可配自动档 / irreversible 永逐次人批 / money 永逐次
        // 人批 + IM 卡) + 设备白名单 (只读消费 L1 `devices.json` 同一本设备表)。
        // 注册条件 = 配置注入 (双闸, 默认关 fail-closed)。
        let mut device_control_runner = None;
        if let Some(device_control_config) = &config.device_control {
            instrument.constructed(ModuleFamily::DeviceControl);
            let provider =
                super::tool_modules::DeviceControlModule::new(device_control_config.clone());
            device_control_runner = Some(Arc::clone(provider.runner()));
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // K 轴 L5 看家骨架 (`tool.guard_watch`, 默认关): 守护规则面 (阈值/时序
        // 条件, StoredDoc 拒开) + 巡检判定 + 告警账。**只看不动**: 告警零设备动作
        // (动手仍走 `tool.device_control` 三拍人批)。注册条件 = 配置注入 (双闸
        // `APEIRETH_ENABLE_DEVICE_SENSE` + `APEIRETH_ENABLE_GUARD_WATCH`,
        // 默认关 fail-closed)。
        if let Some(guard_watch_config) = &config.guard_watch {
            instrument.constructed(ModuleFamily::GuardWatch);
            let provider = super::tool_modules::GuardWatchModule::new(guard_watch_config.clone());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // W3.3 · 游戏练习场 (`tool.play_*`, 默认关): 装配块在 `install_bio_play`
        // (开关值 = config 的 `bio_play` 总闸, 且要求工作区根注入), 返回共享账本
        // 句柄供自述 play probe 只读接入。
        let bio_play_state = bio_production_wiring::install_bio_play(
            &mut capabilities,
            &clock,
            backends.workspace_root.as_deref(),
            instrument,
            config.bio_play,
        );

        // W3.7 · 仪式完成面 (`tool.ritual_done` / `tool.ritual_skip`, 默认关):
        // 复用族总闸 + 关怀数据目录注入; 双工具写同一本 `<care_dir>/ritual_state.json`
        // (与 care 心跳同一目录)。返回的账句柄在自述槽武装后填入 SG-22 观测槽。
        let bio_ritual_store = bio_production_wiring::install_bio_ritual_tools(
            &mut capabilities,
            &clock,
            bio_lifecycle_store_dir.as_deref(),
            config.bio_care_candidates,
        );

        // 自省通道 (与既有 5 内置工具同列): `tool.self_status` 结构化自述面。
        // 只读档、默认可用、零审批; data 系探测口缺位时对应字段显式 null +
        // 原因, 注册与其余字段不受影响。
        if config.self_status {
            // 名册**现投影**口: 外部工具的登记件数在启动期接线之后才有数 ⇒ 每次
            // 自述现投影（纯函数 + 晚绑定真值; 函数零副作用）。
            let roster_config = roster_config.clone();
            let roster_extras = backends.self_status_extras.clone();
            let roster_workspace_present = backends.workspace_root.is_some();
            let roster_bag = mcp_tool_bag.clone();
            let mut source = ProductionSelfStatusSource::new(
                Vec::new(),
                self_tuning_effective,
                backends.workspace_root.clone(),
            )
            .with_roster_projection(Arc::new(move || {
                let mut effective = roster_config.clone();
                effective.mcp_registered_tools = roster_bag
                    .as_ref()
                    .map_or(0, |bag| bag.capabilities().len());
                roster_from_config(
                    &effective,
                    roster_workspace_present,
                    self_tuning_effective,
                    retrieval_tuning_effective,
                    typed_recall_effective,
                    &roster_extras,
                )
            }));
            if let Some(probe) = &backends.self_status_ledger {
                source = source.with_ledger_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_accountability {
                source = crate::accountability_probe::with_accountability_probe(
                    source,
                    Arc::clone(probe),
                );
            }
            if let Some(probe) = &backends.self_status_credentials {
                source = source.with_credentials_probe(Arc::clone(probe));
            }
            if let Some(budget) = &backends.self_status_budget {
                source = source.with_budget(budget.clone());
            }
            if let Some(probe) = &backends.self_status_quota {
                source = source.with_quota_probe(Arc::clone(probe));
            }
            if let Some(profile) = &backends.self_status_model_profile {
                source = source.with_model_profile(profile.clone());
            }
            if let Some(probe) = &backends.self_status_cost_routing {
                source = source.with_cost_routing_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_domain {
                source = source.with_domain_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_content_feed {
                source = source.with_content_feed_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_care {
                source = source.with_care_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_backup {
                source = source.with_backup_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_voice {
                source = source.with_voice_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_persona {
                source = source.with_persona_probe(Arc::clone(probe));
            }
            if let Some(queue) = &config.task_board {
                source = source.with_task_board_probe(task_board_status_probe(Arc::clone(queue)));
            }
            // 自主度第一小步: 自述提案节与 `tool.proposal_draft` 工具面同一本
            // 提案账 (同账同源; 缺落位 = 显式 null + 原因)。
            if let Some(proposal_config) = &config.proposals {
                source = source
                    .with_proposal_probe(proposal_status_probe(proposal_config.data_dir.clone()));
            }
            // 性格节真值 (裁定 ④⑤): 锁定态 + 最近移动与 `tool.trait_report`
            // 同一成长计既有账 (同账同源, 缺落位 = 显式 null + 原因)。
            if let Some(paths) = &config.trait_report {
                source = source.with_trait_growth_probe(trait_status_probe(paths.clone()));
            }
            // 创作工具面 W1: 自述创作节 (最近操作计数) 与 `tool.image_ops` 工具面
            // 同一计数器 (同源同账); 面没注册 = 创作节显式 null + 原因。
            if let Some(activity) = &image_ops_activity {
                source = source.with_image_ops_probe(image_ops_status_probe(Arc::clone(activity)));
            }
            // 创作工具面 W2: 自述创作节 (文档操作计数) 与 `tool.doc_ops` 工具面
            // 同一计数器 (同源同账); 面没注册 = 创作节显式 null + 原因。
            if let Some(counters) = &doc_ops_counters {
                source = source.with_doc_ops_probe(doc_ops_status_probe(Arc::clone(counters)));
            }
            // 创作工具面 K3: 自述创作节与 `tool.sheet_ops` 工具面同一计数器 (同源同账)。
            if let Some(counters) = &sheet_ops_counters {
                source = source.with_sheet_ops_probe(sheet_ops_status_probe(Arc::clone(counters)));
            }
            // W3.2/W3.5/W3.6: `capabilities.bio_mechanisms` 只读聚合面 (装配块已搬
            // 至 `bio_production_wiring`; 迟绑定槽由组合根在 build 后填句柄)。
            source = bio_production_wiring::install_bio_self_status(
                source,
                config.task_board.as_ref(),
                bio_play_state.as_ref(),
                &bio_probe_slots,
                nervous_switchboard.switch_enabled(BioSwitch::SelfStatus),
            );
            // K 轴 L1: 自述在场节 (设备数/白名单数/最近感知) 与 `tool.device_sense`
            // 工具面同一本设备表同一进程内感知计数 (同源同账; 面没注册 = 在场节
            // 显式 null + 原因)。
            if let (Some(device_config), Some(activity)) =
                (&config.device_sense, &device_sense_activity)
            {
                source = source.with_device_presence_probe(
                    super::self_status_source::device_presence_probe(
                        device_config.data_dir.clone(),
                        Arc::clone(activity),
                    ),
                );
            }
            // K 轴 L3/L4 受控执行: 自述设备受控执行节 (控制计数/最近动作真值)
            // 与 `tool.device_control` 工具面同一执行核心同一状态账 (同源同账;
            // 面没注册 = 该节显式 null + 原因, 不编数)。
            if let Some(runner) = &device_control_runner {
                source = source.with_device_control_probe(
                    super::self_status_source::device_control_status_probe(Arc::clone(runner)),
                );
            }
            instrument.constructed(ModuleFamily::SelfStatus);
            // R1b-4a：自述帧 `capabilities.nervous_system.bus` 报总线观测面
            // （装配根唯一 NervousBus；本帧未绑 = null，不编造计数）。
            let provider =
                SelfStatusModule::new_with_bus(Arc::new(source), nervous_bus.as_ref().clone());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // Wave 4 观测补格: 自述槽此时已按开关武装 → 把仪式账句柄填入 SG-22 观测槽
        //（开关关 / 缺目录 = `None` = 槽保持空；fill 未知武装时 no-op）。
        bio_probe_slots.fill_ritual_store(bio_ritual_store.as_ref());

        // External tool bridge (MCP): the slot is opt-in, and enabling it loads
        // the server list at assembly time (env primary, data-directory stored
        // document secondary) with fail-closed semantics. Connections are
        // established asynchronously after build; the module bag is the
        // registration target for the dynamic tools discovery produces. The
        // server list was loaded above (before the roster projection) — a
        // server list with no enabled entry assembles an inert bridge (no
        // external tools), and the roster reports that honestly.
        //
        // 合并说明（2026-11-06）: 两侧的外部工具面在此**合一为一条接线**:
        //   * 对方线的**资源工厂形态**（`McpDefinition` + 资源工厂模块）: 运行时
        //     初始化时实例化桥, 外部工具经插件/资源工厂通道进运行时目录。
        //   * 本线的**登记账袋**（`McpModule`）: 早建的袋 ⇒ 自述面可晚绑定
        //     「实际登记进运行时能力表的外部工具件数」。
        // 两侧共用**同一个桥**（资源工厂在初始化时交出的活桥, 见
        // [`Self::mcp_bridge`]）⇒ 不重复连进程、不重复构造桥对象。
        let mut mcp_definition = None;
        let mut mcp_module = None;
        if let (Some(server_config), Some(module)) = (mcp_server_config, mcp_tool_bag.clone()) {
            instrument.constructed(ModuleFamily::Mcp);
            let options = McpBridgeOptions::from_env();
            let definition =
                apeireth_tools_canonical::mcp_bridge::McpDefinition::new(server_config, options)
                    .map_err(|error| {
                        RuntimeError::misconfigured(format!(
                            "mcp definition refused to assemble: {error}"
                        ))
                    })?;
            let definition = Arc::new(definition);
            tool_modules.push(apeireth_plugin::resource_factory::module_from_definition(
                definition.clone(),
            ));
            mcp_definition = Some(definition);
            // 登记账袋 (兼容视图): 保留本线的**早建袋**（不是另造一只）⇒ 自述面
            // 晚绑定时读到的是真登记件数。
            mcp_module = Some(module);
        }

        // 安全/逆向侦察 (`tool.recon.*`, 9 件, 默认关): 注册即闸 —— 开关
        // `APEIRETH_ENABLE_RECON=1` 才构造, 且必须有工作区根 (文件类工具的落点;
        // 缺省 = 不注册, 不静默降级为「只装网络三件」)。网络目标由 scope 把关,
        // 空 scope 在工具内即拒绝 (fail-closed)。
        if let Some(recon_config) = &config.recon {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::Recon);
                let provider = ReconModule::new(root.clone(), Arc::clone(&recon_config.scope));
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 外部适配器 (`ext.*`, 10 件, 默认关): 注册即闸 —— 开关
        // `APEIRETH_ENABLE_EXTERNAL=1` 才构造。配置自带工作区根与 scope
        // (文件类限域 + 网络目标授权), 启动期不探测系统二进制。
        if let Some(external_config) = &config.external {
            instrument.constructed(ModuleFamily::ExternalTools);
            let provider = ProcessModule::new(external_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // SG-25 工具生态健康: 静态能力表装配完毕 → 把剖面填入迟绑定只读槽
        // (开关关 = fill no-op, 槽保持空、probe 未注册, 自述帧逐字节不变)。
        // 快照 = 装配期的静态注册面; 运行期动态发现的外部工具不在其中 (如实缺口)。
        bio_probe_slots.fill_ecosystem_profiles(&capabilities);

        // Unified Memory 2.0 coordinator wiring
        let mut shared_coordinator: Option<Arc<MemoryCoordinator>> = None;
        if config.memory_recall || config.memory_writeback {
            instrument.constructed(ModuleFamily::MemoryCoordinator);
            let memory = required(backends.memory.clone(), "memory", "memory")?;
            let governance = required(
                backends.memory_governance.clone(),
                "memory_governance",
                "memory_governance",
            )?;
            let mut coordinator =
                MemoryCoordinator::new(Arc::clone(&memory), Arc::clone(&governance));
            if let Some(scoped) = &backends.scoped_memory {
                coordinator = coordinator.with_scoped_backend(Arc::clone(scoped));
            }
            if let Some(embed) = &backends.embedding_provider {
                coordinator = coordinator.with_embedding_provider(Arc::clone(embed));
            }
            if config.memory_injection {
                coordinator = coordinator.with_memory_injection_format();
            }
            // 记忆线（2026-11-03）：接上**夜间整编驱动** ⇒
            // 「只整新料 + 同一窗口只整一次」⇒ ⇒ 老素材不再被反复重整编。
            // ⚠️ **水位跨进程持久化**（文件）⇒ ⇒ 进程重启后**不会**把老料重整一遍。
            // ⚠️ **窗口 = 0** ⇒ **不设时间闸**（时间节奏仍由既有的
            // `APEIRETH_TUNE_CONSOLIDATION_CADENCE` 旋钮管）⇒ ⇒ 只剩「只整新料」这一条语义。
            // ⚠️ **装不上驱动 ⇒ 走旧路径**（`run_consolidation_at` 的降级分支）⇒ ⇒ 不阻断启动。
            let watermark_path = default_data_root()
                .join("memory")
                .join("consolidation-watermark.json");
            if std::fs::create_dir_all(
                watermark_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new(".")),
            )
            .is_ok()
            {
                coordinator = coordinator.with_consolidation_trigger(ConsolidationTrigger::new(
                    ConsolidationPolicy::new(0).with_min_episodes(1),
                    Arc::new(FileConsolidationWatermark::new(watermark_path)),
                ));
            }
            if let Some(pref) = &backends.preferences {
                coordinator = coordinator.with_preferences(Arc::clone(pref));
            }
            if let (Some(graph), Some(associations)) = (&backends.graph, &backends.associations) {
                coordinator =
                    coordinator.with_experience(Arc::clone(graph), Arc::clone(associations));
            }
            if let Some(history) = &backends.access_history {
                coordinator = coordinator.with_activation_source(Arc::new(
                    AccessHistoryActivationSource::new(Arc::clone(history), 0.5, 0.0),
                ));
            }
            // 跨日记检索接线 (只读): 日记面在场 ⇒ 命中事实可附关联日记片段.
            // 缺席 = 检索路径不碰日记 ⇒ 逐字节零行为变化.
            if let Some(diary) = &backends.diary {
                coordinator = coordinator.with_diary_store(Arc::clone(diary));
            }
            // 用户知识库接线 (只读): 语料根 = <data>/wiki。
            // 目录不存在 ⇒ 检索路径整段跳过 ⇒ 逐字节零行为变化。
            coordinator =
                coordinator.with_wiki_root(crate::paths::default_data_root().join("wiki"));
            if let (Some(source), Some(identity)) =
                (&backends.typed_recall, &backends.typed_recall_identity)
            {
                coordinator = coordinator.with_typed_recall(Arc::clone(source), identity.clone());
            }
            shared_coordinator = Some(Arc::new(coordinator));
        }

        // Register cognitive modules
        if config.memory_recall {
            let memory = required(backends.memory.clone(), "memory_recall", "memory")?;
            instrument.constructed(ModuleFamily::MemoryRecall);
            let mut module = MemoryRecallModule::new(memory);
            if let Some(coord) = &shared_coordinator {
                module = module.with_coordinator(Arc::clone(coord));
            }
            if let Some(wire) = &backends.self_tuning {
                module = module.with_self_tuning(Arc::clone(wire));
            }
            // V4-c 检索维度自进化: 注入回执面挂记忆召回模块 (默认无 = 调权关)。
            if let Some(wire) = &backends.retrieval_tuning {
                module = module.with_retrieval_tuning(Arc::clone(wire));
            }
            if let (Some(wiki), Some(graph), Some(associations)) =
                (&backends.wiki, &backends.graph, &backends.associations)
            {
                module = module.with_experience(
                    Arc::clone(wiki),
                    Arc::clone(graph),
                    Arc::clone(associations),
                );
            }
            if let Some(history) = &backends.access_history {
                let history: Arc<dyn MemoryRecallAccessStore> = history.clone();
                module = module.with_access_store(history, Arc::clone(&clock));
            }
            if let Some(policy) = &config.proactive_recall {
                module = module.with_proactive_recall(policy.clone());
            }
            if config.morphology_recall {
                module = module.with_morphology_recall();
            }
            if config.community_triage {
                if let Some(graph) = &backends.graph {
                    module = module.with_community_triage(Arc::clone(graph));
                }
            }
            modules.push(Arc::new(module.with_telemetry(Arc::clone(&telemetry))));
        }

        if config.preference_recall {
            instrument.constructed(ModuleFamily::PreferenceRecall);
            modules.push(Arc::new(
                PreferenceRecallModule::new(required(
                    backends.preferences.clone(),
                    "preference_recall",
                    "preferences",
                )?)
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        // Learning sits beside recall: recall overlays at TurnStart, learning
        // writes at AfterTurn for future turns.
        if config.preference_learning {
            instrument.constructed(ModuleFamily::PreferenceLearning);
            modules.push(Arc::new(
                PreferenceLearningModule::new(required(
                    backends.preferences.clone(),
                    "preference_learning",
                    "preferences",
                )?)
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.judge.enabled {
            instrument.constructed(ModuleFamily::Judge);
            modules.push(Arc::new(
                JudgeModule::new(config.judge, Arc::clone(&observations))
                    .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.self_assessment {
            instrument.constructed(ModuleFamily::SelfAssessment);
            modules.push(Arc::new(
                SelfAssessmentModule::new(
                    required(
                        backends.self_assessments.clone(),
                        "self_assessment",
                        "self_assessments",
                    )?,
                    Arc::clone(&clock),
                    Arc::clone(&observations),
                )
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.council {
            instrument.constructed(ModuleFamily::Council);
            // D-2：council 决议的票落盘（append-only）⇒ 守夜人的「立场平衡复盘」段
            // 不再永远空转。路径与 CLI 侧读账处**同一份**（`default_data_root()` 与
            // `default_panel_data_dir()` 同实现同路径：`$APEIRETH_DATA_DIR` 或 `~/.apeireth`）。
            let ballot_log = Arc::new(super::council_ballot_log::CouncilBallotLog::open(
                super::council_ballot_log::default_council_ballot_log_path(&default_data_root()),
            ));
            modules.push(Arc::new(
                CouncilModule::new(
                    required(backends.council, "council", "council")?,
                    Arc::clone(&clock),
                )
                .with_ballot_log(ballot_log)
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.organs {
            instrument.constructed(ModuleFamily::Organs);
            let module = Arc::new(OrganModule::new(Arc::clone(&clock)));
            organ_module = Some(Arc::clone(&module));
            modules.push(module as Arc<dyn Module>);
        }

        if config.reflexion {
            instrument.constructed(ModuleFamily::Reflexion);
            modules.push(Arc::new(
                super::cognitive::ReflexionModule::new(
                    required(backends.reflexion_store, "reflexion", "reflexion_store")?,
                    Arc::clone(&observations),
                    Arc::clone(&clock),
                )
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.absorption_insight {
            instrument.constructed(ModuleFamily::AbsorptionInsight);
            modules.push(Arc::new(super::cognitive::AbsorptionInsightModule::new()));
        }

        if config.partner_bond {
            instrument.constructed(ModuleFamily::PartnerBond);
            let store = required(backends.partner_store, "partner_bond", "partner_store")?;
            let subject_id = backends
                .typed_recall_identity
                .as_ref()
                .map(|identity| identity.subject_id.clone())
                .unwrap_or_else(|| "local-user".to_string());
            let mut bond_module =
                super::cognitive::PartnerBondModule::new(store, subject_id, Arc::clone(&clock))
                    .with_telemetry(Arc::clone(&telemetry));
            // 关系里程碑（初见 / 阶段跃迁）：只在注入了存储时挂 —— 缺存储 =
            // 不记 = 零行为变化。记录面本身带幂等闸，装配层不必自己判重。
            if let Some(milestones) = backends.milestone_store {
                bond_module = bond_module.with_milestones(Arc::new(
                    apeireth_memory::milestone::MilestoneRecorder::new(milestones),
                ));
            }
            modules.push(Arc::new(bond_module));
        }

        // V4-a 恒定小注入 (人格记忆层回取面 ①) 的 TurnStart overlay 装配点:
        // 自我叙事摘要 ≤ 恒预算 (`CONSTANT_INJECTION_BUDGET_CHARS`) 进提示词。
        // **缺人格层 = 不注册本模块 = 零注入零变化**; 人格层无自我叙事沉淀时
        // 同样零注入 (未沉淀就是未沉淀, 不编)。
        if let Some(persona) = &backends.persona_layer {
            instrument.constructed(ModuleFamily::PersonaConstantInjection);
            modules.push(Arc::new(
                super::cognitive::PersonaConstantInjectionModule::new(Arc::clone(persona))
                    .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        // W3.2 / W3.3 / W3.5 / W3.6: 修复指令段 + 调质 tone + 生命周期装配块 (全默认关,
        // 关 = 不构造不注册), 句柄回传装配根；生命周期账本持久化目录 = 关怀数据目录
        // （缺省 None = 纯内存，不建文件）。
        let bio_wiring = bio_production_wiring::install_bio_modules(
            &mut modules,
            &clock,
            &capabilities,
            bio_lifecycle_store_dir.as_deref(),
            config.context_budget_chars,
            &nervous_switchboard,
        );
        // R1b-2a: 调质事件源收敛到总线订阅——开关开才有订阅者, 拿到总线后注册
        // (关 = `None` = 不注册 = 总线零订阅者 = 零变化)。
        if let Some(subscriber) = &bio_wiring.tone_subscriber {
            subscriber.subscribe(&nervous_bus);
        }
        // Wave 4 (SG-20): 社会修复观测源订阅人工拒绝信号（源 None = 开关关 = 零订阅）。
        if let Some(source) = bio_wiring.social_repair_source.as_ref() {
            source.subscribe(&nervous_bus);
        }
        // R1b-2b: 生命周期账本在模块安装后才存在 → 迟绑定槽填同一 Arc
        //（自述开关关时 fill no-op；槽空 = 单格 null + 未接线真因）。
        bio_probe_slots.fill_lifecycle_state(bio_wiring.lifecycle_state.as_ref());
        // Wave 4 观测补格: 调质句柄与昼夜节律配置在模块安装后填入迟绑定槽
        //（自述开关关时 fill no-op；槽空 = 单格 null + 未接线真因）。
        bio_probe_slots.fill_tone_state(bio_wiring.tone_state.as_ref());
        bio_probe_slots.fill_circadian_config(CircadianProbeConfig::new(
            nervous_switchboard.circadian_tz_offset_min(),
            nervous_switchboard.switch_enabled(BioSwitch::Circadian),
        ));
        // Wave 4 homeostasis 行为接线: 装配装饰器的观测源（开关关时 fill no-op）。
        bio_probe_slots.fill_homeostasis(bio_wiring.homeostasis_source.as_ref());
        bio_probe_slots.fill_social_repair(bio_wiring.social_repair_source.as_ref());

        if config.memory_writeback {
            instrument.constructed(ModuleFamily::MemoryWriteback);
            let mut module = MemoryWritebackModule::new(
                required(backends.memory, "memory_writeback", "memory")?,
                clock,
            );
            if let Some(coord) = &shared_coordinator {
                module = module.with_coordinator(Arc::clone(coord));
            }
            if let Some(materializer) = &backends.memory_materializer {
                module = module.with_materializer(Arc::clone(materializer));
            } else if let Some(extractor) = &backends.memory_extractor {
                module = module.with_extractor(Arc::clone(extractor));
            }
            if let Some(sink) = &backends.typed_sink {
                module = module.with_typed_sink(Arc::clone(sink));
            }
            if config.consolidation {
                module = module.with_consolidation();
            }
            // V4-a 人格提取步: 开关 + 人格层落库面齐备才接 (默认未接线);
            // 运行触发点 = 整编触发点 (with_consolidation), 同一条人格管线。
            if config.persona_consolidation {
                if let Some(persona) = &backends.persona_layer {
                    module = module.with_persona_layer(Arc::clone(persona));
                }
            }
            // V4-b 散列增量门: 开关 + 散列门落库面齐备才接 (默认未接线);
            // 运行触发点 = 整编触发点 (with_consolidation), 同一条整编管线。
            if config.consolidation_hash_gate {
                if let Some(gate) = &backends.ingest_gate {
                    module = module.with_ingest_gate(Arc::clone(gate));
                }
            }
            if let (Some(wiki), Some(graph), Some(associations)) =
                (&backends.wiki, &backends.graph, &backends.associations)
            {
                module = module.with_experience(
                    Arc::clone(wiki),
                    Arc::clone(graph),
                    Arc::clone(associations),
                );
            }
            modules.push(Arc::new(module.with_telemetry(Arc::clone(&telemetry))));
        }

        // W3.6: 器官模块装配后把审议门账读取口交给自述槽 (未装配 = no-op).
        bio_probe_slots.fill_from_organ_module(organ_module.as_ref());

        let mut seen = std::collections::BTreeSet::new();
        for module in &modules {
            let id = module.manifest().id.clone();
            if !seen.insert(id.clone()) {
                return Err(RuntimeError::misconfigured(format!(
                    "duplicate cognitive module id {id:?}"
                )));
            }
        }

        let owned: std::collections::BTreeSet<_> = tool_modules
            .iter()
            .flat_map(|module| module.tools())
            .map(|tool| tool.id().clone())
            .collect();
        let remaining = capabilities
            .iter()
            .filter(|tool| !owned.contains(tool.id()))
            .cloned()
            .collect::<Vec<_>>();
        if !remaining.is_empty() {
            tool_modules.push(Arc::new(ToolModule::new(
                apeireth_core::kernel::PluginId::new("module.tool.host_extensions")
                    .expect("static id"),
                remaining,
            )?));
        }
        Ok(Self {
            modules,
            tool_modules,
            capabilities,
            telemetry,
            mcp_definition,
            mcp_module,
            organ_module,
            bio_tone_state: bio_wiring.tone_state,
            bio_lifecycle_state: bio_wiring.lifecycle_state,
            bio_probe_slots,
            nervous_bus,
            nervous_switchboard,
            prompt_injection_report,
        })
    }

    /// Wave 4 / ADR-0009 R1b-1a: 神经信号总线句柄 (适配器只 publish, 不直写
    /// `ToneState`; 与注册的桥共享同一 `Arc`)。
    pub fn nervous_bus(&self) -> Arc<NervousBus> {
        Arc::clone(&self.nervous_bus)
    }

    /// 本次装配生效的开关快照 (`NervousSwitchboard::from_env` 一次解析)。
    pub fn nervous_switchboard(&self) -> &NervousSwitchboard {
        &self.nervous_switchboard
    }

    /// Wave 4 / ADR-0009 R1b-2c: 审批社交信号源句柄。
    ///
    /// 只有调质开关开时才交付 (`Some`); 开关关 = `None` = 审批路径不发布、
    /// 总线零计数, 与未接线逐字节一致。句柄与注册的调质订阅者共享同一条
    /// [`NervousBus`] (同一 `Arc`)。
    #[must_use]
    pub fn bio_approval_social_source(&self) -> Option<Arc<BioApprovalSocialSource>> {
        self.nervous_switchboard
            .switch_enabled(BioSwitch::Tone)
            .then(|| Arc::new(BioApprovalSocialSource::new((*self.nervous_bus).clone())))
    }

    /// Attach the context-window projector using its default policy.
    #[must_use]
    pub fn register_context_projection(
        &self,
        builder: super::runtime::RuntimeBuilder,
    ) -> super::runtime::RuntimeBuilder {
        with_memory_context_projection(builder, ContextWindowManager::default())
    }

    pub fn modules(&self) -> &[Arc<dyn Module>] {
        &self.modules
    }

    /// Capabilities contributed by concrete production providers.
    pub fn capabilities(&self) -> &[Arc<dyn ToolCapability>] {
        &self.capabilities
    }

    /// Consume the set into the builder's module registration calls.
    pub fn register_into(
        self,
        mut builder: super::runtime::RuntimeBuilder,
    ) -> super::runtime::RuntimeBuilder {
        for module in self.modules {
            builder = builder.with_registered_behavior(module);
        }
        for module in self.tool_modules {
            builder = builder.with_plugin(module);
        }
        builder
    }

    /// Opt-in Host mapping. Tool modules retain the same factory/lifecycle path.
    /// The Host must still grant each selected internal capability explicitly.
    pub fn register_with_profile_mappings(
        self,
        builder: RuntimeBuilder,
        mappings: Arc<crate::profile_memory::ProfileMemoryMappings>,
    ) -> RuntimeBuilder {
        let mut builder = mappings.register_into(builder, &self.modules);
        for module in self.tool_modules {
            builder = builder.with_plugin(module);
        }
        builder
    }

    /// Stable slot ids in the exact registration order.
    pub fn ids(&self) -> Vec<String> {
        self.modules
            .iter()
            .map(|module| module.manifest().id.clone())
            .collect()
    }

    /// Shared non-sensitive hook telemetry for the registered modules.
    pub fn telemetry(&self) -> Arc<CognitiveTelemetry> {
        Arc::clone(&self.telemetry)
    }

    /// W3.3 外部事件源: 把 guard 免疫记忆共享句柄接到器官模块的审议门, 并把
    /// **同一份**句柄填进 W3.6 自述槽 (开关关 = 槽未武装, fill no-op)。传进来的
    /// 必须是 hook 正在使用的同一份句柄 (否则观测与门读的不是同一份状态);
    /// 未启用器官模块或句柄为 `None` 时不做任何事, 返回是否真的挂上审议门。
    ///
    /// **默认零行为变化**: 不调用本方法 / 传 `None` = 审议门从未读免疫记忆。
    pub fn attach_immune_memory(&self, memory: Option<SharedImmuneMemory>) -> bool {
        self.bio_probe_slots.fill_immune_memory(memory.clone());
        match (&self.organ_module, memory) {
            (Some(module), Some(memory)) => {
                module.attach_immune_memory(memory);
                true
            }
            _ => false,
        }
    }
}

fn required<T>(value: Option<Arc<T>>, slot: &str, dependency: &str) -> RuntimeResult<Arc<T>>
where
    T: ?Sized,
{
    value.ok_or_else(|| {
        RuntimeError::misconfigured(format!(
            "cognitive slot {slot:?} requires injected backend {dependency:?}"
        ))
    })
}

#[path = "production_mcp.rs"]
mod mcp;
