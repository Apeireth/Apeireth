//! W1/W2 organ compatibility with the canonical `InvokerLlmFactory` bridge.
//!
//! Proves that the existing organ constructors accept the runtime-owned
//! `InvokerLlmFactory` (plugin `LlmFactory` over `Arc<dyn ModuleInvoker>`)
//! without any algorithm change, and that a real W2 LLM proposal round rides
//! the adapter end to end. The organ algorithms and their `LlmFactory`
//! contract are untouched; only the factory handed in differs.

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_orchestration::SubagentRole;
use apeireth_plugin::llm_factory::{CompletionRequest, LlmError, LlmFactory};
use apeireth_protocol::canonical::NormalizedResponse;
use apeireth_runtime::canonical::{
    InvokerLlmFactory, ModuleInvocationError, ModuleInvocationRequest, ModuleInvocationResponse,
    ModuleInvoker,
};

use apeireth_organ::causal_world_model::{
    CausalWorldModelOrgan, EdgeProposalRequest, EdgeSource, ProposeCausalEdges, TimelineFact,
};
use apeireth_organ::world_model::WorldModelOrgan;
use apeireth_organ::{OrganKind, OrganTrait};

/// Minimal scripted invoker: records the requests it receives and returns a
/// fixed body. This stands in for the per-turn `RuntimeModuleInvoker` the
/// future organ wiring will inject.
struct ScriptedInvoker {
    body: &'static str,
    invocations: std::sync::atomic::AtomicUsize,
}

impl ScriptedInvoker {
    fn new(body: &'static str) -> Arc<Self> {
        Arc::new(Self {
            body,
            invocations: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    fn invocations(&self) -> usize {
        self.invocations.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl ModuleInvoker for ScriptedInvoker {
    async fn invoke(
        &self,
        request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
        self.invocations
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // The adapter must forward the organ system prompt verbatim and the
        // selected model through the canonical request shape.
        assert!(
            request
                .system
                .as_deref()
                .unwrap_or_default()
                .contains("因果图边提议器"),
            "organ system prompt must survive the bridge"
        );
        assert_eq!(request.model.as_deref(), Some("test-model"));
        Ok(ModuleInvocationResponse {
            response: NormalizedResponse::text("scripted-1", "test-model", self.body),
            served_by: CapabilityId::new("provider.scripted").unwrap(),
        })
    }
}

fn edge_proposal_json() -> &'static str {
    r#"[
        {"from": "rain|causes|wet", "to": "wet|causes|slippery", "predicate": "makes ground slippery", "confidence": 0.8, "evidence_strength": 3}
    ]"#
}

/// W1: the existing constructor consumes the adapter unchanged.
#[test]
fn world_model_organ_accepts_invoker_llm_factory() {
    let invoker = ScriptedInvoker::new("unused by the constructor");
    let factory: Arc<dyn apeireth_plugin::llm_factory::LlmFactory> =
        Arc::new(InvokerLlmFactory::new(invoker));
    let organ = WorldModelOrgan::new(factory, "test-model");
    assert_eq!(organ.organ_id(), OrganKind::W1);
    assert_eq!(organ.name(), "W1 World Model");
}

/// W2: the existing constructor consumes the adapter unchanged.
#[test]
fn causal_world_model_organ_accepts_invoker_llm_factory() {
    let invoker = ScriptedInvoker::new("unused by the constructor");
    let factory: Arc<dyn apeireth_plugin::llm_factory::LlmFactory> =
        Arc::new(InvokerLlmFactory::new(invoker));
    let organ = CausalWorldModelOrgan::new(factory, "test-model");
    assert_eq!(organ.organ_id(), OrganKind::W2);
    assert_eq!(organ.name(), "W2 Causal World Model");
}

/// Spawn honours the factory contract: empty model → inherit at invoke time,
/// and `available_models` stays honest (no invented model discovery).
#[tokio::test]
async fn factory_spawn_contract_holds_over_the_bridge() {
    let invoker = ScriptedInvoker::new("unused here");
    let factory = InvokerLlmFactory::new(invoker);

    let instance = factory
        .spawn(SubagentRole::Reviewer, "")
        .await
        .expect("empty model spawns (inherits at invoke time)");
    assert_eq!(instance.name(), "invoker-llm-factory");

    assert!(factory
        .available_models()
        .await
        .expect("available_models is honest on the bridge")
        .is_empty());
}

/// Real organ algorithm through the bridge: W2 edge proposal over a scripted
/// invoker reply, exactly the shape `ProposeCausalEdges` parses.
#[tokio::test]
async fn w2_edge_proposal_runs_through_the_bridge() {
    let invoker = ScriptedInvoker::new(edge_proposal_json());
    let factory: Arc<dyn apeireth_plugin::llm_factory::LlmFactory> =
        Arc::new(InvokerLlmFactory::new(invoker.clone()));
    let proposer = ProposeCausalEdges::new(factory, "test-model");

    let edges = proposer
        .llm_suggest(&EdgeProposalRequest {
            facts: vec![TimelineFact {
                chain: "rain|causes|wet".into(),
                subject: "rain".into(),
                predicate: "causes".into(),
                object: "wet".into(),
                valid_at: 0,
                invalid_at: None,
                importance: 5,
            }],
            max_proposals: 3,
        })
        .await
        .expect("edge proposal must succeed over the bridge");

    assert_eq!(edges.len(), 1, "scripted proposal parsed");
    assert_eq!(edges[0].source, EdgeSource::LlmProposed);
    assert!(
        (edges[0].weight - 0.8).abs() < 1e-6,
        "confidence 0.8 (f32) widens on the f64 edge weight, got {}",
        edges[0].weight
    );
    assert_eq!(edges[0].evidence_count, 3);
    assert_eq!(invoker.invocations(), 1, "exactly one isolated invocation");
}

/// A deny outcome surfaces as a structured fail-closed organ-side error; the
/// adapter never fabricates a completion out of a governance refusal.
#[tokio::test]
async fn denied_invocation_maps_to_fail_closed_llm_error() {
    struct DenyInvoker;
    #[async_trait::async_trait]
    impl ModuleInvoker for DenyInvoker {
        async fn invoke(
            &self,
            _request: ModuleInvocationRequest,
        ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
            Err(ModuleInvocationError::Denied {
                reason: "organ completions disabled".into(),
            })
        }
    }

    let factory = InvokerLlmFactory::new(Arc::new(DenyInvoker));
    let instance = factory
        .spawn(SubagentRole::Reviewer, "test-model")
        .await
        .expect("spawn performs no model call");
    let error = instance
        .complete(CompletionRequest {
            system_prompt: "s".into(),
            messages: vec![apeireth_plugin::llm_factory::CompletionMessage {
                role: "user".into(),
                content: "c".into(),
            }],
            temperature: 0.0,
            tools: vec![],
            max_tokens: None,
        })
        .await
        .expect_err("denied completion must fail closed");
    match error {
        LlmError::Provider(reason) => {
            assert!(reason.contains("governance denied isolated completion"));
            assert!(reason.contains("organ completions disabled"));
        }
        other => panic!("expected fail-closed provider error, got {other:?}"),
    }
}
