//! Governance hook implementation for Two-Stage Behavior-Chain Safety Classifier.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_governance::{
    Decision, GovernanceHook, GovernanceRequest, GovernanceVerdict, TaskIntentEnvelopeV1,
};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::chain::{ActionStatus, BehaviorChain};
use crate::chain_guard::ChainGuard;
use crate::classifier::{ChainRiskClassifier, NoClassifier};
use crate::dataset::DatasetRecorder;
use crate::decision::GuardDecision;
use crate::fast_guard::FastGuard;
use crate::features_v2::{AgentChainFeatureV2, CrossTurnRiskSummary};
use crate::fusion::DecisionFusion;
use crate::intent::{AlignmentAssessment, AlignmentClass, IntentAlignmentGuard};
use crate::introspection::{
    GuardDryRunRequest, GuardDryRunResponse, GuardEventDto, GuardStatusDto,
};
use crate::observation::SafetyObservation;

const MAX_RECENT_EVENTS: usize = 200;

fn apply_alignment(mut decision: GuardDecision, alignment: &AlignmentAssessment) -> GuardDecision {
    decision.risk_score = decision.risk_score.max(alignment.score);
    for reason in &alignment.reasons {
        if !decision.reasons.contains(reason) {
            decision.reasons.push(reason.clone());
        }
    }
    if !alignment.reasons.is_empty() {
        decision.evidence.push(format!(
            "alignment={:?} score={:.2}",
            alignment.class, alignment.score
        ));
    }
    if matches!(decision.decision, Decision::Deny { .. }) {
        return decision;
    }
    if alignment.score >= 0.85 {
        decision.decision = Decision::deny("intent and action alignment is high risk");
        decision.stage = crate::decision::GuardStage::ChainGuard;
    } else if alignment.score >= 0.65 {
        decision.decision = Decision::require_approval("action expands the trusted task intent");
        decision.stage = crate::decision::GuardStage::ChainGuard;
    }
    decision
}

#[derive(Debug, Default, Clone)]
struct GuardCounters {
    total_evaluations: u64,
    total_allowed: u64,
    total_denied: u64,
    total_approval_required: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SessionBehaviorSummary {
    pub recent_turns: u32,
    pub denied_action_count: u32,
    pub approval_rejection_count: u32,
    pub credential_probe_count: u32,
    pub sensitive_read_count: u32,
    pub network_egress_count: u32,
    pub repeated_scope_expansion_count: u32,
    pub repeated_alternate_tool_count: u32,
    pub risk_trend: f64,
}

/// Summary of turn-level risk evaluation preserved across turns for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRiskSummary {
    pub trace_id: String,
    pub max_risk_score: f64,
    pub denied: bool,
    pub timestamp_ms: i64,
}

/// Bounded risk history across turns for a session (max 10 items).
#[derive(Debug, Clone, Default)]
pub struct SessionRiskHistory {
    pub summaries: VecDeque<TurnRiskSummary>,
}

impl SessionRiskHistory {
    pub fn push(&mut self, summary: TurnRiskSummary) {
        if self.summaries.len() >= 10 {
            self.summaries.pop_front();
        }
        self.summaries.push_back(summary);
    }
}

/// Canonical governance hook evaluating agent behavior chains across two stages.
pub struct BehaviorChainGuardHook {
    fast_guard: FastGuard,
    chain_guard: ChainGuard,
    chains: Mutex<HashMap<(SessionId, String), BehaviorChain>>,
    session_scopes: Mutex<HashMap<SessionId, String>>,
    session_risk_history: Mutex<HashMap<SessionId, SessionRiskHistory>>,
    session_behavior_summary: Mutex<HashMap<SessionId, SessionBehaviorSummary>>,
    turn_intents: Mutex<HashMap<(SessionId, String), TaskIntentEnvelopeV1>>,
    dataset_recorder: Option<Arc<DatasetRecorder>>,
    classifier: Arc<dyn ChainRiskClassifier>,
    recent_events: Mutex<VecDeque<GuardEventDto>>,
    counters: Mutex<GuardCounters>,
}

impl Default for BehaviorChainGuardHook {
    fn default() -> Self {
        Self::new()
    }
}

impl BehaviorChainGuardHook {
    /// Create a new behavior chain guard hook.
    pub fn new() -> Self {
        Self {
            fast_guard: FastGuard::new(),
            chain_guard: ChainGuard::new(),
            chains: Mutex::new(HashMap::new()),
            session_scopes: Mutex::new(HashMap::new()),
            session_risk_history: Mutex::new(HashMap::new()),
            session_behavior_summary: Mutex::new(HashMap::new()),
            turn_intents: Mutex::new(HashMap::new()),
            dataset_recorder: None,
            classifier: Arc::new(NoClassifier),
            recent_events: Mutex::new(VecDeque::with_capacity(MAX_RECENT_EVENTS)),
            counters: Mutex::new(GuardCounters::default()),
        }
    }

    /// Set an optional dataset recorder for offline ML dataset collection.
    pub fn with_dataset_recorder(mut self, recorder: Arc<DatasetRecorder>) -> Self {
        self.dataset_recorder = Some(recorder);
        self
    }

    /// Return the composition-owned recorder so a runtime event observer can
    /// be attached to the same dataset sink without creating a second file
    /// writer.
    pub fn dataset_recorder(&self) -> Option<Arc<DatasetRecorder>> {
        self.dataset_recorder.clone()
    }

    /// Install an optional local classifier. The default remains unavailable,
    /// preserving the deterministic Fast/Chain Guard path.
    pub fn with_classifier(mut self, classifier: Arc<dyn ChainRiskClassifier>) -> Self {
        self.classifier = classifier;
        self
    }

    /// Set the declared task scope for a given session.
    pub fn set_declared_scope(&self, session_id: &SessionId, scope: impl Into<String>) {
        let scope_str = scope.into();
        self.session_scopes.lock().insert(*session_id, scope_str);
    }

    /// Bind a trusted envelope before the provider starts tool use.
    pub fn bind_turn_intent(
        &self,
        session_id: SessionId,
        trace_id: impl Into<String>,
        intent: TaskIntentEnvelopeV1,
    ) {
        self.turn_intents
            .lock()
            .insert((session_id, trace_id.into()), intent);
    }

    pub fn turn_intent(
        &self,
        session_id: &SessionId,
        trace_id: &str,
    ) -> Option<TaskIntentEnvelopeV1> {
        self.turn_intents
            .lock()
            .get(&(*session_id, trace_id.to_string()))
            .cloned()
    }

    pub fn session_behavior_summary(&self, session_id: &SessionId) -> SessionBehaviorSummary {
        self.session_behavior_summary
            .lock()
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Retrieve a cloned snapshot of a behavior chain for a specific session and trace.
    pub fn chain_for_trace(&self, session_id: &SessionId, trace_id: &str) -> Option<BehaviorChain> {
        self.chains
            .lock()
            .get(&(*session_id, trace_id.to_string()))
            .cloned()
    }

    /// Retrieve bounded risk history for a session.
    pub fn session_risk_history(&self, session_id: &SessionId) -> Vec<TurnRiskSummary> {
        self.session_risk_history
            .lock()
            .get(session_id)
            .map(|h| h.summaries.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Retrieve the current status summary for desktop telemetry and gateway queries.
    pub fn status(&self) -> GuardStatusDto {
        let chains = self.chains.lock();
        let counters = self.counters.lock();
        let rec_enabled = self
            .dataset_recorder
            .as_ref()
            .map(|r| r.is_enabled())
            .unwrap_or(false);

        GuardStatusDto {
            enabled: true,
            fast_guard_active: true,
            chain_guard_active: true,
            intent_guard_active: true,
            cross_turn_monitoring_active: true,
            active_chains: chains.len(),
            total_evaluations: counters.total_evaluations,
            total_allowed: counters.total_allowed,
            total_denied: counters.total_denied,
            total_approval_required: counters.total_approval_required,
            dataset_recording_enabled: rec_enabled,
            ml_classifier_available: self.classifier.available(),
            ml_model_version: self.classifier.model_version(),
            feature_schema_version: crate::features_v2::AGENT_CHAIN_FEATURE_V2.to_string(),
            dataset_version: "guard-dataset-v3".to_string(),
        }
    }

    /// Retrieve the recent evaluation events (newest first).
    pub fn recent_events(&self, limit: Option<usize>) -> Vec<GuardEventDto> {
        let events = self.recent_events.lock();
        let limit = limit.unwrap_or(50).min(events.len());
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Clear memory for a completed session.
    pub fn clear_session(&self, session_id: &SessionId) {
        let mut chains = self.chains.lock();
        chains.retain(|(s, _), _| s != session_id);
        self.session_scopes.lock().remove(session_id);
        self.session_risk_history.lock().remove(session_id);
        self.session_behavior_summary.lock().remove(session_id);
        self.turn_intents
            .lock()
            .retain(|(session, _), _| session != session_id);
    }

    /// Dry-run evaluate an action without permanently recording it in the chain.
    pub fn dry_run(&self, req: &GuardDryRunRequest) -> GuardDryRunResponse {
        let obs = SafetyObservation {
            trace_id: "dry_run".to_string(),
            request_id: "dry_run".to_string(),
            session_id: req
                .session_id
                .clone()
                .unwrap_or_else(|| "dry_run".to_string()),
            stage: "capability_dispatch".to_string(),
            capability_id: req.capability_id.clone(),
            tool_name: req.capability_id.clone(),
            resource_classes: Vec::new(),
            source_classes: Vec::new(),
            sink_classes: Vec::new(),
            permission_scope: req.capability_id.clone(),
            approval_state: "dry_run".to_string(),
            argument_shape: "dry_run".to_string(),
            redacted_argument_features: req.arguments.clone(),
            result_class: None,
            result_size_bucket: None,
            retry_count: 0,
            denied_before: false,
            prior_actions: Vec::new(),
            external_effect: false,
            operation_class: apeireth_governance::OperationClass::Unknown,
            data_sensitivity: crate::observation::DataSensitivity::Unknown,
            persistent_effect: false,
            destructive_effect: false,
            requires_network: false,
            may_access_credentials: false,
            effect_fingerprint: "dry_run".to_string(),
        };

        let mut temp_chain = BehaviorChain::new("dry_run", "dry_run");
        if let Some(intent) = req.intent.clone() {
            temp_chain.set_intent(intent);
        }
        let fast_res = self
            .fast_guard
            .evaluate(&obs, req.declared_scope.as_deref());
        let decision = if fast_res.clear {
            GuardDecision::allow_fast()
        } else {
            self.chain_guard.evaluate(&temp_chain, &obs, &fast_res)
        };
        let features = AgentChainFeatureV2::from_chain(&temp_chain);
        let prediction = self.classifier.classify_v2(&features);
        let decision = DecisionFusion::fuse_v2(&decision, &fast_res, &prediction, &features);

        GuardDryRunResponse {
            decision: decision.decision.label().to_string(),
            stage: decision.stage,
            risk_score: decision.risk_score,
            reasons: decision.reasons,
            evidence: decision.evidence,
        }
    }

    fn evaluate_internal(
        &self,
        request: &GovernanceRequest<'_>,
    ) -> (GuardDecision, GovernanceVerdict) {
        let trace_id = request.trace.to_string();
        let key = (request.session, trace_id.clone());
        let mut chains = self.chains.lock();
        if chains.len() >= 256 {
            if let Some(oldest_key) = chains.keys().next().cloned() {
                chains.remove(&oldest_key);
            }
        }
        let intent = request
            .security_context
            .and_then(|context| {
                if context.trace_id.is_empty() || context.trace_id == trace_id {
                    context.intent.clone().map(|mut intent| {
                        intent.trace_id = trace_id.clone();
                        intent.session_id = request.session.to_string();
                        intent
                    })
                } else {
                    None
                }
            })
            .or_else(|| self.turn_intent(&request.session, &trace_id));
        if let Some(intent) = intent.clone() {
            self.turn_intents
                .lock()
                .insert((request.session, trace_id.clone()), intent);
        }
        let chain = chains.entry(key).or_insert_with(|| {
            let mut c = BehaviorChain::new(request.session.to_string(), trace_id.clone());
            if let Some(scope) = self.session_scopes.lock().get(&request.session) {
                c.set_declared_scope(scope.clone());
            }
            if let Some(intent) = intent.clone() {
                c.set_intent(intent);
            }
            c
        });
        if chain.intent.is_none() {
            if let Some(intent) = intent {
                chain.set_intent(intent);
            }
        }

        // Calculate retry stats and prior actions
        let actions = chain.actions();
        let denied_before = actions.iter().any(|a| a.denied);
        let retry_count = actions.iter().rev().take_while(|a| a.denied).count() as u32;
        let prior_actions = actions.iter().map(|a| a.capability_id.clone()).collect();

        // Extract normalized safety observation
        let obs = SafetyObservation::from_governance_request(
            request,
            retry_count,
            denied_before,
            prior_actions,
        );

        // Add action to behavior chain
        let action_id = chain.add_action_with_id(&obs, request.round, request.action_id);

        let alignment = IntentAlignmentGuard.evaluate(chain.intent.as_ref(), &obs);
        chain.set_action_alignment(&action_id, alignment.class);

        // Stage A: Fast Guard
        let fast_res = self
            .fast_guard
            .evaluate(&obs, chain.declared_task_scope.as_deref());

        // Stage B: Chain Guard (or early allow)
        let mut guard_decision = if fast_res.clear {
            GuardDecision::allow_fast()
        } else {
            self.chain_guard.evaluate(chain, &obs, &fast_res)
        };
        guard_decision = apply_alignment(guard_decision, &alignment);
        let summary = self.session_behavior_summary(&request.session);
        if summary.credential_probe_count >= 2 && obs.may_access_credentials {
            guard_decision.decision = Decision::deny("repeated sensitive probing across turns");
            guard_decision.risk_score = guard_decision.risk_score.max(0.95);
            guard_decision
                .reasons
                .push("cross_turn_sensitive_probing".to_string());
            guard_decision.evidence.push(
                "bounded session summary observed repeated credential/environment probes"
                    .to_string(),
            );
            guard_decision.stage = crate::decision::GuardStage::ChainGuard;
        }
        let features = AgentChainFeatureV2::from_chain_with_cross_turn(
            chain,
            CrossTurnRiskSummary {
                recent_turns: summary.recent_turns,
                denied_action_count: summary.denied_action_count,
                credential_probe_count: summary.credential_probe_count,
                sensitive_read_count: summary.sensitive_read_count,
                network_egress_count: summary.network_egress_count,
                repeated_scope_expansion_count: summary.repeated_scope_expansion_count,
                repeated_alternate_tool_count: summary.repeated_alternate_tool_count,
                risk_trend: summary.risk_trend,
            },
        );
        let prediction = self.classifier.classify_v2(&features);
        let guard_decision =
            DecisionFusion::fuse_v2(&guard_decision, &fast_res, &prediction, &features);

        // Update action status in chain
        let status = match &guard_decision.decision {
            Decision::Allow => ActionStatus::Allowed,
            Decision::Deny { .. } => ActionStatus::Denied,
            Decision::RequireApproval { .. } => ActionStatus::RequireApproval,
        };
        chain.update_action_status(&action_id, status);

        // Record turn risk summary for session continuity (without leaking full DAG)
        {
            let mut risk_histories = self.session_risk_history.lock();
            let hist = risk_histories.entry(request.session).or_default();
            hist.push(TurnRiskSummary {
                trace_id: request.trace.to_string(),
                max_risk_score: guard_decision.risk_score,
                denied: matches!(guard_decision.decision, Decision::Deny { .. }),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
            });
        }

        {
            let mut summary = self.session_behavior_summary.lock();
            let state = summary.entry(request.session).or_default();
            state.recent_turns = state.recent_turns.saturating_add(1).min(16);
            state.denied_action_count +=
                u32::from(matches!(guard_decision.decision, Decision::Deny { .. }));
            state.approval_rejection_count += u32::from(matches!(
                guard_decision.decision,
                Decision::RequireApproval { .. }
            ));
            state.credential_probe_count += u32::from(obs.may_access_credentials);
            state.sensitive_read_count +=
                u32::from(!obs.source_classes.is_empty() && obs.may_access_credentials);
            state.network_egress_count += u32::from(obs.requires_network && obs.external_effect);
            state.repeated_scope_expansion_count += u32::from(matches!(
                alignment.class,
                AlignmentClass::ScopeExpansion
                    | AlignmentClass::Contradictory
                    | AlignmentClass::HighRiskMismatch
            ));
            state.risk_trend =
                (state.risk_trend * 0.75 + guard_decision.risk_score * 0.25).clamp(0.0, 1.0);
        }

        // Update counters
        {
            let mut c = self.counters.lock();
            c.total_evaluations += 1;
            match &guard_decision.decision {
                Decision::Allow => c.total_allowed += 1,
                Decision::Deny { .. } => c.total_denied += 1,
                Decision::RequireApproval { .. } => c.total_approval_required += 1,
            }
        }

        // Record recent event for desktop observability
        {
            let event = GuardEventDto {
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                session_id: request.session.to_string(),
                trace_id: request.trace.to_string(),
                round: request.round,
                capability_id: obs.capability_id.clone(),
                stage: guard_decision.stage,
                decision: guard_decision.decision.label().to_string(),
                risk_score: guard_decision.risk_score,
                reasons: guard_decision.reasons.clone(),
                evidence: guard_decision.evidence.clone(),
            };

            let mut events = self.recent_events.lock();
            if events.len() >= MAX_RECENT_EVENTS {
                events.pop_front();
            }
            events.push_back(event);
        }

        // Record dataset record if recorder is configured
        if let Some(recorder) = &self.dataset_recorder {
            recorder.record_classification(&action_id, &obs, chain, &fast_res, &guard_decision);
        }

        let verdict = guard_decision.to_verdict(self.name());
        (guard_decision, verdict)
    }
}

#[async_trait]
impl GovernanceHook for BehaviorChainGuardHook {
    fn name(&self) -> &str {
        "behavior_chain_guard"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let (decision, _) = self.evaluate_internal(request);
        decision.decision
    }

    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        let (_, verdict) = self.evaluate_internal(request);
        verdict
    }
}
