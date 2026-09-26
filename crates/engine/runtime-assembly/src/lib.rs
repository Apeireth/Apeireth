//! Production composition for [`apeireth_runtime`].
//!
//! This crate owns concrete Memory, Organ, Tool, and SQLite wiring. The
//! runtime crate itself only exposes the lifecycle and port contracts.

#![deny(unsafe_code)]

pub mod canonical;
pub mod sqlite_session;

pub use canonical::{
    consolidation_cadence_turns, is_write_or_execute_capability, tuning_log_path,
    tuning_log_path_from_session_db, with_memory_context_projection, CanonicalMemoryTypedSink,
    CognitiveBackends, CognitiveModuleConfig, CognitiveModuleEvent, CognitiveTelemetry,
    CouncilModule, FetchModule, FilesystemModule, GuardDatasetObserver, InvokerLlmFactory,
    InvokerLlmInstance, JudgeConfig, JudgeModule, JudgeObservations, JudgeResult, JudgeVerdict,
    McpModule, MemoryContextProjector, MemoryRecallAccessRecorder, MemoryRecallModule,
    MemoryWritebackModule, ModuleMetricsSnapshot, OnionLayerHook, OrganModule,
    OrganModuleObservation, PermissionPresetGovernanceHook, PreferenceEvidence,
    PreferenceLearningModule, PreferenceLearningStats, PreferencePolarity, PreferenceRecallModule,
    ProductionBackends, ProductionCognitiveModules, ProductionModules, ProductionModulesConfig,
    ReflexionModule, RepoModule, SearchModule, SelfAssessmentModule, SelfTuningWire, ShellModule,
    SqliteTypedMemoryRecallSource, COUNCIL_MODULE_ID, DEFERRED_COGNITIVE_SLOTS,
    INVOKER_LLM_FACTORY_NAME, JUDGE_MODULE_ID, MEMORY_RECALL_MODULE_ID, MEMORY_WRITEBACK_MODULE_ID,
    ORGAN_MODULE_ID, PREFERENCE_LEARNING_MODULE_ID, PREFERENCE_RECALL_MODULE_ID,
    REFLEXION_MODULE_ID, SELF_ASSESSMENT_MODULE_ID, SELF_TUNING_ENABLE_ENV, TUNING_LOG_FILE,
};
pub use sqlite_session::SqliteSessionStore;
