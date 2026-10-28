//! Compatibility ABI over the ordinary Plugin tool path; never a second evaluator.
use super::module::*;
use apeireth_core::kernel::{CapabilityId, PluginId, RequestId, SessionId, TaskId, TraceId};
use apeireth_plugin::hooks::{HookContextV1, HookToolRequest};
use apeireth_plugin::{
    CapabilityKind, FrozenInvocation, Plugin, PluginContext, PluginManifest, PluginResult,
    ToolCapability, ToolExecutionContext,
};
use apeireth_protocol::canonical::{
    ContentPart, ModuleHookV1, NormalizedMessage, NormalizedResponse, NormalizedTool, ToolCall,
    ToolResult,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub fn registered_behavior_capability(id: &str) -> Result<CapabilityId, String> {
    CapabilityId::new(format!("tool.behavior.{id}")).map_err(|e| e.to_string())
}
pub(crate) fn compatibility_hook(hook: HookPoint) -> ModuleHookV1 {
    match hook {
        HookPoint::TurnStart => ModuleHookV1::TurnStart,
        HookPoint::BeforeFinalCommit => ModuleHookV1::BeforeFinalCommit,
        HookPoint::OnError => ModuleHookV1::OnError,
        other => super::registered_hooks::fixed_hook(other).expect("fixed compatibility hook"),
    }
}
pub(crate) struct BehaviorInput {
    pub session: SessionId,
    pub principal: Option<apeireth_plugin::agents::AgentPrincipal>,
    pub authorization: Option<super::principal_authority::PrincipalGrant>,
    pub request: RequestId,
    pub trace: TraceId,
    pub hook: HookPoint,
    pub model: String,
    pub messages: Arc<[NormalizedMessage]>,
    pub candidate: Option<NormalizedResponse>,
    pub tool_call: Option<ToolCall>,
    pub tool_result: Option<ToolResult>,
    pub invocation: InvocationContext,
    pub error: Option<String>,
    pub spawner: Arc<super::subloop::RuntimeSubLoopSpawner>,
}
struct Shared {
    module: Arc<dyn BehaviorModule>,
    owner: PluginId,
    capability: CapabilityId,
    name: String,
    pending: Mutex<BTreeMap<TaskId, BehaviorInput>>,
    outcomes: Mutex<BTreeMap<TaskId, Result<ModuleOutcome, ModuleError>>>,
    scope: Mutex<Option<apeireth_plugin::task_scope::ModuleTaskScope>>,
}
pub(crate) struct RegisteredBehavior {
    manifest: PluginManifest,
    shared: Arc<Shared>,
}
pub(crate) struct BehaviorReceipt {
    shared: Arc<Shared>,
    pub execution: TaskId,
}
impl Drop for BehaviorReceipt {
    fn drop(&mut self) {
        self.shared
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.execution);
        self.shared
            .outcomes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.execution);
    }
}
impl BehaviorReceipt {
    pub fn take(&self) -> Result<ModuleOutcome, ModuleError> {
        self.shared
            .outcomes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.execution)
            .unwrap_or_else(|| Ok(ModuleOutcome::continue_()))
    }
}
impl RegisteredBehavior {
    pub(crate) fn new(module: Arc<dyn BehaviorModule>) -> Result<Self, String> {
        let owner =
            PluginId::new(format!("compat.{}", module.manifest().id)).map_err(|e| e.to_string())?;
        let capability = registered_behavior_capability(&module.manifest().id)?;
        let name = format!("module_{}", module.manifest().id.replace('.', "_"));
        let manifest = PluginManifest::new(
            owner.clone(),
            "1.0.0",
            format!("Registered compatibility for {}", module.manifest().name),
        )
        .declare_capability(
            capability.clone(),
            CapabilityKind::Tool,
            "Host-selected cognitive callback",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            manifest,
            shared: Arc::new(Shared {
                module,
                owner,
                capability,
                name,
                pending: Mutex::new(BTreeMap::new()),
                outcomes: Mutex::new(BTreeMap::new()),
                scope: Mutex::new(None),
            }),
        })
    }
    pub fn id(&self) -> &PluginId {
        &self.shared.owner
    }
    pub fn capability(&self) -> &CapabilityId {
        &self.shared.capability
    }
    pub fn prepare(&self, input: BehaviorInput) -> Result<Option<BehaviorReceipt>, ModuleError> {
        if !self
            .shared
            .scope
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|s| s.is_open())
        {
            return Ok(None);
        }
        let mut pending = self
            .shared
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // Bounded receipts; dropped turns release them through BehaviorReceipt.
        if pending.len() >= 256 {
            return Err(ModuleError::Message("module receipt limit reached".into()));
        }
        let execution = TaskId::new();
        pending.insert(execution, input);
        Ok(Some(BehaviorReceipt {
            shared: self.shared.clone(),
            execution,
        }))
    }
}
#[async_trait]
impl Plugin for RegisteredBehavior {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn hooks(&self) -> Vec<ModuleHookV1> {
        vec![
            ModuleHookV1::TurnStart,
            ModuleHookV1::BeforeModelCall,
            ModuleHookV1::AfterModelCall,
            ModuleHookV1::BeforeToolCall,
            ModuleHookV1::AfterToolCall,
            ModuleHookV1::BeforeFinalCommit,
            ModuleHookV1::OnIdle,
            ModuleHookV1::OnError,
        ]
    }
    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        *self.shared.scope.lock().unwrap_or_else(|e| e.into_inner()) = Some(ctx.tasks.clone());
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        self.shared
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        self.shared
            .outcomes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        Ok(())
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::new(BehaviorTool(self.shared.clone()))]
    }
    async fn on_hook(&self, ctx: &HookContextV1) -> PluginResult<Vec<HookToolRequest>> {
        Ok(self
            .shared
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|(_, i)| {
                i.session == ctx.session
                    && i.request == ctx.request
                    && i.trace == ctx.trace
                    && compatibility_hook(i.hook) == ctx.hook
            })
            .map(|(id, _)| HookToolRequest {
                execution: *id,
                call: ToolCall {
                    id: id.to_string(),
                    name: self.shared.name.clone(),
                    arguments: serde_json::json!({"invocation":id}),
                },
            })
            .collect())
    }
}
struct BehaviorTool(Arc<Shared>);
#[async_trait]
impl ToolCapability for BehaviorTool {
    fn id(&self) -> &CapabilityId {
        &self.0.capability
    }
    fn model_visible(&self) -> bool {
        false
    }
    fn module_result_delivery(
        &self,
        result: &ToolResult,
    ) -> apeireth_plugin::tool_execution::ModuleResultDelivery {
        match &result.outcome {
            apeireth_protocol::canonical::ToolOutcome::Ok { value }
                if value["context"].as_array().is_some_and(|v| !v.is_empty()) =>
            {
                apeireth_plugin::tool_execution::ModuleResultDelivery::Provider
            }
            _ => apeireth_plugin::tool_execution::ModuleResultDelivery::AuditOnly,
        }
    }
    fn declaration(&self) -> NormalizedTool {
        let mut tool = NormalizedTool::new(&self.0.name);
        tool.parameters=serde_json::json!({"type":"object","properties":{"invocation":{"type":"string"}},"required":["invocation"],"additionalProperties":false}).as_object().unwrap().clone();
        tool
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::permanent_error(&call.id, "authenticated module invocation required")
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        _: Option<&FrozenInvocation>,
        ctx: &ToolExecutionContext,
    ) -> ToolResult {
        if !matches!(&ctx.caller,apeireth_plugin::ToolCaller::Module(id) if id==&self.0.owner) {
            return self.invoke(call).await;
        }
        let id: TaskId = match serde_json::from_value(call.arguments["invocation"].clone()) {
            Ok(id) => id,
            Err(_) => return self.invoke(call).await,
        };
        let input = {
            let mut pending = self.0.pending.lock().unwrap_or_else(|e| e.into_inner());
            if !pending.get(&id).is_some_and(|i| {
                Some(i.session) == ctx.session
                    && i.principal == ctx.principal
                    && (i.principal.is_none()
                        || i.authorization.as_ref().is_some_and(|g| {
                            g.allows_behavior(&self.0.module.manifest().id)
                                && ctx.authorization.principal_epoch.as_deref() == Some(g.epoch())
                        }))
                    && Some(i.request) == ctx.turn
                    && Some(i.trace) == ctx.trace
            }) {
                return ToolResult::permanent_error(&call.id, "invalid invocation receipt");
            }
            pending.remove(&id).expect("checked receipt")
        };
        let invoker_handle: Arc<dyn ModuleInvoker> = input.spawner.clone();
        let context = ModuleContext {
            session_id: &input.session,
            model: &input.model,
            messages: &input.messages,
            candidate: input.candidate.as_ref(),
            tool_call: input.tool_call.as_ref(),
            tool_result: input.tool_result.as_ref(),
            invocation: &input.invocation,
            module_id: &self.0.module.manifest().id,
            error: input.error.as_deref(),
            invoker: &*input.spawner,
            invoker_handle,
            subloop: &*input.spawner,
        };
        let outcome = self
            .0
            .module
            .on_hook_authenticated(input.hook, &context, ctx)
            .await;
        if ctx.cancellation.is_cancelled()
            || input.authorization.as_ref().is_some_and(|g| !g.is_active())
        {
            return ToolResult::permanent_error(&call.id, "principal authorization revoked");
        }
        let result = match &outcome {
            Ok(outcome) => {
                let data=outcome.prompt_overlays.iter().map(|overlay|serde_json::json!({"source":overlay.cross_source_label(),"text":ContentPart::join_text(&overlay.message().content)})).collect::<Vec<_>>();
                ToolResult::ok(
                    &call.id,
                    serde_json::json!({"untrusted_data":true,"context":data}),
                )
            }
            Err(error) => ToolResult::permanent_error(&call.id, error.to_string()),
        };
        self.0
            .outcomes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(
                id,
                outcome.map(|mut outcome| {
                    outcome.prompt_overlays.clear();
                    outcome
                }),
            );
        result
    }
}

/// Legacy engines see the same business transcript, not copies of their own module data.
pub(crate) fn business_messages(
    messages: &[NormalizedMessage],
    session: &super::session::Session,
) -> Vec<NormalizedMessage> {
    let pairs: BTreeMap<String, [NormalizedMessage; 2]> = session
        .events
        .iter()
        .filter_map(|e| match &e.event {
            super::session::SessionEventKind::ToolEventRecorded { fact }
                if matches!(
                    fact.source(),
                    apeireth_protocol::canonical::ToolEventSourceV1::Module { .. }
                ) =>
            {
                Some((fact.call().id.clone(), fact.provider_messages()))
            }
            _ => None,
        })
        .collect();
    let mut output = Vec::with_capacity(messages.len());
    let mut index = 0;
    while index < messages.len() {
        let pair = messages[index]
            .tool_calls
            .first()
            .filter(|_| messages[index].tool_calls.len() == 1)
            .and_then(|call| pairs.get(&call.id));
        if pair.is_some_and(|pair| {
            messages.get(index) == Some(&pair[0]) && messages.get(index + 1) == Some(&pair[1])
        }) {
            index += 2;
        } else {
            output.push(messages[index].clone());
            index += 1;
        }
    }
    output
}
