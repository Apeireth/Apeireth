//! The ONE canonical organ-ownership module.
//!
//! This is the first post-freeze organ integration. It gives the 9 existing
//! `OrganTrait` implementations a single production owner without turning
//! organs into modules and without adding a second runtime, registry, bus, or
//! provider path:
//!
//! ```text
//! Main Loop → ModuleRegistry → OrganModule → OrganOrchestrator → 9 organs
//! ```
//!
//! # Ownership rules enforced here
//!
//! - Long-lived state: the seven deterministic organs (E4/F1/F4/F6/W3/E7/Memory)
//!   and the `OrganOrchestrator` backend. W1/W2 sit as explicit `NoopOrgan`
//!   placeholders in the orchestrator's persistent slots — the AfterTurn path
//!   never uses them.
//! - Transient per invocation: W1/W2 need the current turn's LLM factory, so
//!   they are constructed inside the AfterTurn hook from
//!   `ctx.invoker_handle()` and dropped before the hook returns. This module
//!   struct holds no `ModuleInvoker`, no `InvokerLlmFactory`, no router, no
//!   governance hook, no session store — a turn-scoped handle must die with
//!   its invocation.
//! - AfterTurn only: the organ chain observes the committed turn; it never
//!   touches BeforeModelCall/AfterModelResponse/BeforeFinalCommit, never
//!   mutates the primary transcript, never emits frontend events, and never
//!   runs the proactive `tick()` path.
//! - Fail-open enhancement: organ cognition cannot block or amend the already
//!   committed reply. Individual organ failures are isolated inside the chain;
//!   governance denials of the LLM side-calls surface as ordinary organ
//!   failures. The hook always returns `Continue`.
//!
//! The council dependency of `OrganOrchestrator` is kept dormant: the AfterTurn
//! chain never deliberates, so the injected invoker fails closed if anything
//! ever tried to consult it. No new LLM route and no duplicate CouncilModule.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{Clock, Episode};
use apeireth_orchestration::{Advisor, Council, CouncilCallError, CouncilInvoker, Proposal};
use apeireth_organ::causal_world_model::CausalWorldModelOrgan;
use apeireth_organ::causal_world_model_edges::EdgeMinerOrgan;
use apeireth_organ::curiosity::CuriosityOrgan;
use apeireth_organ::emergence::EmergenceOrgan;
use apeireth_organ::emotion_memory::EmotionOrgan;
use apeireth_organ::hypothesis::HypothesisOrgan;
use apeireth_organ::memory::MemoryMergerOrgan;
use apeireth_organ::value_cases::ValueCasesOrgan;
use apeireth_organ::world_model::WorldModelOrgan;
use apeireth_organ::{NoopOrgan, OrganKind, OrganTrait};
use apeireth_plugin::llm_factory::{LlmFactory, NoopLlmFactory};
use apeireth_plugin::organ::OrganInput;
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use async_trait::async_trait;

use super::module::{
    AgentModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use super::orchestrator::{
    LocalOrchestratorRelationship, OrchestratorBoundaries, OrchestratorLoopConfig,
    OrganOrchestrator,
};

/// Stable module id (matches the `cognitive.*` slot family).
pub const ORGAN_MODULE_ID: &str = "cognitive.organs";

/// Observation ring capacity: enough for diagnosis, bounded forever.
const MAX_OBSERVATIONS: usize = 128;

/// One executed organ observation (diagnostic, non-sensitive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganModuleObservation {
    /// Hook the observation came from (always `"AfterTurn"` today).
    pub hook: String,
    /// The real session id the turn ran under.
    pub session_id: String,
    /// Whether the chain produced all nine organ outputs (failures are
    /// isolated into placeholder outputs, so this is presence, not success).
    pub organs_all_present: bool,
}

/// Dormant council invoker: the AfterTurn organ chain never deliberates.
///
/// If any future code path tried to consult the council through this module,
/// it would fail closed here instead of silently allowing.
struct DormantCouncilInvoker;

#[async_trait::async_trait]
impl CouncilInvoker for DormantCouncilInvoker {
    async fn invoke(
        &self,
        _advisor: Arc<dyn Advisor>,
        _proposal: &Proposal,
    ) -> Result<apeireth_orchestration::AdvisorVerdict, CouncilCallError> {
        Err(CouncilCallError::Provider(
            "organ module council is dormant: the AfterTurn organ chain never deliberates"
                .to_string(),
        ))
    }
}

/// The single organ-ownership module.
///
/// See the module docs for the long-lived vs transient split. The struct holds
/// persistent organ cognition state only; every turn-scoped capability enters
/// through the hook's `ModuleContext` and dies with the hook.
pub struct OrganModule {
    manifest: ModuleManifest,
    orchestrator: OrganOrchestrator<LocalOrchestratorRelationship>,
    clock: Arc<dyn Clock>,
    observations: Mutex<Vec<OrganModuleObservation>>,
}

impl OrganModule {
    /// Build the module with default organ configurations.
    ///
    /// The LLM factories handed to the currently-LLM-free organ constructors
    /// (E4/F4/F6/W3/E7) are the plugin `NoopLlmFactory`: those algorithms never
    /// call it today, and if a future change tried to, it would fail closed
    /// rather than smuggle in a second LLM path.
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
        let reserved_model = "reserved-organ-llm-not-called";
        let orchestrator = OrganOrchestrator::new(
            Arc::new(CuriosityOrgan::new(noop.clone(), reserved_model)),
            Arc::new(EmotionOrgan::new()),
            Arc::new(HypothesisOrgan::new(noop.clone(), reserved_model)),
            Arc::new(ValueCasesOrgan::new(noop.clone(), reserved_model)),
            // Persistent W1/W2 slots are deliberately Noop: the AfterTurn path
            // builds transient, turn-scoped W1/W2 per invocation instead.
            Arc::new(NoopOrgan::new(OrganKind::W1)),
            Arc::new(NoopOrgan::new(OrganKind::W2)),
            Arc::new(EdgeMinerOrgan::new(noop.clone(), reserved_model)),
            Arc::new(EmergenceOrgan::new(noop.clone(), reserved_model)),
            Arc::new(MemoryMergerOrgan::with_default()),
            Arc::new(Council::default_allow()),
            Arc::new(DormantCouncilInvoker),
            Arc::new(parking_lot::Mutex::new(
                super::orchestrator::LocalSovereignty::default(),
            )),
            LocalOrchestratorRelationship::default(),
            OrchestratorBoundaries::default(),
            OrchestratorLoopConfig::default(),
            clock.clone(),
        );
        Self {
            manifest: ModuleManifest::new(ORGAN_MODULE_ID, "Organ cognition"),
            orchestrator,
            clock,
            observations: Mutex::new(Vec::new()),
        }
    }

    /// Diagnostic observations of executed organ chains (bounded ring).
    pub fn observations(&self) -> Vec<OrganModuleObservation> {
        self.observations
            .lock()
            .expect("organ observations")
            .clone()
    }

    /// Derive the turn's organ input from the canonical context.
    ///
    /// The episode is the committed assistant reply for this session; the last
    /// user message travels as a context hint. Nothing is invented: the
    /// session id comes from the canonical context and the timestamp from the
    /// injected clock.
    fn turn_input(&self, ctx: &ModuleContext<'_>) -> OrganInput {
        let mut hints: Vec<String> = Vec::new();
        for message in ctx.messages.iter().rev() {
            if message.role == MessageRole::User {
                let text = ContentPart::join_text(&message.content);
                if !text.is_empty() {
                    hints.push(text);
                }
                break;
            }
        }
        let content = ctx
            .candidate
            .map(|candidate| candidate.content.clone())
            .unwrap_or_default();
        let timestamp = self.clock.now().timestamp_millis();
        let episode = Episode {
            id: format!("organ-afterturn-{}-{timestamp}", ctx.session_id),
            session_id: ctx.session_id.to_string(),
            role: "assistant".into(),
            content,
            timestamp,
        };
        OrganInput::new(episode, hints)
    }
}

#[async_trait]
impl AgentModule for OrganModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        // AfterTurn only: post-turn cognition over the committed reply.
        if hook != HookPoint::AfterTurn {
            return Ok(ModuleOutcome::continue_());
        }
        let input = self.turn_input(ctx);

        // Ephemeral, turn-scoped LLM path: handle → factory → transient
        // W1/W2. Everything in this block dies before the hook returns.
        let factory: Arc<dyn LlmFactory> = Arc::new(
            super::organ_llm_bridge::InvokerLlmFactory::new(ctx.invoker_handle()),
        );
        let w1: Arc<dyn OrganTrait> =
            Arc::new(WorldModelOrgan::new(factory.clone(), ctx.model.to_string()));
        let w2: Arc<dyn OrganTrait> = Arc::new(CausalWorldModelOrgan::new(
            factory.clone(),
            ctx.model.to_string(),
        ));
        let outputs = self
            .orchestrator
            .chain_9_organs_with_transient_llm(input, w1, w2)
            .await;
        // w1, w2, factory and the invoker handle drop here.

        {
            let mut observations = self.observations.lock().expect("organ observations");
            if observations.len() == MAX_OBSERVATIONS {
                observations.remove(0);
            }
            observations.push(OrganModuleObservation {
                hook: format!("{hook:?}"),
                session_id: ctx.session_id.to_string(),
                organs_all_present: outputs.all_present(),
            });
        }

        // Fail-open: organ cognition is post-commit enhancement. Chain-internal
        // organ failures were already isolated by the orchestrator; governance
        // refusals of the side-calls surface as those isolated organ failures.
        // The committed reply is never retried, amended, or blocked here.
        Ok(ModuleOutcome::continue_())
    }
}
