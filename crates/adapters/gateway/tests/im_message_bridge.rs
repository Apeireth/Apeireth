//! 件二 (双向消息桥) 集成测试: 文本回合双向 / 分段截断 / 会话映射持久 /
//! 签名校验 / 真实对话链 / fail-closed 边界。外部端点一律本地 mock, 0 真实网络。

mod common;

use std::sync::{Arc, Mutex};

use apeireth_core::clock::Clock;
use apeireth_core::kernel::SessionId;
use apeireth_gateway::{
    assemble_im_channels, im_sessions_path, CanonicalChainHandler, ImApprovalCloser, ImBridge,
    ImBridgeError, ImChannelConfig, ImChannelSpec, ImInboundResult, ImSessionMapStore,
    ImTurnHandler, ImTurnOutcome, MemoryApprovalAudit, IM_SIGNATURE_HEADER,
};
use apeireth_sdk::im::{
    parse_inbound_event, ImChannelKind, ImHttpSender, ImInboundEvent, ImReconnectPolicy, ImSecret,
    ImSegmentPolicy,
};
use async_trait::async_trait;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::{allow_all, build_runtime, feishu_text_event, test_clock, FakeProvider};

/// 脚本对话链 (记录收到的回合)。
struct ScriptedHandler {
    reply: String,
    seen: Mutex<Vec<(SessionId, String)>>,
}

impl ScriptedHandler {
    fn new(reply: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            reply: reply.into(),
            seen: Mutex::new(Vec::new()),
        })
    }

    fn seen(&self) -> Vec<(SessionId, String)> {
        self.seen.lock().unwrap().clone()
    }
}

#[async_trait]
impl ImTurnHandler for ScriptedHandler {
    async fn handle_text(&self, session: SessionId, text: &str) -> ImTurnOutcome {
        self.seen.lock().unwrap().push((session, text.to_string()));
        ImTurnOutcome {
            text: self.reply.clone(),
            approval: None,
        }
    }
}

struct RejectingResolver;

#[async_trait]
impl apeireth_gateway::ImApprovalResolver for RejectingResolver {
    async fn resolve(
        &self,
        _session: SessionId,
        _approval_ref: &str,
        decision: &str,
    ) -> apeireth_gateway::ImApprovalResolution {
        apeireth_gateway::ImApprovalResolution::Resumed {
            text: String::new(),
            label: if decision == "approve" {
                "approved".to_string()
            } else {
                "rejected".to_string()
            },
        }
    }
}

fn channel_spec(secret: Option<ImSecret>) -> ImChannelSpec {
    ImChannelSpec {
        id: "primary".to_string(),
        kind: ImChannelKind::ImFeishu,
        webhook_or_endpoint: "https://placeholder.invalid/hook".to_string(),
        secret,
        enabled: true,
    }
}

async fn mock_endpoint() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "message_id": "m-out"
        })))
        .mount(&server)
        .await;
    server
}

fn build_bridge(
    endpoint: String,
    secret: Option<ImSecret>,
    handler: Arc<dyn ImTurnHandler>,
    clock: Arc<dyn Clock>,
    data_dir: Option<&std::path::Path>,
) -> ImBridge {
    let mut spec = channel_spec(secret);
    spec.webhook_or_endpoint = endpoint;
    let config = ImChannelConfig::new(vec![spec]);
    let assembly = assemble_im_channels(&config, ImReconnectPolicy::default()).expect("assembly");
    let sessions = match data_dir {
        Some(dir) => Arc::new(Mutex::new(
            ImSessionMapStore::open(dir).expect("open session map"),
        )),
        None => Arc::new(Mutex::new(ImSessionMapStore::in_memory())),
    };
    let audit = Arc::new(MemoryApprovalAudit::new());
    let approvals = Arc::new(ImApprovalCloser::new(Arc::new(RejectingResolver), audit));
    ImBridge::new(
        &assembly,
        Arc::new(ImHttpSender::new(ImReconnectPolicy {
            max_attempts: 2,
            backoff_base_ms: 1,
            backoff_cap_ms: 2,
        })),
        sessions,
        handler,
        approvals,
    )
    .with_clock(clock)
}

async fn posted_bodies(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|request| String::from_utf8_lossy(&request.body).to_string())
        .collect()
}

/// 1. 文本消息 → 会话回合 → 回复流回 IM (mock 端点收到分段消息)。
#[tokio::test]
async fn inbound_text_turns_into_a_reply_on_the_mock_endpoint() {
    let server = mock_endpoint().await;
    let handler = ScriptedHandler::new("你好, 回复已生成");
    let bridge = build_bridge(server.uri(), None, handler.clone(), test_clock(), None);

    let raw = feishu_text_event("oc_demo_chat", "在吗?");
    let result = bridge.handle_inbound("primary", &raw, None).await.unwrap();
    match result {
        ImInboundResult::Replied {
            segments,
            truncated,
            dropped_chars,
            ..
        } => {
            assert_eq!(segments, 1);
            assert!(!truncated);
            assert_eq!(dropped_chars, 0);
        }
        other => panic!("expected a reply, got {other:?}"),
    }
    assert_eq!(handler.seen().len(), 1);
    assert_eq!(handler.seen()[0].1, "在吗?");

    let bodies = posted_bodies(&server).await;
    assert_eq!(bodies.len(), 1);
    assert!(bodies[0].contains("oc_demo_chat"), "{bodies:?}");
    assert!(bodies[0].contains("你好, 回复已生成"), "{bodies:?}");
}

/// 2. 长回复显式分段 + 截断标记 + 丢弃计数。
#[tokio::test]
async fn long_replies_segment_and_truncate_with_an_explicit_marker() {
    let server = mock_endpoint().await;
    let long = "段落一内容很长很长\n段落二内容也很长很长\n段落三会被丢弃";
    let handler = ScriptedHandler::new(long);
    let bridge = build_bridge(server.uri(), None, handler, test_clock(), None).with_segment_policy(
        ImSegmentPolicy::default()
            .with_max_segment_bytes(30)
            .with_max_segments(2)
            .with_truncation_marker("[cut]"),
    );

    let raw = feishu_text_event("oc_demo_chat", "讲讲细节");
    match bridge.handle_inbound("primary", &raw, None).await.unwrap() {
        ImInboundResult::Replied {
            segments,
            truncated,
            dropped_chars,
            ..
        } => {
            assert_eq!(segments, 2);
            assert!(truncated);
            assert!(dropped_chars > 0);
        }
        other => panic!("expected a reply, got {other:?}"),
    }

    let bodies = posted_bodies(&server).await;
    assert_eq!(bodies.len(), 2, "分段逐条投递");
    assert!(bodies[1].contains("[cut]"), "{bodies:?}");
    for body in &bodies {
        let envelope: serde_json::Value = serde_json::from_str(body).unwrap();
        // im-feishu 的 content 是 `{"text": ...}` 的 JSON 字符串, 再解一层看段本身。
        let content: serde_json::Value =
            serde_json::from_str(envelope["content"].as_str().unwrap()).unwrap();
        let segment = content["text"].as_str().unwrap();
        assert!(segment.len() <= 30 + "[cut]".len(), "{segment}");
    }
}

/// 3. IM 会话 ↔ 桌面会话同源且映射持久 (重开存储表仍在)。
#[tokio::test]
async fn im_conversation_maps_to_one_desktop_session_and_persists() {
    let dir = std::env::temp_dir().join(format!("apeireth-im-bridge-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).unwrap();

    let server = mock_endpoint().await;
    let handler = ScriptedHandler::new("ok");
    let bridge = build_bridge(server.uri(), None, handler, test_clock(), Some(&dir));

    let first = bridge
        .handle_inbound(
            "primary",
            &feishu_text_event("oc_demo_chat", "第一条"),
            None,
        )
        .await
        .unwrap();
    let second = bridge
        .handle_inbound(
            "primary",
            &feishu_text_event("oc_demo_chat", "第二条"),
            None,
        )
        .await
        .unwrap();
    let (session_a, session_b) = match (first, second) {
        (
            ImInboundResult::Replied { session: a, .. },
            ImInboundResult::Replied { session: b, .. },
        ) => (a, b),
        other => panic!("expected two replies, got {other:?}"),
    };
    assert_eq!(session_a, session_b, "同一 IM 会话必须同源到同一桌面会话");

    let entries = bridge.session_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].conversation_id, "oc_demo_chat");
    assert_eq!(entries[0].session, session_a);

    // 持久: 重开存储表读到同一映射 (StoredDoc 语义)。
    let reopened = ImSessionMapStore::open(&dir).expect("session map reopens");
    assert_eq!(reopened.lookup("primary", "oc_demo_chat"), Some(session_a));
    assert_eq!(
        reopened.conversation_for(&session_a),
        Some(("primary".to_string(), "oc_demo_chat".to_string()))
    );
    assert!(im_sessions_path(&dir).exists());
    let _ = fs_err::remove_dir_all(&dir);
}

/// 4. 签名校验: 无签/错签拒收, 对签名放行, 错误 0 明文秘密。
#[tokio::test]
async fn inbound_signature_is_verified_without_leaking_the_secret() {
    let server = mock_endpoint().await;
    let secret_value = "shared-super-secret";
    let secret = ImSecret::new(secret_value).unwrap();
    let handler = ScriptedHandler::new("ok");
    let bridge = build_bridge(
        server.uri(),
        Some(secret.clone()),
        handler,
        test_clock(),
        None,
    );

    let raw = feishu_text_event("oc_demo_chat", "在吗?");
    let missing = bridge
        .handle_inbound("primary", &raw, None)
        .await
        .unwrap_err();
    assert!(matches!(missing, ImBridgeError::Verify(_)), "{missing}");
    assert!(!missing.to_string().contains(secret_value), "{missing}");

    let wrong = bridge
        .handle_inbound("primary", &raw, Some("sha256=deadbeef"))
        .await
        .unwrap_err();
    assert!(matches!(wrong, ImBridgeError::Verify(_)), "{wrong}");
    assert!(!wrong.to_string().contains(secret_value), "{wrong}");

    let signature = secret.sign_body(raw.as_bytes());
    assert!(matches!(
        bridge
            .handle_inbound("primary", &raw, Some(&signature))
            .await
            .unwrap(),
        ImInboundResult::Replied { .. }
    ));
}

/// 5. 走既有对话链: 真实 canonical 运行时回合经桥流回 IM。
#[tokio::test]
async fn the_real_conversation_chain_serves_the_bridge() {
    let server = mock_endpoint().await;
    let provider = FakeProvider::new();
    let tool_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (runtime, _store) =
        build_runtime(test_clock(), allow_all(), provider.clone(), tool_calls).await;
    let handler = Arc::new(CanonicalChainHandler::new(runtime));
    let bridge = build_bridge(server.uri(), None, handler, test_clock(), None);

    let result = bridge
        .handle_inbound(
            "primary",
            &feishu_text_event("oc_demo_chat", "calculate 1 + 1"),
            None,
        )
        .await
        .unwrap();
    match result {
        ImInboundResult::Replied {
            session, segments, ..
        } => {
            assert_eq!(segments, 1);
            assert_eq!(
                bridge.session_entries()[0].session,
                session,
                "会话映射与回合会话同源"
            );
        }
        other => panic!("expected a reply, got {other:?}"),
    }
    assert_eq!(provider.call_count(), 2, "真实对话链跑了完整工具回路");

    let bodies = posted_bodies(&server).await;
    assert!(bodies[0].contains("The result is 2."), "{bodies:?}");
}

/// 6. fail-closed 边界: 未知渠道 / 未映射会话的按钮回调都被拒。
#[tokio::test]
async fn unknown_channels_and_unmapped_conversations_fail_closed() {
    let server = mock_endpoint().await;
    let handler = ScriptedHandler::new("ok");
    let bridge = build_bridge(server.uri(), None, handler, test_clock(), None);

    let error = bridge
        .handle_inbound("ghost", &feishu_text_event("oc_demo_chat", "hi"), None)
        .await
        .unwrap_err();
    assert!(matches!(error, ImBridgeError::UnknownChannel(_)), "{error}");

    let button = serde_json::json!({
        "header": {"event_type": "card.action.trigger", "app_id": "app", "token": "t"},
        "event": {
            "operator": {"open_id": "ou_2"},
            "action": {"tag": "button", "value": {
                "approval_ref": "apr_1",
                "pair_id": "pair-1",
                "round": 1,
                "subject": "tool.calculator",
                "expires_at_ms": 4_000_000_000_000_i64,
                "action": "approve"
            }},
            "context": {"open_chat_id": "oc_unmapped", "open_message_id": "om_9"},
        }
    })
    .to_string();
    let error = bridge
        .handle_inbound("primary", &button, None)
        .await
        .unwrap_err();
    assert!(
        matches!(error, ImBridgeError::UnmappedConversation(_)),
        "{error}"
    );

    // 归一化层也能独立复核按钮载荷形状。
    match parse_inbound_event(ImChannelKind::ImFeishu, &button).unwrap() {
        ImInboundEvent::CardAction(action) => assert_eq!(action.payload.pair_id, "pair-1"),
        other => panic!("expected a card action, got {other:?}"),
    }
}

/// 7. 未映射会话的按钮回调不产生任何出站投递 (mock 端点 0 请求)。
#[tokio::test]
async fn failed_inbound_never_posts_to_the_endpoint() {
    let server = mock_endpoint().await;
    let handler = ScriptedHandler::new("ok");
    let bridge = build_bridge(server.uri(), None, handler, test_clock(), None);
    let _ = bridge
        .handle_inbound("primary", "not json", None)
        .await
        .unwrap_err();
    assert!(posted_bodies(&server).await.is_empty());
}
