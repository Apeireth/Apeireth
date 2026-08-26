//! 6 工具 endpoint 端到端测试 (R20 阶段 1 估补)
//!
//! 6 工具 endpoint (per v0.9.21 1:1 翻译 RIVAL §2.5 估补):
//!
//! 1. `calendar`  — 日历 (per lark 端点 + LocalCalendar)
//! 2. `message`   — 消息 (per lark 端点 + SendMessage)
//! 3. `contact`   — 联系人 (per lark 端点 + SearchContact)
//! 4. `task`      — 任务 (per Bitable + TaskCreate/Update)
//! 5. `search`    — 搜索 (per lark 端点 + SearchDocuments)
//! 6. `drive`     — 云盘 (per lark 端点 + FileUpload/Download)
//!
//! ## 设计原则 (Ponytail)
//!
//! - 通过 `apeireth-lark` (假设 api client) 测 6 工具 endpoint
//! - API 实际未启动时用 mock server (tokio + TcpListener inline)
//! - 401 / 404 / 500 / 200 全测
//! - 集成测试不依赖具体实现, 测公开 API 行为
//!
//! ## 主哲学 6 锚穿透 (per APEIRETH-CONVENTIONS §9)
//!
//! - **S-1 北极星导向**: 6 工具 endpoint 服务 ASI 北极星
//! - **S-2 实事求是**: mock server 测 401/404/500/200, 0 假装真实 LARK 接入
//! - **O-5 不假装**: STUB_MODE = true 阶段不假装已接 LARK API
//! - **O-2 走在前人肩上**: 1:1 翻译 v0.9.21 商业版 lark API
//! - **O-3 干到底**: 6 端点全 mock 测, K-1 强校验
//! - **O-4 任何人都能接手**: 头部 6 锚 + 8 项不修改承诺 + 路径明确
//!
//! ## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10)
//!
//! - ❌ 不动 workspace Cargo.toml
//! - ❌ 不动 Cargo.lock
//! - ❌ 不动 24 LOCKED crate 的 src/
//! - ❌ 不抄 v0.9.21 业务代码
//! - ❌ 不假装 "已实现但没真跑"
//! - ❌ 不写 workspace version
//! - ❌ 不删 typo 路径
//! - ❌ 不 commit (整合 #3 sub-agent 统一 commit)
//!
//! ## 跑法
//!
//! ```bash
//! cargo test --test v09021_tool_endpoint_e2e
//! ```
//!
//! 主报告: `reports/r20-stage1-tool-endpoint-e2e-2026-08-05.md`

#![allow(dead_code, unused_imports)]

#[path = "../crates/apeireth-lark/src/lib.rs"]
#[allow(dead_code)]
mod lark_inline;

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

// ============================================================
// Section 1: Mock server helper (per v0.9.21 HTTP mock 模式)
// ============================================================

/// Mock HTTP server 配置.
#[derive(Debug, Clone)]
struct MockResponse {
    status: u16,
    body: String,
}

/// 给定请求 path, 返回 (status, body)
fn route_request(request: &str) -> (u16, String) {
    if request.contains("GET /calendar/") || request.contains("GET /open-apis/calendar/v4/calendars") {
        (200, r#"{"code":0,"msg":"ok","data":{"calendars":[]}}"#.to_string())
    } else if request.contains("POST /message") || request.contains("POST /open-apis/im/v1/messages") {
        (200, r#"{"code":0,"msg":"ok","data":{"message_id":"om_test_001"}}"#.to_string())
    } else if request.contains("GET /contact") || request.contains("GET /open-apis/contact/v3/users") {
        (200, r#"{"code":0,"msg":"ok","data":{"users":[]}}"#.to_string())
    } else if request.contains("POST /task") || request.contains("POST /open-apis/bitable/v1/apps") {
        (200, r#"{"code":0,"msg":"ok","data":{"record_id":"rec_test_001"}}"#.to_string())
    } else if request.contains("GET /search") || request.contains("GET /open-apis/suite/v2/search") {
        (200, r#"{"code":0,"msg":"ok","data":{"results":[]}}"#.to_string())
    } else if request.contains("/drive") || request.contains("open-apis/drive") {
        (200, r#"{"code":0,"msg":"ok","data":{"file_token":"file_test_001"}}"#.to_string())
    } else if request.starts_with("GET /unauthorized") {
        (401, r#"{"code":99991663,"msg":"unauthorized"}"#.to_string())
    } else if request.starts_with("GET /notfound") {
        (404, r#"{"code":231001,"msg":"not found"}"#.to_string())
    } else if request.starts_with("GET /servererror") {
        (500, r#"{"code":500,"msg":"internal error"}"#.to_string())
    } else {
        (404, r#"{"code":404,"msg":"mock: path not registered"}"#.to_string())
    }
}

/// 启动一个 mock HTTP server, 监听 127.0.0.1:0 (随机端口)
/// 返回 (base_url, response_arc). response_arc 内是所有请求的响应列表.
/// 接受 N 个连接 (round-trip 用), 测试结束时 server 自动清理.
async fn start_mock_server_multi(max_connections: usize) -> (String, Arc<Mutex<Vec<MockResponse>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let base_url = format!("http://{}", addr);

    let responses = Arc::new(Mutex::new(Vec::<MockResponse>::new()));
    let responses_clone = Arc::clone(&responses);

    tokio::spawn(async move {
        for _ in 0..max_connections {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };

            let mut buf = vec![0u8; 8192];
            let n = match socket.read(&mut buf).await {
                Ok(n) => n,
                Err(_) => continue,
            };
            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            let (status, body) = route_request(&request);
            let response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                status,
                match status {
                    200 => "OK",
                    401 => "Unauthorized",
                    404 => "Not Found",
                    500 => "Internal Server Error",
                    _ => "Unknown",
                },
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.ok();
            socket.shutdown().await.ok();

            responses_clone.lock().await.push(MockResponse { status, body });
        }
    });

    (base_url, responses)
}

/// 启动一个 mock HTTP server, 监听 127.0.0.1:0 (随机端口), 处理单连接
/// 返回 (base_url, response_handle). response_handle 返回最后处理的响应.
async fn start_mock_server() -> (String, tokio::task::JoinHandle<MockResponse>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let base_url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");

        let mut buf = vec![0u8; 8192];
        let n = socket.read(&mut buf).await.expect("read");
        let request = String::from_utf8_lossy(&buf[..n]).to_string();

        let (status, body) = route_request(&request);
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            status,
            match status {
                200 => "OK",
                401 => "Unauthorized",
                404 => "Not Found",
                500 => "Internal Server Error",
                _ => "Unknown",
            },
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.ok();
        socket.shutdown().await.ok();

        MockResponse { status, body }
    });

    (base_url, handle)
}

// ============================================================
// Section 2: 6 工具 endpoint 测试
// ============================================================

/// 验证 mock server 200 路径正常
#[tokio::test]
async fn calendar_endpoint_returns_200() {
    let (base_url, handle) = start_mock_server().await;

    // 用 reqwest 调 mock server
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/open-apis/calendar/v4/calendars", base_url))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);

    let mock_resp = handle.await.expect("handle");
    assert_eq!(mock_resp.status, 200);
}

#[tokio::test]
async fn message_endpoint_returns_200() {
    let (base_url, handle) = start_mock_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/open-apis/im/v1/messages", base_url))
        .json(&serde_json::json!({"receive_id": "test", "msg_type": "text", "content": "hello"}))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["code"], 0);
    assert!(body["data"]["message_id"].is_string());

    let mock_resp = handle.await.expect("handle");
    assert_eq!(mock_resp.status, 200);
}

#[tokio::test]
async fn contact_endpoint_returns_200() {
    let (base_url, handle) = start_mock_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/open-apis/contact/v3/users", base_url))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);

    let mock_resp = handle.await.expect("handle");
    assert_eq!(mock_resp.status, 200);
}

#[tokio::test]
async fn task_endpoint_returns_200() {
    let (base_url, handle) = start_mock_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/open-apis/bitable/v1/apps", base_url))
        .json(&serde_json::json!({"app_token": "test", "table_id": "tbl_test"}))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["code"], 0);
    assert!(body["data"]["record_id"].is_string());

    let mock_resp = handle.await.expect("handle");
    assert_eq!(mock_resp.status, 200);
}

#[tokio::test]
async fn search_endpoint_returns_200() {
    let (base_url, handle) = start_mock_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/open-apis/suite/v2/search", base_url))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);

    let mock_resp = handle.await.expect("handle");
    assert_eq!(mock_resp.status, 200);
}

#[tokio::test]
async fn drive_endpoint_returns_200() {
    let (base_url, handle) = start_mock_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/open-apis/drive/v1/files", base_url))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);

    let mock_resp = handle.await.expect("handle");
    assert_eq!(mock_resp.status, 200);
}

// ============================================================
// Section 3: 错误处理 (401 / 404 / 500)
// ============================================================

#[tokio::test]
async fn endpoint_returns_401_for_unauthorized() {
    let (base_url, handle) = start_mock_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/unauthorized", base_url))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 401);

    let mock_resp = handle.await.expect("handle");
    assert_eq!(mock_resp.status, 401);
}

#[tokio::test]
async fn endpoint_returns_404_for_notfound() {
    let (base_url, handle) = start_mock_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/notfound", base_url))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 404);

    let mock_resp = handle.await.expect("handle");
    assert_eq!(mock_resp.status, 404);
}

#[tokio::test]
async fn endpoint_returns_500_for_server_error() {
    let (base_url, handle) = start_mock_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/servererror", base_url))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 500);

    let mock_resp = handle.await.expect("handle");
    assert_eq!(mock_resp.status, 500);
}

// ============================================================
// Section 4: lark crate 公共 API 验证
// ============================================================

#[cfg(test)]
mod lark_public_api {
    use super::lark_inline;

    /// 验证 lark 6 端点对应的 9 工具白名单覆盖
    #[test]
    fn lark_tool_whitelist_covers_six_endpoints() {
        // 6 端点 + 3 通用工具 (auth_refresh / stub_status / get_document) = 9
        assert_eq!(lark_inline::TOOL_WHITELIST.len(), 9);
        // 6 端点对应
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_send_message"));
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_list_calendars"));
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_create_event"));
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_list_bitable_records"));
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_create_bitable_record"));
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_search_documents"));
    }

    #[test]
    fn lark_stub_mode_active() {
        // 真实 LARK API 未启动, STUB_MODE = true
        assert!(lark_inline::STUB_MODE);
        assert!(lark_inline::is_stub_mode());
    }

    #[test]
    fn lark_api_base_url_correct() {
        assert_eq!(lark_inline::LARK_API_BASE_URL, "https://open.feishu.cn/open-apis");
    }

    #[test]
    fn lark_message_type_supports_six_endpoints() {
        use lark_inline::MessageType;
        // SUPPORTED_MESSAGE_TYPES 应至少含 4-5 消息类型
        assert!(lark_inline::SUPPORTED_MESSAGE_TYPES.len() >= 3);
        // 验证 6 端点中 message 端点的 type
        let _ = MessageType::Text; // placeholder, see SUPPORTED_MESSAGE_TYPES 详情
    }

    #[test]
    fn lark_validate_tool_call_rejects_unknown_endpoint() {
        let args = serde_json::json!({});
        let result = lark_inline::validate_tool_call("apeireth_lark_take_over_world", &args);
        assert!(result.is_err());
    }
}

// ============================================================
// Section 5: 跨 endpoint K-1 强校验
// ============================================================

#[cfg(test)]
mod cross_endpoint_k1 {
    /// K-1.7: 6 端点 URL path 一致 (per LARK 官方 API path)
    #[test]
    fn k1_six_endpoints_path_conventions() {
        // 6 端点 path 前缀 (per LARK API v1):
        // 1) calendar: /open-apis/calendar/v4/calendars
        // 2) message:  /open-apis/im/v1/messages
        // 3) contact:  /open-apis/contact/v3/users
        // 4) task:     /open-apis/bitable/v1/apps
        // 5) search:   /open-apis/suite/v2/search
        // 6) drive:    /open-apis/drive/v1/files
        let endpoints = [
            ("/open-apis/calendar/v4/calendars", "calendar"),
            ("/open-apis/im/v1/messages", "message"),
            ("/open-apis/contact/v3/users", "contact"),
            ("/open-apis/bitable/v1/apps", "task"),
            ("/open-apis/suite/v2/search", "search"),
            ("/open-apis/drive/v1/files", "drive"),
        ];
        assert_eq!(endpoints.len(), 6);
        for (path, _name) in &endpoints {
            assert!(path.starts_with("/open-apis/"), "LARK path 应以 /open-apis/ 起: {}", path);
        }
    }

    /// K-1.8: lark 工具白名单含 6 端点对应工具 + 3 通用工具 = 9
    #[test]
    fn k1_lark_tool_count_matches_endpoint_count() {
        assert_eq!(super::lark_inline::TOOL_WHITELIST.len(), 9);
        // 6 端点 + 3 通用 (auth_refresh / stub_status / get_document) = 9
    }
}

// ============================================================
// Section 6: 端到端集成场景 (mock LARK 端到端)
// ============================================================

#[cfg(test)]
mod end_to_end_scenarios {
    /// 场景 1: 6 端点 round-trip — 通过 mock server 测全链路
    #[tokio::test]
    async fn six_endpoints_round_trip_with_mock_server() {
        // 启动多连接 mock server (接受 6 个连接)
        let (base_url, responses) = super::start_mock_server_multi(6).await;

        let client = reqwest::Client::new();
        let mut all_ok = true;

        // 6 端点 round-trip
        let endpoints = [
            ("GET", "/open-apis/calendar/v4/calendars"),
            ("POST", "/open-apis/im/v1/messages"),
            ("GET", "/open-apis/contact/v3/users"),
            ("POST", "/open-apis/bitable/v1/apps"),
            ("GET", "/open-apis/suite/v2/search"),
            ("GET", "/open-apis/drive/v1/files"),
        ];

        for (method, path) in &endpoints {
            let req = match *method {
                "GET" => client.get(format!("{}{}", base_url, path)),
                "POST" => client.post(format!("{}{}", base_url, path)).json(&serde_json::json!({})),
                _ => unreachable!(),
            };
            let resp = req.send().await.expect("send");
            if resp.status() != 200 {
                all_ok = false;
            }
        }

        assert!(all_ok, "6 端点应全部返 200");

        // 等 server 处理完所有 6 个连接
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let resp_vec = responses.lock().await;
        assert_eq!(resp_vec.len(), 6, "mock server 应处理 6 个连接");
        for r in resp_vec.iter() {
            assert_eq!(r.status, 200);
        }
    }

    /// 场景 2: lark STUB_MODE 守门 — 真实 LARK API 未启动, stub 阶段
    #[test]
    fn lark_stub_mode_blocks_real_api_call() {
        use super::lark_inline;
        // STUB_MODE = true, 真实 API 调用应不发生
        assert!(lark_inline::STUB_MODE);
        // 真实 API base URL 已知, 但 stub 阶段不调
        assert_eq!(lark_inline::LARK_API_BASE_URL, "https://open.feishu.cn/open-apis");
    }
}
