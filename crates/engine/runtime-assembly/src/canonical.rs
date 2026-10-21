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

/// 常驻心跳关怀生产者装配面 (V3-a 收尾一针): 定时 poll 关怀内核 → 命中发起
/// 标准回合 (开关 `APEIRETH_ENABLE_CARE_HEARTBEAT` 注册即闸, 默认关 fail-closed)。
#[path = "canonical/care_heartbeat.rs"]
pub mod care_heartbeat;
/// 提醒侧消费桥 (K 轴 L2 提醒 + L5 守护告警 → care 引擎同源接缝): 两本信号账 →
/// 关怀候选 (同源打扰纪律; 守护例外档)。
#[path = "canonical/care_reminder.rs"]
pub mod care_reminder;
#[path = "canonical/causal_world_model.rs"]
pub mod causal_world_model;
#[path = "canonical/cognitive.rs"]
pub mod cognitive;
#[path = "canonical/context_budget.rs"]
pub mod context_budget;
#[path = "canonical/cost_telemetry.rs"]
pub mod cost_telemetry;
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
#[path = "canonical/memory_typed_sink.rs"]
pub mod memory_typed_sink;
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
pub mod onion_layer;
#[path = "canonical/orchestrator.rs"]
pub mod orchestrator;
#[path = "canonical/organ_llm_bridge.rs"]
pub mod organ_llm_bridge;
#[path = "canonical/organ_module.rs"]
pub mod organ_module;
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
pub use care_heartbeat::{
    bio_circadian, bio_tone, CareHeartbeatConfig, CareHeartbeatPoll, CareHeartbeatProducer,
    CareHeartbeatStatus, CARE_ENABLE_ENV, CARE_HEARTBEAT_ENABLE_ENV,
    CARE_HEARTBEAT_POLL_INTERVAL_ENV, DEFAULT_CARE_HEARTBEAT_POLL_INTERVAL_SECS,
};
pub use care_reminder::{
    guard_alert_to_candidate, weather_signal_to_candidate, CompositeReminderFeed, GuardAlertFeed,
    WeatherSignalFeed,
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
    CountingBuildProbe, MemoryContextProjector, ModuleBuildProbe, ModuleFamily, ProductionBackends,
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
    FilesystemModule, GitWriteModule, GuardWatchModule, ImageOpsModule, McpModule, RepoModule,
    SearchModule, SelfStatusModule, SheetOpsModule, ShellModule, TraitReportModule,
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
