//! C1 规范流的**后端原生流面** (后端合同 C 批 · C1 段 · 裁决 15)。
//!
//! 路径: `GET /v1/sessions/{session_id}/requests/{request_id}/stream` (SSE)。
//!
//! 职责边界 (裁决 16): 网关**只做投影** —— 权威缓冲在运行时注册表
//! (`apeireth_runtime::canonical::canonical_stream`), 网关不持有第二份缓冲,
//! 断线重连的续发依据也只来自那一份。
//!
//! 续发锚点:
//! - `Last-Event-ID` 请求头 (**优先**) 或 `?after_sequence=` 查询参数 (次之),
//!   两者都缺省 ⇒ 从头续发 (`after_sequence = 0`);
//! - 客户端落后于环形窗口 ⇒ 服务端先发一条 `resync_required: true` 的**事实帧**
//!   (终态 `unknown`), **然后关闭通道** (裁决 17: 这是显式事实, 不是错误码)。
//!
//! 帧形态 (与 `apeireth-protocol::canonical_stream` 同源): 每帧带
//! `event: <帧族>` / `data: <帧 JSON>` / `id: <sequence>` —— `id` 就是帧序号,
//! 客户端可直接把它作为下一次重连的 `Last-Event-ID`。
//!
//! 停止 (C2-2): `POST /v1/sessions/{session_id}/requests/{request_id}/stop`
//! 取消运行时该回合的取消令牌 ⇒ 只阻断**后续工具派发与提示工具**, 已发生的
//! 副作用不撤销。
//!
//! 不做什么 (诚实边界):
//! - 不实现 `/bridge/*` 宿主映射 (由前端负责);
//! - 停止端点不发布 `stopped` 终帧 (C1 帧族接线归 C2-3);
//! - 缓冲与停止摘要都是进程内内存 ⇒ 进程重启后本流不再可续 (客户端须改取回执)。

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::{RequestId, SessionId};
use apeireth_protocol::canonical_stream::{C1FinalFrame, C1Frame, C1StreamState};
use apeireth_runtime::canonical::canonical_stream::{global_registry, ReplayOutcome};
use apeireth_runtime::canonical::turn_cancellation::{self, StopOutcome};
use apeireth_runtime::canonical::PendingApprovalView;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};

use crate::error_frame::{ErrorCode, ErrorFrame};
use crate::panels::GatewayState;

/// 续发锚点查询参数 (与 `Last-Event-ID` 头同义, 头优先)。
#[derive(Debug, Default, Deserialize)]
pub struct CanonicalStreamQuery {
    /// 客户端已收到的最后序号 (字符串形态: 非法值给显式 400, 不静默当 0)。
    #[serde(default)]
    pub after_sequence: Option<String>,
}

/// C1 规范流的路由面 (后端原生 SSE 路径)。
///
/// 路由定义放在本模块 ⇒ 主路由表只需一行 `merge` ⇒ 既有路由表文件净增 0 行
/// (尺寸棘轮 `size crates/adapters/gateway/src/canonical_entry.rs 2291` 不变)。
pub fn routes() -> Router<GatewayState> {
    Router::<GatewayState>::new()
        .route(
            "/v1/sessions/:session_id/requests/:request_id/stream",
            get(canonical_stream_handler),
        )
        .route(
            "/v1/sessions/:session_id/requests/:request_id/stop",
            post(stop_turn_handler),
        )
}

/// C1 规范流 SSE 面。
///
/// 投影顺序: 先重放窗口内帧 (序号严格大于续发锚点), 再接实时帧; 实时帧按序号
/// 去重 ⇒ 重放与实时交界处不会重复投同一帧。
pub async fn canonical_stream_handler(
    State(_state): State<GatewayState>,
    Path((session_id, request_id)): Path<(String, String)>,
    Query(query): Query<CanonicalStreamQuery>,
    headers: HeaderMap,
) -> Response {
    let registry = global_registry();
    let Some(handle) = registry.handle(&session_id, &request_id) else {
        // 流不存在 ⇒ 404 (错误码目录是封闭的: 复用 `session_not_found`, 不新增码)。
        return ErrorFrame::response(
            StatusCode::NOT_FOUND,
            ErrorCode::SessionNotFound,
            format!("no canonical stream for session {session_id} / request {request_id}"),
        );
    };

    let after_sequence = match resolve_after_sequence(&headers, &query) {
        Ok(value) => value,
        Err(message) => {
            return ErrorFrame::response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                message,
            )
        }
    };

    // 先订阅后重放: 订阅与重放之间发布的帧会落进实时队列 (不漏);
    // 重放窗口与实时队列的重复帧按序号去重 (不重投)。
    let subscription = match registry.subscribe(&handle) {
        Ok(subscription) => subscription,
        Err(error) => {
            return ErrorFrame::response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::Internal,
                error.to_string(),
            )
        }
    };
    let replay = registry.replay(&handle, after_sequence);

    let (head, close_after_head, last_sequence) = match replay {
        Ok(ReplayOutcome::Window(frames)) => {
            let mut last = after_sequence;
            let mut events = Vec::with_capacity(frames.len());
            for frame in &frames {
                last = last.max(frame.sequence());
                events.push(Ok::<SseEvent, Infallible>(frame_to_event(frame)));
            }
            (events, false, last)
        }
        Ok(ReplayOutcome::ResyncRequired {
            oldest_available_sequence,
        }) => {
            // 落后事实帧: 显式告知「需要重同步」, 随后关闭通道。
            let mut fact =
                C1FinalFrame::new(&session_id, &request_id, None, C1StreamState::Unknown);
            fact.sequence = oldest_available_sequence;
            fact.resync_required = true;
            (
                vec![Ok::<SseEvent, Infallible>(frame_to_event(&C1Frame::Final(
                    fact,
                )))],
                true,
                oldest_available_sequence,
            )
        }
        Err(error) => {
            return ErrorFrame::response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::Internal,
                error.to_string(),
            )
        }
    };

    let last = Arc::new(AtomicU64::new(last_sequence));
    let stream: Pin<Box<dyn Stream<Item = Result<SseEvent, Infallible>> + Send>> =
        if close_after_head {
            // 事实帧之后关闭通道: 订阅随本作用域释放 (接收端关闭), 不再续发。
            drop(subscription);
            Box::pin(tokio_stream::iter(head))
        } else {
            let last = Arc::clone(&last);
            let live = ReceiverStream::new(subscription.into_receiver()).filter_map(move |frame| {
                // 已由重放窗口投过的帧 ⇒ 按序号去重, 不重投。
                if frame.sequence() <= last.load(Ordering::Relaxed) {
                    return None;
                }
                last.store(frame.sequence(), Ordering::Relaxed);
                Some(Ok::<SseEvent, Infallible>(frame_to_event(&frame)))
            });
            Box::pin(tokio_stream::iter(head).chain(live))
        };

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

/// 续发锚点: `Last-Event-ID` 头优先, 再落 `?after_sequence=`。
///
/// 非法值 ⇒ 显式 400 错误帧 (不静默当 0 —— 静默当 0 会让客户端重收整窗)。
fn resolve_after_sequence(
    headers: &HeaderMap,
    query: &CanonicalStreamQuery,
) -> Result<u64, String> {
    if let Some(raw) = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
    {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed
                .parse::<u64>()
                .map_err(|_| format!("invalid Last-Event-ID header {raw:?}"));
        }
    }
    if let Some(raw) = query.after_sequence.as_deref() {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed
                .parse::<u64>()
                .map_err(|_| format!("invalid after_sequence query {raw:?}"));
        }
    }
    Ok(0)
}

/// 帧 → SSE 事件 (`event:` / `data:` / `id:` 三行, `id` = 帧序号)。
fn frame_to_event(frame: &C1Frame) -> SseEvent {
    let payload = serde_json::to_string(frame).unwrap_or_else(|_| "{}".to_string());
    match frame {
        C1Frame::Delta(delta) => SseEvent::default()
            .event(delta.event.as_str())
            .data(payload)
            .id(delta.sequence.to_string()),
        C1Frame::Final(final_frame) => SseEvent::default()
            .event(final_frame.event.as_str())
            .data(payload)
            .id(final_frame.sequence.to_string()),
    }
}

/// 停止一个在飞回合 (C2-2)。
///
/// 路由: `POST /v1/sessions/{session_id}/requests/{request_id}/stop`。只取消
/// 运行时回合级取消令牌: **不取会话写闸**, 不执行回合, 不撤销已发生的副作用;
/// 第一次 stop 的摘要留在进程内有界队列里, 重复调用回放同一份摘要 (幂等)。
pub async fn stop_turn_handler(
    State(state): State<GatewayState>,
    Path((session_id, request_id)): Path<(String, String)>,
) -> Response {
    let Ok(session) = session_id.parse::<SessionId>() else {
        return ErrorFrame::response(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            format!("invalid session_id: {session_id}"),
        );
    };
    let Ok(request) = request_id.parse::<RequestId>() else {
        return ErrorFrame::response(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            format!("invalid request_id: {request_id}"),
        );
    };

    // 待批回合: stop 不越过审批裁决, 先让调用方批准/拒绝或等它过期。
    match state.runtime.pending_approvals(session).await {
        Ok(pending) if approval_blocks_stop(&pending, request) => {
            return ErrorFrame::response(
                StatusCode::CONFLICT,
                ErrorCode::InvalidRequest,
                format!(
                    "request {request} is awaiting approval; decide it or let it expire before stopping"
                ),
            )
        }
        Ok(_) => {}
        Err(error) => {
            return ErrorFrame::response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::Internal,
                error.to_string(),
            )
        }
    }

    match turn_cancellation::stop(session, request) {
        StopOutcome::Stopped(summary) | StopOutcome::AlreadyStopped(summary) => {
            Json(summary).into_response()
        }
        StopOutcome::NotFound => ErrorFrame::response(
            StatusCode::NOT_FOUND,
            ErrorCode::SessionNotFound,
            format!("no running turn for session {session} request {request}"),
        ),
    }
}

/// 只对**同一 request** 的待批命中返回 true (别的 request 待批不阻塞本回合)。
fn approval_blocks_stop(pending: &[PendingApprovalView], request: RequestId) -> bool {
    pending.iter().any(|view| view.request_id == request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{ApprovalId, CapabilityId, Timestamp, TraceId};
    use apeireth_protocol::canonical::ToolCall;

    fn pending_view(request: RequestId) -> PendingApprovalView {
        PendingApprovalView {
            approval_id: ApprovalId::new(),
            session_id: SessionId::new(),
            request_id: request,
            trace_id: TraceId::new(),
            round: 1,
            capability_id: CapabilityId::new("test.capability").expect("static capability id"),
            tool_name: "test.tool".to_string(),
            tool_call: ToolCall {
                id: "call-1".to_string(),
                name: "test.tool".to_string(),
                arguments: serde_json::json!({}),
            },
            effective_invocation: None,
            governance_hook: String::new(),
            governance_reason: String::new(),
            command_text: String::new(),
            arguments_summary: String::new(),
            authorization_families: Vec::new(),
            action_binding_digest: String::new(),
            created_at: Timestamp::now(),
            expires_at: Timestamp::now(),
            operation_fingerprint: String::new(),
            generated_text: String::new(),
            rounds_used: 0,
        }
    }

    #[test]
    fn approval_blocks_stop_matches_only_the_same_request() {
        let request = RequestId::new();
        let other = RequestId::new();
        let pending = vec![pending_view(request)];
        assert!(approval_blocks_stop(&pending, request));
        assert!(!approval_blocks_stop(&pending, other));
        assert!(!approval_blocks_stop(&[], request));
    }
}
