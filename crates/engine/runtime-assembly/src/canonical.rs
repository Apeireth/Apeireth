//! Concrete production modules, kept outside the runtime kernel.

// Kernel ports are re-exported as submodules because the migrated concrete
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

#[path = "canonical/causal_world_model.rs"]
pub mod causal_world_model;
#[path = "canonical/cognitive.rs"]
pub mod cognitive;
#[path = "canonical/cost_telemetry.rs"]
pub mod cost_telemetry;
#[path = "canonical/dream_llm.rs"]
pub mod dream_llm;
/// W3 移植批: 自我改进闭环实验侧 (v1 donor experiment_field, 2026-10-10)。
#[path = "canonical/experiment_field.rs"]
pub mod experiment_field;
#[path = "canonical/guard_observer.rs"]
pub mod guard_observer;
#[path = "canonical/harness_patch.rs"]
pub mod harness_patch;
#[path = "canonical/memory_typed_sink.rs"]
pub mod memory_typed_sink;
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
#[path = "canonical/preference_learning.rs"]
pub mod preference_learning;
#[path = "canonical/production.rs"]
pub mod production;
/// 「性格养成」第一铲: 自校准接线层 (引擎落地 + tuning-log.jsonl 落盘)。
#[path = "canonical/self_tuning_wire.rs"]
pub mod self_tuning_wire;
#[path = "canonical/tool_modules.rs"]
pub mod tool_modules;
#[path = "canonical/typed_recall.rs"]
pub mod typed_recall;
#[path = "canonical/upgrade_cycle.rs"]
pub mod upgrade_cycle;

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
    ModuleMetricsSnapshot, PreferenceRecallModule, ReflexionModule, SelfAssessmentModule,
    COUNCIL_MODULE_ID, DEFERRED_COGNITIVE_SLOTS, JUDGE_MODULE_ID, MEMORY_RECALL_MODULE_ID,
    MEMORY_WRITEBACK_MODULE_ID, PREFERENCE_RECALL_MODULE_ID, REFLEXION_MODULE_ID,
    SELF_ASSESSMENT_MODULE_ID,
};
pub use dream_llm::{FallbackMetaThinker, LlmMetaThinker};
pub use guard_observer::GuardDatasetObserver;
pub use memory_typed_sink::CanonicalMemoryTypedSink;
pub use onion_layer::{onion_layer_for_capability, OnionLayerHook};
pub use organ_llm_bridge::{InvokerLlmFactory, InvokerLlmInstance, INVOKER_LLM_FACTORY_NAME};
pub use organ_module::{OrganModule, OrganModuleObservation, ORGAN_MODULE_ID};
pub use permission_preset::{is_write_or_execute_capability, PermissionPresetGovernanceHook};
pub use preference_learning::{
    PreferenceEvidence, PreferenceLearningModule, PreferenceLearningStats, PreferencePolarity,
    PREFERENCE_LEARNING_MODULE_ID,
};
pub use production::{
    with_memory_context_projection, CognitiveBackends, CognitiveModuleConfig,
    MemoryContextProjector, ProductionBackends, ProductionCognitiveModules, ProductionModules,
    ProductionModulesConfig,
};
pub use self_tuning_wire::{
    consolidation_cadence_turns, tuning_log_path, tuning_log_path_from_session_db, SelfTuningWire,
    SELF_TUNING_ENABLE_ENV, TUNING_LOG_FILE,
};
pub use tool_modules::{
    EducationModule, FetchModule, FilesystemModule, McpModule, RepoModule, SearchModule,
    ShellModule,
};
pub use typed_recall::SqliteTypedMemoryRecallSource;
