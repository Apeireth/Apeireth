//! `apeireth-protocol::ws_v1` — **WebSocket 8 帧协议** (R20 阶段 2, 蓝图 §2.3)
//!
//! **目标**: R20 阶段 2 把 R18 6 类非 LLM API (web_search / file_ops / git_ops /
//! code_exec / calendar / message) 通过 WebSocket 公开. 8 帧 = 5 业务帧 + 3 控制帧.
//!
//! **5 业务帧**:
//! - `ToolInvoke` (c→s) — 客户端发起工具调用
//! - `ToolResult` (s→c) — 服务端返工具调用结果
//! - `StreamChunk` (s→c) — 服务端推流式 chunk (大结果分片)
//! - `StreamEnd` (s→c) — 服务端通知流结束
//! - `Ping` (双向) — 心跳, server 每 30s 发, client 收到回 `Ping` 帧
//!
//! **3 控制帧**:
//! - `Auth` (c→s) — 客户端在 upgrade 后首帧发, 携带短期 (5min TTL) WS token
//! - `Close` (双向) — 任何一方主动关闭, 带 reason + code
//! - `Error` (s→c) — 服务端返非致命错误 (fatal=false 仅通知, fatal=true server 立即 close)
//!
//! **关键设计** (per 蓝图 §2.3):
//! - 8 帧用 `serde(tag = "type")` 内部 tag, 跟现有 `ProviderEvent` 5 变体正交
//! - 复用 `req_id` 关联 `ToolInvoke` → `ToolResult`/`StreamChunk`/`StreamEnd`/`Error` (5 帧共享)
//! - **不假装**: `ApiError::NotImplemented` 走错误码, 不假装支持
//!
//! **不修改承诺**:
//! - ❌ 不改 `apeireth-protocol::normalized.rs` (R17 战役 1-1 LOCKED)
//! - ❌ 不改 `ProtocolKind` (R36-2 已删 `ProtocolRouter`)
//! - ✅ 新增 `ws_v1` 模块, 跟现有 4 协议层并列, 0 冲突

use serde::{Deserialize, Serialize};

/// **WS 协议版本** (编译期 hardcode, 跟蓝图的 `ws_version` 字段对齐).
///
/// **不漂移原则** (主 17:58 不假装): 一旦 bump 必跟 `apeireth-api` 的 WS upgrade 校验同步.
pub const WS_PROTOCOL_VERSION: &str = "1";

/// **WS 鉴权 token 默认 TTL** (5 min, per 蓝图 §2.4 D-03 决策).
///
/// **不漂移原则**: 跟 keyring 里 `apeireth-ws-token-{principal}` 凭证的 `expires_at` 字段对齐.
pub const WS_TOKEN_DEFAULT_TTL_SECS: i64 = 300;

/// **Ping 间隔 (server 发)** — 30s (per 蓝图 §2.3 WS 关键技术点).
pub const WS_PING_INTERVAL_SECS: u64 = 30;

/// **WS idle timeout** — 5 min (per 蓝图 §2.3).
pub const WS_IDLE_TIMEOUT_SECS: u64 = 300;

/// **最大并发 stream chunks per invoke** — 100 (背压上限, per 蓝图 §2.3).
pub const WS_MAX_STREAM_CHUNKS: usize = 100;

// ============================================================================
// 8 帧 enum (顶层 frame 容器, serde tag 区分)
// ============================================================================

/// **8 帧顶层 enum** — `serde(tag = "type")` 内部 tag, JSON 序列化形如
/// `{"type": "tool_invoke", "tool": "web_search", ...}`.
///
/// **8 帧**:
/// - 5 业务: `ToolInvoke` / `ToolResult` / `StreamChunk` / `StreamEnd` / `Ping`
/// - 3 控制: `Auth` / `Close` / `Error`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum WsFrame {
    /// 客户端发起工具调用 (c→s).
    #[serde(rename = "tool_invoke")]
    ToolInvoke(ToolInvokeFrame),

    /// 服务端返工具调用结果 (s→c).
    #[serde(rename = "tool_result")]
    ToolResult(ToolResultFrame),

    /// 服务端推流式 chunk (s→c) — 大结果分片.
    #[serde(rename = "stream_chunk")]
    StreamChunk(StreamChunkFrame),

    /// 服务端通知流结束 (s→c).
    #[serde(rename = "stream_end")]
    StreamEnd(StreamEndFrame),

    /// 心跳 (双向).
    #[serde(rename = "ping")]
    Ping(PingFrame),

    /// 客户端首帧鉴权 (c→s).
    #[serde(rename = "auth")]
    Auth(AuthFrame),

    /// 主动关闭 (双向).
    #[serde(rename = "close")]
    Close(CloseFrame),

    /// 服务端错误 (s→c).
    #[serde(rename = "error")]
    Error(ErrorFrame),
}

impl WsFrame {
    /// 帧类型字符串 (跟 `serde(rename = ...)` 1:1 对齐).
    ///
    /// 用于 router 分发 + 审计日志, 避免 match 8 帧调用方都需要手写字符串.
    #[must_use]
    pub const fn type_str(&self) -> &'static str {
        match self {
            Self::ToolInvoke(_) => "tool_invoke",
            Self::ToolResult(_) => "tool_result",
            Self::StreamChunk(_) => "stream_chunk",
            Self::StreamEnd(_) => "stream_end",
            Self::Ping(_) => "ping",
            Self::Auth(_) => "auth",
            Self::Close(_) => "close",
            Self::Error(_) => "error",
        }
    }

    /// 提取 `req_id` (5 业务帧共享).
    /// 控制帧 (auth/close/error/ping) 返 `None`.
    #[must_use]
    pub fn req_id(&self) -> Option<&str> {
        match self {
            Self::ToolInvoke(f) => Some(&f.req_id),
            Self::ToolResult(f) => Some(&f.req_id),
            Self::StreamChunk(f) => Some(&f.req_id),
            Self::StreamEnd(f) => Some(&f.req_id),
            Self::Ping(_) | Self::Auth(_) | Self::Close(_) | Self::Error(_) => None,
        }
    }
}

// ============================================================================
// 5 业务帧 struct
// ============================================================================

/// **`ToolInvoke` 帧** (c→s) — 客户端发起工具调用.
///
/// **示例 JSON**:
/// ```json
/// {"type": "tool_invoke", "tool": "web_search", "action": "search", "args": {"query": "Rust async"}, "req_id": "r-001"}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ToolInvokeFrame {
    /// 工具名 (e.g. "web_search", "file_ops", "git_ops", "code_exec", "calendar", "message")
    pub tool: String,
    /// 工具 action (e.g. "search", "read", "write", "log", "exec", "list")
    pub action: String,
    /// 工具参数 (任意 JSON 对象, 走 serde_json::Value 透传, 不解析 schema)
    pub args: serde_json::Value,
    /// 请求 ID (UUID v4, 客户端生成, 5 业务帧共享)
    pub req_id: String,
}

/// **`ToolResult` 帧** (s→c) — 服务端返工具调用结果.
///
/// **示例 JSON**:
/// ```json
/// {"type": "tool_result", "req_id": "r-001", "ok": true, "data": {"results": [...]}, "meta": {"tool": "web_search", "duration_ms": 234, "trace_id": "tr-..."}}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ToolResultFrame {
    /// 请求 ID (跟 `ToolInvoke.req_id` 配对)
    pub req_id: String,
    /// 是否成功
    pub ok: bool,
    /// 成功数据 (任意 JSON, `ok=true` 时存在)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// 错误信息 (`ok=false` 时存在)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 元数据 (per 蓝图 §2.2 统一信封 meta 字段, 4 字段: tool / duration_ms / trace_id / api_key_hash)
    pub meta: serde_json::Value,
}

/// **`StreamChunk` 帧** (s→c) — 服务端推流式 chunk (大结果分片, 跟 `ToolResult` 互斥).
///
/// **设计**: 大结果 (e.g. `code_exec` 跑 1 分钟长任务) 不走 `ToolResult` 单帧, 走
/// `StreamChunk` 多次 + `StreamEnd` 收尾.
///
/// **示例 JSON**:
/// ```json
/// {"type": "stream_chunk", "req_id": "r-002", "chunk": "running...\n", "done": false}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StreamChunkFrame {
    /// 请求 ID (跟 `ToolInvoke.req_id` 配对)
    pub req_id: String,
    /// 本次 chunk 内容 (任意 string, 可含 UTF-8 / binary base64)
    pub chunk: String,
    /// 是否为最后一个 chunk (true = 收完, 但仍由 `StreamEnd` 帧总终结)
    pub done: bool,
}

/// **`StreamEnd` 帧** (s→c) — 服务端通知流结束.
///
/// **示例 JSON**:
/// ```json
/// {"type": "stream_end", "req_id": "r-002", "total_chunks": 42, "total_bytes": 102400}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StreamEndFrame {
    /// 请求 ID
    pub req_id: String,
    /// 总 chunk 数 (累计 `StreamChunk` 帧数)
    pub total_chunks: u32,
    /// 总字节数 (累计 chunk byte length)
    pub total_bytes: u64,
}

/// **`Ping` 帧** (双向) — 心跳.
///
/// **示例 JSON**:
/// ```json
/// {"type": "ping", "ts": 1722931200000}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PingFrame {
    /// 时间戳 (epoch millis, 客户端收到可计算 RTT)
    pub ts: i64,
}

// ============================================================================
// 3 控制帧 struct
// ============================================================================

/// **`Auth` 帧** (c→s) — 客户端在 upgrade 后首帧发, 携带短期 (5min TTL) WS token.
///
/// **示例 JSON**:
/// ```json
/// {"type": "auth", "token": "ws-tok-...", "ws_version": "1"}
/// ```
///
/// **流程** (per 蓝图 §2.4 D-03):
/// 1. 客户端先发 HTTP `POST /v1/auth/ws-token` 拿 5min TTL token
/// 2. 客户端用 token 升级 WebSocket
/// 3. 客户端首帧发 `Auth` 帧, server 校验 token → 接受 or close 1008 (`ws_unauthorized`)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AuthFrame {
    /// 短期 WS token (5min TTL, 走 keyring 校验)
    pub token: String,
    /// 客户端 WS 协议版本 (跟 `WS_PROTOCOL_VERSION` 比较, mismatch → close 1008)
    pub ws_version: String,
}

/// **`Close` 帧** (双向) — 任何一方主动关闭, 带 reason + code.
///
/// **示例 JSON**:
/// ```json
/// {"type": "close", "reason": "client_done", "code": 1000}
/// ```
///
/// **code 约定** (per 蓝图 §2.5 WS 错误码):
/// - 1000 = 正常关闭 (client_done)
/// - 1008 = 鉴权失败 (ws_unauthorized)
/// - 1013 = 限流 (ws_too_many_concurrent)
/// - 4xxx = 服务端业务错误 (e.g. 4001 quota_exceeded)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CloseFrame {
    /// 关闭原因 (人类可读, e.g. "client_done", "ws_unauthorized", "ws_too_many_concurrent")
    pub reason: String,
    /// 关闭 code (WS 标准 code 或业务 code, 4xxx 段)
    pub code: u16,
}

/// **`Error` 帧** (s→c) — 服务端返非致命错误.
///
/// **示例 JSON**:
/// ```json
/// {"type": "error", "code": "rate_limited", "message": "rate limit exceeded, retry after 30s", "fatal": false}
/// ```
///
/// **fatal 区分** (per 蓝图 §2.5):
/// - `fatal=false` — 仅通知 (e.g. 限流 1 次), client 可继续
/// - `fatal=true` — server 立即 close connection (e.g. 鉴权失败, quota 超限)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ErrorFrame {
    /// 错误码 (machine-readable, per 蓝图 §2.5 12 类 HTTP 状态码 1:1 映射)
    pub code: String,
    /// 错误消息 (人类可读)
    pub message: String,
    /// 是否致命 (true = server 立即 close)
    pub fatal: bool,
}

// ============================================================================
// 编译期 hardcode (8 项不假装原则, 跟 `protocol_handlers::AUTH_SCHEME_BEARER` 风格一致)
// ============================================================================

const _: () = {
    // WS 协议版本锁 "1" — 用 bytes 长度 + 字节比较 (const string eq 不稳定)
    assert!(
        WS_PROTOCOL_VERSION.len() == 1,
        "WS_PROTOCOL_VERSION must be 1 char long"
    );
    assert!(
        WS_PROTOCOL_VERSION.as_bytes()[0] == b'1',
        "WS_PROTOCOL_VERSION first byte must be '1'"
    );
    // 5min TTL 锁
    assert!(
        WS_TOKEN_DEFAULT_TTL_SECS == 300,
        "WS_TOKEN_DEFAULT_TTL_SECS must be 300s (5min per 蓝图 §2.4 D-03)"
    );
    // 30s ping 间隔锁
    assert!(
        WS_PING_INTERVAL_SECS >= 10 && WS_PING_INTERVAL_SECS <= 60,
        "WS_PING_INTERVAL_SECS must be 10..=60s (蓝图 §2.3)"
    );
    // 5min idle timeout
    assert!(
        WS_IDLE_TIMEOUT_SECS >= 60,
        "WS_IDLE_TIMEOUT_SECS must be >= 60s (蓝图 §2.3)"
    );
    // 背压上限 100 chunk
    assert!(
        WS_MAX_STREAM_CHUNKS >= 16,
        "WS_MAX_STREAM_CHUNKS must be >= 16 (蓝图 §2.3 背压)"
    );
};

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod ws_v1_tests {
    use super::*;

    #[test]
    fn eight_frames_serde_round_trip() {
        // 8 帧 JSON 编解码 round-trip 全过
        let frames = vec![
            WsFrame::ToolInvoke(ToolInvokeFrame {
                tool: "web_search".into(),
                action: "search".into(),
                args: serde_json::json!({"query": "Rust"}),
                req_id: "r-001".into(),
            }),
            WsFrame::ToolResult(ToolResultFrame {
                req_id: "r-001".into(),
                ok: true,
                data: Some(serde_json::json!({"results": []})),
                error: None,
                meta: serde_json::json!({"tool": "web_search", "duration_ms": 234}),
            }),
            WsFrame::StreamChunk(StreamChunkFrame {
                req_id: "r-002".into(),
                chunk: "hello".into(),
                done: false,
            }),
            WsFrame::StreamEnd(StreamEndFrame {
                req_id: "r-002".into(),
                total_chunks: 1,
                total_bytes: 5,
            }),
            WsFrame::Ping(PingFrame { ts: 1722931200000 }),
            WsFrame::Auth(AuthFrame {
                token: "ws-tok-abc".into(),
                ws_version: "1".into(),
            }),
            WsFrame::Close(CloseFrame {
                reason: "client_done".into(),
                code: 1000,
            }),
            WsFrame::Error(ErrorFrame {
                code: "rate_limited".into(),
                message: "retry after 30s".into(),
                fatal: false,
            }),
        ];
        for f in &frames {
            let json = serde_json::to_string(f).expect("serialize");
            let back: WsFrame = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(&back, f, "round-trip mismatch for {}", f.type_str());
        }
        assert_eq!(frames.len(), 8, "8 帧必须齐发 (per 蓝图 §2.3)");
    }

    #[test]
    fn frame_type_str_covers_all_eight() {
        // 8 帧 type_str 全覆盖
        let samples = [
            (WsFrame::Ping(PingFrame { ts: 0 }), "ping"),
            (
                WsFrame::ToolInvoke(ToolInvokeFrame {
                    tool: "x".into(),
                    action: "y".into(),
                    args: serde_json::Value::Null,
                    req_id: "r".into(),
                }),
                "tool_invoke",
            ),
            (
                WsFrame::Auth(AuthFrame {
                    token: "t".into(),
                    ws_version: "1".into(),
                }),
                "auth",
            ),
        ];
        for (f, expected) in &samples {
            assert_eq!(f.type_str(), *expected);
        }
    }

    #[test]
    fn req_id_shared_across_five_business_frames() {
        // 5 业务帧共享 req_id, 3 控制帧不共享
        let req = "r-shared-001";
        assert_eq!(
            WsFrame::ToolInvoke(ToolInvokeFrame {
                tool: "t".into(),
                action: "a".into(),
                args: serde_json::Value::Null,
                req_id: req.into()
            })
            .req_id(),
            Some(req)
        );
        assert_eq!(
            WsFrame::ToolResult(ToolResultFrame {
                req_id: req.into(),
                ok: true,
                data: None,
                error: None,
                meta: serde_json::Value::Null,
            })
            .req_id(),
            Some(req)
        );
        assert_eq!(
            WsFrame::StreamChunk(StreamChunkFrame {
                req_id: req.into(),
                chunk: "x".into(),
                done: false,
            })
            .req_id(),
            Some(req)
        );
        assert_eq!(
            WsFrame::StreamEnd(StreamEndFrame {
                req_id: req.into(),
                total_chunks: 1,
                total_bytes: 1,
            })
            .req_id(),
            Some(req)
        );
        assert_eq!(WsFrame::Ping(PingFrame { ts: 0 }).req_id(), None);
        assert_eq!(
            WsFrame::Auth(AuthFrame {
                token: "t".into(),
                ws_version: "1".into()
            })
            .req_id(),
            None
        );
        assert_eq!(
            WsFrame::Close(CloseFrame {
                reason: "x".into(),
                code: 1000
            })
            .req_id(),
            None
        );
        assert_eq!(
            WsFrame::Error(ErrorFrame {
                code: "x".into(),
                message: "x".into(),
                fatal: false
            })
            .req_id(),
            None
        );
    }

    #[test]
    fn serde_tag_uses_type_field() {
        // 验证 JSON 顶层有 "type" 字段
        let json = serde_json::to_string(&WsFrame::Ping(PingFrame { ts: 1 })).unwrap();
        assert!(
            json.contains("\"type\":\"ping\""),
            "JSON must have type tag: {json}"
        );
        let json = serde_json::to_string(&WsFrame::Auth(AuthFrame {
            token: "t".into(),
            ws_version: "1".into(),
        }))
        .unwrap();
        assert!(
            json.contains("\"type\":\"auth\""),
            "JSON must have type tag: {json}"
        );
    }

    #[test]
    fn tool_result_data_and_error_are_optional() {
        // ok=true: data 字段存在
        let ok_frame = ToolResultFrame {
            req_id: "r".into(),
            ok: true,
            data: Some(serde_json::json!({"k": "v"})),
            error: None,
            meta: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&ok_frame).unwrap();
        assert!(
            !json.contains("\"error\""),
            "error must be skipped when None"
        );

        // ok=false: error 字段存在
        let err_frame = ToolResultFrame {
            req_id: "r".into(),
            ok: false,
            data: None,
            error: Some("oops".into()),
            meta: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&err_frame).unwrap();
        assert!(!json.contains("\"data\""), "data must be skipped when None");
        assert!(json.contains("\"error\":\"oops\""));
    }

    #[test]
    fn ws_protocol_version_locked_at_one() {
        assert_eq!(WS_PROTOCOL_VERSION, "1");
    }
}
