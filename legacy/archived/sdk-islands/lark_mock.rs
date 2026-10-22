//! lark 子模块 mock 服务测试 (本地 RecordingServer 模式, 0 真实网络)
//!
//! 每个协议面 ≥3 测试: 成功 / 错误分类 (闭合词表) / 分页或边界。
//! mock HTTP 服务照仓内既有 RecordingServer 模式: 环回 `127.0.0.1:0`
//! TcpListener + 录制请求 + 脚本化响应; 所有请求/响应都在本机闭环。

#![cfg(feature = "lark")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_sdk::lark::{
    self, decrypt_event_payload, encrypt_event_payload, verify_webhook_event_at, ApprovalInstance,
    CalendarEvent, CalendarEventQuery, Department, Document, ErrorClass, EventStatus,
    InstanceStatus, LarkClient, LarkClientImpl, LarkError, Message, Message as LarkMessage,
    MessageType, ReceiveIdType, RetryPolicy, TaskStatus, TransportConfig, UserIdType, UserQuery,
    WebhookEvent, WebhookToken, WebhookVerifyResult, WEBHOOK_TIMESTAMP_SKEW_SECS,
};
use serde_json::{json, Value};

// ============================================================================
// §0 RecordingServer 模式 mock (环回 + 录制 + 脚本化响应)
// ============================================================================

/// 录制到的请求 (脱敏口径: 测试内明文可读, 生产日志不落这些面)。
#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    /// 请求目标 (path + query)。
    target: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl RecordedRequest {
    fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or(&self.target)
    }

    fn query(&self) -> &str {
        self.target.split_once('?').map(|(_, q)| q).unwrap_or("")
    }

    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    }

    fn json_body(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

/// 脚本化响应。
#[derive(Debug, Clone)]
struct MockResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
    delay: Option<Duration>,
}

impl MockResponse {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.to_string(),
            delay: None,
        }
    }

    /// 平台业务信封 (HTTP 200 + `{code, msg, data}`)。
    fn envelope(code: i32, msg: &str, data: Value) -> Self {
        Self::json(200, json!({"code": code, "msg": msg, "data": data}))
    }

    fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
}

/// token 颁发成功响应 (flat 形状)。
fn token_ok(token: &str) -> MockResponse {
    MockResponse::json(
        200,
        json!({"code": 0, "msg": "ok", "tenant_access_token": token, "expire": 7200}),
    )
}

type Handler = Arc<dyn Fn(&RecordedRequest, usize) -> MockResponse + Send + Sync>;

/// 本地 mock 平台 (RecordingServer 模式)。
struct MockPlatform {
    base_url: String,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl MockPlatform {
    async fn start(handler: Handler) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests: Arc<Mutex<Vec<RecordedRequest>>> = Arc::new(Mutex::new(Vec::new()));
        let requests_clone = Arc::clone(&requests);
        tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let requests = Arc::clone(&requests_clone);
                let handler = Arc::clone(&handler);
                tokio::spawn(async move {
                    serve_one(socket, requests, handler).await;
                });
            }
        });
        Self {
            base_url: format!("http://{addr}"),
            requests,
        }
    }

    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }

    fn count_where(&self, f: impl Fn(&RecordedRequest) -> bool) -> usize {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| f(r))
            .count()
    }

    fn token_fetches(&self) -> usize {
        self.count_where(|r| r.path().contains("/auth/v3/tenant_access_token/internal"))
    }
}

async fn serve_one(
    mut socket: tokio::net::TcpStream,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    handler: Handler,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 4096];
    // 读到头部结束
    let header_end = loop {
        let n = socket.read(&mut tmp).await.unwrap_or(0);
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
        if buf.len() > 1 << 20 {
            return;
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let content_length = head
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once(':')?;
            if k.trim().eq_ignore_ascii_case("content-length") {
                v.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);
    // 读完 body
    while buf.len() < header_end + content_length {
        let n = socket.read(&mut tmp).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let body_end = (header_end + content_length).min(buf.len());
    let body = String::from_utf8_lossy(&buf[header_end..body_end]).to_string();

    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| {
            let (k, v) = line.split_once(':')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect();

    let record = RecordedRequest {
        method,
        target,
        headers,
        body,
    };
    let hit_index = {
        let mut guard = requests.lock().unwrap();
        let idx = guard.len();
        guard.push(record.clone());
        idx
    };

    let response = handler(&record, hit_index);
    if let Some(delay) = response.delay {
        tokio::time::sleep(delay).await;
    }
    let mut out = format!("HTTP/1.1 {} OK\r\n", response.status);
    out.push_str("content-type: application/json\r\n");
    for (k, v) in &response.headers {
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    out.push_str(&format!(
        "content-length: {}\r\nconnection: close\r\n\r\n",
        response.body.len()
    ));
    out.push_str(&response.body);
    let _ = socket.write_all(out.as_bytes()).await;
    let _ = socket.flush().await;
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// ============================================================================
// §1 测试脚手架
// ============================================================================

const APP_ID: &str = "cli_a1b2c3d4e5f6";
const APP_SECRET: &str = "abcdef1234567890abcdef1234567890";

fn test_client(base_url: &str, token_cache_path: Option<PathBuf>) -> LarkClientImpl {
    let config = TransportConfig {
        api_base: base_url.to_string(),
        attempt_timeout: Duration::from_secs(5),
        call_deadline: Duration::from_secs(10),
        retry: RetryPolicy {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(5),
            backoff_multiplier: 2,
        },
        token_cache_path,
        user_agent: "apeireth-sdk-lark-mock-test".to_string(),
    };
    let mut client = LarkClientImpl::with_config(config).expect("valid transport config");
    client.set_app_id(APP_ID.to_string()).expect("valid app id");
    client
        .set_app_secret(APP_SECRET.to_string())
        .expect("valid app secret");
    client
}

fn sample_text_message() -> LarkMessage {
    Message::text(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "hello".to_string(),
    )
    .expect("valid message")
}

fn sample_calendar_query() -> CalendarEventQuery {
    let start = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 5, 10, 0, 0).unwrap();
    let end = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 5, 11, 0, 0).unwrap();
    CalendarEventQuery {
        calendar_id: "cal_x".to_string(),
        start_time: start,
        end_time: end,
        page_size: 50,
        page_token: None,
    }
}

fn event_wire(summary: &str, status: Option<&str>) -> Value {
    let mut obj = json!({
        "event_id": format!("evt_{summary}"),
        "summary": summary,
        "start_time": "2026-08-05T10:00:00Z",
        "end_time": "2026-08-05T10:30:00Z",
        "future_field": {"ignored": true}
    });
    match status {
        Some(s) => obj["status"] = json!(s),
        None => {
            obj.as_object_mut().expect("object").remove("status");
        }
    }
    obj
}

/// 大多数面共用的路由: token 端点自动应答, 其它交给测试给的业务分支。
fn handler_with_token(
    token: &'static str,
    business: impl Fn(&RecordedRequest, usize) -> MockResponse + Send + Sync + 'static,
) -> Handler {
    Arc::new(move |req, idx| {
        if req.path().contains("/auth/v3/tenant_access_token/internal") {
            return token_ok(token);
        }
        business(req, idx)
    })
}

// ============================================================================
// §2 auth 面 (token 颁发 / 缓存 / 刷新 / 分类)
// ============================================================================

#[tokio::test]
async fn auth_face_token_fetched_once_and_carried_as_bearer() {
    let server = MockPlatform::start(handler_with_token("t-test-1", |_, _| {
        MockResponse::envelope(0, "ok", json!({"message_id": "om_1"}))
    }))
    .await;
    let client = test_client(&server.base_url, None);

    let msg = sample_text_message();
    let id1 = client.send_message(&msg).await.expect("send 1");
    let id2 = client.send_message(&msg).await.expect("send 2");
    assert_eq!(id1, "om_1");
    assert_eq!(id2, "om_1");

    // 成功: token 只颁发一次 (内存缓存), 两次 API 调用都带 Bearer
    assert_eq!(server.token_fetches(), 1, "token 必须只颁发一次");
    let api_requests: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path() == "/im/v1/messages")
        .collect();
    assert_eq!(api_requests.len(), 2);
    for r in &api_requests {
        let auth = r.header("authorization").expect("auth header");
        assert_eq!(auth, "Bearer t-test-1", "认证头必须是 Bearer token");
    }
    // token 颁发请求体带凭证 (App Secret 不进日志但要进协议体)
    let token_req = server
        .requests()
        .into_iter()
        .find(|r| r.path().contains("/auth/v3/tenant_access_token/internal"))
        .expect("token request");
    assert_eq!(token_req.json_body()["app_id"], APP_ID);
    assert_eq!(token_req.json_body()["app_secret"], APP_SECRET);
}

#[tokio::test]
async fn auth_face_token_cache_persists_and_reuses_across_clients() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache_path: PathBuf = dir.path().join("tenant-token.json");

    let server = MockPlatform::start(handler_with_token("t-cached", |_, _| {
        MockResponse::envelope(0, "ok", json!({"message_id": "om_1"}))
    }))
    .await;

    let client_a = test_client(&server.base_url, Some(cache_path.clone()));
    client_a
        .send_message(&sample_text_message())
        .await
        .expect("send");
    assert_eq!(server.token_fetches(), 1);
    assert!(
        cache_path.exists(),
        "token 缓存必须落盘 (storage_atomic 原子写)"
    );

    // 新客户端 (同缓存路径): 直接复用磁盘缓存, 0 再颁发
    let client_b = test_client(&server.base_url, Some(cache_path.clone()));
    client_b
        .send_message(&sample_text_message())
        .await
        .expect("send");
    assert_eq!(server.token_fetches(), 1, "第二客户端必须复用持久化缓存");
    assert_eq!(
        client_b.tenant_token().map(|t| t.token),
        Some("t-cached".to_string())
    );
}

#[tokio::test]
async fn auth_face_auth_failure_refreshes_token_exactly_once() {
    // 业务分支: 第 1 次 API 调用返认证失败码, 之后成功 → 应自动失效缓存 + 重颁发 + 重试
    let server = MockPlatform::start(handler_with_token("t-refresh", |_, idx| {
        // idx 0 = 第一次 token, idx 1 = 第一次 API (失败), idx 2 = 重颁发, idx 3 = 重试 API
        if idx == 1 {
            MockResponse::envelope(99991663, "invalid access token", json!({}))
        } else {
            MockResponse::envelope(0, "ok", json!({"message_id": "om_after_refresh"}))
        }
    }))
    .await;
    let client = test_client(&server.base_url, None);

    let id = client
        .send_message(&sample_text_message())
        .await
        .expect("send must recover after token refresh");
    assert_eq!(id, "om_after_refresh");
    assert_eq!(server.token_fetches(), 2, "认证失败后恰补发一次 token");
    assert_eq!(
        server.count_where(|r| r.path() == "/im/v1/messages"),
        2,
        "API 恰好重试一次"
    );
}

#[tokio::test]
async fn auth_face_invalid_credentials_are_classified_auth_failed() {
    let server = MockPlatform::start(Arc::new(|req: &RecordedRequest, _| {
        if req.path().contains("/auth/v3/tenant_access_token/internal") {
            return MockResponse::envelope(99991663, "app secret invalid", json!({}));
        }
        MockResponse::envelope(0, "ok", json!({"message_id": "om_never"}))
    }))
    .await;
    let client = test_client(&server.base_url, None);

    match client.send_message(&sample_text_message()).await {
        Err(err) => {
            assert_eq!(err.class(), ErrorClass::AuthFailed);
            assert!(err.is_auth_failure());
        }
        other => panic!("expected AuthFailed, got {other:?}"),
    }
}

// ============================================================================
// §3 message 面
// ============================================================================

#[tokio::test]
async fn message_face_send_success_returns_message_id() {
    let server = MockPlatform::start(handler_with_token("t-m", |_, _| {
        MockResponse::envelope(
            0,
            "ok",
            json!({"message_id": "om_abc123", "chat_id": "oc_x"}),
        )
    }))
    .await;
    let client = test_client(&server.base_url, None);

    let msg = sample_text_message();
    let id = client.send_message(&msg).await.expect("send");
    assert_eq!(id, "om_abc123");

    let req = server
        .requests()
        .into_iter()
        .find(|r| r.path() == "/im/v1/messages")
        .expect("api request");
    assert_eq!(req.method, "POST");
    assert!(req.query().contains("receive_id_type=chat_id"));
    assert_eq!(req.json_body()["receive_id"], "oc_a1b2c3d4e5f6");
    assert_eq!(req.json_body()["msg_type"], "text");
    assert_eq!(req.json_body()["content"], r#"{"text":"hello"}"#);
}

#[tokio::test]
async fn message_face_error_classification_permanent_vs_retryable() {
    // 永久: 业务码 → ApiError (Permanent), 不重试
    let server = MockPlatform::start(handler_with_token("t-m", |_, _| {
        MockResponse::envelope(230001, "receive id not found", json!({}))
    }))
    .await;
    let client = test_client(&server.base_url, None);
    match client.send_message(&sample_text_message()).await {
        Err(LarkError::ApiError { code, .. }) => {
            assert_eq!(code, 230001);
        }
        other => panic!("expected ApiError(230001), got {other:?}"),
    }
    assert_eq!(
        server.count_where(|r| r.path() == "/im/v1/messages"),
        1,
        "永久错误不得重试"
    );

    // 可重试: 5xx → Network (Retryable), 退避后按 max_attempts 重试
    let server = MockPlatform::start(handler_with_token("t-m", |_, _| {
        MockResponse::json(500, json!({"error": "boom"}))
    }))
    .await;
    let client = test_client(&server.base_url, None);
    match client.send_message(&sample_text_message()).await {
        Err(err) => {
            assert_eq!(err.class(), ErrorClass::Retryable);
            assert!(matches!(err, LarkError::Network(_)));
        }
        other => panic!("expected Retryable network error, got {other:?}"),
    }
    assert_eq!(
        server.count_where(|r| r.path() == "/im/v1/messages"),
        3,
        "可重试错误应打满 max_attempts=3"
    );
}

#[tokio::test]
async fn message_face_rate_limit_backoff_retries_and_respects_retry_after() {
    // 第 1 次 429 (Retry-After: 0) → 退避后重试成功
    let server = MockPlatform::start(handler_with_token("t-m", |_, idx| {
        if idx <= 1 {
            MockResponse::json(429, json!({"error": "rate limited"}))
                .with_header("retry-after", "0")
        } else {
            MockResponse::envelope(0, "ok", json!({"message_id": "om_after_429"}))
        }
    }))
    .await;
    let client = test_client(&server.base_url, None);
    let id = client
        .send_message(&sample_text_message())
        .await
        .expect("send must recover after rate limit");
    assert_eq!(id, "om_after_429");
    assert_eq!(server.count_where(|r| r.path() == "/im/v1/messages"), 2);
}

#[tokio::test]
async fn message_face_boundaries_reject_before_any_network() {
    let server = MockPlatform::start(handler_with_token("t-m", |_, _| {
        MockResponse::envelope(0, "ok", json!({"message_id": "om_never"}))
    }))
    .await;
    let client = test_client(&server.base_url, None);

    // 文本超上限 → 本地拒 (0 网络请求)
    let huge = Message::text(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "x".repeat(lark::MAX_MESSAGE_TEXT_BYTES + 1),
    )
    .expect("construct");
    assert!(matches!(
        client.send_message(&huge).await,
        Err(LarkError::Other(_))
    ));

    // 缺 message_id 的成功响应 → 协议体畸形 (Permanent)
    let server2 = MockPlatform::start(handler_with_token("t-m", |_, _| {
        MockResponse::envelope(0, "ok", json!({"message_id": "   "}))
    }))
    .await;
    let client2 = test_client(&server2.base_url, None);
    match client2.send_message(&sample_text_message()).await {
        Err(err) => {
            assert_eq!(err.class(), ErrorClass::Permanent);
            assert!(matches!(err, LarkError::Other(_)));
        }
        other => panic!("expected permanent malformed-response error, got {other:?}"),
    }

    assert_eq!(
        server.requests().len(),
        0,
        "本地边界校验不得发请求 (超限文本直接拒)"
    );
    assert_eq!(
        server2.count_where(|r| r.path() == "/im/v1/messages"),
        1,
        "缺 message_id 的那次调用恰好打一次网络"
    );
}

// ============================================================================
// §4 calendar 面 (分页)
// ============================================================================

#[tokio::test]
async fn calendar_face_list_success_single_page() {
    let server = MockPlatform::start(handler_with_token("t-c", |req, _| {
        assert!(req.path() == "/calendar/v4/calendars/cal_x/events");
        assert!(req.query().contains("start_time=2026-08-05T10%3A00%3A00Z"));
        assert!(req.query().contains("page_size=50"));
        MockResponse::envelope(
            0,
            "ok",
            json!({
                "items": [event_wire("standup", Some("confirmed")), event_wire("review", None)],
                "has_more": false
            }),
        )
    }))
    .await;
    let client = test_client(&server.base_url, None);

    let events = client
        .list_calendar_events(&sample_calendar_query())
        .await
        .expect("list");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].summary, "standup");
    assert_eq!(events[0].status, EventStatus::Confirmed);
    assert_eq!(
        events[1].status,
        EventStatus::Tentative,
        "缺省状态 = tentative"
    );
    assert_eq!(server.count_where(|r| r.path().contains("/events")), 1);
}

#[tokio::test]
async fn calendar_face_pagination_merges_pages_and_follows_page_token() {
    let server = MockPlatform::start(handler_with_token("t-c", |req, idx| {
        if req.query().contains("page_token=p2") {
            MockResponse::envelope(
                0,
                "ok",
                json!({"items": [event_wire("c", None)], "has_more": false}),
            )
        } else {
            let _ = idx;
            MockResponse::envelope(
                0,
                "ok",
                json!({
                    "items": [event_wire("a", None), event_wire("b", None)],
                    "has_more": true,
                    "page_token": "p2"
                }),
            )
        }
    }))
    .await;
    let client = test_client(&server.base_url, None);

    let events = client
        .list_calendar_events(&sample_calendar_query())
        .await
        .expect("list");
    let summaries: Vec<_> = events.iter().map(|e| e.summary.as_str()).collect();
    assert_eq!(summaries, vec!["a", "b", "c"], "两页必须按序合并");
    assert_eq!(
        server.count_where(|r| r.path().contains("/events")),
        2,
        "分页必须跟随 page_token"
    );
}

#[tokio::test]
async fn calendar_face_pagination_loop_guard_and_unknown_status() {
    // 服务端 page_token 不前进 → 客户端必须拒绝 (边界守卫), 不无限拉页
    let server = MockPlatform::start(handler_with_token("t-c", |_, _| {
        MockResponse::envelope(
            0,
            "ok",
            json!({"items": [], "has_more": true, "page_token": "same"}),
        )
    }))
    .await;
    let client = test_client(&server.base_url, None);
    match client.list_calendar_events(&sample_calendar_query()).await {
        Err(LarkError::Other(msg)) => {
            assert!(msg.contains("pagination"), "必须报分页异常: {msg}");
        }
        other => panic!("expected pagination error, got {other:?}"),
    }

    // 未知状态取值 → 永久错误 (闭合枚举), 首页即拒
    let server2 = MockPlatform::start(handler_with_token("t-c", |_, _| {
        MockResponse::envelope(
            0,
            "ok",
            json!({"items": [event_wire("x", Some("exploded"))], "has_more": false}),
        )
    }))
    .await;
    let client2 = test_client(&server2.base_url, None);
    match client2.list_calendar_events(&sample_calendar_query()).await {
        Err(err) => {
            assert_eq!(err.class(), ErrorClass::Permanent);
            assert!(matches!(err, LarkError::Other(_)));
        }
        other => panic!("expected permanent error, got {other:?}"),
    }
}

// ============================================================================
// §5 contact 面
// ============================================================================

#[tokio::test]
async fn contact_face_get_user_success_maps_fields() {
    let server = MockPlatform::start(handler_with_token("t-u", |req, _| {
        assert_eq!(req.path(), "/contact/v3/users/ou_user1234567890abcdef");
        assert!(req.query().contains("user_id_type=open_id"));
        MockResponse::envelope(
            0,
            "ok",
            json!({
                "user": {
                    "open_id": "ou_user1234567890abcdef",
                    "name": "Alice",
                    "email": "alice@example.com",
                    "mobile": "+8613800138000",
                    "is_activated": true,
                    "future_field": 7
                }
            }),
        )
    }))
    .await;
    let client = test_client(&server.base_url, None);

    let query = UserQuery::new("ou_user1234567890abcdef".to_string(), UserIdType::OpenId)
        .expect("valid query");
    let user = client.get_user(&query).await.expect("get_user");
    assert_eq!(user.name, "Alice");
    assert_eq!(user.email.as_deref(), Some("alice@example.com"));
    assert_eq!(user.mobile.as_deref(), Some("+8613800138000"));
    assert!(user.is_activated);
}

#[tokio::test]
async fn contact_face_error_classification_business_vs_http() {
    // 业务码 → ApiError (Permanent)
    let server = MockPlatform::start(handler_with_token("t-u", |_, _| {
        MockResponse::envelope(230002, "user not found", json!({}))
    }))
    .await;
    let client = test_client(&server.base_url, None);
    let query =
        UserQuery::new("ou_user1234567890abcdef".to_string(), UserIdType::OpenId).expect("valid");
    match client.get_user(&query).await {
        Err(LarkError::ApiError { code, msg }) => {
            assert_eq!(code, 230002);
            assert_eq!(msg, "user not found");
        }
        other => panic!("expected ApiError, got {other:?}"),
    }

    // HTTP 404 → 永久错误, 不重试
    let server2 = MockPlatform::start(handler_with_token("t-u", |_, _| {
        MockResponse::json(404, json!({"error": "nope"}))
    }))
    .await;
    let client2 = test_client(&server2.base_url, None);
    match client2.get_user(&query).await {
        Err(err) => {
            assert_eq!(err.class(), ErrorClass::Permanent);
            assert_eq!(
                server2.count_where(|r| r.path().contains("/contact/v3/users")),
                1,
                "永久错误不得重试"
            );
        }
        other => panic!("expected permanent error, got {other:?}"),
    }
}

#[tokio::test]
async fn contact_face_boundary_response_field_validation() {
    // 响应里的 mobile 非 E.164 → K-1 #6 拒 (映射期字段校验)
    let server = MockPlatform::start(handler_with_token("t-u", |_, _| {
        MockResponse::envelope(
            0,
            "ok",
            json!({"user": {"open_id": "ou_user1234567890abcdef", "name": "Alice", "mobile": "13800138000"}}),
        )
    }))
    .await;
    let client = test_client(&server.base_url, None);
    let query =
        UserQuery::new("ou_user1234567890abcdef".to_string(), UserIdType::OpenId).expect("valid");
    match client.get_user(&query).await {
        Err(LarkError::MobileInvalid(_)) => {}
        other => panic!("expected MobileInvalid, got {other:?}"),
    }

    // department_id 为空 → 本地拒 (0 网络)
    let before = server.requests().len();
    assert!(matches!(
        client.get_department("").await,
        Err(LarkError::Other(_))
    ));
    assert_eq!(
        server.requests().len(),
        before,
        "空 department_id 不得发请求"
    );
}

#[tokio::test]
async fn contact_face_get_department_success() {
    let server = MockPlatform::start(handler_with_token("t-d", |req, _| {
        assert_eq!(req.path(), "/contact/v3/departments/od_dept123");
        MockResponse::envelope(
            0,
            "ok",
            json!({
                "department": {
                    "open_department_id": "od_dept123",
                    "name": "工程部",
                    "leader_open_ids": ["ou_leader1234567890abcdef"],
                    "member_count": 12,
                    "status": "active"
                }
            }),
        )
    }))
    .await;
    let client = test_client(&server.base_url, None);
    let dept: Department = client
        .get_department("od_dept123")
        .await
        .expect("get_department");
    assert_eq!(dept.name, "工程部");
    assert_eq!(dept.member_count, Some(12));
    assert_eq!(dept.leader_open_ids.len(), 1);
}

// ============================================================================
// §6 doc 面
// ============================================================================

#[tokio::test]
async fn doc_face_create_doc_success_maps_document() {
    let server = MockPlatform::start(handler_with_token("t-doc", |req, _| {
        assert_eq!(req.path(), "/docx/v1/documents");
        assert_eq!(req.json_body()["title"], "项目计划");
        assert_eq!(req.json_body()["folder_token"], "fld_1");
        MockResponse::envelope(
            0,
            "ok",
            json!({"document": {"document_id": "doxcnabc123", "title": "项目计划", "future": 1}}),
        )
    }))
    .await;
    let client = test_client(&server.base_url, None);

    let template =
        Document::new_docx("项目计划".to_string(), Some("fld_1".to_string())).expect("doc");
    let doc = client.create_doc(&template).await.expect("create_doc");
    assert_eq!(doc.document_id.as_deref(), Some("doxcnabc123"));
    assert_eq!(doc.token.as_deref(), Some("doxcnabc123"));
    assert_eq!(doc.doc_type, lark::DocumentType::Doc);
}

#[tokio::test]
async fn doc_face_create_sheet_success_and_error_classification() {
    // 成功: spreadsheet token / url 映射
    let server = MockPlatform::start(handler_with_token("t-doc", |req, _| {
        assert_eq!(req.path(), "/sheets/v3/spreadsheets");
        MockResponse::envelope(
            0,
            "ok",
            json!({"spreadsheet": {"spreadsheet_token": "shtcnabc", "url": "https://docs.example.test/s/shtcnabc"}}),
        )
    }))
    .await;
    let client = test_client(&server.base_url, None);
    let sheet = Document::new_sheet("预算表".to_string(), None).expect("sheet");
    let created = client.create_sheet(&sheet).await.expect("create_sheet");
    assert_eq!(created.document_id.as_deref(), Some("shtcnabc"));
    assert_eq!(
        created.url.as_deref(),
        Some("https://docs.example.test/s/shtcnabc")
    );

    // 业务码 → 永久错误
    let server2 = MockPlatform::start(handler_with_token("t-doc", |_, _| {
        MockResponse::envelope(230003, "folder not found", json!({}))
    }))
    .await;
    let client2 = test_client(&server2.base_url, None);
    match client2.create_doc(&sheet).await {
        Err(err) => {
            // sheet 类型发 create_doc 会被本地类型守卫先拒 — 换 docx 测业务码
            assert!(matches!(err, LarkError::Other(_)));
        }
        other => panic!("expected type mismatch error, got {other:?}"),
    }
    let doc = Document::new_docx("title".to_string(), None).expect("doc");
    match client2.create_doc(&doc).await {
        Err(LarkError::ApiError { code, .. }) => assert_eq!(code, 230003),
        other => panic!("expected ApiError, got {other:?}"),
    }
}

#[tokio::test]
async fn doc_face_boundary_type_mismatch_and_malformed_response() {
    let server = MockPlatform::start(handler_with_token("t-doc", |_, _| {
        MockResponse::envelope(0, "ok", json!({"document": {"document_id": "  "}}))
    }))
    .await;
    let client = test_client(&server.base_url, None);

    // 端点/类型不匹配 → 本地拒 (0 网络)
    let sheet = Document::new_sheet("budget".to_string(), None).expect("sheet");
    let before = server.requests().len();
    assert!(matches!(
        client.create_doc(&sheet).await,
        Err(LarkError::Other(_))
    ));
    assert_eq!(server.requests().len(), before, "类型守卫不得发请求");

    // 空 document_id → 协议体畸形 (Permanent)
    let doc = Document::new_docx("title".to_string(), None).expect("doc");
    match client.create_doc(&doc).await {
        Err(err) => {
            assert_eq!(err.class(), ErrorClass::Permanent);
        }
        other => panic!("expected permanent malformed error, got {other:?}"),
    }
}

// ============================================================================
// §7 approval 面
// ============================================================================

#[tokio::test]
async fn approval_face_get_instance_success_maps_tasks_and_form() {
    let server = MockPlatform::start(handler_with_token("t-a", |req, _| {
        assert_eq!(req.path(), "/approval/v4/instances/inst_001");
        MockResponse::envelope(
            0,
            "ok",
            json!({
                "instance": {
                    "instance_id": "inst_001",
                    "approval_code": "approval_code_xxx",
                    "status": "approved",
                    "user_open_id": "ou_user1234567890abcdef",
                    "form": [{"id": "reason", "type": "textarea", "value": "出差"}],
                    "tasks": [{
                        "task_id": "task_1",
                        "instance_id": "inst_001",
                        "approver_open_id": "ou_approver1234567890abcdef",
                        "status": "approved",
                        "action_time": "2026-08-05T11:00:00Z"
                    }],
                    "start_time": "2026-08-05T10:00:00Z",
                    "end_time": "2026-08-05T12:00:00Z",
                    "future_field": true
                }
            }),
        )
    }))
    .await;
    let client = test_client(&server.base_url, None);

    let inst: ApprovalInstance = client
        .get_approval_instance("inst_001")
        .await
        .expect("get_approval_instance");
    assert_eq!(inst.status, InstanceStatus::Approved);
    assert_eq!(inst.form.len(), 1);
    assert_eq!(inst.tasks.len(), 1);
    assert_eq!(inst.tasks[0].status, TaskStatus::Approved);
}

#[tokio::test]
async fn approval_face_error_classification() {
    // 业务码 → 永久错误 (保留 code)
    let server = MockPlatform::start(handler_with_token("t-a", |_, _| {
        MockResponse::envelope(230004, "instance not found", json!({}))
    }))
    .await;
    let client = test_client(&server.base_url, None);
    match client.get_approval_instance("inst_x").await {
        Err(LarkError::ApiError { code, .. }) => assert_eq!(code, 230004),
        other => panic!("expected ApiError, got {other:?}"),
    }

    // 未知状态取值 → 永久错误 (闭合枚举)
    let server2 = MockPlatform::start(handler_with_token("t-a", |_, _| {
        MockResponse::envelope(
            0,
            "ok",
            json!({"instance": {"approval_code": "c", "status": "exploded", "user_open_id": "ou_user1234567890abcdef"}}),
        )
    }))
    .await;
    let client2 = test_client(&server2.base_url, None);
    match client2.get_approval_instance("inst_x").await {
        Err(err) => assert_eq!(err.class(), ErrorClass::Permanent),
        other => panic!("expected permanent error, got {other:?}"),
    }
}

#[tokio::test]
async fn approval_face_boundary_rejects_empty_instance_id_without_network() {
    let server = MockPlatform::start(handler_with_token("t-a", |_, _| {
        MockResponse::envelope(0, "ok", json!({}))
    }))
    .await;
    let client = test_client(&server.base_url, None);
    assert!(matches!(
        client.get_approval_instance("   ").await,
        Err(LarkError::Other(_))
    ));
    assert_eq!(server.requests().len(), 0, "空 instance_id 不得发请求");
}

// ============================================================================
// §8 webhook 面 (本地校验 + AES 解密; 0 HTTP —— 该面是入站校验面)
// ============================================================================

#[tokio::test]
async fn webhook_face_challenge_round_trip_via_client() {
    let server = MockPlatform::start(handler_with_token("t-w", |_, _| {
        MockResponse::envelope(0, "ok", json!({}))
    }))
    .await;
    let client = test_client(&server.base_url, None);

    let wh = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string()).expect("wh");
    let event = WebhookEvent::from_raw_json(
        r#"{"type":"url_verification","challenge":"challenge-42","token":"token_xxx"}"#,
    )
    .expect("parse");
    match client.verify_webhook(&event, &wh).await.expect("verify") {
        WebhookVerifyResult::Challenge(c) => assert_eq!(c, "challenge-42"),
        other => panic!("expected Challenge, got {other:?}"),
    }
    assert_eq!(server.requests().len(), 0, "webhook 校验是本地面, 0 HTTP");
}

#[tokio::test]
async fn webhook_face_encrypted_callback_decrypts_and_accepts() {
    let inner = r#"{"type":"event_callback","token":"token_xxx","ts":1700000000,"event":{"id":"evt_9","type":"im.message.receive_v1"}}"#;
    let blob = encrypt_event_payload(inner, "encrypt_key_xxx", [11u8; 16]).expect("encrypt");
    let event = WebhookEvent::from_raw_json(&format!(r#"{{"encrypt":"{blob}"}}"#)).expect("parse");
    let wh = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string()).expect("wh");

    match verify_webhook_event_at(&event, &wh, 1_700_000_010, WEBHOOK_TIMESTAMP_SKEW_SECS)
        .expect("verify")
    {
        WebhookVerifyResult::Accepted { event } => {
            assert_eq!(event.get("id"), Some(&json!("evt_9")));
        }
        other => panic!("expected Accepted, got {other:?}"),
    }
    // 解密单测面: 直接解密也拿到同一事件 JSON
    let plain = decrypt_event_payload(&blob, "encrypt_key_xxx").expect("decrypt");
    assert_eq!(plain, inner);
}

#[tokio::test]
async fn webhook_face_error_classification_and_boundaries() {
    let wh = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string()).expect("wh");

    // token 不匹配 → 永久错误 (且 0 回显共享秘密)
    let bad = WebhookEvent::from_raw_json(
        r#"{"type":"url_verification","challenge":"c","token":"wrong"}"#,
    )
    .expect("parse");
    match verify_webhook_event_at(&bad, &wh, 1_700_000_000, WEBHOOK_TIMESTAMP_SKEW_SECS) {
        Err(LarkError::Other(msg)) => {
            assert!(msg.contains("mismatch"));
            assert!(!msg.contains("token_xxx"));
            assert!(!msg.contains("wrong"));
        }
        other => panic!("expected mismatch error, got {other:?}"),
    }

    // 重放窗口外 → 永久错误
    let stale = WebhookEvent::from_raw_json(
        r#"{"type":"event_callback","token":"token_xxx","ts":1700000000,"event":{"id":"e"}}"#,
    )
    .expect("parse");
    match verify_webhook_event_at(
        &stale,
        &wh,
        1_700_000_000 + 10_000,
        WEBHOOK_TIMESTAMP_SKEW_SECS,
    ) {
        Err(err) => assert_eq!(err.class(), ErrorClass::Permanent),
        other => panic!("expected permanent replay error, got {other:?}"),
    }

    // 未知信封形状 → 显式 Unsupported (永久分类)
    match WebhookEvent::from_raw_json(r#"{"mystery":true}"#) {
        Err(LarkError::Unsupported("webhook_event_type")) => {}
        other => panic!("expected Unsupported, got {other:?}"),
    }

    // 换加密密钥 → 解密失败 (永久错误)
    let blob = encrypt_event_payload("{}", "another_encrypt_key", [1u8; 16]).expect("encrypt");
    let encrypted = WebhookEvent::new_encrypted(blob);
    assert!(matches!(
        verify_webhook_event_at(&encrypted, &wh, 1_700_000_000, WEBHOOK_TIMESTAMP_SKEW_SECS),
        Err(LarkError::Other(_))
    ));
}

// ============================================================================
// §9 传输层边界 (超时预算 + 限流码闭合映射)
// ============================================================================

#[tokio::test]
async fn transport_face_timeout_is_classified_retryable() {
    // 响应延迟 > attempt_timeout → Network (Retryable); 打满 max_attempts
    let server = MockPlatform::start(handler_with_token("t-t", |_, _| {
        MockResponse::envelope(0, "ok", json!({"message_id": "om_slow"}))
            .with_delay(Duration::from_millis(300))
    }))
    .await;

    let config = TransportConfig {
        api_base: server.base_url.clone(),
        attempt_timeout: Duration::from_millis(40),
        call_deadline: Duration::from_secs(5),
        retry: RetryPolicy {
            max_attempts: 2,
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(2),
            backoff_multiplier: 2,
        },
        token_cache_path: None,
        user_agent: "apeireth-sdk-lark-mock-test".to_string(),
    };
    let mut client = LarkClientImpl::with_config(config).expect("config");
    client.set_app_id(APP_ID.to_string()).expect("id");
    client
        .set_app_secret(APP_SECRET.to_string())
        .expect("secret");

    // token 端点也要够快 —— 延迟只加在业务分支 (idx>0)
    match client.send_message(&sample_text_message()).await {
        Err(err) => {
            assert_eq!(err.class(), ErrorClass::Retryable);
            assert!(matches!(err, LarkError::Network(_)));
        }
        other => panic!("expected retryable timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn transport_face_rate_limit_platform_code_maps_to_retryable() {
    // 平台限流业务码 → RateLimited (Retryable), 退避后重试
    let server = MockPlatform::start(handler_with_token("t-t", |_, idx| {
        if idx <= 1 {
            MockResponse::envelope(99991400, "rate limit exceeded, retry after 0 s", json!({}))
        } else {
            MockResponse::envelope(0, "ok", json!({"message_id": "om_ok"}))
        }
    }))
    .await;
    let client = test_client(&server.base_url, None);
    let id = client
        .send_message(&sample_text_message())
        .await
        .expect("recover after platform rate limit");
    assert_eq!(id, "om_ok");
    assert_eq!(server.count_where(|r| r.path() == "/im/v1/messages"), 2);
}

// ============================================================================
// §10 领域类型互通 (编译期面: mock 测试与领域类型同源)
// ============================================================================

#[test]
fn domain_types_round_trip_through_wire_shapes() {
    // 消息: 6 类型都能过 validate_for_send (形状校验)
    for (msg, expected) in [
        (sample_text_message(), MessageType::Text),
        (
            Message::post(
                "oc_a1b2c3d4e5f6".to_string(),
                ReceiveIdType::ChatId,
                lark::PostContent::new_zh_cn("通知", vec![]),
            )
            .expect("post"),
            MessageType::Post,
        ),
        (
            Message::image(
                "oc_a1b2c3d4e5f6".to_string(),
                ReceiveIdType::ChatId,
                "img_1".to_string(),
            )
            .expect("image"),
            MessageType::Image,
        ),
        (
            Message::file(
                "oc_a1b2c3d4e5f6".to_string(),
                ReceiveIdType::ChatId,
                "file_1".to_string(),
            )
            .expect("file"),
            MessageType::File,
        ),
        (
            Message::card(
                "oc_a1b2c3d4e5f6".to_string(),
                ReceiveIdType::ChatId,
                lark::CardContent::plain("t", "b"),
            )
            .expect("card"),
            MessageType::Card,
        ),
    ] {
        assert_eq!(msg.msg_type, expected);
        msg.validate_for_send().expect("shape must be valid");
    }

    // 事件时间字段: 领域类型自带严格区间校验
    let start = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 5, 10, 0, 0).unwrap();
    let end = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 5, 11, 0, 0).unwrap();
    let event = CalendarEvent::new("cal_x", "会", start, end).expect("event");
    assert!(event.validate().is_ok());

    let mut map = HashMap::new();
    map.insert("k".to_string(), json!("v"));
    let callback = WebhookEvent::new_event_callback(
        "cli_a1b2c3d4e5f6".to_string(),
        "token_xxx".to_string(),
        1_700_000_000,
        map,
    );
    assert_eq!(callback.timestamp_secs, 1_700_000_000);
}
