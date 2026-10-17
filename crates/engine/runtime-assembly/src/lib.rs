//! Production composition for [`apeireth_runtime`].
//!
//! This crate owns concrete Memory, Organ, Tool, and SQLite wiring. The
//! runtime crate itself only exposes the lifecycle and port contracts.

#![deny(unsafe_code)]

pub mod canonical;
// W3-① 多户数据隔离 (星群数据层): 用户域数据平面 (三面按域分库 + 跨域拒绝即帧)。
pub mod domain_plane;
// V6-a 持久任务队列存储: 与会话账本**同库** (`sessions.sqlite3`) 的任务表
// (一次 update = 一次 IMMEDIATE 事务原子读改写)。
pub mod sqlite_session;
pub mod sqlite_task_queue;

pub use domain_plane::{
    session_row_count, DefaultDomainStores, DomainDataCounts, DomainDataPlane, DomainHandles,
    DomainMemoryStores,
};

pub use canonical::{
    backup_status_probe, budget_status_from_configured, budget_status_from_constants,
    care_status_probe, consolidation_cadence_turns, domain_status_probe,
    is_write_or_execute_capability, max_rounds_per_turn_constant,
    max_tool_calls_per_round_constant, migrate_legacy_tuning_log_file, persona_status_probe,
    roster_from_config, session_quota_probe, tuning_log_path, tuning_log_path_from_session_db,
    voice_quiet_hours_from_care_rules, with_memory_context_projection, BackupExportModule,
    CanonicalMemoryTypedSink, CognitiveBackends, CognitiveModuleConfig, CognitiveModuleEvent,
    CognitiveTelemetry, CouncilModule, FetchModule, FilesystemModule, GuardDatasetObserver,
    InvokerLlmFactory, InvokerLlmInstance, JudgeConfig, JudgeModule, JudgeObservations,
    JudgeResult, JudgeVerdict, McpModule, MemoryContextProjector, MemoryRecallAccessRecorder,
    MemoryRecallModule, MemoryWritebackModule, ModuleMetricsSnapshot, OnionLayerHook, OrganModule,
    OrganModuleObservation, PermissionPresetGovernanceHook, PersonaConstantInjectionModule,
    PreferenceEvidence, PreferenceLearningModule, PreferenceLearningStats, PreferencePolarity,
    PreferenceRecallModule, ProductionBackends, ProductionCognitiveModules, ProductionModules,
    ProductionModulesConfig, ProductionSelfStatusSource, ReflexionModule, RepoModule, SearchModule,
    SelfAssessmentModule, SelfStatusModule, SelfTuningWire, ShellModule,
    SqliteTypedMemoryRecallSource, VoiceSpeakModule, BUDGET_SOURCE_CONFIGURED,
    BUDGET_SOURCE_CONFIGURED_NOTE, BUDGET_SOURCE_CONSTANT, COUNCIL_MODULE_ID,
    DEFERRED_COGNITIVE_SLOTS, INVOKER_LLM_FACTORY_NAME, JUDGE_MODULE_ID, MEMORY_RECALL_MODULE_ID,
    MEMORY_WRITEBACK_MODULE_ID, ORGAN_MODULE_ID, PERSONA_INJECTION_MODULE_ID,
    PREFERENCE_LEARNING_MODULE_ID, PREFERENCE_RECALL_MODULE_ID, REFLEXION_MODULE_ID,
    SELF_ASSESSMENT_MODULE_ID, SELF_STATUS_TIMEOUT_MS, SELF_TUNING_ENABLE_ENV, TUNING_LOG_DOC_NAME,
    TUNING_LOG_DOC_VERSION, TUNING_LOG_FILE,
};
pub use sqlite_session::SqliteSessionStore;
// V6-a 任务编排持久队列: sqlite 任务表 (会话账本同库) + `tool.task_board` 工具面
// (含治理档钩子) + 自述任务节探测口。
pub use canonical::self_status_source::task_board_status_probe;
pub use canonical::task_board_tool::{
    task_board_approval_reason, task_board_write_face_for_action, TaskBoardApprovalHook,
    TaskBoardTool, TaskBoardWriteFace, ACTION_TASK_ADD, ACTION_TASK_COMPLETE, ACTION_TASK_LIST,
    ACTION_TASK_REASSIGN, TASK_BOARD_ACTIONS, TASK_BOARD_CAPABILITY_ID, TASK_BOARD_TOOL_NAME,
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
