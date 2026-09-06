//! Introspection and telemetry DTOs for behavior-chain safety guard.
//!
//! Exposes typed status, recent evaluation events, and dry-run endpoints
//! for Desktop and Gateway introspection without exposing secrets or raw CoT.

use serde::{Deserialize, Serialize};

use crate::decision::GuardStage;
use apeireth_governance::TaskIntentEnvelopeV1;

/// Overall status of the safety guard service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardStatusDto {
    pub enabled: bool,
    pub fast_guard_active: bool,
    pub chain_guard_active: bool,
    #[serde(default)]
    pub intent_guard_active: bool,
    #[serde(default)]
    pub cross_turn_monitoring_active: bool,
    pub active_chains: usize,
    pub total_evaluations: u64,
    pub total_allowed: u64,
    pub total_denied: u64,
    pub total_approval_required: u64,
    pub dataset_recording_enabled: bool,
    #[serde(default)]
    pub ml_classifier_available: bool,
    #[serde(default)]
    pub ml_model_version: Option<String>,
    #[serde(default)]
    pub ml_mode: String,
    #[serde(default)]
    pub ml_reason: Option<String>,
    #[serde(default)]
    pub feature_schema_version: String,
    #[serde(default)]
    pub dataset_version: String,
}

/// A recent guard evaluation event suitable for SSE or audit log display in Desktop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardEventDto {
    pub timestamp_ms: i64,
    pub session_id: String,
    pub trace_id: String,
    pub round: u32,
    pub capability_id: String,
    pub stage: GuardStage,
    pub decision: String,
    pub risk_score: f64,
    pub reasons: Vec<String>,
    pub evidence: Vec<String>,
}

/// Request payload for a dry-run evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardDryRunRequest {
    pub session_id: Option<String>,
    pub capability_id: String,
    pub arguments: serde_json::Value,
    pub declared_scope: Option<String>,
    #[serde(default)]
    pub intent: Option<TaskIntentEnvelopeV1>,
}

/// Result payload for a dry-run evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardDryRunResponse {
    pub decision: String,
    pub stage: GuardStage,
    pub risk_score: f64,
    pub reasons: Vec<String>,
    pub evidence: Vec<String>,
}
