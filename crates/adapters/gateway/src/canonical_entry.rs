//! HTTP and native chat adapters for the canonical runtime.
//!
//! This module owns decoding, transport validation, canonical turn
//! construction, runtime invocation, and response encoding. It intentionally
//! owns no provider selection, governance composition, session orchestration,
//! plugin lifecycle, tool dispatch, retry, or agent loop.

use std::sync::{Arc, RwLock};

use apeireth_core::kernel::{ApprovalId, RequestId, SessionId, Timestamp};
use apeireth_governance::{may_resolve_approval, UserCapability};
use apeireth_guard::{IntentInput, IntentInterpreter, RuleIntentInterpreter};
use apeireth_protocol::canonical::{ContentPart, NormalizedUsage};
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, ExecutionTrace, PendingApprovalView, Runtime,
    RuntimeError, TraceEvent, TurnOutcome, TurnRequest, TurnSecurityContext,
};
use axum::extract::{Extension, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoConnBuilder;
use hyper_util::service::TowerToHyperService;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use tower_http::cors::AllowOrigin;

use crate::admin::{admin_config_get, admin_config_update, GatewayRuntimeConfig};
use crate::client_tokens::{
    client_name_binds, record_session_client, record_user_action, ClientIdentity, UserContext,
    CLIENT_NAME_CLAIM_HEADER,
};
use crate::error_frame::{ErrorCode, ErrorFrame};
use crate::events::{events_handler, EventBus, GatewayEvent, RuntimeObservationSink};
use crate::panels::{panel_routes, GatewayServices, GatewayState, PanelData};

/// Native gateway request. HTTP and CLI transports can both construct it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CanonicalChatRequest {
    /// Existing canonical session, or a fresh session when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionId>,
    /// User input for this turn.
    pub input: String,
    /// Optional model override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// System instruction used only when the session is new.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

impl CanonicalChatRequest {
    /// A request containing one user turn.
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            session: None,
            input: input.into(),
            model: None,
            system: None,
        }
    }
}

/// Transport-neutral response returned after canonical execution.
#[derive(Debug, Clone, Serialize)]
pub struct CanonicalChatResponse {
    /// Stable session used by the full turn.
    pub session: SessionId,
    /// Runtime request identifier.
    pub request: String,
    /// Runtime trace identifier.
    pub trace_id: String,
    /// Final assistant text.
    pub text: String,
    /// Provider capability that served the final round.
    pub served_by: String,
    /// Provider round-trips taken.
    pub rounds: u32,
    /// Canonical token accounting.
    pub usage: NormalizedUsage,
    /// Structured execution metadata; never raw model reasoning.
    pub trace: ExecutionTrace,
    /// Product-facing execution events derived from the trace.
    ///
    /// Desktop may render these. It must not execute tools from them.
    pub events: Vec<CanonicalExecutionEvent>,
}

/// Minimal product-facing execution event.
///
/// These are observations of the Main Loop. They are not a tool-call protocol
/// and they never authorize the client to run a capability.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CanonicalExecutionEvent {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub succeeded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<u32>,
}

fn events_from_trace(trace: &ExecutionTrace) -> Vec<CanonicalExecutionEvent> {
    let mut events = Vec::new();
    for entry in &trace.entries {
        match &entry.event {
            TraceEvent::CapabilityDispatched {
                capability,
                tool_call_id,
                round,
            } => events.push(CanonicalExecutionEvent {
                event: "tool_started".into(),
                tool_name: Some(capability.to_string()),
                capability_id: Some(capability.to_string()),
                tool_call_id: Some(tool_call_id.clone()),
                succeeded: None,
                approval_id: None,
                round: Some(*round),
            }),
            TraceEvent::CapabilityCompleted {
                capability,
                tool_call_id,
                succeeded,
                round,
            } => events.push(CanonicalExecutionEvent {
                event: if *succeeded {
                    "tool_completed".into()
                } else {
                    "tool_failed".into()
                },
                tool_name: Some(capability.to_string()),
                capability_id: Some(capability.to_string()),
                tool_call_id: Some(tool_call_id.clone()),
                succeeded: Some(*succeeded),
                approval_id: None,
                round: Some(*round),
            }),
            TraceEvent::ApprovalRequested {
                approval_id,
                capability,
                tool_call_id,
                round,
            } => events.push(CanonicalExecutionEvent {
                event: "approval_required".into(),
                tool_name: Some(capability.to_string()),
                capability_id: Some(capability.to_string()),
                tool_call_id: Some(tool_call_id.clone()),
                succeeded: None,
                approval_id: Some(approval_id.to_string()),
                round: Some(*round),
            }),
            _ => {}
        }
    }
    events
}

/// Transport-neutral pending-approval payload. Exposes identity and safe
/// metadata only; it never includes the executable frozen payload.
#[derive(Debug, Clone, Serialize)]
pub struct CanonicalPendingApproval {
    pub session: SessionId,
    pub approval_id: ApprovalId,
    pub request: String,
    pub trace_id: String,
    /// Runtime round that produced the pending operation (audit pairing key).
    pub round: u32,
    pub capability_id: String,
    pub tool_name: String,
    pub governance_hook: String,
    pub governance_reason: String,
    pub command_text: String,
    pub arguments_summary: String,
    /// 审批卡族标注 / 授权对象（E-P2 族归一）：被批命令的族 id 列表；
    /// 空 = 非命令面操作（授权对象 = 操作本身）。
    pub authorization_families: Vec<String>,
    /// 动作摘要（G2.5 动作绑定）：被批动作内容的 SHA-256 摘要（hex）。
    /// 执行边界重算比对，载荷被改 → 拒绝即帧；卡面只出短摘要。
    /// 存量记录（无冻结绑定）为空串。
    pub action_binding_digest: String,
    /// The turn's generated text so far, preserved by the pending close so the
    /// UI can keep what the model already said alongside "waiting for
    /// approval: <tool>".
    pub generated_text: String,
    /// Round slots the turn had consumed when it paused for this approval.
    pub rounds_used: u32,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

impl From<PendingApprovalView> for CanonicalPendingApproval {
    fn from(view: PendingApprovalView) -> Self {
        Self {
            session: view.session_id,
            approval_id: view.approval_id,
            request: view.request_id.to_string(),
            trace_id: view.trace_id.to_string(),
            round: view.round,
            capability_id: view.capability_id.to_string(),
            tool_name: view.tool_name,
            governance_hook: view.governance_hook,
            governance_reason: view.governance_reason,
            command_text: view.command_text,
            arguments_summary: view.arguments_summary,
            authorization_families: view.authorization_families,
            action_binding_digest: view.action_binding_digest,
            generated_text: view.generated_text,
            rounds_used: view.rounds_used,
            created_at: view.created_at,
            expires_at: view.expires_at,
        }
    }
}

/// Result of a canonical chat or approval-resume call.
#[derive(Debug, Clone)]
pub enum CanonicalChatOutcome {
    /// The turn completed.
    Completed(CanonicalChatResponse),
    /// The turn is paused for human approval. The `ApprovalId` is retained.
    PendingApproval(CanonicalPendingApproval),
}

/// Failure at the gateway adapter boundary.
#[derive(Debug, thiserror::Error)]
pub enum CanonicalEntryError {
    /// Transport input was not meaningful enough to form a turn.
    #[error("invalid chat request: {0}")]
    InvalidRequest(String),
    /// Canonical runtime execution failed.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

/// Request to resolve one pending approval.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CanonicalApprovalRequest {
    pub session: SessionId,
    pub approval: ApprovalId,
    /// `approve`, `reject`, or `cancel`.
    pub decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// W2-③ 会话归属: 这次会话写以谁的名义署名、事实落哪个审计面。
///
/// 署名键只来自令牌绑定 ([`ClientIdentity`]); 会话写无论有无归属都过同一把
/// 会话写闸 (排队 + 串行), 归属只决定 `gateway.session_client` 审计行怎么署名。
#[derive(Clone)]
pub struct SessionAttribution {
    /// 审计署名键 (令牌表 `name` / `env` / `anonymous` / `im-bridge` / `unattributed`)。
    pub client: String,
    /// 观测攒批面: `gateway.session_client` 审计行落点 (None = 不留审计行)。
    pub audit: Option<Arc<RuntimeObservationSink>>,
}

impl std::fmt::Debug for SessionAttribution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionAttribution")
            .field("client", &self.client)
            .field("audit", &self.audit.is_some())
            .finish()
    }
}

impl SessionAttribution {
    /// 无归属的内部直调 (不署名、不留审计行; 会话写仍过写闸串行)。
    pub fn unattributed() -> Self {
        Self {
            client: ClientIdentity::UNATTRIBUTED.to_string(),
            audit: None,
        }
    }

    /// 以具名客户端署名 (审计面可选)。
    pub fn new(client: impl Into<String>, audit: Option<Arc<RuntimeObservationSink>>) -> Self {
        Self {
            client: client.into(),
            audit,
        }
    }
}

/// 持会话写闸跑一个回合 (W2-③ 并发写语义: **排队 + 审计**)。
///
/// 同一会话的回合先到先服务串行 —— 第二个客户端的回合**排队**等前一个跑完,
/// 不发 busy 帧、不交叠写会话账本; 入场事实 (排第几 / 等多久) 落
/// `gateway.session_client` 审计行, 排队因此可观测可审计。
async fn run_turn_serialized(
    runtime: &Runtime,
    turn: TurnRequest,
    attribution: &SessionAttribution,
    on_delta: Option<Arc<dyn Fn(String) + Send + Sync>>,
) -> Result<TurnOutcome, RuntimeError> {
    let permit = runtime.sessions().write_permit(turn.session).await;
    record_session_client(
        attribution.audit.as_deref(),
        turn.session,
        &attribution.client,
        "turn",
        Some(permit.admission()),
    );
    let outcome = match on_delta {
        Some(sink) => runtime.execute_outcome_streaming(turn, sink).await,
        None => runtime.execute_outcome(turn).await,
    };
    drop(permit);
    outcome
}

/// Invoke the canonical runtime through the real gateway entry adapter.
pub async fn execute_chat(
    runtime: &Runtime,
    request: CanonicalChatRequest,
) -> Result<CanonicalChatOutcome, CanonicalEntryError> {
    execute_chat_attributed(runtime, request, SessionAttribution::unattributed()).await
}

/// 归属化回合入口 (W2-③): 会话写闸串行 (排队语义) + `gateway.session_client`
/// 审计行 (客户端名 ↔ 会话事件关联)。
pub async fn execute_chat_attributed(
    runtime: &Runtime,
    request: CanonicalChatRequest,
    attribution: SessionAttribution,
) -> Result<CanonicalChatOutcome, CanonicalEntryError> {
    validate_chat_input(&request.input)?;

    let session = request.session.unwrap_or_else(SessionId::new);
    let input = request.input;
    let intent = RuleIntentInterpreter.interpret(IntentInput {
        session_id: session.to_string(),
        trace_id: String::new(),
        user_request: input.clone(),
        created_at_ms: 0,
    });
    let context = TurnSecurityContext::new(intent.intent_id.clone(), "").with_intent(intent);
    let mut turn = TurnRequest::new(session, input).with_security_context(context);
    if let Some(model) = request.model {
        turn = turn.with_model(model);
    }
    if let Some(system) = request.system {
        turn = turn.with_system(system);
    }

    let outcome = run_turn_serialized(runtime, turn, &attribution, None).await?;
    Ok(turn_outcome_to_chat(outcome))
}

/// Resolve a pending approval through the canonical runtime API.
pub async fn resolve_approval(
    runtime: &Runtime,
    request: CanonicalApprovalRequest,
) -> Result<CanonicalChatOutcome, CanonicalEntryError> {
    resolve_approval_attributed(runtime, request, SessionAttribution::unattributed()).await
}

/// 归属化审批续跑 (W2-③): 续跑写同一本会话账本, 同样过会话写闸串行 +
/// `gateway.session_client` 审计 (`action=approval.resolve`)。
pub async fn resolve_approval_attributed(
    runtime: &Runtime,
    request: CanonicalApprovalRequest,
    attribution: SessionAttribution,
) -> Result<CanonicalChatOutcome, CanonicalEntryError> {
    let decision = parse_approval_decision(&request.decision, request.reason.clone())?;
    let permit = runtime.sessions().write_permit(request.session).await;
    record_session_client(
        attribution.audit.as_deref(),
        request.session,
        &attribution.client,
        "approval.resolve",
        Some(permit.admission()),
    );
    let resolved = runtime
        .resolve_approval(request.session, request.approval, decision)
        .await;
    drop(permit);
    match resolved? {
        ApprovalResolution::Resumed(outcome) => Ok(turn_outcome_to_chat(outcome)),
        ApprovalResolution::AlreadyResolved { status } => Err(CanonicalEntryError::InvalidRequest(
            format!("approval already resolved: {status:?}"),
        )),
        ApprovalResolution::ExecutionInterrupted { approval_id } => {
            Err(CanonicalEntryError::InvalidRequest(format!(
                "approval {approval_id} was interrupted and must not be retried automatically"
            )))
        }
        ApprovalResolution::Expired => Err(CanonicalEntryError::InvalidRequest(
            "approval expired before it was resolved".into(),
        )),
        ApprovalResolution::NotFound => Err(CanonicalEntryError::InvalidRequest(
            "approval was not found for this session".into(),
        )),
    }
}

fn turn_outcome_to_chat(outcome: TurnOutcome) -> CanonicalChatOutcome {
    match outcome {
        TurnOutcome::Completed(response) => {
            CanonicalChatOutcome::Completed(CanonicalChatResponse {
                session: response.session,
                request: response.request.to_string(),
                trace_id: response.trace.trace.to_string(),
                text: response.text,
                served_by: response.served_by.to_string(),
                rounds: response.rounds,
                usage: response.usage,
                trace: response.trace.clone(),
                events: events_from_trace(&response.trace),
            })
        }
        TurnOutcome::PendingApproval(view) => {
            CanonicalChatOutcome::PendingApproval(CanonicalPendingApproval::from(view))
        }
    }
}

fn parse_approval_decision(
    decision: &str,
    reason: Option<String>,
) -> Result<ApprovalDecision, CanonicalEntryError> {
    match decision.trim().to_ascii_lowercase().as_str() {
        "approve" => Ok(ApprovalDecision::Approve),
        "reject" => Ok(ApprovalDecision::Reject { reason }),
        "cancel" => Ok(ApprovalDecision::Cancel { reason }),
        other => Err(CanonicalEntryError::InvalidRequest(format!(
            "unknown approval decision {other:?}; expected approve, reject, or cancel"
        ))),
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatRequest {
    model: Option<String>,
    messages: Vec<OpenAiMessage>,
    #[serde(default)]
    session_id: Option<SessionId>,
    #[serde(default)]
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OpenAiChatResponse {
    id: String,
    object: &'static str,
    created: i64,
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: NormalizedUsage,
    apeireth: OpenAiExecutionMetadata,
}

#[derive(Debug, Serialize)]
struct OpenAiChoice {
    index: u32,
    message: OpenAiAssistantMessage,
    finish_reason: &'static str,
}

#[derive(Debug, Serialize)]
struct OpenAiAssistantMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiExecutionMetadata {
    session_id: String,
    trace_id: String,
    served_by: String,
    rounds: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    events: Vec<CanonicalExecutionEvent>,
    /// Present on the final streaming chunk (the non-stream body carries
    /// `usage` at the top level instead).
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<NormalizedUsage>,
}

#[derive(Debug, Serialize)]
struct OpenAiStreamChunk {
    id: String,
    object: &'static str,
    created: i64,
    model: String,
    choices: Vec<OpenAiStreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    apeireth: Option<OpenAiExecutionMetadata>,
    /// OpenAI-conventional top-level usage (final chunk only).
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<NormalizedUsage>,
}

#[derive(Debug, Serialize)]
struct OpenAiStreamChoice {
    index: u32,
    delta: OpenAiStreamDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    finish_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct OpenAiStreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Debug, Serialize)]
struct ModelListItem {
    id: String,
    object: &'static str,
    created: i64,
    owned_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct ModelListResponse {
    object: &'static str,
    data: Vec<ModelListItem>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorFrame,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

type HttpError = (StatusCode, Json<ErrorBody>);

/// Assemble the shared gateway state (runtime + optional panels + event bus).
pub fn build_gateway_state(
    runtime: Arc<Runtime>,
    panels: Option<Arc<dyn PanelData>>,
) -> GatewayState {
    build_gateway_state_with_services(runtime, GatewayServices::from_panel(panels))
}

/// Assemble gateway state from bounded-context ports supplied by the
/// composition root. This is the production path; the legacy `PanelData`
/// adapter above exists only for compatibility with older embedders/tests.
pub fn build_gateway_state_with_services(
    runtime: Arc<Runtime>,
    services: GatewayServices,
) -> GatewayState {
    let events = EventBus::default();
    let observations = Arc::new(crate::events::RuntimeObservationSink::new(
        services.trace_commands.clone(),
        services.audit_commands.clone(),
    ));
    let presence = crate::presence::PresenceService::new(events.clone());
    runtime.add_event_sink(Arc::new(
        apeireth_runtime::canonical::CompositeRuntimeEventSink::new(vec![
            Arc::new(events.clone()),
            observations.clone(),
            presence.clone(),
        ]),
    ));
    // 在场帧面 (资源租约示范接线): 装配面持一枚保活 pin —— 首持有者开流
    // (心跳产出起), 状态回收即末持有者关停 (产出止), 无幽灵心跳。
    let presence_stream = Arc::new(
        presence
            .keepalive()
            .expect("fresh presence surface cannot be shut down"),
    );
    // SSE 交付面 (逐连接租约): 装配 pin 保活, 帧留存跨连接空窗, 每次连接
    // 都能即读当前快照帧。
    let events_stream = Arc::new(
        events
            .keepalive()
            .expect("fresh events surface cannot be shut down"),
    );
    // 观测攒批面 (flush 循环租约): 装配 pin 保活 —— flush 循环随装配存活,
    // 装配回收即循环止, 无幽灵 pass。
    let observations_stream = Arc::new(
        observations
            .keepalive()
            .expect("fresh flush surface cannot be shut down"),
    );
    // H1/G10 令牌门口径在装配期解析一次 (env → 三态)。逃生阀
    // (`APEIRETH_GATEWAY_TOKEN=none`) 是操作者的显式声明 —— 装配即入审计,
    // 之后每次放行另留痕。
    let token_gate = GatewayTokenGate::from_env();
    if token_gate == GatewayTokenGate::UnprotectedDeclared {
        observations.buffer_audit(
            "gateway.auth_declared_unprotected",
            Some(format!(
                "{}={}: operator explicitly declared an unprotected gateway (escape valve)",
                GatewayTokenGate::ENV,
                GatewayTokenGate::UNPROTECTED_SENTINEL
            )),
        );
    }
    GatewayState {
        runtime,
        services,
        events,
        events_stream,
        observations,
        observations_stream,
        presence,
        presence_stream,
        hot_config: Arc::new(RwLock::new(GatewayRuntimeConfig::from_env())),
        token_gate,
        // 客户端令牌表由组合根显式装载注入 (坏配置拒开在可失败边界执行);
        // 装配面不隐式读配置面 —— 见 `client_tokens` 字段文档。
        client_tokens: crate::client_tokens::ClientTokenTable::default(),
        // 用户角色面 (W3-②) 同一口径: 未注入 = 角色面未激活 (兼容路径),
        // 坏表/绑定不一致由组合根在可失败边界拒开 —— 见 `user_access` 字段文档。
        user_access: crate::client_tokens::UserAccess::default(),
        // 用户域平面 (W3-①) 同一口径: 未注入 = 域面未接线 (兼容路径,
        // 一切按默认域原位语义) —— 组合根在可失败边界装载注入。
        domains: None,
        // 逐域伙伴运行时实例面 (W3-③) 同一口径: 未注入 = 非默认域 chat 面
        // 拒开 (fail-closed) —— 组合根在可失败边界装载注入。
        domain_runtimes: None,
        shutdown: Arc::new(crate::shutdown::ShutdownTrigger::default()),
    }
}

/// Build the production HTTP router around one long-lived canonical runtime.
///
/// Panel routes answer `501 unsupported` while no [`PanelData`] is attached —
/// see [`canonical_router_with_panels`].
pub fn canonical_router(runtime: Arc<Runtime>) -> Router {
    canonical_router_with_services(runtime, GatewayServices::default())
}

/// Build the production router with optional panel/introspection backends.
pub fn canonical_router_with_panels(
    runtime: Arc<Runtime>,
    panels: Option<Arc<dyn PanelData>>,
) -> Router {
    canonical_router_with_state(build_gateway_state(runtime, panels))
}

/// Build the production router over explicit bounded-context gateway ports.
pub fn canonical_router_with_services(runtime: Arc<Runtime>, services: GatewayServices) -> Router {
    canonical_router_with_state(build_gateway_state_with_services(runtime, services))
}

// ---------------------------------------------------------------------------
// H1 (2026-09-24 安全审计): 本地来源白名单 + loopback 令牌门
// (G10 W1-③ 加固: 令牌门口径三态化, 桌面侧自铸令牌注入)
// ---------------------------------------------------------------------------

/// H1: CORS 只放行本地桌面件来源 (Tauri WebView 与本机开发服务器)。
///
/// gateway 零认证 + `CorsLayer::permissive` 的组合意味着用户浏览器里任意
/// 网页都能跨源调用本网关 —— 驱动 agent、代替主人批准工具调用
/// (`/v1/approvals/resolve`)、读取全部会话与记忆 (panel 族路由)、热改
/// provider base_url (`/v1/admin/config`)。本谓词把可跨源访问本 gateway
/// 的页面收敛到 Tauri 桌面件与本机回环开发服务器。
/// **跨源请求闸**：带 `Origin` 且不在本地白名单 ⇒ **拒绝**（403）。
///
/// # 为什么需要它（CORS 层不够）
///
/// `tower_http::cors::CorsLayer` 只**加响应头**，它**不拒绝**非预检请求 ⇒
/// ⇒ 跨源的 **simple POST**（无预检）**照样抵达 handler** ⇒
/// ⇒ 浏览器读不到响应，但**副作用照样发生**（例如 `POST /v1/admin/shutdown`）。
///
/// ⇒ 本层补上「**拒绝**」这一半：请求带 `Origin` 时按 [`is_allowed_local_origin`] 判定。
///
/// # 口径
///
/// * **只拦状态变更方法**（POST / PUT / PATCH / DELETE）
///   ⇒ 读方法（GET / HEAD）与预检（OPTIONS）**维持既有 CORS 语义**
///   （跨源读返回 200 但**不带** `access-control-allow-origin` ⇒ 浏览器自己挡）
///   ⇒ ⇒ **改动面最小**（既有的 `the_gateway_refuses_cors_for_non_local_origins` 语义不变），
///   而**危险面**（跨源触发的状态变更）被真正堵上；
/// * **不带 `Origin`** ⇒ **放行**（CLI / 服务端 / 脚本等非浏览器客户端 ⇒ 本来就没有这个头）；
/// * 带 `Origin` 且不在白名单 ⇒ **403**；
/// * 层序：**在令牌门之内** ⇒ ⇒ 未通过令牌门时**仍是 401**
///   ⇒ 不借 403/401 的差别泄露路由存在性（与既有纪律一致）。
async fn reject_foreign_origin(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let mutates_state = !matches!(
        method,
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    );
    if mutates_state {
        if let Some(origin) = req.headers().get(axum::http::header::ORIGIN) {
            if !is_allowed_local_origin(origin) {
                return (
                    axum::http::StatusCode::FORBIDDEN,
                    "cross-origin request rejected: origin is not allowed",
                )
                    .into_response();
            }
        }
    }
    next.run(req).await
}

fn is_allowed_local_origin(origin: &axum::http::HeaderValue) -> bool {
    let Ok(text) = origin.to_str() else {
        return false;
    };
    // Windows WebView2 的 Tauri 源是裸 `tauri://localhost` (无 path 可借)。
    if text == "tauri://localhost" {
        return true;
    }
    // 其余按 `<scheme>://<host>[:port]` 手工解析 (gateway 无 url 依赖)。
    let Some((scheme, rest)) = text.split_once("://") else {
        return false;
    };
    if !matches!(scheme, "http" | "https") {
        return false;
    }
    // host 段 = authority 去掉 userinfo / path / query / fragment。
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let host = match authority.rsplit_once(':') {
        // 只把"最后一段是纯数字"当端口剥掉; IPv6 字面量 [::1]:8080 同理。
        Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => host,
        _ => authority,
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    matches!(host, "tauri.localhost" | "localhost" | "127.0.0.1" | "::1")
}

/// H1/G10/W2-②: API 面令牌门的三态口径 (env `APEIRETH_GATEWAY_TOKEN` +
/// 客户端令牌表 [`crate::client_tokens::ClientTokenTable`])。
///
/// **W2-② 认证面全网关化**: 门从两个敏感端点扩到**全 API 面** (chat / 事件流 /
/// 面板 / 会话 / 审批 / admin 族), 唯一显式豁免是健康探活
/// [`UNAUTHENTICATED_PROBE_PATHS`] (探测刚需, 见其文档)。三态语义沿用,
/// 「设/未设」按**整个令牌族**判定:
///
/// - [`GatewayTokenGate::Enforced`] — env 带密钥值: 令牌族在场, 每次调用必须
///   带 `Authorization: Bearer <token>` (恒定时间比较), 呈现的令牌可以是 env
///   单令牌 (审计客户端名 [`GatewayTokenGate::ENV_FACE_CLIENT_NAME`]) 或令牌表
///   内任一枚 (审计客户端名 = 该条目 `name`); 缺失/错误分别 401
///   `auth_missing_key` / `auth_invalid_key`;
/// - [`GatewayTokenGate::UnprotectedDefault`] — **env 未设且令牌表为空
///   (收紧后的默认态)**: 无认证调用仍放行 (保守过渡, 无令牌客户端零断), 但
///   每次放行记警告帧 (`gateway_auth_unprotected`) + 审计事实
///   (`gateway.auth_unprotected`); **令牌表配置了任一枚 = 令牌族在场, 照
///   `Enforced` 校验** (只认表内具名令牌);
/// - [`GatewayTokenGate::UnprotectedDeclared`] — **逃生阀** (env 显式取值
///   [`GatewayTokenGate::UNPROTECTED_SENTINEL`]): 操作者明示不设防, 放行且
///   不发警告帧, 但声明本身与每次放行都入审计 (留痕)。逃生阀与客户端令牌表
///   互斥 (配置矛盾拒开, 见
///   [`crate::client_tokens::validate_gate_table_pair`])。
///
/// 桌面壳启动侧车时自铸 32 字节随机令牌注入 env, 生产路径恒为 `Enforced`;
/// 未设/逃生阀只出现在 CLI 裸跑。令牌值不进日志、不进帧、不进 Debug。
#[derive(Clone, PartialEq, Eq)]
pub enum GatewayTokenGate {
    /// env 带真实令牌: 全 API 面强制 Bearer 匹配 (令牌族: env 单令牌 + 令牌表)。
    Enforced(String),
    /// env 未设且令牌表为空: 放行 + 警告帧 + 审计 (保守过渡, 催化迁移到带令牌)。
    UnprotectedDefault,
    /// env 显式 `none`: 逃生阀 —— 明示不设防, 审计留痕, 不逐次告警。
    UnprotectedDeclared,
}

/// 过门裁决 (纯结果, 中间件负责帧与审计)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateVerdict {
    /// 令牌命中: `client` 是审计署名键 (env 面单令牌 / 令牌表条目名)。
    Pass {
        /// 命中的客户端名。
        client: String,
    },
    /// 未设防口径下的放行: `mode` 是 `env_unset` 或 `declared_none`。
    PassUnprotected {
        /// 未设防模式 (进警告帧与审计明细)。
        mode: &'static str,
    },
    /// 强制态且没带 Bearer。
    RejectMissing,
    /// 强制态且 Bearer 不匹配令牌族任何一枚 (伪造 Bearer)。
    RejectInvalid,
}

impl GatewayTokenGate {
    /// 配置令牌门的环境变量名。
    pub const ENV: &str = "APEIRETH_GATEWAY_TOKEN";
    /// 逃生阀字面值: 显式声明「本网关不设防」。真实令牌不得取此值。
    pub const UNPROTECTED_SENTINEL: &str = "none";
    /// env 单令牌面在审计里的客户端名 (令牌表条目用各自 `name`)。
    pub const ENV_FACE_CLIENT_NAME: &str = "env";

    /// 从进程环境解析口径 (网关装配期解析一次)。
    pub fn from_env() -> Self {
        Self::parse(std::env::var(Self::ENV).ok())
    }

    /// 从原始 env 值解析口径: 哨兵值 = 显式不设防; 非空值 = 强制令牌;
    /// 缺失/空白 = 收紧后的默认态。
    pub fn parse(raw: Option<String>) -> Self {
        match raw.map(|value| value.trim().to_string()) {
            Some(value) if value == Self::UNPROTECTED_SENTINEL => Self::UnprotectedDeclared,
            Some(value) if !value.is_empty() => Self::Enforced(value),
            _ => Self::UnprotectedDefault,
        }
    }

    /// 门在生效时要求的 env 单令牌 (仅 [`GatewayTokenGate::Enforced`])。
    /// 令牌表条目是并列的通行证, 见 [`Self::verify`]。
    pub fn enforced_token(&self) -> Option<&str> {
        match self {
            Self::Enforced(token) => Some(token),
            Self::UnprotectedDefault | Self::UnprotectedDeclared => None,
        }
    }

    /// 令牌族是否在场 (env 单令牌或令牌表任一非空) —— 在场即强制校验。
    pub fn family_enforced(&self, clients: &crate::client_tokens::ClientTokenTable) -> bool {
        matches!(self, Self::Enforced(_)) || !clients.is_empty()
    }

    /// 过门裁决 (纯函数): `bearer` 是 `Authorization: Bearer ` 后面的裸令牌。
    ///
    /// 口径 (令牌族合并后):
    /// - `Enforced` — bearer 必须命中 env 单令牌或令牌表任一枚;
    /// - `UnprotectedDefault` — 令牌表为空 = 兼容路径放行; 令牌表非空 = 只认
    ///   表内具名令牌;
    /// - `UnprotectedDeclared` — 逃生阀放行 (配置矛盾已由装配闸拒开)。
    pub fn verify(
        &self,
        bearer: Option<&str>,
        clients: &crate::client_tokens::ClientTokenTable,
    ) -> GateVerdict {
        match self {
            Self::Enforced(expected) => match bearer {
                None => GateVerdict::RejectMissing,
                Some(token) => {
                    if constant_time_eq(token.as_bytes(), expected.as_bytes()) {
                        GateVerdict::Pass {
                            client: Self::ENV_FACE_CLIENT_NAME.to_string(),
                        }
                    } else {
                        match clients.authenticate(token) {
                            Some(name) => GateVerdict::Pass {
                                client: name.to_string(),
                            },
                            None => GateVerdict::RejectInvalid,
                        }
                    }
                }
            },
            Self::UnprotectedDefault => {
                if clients.is_empty() {
                    GateVerdict::PassUnprotected { mode: "env_unset" }
                } else {
                    // 令牌族在场即强制: 表内具名令牌是唯一通行证。
                    match bearer {
                        None => GateVerdict::RejectMissing,
                        Some(token) => match clients.authenticate(token) {
                            Some(name) => GateVerdict::Pass {
                                client: name.to_string(),
                            },
                            None => GateVerdict::RejectInvalid,
                        },
                    }
                }
            }
            Self::UnprotectedDeclared => GateVerdict::PassUnprotected {
                mode: "declared_none",
            },
        }
    }
}

/// 令牌永不进 Debug/日志: 值一律打码。
impl std::fmt::Debug for GatewayTokenGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enforced(_) => write!(f, "Enforced(<redacted>)"),
            Self::UnprotectedDefault => write!(f, "UnprotectedDefault"),
            Self::UnprotectedDeclared => write!(f, "UnprotectedDeclared"),
        }
    }
}

/// 恒定时间字节比较 (防时序侧信道)。
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// 自称名的有界审计渲染 (请求头值是攻击方可控输入, 不无界进审计行)。
pub(crate) fn truncate_label(claimed: &str) -> String {
    let mut bounded: String = claimed.chars().take(64).collect();
    if bounded.chars().count() < claimed.chars().count() {
        bounded.push('…');
    }
    bounded
}

/// 无认证探活端点: 令牌门对这些路径**整体豁免** (不校验、不警告、不审计)。
///
/// 健康探活是探测刚需 (服务管理器 / 桌面侧车监督 / 服务脚本的就绪探针都只打
/// `/health`, 且浏览器 `EventSource` 之外的探针多无法带凭据) —— 认证面上**唯一**
/// 的显式豁免就是这张表。改这张表 = 改认证面的权力边界, 由
/// `gateway_auth_face` 测试钉住。
pub const UNAUTHENTICATED_PROBE_PATHS: &[&str] = &["/health"];

/// H1/G10/W2-②/W2-③: 全 API 面的令牌门中间件 (口径由 [`GatewayTokenGate`] 给出)。
///
/// - `Enforced` (或令牌表在场): 缺失/错误的 Bearer 令牌分别返回 401
///   `auth_missing_key` / `auth_invalid_key`, 拒绝入审计 (`gateway.auth_rejected`);
///   命中则放行并入审计 (`gateway.auth_client`), **审计带客户端名**;
/// - `UnprotectedDefault` (env 未设且令牌表为空): 放行但每次记警告帧 + 审计事实;
/// - `UnprotectedDeclared` (逃生阀 `none`): 放行, 审计留痕, 不逐次告警。
///
/// W2-③ 令牌隔离: 过门身份 ([`ClientIdentity`]) 随请求扩展下行给处理器
/// (署名键**只来自令牌绑定**); 请求可带自称名头
/// [`CLIENT_NAME_CLAIM_HEADER`] 做 name↔token 绑定校验 —— 自称名与令牌绑定名
/// 不一致、或在无绑定的未设防口径下自称名, 一律 401 拒绝 (冒名 fail-closed,
/// 审计署名不可冒名)。
///
/// [`UNAUTHENTICATED_PROBE_PATHS`] 显式豁免。任何分支都不把令牌值写进帧、
/// 审计或错误正文。
async fn require_gateway_token(
    State(state): State<GatewayState>,
    mut request: Request,
    next: Next,
) -> Response {
    let endpoint = request.uri().path().to_string();
    if UNAUTHENTICATED_PROBE_PATHS.contains(&endpoint.as_str()) {
        return next.run(request).await;
    }
    let bearer = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_string);
    // W2-③: 客户端自称名 (可选) —— 只做绑定校验, 永不改写审计署名。
    let claimed = request
        .headers()
        .get(CLIENT_NAME_CLAIM_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    match state
        .token_gate
        .verify(bearer.as_deref(), &state.client_tokens)
    {
        GateVerdict::Pass { client } => {
            if let Some(claimed) = &claimed {
                if !client_name_binds(claimed, Some(client.as_str())) {
                    // 冒名即拒 (错误即帧): 自称名必须与令牌绑定名一致。
                    state.observations.buffer_audit(
                        "gateway.auth_rejected",
                        Some(format!(
                            "endpoint={endpoint} reason=client_name_mismatch client={client} \
                             claimed={}",
                            truncate_label(claimed)
                        )),
                    );
                    return ErrorFrame::response(
                        StatusCode::UNAUTHORIZED,
                        ErrorCode::AuthInvalidKey,
                        "client name does not match the presented credential",
                    )
                    .into_response();
                }
            }
            // 过门留痕: 谁 (客户端名) 调了哪个端点 —— W3 多用户授权的问责地基。
            state.observations.buffer_audit(
                "gateway.auth_client",
                Some(format!("endpoint={endpoint} client={client}")),
            );
            request
                .extensions_mut()
                .insert(ClientIdentity::authenticated(client));
            next.run(request).await
        }
        GateVerdict::PassUnprotected { mode } => {
            if let Some(claimed) = &claimed {
                // 无凭证即无绑定: 自称名不可校验 → 拒 (审计署名不可冒名)。
                state.observations.buffer_audit(
                    "gateway.auth_rejected",
                    Some(format!(
                        "endpoint={endpoint} reason=client_name_unverifiable mode={mode} \
                         claimed={}",
                        truncate_label(claimed)
                    )),
                );
                return ErrorFrame::response(
                    StatusCode::UNAUTHORIZED,
                    ErrorCode::AuthInvalidKey,
                    "a claimed client name requires a token binding to verify against",
                )
                .into_response();
            }
            if mode == "env_unset" {
                // 收紧后的默认态 (保守过渡): 无认证调用仍放行 (无令牌客户端零断),
                // 但不再是静默 —— 每次放行记警告帧 + 审计事实。
                state.events.publish(GatewayEvent::new(
                    "gateway_auth_unprotected",
                    serde_json::json!({
                        "endpoint": endpoint,
                        "mode": mode,
                        "detail": "gateway token not configured; an unauthenticated call reached an API endpoint",
                    }),
                ));
            }
            // 逃生阀 (明示不设防): 不逐次告警, 但每次放行留审计痕。
            state.observations.buffer_audit(
                "gateway.auth_unprotected",
                Some(format!("endpoint={endpoint} mode={mode}")),
            );
            request.extensions_mut().insert(ClientIdentity::anonymous());
            next.run(request).await
        }
        GateVerdict::RejectMissing => {
            state.observations.buffer_audit(
                "gateway.auth_rejected",
                Some(format!("endpoint={endpoint} reason=missing")),
            );
            ErrorFrame::response(
                StatusCode::UNAUTHORIZED,
                ErrorCode::AuthMissingKey,
                "gateway token required for this endpoint",
            )
            .into_response()
        }
        GateVerdict::RejectInvalid => {
            state.observations.buffer_audit(
                "gateway.auth_rejected",
                Some(format!("endpoint={endpoint} reason=invalid")),
            );
            ErrorFrame::response(
                StatusCode::UNAUTHORIZED,
                ErrorCode::AuthInvalidKey,
                "gateway token invalid",
            )
            .into_response()
        }
    }
}

/// W3-② 角色门分类: 方法 + 路径 → 所需能力 (治理层 role 权限矩阵的判定入口)。
///
/// - 管理面 (admin 族 / 面板族管理面: 授权热撤 + 用户与域管理列表, 见
///   [`crate::panels::panel_admin_face`])
///   → [`UserCapability::AdminControl`] (owner 档);
/// - 审批解决 (`/v1/approvals/resolve`) → [`UserCapability::ApprovalL2`]
///   (跨用户归属另闸, 见 [`cross_user_approval_refusal`]);
/// - 其余非读方法 (chat / 会话写 / 记忆写) → [`UserCapability::Write`]
///   (guest 零写面);
/// - 读方法 (`GET`/`HEAD`) → 无能力要求 (矩阵全员持 [`UserCapability::Read`])。
fn required_user_capability(method: &axum::http::Method, path: &str) -> Option<UserCapability> {
    if path.starts_with("/v1/admin") || crate::panels::panel_admin_face(path) {
        return Some(UserCapability::AdminControl);
    }
    if path == "/v1/approvals/resolve" {
        return Some(UserCapability::ApprovalL2);
    }
    match *method {
        axum::http::Method::GET | axum::http::Method::HEAD => None,
        _ => Some(UserCapability::Write),
    }
}

/// W3-② 角色门 (整面中间件, 挂在令牌门**之内**): 令牌门认「你是谁」,
/// 角色门判「你能不能」。
///
/// - **角色面未激活** (未注入用户表) = 兼容路径: W2-② 行为逐字不变,
///   不新增拒绝面 (零回归口径);
/// - **角色面激活**: 过门客户端名 → 令牌→用户→角色链 → 治理层 role 权限矩阵;
///   写面/管理面/审批面对**未绑定用户** fail-closed 拒 (403 `permission_denied`);
///   每次裁决 (allow/deny) 落一行 `gateway.user_action` 审计 (user/role/action)。
///
/// 角色只来自令牌绑定的解析链 ([`crate::client_tokens::resolve_user_identity`]),
/// 请求无法自称角色 —— 成员冒名 owner 在这里无从发生 (自称客户端名另有
/// name↔token 绑定校验, 在令牌门内已拒)。
async fn require_user_capability(
    State(state): State<GatewayState>,
    mut request: Request,
    next: Next,
) -> Response {
    if !state.user_access.is_active() {
        request.extensions_mut().insert(UserContext::unbound());
        return next.run(request).await;
    }
    let client = request
        .extensions()
        .get::<ClientIdentity>()
        .map(|identity| identity.client.clone())
        .unwrap_or_else(|| ClientIdentity::UNATTRIBUTED.to_string());
    let user = state.user_access.identity(&client, &state.client_tokens);
    let context = UserContext { user };
    let endpoint = request.uri().path().to_string();
    if let Some(capability) = required_user_capability(request.method(), &endpoint) {
        let allowed = context.allows(capability);
        // V2-c 域标注: 授权裁决 = 行为域 (行为者所在域)。
        let domain_label = crate::domain_context::action_domain_label(&state, context.user_id());
        record_user_action(
            Some(state.observations.as_ref()),
            context.user_id(),
            context.role(),
            capability.label(),
            &endpoint,
            allowed,
            domain_label.as_deref(),
        );
        if !allowed {
            let message = if context.user_id().is_some() {
                format!(
                    "role gate refused {endpoint}: the bound user role may not perform {}",
                    capability.label()
                )
            } else {
                format!("role gate refused {endpoint}: the calling client is not bound to any user")
            };
            return ErrorFrame::response(
                StatusCode::FORBIDDEN,
                ErrorCode::PermissionDenied,
                message,
            )
            .into_response();
        }
    }
    request.extensions_mut().insert(context);
    next.run(request).await
}

/// 会话→用户绑定 (W3-②): 首个具名写者绑定会话归属, 供跨用户审批拒判定。
fn bind_session_owner(
    state: &GatewayState,
    session: SessionId,
    user: Option<&Extension<UserContext>>,
) {
    if let Some(Extension(context)) = user {
        if let Some(user_id) = context.user_id() {
            state.user_access.bind_session_user(session, user_id);
        }
    }
}

/// 跨用户审批拒 (G1 对抗面): 角色面激活时, 审批解决必须过归属闸 —— owner 全权
/// (可批任意会话的审批); 其余角色只可批**本用户绑定会话**的审批; 未绑定用户/
/// 无归属会话 fail-closed 拒 (403 + `gateway.user_action` 审计)。
/// 返回 `Some(拒绝帧)` = 不得继续解决审批。
fn cross_user_approval_refusal(
    state: &GatewayState,
    session: SessionId,
    user: Option<&Extension<UserContext>>,
) -> Option<Response> {
    if !state.user_access.is_active() {
        return None;
    }
    let context = user.map(|Extension(context)| context);
    let subject = state.user_access.session_user(session);
    let allowed = context
        .and_then(|context| context.user.as_ref())
        .is_some_and(|identity| {
            may_resolve_approval(identity.role, &identity.user_id, subject.as_deref())
        });
    if allowed {
        return None;
    }
    // V2-c 域标注: 授权裁决 = 行为域 (行为者所在域)。
    let actor = context.and_then(|context| context.user_id());
    let domain_label = crate::domain_context::action_domain_label(state, actor);
    record_user_action(
        Some(state.observations.as_ref()),
        actor,
        context.and_then(|context| context.role()),
        "approval_ownership",
        "/v1/approvals/resolve",
        false,
        domain_label.as_deref(),
    );
    Some(
        ErrorFrame::response(
            StatusCode::FORBIDDEN,
            ErrorCode::PermissionDenied,
            format!(
                "role gate refused /v1/approvals/resolve: approval for session {session} \
                 belongs to another user (only its owner may resolve it)"
            ),
        )
        .into_response(),
    )
}

/// 审批归属域校验 (V2-c: 跨用户审批闸 W3-② 的**跨域**升级)。
///
/// 审批是会话的资源级归属: 域面接线时, 目标会话必须在**本域账本在册** ——
/// 他域会话与不存在的会话 id **同帧** 404 `session_not_found` (存在性不泄露,
/// W3-① 资源级跨域语义, 与 `/v1/sessions/*`、`?session=` 事件订阅同一口径)。
/// 在册探测失败按不在册处理 (fail-closed, 宁拒不泄)。owner 的全局视角只在
/// users/domains 管理面 (V2-c), **审批面无跨域通路** (升级前 owner 可批任意
/// 会话的审批, 升级后跨域同帧 404)。
///
/// 域面未接线 = 不校验 (W2/W3-② 既有审批语义逐字零变化)。
/// 返回 `Some(帧)` = 不得继续读/解该审批。
async fn approval_session_refusal(
    state: &GatewayState,
    domain: Option<&Extension<crate::domain_context::DomainContext>>,
    endpoint: &str,
    session: SessionId,
) -> Option<Response> {
    use crate::domain_context::domain_refused_frame;

    let Some(plane) = state.domains.as_ref() else {
        return None;
    };
    let Some(bound) = domain.and_then(|Extension(context)| context.domain.as_ref()) else {
        return None;
    };
    let manager = match plane.sessions(bound) {
        Ok(manager) => manager,
        Err(error) => return Some(domain_refused_frame(state, endpoint, &error)),
    };
    // 他域/不存在同帧读作不存在 (跨域归属拒 = 资源级 404, 不发 403 泄露存在性)。
    if manager.contains(&session).await.unwrap_or(false) {
        return None;
    }
    Some(ErrorFrame::response(
        StatusCode::NOT_FOUND,
        ErrorCode::SessionNotFound,
        format!("session {session} does not exist"),
    ))
}

/// Build the production router over an explicit [`GatewayState`] (lets tests
/// keep a handle on the event bus).
pub fn canonical_router_with_state(state: GatewayState) -> Router {
    // W2-② 认证面全网关化: 令牌门是**整面**中间件 (chat / 事件流 / 面板 / 会话 /
    // 审批 / admin 族全过门), 不再逐端点挂 route_layer —— 新路由默认在门内,
    // 漏挂面为零; 唯一豁免是 [`UNAUTHENTICATED_PROBE_PATHS`] (健康探活)。
    // 中间件自带一份门状态 (口径 + 警告/审计面), 所以 from_fn_with_state。
    let gate_state = state.clone();
    Router::<GatewayState>::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/providers", get(list_providers))
        .route("/v1/runtime/snapshot", get(runtime_snapshot))
        .route("/v1/apeireth/runtime/snapshot", get(runtime_snapshot))
        .route("/v1/chat", post(native_chat))
        .route("/v1/chat/completions", post(openai_chat))
        .route("/v1/approvals", get(list_pending_approvals))
        .route("/v1/approvals/resolve", post(native_resolve_approval))
        .route(
            "/v1/sessions/:session_id/settings",
            get(crate::session_settings::get_session_settings)
                .patch(crate::session_settings::patch_session_settings),
        )
        .route(
            "/v1/sessions/:session_id",
            axum::routing::delete(crate::session_settings::delete_session),
        )
        .route("/v1/apeireth/events", get(events_handler))
        .route(
            "/v1/apeireth/presence",
            get(crate::presence::presence_snapshot_handler),
        )
        .route(
            "/v1/admin/config",
            get(admin_config_get).post(admin_config_update),
        )
        // W2-① 常驻形态: 优雅停机控制面 (计划任务 stop 走这里, 控制台信号
        // 不可靠)。与全 API 面同一令牌门同一三态口径。
        .route("/v1/admin/shutdown", post(admin_shutdown))
        // V2-b 用户/域管理面 (W3-②/W3-①, owner 档): 建户/改角色/删户/开域四口
        // 走 admin 族 (整面前缀 → AdminControl 角色门), 与 CLI `apeireth user`
        // 同一 UserTable 服务 (双入口同源); 列表两口在面板族 (panel_routes)。
        .route("/v1/admin/users", post(crate::user_admin::create_user))
        .route(
            "/v1/admin/users/:user_id",
            patch(crate::user_admin::patch_user).delete(crate::user_admin::delete_user),
        )
        .route(
            "/v1/admin/domains",
            post(crate::domain_admin::create_domain),
        )
        .merge(panel_routes())
        // G-P1 指标端点 (指标数据面路由文件), 同在整面令牌门内。
        .merge(crate::panel_metrics::metrics_routes())
        // W3-① 域门 (数据域面): user_id 域解析 + 自称域校验 + 面闸, 过闸后
        // `DomainContext` 随请求扩展下行 (会话面按域路由)。挂在令牌门与角色门
        // **之内** (先认人、再判权、后判域); 域面未注入 = 兼容路径零回归。
        .layer(middleware::from_fn_with_state(
            gate_state.clone(),
            crate::domain_context::require_domain_context,
        ))
        // W3-② 角色门 (授权面): 挂在令牌门**之内** —— 先认人 (authn) 再判权
        // (authz)。角色面未注入 = 兼容路径零回归; 激活后按治理层 role 权限矩阵
        // 过闸 (写面/管理面/审批面), 每次裁决落 `gateway.user_action` 审计。
        .layer(middleware::from_fn_with_state(
            gate_state.clone(),
            require_user_capability,
        ))
        // 跨源请求闸 (2026-10-31 · 静态安全审计复核): CORS 层只加头、**不拒绝**
        // ⇒ 跨源 simple POST 的副作用照样发生 ⇒ 这里补「拒绝」那一半。
        // 层序: **令牌门之内** ⇒ 未过令牌门仍回 401 (不借 403/401 差别泄露路由存在性)。
        .layer(middleware::from_fn(reject_foreign_origin))
        // 令牌门包住整个 API 面 (含未匹配路径的 fallback: 强制态下未知路径也
        // 401, 不借 404/401 差别泄露路由存在性)。
        .layer(middleware::from_fn_with_state(
            gate_state,
            require_gateway_token,
        ))
        // CORS 白名单 (H1, 2026-09-24 安全审计): 桌面 WebView 是独立源
        // (tauri://localhost / http://tauri.localhost), 浏览器对回环地址同样
        // 执行跨源策略 —— 没有这一层每个 UI fetch 都会被拒 (2026-09-28 真机
        // 故障), 但 **permissive** 会让用户浏览器里任意网页获得同等能力
        // (驱动 agent / 批准工具 / 读会话与记忆)。收敛为本地来源白名单:
        // Tauri 源 + 本机回环开发服务器 (任意端口)。LAN 暴露部署
        // (非回环绑定) 由远程绑定闸强制令牌 (见 `remote_access`), 并叠加
        // 令牌门全端点校验。
        //
        // 层序: CORS 在令牌门**之外** —— 浏览器预检 (OPTIONS, 规范上不带
        // 凭据) 由 CORS 直接应答, 不会被令牌门 401 拦死; 真实请求仍过门。
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|origin, _request_parts| {
                    is_allowed_local_origin(origin)
                }))
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state)
}

/// Serve the canonical gateway until the listener closes.
pub async fn serve_canonical(
    listener: tokio::net::TcpListener,
    runtime: Arc<Runtime>,
    panels: Option<Arc<dyn PanelData>>,
) -> std::io::Result<()> {
    serve_canonical_with_services(listener, runtime, GatewayServices::from_panel(panels)).await
}

/// Serve the canonical gateway over explicit bounded-context gateway ports.
pub async fn serve_canonical_with_services(
    listener: tokio::net::TcpListener,
    runtime: Arc<Runtime>,
    services: GatewayServices,
) -> std::io::Result<()> {
    serve_with_shutdown(listener, runtime, services, std::future::pending::<()>()).await
}

/// Serve the canonical gateway until the listener closes **or** a shutdown is
/// requested (W2-① 常驻形态的优雅停机面).
///
/// Shutdown sources funnel into one policy:
///
/// - `external_shutdown` — the composition root's process-signal face
///   (`SIGTERM` / console control events);
/// - [`crate::shutdown::ShutdownTrigger`] — fired by the token-gated
///   `POST /v1/admin/shutdown` endpoint (the service scripts' stop path).
///
/// When either fires, the server stops accepting new work, waits for in-flight
/// requests to finish, then performs the closing bookkeeping (收尾): one final
/// `gateway.shutdown` audit fact and a full flush of the observation archive to
/// its port (the `daemon-audit.jsonl` archive in the production assembly), so
/// the last facts of the process are on disk before it exits (落盘).
pub async fn serve_with_shutdown(
    listener: tokio::net::TcpListener,
    runtime: Arc<Runtime>,
    services: GatewayServices,
    external_shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let state = build_gateway_state_with_services(runtime, services);
    serve_state_with_shutdown(listener, state, external_shutdown).await
}

/// Serve over an explicit [`GatewayState`] until the listener closes or a
/// shutdown fires (see [`serve_with_shutdown`] for the shutdown contract).
///
/// TLS 面 (B5 真接线): `APEIRETH_TLS_CERT` / `APEIRETH_TLS_KEY` 旋钮在场时
/// **真加载**材料并走 TLS listener; 材料坏 = fail-closed **拒绝启动** ——
/// 错误在任何端点被服务前帧化返回, 绝不静默降级明文。未设旋钮 = 明文,
/// 回环缺省语义不变。
///
/// Keeping the state explicit lets embedders and tests hold the observation
/// sink handle across the shutdown boundary — which is exactly what the
/// closing-bookkeeping assertions pin.
pub async fn serve_state_with_shutdown(
    listener: tokio::net::TcpListener,
    state: GatewayState,
    external_shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let tls = crate::remote_access::resolve_tls_material().map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("TLS configuration refused (fail-closed, never plaintext): {error}"),
        )
    })?;
    serve_state_with_tls(listener, state, tls, external_shutdown).await
}

/// [`serve_state_with_shutdown`] 的显式材料形态: `tls = Some(..)` 走 TLS
/// listener, `None` 保持明文 —— 组合根/测试可直接给材料, 不经环境旋钮。
pub async fn serve_state_with_tls(
    listener: tokio::net::TcpListener,
    state: GatewayState,
    tls: Option<crate::remote_access::TlsMaterial>,
    external_shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let endpoint = listener
        .local_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    state.events.publish(GatewayEvent::new(
        "backend_ready",
        serde_json::json!({ "service": "apeireth-gateway-2.0", "endpoint": endpoint }),
    ));
    let trigger = state.shutdown.clone();
    let shutdown = async move {
        tokio::select! {
            _ = external_shutdown => {},
            _ = trigger.fired() => {},
        }
    };
    let router = canonical_router_with_state(state.clone());
    let result = match tls {
        Some(material) => serve_tls_listener(listener, material, router, shutdown).await,
        None => {
            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown)
                .await
        }
    };

    // 优雅停机收尾: 最后一桩审计 + 观测攒批全量落盘, 之后进程才真正退出。
    state.events.publish(GatewayEvent::new(
        "backend_stopping",
        serde_json::json!({ "endpoint": endpoint }),
    ));
    state.observations.buffer_audit(
        "gateway.shutdown",
        Some(format!("endpoint={endpoint} graceful=1")),
    );
    state.observations.flush().await;
    result
}

/// TLS 面的服务循环 (真接线): 每条 TCP 连接先过 TLS 握手, 再交同一路由面。
///
/// 停机政策与明文面同款: 停止收新连接 → 在途连接收到信号后走 graceful
/// shutdown → 全部连接落地后返回, 由 [`serve_state_with_tls`] 做收尾账。
/// 握手失败的连接直接掐掉 —— 没有明文降级口子。
async fn serve_tls_listener(
    listener: tokio::net::TcpListener,
    material: crate::remote_access::TlsMaterial,
    router: Router,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let acceptor = tokio_rustls::TlsAcceptor::from(material.server_config());

    // 停机信号面: 信号任务退出 = 全体连接任务收到通知 (watch closed 语义)。
    let (signal_tx, signal_rx) = tokio::sync::watch::channel(());
    let signal_tx = Arc::new(signal_tx);
    tokio::spawn(async move {
        shutdown.await;
        drop(signal_rx);
    });
    // 在途连接计数面: 每个连接任务持一把 close_rx, 全落地后 closed() 放行。
    let (close_tx, close_rx) = tokio::sync::watch::channel(());

    loop {
        let accepted = tokio::select! {
            conn = listener.accept() => conn,
            _ = signal_tx.closed() => break,
        };
        let (tcp_stream, _remote_addr) = match accepted {
            Ok(conn) => conn,
            Err(error) if is_transient_accept_error(&error) => continue,
            // 资源类瞬时错误 (如句柄耗尽): 睡一秒再收, 不把风暴当致命错退服务。
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        let acceptor = acceptor.clone();
        let service = router.clone();
        let signal_tx = Arc::clone(&signal_tx);
        let close_rx = close_rx.clone();
        tokio::spawn(async move {
            // 握手失败 = 掐连接 (不降级明文、不重试)。
            let Ok(tls_stream) = acceptor.accept(tcp_stream).await else {
                drop(close_rx);
                return;
            };
            let io = TokioIo::new(tls_stream);
            let hyper_service = TowerToHyperService::new(service);
            let builder = AutoConnBuilder::new(TokioExecutor::new());
            let conn = builder.serve_connection_with_upgrades(io, hyper_service);
            tokio::pin!(conn);
            loop {
                tokio::select! {
                    _result = conn.as_mut() => break,
                    _ = signal_tx.closed() => {
                        conn.as_mut().graceful_shutdown();
                    }
                }
            }
            drop(close_rx);
        });
    }

    drop(close_rx);
    drop(listener);
    close_tx.closed().await;
    Ok(())
}

/// 对端瞬断类 accept 错误不值得退服务; 其余 (句柄耗尽等) 睡后重试。
fn is_transient_accept_error(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
    )
}

/// W2-① 常驻形态: 优雅停机控制面 (`POST /v1/admin/shutdown`, 令牌门下)。
///
/// 幂等: 首次触发返回 `first`, 重复请求返回 `repeat` —— 服务脚本的 stop 重试
/// 不会再掀一次停机风暴。真正的停机收尾 (审计 + 落盘) 在 [`serve_with_shutdown`]
/// 的收尾段执行, 本端点只放闸。
async fn admin_shutdown(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    let first = state.shutdown.trigger();
    state
        .observations
        .buffer_audit("gateway.shutdown_requested", Some(format!("first={first}")));
    Json(serde_json::json!({
        "status": "shutting_down",
        "trigger": if first { "first" } else { "repeat" },
    }))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "execution_owner": "apeireth-runtime::canonical"
    }))
}

async fn list_models(
    State(state): State<GatewayState>,
) -> Result<Json<ModelListResponse>, Response> {
    let created = Timestamp::from_clock(state.runtime.clock().as_ref()).epoch_millis() / 1_000;
    // Dedupe by model id: two providers can serve the same canonical model over
    // different wires (the anthropic plugin defaults to MiniMax's
    // Anthropic-compatible gateway, so both it and the native minimax plugin
    // advertise `minimax-m3`). The UI model list needs one entry per id; model
    // resolution by the runtime is unaffected by this display layer.
    let mut seen = std::collections::HashSet::new();
    let data: Vec<ModelListItem> = state
        .runtime
        .providers()
        .model_descriptors()
        .into_iter()
        .filter(|model| seen.insert(model.id.to_string()))
        .map(|model| ModelListItem {
            id: model.id.to_string(),
            object: "model",
            created,
            owned_by: model.provider.to_string(),
            description: model.display_name.clone(),
        })
        .collect();

    if data.is_empty() {
        let has_key = state
            .hot_config
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .api_key
            .is_some();
        let (code, message) = if has_key {
            (
                ErrorCode::ProviderUnreachable,
                "没有可用的 provider/model; 请检查 base_url 与网络连通性".to_string(),
            )
        } else {
            (
                ErrorCode::AuthMissingKey,
                "未配置 API 密钥; 请在「设置 → 模型」中保存密钥后重试".to_string(),
            )
        };
        return Err(ErrorFrame::response(
            StatusCode::UNAUTHORIZED,
            code,
            message,
        ));
    }

    Ok(Json(ModelListResponse {
        object: "list",
        data,
    }))
}

async fn list_providers(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "object": "list",
        "data": state.runtime.snapshot().providers,
    }))
}

async fn runtime_snapshot(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    Json(serde_json::json!(state.runtime.snapshot()))
}

/// 请求过门身份 → 会话归属 (W2-③): 署名键只来自令牌绑定。
fn attribution_for(
    state: &GatewayState,
    identity: Option<&Extension<ClientIdentity>>,
) -> SessionAttribution {
    let client = identity
        .map(|Extension(identity)| identity.client.clone())
        .unwrap_or_else(|| ClientIdentity::UNATTRIBUTED.to_string());
    SessionAttribution::new(client, Some(Arc::clone(&state.observations)))
}

/// 回合输入的最小校验 (空输入不成回合)。抽出来是为了让会话立案
/// ([`admit_domain_session`]) 只发生在有效请求上 —— 400 请求不落空会话。
fn validate_chat_input(input: &str) -> Result<(), CanonicalEntryError> {
    if input.trim().is_empty() {
        return Err(CanonicalEntryError::InvalidRequest(
            "input must not be empty".into(),
        ));
    }
    Ok(())
}

/// W3-③ 一个域回合的执行目标: 跑在哪个运行时实例上 + 该域的运行时上下文
/// (模型/性格人设覆盖, 缺省回落全局)。
struct DomainChatExecution<'a> {
    /// 本回合的运行时实例 (默认域/域面未接线 = 现 [`Runtime`], 零变化)。
    runtime: &'a Arc<Runtime>,
    /// 该域的对话运行时上下文 (域文档覆盖, 缺省回落全局)。
    context: apeireth_runtime::canonical::DomainRuntimeContext,
}

impl DomainChatExecution<'_> {
    /// 生效模型 (缺省回落链): 请求显式 > 域覆盖 > admin 热配 > 运行时默认。
    fn resolve_model(&self, request_model: Option<String>, hot: Option<String>) -> Option<String> {
        request_model.or(self.context.model.clone()).or(hot)
    }

    /// 生效会话种子: 域人设在前, 请求 system 在后; 均缺省 = 不注入 (零变化)。
    fn seeded_system(&self, request_system: Option<String>) -> Option<String> {
        self.context.seeded_system(request_system.as_deref())
    }
}

/// W3-③ 域伙伴运行时解析 (星群正身: 每户各得其伴):
///
/// - 域面未接线 / 默认域 → 现 [`Runtime`] (零变化);
/// - 非默认域 → 该域自己的伙伴实例 (模型/记忆/配额/写闸全按域);
/// - 非默认域未注入实例面 / 该域未注册伙伴实例 / 域配置坏 → 403
///   `domain_refused` (fail-closed, **绝不拿默认域代跑** —— 代跑会把默认域
///   的会话/记忆/配额面泄给该域)。
fn chat_execution<'a>(
    state: &'a GatewayState,
    domain: Option<&crate::domain_context::DomainContext>,
    endpoint: &'static str,
) -> Result<DomainChatExecution<'a>, Response> {
    use apeireth_core::user_domain::UserDomainError;

    let Some(bound) = domain.and_then(|context| context.domain.as_ref()) else {
        return Ok(DomainChatExecution {
            runtime: &state.runtime,
            context: apeireth_runtime::canonical::DomainRuntimeContext::default(),
        });
    };
    let Some(plane) = state.domain_runtimes.as_ref() else {
        // 实例面未注入: 默认域仍是组合根注入的现运行时 (零变化, 不陪非默认域
        // 一起拒); 非默认域拒开不代跑。
        if bound.is_default() {
            return Ok(DomainChatExecution {
                runtime: &state.runtime,
                context: apeireth_runtime::canonical::DomainRuntimeContext::default(),
            });
        }
        return Err(crate::domain_context::domain_refused_frame(
            state,
            endpoint,
            &UserDomainError::Refused(format!(
                "user domain `{}` chat surface refused: no open partner runtime plane",
                bound.user_id
            )),
        ));
    };
    let runtime = plane
        .runtime_for(bound)
        .map_err(|error| crate::domain_context::domain_refused_frame(state, endpoint, &error))?;
    let context = plane
        .context_for(bound)
        .map_err(|error| crate::domain_context::domain_refused_frame(state, endpoint, &error))?;
    Ok(DomainChatExecution { runtime, context })
}

/// 会话入域立案 (W3-③ 域事件过滤面): 域面接线时, 回合开跑前把会话在**本域**
/// 账本立案 —— 该会话的事件帧自 `turn_started` 起即在本域在册, 域订阅按域
/// 过滤不丢首帧; 已立案 = 幂等原样。域面未接线 = 不动 (兼容路径零变化)。
async fn admit_domain_session(
    state: &GatewayState,
    execution: &DomainChatExecution<'_>,
    session: SessionId,
) -> Result<(), HttpError> {
    if state.domains.is_none() {
        return Ok(());
    }
    let manager = execution.runtime.sessions();
    let record = manager
        .load_or_create(session)
        .await
        .map_err(|error| http_error(CanonicalEntryError::Runtime(error), Some(session)))?;
    manager
        .save(&record)
        .await
        .map_err(|error| http_error(CanonicalEntryError::Runtime(error), Some(session)))
}

async fn native_chat(
    State(state): State<GatewayState>,
    identity: Option<Extension<ClientIdentity>>,
    user: Option<Extension<UserContext>>,
    domain: Option<Extension<crate::domain_context::DomainContext>>,
    Json(request): Json<CanonicalChatRequest>,
) -> Result<Response, HttpError> {
    let mut request = request;
    let execution = match chat_execution(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/chat",
    ) {
        Ok(execution) => execution,
        Err(refusal) => return Ok(refusal),
    };
    request.model = execution.resolve_model(request.model.take(), hot_model_override(&state));
    // 输入校验先于会话立案: 400 请求不落空会话 (与 execute 内的校验同一实现)。
    if let Err(error) = validate_chat_input(&request.input) {
        return Err(http_error(error, request.session));
    }
    let session = request.session.unwrap_or_else(SessionId::new);
    request.session = Some(session);
    request.system = execution.seeded_system(request.system.take());
    // W3-②: 会话→用户绑定 (首个具名写者), 跨用户审批拒的判定依据。
    bind_session_owner(&state, session, user.as_ref());
    // W3-③: 会话入域立案 (域事件过滤面)。
    admit_domain_session(&state, &execution, session).await?;
    let attribution = attribution_for(&state, identity.as_ref());
    let outcome = execute_chat_attributed(execution.runtime.as_ref(), request, attribution).await;
    state.observations.flush().await;
    let outcome = outcome.map_err(|error| http_error(error, Some(session)))?;
    publish_turn_delta(&state.events, &outcome);
    Ok(chat_http_response(outcome))
}

/// Emit the final assistant text as a transport delta. Lifecycle semantics are
/// emitted by the RuntimeEventSink; this helper does not infer them.
fn publish_turn_delta(bus: &EventBus, outcome: &CanonicalChatOutcome) {
    if let CanonicalChatOutcome::Completed(response) = outcome {
        // v1 honesty: the runtime completes a turn before the gateway can
        // encode it, so `turn_delta` carries the final text as ONE delta.
        bus.publish(GatewayEvent::new(
            "turn_delta",
            serde_json::json!({ "session": response.session, "text": response.text }),
        ));
    }
}

async fn native_resolve_approval(
    State(state): State<GatewayState>,
    identity: Option<Extension<ClientIdentity>>,
    user: Option<Extension<UserContext>>,
    domain: Option<Extension<crate::domain_context::DomainContext>>,
    Json(request): Json<CanonicalApprovalRequest>,
) -> Result<Response, HttpError> {
    let session = request.session;
    // V2-c 跨域归属拒 (资源级, 先于角色/归属判定 —— 他域会话与不存在同帧,
    // 连「此处有审批」都不泄露)。
    if let Some(refusal) =
        approval_session_refusal(&state, domain.as_ref(), "/v1/approvals/resolve", session).await
    {
        state.observations.flush().await;
        return Ok(refusal);
    }
    // W3-② 跨用户审批拒: 归属闸在角色门能力判定之后 (owner 可批任意会话)。
    if let Some(refusal) = cross_user_approval_refusal(&state, session, user.as_ref()) {
        state.observations.flush().await;
        return Ok(refusal);
    }
    // V2-c: 续跑走**本域伙伴实例** (默认域/域面未接线 = 现运行时, 零变化)。
    let execution = match chat_execution(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/approvals/resolve",
    ) {
        Ok(execution) => execution,
        Err(refusal) => {
            state.observations.flush().await;
            return Ok(refusal);
        }
    };
    let attribution = attribution_for(&state, identity.as_ref());
    let outcome =
        resolve_approval_attributed(execution.runtime.as_ref(), request, attribution).await;
    state.observations.flush().await;
    let outcome = outcome.map_err(|error| http_error(error, Some(session)))?;
    publish_turn_delta(&state.events, &outcome);
    Ok(chat_http_response(outcome))
}

#[derive(Debug, Deserialize)]
struct ApprovalInboxQuery {
    session: SessionId,
}

#[derive(Debug, Serialize)]
struct ApprovalInboxResponse {
    session: SessionId,
    approvals: Vec<CanonicalPendingApproval>,
}

async fn list_pending_approvals(
    State(state): State<GatewayState>,
    domain: Option<Extension<crate::domain_context::DomainContext>>,
    Query(query): Query<ApprovalInboxQuery>,
) -> Result<Response, HttpError> {
    // V2-c 跨域归属拒: 本域不在册的会话 id 与不存在同帧 (审批收件箱同样不泄
    // 他域会话的存在性)。
    if let Some(refusal) =
        approval_session_refusal(&state, domain.as_ref(), "/v1/approvals", query.session).await
    {
        state.observations.flush().await;
        return Ok(refusal);
    }
    // V2-c: 收件箱读**本域伙伴实例**的挂起审批 (默认域/域面未接线 = 现运行时)。
    let execution = match chat_execution(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/approvals",
    ) {
        Ok(execution) => execution,
        Err(refusal) => {
            state.observations.flush().await;
            return Ok(refusal);
        }
    };
    let approvals = execution
        .runtime
        .pending_approvals(query.session)
        .await
        .map_err(|error| http_error(CanonicalEntryError::Runtime(error), Some(query.session)))?
        .into_iter()
        .map(CanonicalPendingApproval::from)
        .collect();
    Ok(Json(ApprovalInboxResponse {
        session: query.session,
        approvals,
    })
    .into_response())
}

fn chat_http_response(outcome: CanonicalChatOutcome) -> Response {
    match outcome {
        CanonicalChatOutcome::Completed(response) => {
            (StatusCode::OK, Json(response)).into_response()
        }
        CanonicalChatOutcome::PendingApproval(pending) => {
            (StatusCode::ACCEPTED, Json(pending)).into_response()
        }
    }
}

async fn openai_chat(
    State(state): State<GatewayState>,
    identity: Option<Extension<ClientIdentity>>,
    user: Option<Extension<UserContext>>,
    domain: Option<Extension<crate::domain_context::DomainContext>>,
    Json(request): Json<OpenAiChatRequest>,
) -> Result<Response, HttpError> {
    let mut request = request;
    let execution = match chat_execution(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/chat/completions",
    ) {
        Ok(execution) => execution,
        Err(refusal) => return Ok(refusal),
    };
    request.model = execution.resolve_model(request.model.take(), hot_model_override(&state));
    let is_stream = request.stream;
    let session = request.session_id.unwrap_or_else(SessionId::new);
    // W3-②: 会话→用户绑定 (首个具名写者), 跨用户审批拒的判定依据。
    bind_session_owner(&state, session, user.as_ref());
    let input = request
        .messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| ContentPart::join_text(&ContentPart::from_legacy_value(&message.content)))
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| {
            http_error(
                CanonicalEntryError::InvalidRequest(
                    "messages must contain a non-empty user message".into(),
                ),
                Some(session),
            )
        })?;
    let system = request
        .messages
        .iter()
        .filter(|message| message.role == "system")
        .map(|message| ContentPart::join_text(&ContentPart::from_legacy_value(&message.content)))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let native = CanonicalChatRequest {
        session: Some(session),
        input,
        model: request.model.clone(),
        system: execution.seeded_system((!system.is_empty()).then_some(system)),
    };
    let model_name = request.model.unwrap_or_default();
    let attribution = attribution_for(&state, identity.as_ref());
    // W3-③: 会话入域立案 (域事件过滤面; 输入校验已过)。
    admit_domain_session(&state, &execution, session).await?;

    // True incremental streaming: deltas flow from the provider through the
    // canonical loop to the client as they arrive. Non-streaming providers
    // fall back inside the runtime (one final delta), so this branch is
    // correct for every provider.
    if is_stream {
        return openai_chat_streaming(
            &state,
            Arc::clone(execution.runtime),
            native,
            model_name,
            attribution,
        )
        .await;
    }

    let outcome = execute_chat_attributed(execution.runtime.as_ref(), native, attribution).await;
    state.observations.flush().await;
    let outcome = outcome.map_err(|error| http_error(error, Some(session)))?;
    let outcome = match outcome {
        CanonicalChatOutcome::Completed(completed) => completed,
        CanonicalChatOutcome::PendingApproval(pending) => {
            return Ok((StatusCode::ACCEPTED, Json(pending)).into_response());
        }
    };
    let created = Timestamp::from_clock(state.runtime.clock().as_ref()).epoch_millis() / 1_000;
    let events = outcome.events.clone();
    publish_turn_delta(
        &state.events,
        &CanonicalChatOutcome::Completed(outcome.clone()),
    );

    Ok(Json(OpenAiChatResponse {
        id: outcome.request.clone(),
        object: "chat.completion",
        created,
        model: model_name,
        choices: vec![OpenAiChoice {
            index: 0,
            message: OpenAiAssistantMessage {
                role: "assistant",
                content: outcome.text,
            },
            finish_reason: "stop",
        }],
        usage: outcome.usage,
        apeireth: OpenAiExecutionMetadata {
            session_id: outcome.session.to_string(),
            trace_id: outcome.trace_id,
            served_by: outcome.served_by,
            rounds: outcome.rounds,
            events,
            usage: None,
        },
    })
    .into_response())
}

/// Run the canonical turn with incremental delta forwarding and write an
/// OpenAI-spec SSE response: role chunk → one chunk per content delta →
/// final chunk (finish_reason + apeireth metadata + usage) → `[DONE]`.
/// A pending approval or a runtime error also terminates the stream with an
/// explicit final frame, so the client never hangs.
async fn openai_chat_streaming(
    state: &GatewayState,
    runtime: Arc<Runtime>,
    native: CanonicalChatRequest,
    model_name: String,
    attribution: SessionAttribution,
) -> Result<Response, HttpError> {
    let session = native
        .session
        .expect("canonical chat request always carries a session");
    let stream_id = RequestId::new().to_string();
    let created = Timestamp::from_clock(state.runtime.clock().as_ref()).epoch_millis() / 1_000;

    let (tx, rx) = tokio::sync::mpsc::channel::<String>(64);
    let delta_tx = tx.clone();
    let delta_stream_id = stream_id.clone();
    let delta_model = model_name.clone();
    let sink: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |delta: String| {
        // Wrap each delta in its own OpenAI-spec SSE chunk frame; the channel
        // carries only complete, pre-serialized frames.
        let chunk = OpenAiStreamChunk {
            id: delta_stream_id.clone(),
            object: "chat.completion.chunk",
            created,
            model: delta_model.clone(),
            choices: vec![OpenAiStreamChoice {
                index: 0,
                delta: OpenAiStreamDelta {
                    role: None,
                    content: Some(delta),
                },
                finish_reason: None,
            }],
            apeireth: None,
            usage: None,
        };
        if let Ok(json) = serde_json::to_string(&chunk) {
            // try_send: a vanished reader must never block the canonical loop.
            let _ = delta_tx.try_send(format!("data: {json}\n\n"));
        }
    });

    let observations = Arc::clone(&state.observations);
    let events = state.events.clone();
    let mut turn = TurnRequest::new(session, native.input);
    if let Some(model) = native.model {
        turn = turn.with_model(model);
    }
    if let Some(system) = native.system {
        turn = turn.with_system(system);
    }
    tokio::spawn(async move {
        let role_chunk = OpenAiStreamChunk {
            id: stream_id.clone(),
            object: "chat.completion.chunk",
            created,
            model: model_name.clone(),
            choices: vec![OpenAiStreamChoice {
                index: 0,
                delta: OpenAiStreamDelta {
                    role: Some("assistant"),
                    content: None,
                },
                finish_reason: None,
            }],
            apeireth: None,
            usage: None,
        };
        if let Ok(json) = serde_json::to_string(&role_chunk) {
            let _ = tx.send(format!("data: {json}\n\n")).await;
        }

        // W2-③: 流式回合同样过会话写闸 (排队 + 串行 + 归属审计)。
        let outcome = run_turn_serialized(runtime.as_ref(), turn, &attribution, Some(sink)).await;
        observations.flush().await;

        match outcome {
            Ok(TurnOutcome::Completed(response)) => {
                let chat_outcome = turn_outcome_to_chat(TurnOutcome::Completed(response.clone()));
                publish_turn_delta(&events, &chat_outcome);
                let final_chunk = OpenAiStreamChunk {
                    id: stream_id.clone(),
                    object: "chat.completion.chunk",
                    created,
                    model: model_name.clone(),
                    choices: vec![OpenAiStreamChoice {
                        index: 0,
                        delta: OpenAiStreamDelta {
                            role: None,
                            content: None,
                        },
                        finish_reason: Some("stop"),
                    }],
                    apeireth: Some(OpenAiExecutionMetadata {
                        session_id: response.session.to_string(),
                        trace_id: response.trace.trace.to_string(),
                        served_by: response.served_by.to_string(),
                        rounds: response.rounds,
                        events: events_from_trace(&response.trace),
                        usage: Some(response.usage.clone()),
                    }),
                    usage: Some(response.usage),
                };
                if let Ok(json) = serde_json::to_string(&final_chunk) {
                    let _ = tx.send(format!("data: {json}\n\ndata: [DONE]\n\n")).await;
                }
            }
            Ok(TurnOutcome::PendingApproval(view)) => {
                let pending = CanonicalPendingApproval::from(view);
                let frame = serde_json::json!({
                    "id": stream_id,
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model_name,
                    "choices": [{"index": 0, "delta": {}, "finish_reason": "approval_required"}],
                    "apeireth": { "pending_approval": pending },
                });
                if let Ok(json) = serde_json::to_string(&frame) {
                    let _ = tx.send(format!("data: {json}\n\ndata: [DONE]\n\n")).await;
                }
            }
            Err(error) => {
                let frame = runtime_error_frame(&error);
                let body = serde_json::json!({
                    "error": frame,
                    "session_id": session.to_string(),
                });
                if let Ok(json) = serde_json::to_string(&body) {
                    let _ = tx.send(format!("data: {json}\n\ndata: [DONE]\n\n")).await;
                }
            }
        }
    });

    let stream =
        tokio_stream::wrappers::ReceiverStream::new(rx).map(Ok::<String, std::convert::Infallible>);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .header("connection", "keep-alive")
        .body(axum::body::Body::from_stream(stream))
        .map_err(|e| {
            http_error(
                CanonicalEntryError::InvalidRequest(e.to_string()),
                Some(session),
            )
        })?)
}

/// The admin-patched default model, when one was set through
/// `/v1/admin/config`. The startup env baseline is deliberately not injected:
/// the composition root already made it the runtime's own default model.
fn hot_model_override(state: &GatewayState) -> Option<String> {
    let config = state
        .hot_config
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    config
        .admin_model_set
        .then(|| config.model.clone())
        .flatten()
}

fn http_error(error: CanonicalEntryError, session: Option<SessionId>) -> HttpError {
    let (status, code) = classify_entry_error(&error);
    let frame = match &error {
        CanonicalEntryError::Runtime(runtime) => runtime_error_frame(runtime),
        CanonicalEntryError::InvalidRequest(_) => ErrorFrame::new(code, error.to_string()),
    };
    (
        status,
        Json(ErrorBody {
            error: frame,
            session_id: session.map(|id| id.to_string()),
        }),
    )
}

/// The error frame for one runtime failure.
///
/// A round-limit failure carries its real budget numbers (configured limit,
/// consumed rounds, and the approval-frozen tool name when one existed) as
/// structured `details`, so the frame is diagnosable without a second lookup.
fn runtime_error_frame(error: &RuntimeError) -> ErrorFrame {
    let (_, code) = classify_runtime_error(error);
    let frame = ErrorFrame::new(code, error.to_string());
    match error {
        RuntimeError::RoundLimitExceeded {
            limit,
            rounds,
            pending_tool,
        } => frame.with_details(serde_json::json!({
            "limit": limit,
            "rounds": rounds,
            "pending_tool": pending_tool,
        })),
        _ => frame,
    }
}

fn classify_entry_error(error: &CanonicalEntryError) -> (StatusCode, ErrorCode) {
    match error {
        CanonicalEntryError::InvalidRequest(_) => {
            (StatusCode::BAD_REQUEST, ErrorCode::InvalidRequest)
        }
        CanonicalEntryError::Runtime(runtime) => classify_runtime_error(runtime),
    }
}

fn classify_runtime_error(error: &RuntimeError) -> (StatusCode, ErrorCode) {
    match error {
        RuntimeError::Provider(provider) => (
            StatusCode::BAD_GATEWAY,
            provider_code(&provider.to_string()),
        ),
        RuntimeError::ProvidersExhausted { source, .. } => {
            (StatusCode::BAD_GATEWAY, provider_code(&source.to_string()))
        }
        RuntimeError::NoProvider { .. } | RuntimeError::NoHealthyProvider { .. } => (
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::ProviderUnreachable,
        ),
        RuntimeError::Misconfigured(_) => (StatusCode::SERVICE_UNAVAILABLE, ErrorCode::Internal),
        RuntimeError::Denied { .. } => (StatusCode::FORBIDDEN, ErrorCode::InvalidRequest),
        RuntimeError::ModuleStopped { .. } => {
            (StatusCode::UNPROCESSABLE_ENTITY, ErrorCode::ReviewRejected)
        }
        RuntimeError::RoundLimitExceeded { .. } => (
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::TurnNotConverged,
        ),
        RuntimeError::ApprovalRequired { .. } | RuntimeError::SessionApprovalPending { .. } => {
            (StatusCode::CONFLICT, ErrorCode::InvalidRequest)
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::Internal),
    }
}

/// Classify a provider failure from its human-readable rendering.
///
/// The gateway crate has no regular dependency on `apeireth_plugin` (where
/// `ProviderError` is declared), so it cannot destructure the error type. It
/// reads the stable `Display` wording instead and preserves the full text in
/// the frame `message`.
fn provider_code(text: &str) -> ErrorCode {
    let lower = text.to_ascii_lowercase();
    if lower.contains("rate limited") {
        ErrorCode::RateLimited
    } else if lower.contains("timed out") || lower.contains("network error") {
        ErrorCode::ProviderUnreachable
    } else if lower.contains("authentication failed") {
        if lower.contains("missing") {
            ErrorCode::AuthMissingKey
        } else {
            ErrorCode::AuthInvalidKey
        }
    } else {
        ErrorCode::ProviderError
    }
}
