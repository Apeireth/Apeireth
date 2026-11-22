//! Owns admitted native turns independently of the HTTP response waiter.

use super::*;
use crate::operation_header::operation_request;
use apeireth_runtime::canonical::ProjectWorkspaceBinding;
use tokio::sync::Semaphore;

/// Each router bounds its running and session-queued detached requests.
/// A request still waiting for a slot belongs to its HTTP connection.
const MAX_ADMITTED_REQUESTS: usize = 128;

const WORKSPACE_CHAT: &str = "/v1/workspace-chat";

pub(super) fn routes() -> Router<GatewayState> {
    Router::new()
        .route("/v1/chat", post(native_chat))
        .route(WORKSPACE_CHAT, post(workspace_chat))
        .layer(Extension(Arc::new(Semaphore::new(MAX_ADMITTED_REQUESTS))))
}

async fn native_chat(
    State(state): State<GatewayState>,
    Extension(capacity): Extension<Arc<Semaphore>>,
    identity: Option<Extension<ClientIdentity>>,
    user: Option<Extension<UserContext>>,
    domain: Option<Extension<crate::domain_context::DomainContext>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CanonicalChatRequest>,
) -> Result<Response, HttpError> {
    dispatch(
        state, capacity, identity, user, domain, headers, request, None,
    )
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceChat {
    session: SessionId,
    input: String,
    model: Option<String>,
    system: Option<String>,
    workspace_binding: ProjectWorkspaceBinding,
}

async fn workspace_chat(
    State(state): State<GatewayState>,
    Extension(capacity): Extension<Arc<Semaphore>>,
    identity: Option<Extension<ClientIdentity>>,
    user: Option<Extension<UserContext>>,
    domain: Option<Extension<crate::domain_context::DomainContext>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<WorkspaceChat>,
) -> Result<Response, HttpError> {
    if !crate::model_connection::is_managed_single_operator(&state)
        || !identity
            .as_ref()
            .is_some_and(|Extension(id)| id.authenticated)
        || domain
            .as_ref()
            .is_some_and(|Extension(domain)| domain.is_non_default())
    {
        return Ok(ErrorFrame::response(
            StatusCode::FORBIDDEN,
            ErrorCode::PermissionDenied,
            "项目工作区需要认证的桌面单操作者运行时。",
        ));
    }
    let chat = CanonicalChatRequest {
        session: Some(request.session),
        input: request.input,
        model: request.model,
        system: request.system,
        expected_contact_id: None,
    };
    dispatch(
        state,
        capacity,
        identity,
        user,
        domain,
        headers,
        chat,
        Some(request.workspace_binding),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn dispatch(
    state: GatewayState,
    capacity: Arc<Semaphore>,
    identity: Option<Extension<ClientIdentity>>,
    user: Option<Extension<UserContext>>,
    domain: Option<Extension<crate::domain_context::DomainContext>>,
    headers: axum::http::HeaderMap,
    request: CanonicalChatRequest,
    workspace: Option<ProjectWorkspaceBinding>,
) -> Result<Response, HttpError> {
    let mut request = request;
    let contact_mode = request.expected_contact_id.is_some();
    if contact_mode && !contact_session::supports_scope(&state, identity.as_ref(), domain.as_ref())
    {
        return Ok(contact_session::scope_refused());
    }
    let execution = match chat_execution(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        if workspace.is_some() {
            WORKSPACE_CHAT
        } else {
            "/v1/chat"
        },
    ) {
        Ok(execution) => execution,
        Err(refusal) => return Ok(refusal),
    };
    request.model = execution.resolve_model(request.model.take(), hot_model_override(&state));
    // 输入校验先于会话立案: 400 请求不落空会话 (与 execute 内的校验同一实现)。
    if let Err(error) = validate_chat_input(&request.input) {
        return Err(http_error(error, request.session));
    }
    let session = request
        .effective_session()
        .map_err(|error| http_error(error, request.session))?;
    request.session = Some(session);
    request.system = execution.seeded_system(request.system.take());
    // C2-1: 可选的幂等键先解码 (摘要取解析后的 model/system); 非法键在
    // 会话立案、绑主、入域之前就 400, 不留下副作用。
    let operation = match operation_request(&headers, &request, workspace) {
        Ok(operation) => operation,
        Err(error) => return Err(http_error(error, request.session)),
    };
    let runtime = Arc::clone(execution.runtime);
    let permit = capacity
        .acquire_owned()
        .await
        .map_err(|_| owner_failed(session))?;
    let (progress, reader) = native_stream::requested(&headers)
        .then(native_stream::channel)
        .unzip();
    let stream_request = progress.as_ref().map(|value| value.request);
    // Only admitted work is detached. Dropping the HTTP JoinHandle drops the
    // waiter, while the worker retains its permit and the original operation.
    let worker = tokio::spawn(async move {
        let _permit = permit;
        let outcome = async {
            // W3-②: 会话→用户绑定 (首个具名写者), 跨用户审批拒的判定依据。
            if !contact_mode {
                bind_session_owner(&state, session, user.as_ref());
                // W3-③: 会话入域立案 (域事件过滤面)。
                admit_domain_session(&state, &runtime, session).await?;
            }

            // Keep episode ownership attached to the same principal for both delivery modes.
            if workspace.is_none() {
                if let Err(error) = crate::principal_attribution::record_principal_attribution(
                    &state,
                    identity.as_ref().map(|Extension(identity)| identity),
                    session,
                ) {
                    eprintln!("[principal-attribution] 归属登记失败 (旁路, 不阻断请求): {error}");
                }
            }
            let attribution = attribution_for(&state, identity.as_ref());
            let progress = progress.map(TurnProgress::Native);
            let outcome = execute_chat_attributed_operation(
                &runtime,
                request,
                attribution,
                operation,
                progress,
                workspace,
            )
            .await;
            state.observations.flush().await;
            outcome.map_err(|error| http_error(error, Some(session)))
        }
        .await;
        let stopped = stream_request.and_then(|request| {
            native_stream::stopped_request(session, request, outcome.as_ref().ok())
        });
        let reply = outcome.map(|outcome| {
            if stopped.is_none() {
                publish_turn_delta(&state.events, &outcome);
            }
            chat_http_response(outcome)
        });
        (reply, stopped)
    });
    if let Some(reader) = reader {
        return Ok(reader.response(session, worker));
    }
    worker.await.map_err(|_| owner_failed(session))?.0
}

pub(super) fn owner_failed(session: SessionId) -> HttpError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorBody {
            error: ErrorFrame::new(
                ErrorCode::Internal,
                "native turn ended without a confirmed response",
            ),
            session_id: Some(session.to_string()),
        }),
    )
}
