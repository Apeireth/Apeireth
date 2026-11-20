//! Owns admitted native turns independently of the HTTP response waiter.

use super::*;
use crate::operation_header::operation_request;
use tokio::sync::Semaphore;

/// Each router bounds its running and session-queued detached requests.
/// A request still waiting for a slot belongs to its HTTP connection.
const MAX_ADMITTED_REQUESTS: usize = 128;

pub(super) fn route() -> axum::routing::MethodRouter<GatewayState> {
    post(native_chat).layer(Extension(Arc::new(Semaphore::new(MAX_ADMITTED_REQUESTS))))
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
    // C2-1: 可选的幂等键先解码 (摘要取解析后的 model/system); 非法键在
    // 会话立案、绑主、入域之前就 400, 不留下副作用。
    let operation = match operation_request(&headers, &request) {
        Ok(operation) => operation,
        Err(error) => return Err(http_error(error, request.session)),
    };
    let runtime = Arc::clone(execution.runtime);
    let permit = capacity
        .acquire_owned()
        .await
        .map_err(|_| owner_failed(session))?;
    // Only admitted work is detached. Dropping the HTTP JoinHandle drops the
    // waiter, while the worker retains its permit and the original operation.
    tokio::spawn(async move {
        let _permit = permit;
        // W3-②: 会话→用户绑定 (首个具名写者), 跨用户审批拒的判定依据。
        bind_session_owner(&state, session, user.as_ref());
        // W3-③: 会话入域立案 (域事件过滤面)。
        admit_domain_session(&state, &runtime, session).await?;

        // **主体 → 会话**归属登记（2026-11-01 安全审计 P1 收口）:
        // episode 维度的「被遗忘权」要成立, 必须能把 episode 归到主体
        // ⇒ 靠 memory 侧的 `memory_principal_sessions` 映射连接
        // ⇒ 生产侧原先**无写入方** ⇒ 那一维的被遗忘权无法生效。
        // 主体 = 域面 `user_id`（人/账户级）；**无 user_id ⇒ 不登记**
        //（宁可归属不可判定, 也不要错归属 ⇒ 误删风险）。
        // ⚠️ **旁路**: 写失败 ⇒ 记审计、**不阻断主流程**。
        if let Err(error) = crate::principal_attribution::record_principal_attribution(
            &state,
            identity.as_ref().map(|Extension(identity)| identity),
            session,
        ) {
            eprintln!("[principal-attribution] 归属登记失败 (旁路, 不阻断请求): {error}");
        }
        let attribution = attribution_for(&state, identity.as_ref());
        let outcome =
            execute_chat_attributed_operation(&runtime, request, attribution, operation).await;
        state.observations.flush().await;
        let outcome = outcome.map_err(|error| http_error(error, Some(session)))?;
        publish_turn_delta(&state.events, &outcome);
        Ok(chat_http_response(outcome))
    })
    .await
    .map_err(|_| owner_failed(session))?
}

fn owner_failed(session: SessionId) -> HttpError {
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
