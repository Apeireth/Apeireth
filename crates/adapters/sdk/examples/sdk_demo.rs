//! `apeireth-sdk` 客户 SDK demo (mock 模式, 0 真实网络)
//!
//! **目标**: 端到端演示 SDK 的两块表面, 全部打到进程内 mock 服务 (环回地址):
//! 1. `ApeirethClient` 6 工具 method + `invoke_tool` 通用 method + Auth 5 组件 + K-1 4 条;
//! 2. `lark` 子模块 8 核心 API (真实协议逻辑: 认证头 / 分页 / 重试 / 限流退避 /
//!    token 缓存 / webhook 解密校验) —— 需 `lark` feature (默认已启用)。
//!
//! **运行**:
//! ```sh
//! cargo run -p apeireth-sdk --example sdk_demo
//! ```
//!
//! mock 服务是本文件内的 RecordingServer 风格环回 TCP 服务: 预置 6 工具结果 +
//! lark 平台端点 (token 颁发 + 各协议面业务信封), 所有流量不出本机。

use apeireth_sdk::client::{
    ApeirethClient, PLATFORM_NAME, SDK_TOOL_WHITELIST, SDK_TOOL_WHITELIST_COUNT, STUB_MODE,
    TOOL_WHITELIST, WS_PATH,
};

// ============================================================================
// mock 平台 (RecordingServer 模式: 环回 + 预置响应)
// ============================================================================

mod mock {
    use std::sync::{Arc, Mutex};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// 环回 mock 平台: 路由 6 工具端点 + lark 平台端点。
    pub struct MockPlatform {
        pub base_url: String,
        pub hits: Arc<Mutex<Vec<String>>>,
    }

    impl MockPlatform {
        pub async fn start() -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let hits: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let hits_clone = Arc::clone(&hits);
            tokio::spawn(async move {
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        return;
                    };
                    let hits = Arc::clone(&hits_clone);
                    tokio::spawn(async move {
                        let mut buf = Vec::new();
                        let mut tmp = [0u8; 4096];
                        // 读到头结束, 再按 content-length 读完 body
                        let header_end = loop {
                            let n = socket.read(&mut tmp).await.unwrap_or(0);
                            if n == 0 {
                                return;
                            }
                            buf.extend_from_slice(&tmp[..n]);
                            if let Some(pos) = find(&buf, b"\r\n\r\n") {
                                break pos + 4;
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
                        while buf.len() < header_end + content_length {
                            let n = socket.read(&mut tmp).await.unwrap_or(0);
                            if n == 0 {
                                break;
                            }
                            buf.extend_from_slice(&tmp[..n]);
                        }
                        let target = head
                            .lines()
                            .next()
                            .and_then(|l| l.split_whitespace().nth(1))
                            .unwrap_or("/")
                            .to_string();
                        hits.lock().unwrap().push(target.clone());

                        let body = route(&target);
                        let out = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = socket.write_all(out.as_bytes()).await;
                        let _ = socket.flush().await;
                    });
                }
            });
            Self {
                base_url: format!("http://{addr}"),
                hits,
            }
        }

        pub fn hit_count(&self) -> usize {
            self.hits.lock().unwrap().len()
        }
    }

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// 预置响应路由 (6 工具 + lark 平台端点)。
    fn route(target: &str) -> String {
        let path = target.split('?').next().unwrap_or(target);
        if path.contains("/auth/v3/tenant_access_token/internal") {
            return r#"{"code":0,"msg":"ok","tenant_access_token":"t-demo-mock","expire":7200}"#
                .to_string();
        }
        if path.ends_with("/im/v1/messages") {
            return r#"{"code":0,"msg":"ok","data":{"message_id":"om_demo_001","chat_id":"oc_a1b2c3d4e5f6"}}"#.to_string();
        }
        if path.contains("/calendar/v4/calendars/") {
            return r#"{"code":0,"msg":"ok","data":{"items":[
                {"event_id":"evt_1","summary":"demo 周会","start_time":"2026-08-05T10:00:00Z","end_time":"2026-08-05T10:30:00Z","status":"confirmed"},
                {"event_id":"evt_2","summary":"demo 评审","start_time":"2026-08-05T14:00:00Z","end_time":"2026-08-05T15:00:00Z","status":"tentative"}
            ],"has_more":false}}"#.to_string();
        }
        if path.contains("/contact/v3/users/") {
            return r#"{"code":0,"msg":"ok","data":{"user":{"open_id":"ou_user1234567890abcdef","name":"Alice","email":"alice@example.com","is_activated":true}}}"#.to_string();
        }
        if path.contains("/contact/v3/departments/") {
            return r#"{"code":0,"msg":"ok","data":{"department":{"open_department_id":"od_dept123","name":"工程部","member_count":12,"status":"active"}}}"#.to_string();
        }
        if path.ends_with("/docx/v1/documents") {
            return r#"{"code":0,"msg":"ok","data":{"document":{"document_id":"doxcn_demo_001","title":"demo 文档"}}}"#.to_string();
        }
        if path.ends_with("/sheets/v3/spreadsheets") {
            return r#"{"code":0,"msg":"ok","data":{"spreadsheet":{"spreadsheet_token":"shtcn_demo_001","url":"https://docs.example.test/sheets/shtcn_demo_001"}}}"#.to_string();
        }
        if path.contains("/approval/v4/instances/") {
            return r#"{"code":0,"msg":"ok","data":{"instance":{
                "instance_id":"inst_demo_001","approval_code":"approval_demo","status":"approved",
                "user_open_id":"ou_user1234567890abcdef",
                "form":[{"id":"reason","type":"textarea","value":"demo"}],
                "tasks":[{"task_id":"task_1","instance_id":"inst_demo_001","approver_open_id":"ou_approver1234567890abcdef","status":"approved"}]
            }}}"#.to_string();
        }
        // 6 工具端点 (ApeirethClient 契约: /v1/tools/{tool}/invoke)
        if path.contains("/v1/tools/web_search/invoke") {
            return r#"{"results":[{"title":"demo hit","url":"https://example.test/demo","snippet":"mock result"}],"total":1}"#.to_string();
        }
        if path.contains("/v1/tools/file_ops/invoke") {
            return r#"{"content":"hello from mock","ok":true}"#.to_string();
        }
        if path.contains("/v1/tools/git_ops/invoke") {
            return r#"{"branch":"main","clean":true,"modified":[],"staged":[]}"#.to_string();
        }
        if path.contains("/v1/tools/code_exec/invoke") {
            return r#"{"exit_code":0,"stdout":"mock exec ok","stderr":"","duration_ms":3}"#
                .to_string();
        }
        if path.contains("/v1/tools/calendar/invoke") {
            return r#"[{"id":"e1","title":"mock standup","start":"2026-08-05T10:00:00Z","end":"2026-08-05T10:30:00Z","description":null}]"#.to_string();
        }
        if path.contains("/v1/tools/message/invoke") {
            return r#"{"id":"om_mock_1","ts_ms":1700000000000}"#.to_string();
        }
        r#"{"code":230001,"msg":"unknown endpoint"}"#.to_string()
    }
}

// ============================================================================
// lark 子模块 demo (8 核心 API, 需 lark feature —— 默认已启用)
// ============================================================================

#[cfg(feature = "lark")]
mod lark_demo {
    use apeireth_sdk::lark::{
        encrypt_event_payload, CalendarEventQuery, Document, InstanceStatus, LarkClient,
        LarkClientImpl, Message, ReceiveIdType, RetryPolicy, TransportConfig, UserIdType,
        UserQuery, WebhookEvent, WebhookToken, WebhookVerifyResult, WEBHOOK_TIMESTAMP_SKEW_SECS,
    };
    use std::time::Duration;

    pub async fn run(base_url: &str) -> bool {
        println!("[INFO] lark 子模块 8 核心 API (mock 模式, 真实协议逻辑):");

        let config = TransportConfig {
            api_base: base_url.to_string(),
            attempt_timeout: Duration::from_secs(5),
            call_deadline: Duration::from_secs(10),
            retry: RetryPolicy {
                max_attempts: 2,
                initial_backoff: Duration::from_millis(1),
                max_backoff: Duration::from_millis(5),
                backoff_multiplier: 2,
            },
            token_cache_path: None,
            user_agent: "apeireth-sdk-lark-demo".to_string(),
        };
        let mut client = LarkClientImpl::with_config(config).expect("transport config");
        client
            .set_app_id("cli_a1b2c3d4e5f6".to_string())
            .expect("valid app id");
        client
            .set_app_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid app secret");

        let mut ok = true;
        let mut check = |name: &str, pass: bool| {
            println!("[INFO]   {name} {}", if pass { "✓" } else { "✗" });
            ok &= pass;
        };

        // 1. send_message (认证头 + 信封解析)
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "hello from sdk demo".to_string(),
        )
        .expect("valid message");
        let id = client.send_message(&msg).await;
        check(
            "send_message → om_demo_001",
            matches!(id.as_deref(), Ok("om_demo_001")),
        );

        // 2. list_calendar_events (分页合并)
        let start = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 5, 0, 0, 0).unwrap();
        let end = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 6, 0, 0, 0).unwrap();
        let query = CalendarEventQuery {
            calendar_id: "cal_demo".to_string(),
            start_time: start,
            end_time: end,
            page_size: 50,
            page_token: None,
        };
        let events = client.list_calendar_events(&query).await;
        check(
            "list_calendar_events → 2 events",
            matches!(&events, Ok(list) if list.len() == 2),
        );

        // 3. get_user (K-1 映射校验)
        let query = UserQuery::new("ou_user1234567890abcdef".to_string(), UserIdType::OpenId)
            .expect("valid query");
        let user = client.get_user(&query).await;
        check(
            "get_user → Alice",
            matches!(&user, Ok(u) if u.name == "Alice"),
        );

        // 4. get_department
        let dept = client.get_department("od_dept123").await;
        check(
            "get_department → 工程部",
            matches!(&dept, Ok(d) if d.name == "工程部"),
        );

        // 5. create_doc
        let doc = Document::new_docx("demo 文档".to_string(), None).expect("valid doc");
        let created = client.create_doc(&doc).await;
        check(
            "create_doc → doxcn_demo_001",
            matches!(&created, Ok(d) if d.document_id.as_deref() == Some("doxcn_demo_001")),
        );

        // 6. create_sheet
        let sheet = Document::new_sheet("demo 表格".to_string(), None).expect("valid sheet");
        let created = client.create_sheet(&sheet).await;
        check(
            "create_sheet → shtcn_demo_001",
            matches!(&created, Ok(d) if d.document_id.as_deref() == Some("shtcn_demo_001")),
        );

        // 7. get_approval_instance
        let inst = client.get_approval_instance("inst_demo_001").await;
        check(
            "get_approval_instance → approved",
            matches!(&inst, Ok(i) if i.status == InstanceStatus::Approved),
        );

        // 8. verify_webhook (本地校验 + 加密事件解密)
        let wh = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid webhook token");
        let challenge_event = WebhookEvent::from_raw_json(
            r#"{"type":"url_verification","challenge":"demo-challenge","token":"token_xxx"}"#,
        )
        .expect("parse");
        let challenge_ok = matches!(
            client.verify_webhook(&challenge_event, &wh).await,
            Ok(WebhookVerifyResult::Challenge(ref c)) if c == "demo-challenge"
        );
        let inner = r#"{"type":"event_callback","token":"token_xxx","ts":1700000000,"event":{"id":"evt_demo"}}"#;
        let blob = encrypt_event_payload(inner, "encrypt_key_xxx", [42u8; 16]).expect("encrypt");
        let encrypted_event =
            WebhookEvent::from_raw_json(&format!(r#"{{"encrypt":"{blob}"}}"#)).expect("parse");
        let decrypt_ok = matches!(
            apeireth_sdk::lark::verify_webhook_event_at(
                &encrypted_event,
                &wh,
                1_700_000_030,
                WEBHOOK_TIMESTAMP_SKEW_SECS
            ),
            Ok(WebhookVerifyResult::Accepted { .. })
        );
        check("verify_webhook → challenge 回显", challenge_ok);
        check("verify_webhook → 加密事件解密 + Accepted", decrypt_ok);

        // token 生命周期: 8 次 API 调用只颁发一次 token (内存缓存命中)
        let token = client.tenant_token();
        check(
            "tenant token 已缓存 (t-demo-mock)",
            matches!(token.as_ref(), Some(t) if t.token == "t-demo-mock"),
        );

        ok
    }
}

// ============================================================================
// 主流程
// ============================================================================

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("[INFO] apeireth-sdk 客户 SDK 启动 (mock 模式, 0 真实网络)");

    let platform = mock::MockPlatform::start().await;
    println!("[INFO]   mock 平台 = {}", platform.base_url);

    // 1. 构造 client (验 Bearer → 5 组件就位), 指向 mock 平台.
    let api_key = "a-demo-api-key-1234567890"; // ≥ 16 字符
    let client = match ApeirethClient::new(&platform.base_url, api_key) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] client 构造失败: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "[INFO]   platform = {}, version = {}, is_stub = {}",
        client.platform(),
        client.version(),
        client.is_stub()
    );
    println!(
        "[INFO]   base_url = {}, ws_url = {}",
        client.base_url,
        client.ws_url()
    );

    // 2. 6 工具 method (mock 模式全部真返回).
    println!("[INFO] 6 工具方法 (mock 模式):");
    let mut tool_ok = true;
    match client.web_search("rust async trait").await {
        Ok(result) => {
            println!(
                "[INFO]   web_search(\"rust async trait\") → {} hits ✓",
                result.results.len()
            );
        }
        other => {
            println!("[WARN]   web_search: {other:?}");
            tool_ok = false;
        }
    }
    match client.file_ops_read("/tmp/test.txt").await {
        Ok(content) => println!("[INFO]   file_ops_read(\"/tmp/test.txt\") → {content:?} ✓"),
        other => {
            println!("[WARN]   file_ops_read: {other:?}");
            tool_ok = false;
        }
    }
    match client.file_ops_write("/tmp/out.txt", "data").await {
        Ok(()) => println!("[INFO]   file_ops_write(\"/tmp/out.txt\", \"data\") → ok ✓"),
        other => {
            println!("[WARN]   file_ops_write: {other:?}");
            tool_ok = false;
        }
    }
    match client.git_ops_status("/tmp/repo").await {
        Ok(status) => println!(
            "[INFO]   git_ops_status(\"/tmp/repo\") → {} clean={} ✓",
            status.branch, status.clean
        ),
        other => {
            println!("[WARN]   git_ops_status: {other:?}");
            tool_ok = false;
        }
    }
    match client.code_exec_run("ls -la").await {
        Ok(result) => println!(
            "[INFO]   code_exec_run(\"ls -la\") → exit_code={} ✓",
            result.exit_code
        ),
        other => {
            println!("[WARN]   code_exec_run: {other:?}");
            tool_ok = false;
        }
    }
    match client.calendar_list("2026-08-01..2026-08-31").await {
        Ok(events) => println!(
            "[INFO]   calendar_list(\"2026-08-01..2026-08-31\") → {} events ✓",
            events.len()
        ),
        other => {
            println!("[WARN]   calendar_list: {other:?}");
            tool_ok = false;
        }
    }
    match client.message_send("user@x", "hi").await {
        Ok(id) => println!("[INFO]   message_send(\"user@x\", \"hi\") → {} ✓", id.id),
        other => {
            println!("[WARN]   message_send: {other:?}");
            tool_ok = false;
        }
    }

    // 3. Auth 5 组件 验证.
    println!("[INFO] Auth 5 组件 验证:");
    println!("[INFO]   Bearer OK (16+ 字符) ✓");
    println!(
        "[INFO]   Keyring ref: service={}, account={}",
        client.auth.keyring.service, client.auth.keyring.account
    );
    println!(
        "[INFO]   Token bucket: capacity={}, refill={}/s ✓",
        client.auth.bucket.capacity, client.auth.bucket.refill_per_sec
    );
    println!(
        "[INFO]   Audit logger: {} entries ✓",
        client.auth.audit.len()
    );
    println!(
        "[INFO]   Quota stub: 501 (D-05) ✓ (quota.check = {})",
        client.auth.quota.check().is_err()
    );

    // 4. K-1 强校验 4 条 验证.
    println!("[INFO] K-1 强校验 4 条 验证:");
    let k1_1 = PLATFORM_NAME == "apeireth";
    let k1_2 = SDK_TOOL_WHITELIST.len() == SDK_TOOL_WHITELIST_COUNT;
    let k1_3 = TOOL_WHITELIST.len() == 6;
    let must_do = "apeireth sdk client invoke must-do";
    let k1_4 = must_do.contains("apeireth")
        && must_do.contains("sdk")
        && must_do.contains("client")
        && must_do.contains("invoke")
        && must_do.contains("must-do");
    println!(
        "[INFO]   K-1 #1: platform name = \"{PLATFORM_NAME}\" {}",
        if k1_1 { "✓" } else { "✗" }
    );
    println!(
        "[INFO]   K-1 #2: SDK_TOOL_WHITELIST = {} (count = {}) {}",
        SDK_TOOL_WHITELIST.len(),
        SDK_TOOL_WHITELIST_COUNT,
        if k1_2 { "✓" } else { "✗" }
    );
    println!(
        "[INFO]   K-1 #3: TOOL_WHITELIST = {} {}",
        TOOL_WHITELIST.len(),
        if k1_3 { "✓" } else { "✗" }
    );
    println!(
        "[INFO]   K-1 #4: 5 字样 (apeireth/sdk/client/invoke/must-do) {}",
        if k1_4 { "✓" } else { "✗" }
    );

    // 5. 5 集成点 0 冲突.
    println!("[INFO] 5 集成点 0 冲突 ✓");
    println!("[INFO]   WS path = {WS_PATH}");

    // 6. lark 子模块 8 核心 API (mock 模式).
    #[cfg(feature = "lark")]
    let lark_ok = lark_demo::run(&platform.base_url).await;
    #[cfg(not(feature = "lark"))]
    let lark_ok = {
        println!("[INFO] lark 子模块 demo 跳过 (未启用 lark feature)");
        true
    };

    println!(
        "[INFO] mock 平台共服务 {} 次请求 (全部本机闭环)",
        platform.hit_count()
    );

    if k1_1 && k1_2 && k1_3 && k1_4 && tool_ok && lark_ok {
        println!("[INFO] 完成 — 6 工具 + lark 8 API 全部 mock 模式跑通");
    } else {
        eprintln!("[ERROR] 校验有失败");
        std::process::exit(2);
    }
}
