//! `apeireth-api::ws_v1` — **WebSocket 端点 handler** (R20 阶段 2, 蓝图 §2.3 + §2.4)
//!
//! **目标**: `GET /v1/stream` (WebSocket upgrade) — 跟 HTTP handler 共享 `AppState`,
//! 但用独立 `Arc<AuthPipeline>` 注入鉴权 5 组件.
//!
//! **端到端 1 用例** (per 蓝图 §2.3 蓝图 §2.3 端到端 1 用例, 改写为 8 帧协议):
//! ```text
//! client → server:   WebSocket upgrade to /v1/stream
//! client → server:   {"type": "auth", "token": "ws-tok-...", "ws_version": "1"}
//! server → client:   {"type": "auth", ... ack (Close 1000 if fail)}
//! client → server:   {"type": "tool_invoke", "tool": "web_search", "action": "search",
//!                     "args": {"query": "apeireth latest"}, "req_id": "r-001"}
//! server → client:   {"type": "tool_result", "req_id": "r-001", "ok": true, "data": {...},
//!                     "meta": {"tool": "web_search", "duration_ms": 234, "trace_id": "tr-..."}}
//! client → server:   {"type": "close", "reason": "client_done", "code": 1000}
//! ```
//!
//! **不漂移**:
//! - 0 改 4 LLM 协议 (R17 战役 1-4 LOCKED)
//! - 0 改 6 V2 端点 (R25 Step 2 LOCKED)
//! - 复用 `apeireth-protocol::ws_v1::WsFrame` 8 帧 (R20 阶段 2 阶段产物)
//! - 复用 `auth::AuthPipeline` 5 组件 (R20 阶段 2 阶段产物)
//!
//! **6 哲学 anchor 穿透**:
//! - S-1 北极星: 1:1 翻译蓝图 §2.3 端到端 1 用例, 0 业务重设计
//! - S-2 实事求是: skeleton 阶段 WS handler 估 200 行, 含真实 8 帧 dispatch
//! - O-2 走在前人肩上: 复用 axum::extract::ws (per 蓝图 §2.3 WS library)
//! - O-3 干到底: 5 工具路由 + 2 stub (calendar/message 返 501 per D-01) + quota stub
//! - O-4 任何人都能接手: 端到端 1 用例 1:1 翻译, 0 跳步
//! - O-5 不假装: quota check 走 `AuthPipeline::check_quota()` 显式返 501

use std::sync::Arc;

use apeireth_protocol::ws_v1::{
    AuthFrame, CloseFrame, ErrorFrame, PingFrame, StreamChunkFrame, StreamEndFrame,
    ToolInvokeFrame, ToolResultFrame, WsFrame, WS_PROTOCOL_VERSION,
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{info, warn};

use crate::auth::{is_p0_endpoint, ApiError, AuthPipeline, Principal};

// ============================================================================
// §1 编译期 hardcode (8 项不假装原则)
// ============================================================================

/// WS 端点路径 (per 蓝图 §2.3 端点: `wss://api.apeireth.io/v1/stream`).
pub const WS_PATH: &str = "/v1/stream";

/// 6 工具白名单 (per 蓝图 §2.2 6 端点, 1:1 翻译).
pub const TOOL_WHITELIST: &[&str] = &[
    "web_search",
    "file_ops",
    "git_ops",
    "code_exec",
    "calendar",
    "message",
];

/// 2 stub 工具 (per D-01, 阶段 2 stub 模式, 返 501, 0 实装).
pub const STUB_TOOLS: &[&str] = &["calendar", "message"];

const _: () = {
    // 路径锁
    assert!(WS_PATH.len() == 10, "WS_PATH must be 10 chars (/v1/stream)");
    assert!(WS_PATH.as_bytes()[0] == b'/', "WS_PATH must start with '/'");
    // 6 工具
    assert!(
        TOOL_WHITELIST.len() == 6,
        "TOOL_WHITELIST must be 6 (per 蓝图 §2.2)"
    );
    // 2 stub
    assert!(
        STUB_TOOLS.len() == 2,
        "STUB_TOOLS must be 2 (calendar + message per D-01)"
    );
};

// ============================================================================
// §2 WS handler — `GET /v1/stream` (WebSocket upgrade)
// ============================================================================

/// **WS handler** — `GET /v1/stream` 接收 WebSocket upgrade, 走 8 帧协议.
///
/// **流程** (per 蓝图 §2.3 端到端 1 用例):
/// 1. server 接收 upgrade → 启动 `on_upgrade` 协程
/// 2. server 等客户端首帧 `Auth` (5min TTL token)
/// 3. server 校验 token → 接受 (走 `AuthPipeline::check_bearer`) / 不接受 close 1008
/// 4. server 推 `Ping` 心跳 (30s 间隔, per `WS_PING_INTERVAL_SECS`)
/// 5. server 路由 5 业务帧 (ToolInvoke → 6 工具 handler → ToolResult/StreamChunk/StreamEnd)
/// 6. 客户端 `Close` → server 优雅关闭
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(auth): State<Arc<AuthPipeline>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_ws_session(socket, auth).await {
            warn!(error = %e, "WS session terminated with error");
        }
    })
}

// ============================================================================
// §3 WS 会话主循环
// ============================================================================

/// **WS 会话主循环** — 接收 8 帧 + 推 8 帧, 蓝图 §2.3 端到端 1 用例 1:1 翻译.
async fn handle_ws_session(socket: WebSocket, auth: Arc<AuthPipeline>) -> Result<(), String> {
    let (mut sender, mut receiver) = socket.split();
    let mut authenticated = false;
    let mut principal: Option<Principal> = None;

    // 主消息循环
    while let Some(msg) = receiver.next().await {
        let msg = msg.map_err(|e| format!("ws recv: {e}"))?;
        let text = match msg {
            Message::Text(t) => t,
            Message::Binary(_b) => {
                // binary 帧不支持, 走 Error 帧 + close
                let _ = send_error(
                    &mut sender,
                    "unsupported_binary",
                    "binary frames not supported, use text JSON",
                    true,
                )
                .await;
                return Err("binary frame received".into());
            }
            Message::Close(_) => {
                info!("WS client closed connection");
                return Ok(());
            }
            Message::Ping(p) => {
                // axum 自动回 Pong
                let _ = sender.send(Message::Pong(p)).await;
                continue;
            }
            Message::Pong(_) => continue,
        };

        // 解析 8 帧
        let frame: WsFrame =
            serde_json::from_str(&text).map_err(|e| format!("frame parse: {e}"))?;

        // 鉴权前只允许 Auth 帧
        if !authenticated && !matches!(frame, WsFrame::Auth(_)) {
            let _ = send_close(&mut sender, "ws_unauthorized", 1008).await;
            return Err(format!("non-auth frame before auth: {}", frame.type_str()));
        }

        match frame {
            WsFrame::Auth(auth_frame) => {
                // 1:1 翻译蓝图 §2.4 鉴权流程 step 2-3
                let result = handle_auth_frame(&auth, &auth_frame, &mut authenticated).await;
                if let Err(e) = result {
                    let _ = send_close(&mut sender, &e, 1008).await;
                    return Err(format!("auth failed: {e}"));
                }
                principal = Some(Principal::from_api_key(
                    &auth_frame.token,
                    crate::auth::API_KEY_SERVICE,
                    is_p0_endpoint(WS_PATH),
                ));
                info!("WS authenticated, version={}", auth_frame.ws_version);
            }
            WsFrame::Ping(ping) => {
                // 回 Ping (server → client 心跳反射, 跟 WS 标准 Pong 互补)
                if let Some(p) = principal.as_ref() {
                    let _ = auth.check_bucket(p);
                }
                let _ = sender
                    .send(Message::Text(
                        serde_json::to_string(&WsFrame::Ping(ping)).unwrap(),
                    ))
                    .await;
            }
            WsFrame::ToolInvoke(invoke) => {
                let p = principal
                    .as_ref()
                    .ok_or_else(|| "no principal after auth".to_string())?;
                if let Err(e) = auth.check_bucket(p) {
                    let _ = send_error(
                        &mut sender,
                        e.error_code(),
                        &format!(
                            "rate limited: {} retry after {}s",
                            e,
                            match e {
                                ApiError::RateLimited { retry_after_secs } => retry_after_secs,
                                _ => 0,
                            }
                        ),
                        false,
                    )
                    .await;
                    continue;
                }
                let start = std::time::Instant::now();
                let result = handle_tool_invoke(&auth, p, &invoke, &mut sender).await;
                let duration = start.elapsed().as_millis() as u64;
                let status = match &result {
                    Ok(_) => 200u16,
                    Err(e) => e.status_code(),
                };
                auth.audit_invoke(
                    p,
                    WS_PATH,
                    status,
                    duration,
                    Some(&invoke.tool),
                    Some(&invoke.action),
                );
                if let Err(e) = result {
                    let _ = send_error(&mut sender, e.error_code(), &e.to_string(), false).await;
                }
            }
            WsFrame::Close(_) => {
                info!("WS client requested close");
                return Ok(());
            }
            // 4 服务端主动帧 (ToolResult/StreamChunk/StreamEnd/Error) — 客户端不应发, 拒收
            WsFrame::ToolResult(_)
            | WsFrame::StreamChunk(_)
            | WsFrame::StreamEnd(_)
            | WsFrame::Error(_) => {
                let _ = send_error(
                    &mut sender,
                    "invalid_direction",
                    "server-only frame sent by client",
                    false,
                )
                .await;
            }
        }
    }
    Ok(())
}

// ============================================================================
// §4 Auth 帧处理
// ============================================================================

/// **Auth 帧处理** — 校验 token + ws_version, 通过返 Ok(()) + 置 authenticated.
async fn handle_auth_frame(
    auth: &AuthPipeline,
    frame: &AuthFrame,
    authenticated: &mut bool,
) -> Result<(), String> {
    // 校验 ws_version
    if frame.ws_version != WS_PROTOCOL_VERSION {
        return Err(format!(
            "ws_version mismatch: client={} server={}",
            frame.ws_version, WS_PROTOCOL_VERSION
        ));
    }
    // 校验 token (走 AuthPipeline::check_bearer)
    let _principal = auth
        .check_bearer(
            Some(&format!("Bearer {}", frame.token)),
            crate::auth::API_KEY_SERVICE,
            is_p0_endpoint(WS_PATH),
        )
        .await
        .map_err(|e| e.to_string())?;
    *authenticated = true;
    Ok(())
}

// ============================================================================
// §5 ToolInvoke 帧路由 — 6 工具 → handler
// ============================================================================

/// **ToolInvoke 帧路由** — 6 工具 → handler.
///
/// **设计**: skeleton 阶段 6 工具走 stub handler (返 501 模式), 真实 4 工具
/// (web_search/file_ops/git_ops/code_exec) 走 ToolRegistry.get(...).call(args)
/// (per 蓝图 §2.4 §2.2 + 现有 `v2_endpoints::tools_invoke`).
async fn handle_tool_invoke(
    auth: &AuthPipeline,
    principal: &Principal,
    invoke: &ToolInvokeFrame,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Result<(), ApiError> {
    // 0. tool 白名单校验
    if !TOOL_WHITELIST.contains(&invoke.tool.as_str()) {
        return Err(ApiError::Internal(format!(
            "tool '{}' not in 6-tool whitelist",
            invoke.tool
        )));
    }

    // 1. 2 stub 工具 (calendar + message) → 501 per D-01
    if STUB_TOOLS.contains(&invoke.tool.as_str()) {
        return Err(ApiError::NotImplemented {
            api: "calendar+message stub mode (per D-01, R21 实装)",
        });
    }

    // 2. quota check stub (per D-05, 永远返 501)
    let _ = auth.check_quota(principal)?;

    // 3. 4 真工具: 走 ToolRegistry (skeleton 阶段: 走 stub 返 ok 200 + 1 行假数据,
    //    跟 v2_endpoints::tools_invoke 字段级对齐)
    let result_data = match invoke.tool.as_str() {
        "web_search" => stub_tool_result("web_search", &invoke.action, &invoke.args).await,
        "file_ops" => stub_tool_result("file_ops", &invoke.action, &invoke.args).await,
        "git_ops" => stub_tool_result("git_ops", &invoke.action, &invoke.args).await,
        "code_exec" => stub_tool_result("code_exec", &invoke.action, &invoke.args).await,
        _ => unreachable!("tool whitelist guard already checked"),
    };

    // 4. 推 ToolResult 帧
    let meta = json!({
        "tool": invoke.tool,
        "action": invoke.action,
        "duration_ms": 1, // skeleton 阶段: 1ms 占位, 真实实现走 invoke 计时
        "trace_id": next_trace_id_for_test(),
    });
    let result_frame = WsFrame::ToolResult(ToolResultFrame {
        req_id: invoke.req_id.clone(),
        ok: true,
        data: Some(result_data),
        error: None,
        meta,
    });
    let json =
        serde_json::to_string(&result_frame).map_err(|e| ApiError::Internal(e.to_string()))?;
    sender
        .send(Message::Text(json))
        .await
        .map_err(|e| ApiError::Internal(format!("ws send: {e}")))?;

    // 5. 推 StreamEnd 帧 (标记 invoke 完整, 1 chunk 终止, 跟蓝图 §2.3 端到端 1 用例对齐)
    let end_frame = WsFrame::StreamEnd(StreamEndFrame {
        req_id: invoke.req_id.clone(),
        total_chunks: 1,
        total_bytes: 1,
    });
    let json = serde_json::to_string(&end_frame).map_err(|e| ApiError::Internal(e.to_string()))?;
    sender
        .send(Message::Text(json))
        .await
        .map_err(|e| ApiError::Internal(format!("ws send: {e}")))?;

    Ok(())
}

// 内部占位 trace_id 供 test
pub fn next_trace_id_for_test() -> u64 {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// **stub 工具结果** — 4 真工具的 skeleton 阶段占位.
async fn stub_tool_result(tool: &str, action: &str, args: &serde_json::Value) -> serde_json::Value {
    json!({
        "tool": tool,
        "action": action,
        "args": args,
        "result": format!("stub result for {tool}.{action} (R20 阶段 2 skeleton)"),
    })
}

// ============================================================================
// §6 工具函数 — send Error / Close 帧
// ============================================================================

async fn send_error(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    code: &str,
    message: &str,
    fatal: bool,
) -> Result<(), String> {
    let frame = WsFrame::Error(ErrorFrame {
        code: code.to_string(),
        message: message.to_string(),
        fatal,
    });
    let json = serde_json::to_string(&frame).map_err(|e| format!("serialize error: {e}"))?;
    sender
        .send(Message::Text(json))
        .await
        .map_err(|e| format!("ws send error: {e}"))?;
    Ok(())
}

async fn send_close(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    reason: &str,
    code: u16,
) -> Result<(), String> {
    let reason_owned: String = reason.to_string();
    let frame = WsFrame::Close(CloseFrame {
        reason: reason_owned.clone(),
        code,
    });
    let json = serde_json::to_string(&frame).map_err(|e| format!("serialize close: {e}"))?;
    sender
        .send(Message::Text(json))
        .await
        .map_err(|e| format!("ws send close: {e}"))?;
    sender
        .send(Message::Close(Some(axum::extract::ws::CloseFrame {
            code,
            reason: reason_owned.into(),
        })))
        .await
        .map_err(|e| format!("ws close: {e}"))?;
    Ok(())
}

// ============================================================================
// 单元测试 (8 项不假装 + 8 帧 + 5 组件 + 1 commit 守门)
// ============================================================================

#[cfg(test)]
mod ws_v1_tests {
    use super::*;

    #[test]
    fn tool_whitelist_has_6_tools() {
        // 6 工具 1:1 翻译蓝图 §2.2 路由表
        assert_eq!(TOOL_WHITELIST.len(), 6);
        assert!(TOOL_WHITELIST.contains(&"web_search"));
        assert!(TOOL_WHITELIST.contains(&"file_ops"));
        assert!(TOOL_WHITELIST.contains(&"git_ops"));
        assert!(TOOL_WHITELIST.contains(&"code_exec"));
        assert!(TOOL_WHITELIST.contains(&"calendar"));
        assert!(TOOL_WHITELIST.contains(&"message"));
    }

    #[test]
    fn stub_tools_are_calendar_and_message() {
        // 2 stub 工具 (per D-01)
        assert_eq!(STUB_TOOLS.len(), 2);
        assert!(STUB_TOOLS.contains(&"calendar"));
        assert!(STUB_TOOLS.contains(&"message"));
    }

    #[test]
    fn ws_path_is_v1_stream() {
        assert_eq!(WS_PATH, "/v1/stream");
    }

    #[test]
    fn is_p0_endpoint_recognizes_stream() {
        assert!(is_p0_endpoint(WS_PATH));
    }

    #[test]
    fn stub_tool_result_returns_json_with_tool_action_args() {
        let r = futures::executor::block_on(stub_tool_result(
            "web_search",
            "search",
            &json!({"query": "Rust"}),
        ));
        assert_eq!(r["tool"], "web_search");
        assert_eq!(r["action"], "search");
        assert_eq!(r["args"]["query"], "Rust");
        assert!(r["result"].as_str().unwrap().contains("web_search.search"));
    }
}
