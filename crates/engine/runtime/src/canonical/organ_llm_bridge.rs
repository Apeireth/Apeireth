//! Bridge from the plugin `LlmFactory` contract onto the runtime-owned
//! [`ModuleInvoker`].
//!
//! Organ implementations (W1 world model, W2 causal world model) consume LLMs
//! through the foundation-level `apeireth_plugin::llm_factory::LlmFactory`
//! trait. The frozen runtime exposes exactly one sanctioned way for a module
//! to spend an isolated model call: [`ModuleInvoker`], which enforces the
//! per-turn budget, the nesting depth limit, and canonical completion
//! governance (`Decision::Deny` refuses; `Decision::RequireApproval` fails
//! closed — an isolated side-call can never mint a hidden human approval).
//!
//! This adapter is the only sanctioned join between the two: it implements the
//! existing plugin `LlmFactory` trait over an injected `Arc<dyn ModuleInvoker>`
//! so that future organ wiring can route organ LLM calls through the canonical
//! provider path without a second provider abstraction.
//!
//! # What the adapter deliberately does not do
//!
//! - It does not own a provider router, a provider plugin, an HTTP client, a
//!   `Runtime`, a session store, a governance hook, or a tool executor. All
//!   authority stays with the injected invoker and, behind it, the runtime.
//! - It does not re-check governance. The invoker already owns that check;
//!   duplicating it here would create a second enforcement path.
//! - It does not add a parallel budget or invent token accounting. Whatever
//!   turn-scoped invoker the wiring injects carries the canonical
//!   `ModuleTurnState` budget; usage numbers are copied from the provider
//!   response unchanged.
//!
//! # Request mapping
//!
//! A plugin `CompletionRequest` (system prompt + message list) is mapped onto
//! one isolated `ModuleInvocationRequest` (system + single input + optional
//! model). A single user message is passed through verbatim; any other shape
//! is flattened with visible `[role]` prefixes so multi-message transcripts
//! remain legible to the model. Sampling parameters (`temperature`,
//! `max_tokens`) and tool declarations have no counterpart in the frozen
//! isolated-invocation ABI and are dropped: isolated side-calls never carry
//! tools, and sampling policy belongs to the runtime/provider layer, not to
//! side-call callers.
//!
//! # Error mapping (fail closed)
//!
//! Every invoker failure maps onto the plugin `LlmError::Provider` variant
//! with a stable prefix, so organ callers can tell the failure classes apart
//! while the adapter itself never fabricates a completion:
//!
//! - `Denied` → `governance denied isolated completion: …`
//! - `ApprovalRequired` → `isolated completion approval is not permitted
//!   (fail closed): …`
//! - `BudgetExceeded` / `RecursionLimit` / `NoModel` → `module invocation …`
//! - `Provider` → the upstream reason, unchanged.

use std::sync::Arc;

use apeireth_orchestration::SubagentRole;
use apeireth_plugin::llm_factory::{
    CompletionMessage, CompletionRequest, CompletionResponse, LlmError, LlmFactory, LlmInstance,
    TokenUsage,
};
use async_trait::async_trait;

use super::module::{ModuleInvocationError, ModuleInvocationRequest, ModuleInvoker};

/// Factory name reported by [`InvokerLlmFactory::name`].
pub const INVOKER_LLM_FACTORY_NAME: &str = "invoker-llm-factory";

/// `LlmFactory` over the runtime-owned isolated module invoker.
///
/// Construct with the turn-scoped invoker supplied by the future organ wiring;
/// the factory itself holds no runtime state beyond that handle.
#[derive(Clone)]
pub struct InvokerLlmFactory {
    invoker: Arc<dyn ModuleInvoker>,
}

impl std::fmt::Debug for InvokerLlmFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvokerLlmFactory")
            .field("invoker", &"Arc<dyn ModuleInvoker>")
            .finish()
    }
}

impl InvokerLlmFactory {
    /// Bridge the given canonical module invoker.
    pub fn new(invoker: Arc<dyn ModuleInvoker>) -> Self {
        Self { invoker }
    }
}

#[async_trait]
impl LlmFactory for InvokerLlmFactory {
    async fn spawn(
        &self,
        role: SubagentRole,
        model: &str,
    ) -> Result<Box<dyn LlmInstance>, LlmError> {
        // The canonical isolated-invocation ABI has no persistent instance
        // context: every `complete` is one isolated call. The instance is
        // therefore a stateless mapper carrying (role, model) for labelling.
        let model = if model.trim().is_empty() {
            None
        } else {
            Some(model.to_string())
        };
        Ok(Box::new(InvokerLlmInstance {
            invoker: Arc::clone(&self.invoker),
            role,
            model,
        }))
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        // The canonical invoker surface exposes no model discovery. An empty
        // list is the honest answer (mirrors `NoopLlmFactory`); callers must
        // select models explicitly at wiring time.
        Ok(Vec::new())
    }

    fn name(&self) -> &str {
        INVOKER_LLM_FACTORY_NAME
    }
}

/// One stateless organ-side LLM context backed by isolated module invocations.
pub struct InvokerLlmInstance {
    invoker: Arc<dyn ModuleInvoker>,
    role: SubagentRole,
    model: Option<String>,
}

impl std::fmt::Debug for InvokerLlmInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvokerLlmInstance")
            .field("role", &self.role)
            .field("model", &self.model)
            .finish()
    }
}

impl InvokerLlmInstance {
    fn build_request(&self, req: &CompletionRequest) -> ModuleInvocationRequest {
        let system = if req.system_prompt.trim().is_empty() {
            None
        } else {
            Some(req.system_prompt.clone())
        };
        let input = flatten_messages(&req.messages);
        let mut request = ModuleInvocationRequest {
            system,
            input,
            model: None,
        };
        if let Some(model) = &self.model {
            request = request.with_model(model.clone());
        }
        request
    }
}

/// Flatten the plugin message list onto the single isolated input string.
///
/// A lone user message passes through verbatim (the dominant organ shape).
/// Anything else keeps every message visible with a `[role] ` prefix so the
/// model can still tell turns apart.
fn flatten_messages(messages: &[CompletionMessage]) -> String {
    if messages.len() == 1 && messages[0].role == "user" {
        return messages[0].content.clone();
    }
    messages
        .iter()
        .map(|message| format!("[{}] {}", message.role, message.content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Map the invoker failure onto the plugin error channel, fail closed.
///
/// Governance refusals keep their identity in the message text so organ
/// callers can tell "the policy said no" apart from "the network said no".
fn map_invocation_error(error: ModuleInvocationError) -> LlmError {
    let reason = match error {
        ModuleInvocationError::Denied { reason } => {
            format!("governance denied isolated completion: {reason}")
        }
        ModuleInvocationError::ApprovalRequired { reason } => {
            format!("isolated completion approval is not permitted (fail closed): {reason}")
        }
        ModuleInvocationError::BudgetExceeded { limit } => {
            format!("module invocation budget exceeded (limit: {limit})")
        }
        ModuleInvocationError::RecursionLimit { depth, maximum } => {
            format!("module invocation recursion limit {depth} exceeds maximum {maximum}")
        }
        ModuleInvocationError::NoModel => "module invocation has no model".to_string(),
        ModuleInvocationError::Provider { reason } => reason,
    };
    LlmError::Provider(reason)
}

#[async_trait]
impl LlmInstance for InvokerLlmInstance {
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let request = self.build_request(&req);
        let response = self
            .invoker
            .invoke(request)
            .await
            .map_err(map_invocation_error)?;
        Ok(CompletionResponse {
            message: CompletionMessage {
                role: "assistant".to_string(),
                content: response.response.content,
            },
            // Isolated invocations never carry tool declarations, so the
            // provider cannot have produced tool calls on this path.
            tool_calls: Vec::new(),
            finish_reason: response
                .response
                .finish_reason
                .map(|reason| reason.to_openai().to_string())
                .unwrap_or_else(|| "stop".to_string()),
            usage: TokenUsage {
                prompt_tokens: response.response.usage.prompt_tokens,
                completion_tokens: response.response.usage.completion_tokens,
                total_tokens: response.response.usage.total_tokens,
            },
        })
    }

    fn name(&self) -> &str {
        INVOKER_LLM_FACTORY_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{CapabilityId, Clock, ModelId, SessionId, TraceId};
    use apeireth_governance::{Decision, GovernanceHook, GovernanceRequest};
    use apeireth_plugin::{ProviderCapability, ProviderError};
    use apeireth_protocol::canonical::{
        ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse,
    };
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::super::module::{ModuleInvocationResponse, ModuleTurnState};
    use super::super::provider::ProviderRouter;

    // ------------------------------------------------------------------
    // Test doubles
    // ------------------------------------------------------------------

    /// Scripted provider: returns one canned reply and counts invocations.
    struct ScriptedProvider {
        id: CapabilityId,
        reply: &'static str,
        completions: AtomicU32,
    }

    impl ScriptedProvider {
        fn new(reply: &'static str) -> Arc<Self> {
            Arc::new(Self {
                id: CapabilityId::new("provider.scripted").unwrap(),
                reply,
                completions: AtomicU32::new(0),
            })
        }

        fn completions(&self) -> u32 {
            self.completions.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl ProviderCapability for ScriptedProvider {
        fn id(&self) -> &CapabilityId {
            &self.id
        }

        fn models(&self) -> Vec<ModelDescriptor> {
            vec![ModelDescriptor::new(
                ModelId::new("test-model").unwrap(),
                self.id.clone(),
            )]
        }

        async fn complete(
            &self,
            _request: &NormalizedRequest,
        ) -> Result<NormalizedResponse, ProviderError> {
            self.completions.fetch_add(1, Ordering::SeqCst);
            Ok(NormalizedResponse::text(
                "scripted-1",
                "test-model",
                self.reply,
            ))
        }
    }

    /// Governance hook returning a fixed decision and counting evaluations.
    struct FixedDecisionHook {
        decision: Decision,
        evaluations: AtomicU32,
    }

    impl FixedDecisionHook {
        fn allow() -> Self {
            Self {
                decision: Decision::Allow,
                evaluations: AtomicU32::new(0),
            }
        }

        fn deny(reason: &str) -> Self {
            Self {
                decision: Decision::deny(reason),
                evaluations: AtomicU32::new(0),
            }
        }

        fn require_approval(reason: &str) -> Self {
            Self {
                decision: Decision::require_approval(reason),
                evaluations: AtomicU32::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl GovernanceHook for FixedDecisionHook {
        fn name(&self) -> &str {
            "fixed-decision"
        }

        async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
            self.evaluations.fetch_add(1, Ordering::SeqCst);
            self.decision.clone()
        }
    }

    fn virtual_clock() -> Arc<dyn Clock> {
        Arc::new(apeireth_core::clock::VirtualClock::new(
            chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        ))
    }

    /// Real canonical chain: the crate-internal per-turn invoker over a real
    /// router. The invoker borrows the router and the governance hook, so the
    /// test leaks them to `'static` to satisfy the adapter's
    /// `Arc<dyn ModuleInvoker>` contract. Test scaffolding only.
    fn real_invoker(
        provider: &Arc<ScriptedProvider>,
        governance: FixedDecisionHook,
    ) -> (Arc<dyn ModuleInvoker>, &'static FixedDecisionHook) {
        let governance: &'static FixedDecisionHook = Box::leak(Box::new(governance));
        let router: &'static ProviderRouter = Box::leak(Box::new(ProviderRouter::new(
            vec![provider.clone()],
            virtual_clock(),
        )));
        let invoker = super::super::module::RuntimeModuleInvoker::new(
            router,
            governance,
            SessionId::new(),
            TraceId::new(),
            "test-model",
            "test.organ_llm_bridge",
            Arc::new(ModuleTurnState::new(8)),
            0,
        );
        (Arc::new(invoker), governance)
    }

    fn user_request(prompt: &str) -> CompletionRequest {
        CompletionRequest {
            system_prompt: "you are a test fixture".into(),
            messages: vec![CompletionMessage {
                role: "user".into(),
                content: prompt.into(),
            }],
            temperature: 0.3,
            tools: vec![],
            max_tokens: Some(64),
        }
    }

    // ------------------------------------------------------------------
    // A. normal completion
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn organ_factory_call_reaches_scripted_provider_through_canonical_path() {
        let provider = ScriptedProvider::new("counterfactual: 42");
        let (invoker, governance) = real_invoker(&provider, FixedDecisionHook::allow());
        let factory = InvokerLlmFactory::new(invoker);

        let mut instance = factory
            .spawn(SubagentRole::Reviewer, "test-model")
            .await
            .expect("spawn must succeed");
        let response = instance
            .complete(user_request("what if the master asks for 42?"))
            .await
            .expect("completion must succeed");

        assert_eq!(response.message.role, "assistant");
        assert_eq!(response.message.content, "counterfactual: 42");
        assert!(response.tool_calls.is_empty());
        assert_eq!(response.finish_reason, "stop");
        assert_eq!(provider.completions(), 1, "exactly one provider call");
        assert_eq!(
            governance.evaluations.load(Ordering::SeqCst),
            1,
            "the invoker, not the adapter, ran completion governance"
        );
        assert_eq!(factory.name(), INVOKER_LLM_FACTORY_NAME);
    }

    /// The model override must survive the adapter: the scripted provider only
    /// supports `test-model`, so a bogus override fails at routing.
    #[tokio::test]
    async fn model_override_is_forwarded_to_the_invoker() {
        let provider = ScriptedProvider::new("ok");
        let (invoker, _governance) = real_invoker(&provider, FixedDecisionHook::allow());
        let factory = InvokerLlmFactory::new(invoker);

        let mut instance = factory
            .spawn(SubagentRole::Planner, "unsupported-model")
            .await
            .expect("spawn itself must succeed");
        let error = instance
            .complete(user_request("hi"))
            .await
            .expect_err("unsupported model must fail at the router");
        match error {
            LlmError::Provider(reason) => {
                assert!(reason.contains("no provider"), "got: {reason}");
            }
            other => panic!("expected provider error, got {other:?}"),
        }
        assert_eq!(provider.completions(), 0);
    }

    // ------------------------------------------------------------------
    // B. governance deny
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn governance_deny_yields_zero_provider_calls_and_fail_closed_error() {
        let provider = ScriptedProvider::new("should never happen");
        let (invoker, governance) = real_invoker(
            &provider,
            FixedDecisionHook::deny("organ completions are disabled by policy"),
        );
        let factory = InvokerLlmFactory::new(invoker);

        let mut instance = factory
            .spawn(SubagentRole::Reviewer, "test-model")
            .await
            .expect("spawn must succeed (spawn performs no model call)");
        let error = instance
            .complete(user_request("hello"))
            .await
            .expect_err("denied completion must fail closed");

        match error {
            LlmError::Provider(reason) => {
                assert!(
                    reason.starts_with("governance denied isolated completion:"),
                    "deny must keep its governance identity, got: {reason}"
                );
                assert!(reason.contains("disabled by policy"));
            }
            other => panic!("expected provider error, got {other:?}"),
        }
        assert_eq!(
            provider.completions(),
            0,
            "a denied completion must never reach a provider"
        );
        assert_eq!(governance.evaluations.load(Ordering::SeqCst), 1);
    }

    // ------------------------------------------------------------------
    // C. RequireApproval fails closed, no hidden approval
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn require_approval_fails_closed_with_zero_provider_calls() {
        let provider = ScriptedProvider::new("should never happen");
        let (invoker, _governance) = real_invoker(
            &provider,
            FixedDecisionHook::require_approval("escalation needed"),
        );
        let factory = InvokerLlmFactory::new(invoker.clone());

        let mut instance = factory
            .spawn(SubagentRole::Reviewer, "test-model")
            .await
            .expect("spawn must succeed");
        // The canonical invoker refuses without minting an approval: the
        // failure is `ApprovalRequired`, never a pending approval record.
        let direct = invoker
            .invoke(ModuleInvocationRequest::isolated("system", "input"))
            .await
            .expect_err("require-approval must fail closed at the invoker");
        assert!(
            matches!(direct, ModuleInvocationError::ApprovalRequired { .. }),
            "expected ApprovalRequired, got {direct:?}"
        );

        // And the adapter maps it to a fail-closed plugin error.
        let error = instance
            .complete(user_request("hello"))
            .await
            .expect_err("require-approval must fail closed through the adapter");
        match error {
            LlmError::Provider(reason) => {
                assert!(
                    reason.starts_with(
                        "isolated completion approval is not permitted (fail closed):"
                    ),
                    "got: {reason}"
                );
            }
            other => panic!("expected provider error, got {other:?}"),
        }
        assert_eq!(
            provider.completions(),
            0,
            "require-approval must never reach a provider"
        );
    }

    // ------------------------------------------------------------------
    // Error-mapping unit coverage (every ModuleInvocationError variant)
    // ------------------------------------------------------------------

    #[test]
    fn every_invocation_error_maps_to_fail_closed_provider_error() {
        let cases: Vec<(ModuleInvocationError, &str)> = vec![
            (
                ModuleInvocationError::Denied {
                    reason: "policy".into(),
                },
                "governance denied isolated completion: policy",
            ),
            (
                ModuleInvocationError::ApprovalRequired {
                    reason: "escalate".into(),
                },
                "isolated completion approval is not permitted (fail closed): escalate",
            ),
            (
                ModuleInvocationError::BudgetExceeded { limit: 8 },
                "module invocation budget exceeded (limit: 8)",
            ),
            (
                ModuleInvocationError::RecursionLimit {
                    depth: 2,
                    maximum: 1,
                },
                "module invocation recursion limit 2 exceeds maximum 1",
            ),
            (
                ModuleInvocationError::NoModel,
                "module invocation has no model",
            ),
            (
                ModuleInvocationError::Provider {
                    reason: "upstream 500".into(),
                },
                "upstream 500",
            ),
        ];
        for (error, expected) in cases {
            match map_invocation_error(error) {
                LlmError::Provider(reason) => assert_eq!(reason, expected),
                other => panic!("expected fail-closed provider error, got {other:?}"),
            }
        }
    }

    // ------------------------------------------------------------------
    // Request mapping
    // ------------------------------------------------------------------

    #[test]
    fn lone_user_message_passes_through_verbatim_and_multi_turn_is_prefixed() {
        let single = vec![CompletionMessage {
            role: "user".into(),
            content: "plain question".into(),
        }];
        assert_eq!(flatten_messages(&single), "plain question");

        let multi = vec![
            CompletionMessage {
                role: "user".into(),
                content: "first".into(),
            },
            CompletionMessage {
                role: "assistant".into(),
                content: "second".into(),
            },
        ];
        assert_eq!(
            flatten_messages(&multi),
            "[user] first\n\n[assistant] second"
        );
    }

    #[test]
    fn empty_spawn_model_inherits_the_turn_model() {
        // An empty model must reach the invoker as `model: None` (inherit),
        // not as `Some("")`, which the invoker rejects with NoModel.
        let request = InvokerLlmInstance {
            invoker: no_invoker(),
            role: SubagentRole::Reviewer,
            model: None,
        }
        .build_request(&user_request("hi"));
        assert!(request.model.is_none());

        let request = InvokerLlmInstance {
            invoker: no_invoker(),
            role: SubagentRole::Reviewer,
            model: Some("test-model".into()),
        }
        .build_request(&user_request("hi"));
        assert_eq!(request.model.as_deref(), Some("test-model"));
    }

    fn no_invoker() -> Arc<dyn ModuleInvoker> {
        // The request-building paths under test never invoke; a panicking
        // invoker proves that.
        struct Unreachable;
        #[async_trait::async_trait]
        impl ModuleInvoker for Unreachable {
            async fn invoke(
                &self,
                _request: ModuleInvocationRequest,
            ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
                panic!("build_request must not invoke");
            }
        }
        Arc::new(Unreachable)
    }

    // ------------------------------------------------------------------
    // D. no raw provider ownership (source guard over the impl section)
    // ------------------------------------------------------------------

    /// The adapter impl must not name, construct, or reach around the
    /// canonical boundary. Only the `ModuleInvoker` trait surface is allowed.
    #[test]
    fn adapter_impl_owns_no_raw_provider_authority() {
        let source = include_str!("organ_llm_bridge.rs");
        // Guard only the implementation; the test module below legitimately
        // builds the real chain to prove the path end to end.
        let impl_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("impl section before tests");
        let impl_source = strip_comments(impl_source);

        for forbidden in [
            "reqwest",
            "Command::new",
            "std::process",
            "ProviderRouter",
            "ProviderCapability",
            "GovernanceHook",
            "RuntimeBuilder",
            "ModuleRegistry",
            "SessionStore",
            "ToolCapability",
            "authorize_isolated_completion",
        ] {
            assert!(
                !impl_source.contains(forbidden),
                "adapter impl must not reference {forbidden:?}; it may only use the ModuleInvoker surface"
            );
        }
    }

    fn strip_comments(source: &str) -> String {
        source
            .lines()
            .map(|line| {
                let code = line.split("//").next().unwrap_or("");
                code.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
