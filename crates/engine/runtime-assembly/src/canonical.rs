//! Concrete production modules, kept outside the runtime kernel.

// Kernel ports are re-exported as submodules because the concrete
// implementations use the same `super::module` paths they used before. This
// keeps the move source-compatible without making the kernel depend upward.
pub mod approval {
    pub use apeireth_runtime::canonical::approval::*;
}
pub mod capability {
    pub use apeireth_runtime::canonical::capability::*;
}
pub mod error {
    pub use apeireth_runtime::canonical::error::*;
}
pub mod events {
    pub use apeireth_runtime::canonical::events::*;
}
pub mod execute {
    pub use apeireth_runtime::canonical::execute::*;
}
pub mod module {
    pub use apeireth_runtime::canonical::module::*;
}
pub mod provider {
    pub use apeireth_runtime::canonical::provider::*;
}
pub mod runtime {
    pub use apeireth_runtime::canonical::runtime::*;
}
pub mod session {
    pub use apeireth_runtime::canonical::session::*;
}
pub mod subloop {
    pub use apeireth_runtime::canonical::subloop::*;
}
pub mod trace {
    pub use apeireth_runtime::canonical::trace::*;
}

// Keep the assembly crate pleasant to use for integration tests and host
// composition: kernel ports remain available from the same canonical surface,
// while concrete implementations below stay owned by this crate.
pub use approval::*;
pub use capability::*;
pub use error::*;
pub use events::*;
pub use execute::*;
pub use module::*;
pub use provider::*;
pub use runtime::*;
pub use session::*;
pub use subloop::*;
pub use trace::*;

/// Wave 4 / ADR-0009 R1b-2c · 审批社交信号源 (生产面): CLI 人工决定 → `ApprovalSocial`
/// 信号 → 调质订阅者; 取消不发布, 拒绝理由不进信号; 默认关时装配根不交付句柄。
#[path = "canonical/bio_approval_social.rs"]
pub mod bio_approval_social;
/// Wave 4 观测面补格: 调质 (SG-13) / 昼夜节律 (SG-23) / 仪式 (SG-22) 三条真实
/// 只读探测口 —— 值来自装配根正在使用的同一份状态, 开关关一条不注册。
#[path = "canonical/bio_extra_probes.rs"]
pub mod bio_extra_probes;
/// Wave 4 SG-20 · 社会修复观测面: 人工拒绝审批（ApprovalSocial=Rejected）经总线
/// 订阅折算修复计划（只读观测）；修复指引族开关关 = 不构造不订阅。
#[path = "canonical/bio_social_repair.rs"]
pub mod bio_social_repair;

/// 常驻心跳关怀生产者装配面 (V3-a 收尾一针): 定时 poll 关怀内核 → 命中发起
/// 标准回合 (开关 `APEIRETH_ENABLE_CARE_HEARTBEAT` 注册即闸, 默认关 fail-closed)。
#[path = "canonical/care_heartbeat.rs"]
pub mod care_heartbeat;
/// 提醒侧消费桥 (K 轴 L2 提醒 + L5 守护告警 → care 引擎同源接缝): 两本信号账 →
/// 关怀候选 (同源打扰纪律; 守护例外档)。
#[path = "canonical/care_reminder.rs"]
pub mod care_reminder;
#[path = "canonical/care_signals.rs"]
mod care_signals;
#[path = "canonical/causal_world_model.rs"]
pub mod causal_world_model;
#[path = "canonical/cognitive.rs"]
pub mod cognitive;
#[path = "canonical/context_budget.rs"]
pub mod context_budget;
#[path = "canonical/cost_telemetry.rs"]
pub mod cost_telemetry;
/// D-2：council ballot 的 append-only 落盘面（JSONL）。
#[path = "canonical/council_ballot_log.rs"]
pub mod council_ballot_log;
/// D-2：顾问评价 ⇒ 立场票的折算（唯一的折算点）。
#[path = "canonical/council_ballots.rs"]
pub mod council_ballots;
#[path = "canonical/dream_llm.rs"]
pub mod dream_llm;
/// W3 实现批: 自我改进闭环实验侧 (2026-10-10)。
#[path = "canonical/experiment_field.rs"]
pub mod experiment_field;
#[path = "canonical/guard_observer.rs"]
pub mod guard_observer;
#[path = "canonical/harness_patch.rs"]
pub mod harness_patch;
#[path = "canonical/invariant_registry.rs"]
pub mod invariant_registry;
/// B2 批「三接口契约」的**生产装配接线**（读契约 / 标注契约 / 注入契约）
/// + **口径对齐自检**（分叉 ⇒ 拒绝装配）。
#[path = "canonical/memory_contract_wire.rs"]
pub mod memory_contract_wire;
#[path = "canonical/memory_correction.rs"]
pub mod memory_correction;
#[path = "canonical/memory_recovery.rs"]
pub mod memory_recovery;
#[path = "canonical/memory_typed_sink.rs"]
pub mod memory_typed_sink;
/// `rt.module_execution_contract` 的**生产选举面**: 一张静态装配表 (无新 env
/// 旋钮) + 两个入口 (构造期 `elect_provider_module` / builder 期 `attach`);
/// 表内缺席 = 逐字节保持兼容合同。为何需要见文件头注释 (缺口:
/// `production::collect_tool_module` 此前只会走 `tool_module()` 兼容默认)。
#[path = "canonical/module_contract_wiring.rs"]
pub mod module_contract_wiring;
/// Wave 4 / ADR-0009 R1b-1a · 唯一 HookPoint→信号映射模块 (装配根首位注册;
/// 恒 `continue_`, 只发布不改回合)。
#[path = "canonical/nervous_bridge.rs"]
pub mod nervous_bridge;
/// Wave 4 / ADR-0009 R1b-1a · 神经信号总线 (运行时内部私有同步面): 闭包订阅、
/// 同步投递、零订阅零副作用; 不挂公共 ABI、不登记扩展点、不跨 + 层导出。
#[path = "canonical/nervous_bus.rs"]
pub mod nervous_bus;
/// Wave 4 / ADR-0009 R1b-1a · 生物开关表: 9 开关 + 1 参数一次解析 (17 文件→1 处),
/// fail-closed 只认 `"1"`; 全关 = 不注册总线桥 = 零行为变化。
#[path = "canonical/nervous_switchboard.rs"]
pub mod nervous_switchboard;
/// 守夜人 Nightwatch — 离线闲时审计器 (2026-10-10, 主人批准设计)。
#[path = "canonical/nightwatch.rs"]
pub mod nightwatch;
/// W3 三洋葱 L3-L5 物理执行面: 双洋葱判定的治理 hook (2026-10-10)。
#[path = "canonical/onion_layer.rs"]
pub mod onion_layer;
#[path = "canonical/orchestrator.rs"]
pub mod orchestrator;
#[path = "canonical/organ_llm_bridge.rs"]
pub mod organ_llm_bridge;
#[path = "canonical/organ_module.rs"]
pub mod organ_module;
#[path = "canonical/parallel_dispatch.rs"]
pub mod parallel_dispatch;
#[path = "canonical/permission_preset.rs"]
pub mod permission_preset;
/// 生物机制族 W3.3 · 游戏练习场工具面 (`tool.play_propose` / `tool.play_start` /
/// `tool.play_finish` / `tool.play_abort`, 默认关): 只做模型可调的四出口, 内核
/// (`apeireth_organ::play`) 零改动; env `APEIRETH_ENABLE_BIO_PLAY` 只认 `"1"`。
#[path = "canonical/play_tools.rs"]
pub mod play_tools;
#[path = "canonical/preference_learning.rs"]
pub mod preference_learning;
#[path = "canonical/production.rs"]
pub mod production;
/// 回合层提示注入告警面 (2026-11-05): 消费 fetch 工具**输出侧**写下的
/// `prompt_injection` 标记 ⇒ ⇒ 在回合层如实报一条**可见**告警 (固定措辞,
/// 恒不阻断)。标记的写出面在 `apeireth_tools_canonical::fetch`; 治理面的
/// `PromptInjectionHook` 只扫**入参**, 此前无人读这个输出侧标记。
#[path = "canonical/prompt_injection_guard.rs"]
pub mod prompt_injection_guard;
#[path = "canonical/repetition_advisory.rs"]
pub mod repetition_advisory;
/// V4-c 检索维度自进化接线层 (证据账 + `retrieval_tuning.jsonl` 落盘 + 调权落地)。
#[path = "canonical/retrieval_tuning_wire.rs"]
pub mod retrieval_tuning_wire;
/// 生物机制族 W3.7 · 仪式完成面 (`tool.ritual_done` / `tool.ritual_skip`, 默认关):
/// 补「仪式完成」显式事件源 (模型在用户明确确认/明确拒绝后调用), 双工具共享
/// `<care_dir>/ritual_state.json` 存储; 复用族总闸 `APEIRETH_ENABLE_BIO_CARE_CANDIDATES`。
#[path = "canonical/ritual_tools.rs"]
pub mod ritual_tools;
/// 「性格养成」第一铲: 自校准接线层 (引擎落地 + tuning-log.jsonl 落盘)。
#[path = "canonical/self_status_source.rs"]
pub mod self_status_source;
#[path = "canonical/self_tuning_wire.rs"]
pub mod self_tuning_wire;
#[path = "canonical/session_seal.rs"]
pub mod session_seal;
/// 任务簿工具面 (`tool.task_board`, V6-a): 增/列/改派/完成 + 治理档钩子
/// (写面 = write_intent 同族, failed 只能人判无工具面)。
#[path = "canonical/task_board_tool.rs"]
pub mod task_board_tool;
/// SG-25 工具生态健康 · 迟绑定只读探测口: 装配末端填入的静态剖面 →
/// 已投影画像 → 内核公式; 读数只观测不剪枝, 槽空/零收录 = 未接线真因。
#[path = "canonical/tool_ecosystem_probe.rs"]
pub mod tool_ecosystem_probe;
#[path = "canonical/tool_modules.rs"]
pub mod tool_modules;
/// SG-25 工具能力策展表 v0: 生产 `ToolCapability` 声明无能力标签, 本表显式登记
/// 工具身份 → 封闭词汇表标签; 未收录不猜测 (如实计 untagged, 不参与分数)。
#[path = "canonical/tool_taxonomy.rs"]
pub mod tool_taxonomy;
/// 性格成长数据面 (裁定 ④⑤): 特质滑杆锁定态持久档 (`trait-locks.json`) +
/// 成长计既有账 (`tuning-log.jsonl`) 只读读面。
#[path = "canonical/trait_growth.rs"]
pub mod trait_growth;
/// 性格周报工具面 (`tool.trait_report`, 只读零审批): 近 7 天四杆移动量 /
/// 移动原因 top / 锁定态, 数据 = 成长计既有账 (不新造账)。
#[path = "canonical/trait_report_tool.rs"]
pub mod trait_report_tool;
#[path = "canonical/typed_recall.rs"]
pub mod typed_recall;
#[path = "canonical/upgrade_cycle.rs"]
pub mod upgrade_cycle;
/// 发声面勿扰窗口的同源接缝 (V3-b × V3-a): `care_rules` 打扰纪律的窗口为准,
/// 未落/未配回落发声面最小自建 env (坏配置拒开不静默回退)。
#[path = "canonical/voice_quiet.rs"]
pub mod voice_quiet;

pub use bio_approval_social::{
    approval_social_outcome, BioApprovalSocialSource, APPROVAL_SOCIAL_SOURCE,
};
pub use bio_extra_probes::{
    circadian_probe, homeostasis_bio_probe, neuromodulation_probe, ritual_state_probe,
    CircadianProbeConfig, HomeostasisObservationSource, HomeostasisTurnObservation,
};
pub use bio_social_repair::{
    social_repair_bio_probe, SocialRepairObservation, SocialRepairObservationSource,
    SocialRepairProbe, SOCIAL_REPAIR_BASIS, SOCIAL_REPAIR_EVENT_BASIS,
};
pub use care_heartbeat::{
    bio_circadian, bio_tone, CareHeartbeatConfig, CareHeartbeatPoll, CareHeartbeatProducer,
    CareHeartbeatStatus, CARE_ENABLE_ENV, CARE_HEARTBEAT_ENABLE_ENV,
    CARE_HEARTBEAT_POLL_INTERVAL_ENV, DEFAULT_CARE_HEARTBEAT_POLL_INTERVAL_SECS,
};
pub use care_reminder::{
    guard_alert_to_candidate, weather_signal_to_candidate, CompositeReminderFeed, GuardAlertFeed,
    WeatherSignalFeed,
};
pub use care_signals::{
    CarePotentialDriver, CarePotentialFeed, CareSignals, DISTRESS_FULL_FAILURES, FATIGUE_FULL_HOURS,
};
pub use context_budget::{
    context_budget_chars_from_env, context_budget_chars_from_env_value, CONTEXT_BUDGET_ENV,
    DEFAULT_CONTEXT_BUDGET_CHARS,
};
pub use experiment_field::{
    Experiment, ExperimentField, ExperimentStatus, FailureLearningRecord, FailureLearningSink,
    NoopVMRunner, VMRunner, Verdict, WikiFailureLearningSink,
};

pub use nightwatch::{
    audit as nightwatch_audit, write_report as write_nightwatch_report, AuditSnapshot,
    EpisodeSnapshot, Finding, FindingArea, IdleGate, NightwatchIdleScheduler, NightwatchInputs,
    NightwatchReport,
};

pub use cognitive::{
    turn_request_from_perception, CognitiveModuleEvent, CognitiveTelemetry, CouncilModule,
    JudgeConfig, JudgeModule, JudgeObservations, JudgeResult, JudgeVerdict,
    MemoryRecallAccessRecorder, MemoryRecallAccessStore, MemoryRecallModule, MemoryWritebackModule,
    ModuleMetricsSnapshot, PersonaConstantInjectionModule, PreferenceRecallModule, ReflexionModule,
    SelfAssessmentModule, COUNCIL_MODULE_ID, DEFERRED_COGNITIVE_SLOTS, JUDGE_MODULE_ID,
    MEMORY_RECALL_MODULE_ID, MEMORY_WRITEBACK_MODULE_ID, PERSONA_INJECTION_MODULE_ID,
    PREFERENCE_RECALL_MODULE_ID, REFLEXION_MODULE_ID, SELF_ASSESSMENT_MODULE_ID,
};
pub use dream_llm::{FallbackMetaThinker, LlmMetaThinker};
pub use guard_observer::GuardDatasetObserver;
pub use memory_typed_sink::CanonicalMemoryTypedSink;
pub use nervous_bridge::{bridge_signal, NervousBridgeModule, NERVOUS_BRIDGE_MODULE_ID};
pub use nervous_bus::{
    ApprovalSocialOutcome, NervousBus, NervousBusSnapshot, NervousSignal, NervousSignalKind,
    MAX_SIGNAL_TEXT_CHARS,
};
pub use nervous_switchboard::{
    BioSwitch, NervousSwitchboard, NervousSwitchboardError, CIRCADIAN_TZ_OFFSET_ENV,
    CIRCADIAN_TZ_OFFSET_LIMIT_MIN, DEFAULT_CIRCADIAN_TZ_OFFSET_MIN,
};
pub use onion_layer::{onion_layer_for_capability, OnionLayerHook};
pub use organ_llm_bridge::{InvokerLlmFactory, InvokerLlmInstance, INVOKER_LLM_FACTORY_NAME};
pub use organ_module::{OrganModule, OrganModuleObservation, ORGAN_MODULE_ID};
pub use permission_preset::{
    human_approval_floor_applies, is_write_or_execute_capability, PermissionPresetGovernanceHook,
};
// 生物机制族 W3.3 · 游戏练习场工具面 (默认关): 装配门 + 四件套 + 路径纪律 + 审批钩子
// (钩子未在装配根注册, 见模块头 0 装边界)。
pub use play_tools::{
    bio_play_enabled_from_env_value, bio_play_wiring_from_enabled, bio_play_wiring_from_env_value,
    play_approval_reason, play_error_code, validate_play_sandbox_dir, BioPlayState, BioPlayWiring,
    PlayAbortModule, PlayApprovalHook, PlayFinishModule, PlayProposeModule, PlayStartModule,
    BIO_PLAY_ENABLE_ENV, MAX_OUTCOME_ITEMS, MAX_OUTCOME_ITEM_BYTES, MAX_PROPOSAL_CANDIDATES,
    MAX_SANDBOX_DIR_BYTES, MAX_SKILLS_PER_TOPIC, PLAY_ABORT_CAPABILITY_ID, PLAY_ABORT_TOOL_NAME,
    PLAY_CAPABILITY_IDS, PLAY_FINISH_CAPABILITY_ID, PLAY_FINISH_TOOL_NAME,
    PLAY_PROPOSE_CAPABILITY_ID, PLAY_PROPOSE_TOOL_NAME, PLAY_START_CAPABILITY_ID,
    PLAY_START_TOOL_NAME,
};
pub use preference_learning::{
    PreferenceEvidence, PreferenceLearningModule, PreferenceLearningStats, PreferencePolarity,
    PREFERENCE_LEARNING_MODULE_ID,
};
// 生物机制族 W3.7 · 仪式完成面 (默认关): 装配门 + 两件套 + 种类词表。
pub use production::{
    with_memory_context_projection, BuildInstrumentation, CognitiveBackends, CognitiveModuleConfig,
    CountingBuildProbe, MemoryContextProjector, ModuleBuildProbe, ProductionBackends,
    ProductionCognitiveModules, ProductionModules, ProductionModulesConfig,
};
pub use retrieval_tuning_wire::{
    memory_ledger_tuning_fields, read_retrieval_tuning_log, retrieval_tuning_doc_compat,
    retrieval_tuning_enabled_from_env_value, retrieval_tuning_log_path,
    retrieval_tuning_log_path_from_session_db, RetrievalTuningLedgerSummary, RetrievalTuningWire,
    SignalSurface, RETRIEVAL_TUNING_DOC_NAME, RETRIEVAL_TUNING_DOC_VERSION,
    RETRIEVAL_TUNING_ENABLE_ENV, RETRIEVAL_TUNING_LOG_FILE,
};
pub use ritual_tools::{
    bio_ritual_wiring_from_enabled, bio_ritual_wiring_from_env_value, ritual_kind_labels,
    BioRitualState, BioRitualWiring, RitualDoneModule, RitualDoneTool, RitualSkipModule,
    RitualSkipTool, RITUAL_CAPABILITY_IDS, RITUAL_DONE_CAPABILITY_ID, RITUAL_DONE_TOOL_NAME,
    RITUAL_SKIP_CAPABILITY_ID, RITUAL_SKIP_TOOL_NAME,
};
pub use self_status_source::{
    budget_status_from_configured, budget_status_from_constants, care_status_probe,
    cost_routing_probe, device_control_status_probe, doc_ops_status_probe, domain_status_probe,
    image_ops_status_probe, max_rounds_per_turn_constant, max_tool_calls_per_round_constant,
    persona_status_probe, roster_from_config, session_quota_probe, sheet_ops_status_probe,
    task_board_status_probe, ProductionSelfStatusSource, BUDGET_SOURCE_CONFIGURED,
    BUDGET_SOURCE_CONFIGURED_NOTE, BUDGET_SOURCE_CONSTANT,
};
pub use self_tuning_wire::{
    consolidation_cadence_turns, migrate_legacy_tuning_log_file, tuning_log_path,
    tuning_log_path_from_session_db, SelfTuningWire, SELF_TUNING_ENABLE_ENV, TUNING_LOG_DOC_NAME,
    TUNING_LOG_DOC_VERSION, TUNING_LOG_FILE,
};
pub use tool_ecosystem_probe::{
    tool_ecosystem_bio_probe, ToolEcosystemProbe, ToolEcosystemSlot, ToolEcosystemSnapshot,
};
pub use tool_modules::{
    BackupExportModule, DeviceControlModule, DocOpsModule, EducationModule, FetchModule,
    FilesystemModule, GitWriteModule, GuardWatchModule, ImageOpsModule, McpModule, ProcessModule,
    RepoModule, SearchModule, SelfStatusModule, SheetOpsModule, ShellModule, TraitReportModule,
    VoiceSpeakModule, SELF_STATUS_TIMEOUT_MS, TRAIT_REPORT_TIMEOUT_MS,
};
pub use tool_taxonomy::{
    capability_identity, project_capabilities, taxonomy_entry_for, TaxonomyEntry,
    TaxonomyProjection, TAXONOMY_TAGS, TOOL_TAXONOMY, TOOL_TAXONOMY_VERSION,
};
// 性格成长数据面 (裁定 ④⑤): 特质锁定态持久档 + 成长计既有账读面。
pub use trait_growth::{
    load_trait_locks, now_epoch_ms, read_growth_records, save_trait_locks, set_trait_lock,
    trait_growth_probe, trait_locks_path, trait_locks_path_for, trait_status_probe,
    TraitGrowthFacts, TraitGrowthPaths, TraitGrowthProbe, TRAIT_LOCKS_FILE,
};
// 性格周报工具面 (`tool.trait_report`, 只读零审批)。
pub use trait_report_tool::{
    TraitReportTool, TRAIT_REPORT_CAPABILITY_ID, TRAIT_REPORT_FRAME_FIELDS, TRAIT_REPORT_TOOL_NAME,
};
pub use typed_recall::SqliteTypedMemoryRecallSource;
// G5 备份恢复导出面: 备份节探测口 (最近备份时间/清单件数真值或 null) 与
// 备份引擎的组合根入口同面导出 (探针构造与引擎同源, 一个入口两类用法)。
pub use apeireth_tools_canonical::backup::backup_status_probe;
// V3-b 发声面勿扰窗口的同源接缝导出 (care_rules 窗口优先, 最小 env 兜底)。
pub use voice_quiet::voice_quiet_hours_from_care_rules;

#[path = "canonical/memory_module.rs"]
pub mod memory_module;

/// 注册式 Memory 工具面的**生产槽位**（`mem.registered_namespace` / `rt.owned_agents`）:
/// 把 build 期构造的 coordinator 变成 `MemoryModule` 工具面（7 工具 + 固定检索 Hook +
/// 模块自有 Agent 面），并支持注册期迟绑定 Host 映射表。为何需要见文件头注释。
#[path = "canonical/memory_assembly.rs"]
mod memory_assembly;

/// **B3 生产接线**：影子双跑 + 换算法流程（零风险上线 / 可回退）。
/// 默认关（`shadow.json` 不存在 ⇒ 不构造任何东西 ⇒ 逐字节零行为变化）；
/// 切换走仓里既有的治理面 ⇒ 没有治理面 ⇒ 不许换。为何需要见文件头注释。
#[path = "canonical/shadow_wire.rs"]
pub mod shadow_wire;

#[path = "canonical/cognitive_registry.rs"]
mod cognitive_registry;

#[path = "canonical/desktop_host.rs"]
pub mod desktop_host;

/// 模块族枚举（构造探针的计数单位）。
///
/// ⭐ **住在门面这一层**（2026-11-05）：该枚举被**三处**共用
/// （装配根 `production` / 生物机制族装配块 `bio_production_wiring` /
/// 回合层告警面 `prompt_injection_guard`）⇒ ⇒ 一处定义、三处引用
/// ⇒ ⇒ 它的家是**装配面这一层** ⇒ ⇒ 而装配根 `production.rs` 的尺寸棘轮
/// 额度有限 ⇒ ⇒ 搬到这里（门面）⇒ ⇒ **不占装配根的额度**。
///
/// 路径全保：`canonical::ModuleFamily`（本体）与
/// `canonical::production::ModuleFamily`（兼容再导出）⇒ ⇒ **外部调用点零改动**。
/// 模块族 (懒加载断言单位): 一族 = 一个开关族在 [`ProductionModules::build`] 里的构造点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModuleFamily {
    /// 文件读工具族 (`filesystem`)。
    Filesystem,
    /// 搜索工具族 (`search`)。
    Search,
    /// 仓库检视工具族 (`repo`)。
    Repo,
    /// 受控写文件族 (`file_write`)。
    ApplyPatch,
    /// 工作树改单族 (`worktree_plan`)。
    WorktreePlan,
    /// 工作树落盘族 (`worktree_apply`)。
    WorktreeApply,
    /// git 写族 (`git_write`)。
    GitWrite,
    /// 技能沉淀族 (`skill_distill`)。
    SkillDistill,
    /// 技能检索族 (`skill_search`)。
    SkillSearch,
    /// Dx-Check 检查族 (`education`)。
    Education,
    /// 受控 shell 族 (`shell`)。
    Shell,
    /// 受控抓取族 (`fetch`)。
    Fetch,
    /// 回合层提示注入告警族 (`guard.prompt_injection`): 消费 fetch **输出侧**
    /// 写下的 `prompt_injection` 标记 ⇒ ⇒ 回合层报一条可见告警 (恒不阻断)。
    ///
    /// 构造条件与 [`ModuleFamily::Fetch`] 同源 (fetch 工具开着才有它的输出);
    /// 两族一起构 = 抓取面一开, 它的输出侧标记就**一定**有人读。
    PromptInjectionGuard,
    /// 内容流族 (`content_feed`)。
    ContentFeed,
    /// 关怀规则族 (`care_schedule`)。
    CareSchedule,
    /// 备份导出族 (`backup`)。
    BackupExport,
    /// 发声族 (`voice_speak`)。
    VoiceSpeak,
    /// 任务簿族 (`task_board`)。
    TaskBoard,
    /// 性格周报族 (`trait_report`)。
    TraitReport,
    /// 暴露面体检族 (`exposure_audit`)。
    ExposureAudit,
    /// 创作工具面 W1 · 图像操作族 (`image_ops`)。
    ImageOps,
    /// 创作工具面 W2 · 文档操作族 (`doc_ops`)。
    DocOps,
    /// 创作工具面 K3 · 表格操作族 (`sheet_ops`)。
    SheetOps,
    /// K 轴 L1 在场感知族 (`device_sense`)。
    DeviceSense,
    /// K 轴 L1 天气感知族 (`weather`)。
    Weather,
    /// K 轴 L3/L4 受控执行族 (`device_control`)。
    DeviceControl,
    /// K 轴 L5 看家骨架族 (`guard_watch`)。
    GuardWatch,
    /// 生物机制族 W3.3 · 游戏练习场工具族 (`play_*`, 默认关): 四个工具模块各自
    /// 一个构造点 (propose/start/finish/abort, 共享同一本内存账本);
    /// env `APEIRETH_ENABLE_BIO_PLAY` 只认 `"1"` 且工作区根注入才构造。
    BioPlay,
    /// 自省通道族 (`self_status`)。
    SelfStatus,
    /// 安全/逆向侦察族 (`tool.recon.*`, 9 件, 默认关)。
    Recon,
    /// 外部适配器族 (`ext.*`, 10 件, 默认关; 子进程封装)。
    ExternalTools,
    /// 外部工具桥族 (`mcp`)。
    Mcp,
    /// 记忆协调器 (memory_recall / memory_writeback 共用)。
    MemoryCoordinator,
    /// 记忆召回族 (`memory_recall`)。
    MemoryRecall,
    /// 偏好召回族 (`preference_recall`)。
    PreferenceRecall,
    /// 偏好学习族 (`preference_learning`)。
    PreferenceLearning,
    /// 评判族 (`judge`)。
    Judge,
    /// 自评族 (`self_assessment`)。
    SelfAssessment,
    /// 议事族 (`council`)。
    Council,
    /// 器官网族 (`organs`)。
    Organs,
    /// 反思族 (`reflexion`)。
    Reflexion,
    /// 吸收洞察族 (`absorption_insight`)。
    AbsorptionInsight,
    /// 羁绊族 (`partner_bond`)。
    PartnerBond,
    /// 恒定人格注入族 (人格层槽)。
    PersonaConstantInjection,
    /// 记忆写回族 (`memory_writeback`)。
    MemoryWriteback,
}
