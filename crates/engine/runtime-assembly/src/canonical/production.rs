//! The single production module composition root.
//!
//! Adapters supply concrete backends, tools, and the optional Council. This module
//! owns ordering and slot validation, so CLI, gateway, and future
//! embedding callers cannot accidentally register a second cognitive or tool spine.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_memory::{
    AccessHistoryActivationSource, ContextWindowManager, EmbeddingProvider, MemoryCoordinator,
    MemoryExtractor, MemoryGovernanceStore, MemoryMaterializer, MemoryMaterializerPort,
    MemoryTypedMaterializationSink, ProactiveRecallPolicy, ScopedMemoryBackend,
    SqliteAccessHistoryStore, TypedMemoryRecallSource, TypedRecallIdentity,
};
use apeireth_orchestration::Council;
use apeireth_plugin::experience::{AssociationStore, KnowledgeGraphStore, WikiEntryStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_plugin::preference::PreferenceStore;
use apeireth_plugin::self_assessment::SelfAssessmentStore;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::NormalizedMessage;
use apeireth_runtime::{ContextProjectionError, ContextProjector, RuntimeBuilder};
use apeireth_tools_canonical::mcp_bridge::{McpBridgeOptions, McpServerConfig, McpToolBridge};
use apeireth_tools_canonical::{
    CapabilitySwitch, FetchConfig, MemoryLedgerStats, ObservedGate, StatusProbe, TrustedShellConfig,
};

use super::capability::CapabilityProvider;
use super::cognitive::{
    CognitiveTelemetry, CouncilModule, JudgeConfig, JudgeModule, JudgeObservations,
    MemoryRecallAccessStore, MemoryRecallModule, MemoryWritebackModule, PreferenceRecallModule,
    SelfAssessmentModule,
};
use super::error::{RuntimeError, RuntimeResult};
use super::memory_typed_sink::CanonicalMemoryTypedSink;
use super::module::Module;
use super::organ_module::OrganModule;
use super::preference_learning::PreferenceLearningModule;
use super::self_status_source::{
    roster_from_config, task_board_status_probe, ProductionSelfStatusSource,
};
use super::tool_modules::{
    ApplyPatchModule, BackupExportModule, CareScheduleModule, ContentFeedModule, EducationModule,
    FetchModule, FilesystemModule, GitWriteModule, McpModule, RepoModule, SearchModule,
    SelfStatusModule, ShellModule, SkillDistillModule, SkillSearchModule, TaskBoardModule,
    TraitReportModule, VoiceSpeakModule, WorktreeApplyModule, WorktreePlanModule,
};
use super::trait_growth::{trait_growth_probe, trait_status_probe, TraitGrowthPaths};

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

/// Feature switches for the production cognitive and tool modules.
/// default when their injected stores exist. Judge, Council, Shell and Fetch are explicitly
/// opt-in; Judge and Council side-calls stay behind the runtime invoker.
#[derive(Debug, Clone)]
pub struct ProductionModulesConfig {
    /// Register memory recall when a memory backend is supplied.
    pub memory_recall: bool,
    /// Opt-in deterministic proactive recall policy; disabled by default.
    pub proactive_recall: Option<ProactiveRecallPolicy>,
    /// Register AfterTurn memory writeback when a memory backend is supplied.
    pub memory_writeback: bool,
    /// Register preference recall when a preference store is supplied.
    pub preference_recall: bool,
    /// Register Judge-backed self-assessment when its store is supplied.
    pub self_assessment: bool,
    /// Enable the AI-evaluates-AI side-call.
    pub judge: JudgeConfig,
    /// Register the no-tool Council adapter.
    pub council: bool,
    /// Register filesystem tool module when workspace root is supplied.
    pub filesystem: bool,
    /// Register search tool module when workspace root is supplied.
    pub search: bool,
    /// Register repository inspection tool module when workspace root is supplied.
    pub repo: bool,
    /// Register shell tool module when config is supplied.
    pub shell: Option<TrustedShellConfig>,
    /// Register fetch tool module when config is supplied.
    pub fetch: Option<FetchConfig>,
    /// Register the life-integration content feed (`tool.content_feed`, J1) when
    /// config is supplied.
    ///
    /// 内容流接入: 订阅源管理 + 受控拉取 + 结构化摘要 + 入记忆流 (L2 封套
    /// 披露, 跨源内容不直进提示词)。治理档位 (RequireApproval 级: 拉取与入库
    /// 都经审批档, 改订阅 = 写文件走 write_intent 同族) 由治理链钩子承载。
    /// 默认关 (opt-in)。
    pub content_feed: Option<apeireth_tools_canonical::ContentFeedConfig>,
    /// Register the proactive-care rule scheduling tool (`tool.care_schedule`,
    /// V3-a 双引擎主动关怀) when the care rules data directory is supplied.
    ///
    /// 关怀规则面: 增删列关怀规则 (四类触发器词表), 写文件面走 write_intent
    /// 同族 (改规则 = 写文件)。治理档位 (RequireApproval 级: 每次调用都停人工
    /// 审批, 无自动放行档) 由治理链钩子承载。关怀发起不走本面: 发起 = 双引擎
    /// 判定 → 标准回合 (零新消息通道)。默认关 (opt-in)。
    pub care_schedule: Option<PathBuf>,
    /// Register the general-voice speech tool (`tool.voice_speak`, V3-b 通用嗓闭环)
    /// when the voice configuration is supplied.
    ///
    /// 发声面: 文本 → 系统语音引擎合成即播 + 语句账本 (`spoken.jsonl`) + 安静
    /// 纪律 (勿扰窗口, 与 care_rules 同源)。治理 = 发声面档 (注册即闸
    /// `APEIRETH_ENABLE_VOICE_SPEAK`, grant 即发声免逐次人批, 与 shell 的执行档
    /// 分立)。默认关 (opt-in)。
    pub voice_speak: Option<apeireth_tools_canonical::VoiceSpeakConfig>,
    /// Register the backup export tool (`tool.backup_export`, G5 备份恢复) when
    /// config is supplied.
    ///
    /// 备份导出面: 全量打包记忆/账本 (七类数据面) + manifest.json 逐文件摘要
    /// (损坏可检出); **凭据不随行** (密钥/令牌面显式排除 + 测试钉死)。恢复/校验
    /// 是 CLI 面 (`apeireth backup restore|verify`, 本地操作), 不走本面。治理
    /// 档位 (RequireApproval 级: 每次导出都停人工审批) 由授权层同源接线。
    /// 默认关 (opt-in)。
    pub backup: Option<apeireth_tools_canonical::backup::BackupExportConfig>,
    /// Register the durable task board (`tool.task_board`, V6-a 任务编排持久队列)
    /// when the task queue is supplied.
    ///
    /// 任务簿工具面: 增/列/改派/完成 over 持久任务队列 (与会话账本同库的任务表,
    /// 崩溃恢复/幂等唤醒/环路熔断/结算接账在 [`apeireth_orchestration::task_queue`])。
    /// 治理档位 (写面 = write_intent 同族: 改任务簿 = 写文件, 每次人批无自动
    /// 放行档; `task_list` 只读零审批) 由治理链
    /// ([`super::task_board_tool::TaskBoardApprovalHook`]) 唯一承载; **无
    /// failed/retry 工具面** (failed 只能人判)。默认关 (opt-in)。
    pub task_board: Option<Arc<apeireth_orchestration::task_queue::TaskQueue>>,
    /// Register the external tool bridge (MCP) module.
    ///
    /// When enabled, the server list is loaded at assembly time from
    /// `APEIRETH_MCP_SERVERS` (primary entry) or the data directory's
    /// `mcp-servers.json` (secondary entry, stored-document fail-closed
    /// semantics): a defective configuration is a boot error, never a
    /// silent fallback. Off by default.
    ///
    /// Self-report semantics (honest): the `mcp` roster row reads the
    /// registration condition — the bridge assembled **and** at least one
    /// enabled server entry. Enabled with no server configuration means no
    /// external tools, and the roster says so.
    pub mcp: bool,
    /// Data directory whose `mcp-servers.json` is the secondary config entry
    /// for the external tool bridge (see [`Self::mcp`]).
    pub mcp_data_dir: Option<PathBuf>,
    /// Register deterministic preference learning (AfterTurn, explicit
    /// evidence only). Requires the preference backend when enabled.
    pub preference_learning: bool,
    /// Register the single organ module (post-turn organ cognition).
    ///
    /// Opt-in and off by default: the organ chain includes LLM-heavy W1/W2
    /// side-calls, and this is the explicit rollout boundary for the first
    /// production organ integration.
    pub organs: bool,
    /// Render prompt overlays in the closed-world injection format
    /// (numbered evidence + anti-hallucination rules). Off by default.
    pub memory_injection: bool,
    /// Run the deterministic consolidation report after each turn and persist
    /// its extracted insights. Off by default. Hangs off the memory-writeback
    /// module (`with_consolidation`): inert when [`Self::memory_writeback`] is
    /// off, and the roster reports that registration condition.
    pub consolidation: bool,
    /// V4-a 人格提取步 (默认关): 整编触发点上从近期情节提炼人格增量 →
    /// 人格层合并 (同类合并 / 矛盾标注「人格张力」, 不静默覆盖)。
    ///
    /// 挂在记忆写入模块的整编触发点 (`with_persona_layer`): [`Self::consolidation`]
    /// 关或人格层落库面缺席时显式不跑 (0 装诚实), 名册按实际注册条件取值。
    pub persona_consolidation: bool,
    /// V4-b 散列增量门 (默认关): 整编触发点上先查摄入散列门 —— 整编素材
    /// 同源同散列 = 跳过重复整编 (省算力防重复记忆), 变内容照常重摄。
    ///
    /// 挂在记忆写入模块的整编触发点 (`with_ingest_gate`): [`Self::consolidation`]
    /// 关或散列门落库面缺席时显式不跑 (0 装诚实), 名册按实际注册条件取值。
    pub consolidation_hash_gate: bool,
    /// Register the reflexion failure-feedback module (TurnStart lessons +
    /// AfterTurn judge-failure sedimentation). Off by default.
    pub reflexion: bool,
    /// W2 §4.2 partner 羁绊 (2026-10-10, 默认关): TurnStart 关系注入 + AfterTurn 羁绊演化。
    pub partner_bond: bool,
    /// W2 §4.3 (2026-10-10, 默认关): 查询形态学自适应检索深度 (organ/morphology)。
    /// 挂在记忆召回模块 (`with_morphology_recall`): [`Self::memory_recall`] 关
    /// 时静默失效, 名册按实际注册条件取值。
    pub morphology_recall: bool,
    /// W3 (2026-10-10, 默认关): community 社区分诊接检索前置 (需 graph 后端)。
    /// 挂在记忆召回模块 (`with_community_triage`): [`Self::memory_recall`] 关
    /// 或图谱槽缺席时静默失效, 名册按实际注册条件取值。
    pub community_triage: bool,
    /// W2 §4.3 (2026-10-10, 默认关): education Dx-Check 换元检查工具注册。
    pub education: bool,
    /// W2 §4.4 (2026-10-10, 默认关): 研究吸收批认知体操 (四算法实验性洞察)。
    pub absorption_insight: bool,
    /// Register the structured self-report tool (`tool.self_status`, 自省通道).
    ///
    /// 只读档、默认可用、零审批: 模型可实测自身状态再发言。缺 data 系探测口
    /// 时对应字段显式 null + 原因, 不影响注册与其余字段。
    pub self_status: bool,
    /// Register the controlled file-write tool (`tool.apply_patch`, 第七件).
    ///
    /// 补丁式受控写文件 (创建/修改/删除显式声明), 沿用读前观测门禁。写入风险
    /// 档位默认 require-approval 级 (每次写入都要人批, 与本地审批面板 / IM 审批
    /// 卡同链); 授权层由组装根按同一开关接线。默认关 (opt-in)。
    pub file_write: bool,
    /// 「自动放行已读文件的修改」子档 (依赖 [`Self::file_write`]): 仅修改类
    /// 补丁免逐次审批; 删除/新建永不自动放行 (仍走人工审批)。默认关。
    pub file_write_auto_pass: bool,
    /// Register the graded git-write tool (`tool.git_write`, 干活能力轴 P1)。
    ///
    /// 分级词表: L1 可逆 (add/commit/branch 建·切/tag 本地) / L2 出网
    /// (push/fetch/remote) / L3 毁灭 (无工具面, 拒绝即帧)。L1 默认每次人批
    /// (与本地审批面板 / IM 审批卡同链); L2 **永远人批**。默认关 (opt-in)。
    pub git_write: bool,
    /// 「自动放行 L1」子档 (依赖 [`Self::git_write`]): 仅 L1 可逆本地写免逐次
    /// 审批; **L2 出网永不自动放行** (永远人工审批)。默认关。
    pub git_write_auto_pass_l1: bool,
    /// Register the read-only change-order tool (`tool.worktree_plan`, 干活能力轴 P2 计划拍)。
    ///
    /// 多文件大改「计划→改→验证→落」四拍之首: 任务目标 → 改单 (零落盘,
    /// 只读面); 改单是 [`Self::worktree_apply`] 的输入合同。注册条件 = 本开关
    /// + [`Self::file_write`] (唯一写总闸) + 工作区根已注入 (注册即闸)。
    /// 默认关 (opt-in)。
    pub worktree_plan: bool,
    /// Register the controlled worktree-write tool (`tool.worktree_apply`, 干活能力轴 P2)。
    ///
    /// 四拍受控工作树写 (计划→改→验证→落, 全有或全无); 与 `tool.apply_patch`
    /// 同受 [`Self::file_write`] 唯一写总闸。注册条件 = 本开关 +
    /// [`Self::file_write`] + 工作区根已注入 + 共享读前观测门禁 (`ObservedGate`,
    /// 与读工具同一张观测表 —— 未读不得覆盖/删除)。RequireApproval 级 (每次
    /// 落盘都人批, 无自动放行档)。默认关 (opt-in)。
    pub worktree_apply: bool,
    /// Register the skill distill tool (`tool.skill_distill`, 书→技能沉淀 I1)。
    ///
    /// 知识吸收写入口: 内容输入 (正文 / 文件只读通道) + 模型侧草稿 →
    /// 结构化技能卡 → 技能库 ( [`Self::skills_dir`] )。注册条件 = 本开关 +
    /// [`Self::file_write`] (唯一写总闸, 注册即闸) + 工作区根 + 技能库目录。
    /// **审批档**: 每次沉淀都人批 (无自动放行档)。默认关 (opt-in)。
    pub skill_distill: bool,
    /// Register the read-only skill search tool (`tool.skill_search`, P1 调用面; 原 I1, 审计修 D13 改名)。
    ///
    /// 检索式注入 (触发词命中 / 按需拉取) 经 L2 封套披露进上下文; 只读档
    /// 零审批。注册条件 = 本开关 + 技能库目录 (不依赖写总闸)。默认关 (opt-in)。
    pub skill_search: bool,
    /// 技能库目录 (数据目录 `skills/`, 每卡一 StoredDoc)。
    ///
    /// 缺位时两级技能工具都注册不成 (注册即闸), 名册照实报 false。
    pub skills_dir: Option<PathBuf>,
    /// Register the personality growth report (`tool.trait_report`, 裁定 ⑤)。
    ///
    /// 性格周报: 近 7 天四杆移动量 / 移动原因 top / 锁定态。**只读档零审批**
    /// (与 `tool.self_status` 同档)。数据 = 成长计既有账 (`tuning-log.jsonl`,
    /// 不新造账); 注册条件 = 成长账落位已注入 (注册即闸), 生产装配默认注入 =
    /// 默认可用。自述性格节 (锁定态 + 最近移动) 与本工具同账同源。
    pub trait_report: Option<TraitGrowthPaths>,
}

impl Default for ProductionModulesConfig {
    fn default() -> Self {
        Self {
            memory_recall: true,
            proactive_recall: None,
            memory_writeback: true,
            preference_recall: true,
            self_assessment: true,
            judge: JudgeConfig::default(),
            council: false,
            filesystem: true,
            search: true,
            repo: true,
            shell: None,
            fetch: None,
            content_feed: None,
            care_schedule: None,
            voice_speak: None,
            backup: None,
            task_board: None,
            mcp: false,
            mcp_data_dir: None,
            preference_learning: false,
            organs: false,
            memory_injection: false,
            consolidation: false,
            persona_consolidation: false,
            consolidation_hash_gate: false,
            reflexion: false,
            partner_bond: false,
            morphology_recall: false,
            community_triage: false,
            education: false,
            absorption_insight: false,
            self_status: true,
            file_write: false,
            file_write_auto_pass: false,
            git_write: false,
            git_write_auto_pass_l1: false,
            worktree_plan: false,
            worktree_apply: false,
            skill_distill: false,
            skill_search: false,
            skills_dir: None,
            trait_report: None,
        }
    }
}

/// Compatibility alias for [`ProductionModulesConfig`].
pub type CognitiveModuleConfig = ProductionModulesConfig;

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
    pub workspace_root: Option<PathBuf>,
    /// Scoped memory backend for cross-session storage queries.
    pub scoped_memory: Option<Arc<dyn ScopedMemoryBackend>>,
    /// Embedding provider for semantic memory retrieval.
    pub embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    /// Optional durable selected-context access history.
    pub access_history: Option<Arc<SqliteAccessHistoryStore>>,
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
    /// W2 §4.2: partner 羁绊存储 (InMemoryPartnerStore 或未来 sqlite 实现)。
    pub partner_store: Option<Arc<dyn apeireth_memory::partner::PartnerStore>>,
    /// 「性格养成」第一铲 (默认无 = 自学习关): 自校准接线层, 接上后
    /// MemoryRecallModule 的召回结果作为真接信号喂给引擎。
    pub self_tuning: Option<Arc<crate::canonical::self_tuning_wire::SelfTuningWire>>,
    /// 自省通道: 记忆账本计数探测口 (会话/记忆/保护/教训计数, 只回计数)。
    pub self_status_ledger: Option<StatusProbe<MemoryLedgerStats>>,
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

/// The validated, ordered module set to pass to [`RuntimeBuilder::with_module`].
pub struct ProductionModules {
    modules: Vec<Arc<dyn Module>>,
    capabilities: Vec<Arc<dyn ToolCapability>>,
    telemetry: Arc<CognitiveTelemetry>,
    mcp_bridge: Option<Arc<McpToolBridge>>,
    mcp_module: Option<Arc<McpModule>>,
}

/// Compatibility alias for [`ProductionModules`].
pub use ProductionModules as ProductionCognitiveModules;

impl ProductionModules {
    /// Build the canonical registration order.
    pub fn build(
        config: ProductionModulesConfig,
        backends: ProductionBackends,
        clock: Arc<dyn Clock>,
    ) -> RuntimeResult<Self> {
        let mut modules: Vec<Arc<dyn Module>> = Vec::new();
        let mut capabilities: Vec<Arc<dyn ToolCapability>> = Vec::new();
        let observations = Arc::new(JudgeObservations::default());
        let telemetry = Arc::new(CognitiveTelemetry::default());

        // External tool bridge (MCP) server list is loaded BEFORE the roster
        // projection: the self-report must read the same facts the
        // registration uses (已启用但无服务器配置 = 无外部工具). Fail-closed
        // semantics unchanged: a defective configuration is a boot error,
        // never a silent fallback.
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

        // 自省通道: 能力名册在任何字段移动之前投影 —— 名册即**实际用于装配的**
        // config 生效值 (读真实生效值, 不是配置文本的复述)。注入槽级的注册
        // 条件 (图谱槽 / 外部工具服务器面) 先折进生效视图, 名册不复述
        // "配置说开"而注册没成的事实。
        let mut roster_config = config.clone();
        roster_config.community_triage = config.community_triage && backends.graph.is_some();
        roster_config.mcp = mcp_server_config
            .as_ref()
            .is_some_and(|servers| servers.enabled_servers().next().is_some());
        // 自学习 / 类型化召回的生效值 = 接线事实 AND 上游槽 (自学习信号只走
        // 记忆召回模块, 类型化召回候选只走记忆协调器)。
        let self_tuning_effective = backends.self_tuning.is_some() && config.memory_recall;
        let typed_recall_effective =
            backends.typed_recall.is_some() && (config.memory_recall || config.memory_writeback);
        // V4-a 人格提取步的生效值 = 开关 AND 人格层落库面 (面缺席 = 显式不跑).
        roster_config.persona_consolidation =
            config.persona_consolidation && backends.persona_layer.is_some();
        // V4-b 散列增量门的生效值 = 开关 AND 散列门落库面 (面缺席 = 显式不跑)。
        roster_config.consolidation_hash_gate =
            config.consolidation_hash_gate && backends.ingest_gate.is_some();
        let self_status_roster = config.self_status.then(|| {
            roster_from_config(
                &roster_config,
                backends.workspace_root.is_some(),
                self_tuning_effective,
                typed_recall_effective,
                &backends.self_status_extras,
            )
        });

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
                let provider =
                    FilesystemModule::new_with_gate(root.clone(), Arc::clone(&observed_gate));
                capabilities.extend(provider.capabilities());
            }
        }

        if config.search {
            if let Some(root) = &backends.workspace_root {
                let provider = SearchModule::new(root.clone());
                capabilities.extend(provider.capabilities());
            }
        }

        if config.repo {
            if let Some(root) = &backends.workspace_root {
                let provider = RepoModule::new(root.clone());
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
                let provider = ApplyPatchModule::new(root.clone(), Arc::clone(&observed_gate));
                capabilities.extend(provider.capabilities());
            }
        }

        // 干活能力轴 P2 (「计划→改→验证→落」四拍受控工作树写, opt-in): 两级
        // 工具面注册条件 = 开关 + `file_write` 唯一写总闸 + 工作区根 (注册即闸,
        // 与 `tool.apply_patch` 同受一闸)。`worktree_plan` 是只读改单面 (零落盘,
        // 其产物是 apply 的输入合同); `worktree_apply` 走**共享**读前观测门禁
        // (与读工具同一张观测表, 未读不得覆盖/删除)。审批级: apply 每次落盘
        // 都人批 (无自动放行档, 审批卡载荷 = 改单摘要), plan 只读零审批。
        if config.worktree_plan && config.file_write {
            if let Some(root) = &backends.workspace_root {
                let provider = WorktreePlanModule::new(root.clone());
                capabilities.extend(provider.capabilities());
            }
        }

        if config.worktree_apply && config.file_write {
            if let Some(root) = &backends.workspace_root {
                let provider = WorktreeApplyModule::new(root.clone(), Arc::clone(&observed_gate));
                capabilities.extend(provider.capabilities());
            }
        }

        // git 写分级 (`tool.git_write`, 干活能力轴 P1, opt-in): 分级词表 +
        // 词法分级执行面 + 受控通道 (工作区限域/超时/输出归一)。L1 默认人批
        // (设置可开「自动放行 L1」档), L2 永远人批, L3 拒绝即帧无工具面。
        if config.git_write {
            if let Some(root) = &backends.workspace_root {
                let provider = GitWriteModule::new(root.clone());
                capabilities.extend(provider.capabilities());
            }
        }

        // 书→技能沉淀 (P 轴吸收自动化 P1, 默认关): 两级工具面。`tool.skill_distill`
        // 是写文件面 —— 注册条件 = 开关 + `file_write` 唯一写总闸 + 工作区根 +
        // 技能库目录 (注册即闸, 与 `tool.apply_patch` 同受一闸), 审批档 = 每次
        // 沉淀都人批 (无自动放行档, 审批卡载荷 = 技能卡摘要); `tool.skill_search`
        // 是只读档 (零审批), 只要技能库目录。技能库 = 数据目录 `skills/`,
        // 每卡一 StoredDoc (拒开语义)。
        if config.skill_distill && config.file_write {
            if let (Some(root), Some(skills_dir)) = (&backends.workspace_root, &config.skills_dir) {
                let provider = SkillDistillModule::new(root.clone(), skills_dir.clone());
                capabilities.extend(provider.capabilities());
            }
        }

        if config.skill_search {
            if let Some(skills_dir) = &config.skills_dir {
                let provider = SkillSearchModule::new(
                    skills_dir.clone(),
                    apeireth_tools_canonical::skill::DEFAULT_SKILL_DISCLOSURE_BUDGET_CHARS,
                );
                capabilities.extend(provider.capabilities());
            }
        }

        // W2 §4.3 (2026-10-10, 默认关): education Dx-Check 工具 (纯确定性, 0 副作用)。
        if config.education {
            let provider = EducationModule::new();
            capabilities.extend(provider.capabilities());
        }

        // 文件写入总闸 (宪法级, 内测整改): `file_write` 是**唯一写总闸** ——
        // 组装根把同一开关值注入 shell 配置, shell 写命令 (重定向/删除类/写
        // 命令面) 与 apply_patch 同受此闸, 开关名与权力相符。关 = 写意图拒绝
        // 即帧 (`pipeline.pre_deny`); 开 = shell 行为不变 (审批链照旧走既有
        // 风险映射)。
        if let Some(shell_config) = config.shell {
            let provider = ShellModule::new(shell_config.with_file_write(config.file_write));
            capabilities.extend(provider.capabilities());
        }

        if let Some(fetch_config) = config.fetch {
            let provider = FetchModule::new(fetch_config);
            capabilities.extend(provider.capabilities());
        }

        // 内容流接入 (`tool.content_feed`, J1, opt-in): 订阅源管理 + 受控拉取 +
        // 结构化摘要 + 入记忆流 (L2 封套披露)。治理档位在治理链
        // (`ContentFeedApprovalHook`): RequireApproval 级 —— 拉取与入库都经
        // 审批档, 改订阅走 write_intent 同族 (改订阅 = 写文件)。
        if let Some(feed_config) = config.content_feed {
            let provider = ContentFeedModule::new(feed_config);
            capabilities.extend(provider.capabilities());
        }

        // 双引擎主动关怀 (`tool.care_schedule`, V3-a, opt-in): 关怀规则面
        // (增删列, 四类触发器词表)。治理档位在治理链 (`CareScheduleApprovalHook`):
        // RequireApproval 级 —— 改规则 = 写文件 (write_intent 同族), 每次调用
        // 都停人工审批。
        if let Some(care_dir) = config.care_schedule {
            let provider = CareScheduleModule::new(care_dir);
            capabilities.extend(provider.capabilities());
        }

        // 备份导出 (`tool.backup_export`, G5 备份恢复, opt-in): 全量打包 +
        // manifest.json 逐文件摘要 (损坏可检出), 凭据不随行。治理档位在授权层
        // 同源接线 (RequireApproval 级: 每次导出都停人工审批)。
        if let Some(backup_config) = config.backup {
            let provider = BackupExportModule::new(backup_config);
            capabilities.extend(provider.capabilities());
        }

        // 通用嗓发声 (`tool.voice_speak`, V3-b, opt-in): 文本 → 系统语音引擎
        // 合成即播 + 语句账本 (`spoken.jsonl`) + 安静纪律 (勿扰窗口, 与
        // care_rules 同源)。治理 = 发声面档 (注册即闸, grant 即发声免逐次
        // 人批, 与 shell 的执行档分立)。合成执行面 = 系统引擎桥 (可注入)。
        if let Some(voice_config) = config.voice_speak {
            let provider = VoiceSpeakModule::new(
                voice_config,
                apeireth_tools_canonical::system_synth_engine(),
            );
            capabilities.extend(provider.capabilities());
        }

        // 任务簿 (`tool.task_board`, V6-a 任务编排持久队列, opt-in): 增/列/
        // 改派/完成 over 持久任务队列 (与会话账本同库的任务表; 崩溃恢复/幂等
        // 唤醒/环路熔断/结算接账在 orchestration 队列核)。治理档位在治理链
        // (`TaskBoardApprovalHook`): 写面 = write_intent 同族 (改任务簿 = 写
        // 文件) 每次人批, `task_list` 只读零审批; **无 failed/retry 工具面**
        // (failed 只能人判)。自述任务节与工具面同队同账 (task_board_status_probe)。
        if let Some(queue) = config.task_board.clone() {
            let provider = TaskBoardModule::new(queue);
            capabilities.extend(provider.capabilities());
        }

        // 性格周报 (`tool.trait_report`, 裁定 ⑤): 近 7 天四杆移动量 / 移动原因
        // top / 锁定态, 数据 = 成长计既有账 (不新造账)。**只读档零审批**
        // (与 `tool.self_status` 同档); 注册即闸 (成长账落位注入才注册), 名册
        // 按实际注册条件取值。自述性格节与工具面同账 (trait_status_probe)。
        if let Some(paths) = &config.trait_report {
            let provider = TraitReportModule::new(trait_growth_probe(paths.clone()));
            capabilities.extend(provider.capabilities());
        }

        // 自省通道 (与既有 5 内置工具同列): `tool.self_status` 结构化自述面。
        // 只读档、默认可用、零审批; data 系探测口缺位时对应字段显式 null +
        // 原因, 注册与其余字段不受影响。
        if let Some(roster) = self_status_roster {
            let mut source = ProductionSelfStatusSource::new(
                roster,
                self_tuning_effective,
                backends.workspace_root.clone(),
            );
            if let Some(probe) = &backends.self_status_ledger {
                source = source.with_ledger_probe(Arc::clone(probe));
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
            // 性格节真值 (裁定 ④⑤): 锁定态 + 最近移动与 `tool.trait_report`
            // 同一成长计既有账 (同账同源, 缺落位 = 显式 null + 原因)。
            if let Some(paths) = &config.trait_report {
                source = source.with_trait_growth_probe(trait_status_probe(paths.clone()));
            }
            let provider = SelfStatusModule::new(Arc::new(source));
            capabilities.extend(provider.capabilities());
        }

        // External tool bridge (MCP): the slot is opt-in, and enabling it
        // loads the server list at assembly time (env primary, data-directory
        // stored document secondary) with fail-closed semantics. Connections
        // are established asynchronously after build; the module bag is the
        // registration target for the dynamic tools discovery produces. The
        // server list was loaded above (before the roster projection) — a
        // server list with no enabled entry assembles an inert bridge (no
        // external tools), and the roster reports that honestly.
        let mut mcp_bridge = None;
        let mut mcp_module = None;
        if let Some(server_config) = mcp_server_config {
            let options = McpBridgeOptions::from_env();
            let bridge = McpToolBridge::new(server_config, options).map_err(|error| {
                RuntimeError::misconfigured(format!("mcp tool bridge refused to assemble: {error}"))
            })?;
            let module = Arc::new(McpModule::new());
            capabilities.extend(module.capabilities());
            mcp_bridge = Some(Arc::new(bridge));
            mcp_module = Some(module);
        }

        // Unified Memory 2.0 coordinator wiring
        let mut shared_coordinator: Option<Arc<MemoryCoordinator>> = None;
        if config.memory_recall || config.memory_writeback {
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
            let mut module = MemoryRecallModule::new(memory);
            if let Some(coord) = &shared_coordinator {
                module = module.with_coordinator(Arc::clone(coord));
            }
            if let Some(wire) = &backends.self_tuning {
                module = module.with_self_tuning(Arc::clone(wire));
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
            modules.push(Arc::new(
                JudgeModule::new(config.judge, Arc::clone(&observations))
                    .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.self_assessment {
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
            modules.push(Arc::new(
                CouncilModule::new(
                    required(backends.council, "council", "council")?,
                    Arc::clone(&clock),
                )
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.organs {
            modules.push(Arc::new(OrganModule::new(Arc::clone(&clock))));
        }

        if config.reflexion {
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
            modules.push(Arc::new(super::cognitive::AbsorptionInsightModule::new()));
        }

        if config.partner_bond {
            let store = required(backends.partner_store, "partner_bond", "partner_store")?;
            let subject_id = backends
                .typed_recall_identity
                .as_ref()
                .map(|identity| identity.subject_id.clone())
                .unwrap_or_else(|| "local-user".to_string());
            modules.push(Arc::new(
                super::cognitive::PartnerBondModule::new(store, subject_id, Arc::clone(&clock))
                    .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        // V4-a 恒定小注入 (人格记忆层回取面 ①) 的 TurnStart overlay 装配点:
        // 自我叙事摘要 ≤ 恒预算 (`CONSTANT_INJECTION_BUDGET_CHARS`) 进提示词。
        // **缺人格层 = 不注册本模块 = 零注入零变化**; 人格层无自我叙事沉淀时
        // 同样零注入 (未沉淀就是未沉淀, 不编)。
        if let Some(persona) = &backends.persona_layer {
            modules.push(Arc::new(
                super::cognitive::PersonaConstantInjectionModule::new(Arc::clone(persona))
                    .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.memory_writeback {
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

        let mut seen = std::collections::BTreeSet::new();
        for module in &modules {
            let id = module.manifest().id.clone();
            if !seen.insert(id.clone()) {
                return Err(RuntimeError::misconfigured(format!(
                    "duplicate cognitive module id {id:?}"
                )));
            }
        }

        Ok(Self {
            modules,
            capabilities,
            telemetry,
            mcp_bridge,
            mcp_module,
        })
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
            builder = builder.with_module(module);
        }
        for capability in self.capabilities {
            builder = builder.with_capability(capability);
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

    /// The external tool bridge assembled for the MCP slot, when enabled.
    ///
    /// Adapters keep this handle to run the asynchronous connect/discovery
    /// pass and to emit the redacted startup log.
    pub fn mcp_bridge(&self) -> Option<&Arc<McpToolBridge>> {
        self.mcp_bridge.as_ref()
    }

    /// The MCP dynamic tool module bag, when the MCP slot is enabled.
    pub fn mcp_module(&self) -> Option<&Arc<McpModule>> {
        self.mcp_module.as_ref()
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
