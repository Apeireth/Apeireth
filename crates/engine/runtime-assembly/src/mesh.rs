//! Opt-in local Mesh execution proof. Real loopback UDP, no encryption or device claim.
//! The Host schedules one fixed peer; public calls cannot create network authority.
use apeireth_core::kernel::{CapabilityId, ModuleInstanceId, PluginId, TaskId};
use apeireth_plugin::module_contract::{
    EffectClass, ExecutionMode, ModuleTaskHandle, ModuleToolContract, ToolSurface,
};
use apeireth_plugin::module_messages::{
    InternalToolRequest, MessageVisibility, ModuleMessageKind, ModuleMessageRequest,
};
use apeireth_plugin::{
    FrozenInvocation, Plugin, PluginContext, PluginManifest, PluginResult, ToolCapability,
    ToolExecutionContext, ToolExecutionPolicy, ToolModule,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn module_id() -> PluginId {
    PluginId::new("module.mesh").expect("static id")
}
fn capability(suffix: &str) -> CapabilityId {
    CapabilityId::new(format!("tool.mesh.{suffix}")).expect("static id")
}

#[derive(Clone)]
pub struct MeshStatusTarget {
    pub module: PluginId,
    pub capability: CapabilityId,
}
/// Host-only configuration. It is never read from a model payload or environment.
#[derive(Clone)]
pub struct MeshConfig {
    peer: String,
    endpoint: SocketAddr,
    private_payload: Vec<u8>,
    interval: Duration,
    timeout: Duration,
    status_target: Option<MeshStatusTarget>,
}
impl MeshConfig {
    pub fn loopback(
        peer: impl Into<String>,
        endpoint: SocketAddr,
        private_payload: Vec<u8>,
    ) -> Self {
        Self {
            peer: peer.into(),
            endpoint,
            private_payload,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(2),
            status_target: None,
        }
    }
    pub fn with_timing(mut self, interval: Duration, timeout: Duration) -> Self {
        self.interval = interval;
        self.timeout = timeout;
        self
    }
    pub fn with_status_target(mut self, target: MeshStatusTarget) -> Self {
        self.status_target = Some(target);
        self
    }
    fn validate(&self) -> Result<(), String> {
        if !self.endpoint.ip().is_loopback()
            || self.endpoint.port() == 0
            || self.peer.is_empty()
            || self.peer.len() > 64
            || !self
                .peer
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || self.private_payload.len() > 512
            || self.interval < Duration::from_millis(20)
            || self.interval > Duration::from_secs(3600)
            || self.timeout < Duration::from_millis(10)
            || self.timeout > Duration::from_secs(10)
        {
            return Err(
                "Mesh requires a bounded fixed loopback peer and explicit Host timing".into(),
            );
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshState {
    Connecting,
    Online,
    Degraded,
    Stopped,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshHealth {
    Unknown,
    Healthy,
    Degraded,
}
/// The complete public whitelist; no endpoint, raw packet or transcript fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshStatus {
    pub state: MeshState,
    pub online_peers: u8,
    pub network_health: MeshHealth,
    pub pending_operations: u8,
}
struct PrivateState {
    instance: Option<ModuleInstanceId>,
    handle: Option<ModuleTaskHandle>,
    status: MeshStatus,
    publication_ok: bool,
}
impl Default for PrivateState {
    fn default() -> Self {
        Self {
            instance: None,
            handle: None,
            status: MeshStatus {
                state: MeshState::Connecting,
                online_peers: 0,
                network_health: MeshHealth::Unknown,
                pending_operations: 1,
            },
            publication_ok: false,
        }
    }
}
pub struct MeshModule {
    inner: ToolModule,
    config: Arc<MeshConfig>,
    state: Arc<Mutex<PrivateState>>,
}
impl MeshModule {
    pub fn new(config: MeshConfig) -> Result<Self, String> {
        config.validate()?;
        let config = Arc::new(config);
        let state = Arc::new(Mutex::new(PrivateState::default()));
        let tools = [Kind::Connect, Kind::Status, Kind::Exchange]
            .into_iter()
            .map(|kind| {
                Arc::new(MeshTool {
                    id: capability(kind.suffix()),
                    kind,
                    config: config.clone(),
                    state: state.clone(),
                }) as Arc<dyn ToolCapability>
            })
            .collect();
        let inner = ToolModule::new(module_id(), tools).map_err(|e| e.to_string())?;
        Ok(Self {
            inner,
            config,
            state,
        })
    }
    pub fn status(&self) -> MeshStatus {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status
            .clone()
    }
}
#[async_trait]
impl Plugin for MeshModule {
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
        let instance = ctx.module_instance().ok_or_else(|| {
            apeireth_plugin::PluginError::init_failed(
                module_id(),
                "Mesh requires real module activation",
            )
        })?;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        *state = PrivateState {
            instance: Some(instance),
            handle: Some(ModuleTaskHandle {
                version: 1,
                module: module_id(),
                instance,
                task: TaskId::new(),
            }),
            ..Default::default()
        };
        Ok(())
    }
    async fn start(&self, ctx: &PluginContext) -> PluginResult<()> {
        let agents = ctx.agents.clone().ok_or_else(|| {
            apeireth_plugin::PluginError::init_failed(
                module_id(),
                "Mesh requires the existing AgentRuntime",
            )
        })?;
        let port = agents.messages().ok_or_else(|| {
            apeireth_plugin::PluginError::init_failed(module_id(), "Mesh message port unavailable")
        })?;
        let scope = ctx.tasks.clone();
        let lease = scope.lease().map_err(|_| {
            apeireth_plugin::PluginError::init_failed(module_id(), "Mesh owner stopped")
        })?;
        let config = self.config.clone();
        let state = self.state.clone();
        tokio::spawn(async move {
            let _lease = lease;
            let _ = scope
                .run(async {
                    if agents.wait_ready().await.is_err() {
                        return;
                    }
                    let mut published = None;
                    loop {
                        let mut request = InternalToolRequest::new(
                            capability("exchange"),
                            json!({"device":config.peer}),
                        );
                        // A native timeout returns a closed ToolResult before the mailbox deadline.
                        // A stricter Host transport deadline still fails closed and needs Host recovery.
                        request.timeout = config.timeout.saturating_mul(2);
                        let online = port
                            .invoke_internal(request)
                            .await
                            .ok()
                            .and_then(|r| r.result)
                            .is_some_and(|r| r.is_ok());
                        let status = MeshStatus {
                            state: if online {
                                MeshState::Online
                            } else {
                                MeshState::Degraded
                            },
                            online_peers: u8::from(online),
                            network_health: if online {
                                MeshHealth::Healthy
                            } else {
                                MeshHealth::Degraded
                            },
                            pending_operations: 0,
                        };
                        state.lock().unwrap_or_else(|e| e.into_inner()).status = status.clone();
                        if let Some(target) = &config.status_target {
                            if published.as_ref() != Some(&status) {
                                let mut event = ModuleMessageRequest::command(
                                    target.module.clone(),
                                    target.capability.clone(),
                                    serde_json::to_value(&status).expect("status"),
                                );
                                event.kind = ModuleMessageKind::Event;
                                event.visibility = MessageVisibility::PublicStatus;
                                event.timeout = config.timeout;
                                let ok = port.send(event).await.is_ok_and(|r| {
                                    r.envelope.payload.get("delivered") == Some(&json!(true))
                                });
                                state
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .publication_ok = ok;
                                if ok {
                                    published = Some(status);
                                }
                            }
                        }
                        tokio::time::sleep(config.interval).await;
                    }
                })
                .await;
        });
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).status = MeshStatus {
            state: MeshState::Stopped,
            online_peers: 0,
            network_health: MeshHealth::Unknown,
            pending_operations: 0,
        };
        Ok(())
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Connect,
    Status,
    Exchange,
}
impl Kind {
    fn suffix(self) -> &'static str {
        match self {
            Self::Connect => "connect",
            Self::Status => "status",
            Self::Exchange => "exchange",
        }
    }
}
struct MeshTool {
    id: CapabilityId,
    kind: Kind,
    config: Arc<MeshConfig>,
    state: Arc<Mutex<PrivateState>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenExchange {
    version: u16,
    endpoint: SocketAddr,
    nonce: TaskId,
    packet: Vec<u8>,
}
fn packet(nonce: TaskId, payload: &[u8]) -> Vec<u8> {
    let mut bytes = format!("ale.mesh.v1:{nonce}:").into_bytes();
    bytes.extend_from_slice(payload);
    bytes
}
fn refused(call: &ToolCall, message: &str) -> ToolResult {
    ToolResult::permanent_error(&call.id, message).with_name(&call.name)
}
#[async_trait]
impl ToolCapability for MeshTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        let mut d = NormalizedTool::new(format!("mesh.{}", self.kind.suffix()));
        d.parameters=if self.kind==Kind::Status {json!({"type":"object","properties":{"task_handle":{"type":"object"}},"additionalProperties":false})} else {json!({"type":"object","properties":{"device":{"type":"string"}},"required":["device"],"additionalProperties":false})}.as_object().expect("schema").clone();
        d
    }
    fn model_visible(&self) -> bool {
        self.kind != Kind::Exchange
    }
    fn module_contract(&self) -> ModuleToolContract {
        if self.kind == Kind::Exchange {
            ModuleToolContract {
                surface: ToolSurface::Internal,
                execution: ExecutionMode::Background,
                effect: EffectClass::Protected,
                ..Default::default()
            }
        } else {
            let mut c = ModuleToolContract::logical(ExecutionMode::Direct, EffectClass::Pure);
            c.query_safe = self.kind == Kind::Status;
            c
        }
    }
    fn execution_policy(&self) -> ToolExecutionPolicy {
        if self.kind == Kind::Exchange {
            ToolExecutionPolicy {
                timeout: Some(self.config.timeout),
                concurrency: apeireth_plugin::ToolConcurrency::Limited(1),
                ..Default::default()
            }
        } else {
            Default::default()
        }
    }
    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        if self.kind != Kind::Exchange {
            return Ok(None);
        }
        if call.arguments.get("device").and_then(|v| v.as_str()) != Some(self.config.peer.as_str())
        {
            return Err(refused(
                call,
                "peer is outside the fixed Host configuration",
            ));
        }
        let nonce = TaskId::new();
        let frozen = FrozenExchange {
            version: 1,
            endpoint: self.config.endpoint,
            nonce,
            packet: packet(nonce, &self.config.private_payload),
        };
        Ok(Some(FrozenInvocation::new(
            serde_json::to_value(frozen).expect("frozen exchange"),
            json!({"device":self.config.peer,"operation":"loopback_exchange"}),
        )))
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        refused(call, "Mesh requires registered scoped execution")
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        context: &ToolExecutionContext,
    ) -> ToolResult {
        let (handle, status) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.instance != context.module_instance
                || state.instance.is_none()
                || !context.module_tasks.is_open()
            {
                return refused(call, "Mesh generation stopped or changed");
            }
            (state.handle.clone(), state.status.clone())
        };
        match self.kind {
            Kind::Connect => {
                if call.arguments.get("device").and_then(|v| v.as_str())
                    != Some(self.config.peer.as_str())
                {
                    return refused(call, "peer is outside the fixed Host configuration");
                }
                ToolResult::ok(
                    &call.id,
                    json!({"accepted":true,"task_handle":handle,"status":"host_scheduled"}),
                )
                .with_name(&call.name)
            }
            Kind::Status => {
                if let Some(value) = call.arguments.get("task_handle") {
                    if serde_json::from_value::<ModuleTaskHandle>(value.clone()).ok() != handle {
                        return refused(call, "unknown or stale Mesh task handle");
                    }
                }
                ToolResult::ok(&call.id, serde_json::to_value(status).expect("status"))
                    .with_name(&call.name)
            }
            Kind::Exchange => {
                if let Err(error) = apeireth_plugin::system_one::require_internal_review(
                    self.id(),
                    call,
                    frozen,
                    context,
                ) {
                    return error;
                }
                let Some(frozen) = frozen
                    .and_then(|f| serde_json::from_value::<FrozenExchange>(f.payload.clone()).ok())
                else {
                    return refused(call, "invalid frozen Mesh operation");
                };
                if frozen.version != 1
                    || frozen.endpoint != self.config.endpoint
                    || !frozen.endpoint.ip().is_loopback()
                    || frozen.packet != packet(frozen.nonce, &self.config.private_payload)
                {
                    return refused(call, "frozen Mesh operation does not match Host policy");
                }
                let work = async {
                    let bind = if frozen.endpoint.is_ipv4() {
                        "127.0.0.1:0"
                    } else {
                        "[::1]:0"
                    };
                    let socket = tokio::net::UdpSocket::bind(bind).await.map_err(|_| ())?;
                    if context.cancellation.is_cancelled() || Instant::now() >= context.deadline {
                        return Err(());
                    }
                    socket
                        .send_to(&frozen.packet, frozen.endpoint)
                        .await
                        .map_err(|_| ())?;
                    let mut bytes = [0u8; 1024];
                    let (length, source) = socket.recv_from(&mut bytes).await.map_err(|_| ())?;
                    if source != frozen.endpoint || bytes[..length] != frozen.packet {
                        return Err(());
                    }
                    Ok::<_, ()>(())
                };
                let cancelled = async {
                    while !context.cancellation.is_cancelled() {
                        tokio::time::sleep(Duration::from_millis(2)).await;
                    }
                };
                let result = tokio::select! {biased;_=cancelled=>None,result=tokio::time::timeout(context.deadline.saturating_duration_since(Instant::now()).min(self.config.timeout),work)=>Some(result)};
                if matches!(result, Some(Ok(Ok(())))) {
                    ToolResult::ok(&call.id, json!({"connected":true})).with_name(&call.name)
                } else {
                    refused(
                        call,
                        "private Mesh exchange unavailable or interrupted; no rollback is implied",
                    )
                }
            }
        }
    }
}
