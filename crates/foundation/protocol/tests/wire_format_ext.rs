//! Integration tests for apeireth-protocol (extended boundary cases)
//!
//! **R18 路线图 Stage 2 续**: 在 `tests/wire_format.rs` 16 个基线测试基础上,
//! 加 8 个边界 case 覆盖 WS 8 帧 / bridge_ext / ProtocolError 错误构造函数 /
//! 字符串前缀错误检测 (中文).
//!
//! **不假装**: 测试是真跑 (`cargo nextest run -p apeireth-protocol --test wire_format_ext`).

use apeireth_protocol::bridge_ext::{BridgeExtError, BridgeKind, ExtendedBridge, QueueBridge};
use apeireth_protocol::{
    is_tool_result_error, ProtocolError, StreamBridge, StreamChunkFrame, ToolInvokeFrame,
    ToolResultFrame, WsFrame, WS_PROTOCOL_VERSION, WS_TOKEN_DEFAULT_TTL_SECS,
};
use serde_json::{json, Value};

// =====================================================================
// WS 8 帧 (R20 阶段 2) — type_str + JSON round-trip
// =====================================================================

#[test]
fn ws_frame_type_str_returns_8_distinct_strings() {
    // 8 帧 type_str 应返 8 个唯一字符串 (VCP §3.2 设计意图)
    let frames = [
        WsFrame::ToolInvoke(ToolInvokeFrame {
            req_id: "r1".into(),
            tool: "x".into(),
            action: "search".into(),
            args: json!({}),
        }),
        WsFrame::ToolResult(ToolResultFrame {
            req_id: "r1".into(),
            ok: true,
            data: Some(json!({})),
            error: None,
            meta: json!({}),
        }),
        WsFrame::StreamChunk(StreamChunkFrame {
            req_id: "r1".into(),
            chunk: "x".into(),
            done: false,
        }),
        WsFrame::Ping(apeireth_protocol::PingFrame { ts: 0 }),
        WsFrame::Auth(apeireth_protocol::AuthFrame {
            token: "t".into(),
            ws_version: "1".into(),
        }),
        WsFrame::Close(apeireth_protocol::CloseFrame {
            reason: "x".into(),
            code: 1000,
        }),
        WsFrame::Error(apeireth_protocol::ErrorFrame {
            code: "x".into(),
            message: "m".into(),
            fatal: false,
        }),
        WsFrame::StreamEnd(apeireth_protocol::StreamEndFrame {
            req_id: "r1".into(),
            total_chunks: 1,
            total_bytes: 1,
        }),
    ];
    let types: Vec<&str> = frames.iter().map(|f| f.type_str()).collect();
    let mut unique = types.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 8, "8 帧 type_str 应唯一, got {types:?}");
}

#[test]
fn ws_frame_tool_invoke_json_roundtrip() {
    // ToolInvoke 序列化后 type="tool_invoke", 反序列化还原
    let frame = WsFrame::ToolInvoke(ToolInvokeFrame {
        req_id: "req-42".into(),
        tool: "web_search".into(),
        action: "search".into(),
        args: json!({"query": "rust"}),
    });
    let s = serde_json::to_string(&frame).expect("serialize");
    assert!(
        s.contains("\"type\":\"tool_invoke\""),
        "frame should have type tag: {s}"
    );
    let back: WsFrame = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(back.type_str(), "tool_invoke");
    if let WsFrame::ToolInvoke(inv) = &back {
        assert_eq!(inv.req_id, "req-42");
        assert_eq!(inv.tool, "web_search");
        assert_eq!(inv.action, "search");
        assert_eq!(inv.args["query"], "rust");
    } else {
        panic!("expected ToolInvoke, got {:?}", back);
    }
}

#[test]
fn ws_frame_stream_chunk_preserves_done_flag() {
    // StreamChunk 序列化保留 done 字段
    let frame = WsFrame::StreamChunk(StreamChunkFrame {
        req_id: "r1".into(),
        chunk: "hello world".into(),
        done: true,
    });
    let s = serde_json::to_string(&frame).expect("serialize");
    assert!(s.contains("\"done\":true"), "应含 done: {s}");
    let back: WsFrame = serde_json::from_str(&s).expect("deserialize");
    if let WsFrame::StreamChunk(c) = back {
        assert!(c.done);
        assert_eq!(c.chunk, "hello world");
        assert_eq!(c.req_id, "r1");
    } else {
        panic!("expected StreamChunk");
    }
}

#[test]
fn ws_protocol_version_and_ttl_constants() {
    // 蓝图 §2.3 / §2.4 D-03 决策
    assert_eq!(WS_PROTOCOL_VERSION, "1", "WS 协议版本");
    assert_eq!(WS_TOKEN_DEFAULT_TTL_SECS, 300, "WS token 5min TTL");
}

// =====================================================================
// bridge_ext — Stream + Queue 真测
// =====================================================================

#[test]
fn stream_bridge_concatenates_chunks() {
    // 推 2 个 chunk, finish 拿拼接结果
    let mut b = StreamBridge::new();
    b.push_chunk("hello ");
    b.push_chunk("world");
    assert_eq!(b.chunk_count(), 2);
    let s = b.finish().expect("finish utf8");
    assert_eq!(s, "hello world");
    // finish 后 count 归零
    assert_eq!(b.chunk_count(), 0);
}

#[test]
fn stream_bridge_finish_invalid_utf8_returns_err() {
    // 推非法 UTF-8 字节 → finish 返 Err
    let mut b = StreamBridge::new();
    b.push_chunk(&[0xFF, 0xFE, 0xFD][..]); // 单独 bytes 非法 UTF-8
    let r = b.finish();
    assert!(matches!(r, Err(BridgeExtError::InvalidUtf8)), "got: {r:?}");
}

#[test]
fn queue_bridge_capacity_limit() {
    // 容量 2 的 queue, enqueue 3 个 → 第 3 个应返 QueueFull
    let mut q: QueueBridge<i32> = QueueBridge::new(2).expect("new");
    assert_eq!(q.bridge_kind(), BridgeKind::Queue);
    assert!(q.enqueue(1).is_ok());
    assert!(q.enqueue(2).is_ok());
    let r = q.enqueue(3);
    assert!(
        matches!(r, Err(BridgeExtError::QueueFull { capacity: 2 })),
        "got: {r:?}"
    );
    // dequeue 1 个释放空间 → 下一个能 enqueue
    assert_eq!(q.dequeue(), Some(1));
    assert!(q.enqueue(3).is_ok());
}

#[test]
fn queue_bridge_zero_capacity_fails_construct() {
    // 0 容量构造应返 Err
    let r: Result<QueueBridge<i32>, _> = QueueBridge::new(0);
    assert!(matches!(r, Err(BridgeExtError::ZeroCapacity)));
}

// =====================================================================
// is_tool_result_error + ProtocolError (扩展既有 5 测试)
// =====================================================================

#[test]
fn is_tool_result_error_chinese_prefix() {
    // 中文错误前缀 (VCP §3.2 多语言支持)
    assert!(is_tool_result_error(&json!("[错误] 数据库连接失败")));
    assert!(is_tool_result_error(&json!("错误：权限不足")));
    assert!(is_tool_result_error(&json!("失败：网络超时")));
    // 业务正文里包含"错误"不算
    assert!(!is_tool_result_error(&json!("我犯了一个错误")));
}

#[test]
fn protocol_error_constructors() {
    // 5 错误构造器 (per error.rs API)
    let e1 = ProtocolError::parse("messages[0]", "expected array");
    assert!(matches!(e1, ProtocolError::Parse { .. }));
    let e2 = ProtocolError::missing("model");
    assert!(matches!(e2, ProtocolError::Missing { .. }));
    let e3 = ProtocolError::invalid("temperature", "must be >= 0");
    assert!(matches!(e3, ProtocolError::Invalid { .. }));
    let e4 = ProtocolError::unsupported("audio content");
    assert!(matches!(e4, ProtocolError::Unsupported { .. }));
    let e5 = ProtocolError::inconsistent("tools empty but tool_choice=required");
    assert!(matches!(e5, ProtocolError::Inconsistent { .. }));
    // Display 应含 field / message (smoke test)
    assert!(e1.to_string().contains("messages[0]"));
    assert!(e2.to_string().contains("model"));
}
