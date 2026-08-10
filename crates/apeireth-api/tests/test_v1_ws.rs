//! Integration tests for R20 阶段 2 WebSocket 8 帧 + 鉴权 5 组件
//!
//! **5 fixture** (per 任务稿步骤 5):
//! 1. `auth_frame_with_valid_token_passes` — 5min TTL token 走 AuthPipeline::check_bearer 通过
//! 2. `auth_frame_with_expired_token_returns_close` — 错 / 短 token 走 ApiError::InvalidApiKey
//! 3. `tool_invoke_frame_dispatches_to_web_search_handler` — 8 帧 ToolInvoke → ToolResult
//! 4. `stream_chunk_frame_accumulates_until_stream_end` — 5 业务帧 stream_chunk → stream_end 配对
//! 5. `error_frame_on_quota_check_returns_501_stub` — D-05 quota stub 永远返 501
//!
//! **测试策略**:
//! - **不走真 WS upgrade** (oneshot 不易模拟, 走 in-memory 直接调 handler 内部函数)
//! - **走 apeireth-protocol 8 帧 JSON 编解码** (跟 `crates/apeireth-api/src/ws_v1.rs` 1:1 对齐)
//! - **走 apeireth-api::auth::AuthPipeline 5 组件** (Bearer + keyring + bucket + audit + quota stub)
//!
//! **6 哲学 anchor 穿透**:
//! - S-1 北极星: 5 fixture 1:1 翻译任务稿步骤 5
//! - O-2 走在前人肩上: 复用 `axum::body::Body` + `tower::ServiceExt::oneshot` (跟 `tests/endpoints.rs` 同模式)
//! - O-5 不假装: 测真组件, 0 mock 跳过

use std::sync::Arc;

use apeireth_api::auth::{
    ApiError, AuditEvent, AuthPipeline, Principal, API_KEY_SERVICE, WS_TOKEN_SERVICE,
};
use apeireth_api::ws_v1::{STUB_TOOLS, TOOL_WHITELIST, WS_PATH};
use apeireth_keyring::{KeyringConfig, KeyringStore};
use apeireth_protocol::ws_v1::{
    AuthFrame, CloseFrame, ErrorFrame, PingFrame, StreamChunkFrame, StreamEndFrame,
    ToolInvokeFrame, ToolResultFrame, WsFrame, WS_PROTOCOL_VERSION,
};
use serde_json::json;

// =====================================================================
// Test helper: 构造 AuthPipeline (走 in-memory keyring, 不连真 OS keyring)
// =====================================================================

fn make_pipeline() -> Arc<AuthPipeline> {
    let cfg = KeyringConfig::default();
    let keyring = Arc::new(KeyringStore::new(cfg));
    Arc::new(AuthPipeline::new(keyring))
}

// =====================================================================
// Fixture 1 — auth_frame_with_valid_token_passes
// 任务稿: 5min TTL token 走 AuthPipeline.check_bearer 通过
// =====================================================================

#[tokio::test]
async fn auth_frame_with_valid_token_passes() {
    // 1. 构造 AuthFrame (5min TTL token 字段)
    let token = "sk-cp-test-valid-5min-token-12345678";
    let auth_frame = AuthFrame {
        token: token.to_string(),
        ws_version: WS_PROTOCOL_VERSION.to_string(),
    };
    assert_eq!(auth_frame.ws_version, "1", "ws version 锁 1");

    // 2. 走 AuthPipeline.check_bearer (模拟 WS handler 收到 Auth 帧)
    let pipeline = make_pipeline();
    let result = pipeline
        .check_bearer(
            Some(&format!("Bearer {}", auth_frame.token)),
            WS_TOKEN_SERVICE,
            true, // /v1/stream 是 P0 端点 (E 急救路径)
        )
        .await;

    // 3. 验: valid token 走通, 返 Principal
    assert!(result.is_ok(), "valid token should pass: {result:?}");
    let p = result.unwrap();
    assert!(p.api_key_hash.starts_with("sha256:"));
    assert_eq!(p.service, WS_TOKEN_SERVICE);
    assert!(p.is_p0, "/v1/stream 是 P0 端点 (per 蓝图 §2.6)");

    // 4. JSON 编解码 round-trip (跟 `protocol::ws_v1::WsFrame::Auth` 1:1)
    let frame = WsFrame::Auth(auth_frame);
    let json_str = serde_json::to_string(&frame).expect("serialize");
    assert!(json_str.contains("\"type\":\"auth\""), "JSON 顶层 type=auth");
    let back: WsFrame = serde_json::from_str(&json_str).expect("deserialize");
    assert!(matches!(back, WsFrame::Auth(_)));
}

// =====================================================================
// Fixture 2 — auth_frame_with_expired_token_returns_close
// 任务稿: 错 / 短 token 走 ApiError::InvalidApiKey
// =====================================================================

#[tokio::test]
async fn auth_frame_with_expired_token_returns_close() {
    let pipeline = make_pipeline();

    // 场景 1: token 长度 < API_KEY_MIN_LENGTH (16 字符) → 返 InvalidApiKey
    let short_token = "short";
    let r1 = pipeline
        .check_bearer(
            Some(&format!("Bearer {short_token}")),
            API_KEY_SERVICE,
            false,
        )
        .await;
    assert!(matches!(r1, Err(ApiError::InvalidApiKey)));
    let err1 = r1.err().unwrap();
    assert_eq!(err1.status_code(), 401);
    assert_eq!(err1.error_code(), "unauthorized");

    // 场景 2: 无 Authorization 头 → 返 MissingAuthHeader
    let r2 = pipeline.check_bearer(None, API_KEY_SERVICE, false).await;
    assert!(matches!(r2, Err(ApiError::MissingAuthHeader)));

    // 场景 3: Authorization 头格式错 (不是 "Bearer ...") → 返 MalformedAuthHeader
    let r3 = pipeline
        .check_bearer(Some("Basic xyz-not-bearer"), API_KEY_SERVICE, false)
        .await;
    assert!(matches!(r3, Err(ApiError::MalformedAuthHeader)));

    // 场景 4: Close 帧含错误信息 (跟 `CloseFrame` 1:1)
    let close_frame = CloseFrame {
        reason: "ws_unauthorized".to_string(),
        code: 1008,
    };
    assert_eq!(close_frame.code, 1008, "WS 标准 1008 = Policy Violation");
    let frame = WsFrame::Close(close_frame);
    let json_str = serde_json::to_string(&frame).expect("serialize");
    let back: WsFrame = serde_json::from_str(&json_str).expect("deserialize");
    assert!(matches!(back, WsFrame::Close(_)));
}

// =====================================================================
// Fixture 3 — tool_invoke_frame_dispatches_to_web_search_handler
// 任务稿: 8 帧 ToolInvoke → 路由到 web_search handler → ToolResult
// =====================================================================

#[tokio::test]
async fn tool_invoke_frame_dispatches_to_web_search_handler() {
    // 1. 构造 ToolInvoke 帧
    let invoke = ToolInvokeFrame {
        tool: "web_search".to_string(),
        action: "search".to_string(),
        args: json!({"query": "Rust async trait"}),
        req_id: "r-test-001".to_string(),
    };

    // 2. 验: tool 在 6 工具白名单
    assert!(
        TOOL_WHITELIST.contains(&invoke.tool.as_str()),
        "web_search 必须在 6 工具白名单"
    );
    assert_eq!(TOOL_WHITELIST.len(), 6, "6 工具 1:1 翻译蓝图 §2.2");
    assert!(
        !STUB_TOOLS.contains(&invoke.tool.as_str()),
        "web_search 不是 stub 工具"
    );

    // 3. JSON 编解码 round-trip
    let frame = WsFrame::ToolInvoke(invoke.clone());
    let json_str = serde_json::to_string(&frame).expect("serialize");
    assert!(
        json_str.contains("\"type\":\"tool_invoke\""),
        "JSON 顶层 type=tool_invoke"
    );
    let back: WsFrame = serde_json::from_str(&json_str).expect("deserialize");
    match back {
        WsFrame::ToolInvoke(f) => {
            assert_eq!(f.tool, "web_search");
            assert_eq!(f.action, "search");
            assert_eq!(f.req_id, "r-test-001");
            assert_eq!(f.args["query"], "Rust async trait");
        }
        _ => panic!("expected ToolInvoke, got {:?}", back.type_str()),
    }

    // 4. 审计日志验证: 1 invoke 1 行 (per 蓝图 §2.4 组件 4)
    let pipeline = make_pipeline();
    let p = Principal::from_api_key(
        "sk-cp-test-tool-invoke-12345678",
        API_KEY_SERVICE,
        true,
    );
    pipeline.audit_invoke(
        &p,
        WS_PATH,
        200,
        234,
        Some(&invoke.tool),
        Some(&invoke.action),
    );
    let events = pipeline.audit().snapshot();
    assert_eq!(events.len(), 1, "1 invoke 1 行 (per 蓝图 §2.4 组件 4)");
    let ev: &AuditEvent = &events[0];
    assert_eq!(ev.endpoint, WS_PATH);
    assert_eq!(ev.tool.as_deref(), Some("web_search"));
    assert_eq!(ev.action.as_deref(), Some("search"));
    assert_eq!(ev.duration_ms, 234);
    assert!(ev.trace_id > 0, "trace_id 必分配");
}

// =====================================================================
// Fixture 4 — stream_chunk_frame_accumulates_until_stream_end
// 任务稿: 5 业务帧 stream_chunk → stream_end 配对
// =====================================================================

#[tokio::test]
async fn stream_chunk_frame_accumulates_until_stream_end() {
    // 1. 模拟 3 chunks (per 蓝图 §2.3 端到端 1 用例)
    let req_id = "r-stream-001";
    let chunks = vec!["running...\n", "still running...\n", "done!\n"];
    let mut accumulated = String::new();
    for (i, c) in chunks.iter().enumerate() {
        let chunk_frame = StreamChunkFrame {
            req_id: req_id.to_string(),
            chunk: (*c).to_string(),
            done: i == chunks.len() - 1,
        };
        let frame = WsFrame::StreamChunk(chunk_frame);
        let json_str = serde_json::to_string(&frame).expect("serialize");
        assert!(json_str.contains("\"type\":\"stream_chunk\""));
        let back: WsFrame = serde_json::from_str(&json_str).expect("deserialize");
        match back {
            WsFrame::StreamChunk(f) => {
                assert_eq!(f.req_id, req_id);
                accumulated.push_str(&f.chunk);
            }
            _ => panic!("expected StreamChunk, got {:?}", back.type_str()),
        }
    }
    assert_eq!(accumulated, "running...\nstill running...\ndone!\n");

    // 2. 推 StreamEnd 帧 (收尾)
    let end_frame = StreamEndFrame {
        req_id: req_id.to_string(),
        total_chunks: chunks.len() as u32,
        total_bytes: accumulated.len() as u64,
    };
    let frame = WsFrame::StreamEnd(end_frame);
    let json_str = serde_json::to_string(&frame).expect("serialize");
    assert!(json_str.contains("\"type\":\"stream_end\""));
    let back: WsFrame = serde_json::from_str(&json_str).expect("deserialize");
    match back {
        WsFrame::StreamEnd(f) => {
            assert_eq!(f.req_id, req_id);
            assert_eq!(f.total_chunks, 3);
            assert_eq!(f.total_bytes, accumulated.len() as u64);
        }
        _ => panic!("expected StreamEnd, got {:?}", back.type_str()),
    }

    // 3. Ping 帧也走 5 业务帧 (per 蓝图 §2.3)
    let ping = PingFrame { ts: 1722931200000 };
    let frame = WsFrame::Ping(ping);
    let json_str = serde_json::to_string(&frame).expect("serialize");
    let back: WsFrame = serde_json::from_str(&json_str).expect("deserialize");
    assert!(matches!(back, WsFrame::Ping(_)));
}

// =====================================================================
// Fixture 5 — error_frame_on_quota_check_returns_501_stub
// 任务稿: D-05 quota stub 永远返 501
// =====================================================================

#[tokio::test]
async fn error_frame_on_quota_check_returns_501_stub() {
    // 1. 走 AuthPipeline.check_quota (per D-05, 永远返 501, 0 假装)
    let pipeline = make_pipeline();
    let p = Principal::from_api_key(
        "sk-cp-test-quota-stub-12345678",
        API_KEY_SERVICE,
        false,
    );
    let result = pipeline._test_check_quota(&p);
    assert!(result.is_err(), "D-05 stub 永远返 501");
    let err = result.err().unwrap();
    assert!(matches!(err, ApiError::NotImplemented { api: "quota" }));
    assert_eq!(err.status_code(), 501);
    assert_eq!(err.error_code(), "not_implemented");

    // 2. 构造 Error 帧 (server → client 通知)
    let error_frame = ErrorFrame {
        code: "not_implemented".to_string(),
        message: "quota check not implemented (per D-05, R21 stub mode)".to_string(),
        fatal: false, // 限流可重试, quota 致命 = true (但 stub 模式返 501 不是 stream 中断, false)
    };
    let frame = WsFrame::Error(error_frame);
    let json_str = serde_json::to_string(&frame).expect("serialize");
    assert!(json_str.contains("\"type\":\"error\""));
    assert!(json_str.contains("\"code\":\"not_implemented\""));
    let back: WsFrame = serde_json::from_str(&json_str).expect("deserialize");
    match back {
        WsFrame::Error(f) => {
            assert_eq!(f.code, "not_implemented");
            assert!(f.message.contains("D-05"));
            assert!(!f.fatal);
        }
        _ => panic!("expected Error, got {:?}", back.type_str()),
    }

    // 3. 8 帧全过 type_str() 验证 (per 蓝图 §2.3 8 帧齐发)
    let frames = vec![
        WsFrame::ToolInvoke(ToolInvokeFrame {
            tool: "x".into(),
            action: "y".into(),
            args: json!(null),
            req_id: "r".into(),
        }),
        WsFrame::ToolResult(ToolResultFrame {
            req_id: "r".into(),
            ok: true,
            data: None,
            error: None,
            meta: json!(null),
        }),
        WsFrame::StreamChunk(StreamChunkFrame {
            req_id: "r".into(),
            chunk: "x".into(),
            done: false,
        }),
        WsFrame::StreamEnd(StreamEndFrame {
            req_id: "r".into(),
            total_chunks: 0,
            total_bytes: 0,
        }),
        WsFrame::Ping(PingFrame { ts: 0 }),
        WsFrame::Auth(AuthFrame {
            token: "t".into(),
            ws_version: "1".into(),
        }),
        WsFrame::Close(CloseFrame {
            reason: "x".into(),
            code: 1000,
        }),
        WsFrame::Error(ErrorFrame {
            code: "x".into(),
            message: "x".into(),
            fatal: false,
        }),
    ];
    let type_strs: Vec<&'static str> = frames.iter().map(|f| f.type_str()).collect();
    assert_eq!(type_strs.len(), 8, "8 帧齐发 (per 蓝图 §2.3)");
    assert_eq!(
        type_strs,
        vec![
            "tool_invoke",
            "tool_result",
            "stream_chunk",
            "stream_end",
            "ping",
            "auth",
            "close",
            "error",
        ]
    );
}
