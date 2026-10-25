//! Production composition for [`apeireth_runtime`].
//!
//! This crate owns concrete Memory, Organ, Tool, and SQLite wiring. The
//! runtime crate itself only exposes the lifecycle and port contracts.

#![deny(unsafe_code)]

pub mod accountability;
pub mod accountability_probe;
pub mod canonical;
pub mod consequence;
// 问责闭环第 ③ 步 · 自动回填面: 工具失败 ⇒ 对应已声明影响回填 `failed`
// （归属由调用方**显式登记**；`confirmed` 仍只走人工/外部）。
pub mod consequence_backfill;
pub mod weather_probe;
// W3-① 多户数据隔离 (星群数据层): 用户域数据平面 (三面按域分库 + 跨域拒绝即帧)。
pub mod domain_plane;
pub mod egress_gate;
pub mod egress_ledger;
pub mod notification_duty;
// B-P1 冷启动冲刺: 启动路径分段计时 (计时器注入) —— 会话/认知库打开、迁移、
// 模块构造、运行时组装各段可测可断言。
pub mod startup;
// V6-a 持久任务队列存储: 与会话账本**同库** (`sessions.sqlite3`) 的任务表
// (一次 update = 一次 IMMEDIATE 事务原子读改写)。
pub mod sqlite_session;
pub mod sqlite_task_queue;

pub use startup::{
    ScriptedTimer, StartupPhase, StartupSpan, StartupTimeline, StartupTimer, SystemStartupTimer,
};
// B-P1 懒加载断言: 模块族构造探针 + 组装期插桩 (开关关 = 不构造, 零构造可断言)。
pub use canonical::{BuildInstrumentation, CountingBuildProbe, ModuleBuildProbe, ModuleFamily};

pub use domain_plane::{
    session_row_count, DefaultDomainStores, DomainDataCounts, DomainDataPlane, DomainHandles,
    DomainMemoryStores,
};

pub use canonical::{
    backup_status_probe, budget_status_from_configured, budget_status_from_constants,
    care_status_probe, consolidation_cadence_turns, cost_routing_probe, doc_ops_status_probe,
    domain_status_probe, image_ops_status_probe, is_write_or_execute_capability,
    max_rounds_per_turn_constant, max_tool_calls_per_round_constant,
    migrate_legacy_tuning_log_file, persona_status_probe, roster_from_config, session_quota_probe,
    sheet_ops_status_probe, tuning_log_path, tuning_log_path_from_session_db,
    voice_quiet_hours_from_care_rules, with_memory_context_projection, BackupExportModule,
    CanonicalMemoryTypedSink, CognitiveBackends, CognitiveModuleConfig, CognitiveModuleEvent,
    CognitiveTelemetry, CouncilModule, DocOpsModule, FetchModule, FilesystemModule,
    GuardDatasetObserver, ImageOpsModule, InvokerLlmFactory, InvokerLlmInstance, JudgeConfig,
    JudgeModule, JudgeObservations, JudgeResult, JudgeVerdict, McpModule, MemoryContextProjector,
    MemoryRecallAccessRecorder, MemoryRecallModule, MemoryWritebackModule, ModuleMetricsSnapshot,
    OnionLayerHook, OrganModule, OrganModuleObservation, PermissionPresetGovernanceHook,
    PersonaConstantInjectionModule, PreferenceEvidence, PreferenceLearningModule,
    PreferenceLearningStats, PreferencePolarity, PreferenceRecallModule, ProductionBackends,
    ProductionCognitiveModules, ProductionModules, ProductionModulesConfig,
    ProductionSelfStatusSource, ReflexionModule, RepoModule, SearchModule, SelfAssessmentModule,
    SelfStatusModule, SelfTuningWire, SheetOpsModule, ShellModule, SqliteTypedMemoryRecallSource,
    VoiceSpeakModule, BUDGET_SOURCE_CONFIGURED, BUDGET_SOURCE_CONFIGURED_NOTE,
    BUDGET_SOURCE_CONSTANT, COUNCIL_MODULE_ID, DEFERRED_COGNITIVE_SLOTS, INVOKER_LLM_FACTORY_NAME,
    JUDGE_MODULE_ID, MEMORY_RECALL_MODULE_ID, MEMORY_WRITEBACK_MODULE_ID, ORGAN_MODULE_ID,
    PERSONA_INJECTION_MODULE_ID, PREFERENCE_LEARNING_MODULE_ID, PREFERENCE_RECALL_MODULE_ID,
    REFLEXION_MODULE_ID, SELF_ASSESSMENT_MODULE_ID, SELF_STATUS_TIMEOUT_MS, SELF_TUNING_ENABLE_ENV,
    TUNING_LOG_DOC_NAME, TUNING_LOG_DOC_VERSION, TUNING_LOG_FILE,
};
pub use sqlite_session::SqliteSessionStore;
// 跨线机械补位 (设备受控执行自述批报备, K 轴 L2 消费端批代补编译断点): 自述
// 来源 trait 的根级再导出 (trait 本体在 `apeireth_tools_canonical`,
// `device_control_status_probe` 测试按根级路径引用)。
pub use apeireth_tools_canonical::SelfStatusSource;
// V4-c 检索维度自进化: 注入后命中回执 → 证据 → 层权重/条目显著性小幅调权,
// 自进化账 `retrieval_tuning.jsonl` (append-only) + 开关
// `APEIRETH_ENABLE_RETRIEVAL_TUNING` (默认关 fail-closed)。
pub use canonical::{
    memory_ledger_tuning_fields, read_retrieval_tuning_log, retrieval_tuning_doc_compat,
    retrieval_tuning_enabled_from_env_value, retrieval_tuning_log_path,
    retrieval_tuning_log_path_from_session_db, RetrievalTuningLedgerSummary, RetrievalTuningWire,
    SignalSurface, RETRIEVAL_TUNING_DOC_NAME, RETRIEVAL_TUNING_DOC_VERSION,
    RETRIEVAL_TUNING_ENABLE_ENV, RETRIEVAL_TUNING_LOG_FILE,
};
// 常驻心跳关怀生产者 (V3-a 收尾一针): 装配句柄 + 开关 (注册即闸 fail-closed) +
// 自述关怀节心跳态探测口 (下次 poll 时刻 / 最近心跳命中)。
pub use canonical::care_heartbeat::{
    CareHeartbeatConfig, CareHeartbeatPoll, CareHeartbeatProducer, CareHeartbeatStatus,
    CARE_ENABLE_ENV, CARE_HEARTBEAT_ENABLE_ENV, CARE_HEARTBEAT_POLL_INTERVAL_ENV,
    DEFAULT_CARE_HEARTBEAT_POLL_INTERVAL_SECS,
};
pub use canonical::self_status_source::care_status_probe_with_heartbeat;
// 提醒侧消费桥 (K 轴 L2 提醒 + L5 守护告警 → care 同源接缝): 两本信号账 →
// 关怀候选 (同源打扰纪律; 守护例外档) + 关怀节天气提醒计数真值源。
pub use canonical::care_reminder::{
    guard_alert_to_candidate, weather_signal_to_candidate, CompositeReminderFeed, GuardAlertFeed,
    WeatherSignalFeed,
};
pub use weather_probe::weather_reminder_count;
// V6-a 任务编排持久队列: sqlite 任务表 (会话账本同库) + `tool.task_board` 工具面
// (含治理档钩子) + 自述任务节探测口。
pub use canonical::self_status_source::task_board_status_probe;
// K 轴 L1 在场感知 (感知骨架): `tool.device_sense` + `tool.weather` 装配模块 +
// 自述在场节探测口 (设备数/白名单数/最近感知, 与工具面同表同源)。
// K 轴 L3/L4 受控执行骨架: `tool.device_control` 装配模块 (三拍执行 + 三档
// 分级审批 + 白名单只读消费 L1 设备表)。
pub use canonical::self_status_source::device_presence_probe;
// K 轴 L3/L4 受控执行自述接线 (设备审批卡→IM 批): 自述设备受控执行节探测口
// (控制计数/最近动作, 与 `tool.device_control` 工具面同账同源)。
pub use canonical::self_status_source::device_control_status_probe;
pub use canonical::task_board_tool::{
    task_board_approval_reason, TaskBoardApprovalHook, TaskBoardTool, TaskBoardWriteFace,
    ACTION_TASK_ADD, ACTION_TASK_COMPLETE, ACTION_TASK_LIST, ACTION_TASK_REASSIGN,
    TASK_BOARD_ACTIONS, TASK_BOARD_CAPABILITY_ID, TASK_BOARD_TOOL_NAME,
};
pub use canonical::tool_modules::{
    DeviceControlModule, DeviceSenseModule, GuardWatchModule, WeatherModule,
};
// 性格锁定 + 性格周报 (裁定 ④⑤): 特质锁定态持久档 (`trait-locks.json`) /
// 成长计既有账读面 / 只读周报工具面 `tool.trait_report`。
pub use canonical::{
    load_trait_locks, now_epoch_ms, read_growth_records, save_trait_locks, set_trait_lock,
    trait_growth_probe, trait_locks_path, trait_locks_path_for, trait_status_probe,
    TraitGrowthFacts, TraitGrowthPaths, TraitGrowthProbe, TraitReportModule, TraitReportTool,
    TRAIT_LOCKS_FILE, TRAIT_REPORT_CAPABILITY_ID, TRAIT_REPORT_FRAME_FIELDS,
    TRAIT_REPORT_TOOL_NAME,
};
pub use sqlite_task_queue::SqliteTaskStore;
