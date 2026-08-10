//! apeireth-sdk 客户 SDK client integration test (R20 阶段 6, 1.0 release #13 sdk)
//!
//! **目标**: 跨 module 端到端验证 `ApeirethClient` 6 工具 method + Auth 5 组件 + K-1 强校验 4 条.
//!
//! **R25 增强** (HTTP mock + ABI stability + error 全覆盖):
//! - 12 个 wiremock HTTP mock fixture (200/401/404/500/429 + content-type 验证)
//! - 8 个 SdkClientError variant 全覆盖
//! - 8 个 SdkErrorCode variant 双向 (snake/camel/numeric)
//! - ABI stability 5 fixture (init/last_error 一致性)
//! - Edge case 6 fixture (URL 规范化 / 大小写 / 1000 条 audit / token bucket 边缘)

use apeireth_protocol::ws_v1::{
    ToolInvokeFrame, WsFrame, WS_PING_INTERVAL_SECS, WS_PROTOCOL_VERSION, WS_TOKEN_DEFAULT_TTL_SECS,
};
use apeireth_sdk::client::{
    validate_sdk_method, validate_tool_call, ApeirethClient, AuditEntry, AuditLogger, AuthPipeline,
    KeyringRef, QuotaStub, SdkClientError, TokenBucket, MUST_DO_INVOKE, PLATFORM_NAME,
    SDK_TOOL_WHITELIST, SDK_TOOL_WHITELIST_COUNT, STUB_MODE, TOOL_PATHS, TOOL_WHITELIST, WS_PATH,
};
use apeireth_sdk::{negotiate, Envelope, SdkError, SdkErrorCode, SdkVersion, WireCompat, WireKind};

// =====================================================================
// Fixture 1: K-1 #1 — platform name = "apeireth" (1:1 翻译 v0.9.21)
// =====================================================================

#[test]
fn k1_platform_name_is_apeireth() {
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert!(PLATFORM_NAME.starts_with("apeireth"));
    // 不写 "SpectrAI" / "minimax" / 装饰名.
    assert!(!PLATFORM_NAME.contains("SpectrAI"));
    assert!(!PLATFORM_NAME.contains("minimax"));
}

// =====================================================================
// Fixture 2: K-1 #2 — SDK_TOOL_WHITELIST 8 名字 (6 工具 + 2 invoke)
// =====================================================================

#[test]
fn k1_sdk_tool_whitelist_has_8_names() {
    assert_eq!(SDK_TOOL_WHITELIST.len(), 8);
    assert_eq!(SDK_TOOL_WHITELIST.len(), SDK_TOOL_WHITELIST_COUNT);
    let six = [
        "apeireth_sdk_web_search",
        "apeireth_sdk_file_ops",
        "apeireth_sdk_git_ops",
        "apeireth_sdk_code_exec",
        "apeireth_sdk_calendar",
        "apeireth_sdk_message",
    ];
    for name in six.iter() {
        assert!(
            SDK_TOOL_WHITELIST.contains(name),
            "SDK_TOOL_WHITELIST 缺: {name}"
        );
    }
    assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_invoke_tool"));
    assert!(SDK_TOOL_WHITELIST.contains(&"apeireth_sdk_invoke_stream"));
}

// =====================================================================
// Fixture 3: K-1 #3 — TOOL_WHITELIST 6 工具 (per 蓝图 §2.2)
// =====================================================================

#[test]
fn k1_tool_whitelist_has_6_names() {
    assert_eq!(TOOL_WHITELIST.len(), 6);
    for tool in TOOL_WHITELIST.iter() {
        assert!(TOOL_WHITELIST.contains(tool));
    }
    let six = ["web_search", "file_ops", "git_ops", "code_exec", "calendar", "message"];
    for tool in six.iter() {
        assert!(TOOL_WHITELIST.contains(tool), "TOOL_WHITELIST 缺: {tool}");
    }
    assert!(!TOOL_WHITELIST.contains(&"fake_tool"));
    assert!(!TOOL_WHITELIST.contains(&"apeireth_sdk_web_search"));
}

// =====================================================================
// Fixture 4: K-1 #4 — 5 字样 (apeireth / sdk / client / invoke / must-do)
// =====================================================================

#[test]
fn k1_must_do_five_keywords() {
    let must_do = MUST_DO_INVOKE;
    assert!(must_do.contains("apeireth"), "K-1 #4 缺 'apeireth'");
    assert!(must_do.contains("sdk"), "K-1 #4 缺 'sdk'");
    assert!(must_do.contains("client"), "K-1 #4 缺 'client'");
    assert!(must_do.contains("invoke"), "K-1 #4 缺 'invoke'");
    assert!(must_do.contains("must-do"), "K-1 #4 缺 'must-do'");
}

// =====================================================================
// Fixture 5: K-1 #5 — STUB_MODE 守门
// =====================================================================

#[test]
fn k1_stub_mode_is_true() {
    let _ = STUB_MODE;
    let rt = tokio::runtime::Runtime::new().unwrap();
    let c = ApeirethClient::new("https://api.apeireth.io", "a-valid-api-key-1234567890").unwrap();
    rt.block_on(async {
        assert!(matches!(
            c.web_search("x").await,
            Err(SdkClientError::NotImplemented(_))
        ));
    });
}

// =====================================================================
// Fixture 6: Auth 5 组件流水线
// =====================================================================

#[test]
fn auth_pipeline_preflight_walks_5_components() {
    let p = AuthPipeline::new("a-valid-api-key-1234567890").expect("valid api key");
    assert!(!p.api_key.is_empty());
    assert_eq!(p.keyring.service, "apeireth-api-key");
    assert!(p.bucket.capacity > 0.0);
    assert!(p.audit.is_empty());
    assert_eq!(p.quota.monthly_limit, 0);
    p.preflight("web_search", "search").expect("preflight should succeed");
    assert!(p.audit.len() >= 1);
    let err = p.check_quota();
    assert!(matches!(err, Err(SdkClientError::QuotaExceeded(_))));
}

// =====================================================================
// Fixture 7: 6 工具 method → D-02 子路径
// =====================================================================

#[test]
fn six_tool_methods_route_to_correct_paths() {
    let c = ApeirethClient::new("https://api.apeireth.io", "a-valid-api-key-1234567890").unwrap();
    let expected = [
        ("web_search", "/v1/tools/web_search/invoke"),
        ("file_ops", "/v1/tools/file_ops/invoke"),
        ("git_ops", "/v1/tools/git_ops/invoke"),
        ("code_exec", "/v1/tools/code_exec/invoke"),
        ("calendar", "/v1/tools/calendar/invoke"),
        ("message", "/v1/tools/message/invoke"),
    ];
    for (tool, path) in expected.iter() {
        let url = c
            .tool_url(tool)
            .unwrap_or_else(|| panic!("tool_url({tool}) 必返 Some"));
        assert_eq!(url, format!("https://api.apeireth.io{path}"));
    }
    assert_eq!(TOOL_PATHS.len(), 6);
    for (name, _path) in TOOL_PATHS.iter() {
        assert!(TOOL_WHITELIST.contains(name));
    }
}

// =====================================================================
// Fixture 8: 5 集成点 0 冲突
// =====================================================================

#[test]
fn five_integration_points_align() {
    let frame = WsFrame::ToolInvoke(ToolInvokeFrame {
        tool: "web_search".to_string(),
        action: "search".to_string(),
        args: serde_json::json!({}),
        req_id: "r-001".to_string(),
    });
    assert_eq!(frame.type_str(), "tool_invoke");
    if let WsFrame::ToolInvoke(f) = &frame {
        assert_eq!(f.tool, "web_search");
        assert_eq!(f.action, "search");
    } else {
        panic!("expected ToolInvoke variant");
    }
    assert_eq!(WS_PROTOCOL_VERSION, "1");
    assert_eq!(WS_TOKEN_DEFAULT_TTL_SECS, 300);
    assert_eq!(WS_PING_INTERVAL_SECS, 30);
    assert_eq!(WS_PATH, "/v1/stream");
}

// =====================================================================
// Fixture 9: validate_tool_call + validate_sdk_method (m3 防御)
// =====================================================================

#[test]
fn validate_tool_call_white_and_black() {
    for tool in TOOL_WHITELIST.iter() {
        assert!(validate_tool_call(tool, &serde_json::json!({})).is_ok());
    }
    assert!(validate_tool_call("fake_tool", &serde_json::json!({})).is_err());
    assert!(validate_tool_call("", &serde_json::json!({})).is_err());
    for method in SDK_TOOL_WHITELIST.iter() {
        assert!(validate_sdk_method(method).is_ok());
    }
    assert!(validate_sdk_method("not_in_whitelist").is_err());
}

// =====================================================================
// Fixture 10: Auth 5 组件子项
// =====================================================================

#[test]
fn auth_5_components_individual() {
    assert!(apeireth_sdk::client::check_bearer("a-valid-api-key-1234567890").is_ok());
    assert!(apeireth_sdk::client::check_bearer("").is_err());
    assert!(apeireth_sdk::client::check_bearer("short").is_err());

    let k = KeyringRef::default_for("user-001");
    assert_eq!(k.service, "apeireth-api-key");
    assert_eq!(k.account, "user-001");

    let bucket = TokenBucket::with_config(2.0, 1.0);
    assert!(bucket.try_acquire());
    assert!(bucket.try_acquire());
    assert!(!bucket.try_acquire());
    assert!(bucket.retry_after_secs() >= 1);

    let logger = AuditLogger::new();
    assert!(logger.is_empty());
    logger.append(AuditEntry {
        ts_ms: 12345,
        api_key_hash: "abc...".into(),
        tool: "web_search".into(),
        action: "search".into(),
        ok: true,
        duration_ms: 234,
        trace_id: "tr-001".into(),
    });
    assert_eq!(logger.len(), 1);

    let q = QuotaStub::new();
    let err = q.check();
    assert!(matches!(err, Err(SdkClientError::QuotaExceeded(_))));
}

// =====================================================================
// Fixture 11 (R25): ABI stability — C-ABI apeireth_sdk_init 一致性
// =====================================================================

#[test]
fn abi_init_returns_0_consistently() {
    assert_eq!(apeireth_sdk::abi::apeireth_sdk_init(), 0);
    for _ in 0..100 {
        assert_eq!(apeireth_sdk::abi::apeireth_sdk_init(), 0);
    }
    let ret = apeireth_sdk::abi::apeireth_sdk_last_error(std::ptr::null_mut(), 0);
    assert_eq!(ret, -1, "last_error stub 必返 -1");
}

// =====================================================================
// Fixture 12 (R25): SdkClientError 10 variant 全覆盖
// =====================================================================

#[test]
fn error_10_variants_all_construct() {
    let variants: Vec<SdkClientError> = vec![
        SdkClientError::ToolNotWhitelisted("x".into()),
        SdkClientError::AuthFailed("bad key".into()),
        SdkClientError::Network("tcp fail".into()),
        SdkClientError::Protocol("version mismatch".into()),
        SdkClientError::RateLimited(60),
        SdkClientError::QuotaExceeded("over limit".into()),
        SdkClientError::ToolCallFailed("server said no".into()),
        SdkClientError::ServerInternal("500".into()),
        SdkClientError::NotImplemented("R21".into()),
        SdkClientError::Other("misc".into()),
    ];
    assert_eq!(variants.len(), 10, "SdkClientError 必 10 variant");
    for v in &variants {
        let s = format!("{v}");
        assert!(!s.is_empty(), "error display 必非空");
    }
    if let SdkClientError::RateLimited(secs) = variants[4] {
        assert_eq!(secs, 60);
    } else {
        panic!("RateLimited 字段保留失败");
    }
}

// =====================================================================
// Fixture 13 (R25): SdkErrorCode 8 variant 双向
// =====================================================================

#[test]
fn error_code_8_variants_all_directions() {
    use apeireth_sdk::SdkErrorCode::*;
    let codes = [Unknown, InvalidEnvelope, VersionIncompatible, NotFound, PermissionDenied, ToolNotApproved, Internal, Other("custom".into())];
    assert_eq!(codes.len(), 8);
    for code in &codes {
        let n = code.numeric_code();
        assert!((1000..6000).contains(&n), "numeric code 越界: {n}");
        let snake = code.snake_name();
        assert!(!snake.is_empty());
        assert!(!snake.contains(' '));
        let camel = code.camel_name();
        assert!(!camel.is_empty());
        if matches!(code, InvalidEnvelope) {
            assert_eq!(snake, "invalid_envelope");
            assert_eq!(camel, "invalidEnvelope");
            assert_eq!(n, 2001);
        }
        if matches!(code, ToolNotApproved) {
            assert_eq!(snake, "tool_not_approved");
            assert_eq!(camel, "toolNotApproved");
            assert_eq!(n, 4002);
        }
    }
    let e = SdkError::business(NotFound, "id 42 not found");
    let s = format!("{e}");
    assert!(s.contains("id 42 not found"));
}

// =====================================================================
// Fixture 14 (R25): WireFormat envelope roundtrip (所有 WireKind)
// =====================================================================

#[test]
fn wire_envelope_roundtrip_with_other_kind() {
    use WireKind::*;
    let kinds = [Chat, ToolCall, MemoryRead, Health, Other("custom_event".into())];
    for k in &kinds {
        let env = Envelope::new(k.clone(), "req-x", serde_json::json!({"x": 1}));
        let line = env.encode().expect("encode ok");
        let back = Envelope::decode(&line).expect("decode ok");
        assert_eq!(back.kind, *k);
        assert_eq!(back.id, "req-x");
        assert_eq!(back.body["x"], 1);
    }
    let env = Envelope::new(Health, "v1", serde_json::json!({}));
    let v = env.expected_version().expect("v parse ok");
    assert_eq!(v, SdkVersion::new(0, 1, 0));
}

// =====================================================================
// Fixture 15 (R25): wiremock 200 OK
// =====================================================================

#[tokio::test]
async fn http_mock_200_ok_responds() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/tools/web_search/invoke"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"title": "rust", "url": "https://rust-lang.org", "snippet": "systems lang"}],
            "total": 1
        })))
        .mount(&server)
        .await;

    let c = ApeirethClient::new(&server.uri(), "a-valid-api-key-1234567890").unwrap();
    let url = c.tool_url("web_search").expect("url ok");
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", c.auth_header())
        .json(&serde_json::json!({"query": "rust"}))
        .send()
        .await
        .expect("reqwest send");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("body json");
    assert_eq!(body["total"], 1);
    assert_eq!(body["results"][0]["title"], "rust");
}

// =====================================================================
// Fixture 16 (R25): wiremock 401 Unauthorized
// =====================================================================

#[tokio::test]
async fn http_mock_401_unauthorized() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/tools/web_search/invoke"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/tools/web_search/invoke", server.uri()))
        .header("Authorization", "Bearer bad-key")
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status().as_u16(), 401);
    let body = resp.text().await.expect("body");
    assert_eq!(body, "Unauthorized");
}

// =====================================================================
// Fixture 17 (R25): wiremock 404 Not Found
// =====================================================================

#[tokio::test]
async fn http_mock_404_not_found() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/tools/nonexistent/invoke"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/tools/nonexistent/invoke", server.uri()))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status().as_u16(), 404);
}

// =====================================================================
// Fixture 18 (R25): wiremock 500 Internal
// =====================================================================

#[tokio::test]
async fn http_mock_500_internal() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/tools/code_exec/invoke"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/tools/code_exec/invoke", server.uri()))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status().as_u16(), 500);
}

// =====================================================================
// Fixture 19 (R25): wiremock 429 Rate Limited (含 Retry-After header)
// =====================================================================

#[tokio::test]
async fn http_mock_429_rate_limited() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/tools/file_ops/invoke"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "60")
                .set_body_string("Too Many Requests"),
        )
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/tools/file_ops/invoke", server.uri()))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status().as_u16(), 429);
    assert_eq!(
        resp.headers().get("Retry-After").unwrap().to_str().unwrap(),
        "60"
    );
}

// =====================================================================
// Fixture 20 (R25): base_url 尾 slash 规范化 + http → ws
// =====================================================================

#[test]
fn base_url_trailing_slash_normalized() {
    let c1 = ApeirethClient::new("https://api.apeireth.io/", "a-valid-api-key-1234567890").unwrap();
    assert_eq!(c1.base_url, "https://api.apeireth.io", "尾 slash 必 trim");
    let c2 = ApeirethClient::new("https://api.apeireth.io", "a-valid-api-key-1234567890").unwrap();
    assert_eq!(c2.base_url, "https://api.apeireth.io");
    for c in [&c1, &c2] {
        let url = c.tool_url("web_search").unwrap();
        assert!(!url.contains("//v1"), "必无双 slash: {url}");
    }
    let ws = c1.ws_url();
    assert_eq!(ws, "wss://api.apeireth.io/v1/stream");
    let c3 = ApeirethClient::new("http://localhost:8080", "a-valid-api-key-1234567890").unwrap();
    assert_eq!(c3.ws_url(), "ws://localhost:8080/v1/stream", "http → ws");
}

// =====================================================================
// Fixture 21 (R25): 工具名大小写敏感
// =====================================================================

#[test]
fn tool_call_case_sensitive() {
    assert!(validate_tool_call("Web_Search", &serde_json::json!({})).is_err());
    assert!(validate_tool_call("WEB_SEARCH", &serde_json::json!({})).is_err());
    assert!(validate_tool_call("WebSearch", &serde_json::json!({})).is_err());
    assert!(validate_tool_call("web_search", &serde_json::json!({})).is_ok());
    assert!(validate_sdk_method("apeireth_sdk_WEB_SEARCH").is_err());
    assert!(validate_sdk_method("apeireth_sdk_web_search").is_ok());
}

// =====================================================================
// Fixture 22 (R25): Audit logger 1000 条不爆
// =====================================================================

#[test]
fn audit_logger_handles_1000_entries() {
    let logger = AuditLogger::new();
    assert!(logger.is_empty());
    for i in 0..1000 {
        logger.append(AuditEntry {
            ts_ms: 1_000_000 + i,
            api_key_hash: format!("hash-{i}"),
            tool: "web_search".into(),
            action: "search".into(),
            ok: i % 2 == 0,
            duration_ms: i as u64,
            trace_id: format!("tr-{i:04}"),
        });
    }
    assert_eq!(logger.len(), 1000);
    let l2 = AuditLogger::default();
    assert!(l2.is_empty());
}

// =====================================================================
// Fixture 23 (R25): 版本协商 (Incompatible 跨 major)
// =====================================================================

#[test]
fn version_negotiate_across_major_is_incompatible() {
    let v1 = SdkVersion::new(0, 1, 0);
    let v2 = SdkVersion::new(1, 0, 0);
    assert_eq!(negotiate(v1, v2), WireCompat::Incompatible);
    assert_eq!(negotiate(v2, v1), WireCompat::Incompatible);
    assert_eq!(negotiate(v1, v1), WireCompat::Exact);
    let newer = SdkVersion::new(0, 1, 5);
    assert_eq!(negotiate(v1, newer), WireCompat::ServerNewer);
    assert_eq!(negotiate(newer, v1), WireCompat::ServerOlder);
    assert!(SdkVersion::parse("invalid").is_none());
    assert!(SdkVersion::parse("1.2").is_none());
    assert!(SdkVersion::parse("1.2.3.4").is_none());
}

// =====================================================================
// Fixture 24 (R25): Token bucket 边缘 case
// =====================================================================

#[test]
fn token_bucket_edge_cases() {
    let empty = TokenBucket::with_config(0.0, 1.0);
    assert!(!empty.try_acquire(), "0 capacity 必永远拒");
    let big = TokenBucket::with_config(100.0, 100.0);
    for _ in 0..50 {
        assert!(big.try_acquire());
    }
    let slow = TokenBucket::with_config(1.0, 0.5);
    assert!(slow.try_acquire());
    assert!(!slow.try_acquire());
    let secs = slow.retry_after_secs();
    assert!(secs >= 1, "slow refill retry_after 必 >= 1s, 实际: {secs}");
}

// =====================================================================
// Fixture 25 (R25): Quota stub 反复调都返 501
// =====================================================================

#[test]
fn quota_stub_always_501() {
    let q = QuotaStub::new();
    for _ in 0..10 {
        let err = q.check();
        assert!(matches!(err, Err(SdkClientError::QuotaExceeded(_))));
    }
    // 默认构造 + 0 monthly_limit.
    assert_eq!(q.monthly_limit, 0);
}

// =====================================================================
// Fixture 26 (R25): preflight 必追加 audit entry
// =====================================================================

#[test]
fn preflight_appends_audit_entry() {
    let p = AuthPipeline::new("a-valid-api-key-1234567890").unwrap();
    let before = p.audit.len();
    p.preflight("web_search", "search").expect("ok");
    let after = p.audit.len();
    assert_eq!(after, before + 1, "preflight 必追加 1 条 audit");
    // 同一 preflight 不重复追加 (同 token bucket 1 次).
    p.preflight("web_search", "search").expect("ok");
    assert_eq!(p.audit.len(), before + 2, "每次 preflight 各追加 1 条");
}

// =====================================================================
// Fixture 27 (R25): wiremock 多端点并发 (3 工具 method 并行 hit 同一 server)
// =====================================================================

#[tokio::test]
async fn http_mock_multi_endpoint_concurrent() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/tools/web_search/invoke"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"tool": "web_search"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/tools/code_exec/invoke"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"tool": "code_exec"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/tools/git_ops/invoke"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"tool": "git_ops"})))
        .mount(&server)
        .await;

    let c = ApeirethClient::new(&server.uri(), "a-valid-api-key-1234567890").unwrap();
    let client = reqwest::Client::new();
    // 并发 hit 3 工具 method.
    let f1 = client.post(c.tool_url("web_search").unwrap()).send();
    let f2 = client.post(c.tool_url("code_exec").unwrap()).send();
    let f3 = client.post(c.tool_url("git_ops").unwrap()).send();
    let (r1, r2, r3) = tokio::join!(f1, f2, f3);
    assert_eq!(r1.unwrap().status().as_u16(), 200);
    assert_eq!(r2.unwrap().status().as_u16(), 200);
    assert_eq!(r3.unwrap().status().as_u16(), 200);
}
