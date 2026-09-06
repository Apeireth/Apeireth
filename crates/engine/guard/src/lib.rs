//! Two-Stage Agent Behavior-Chain Safety Classifier (`apeireth-guard`).
//!
//! Provides deterministic Fast Guard (Stage A) for immediate sub-millisecond risk rejection,
//! combined with multi-step Behavior Chain Guard (Stage B) for compound risk, escalation,
//! and data egress tracking. Exposes canonical [`apeireth_governance::GovernanceHook`]
//! integration, desensitized ML dataset collection (`guard-dataset-v1`), and Desktop
//! observability endpoints.

pub mod chain;
pub mod chain_guard;
pub mod classifier;
pub mod dataset;
pub mod decision;
pub mod enforcement;
pub mod fast_guard;
pub mod features;
pub mod features_v2;
pub mod fusion;
pub mod hook;
pub mod intent;
pub mod introspection;
pub mod observation;
pub mod semantics;

pub use chain::{ActionNode, ActionStatus, BehaviorChain, BehaviorEdge, BehaviorNode, EdgeType};
pub use chain_guard::ChainGuard;
pub use classifier::{
    ChainRiskClassifier, ClassifierEnforcementMode, JointModelArtifact, JointRiskClassifier,
    NoClassifier, RiskClass, RiskPrediction, ThresholdClassifier,
};
pub use dataset::{DatasetRecorder, GuardDatasetRecord, GuardExecutionOutcome};
pub use decision::{GuardDecision, GuardStage};
pub use enforcement::EnforcementDirective;
pub use fast_guard::{FastGuard, FastGuardResult};
pub use features::{AgentChainFeatureV1, AGENT_CHAIN_FEATURE_V1};
pub use features_v2::{AgentChainFeatureV2, CrossTurnRiskSummary, AGENT_CHAIN_FEATURE_V2};
pub use fusion::DecisionFusion;
pub use hook::BehaviorChainGuardHook;
pub use intent::{
    constrain_to_trusted, AlignmentAssessment, AlignmentClass, IntentAlignmentGuard, IntentInput,
    IntentInterpreter, RuleIntentInterpreter,
};
pub use introspection::{GuardDryRunRequest, GuardDryRunResponse, GuardEventDto, GuardStatusDto};
pub use observation::{DataSensitivity, ResourceClass, SafetyObservation, SinkClass, SourceClass};
pub use semantics::{
    descriptor_for_capability, CapabilitySafetyDescriptor, CapabilitySafetyRegistry,
};
