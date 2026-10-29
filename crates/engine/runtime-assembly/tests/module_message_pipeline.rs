//! Real registered dispatch without a main Provider turn.
#[path = "support/system_one.rs"]
mod review_support;
#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{CapabilityId, ModuleInstanceId};
use apeireth_plugin::module_contract::{
    ContractTool, EffectClass, ExecutionMode, ModuleToolContract, ToolSurface,
};
use apeireth_plugin::module_messages::*;
use apeireth_plugin::{
    Plugin, PluginContext, PluginManifest, PluginResult, ToolCapability, ToolExecutionContext,
    ToolModule,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use apeireth_runtime::canonical::module_messages::{ModuleMessageGrant, ModuleMessagePolicy};
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant};
use async_trait::async_trait;
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use support::*;

fn cid(id: &str) -> CapabilityId {
    CapabilityId::new(id).unwrap()
}
#[derive(Default)]
struct State {
    port: Mutex<Option<Arc<dyn ModuleMessages>>>,
    instance: Mutex<Option<ModuleInstanceId>>,
    contexts: Mutex<Vec<ToolExecutionContext>>,
    calls: AtomicUsize,
    hold: Mutex<Option<Arc<tokio::sync::Notify>>>,
    entered: tokio::sync::Notify,
    relay: Mutex<Option<ModuleMessageRequest>>,
    path: Mutex<Option<std::path::PathBuf>>,
}
impl State {
    fn port(&self) -> Arc<dyn ModuleMessages> {
        self.port.lock().unwrap().clone().unwrap()
    }
}
struct Endpoint {
    id: CapabilityId,
    name: String,
    state: Arc<State>,
}
#[async_trait]
impl ToolCapability for Endpoint {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        let mut d = NormalizedTool::new(&self.name);
        d.parameters = json!({"type":"object","properties":{"value":{"type":"string"}},"additionalProperties":false}).as_object().unwrap().clone();
        d
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::permanent_error(&call.id, "scoped entry required").with_name(&call.name)
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        _: Option<&apeireth_plugin::FrozenInvocation>,
        context: &ToolExecutionContext,
    ) -> ToolResult {
        self.state.contexts.lock().unwrap().push(context.clone());
        self.state.entered.notify_one();
        let hold = self.state.hold.lock().unwrap().clone();
        if let Some(hold) = hold {
            hold.notified().await;
        }
        let relay = self.state.relay.lock().unwrap().clone();
        if let Some(request) = relay {
            return match self.state.port().send(request).await {
                Ok(reply) => {
                    let result = reply.result.unwrap();
                    if result.is_ok() {
                        ToolResult::ok(&call.id, json!({"downstream_ok":true}))
                            .with_name(&call.name)
                    } else {
                        ToolResult::permanent_error(&call.id, result.render()).with_name(&call.name)
                    }
                }
                Err(error) => ToolResult::permanent_error(&call.id, format!("{:?}", error.code))
                    .with_name(&call.name),
            };
        }
        let path = self.state.path.lock().unwrap().clone();
        if let Some(path) = path {
            std::fs::write(path, call.arguments["value"].as_str().unwrap_or("effect")).unwrap();
        }
        self.state.calls.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(
            &call.id,
            json!({"state":"ready","private_marker":"receiver_private"}),
        )
        .with_name(&call.name)
    }
}
struct Module {
    inner: ToolModule,
    state: Arc<State>,
}
impl Module {
    fn new(name: &str, endpoints: &[(&str, ModuleToolContract)], state: Arc<State>) -> Arc<Self> {
        let tools = endpoints
            .iter()
            .map(|(name, contract)| {
                ContractTool::wrap(
                    Arc::new(Endpoint {
                        id: cid(&format!("tool.{name}")),
                        name: (*name).into(),
                        state: state.clone(),
                    }),
                    contract.clone(),
                )
                .unwrap()
            })
            .collect();
        Arc::new(Self {
            inner: ToolModule::new(pid(name), tools).unwrap(),
            state,
        })
    }
}
#[async_trait]
impl Plugin for Module {
    fn manifest(&self) -> &PluginManifest {
        self.inner.manifest()
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.inner.tools()
    }
    fn uses_tool_events_v1(&self) -> bool {
        true
    }
    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        *self.state.port.lock().unwrap() = ctx.agents.as_ref().and_then(|a| a.messages());
        *self.state.instance.lock().unwrap() = ctx.module_instance();
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
}
fn logical(effect: EffectClass) -> ModuleToolContract {
    ModuleToolContract::logical(ExecutionMode::Direct, effect)
}
fn policy(routes: &[(&str, &str, &[&str])]) -> AgentHostPolicy {
    let mut p = AgentHostPolicy {
        message_policy: Some(ModuleMessagePolicy::default()),
        ..Default::default()
    };
    for (source, target, tools) in routes {
        let grant = p
            .message_policy
            .as_mut()
            .unwrap()
            .grants
            .entry(pid(source))
            .or_insert_with(|| ModuleMessageGrant {
                authorization_ref: format!("host:{source}"),
                allow_background: true,
                allow_events: true,
                ..Default::default()
            });
        for tool in *tools {
            grant
                .targets
                .entry(pid(target))
                .or_default()
                .insert(cid(&format!("tool.{tool}")));
            p.modules
                .entry(pid(source))
                .or_insert_with(|| ModuleAgentGrant {
                    max_instances: 4,
                    ..Default::default()
                })
                .tools
                .insert(cid(&format!("tool.{tool}")));
        }
    }
    p
}
fn request(target: &str, name: &str) -> ModuleMessageRequest {
    ModuleMessageRequest::command(
        pid(target),
        cid(&format!("tool.{name}")),
        json!({"value":"actual_effect"}),
    )
}

#[tokio::test]
async fn command_reply_uses_authenticated_generations_gate_and_one_private_fact_without_main_reasoning(
) {
    let a = Arc::new(State::default());
    let b = Arc::new(State::default());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("effect");
    *b.path.lock().unwrap() = Some(path.clone());
    let provider = Provider::new(vec![]);
    let (gate, audit) = review_support::allow_gate();
    let runtime = builder(provider.clone())
        .with_system_one(gate)
        .with_plugin(Module::new("module.a", &[], a.clone()))
        .with_plugin(Module::new(
            "module.b",
            &[("write", logical(EffectClass::Protected))],
            b.clone(),
        ))
        .build_agent_runtime(policy(&[("module.a", "module.b", &["write"])]))
        .await
        .unwrap();
    let reply = a.port().send(request("module.b", "write")).await.unwrap();
    assert!(reply.result.as_ref().unwrap().is_ok(), "{:?}", reply.result);
    assert_eq!(std::fs::read_to_string(path).unwrap(), "actual_effect");
    assert_eq!(provider.requests().len(), 0);
    assert_eq!(audit.records().len(), 1);
    assert_eq!(b.calls.load(Ordering::SeqCst), 1);
    let context = b.contexts.lock().unwrap()[0].clone();
    assert_eq!(context.module_instance, *b.instance.lock().unwrap());
    assert_eq!(
        reply.envelope.source.instance,
        b.instance.lock().unwrap().unwrap()
    );
    assert_eq!(reply.envelope.trace_id, context.trace.unwrap());
    assert_eq!(
        reply.envelope.authority_source,
        MessageAuthoritySource::ModuleStanding
    );
    assert_eq!(reply.envelope.kind, ModuleMessageKind::Reply);
    assert!(reply.envelope.correlation_id.is_some());
    assert_eq!(
        facts(&*runtime.kernel().await, context.session.unwrap())
            .await
            .len(),
        1
    );
    assert_eq!(runtime.usage(None).tool_calls, 1);
}

#[tokio::test]
async fn query_event_private_surface_and_forged_payload_cannot_borrow_authority() {
    let a = Arc::new(State::default());
    let b = Arc::new(State::default());
    let mut query = logical(EffectClass::Pure);
    query.query_safe = true;
    let private = ModuleToolContract {
        surface: ToolSurface::Internal,
        effect: EffectClass::Protected,
        ..Default::default()
    };
    let runtime = builder(Provider::new(vec![]))
        .with_plugin(Module::new("module.a", &[], a.clone()))
        .with_plugin(Module::new(
            "module.b",
            &[
                ("status", query),
                ("private", private),
                ("write", logical(EffectClass::Protected)),
            ],
            b.clone(),
        ))
        .build_agent_runtime(policy(&[(
            "module.a",
            "module.b",
            &["status", "private", "write"],
        )]))
        .await
        .unwrap();
    let mut r = request("module.b", "status");
    r.kind = ModuleMessageKind::Query;
    assert!(a
        .port()
        .send(r.clone())
        .await
        .unwrap()
        .result
        .unwrap()
        .is_ok());
    r.kind = ModuleMessageKind::Event;
    let reply = a.port().send(r.clone()).await.unwrap();
    assert!(reply.result.is_none());
    assert_eq!(reply.envelope.payload, json!({"delivered":true}));
    r.kind = ModuleMessageKind::Reply;
    assert_eq!(
        a.port().send(r).await.unwrap_err().code,
        ModuleMessageErrorCode::InvalidEnvelope
    );
    let mut r = request("module.b", "write");
    r.kind = ModuleMessageKind::Query;
    assert_eq!(
        a.port().send(r).await.unwrap_err().code,
        ModuleMessageErrorCode::Unauthorized
    );
    assert_eq!(
        a.port()
            .send(request("module.b", "private"))
            .await
            .unwrap_err()
            .code,
        ModuleMessageErrorCode::PrivateTarget
    );
    let mut r = request("module.b", "status");
    r.payload = json!({"source":"module.admin","principal":"admin"});
    assert!(!a.port().send(r).await.unwrap().result.unwrap().is_ok());
    assert_eq!(b.calls.load(Ordering::SeqCst), 2);
    drop(runtime);
}

#[tokio::test]
async fn messages_default_disabled_and_missing_supervisor_never_execute_effects() {
    for enabled in [false, true] {
        let a = Arc::new(State::default());
        let b = Arc::new(State::default());
        let p = if enabled {
            policy(&[("module.a", "module.b", &["write"])])
        } else {
            AgentHostPolicy::default()
        };
        let _runtime = builder(Provider::new(vec![]))
            .with_plugin(Module::new("module.a", &[], a.clone()))
            .with_plugin(Module::new(
                "module.b",
                &[("write", logical(EffectClass::Protected))],
                b.clone(),
            ))
            .build_agent_runtime(p)
            .await
            .unwrap();
        let result = a.port().send(request("module.b", "write")).await;
        if enabled {
            assert!(!result.unwrap().result.unwrap().is_ok());
        } else {
            assert_eq!(result.unwrap_err().code, ModuleMessageErrorCode::Disabled);
        }
        assert_eq!(b.calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn loop_detection_precedes_reentrant_mailbox_and_idempotency_refuses_duplicate_effects() {
    let a = Arc::new(State::default());
    let b = Arc::new(State::default());
    *b.relay.lock().unwrap() = Some(request("module.a", "a"));
    let _runtime = builder(Provider::new(vec![]))
        .with_plugin(Module::new(
            "module.a",
            &[("a", logical(EffectClass::Orchestration))],
            a.clone(),
        ))
        .with_plugin(Module::new(
            "module.b",
            &[("b", logical(EffectClass::Orchestration))],
            b.clone(),
        ))
        .build_agent_runtime(policy(&[
            ("module.a", "module.b", &["b"]),
            ("module.a", "module.a", &["a"]),
            ("module.b", "module.a", &["a"]),
        ]))
        .await
        .unwrap();
    let result = a
        .port()
        .send(request("module.b", "b"))
        .await
        .unwrap()
        .result
        .unwrap();
    assert!(result.render().contains("Loop"), "{}", result.render());
    assert_eq!(a.calls.load(Ordering::SeqCst), 0);
    *b.relay.lock().unwrap() = None;
    let mut r = request("module.b", "b");
    r.idempotency_key = Some("operation".into());
    let reply = a.port().send(r.clone()).await.unwrap();
    assert!(reply.result.unwrap().is_ok());
    let error = a.port().send(r.clone()).await.unwrap_err();
    assert_eq!(error.code, ModuleMessageErrorCode::Duplicate);
    assert_eq!(error.original_message, reply.envelope.correlation_id);
    r.payload = json!({"value":"different"});
    assert_eq!(
        a.port().send(r).await.unwrap_err().code,
        ModuleMessageErrorCode::IdempotencyConflict
    );
    assert_eq!(b.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn bounded_admission_cancellation_and_timeout_report_uncertain_execution() {
    for cancel in [true, false] {
        let a = Arc::new(State::default());
        let b = Arc::new(State::default());
        let release = Arc::new(tokio::sync::Notify::new());
        *b.hold.lock().unwrap() = Some(release.clone());
        let mut p = policy(&[("module.a", "module.b", &["hold"])]);
        p.message_policy.as_mut().unwrap().limits.in_flight = 1;
        let _runtime = builder(Provider::new(vec![]))
            .with_plugin(Module::new("module.a", &[], a.clone()))
            .with_plugin(Module::new(
                "module.b",
                &[("hold", logical(EffectClass::Pure))],
                b.clone(),
            ))
            .build_agent_runtime(p)
            .await
            .unwrap();
        let mut r = request("module.b", "hold");
        r.timeout = Duration::from_millis(100);
        r.idempotency_key = Some("pending".into());
        let token = r.cancellation.clone();
        let port = a.port();
        let pending = tokio::spawn(async move { port.send(r).await });
        tokio::time::timeout(Duration::from_secs(2), b.entered.notified())
            .await
            .unwrap();
        assert_eq!(
            a.port()
                .send(request("module.b", "hold"))
                .await
                .unwrap_err()
                .code,
            ModuleMessageErrorCode::Backpressure
        );
        if cancel {
            token.cancel();
        }
        let error = pending.await.unwrap().unwrap_err();
        assert_eq!(
            error.code,
            if cancel {
                ModuleMessageErrorCode::Cancelled
            } else {
                ModuleMessageErrorCode::Timeout
            }
        );
        assert!(error.may_have_executed);
        release.notify_waiters();
        tokio::task::yield_now().await;
        assert_eq!(b.calls.load(Ordering::SeqCst), 0);
        assert!(b.contexts.lock().unwrap()[0].cancellation.is_cancelled());
    }
}

#[tokio::test]
async fn caller_delegation_intersects_scope_and_never_uses_standing_grant_as_fallback() {
    use apeireth_plugin::agents::AgentConfig;
    for delegate in [false, true] {
        let a = Arc::new(State::default());
        let b = Arc::new(State::default());
        *a.relay.lock().unwrap() = Some(request("module.b", "write"));
        let provider = Provider::new(vec![vec![call("relay", json!({}))], vec![]]);
        let (gate, audit) = review_support::allow_gate();
        let runtime = builder(provider.clone())
            .with_system_one(gate)
            .with_plugin(Module::new(
                "module.a",
                &[("relay", logical(EffectClass::Orchestration))],
                a.clone(),
            ))
            .with_plugin(Module::new(
                "module.b",
                &[("write", logical(EffectClass::Protected))],
                b.clone(),
            ))
            .build_agent_runtime(policy(&[("module.a", "module.b", &["write"])]))
            .await
            .unwrap();
        let mut tools = vec![cid("tool.relay")];
        if delegate {
            tools.push(cid("tool.write"));
        }
        let main = runtime
            .spawn_agent(
                "main",
                AgentConfig {
                    tools,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        main.run("request through logical module".into())
            .await
            .unwrap();
        assert_eq!(b.calls.load(Ordering::SeqCst), usize::from(delegate));
        assert_eq!(audit.records().len(), usize::from(delegate));
        assert_eq!(provider.requests().len(), 2);
        let results = provider.results().join("\n");
        assert!(!results.contains("receiver_private"));
        if delegate {
            assert!(results.contains("downstream_ok"));
            assert!(b.contexts.lock().unwrap()[0].delegation.is_some());
        } else {
            assert!(results.contains("Unauthorized"));
        }
    }
}

#[tokio::test]
async fn owner_internal_port_requires_its_own_grant_and_the_same_supervisor() {
    for decision in [
        apeireth_plugin::system_one::SystemOneDecision::Allow,
        apeireth_plugin::system_one::SystemOneDecision::Deny,
    ] {
        let a = Arc::new(State::default());
        let contract = ModuleToolContract {
            surface: ToolSurface::Internal,
            effect: EffectClass::Protected,
            ..Default::default()
        };
        let mut p = policy(&[("module.a", "module.a", &["internal"])]);
        p.message_policy
            .as_mut()
            .unwrap()
            .grants
            .get_mut(&pid("module.a"))
            .unwrap()
            .internal_tools
            .insert(cid("tool.internal"));
        let (gate, audit, _) = review_support::gate(review_support::Mode::Decision(decision));
        let runtime = builder(Provider::new(vec![]))
            .with_system_one(gate)
            .with_plugin(Module::new(
                "module.a",
                &[("internal", contract)],
                a.clone(),
            ))
            .build_agent_runtime(p)
            .await
            .unwrap();
        assert!(runtime.kernel().await.tools().is_empty());
        assert_eq!(
            a.port()
                .send(request("module.a", "internal"))
                .await
                .unwrap_err()
                .code,
            ModuleMessageErrorCode::PrivateTarget
        );
        let result = a
            .port()
            .invoke_internal(InternalToolRequest::new(
                cid("tool.internal"),
                json!({"value":"internal"}),
            ))
            .await
            .unwrap()
            .result
            .unwrap();
        let allowed = decision == apeireth_plugin::system_one::SystemOneDecision::Allow;
        assert_eq!(result.is_ok(), allowed);
        assert_eq!(a.calls.load(Ordering::SeqCst), usize::from(allowed));
        assert_eq!(audit.records().len(), 1);
    }
}

#[tokio::test]
async fn stopping_source_drains_private_mailbox_and_old_generation_cannot_reacquire_authority() {
    use apeireth_core::kernel::{SystemClock, TraceId};
    use apeireth_plugin::credentials::NoCredentials;
    let a = Arc::new(State::default());
    let b = Arc::new(State::default());
    *b.hold.lock().unwrap() = Some(Arc::new(tokio::sync::Notify::new()));
    let runtime = builder(Provider::new(vec![]))
        .with_plugin(Module::new("module.a", &[], a.clone()))
        .with_plugin(Module::new(
            "module.b",
            &[("hold", logical(EffectClass::Pure))],
            b.clone(),
        ))
        .build_agent_runtime(policy(&[("module.a", "module.b", &["hold"])]))
        .await
        .unwrap();
    let old = a.port();
    let port = old.clone();
    let pending = tokio::spawn(async move { port.send(request("module.b", "hold")).await });
    tokio::time::timeout(Duration::from_secs(2), b.entered.notified())
        .await
        .unwrap();
    let session = b.contexts.lock().unwrap()[0].session.unwrap();
    runtime.unregister_module(&pid("module.a")).await.unwrap();
    assert!(pending.await.unwrap().is_err());
    let kernel = runtime.kernel().await;
    let saved = kernel.sessions().load(&session).await.unwrap().unwrap();
    assert_eq!(
        saved
            .events
            .iter()
            .filter(|e| matches!(
                e.event,
                apeireth_runtime::SessionEventKind::ModuleMessageInterrupted { .. }
            ))
            .count(),
        1
    );
    drop(kernel);
    let replacement = Arc::new(State::default());
    runtime
        .register_module(
            Module::new("module.a", &[], replacement.clone()),
            PluginContext::new(
                Arc::new(SystemClock),
                Arc::new(NoCredentials),
                TraceId::new(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(
        *a.instance.lock().unwrap(),
        *replacement.instance.lock().unwrap()
    );
    assert_eq!(
        old.send(request("module.b", "hold"))
            .await
            .unwrap_err()
            .code,
        ModuleMessageErrorCode::SourceStopped
    );
    *b.hold.lock().unwrap() = None;
    assert!(replacement
        .port()
        .send(request("module.b", "hold"))
        .await
        .unwrap()
        .result
        .unwrap()
        .is_ok());
}
