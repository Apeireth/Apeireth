//! W2-① resident form (daemon 化) behavior tests: the PID-file resident slot
//! (single-instance claim + explicit "already running" refusal), the graceful
//! shutdown closing bookkeeping (final audit fact + full observation flush),
//! and the token gate on the shutdown control endpoint.
//!
//! The shutdown endpoint is a security-relevant surface (it stops a serving
//! gateway), so its tests are adversarial by construction: missing and forged
//! tokens are refused, the three-state gate semantics are unchanged, and no
//! path leaks a token value.

use std::sync::{Arc, Mutex};

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, claim_slot, read_slot,
    serve_state_with_shutdown, AuditCommand, GatewayServices, GatewayState, GatewayTokenGate,
    ResidentError, ResidentRecord,
};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const TOKEN: &str = "loopback-token-0123456789abcdef";
const WRONG_TOKEN: &str = "forged-token-fedcba9876543210";

/// Fresh throwaway directory under the process temp dir (no extra deps).
fn scratch_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-resident-test-{}-{label}",
        std::process::id()
    ));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A TCP port with nothing listening (probe target for "stale" slots).
async fn closed_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Audit archive fake that writes real JSON lines to disk, so the graceful
/// shutdown test can assert the closing facts are actually persisted (落盘).
#[derive(Debug)]
struct FileAudit {
    path: std::path::PathBuf,
    facts: Mutex<Vec<(String, Option<String>)>>,
}

impl FileAudit {
    fn new(path: std::path::PathBuf) -> Self {
        Self {
            path,
            facts: Mutex::new(Vec::new()),
        }
    }

    fn contents(&self) -> String {
        fs_err::read_to_string(&self.path).unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl AuditCommand for FileAudit {
    async fn append_audit(&self, event: &str, detail: Option<&str>) {
        self.facts
            .lock()
            .unwrap()
            .push((event.to_string(), detail.map(str::to_string)));
        let line = serde_json::json!({
            "event": event,
            "detail": detail,
        });
        use std::io::Write;
        let mut file = fs_err::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .expect("open audit archive");
        writeln!(file, "{line}").expect("append audit line");
    }
}

// ---------------------------------------------------------------------------
// 常驻占位 (PID 文件): 二次启动显式拒绝 + 陈旧占位接管 + 停机释放
// ---------------------------------------------------------------------------

/// 二次启动不双开: 活占位 (记录端口仍应答) 必须被显式拒绝, 占位内容不被篡改;
/// 陈旧占位 (端口已死) 与损坏占位必须被接管; 停机释放只清自己的占位。
#[tokio::test]
async fn resident_slot_refuses_a_live_instance_and_takes_over_a_stale_one() {
    let dir = scratch_dir("claim");
    let slot_path = dir.join("gateway.pid");

    // 活占位: 一个真实监听者占住记录端口 → 二次启动显式报"已在运行"。
    let live = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let live_port = live.local_addr().unwrap().port();
    let live_record = ResidentRecord {
        pid: 4242,
        port: live_port,
        bind: "127.0.0.1".to_string(),
        started_at_ms: 1,
    };
    fs_err::write(&slot_path, live_record.render()).unwrap();

    let refused = claim_slot(
        &slot_path,
        ResidentRecord::for_this_process("127.0.0.1", 9, 2),
    )
    .await;
    let error = refused.err().expect("second start must be refused");
    assert!(
        matches!(&error, ResidentError::AlreadyRunning { pid: 4242, port } if *port == live_port),
        "unexpected refusal: {error:?}"
    );
    let message = error.to_string();
    assert!(
        message.contains("already running"),
        "拒绝必须显式报'已在运行': {message}"
    );
    assert!(
        message.contains("refusing to start a second instance"),
        "{message}"
    );
    // 拒绝不篡改既有占位。
    assert_eq!(
        read_slot(&slot_path).unwrap().expect("slot kept"),
        live_record
    );

    // 陈旧占位: 端口已死 → 接管, 占位被新记录覆盖, 停机释放清掉。
    let stale_port = closed_port().await;
    let stale_record = ResidentRecord {
        pid: 4242,
        port: stale_port,
        bind: "127.0.0.1".to_string(),
        started_at_ms: 1,
    };
    fs_err::write(&slot_path, stale_record.render()).unwrap();
    let taken = claim_slot(
        &slot_path,
        ResidentRecord::for_this_process("127.0.0.1", 9, 2),
    )
    .await
    .expect("stale slot must be taken over");
    let ours = read_slot(&slot_path).unwrap().expect("slot rewritten");
    assert_eq!(ours.pid, std::process::id(), "占位被本进程记录接管");
    drop(taken);
    assert!(
        read_slot(&slot_path).unwrap().is_none(),
        "停机释放必须清掉自己的 PID 文件"
    );

    // 损坏占位: 内容不可读 = 它描述的实例不可信 → 同样接管。
    fs_err::write(&slot_path, "not a slot file at all").unwrap();
    let taken = claim_slot(
        &slot_path,
        ResidentRecord::for_this_process("127.0.0.1", 9, 2),
    )
    .await
    .expect("corrupt slot must be taken over");
    assert_eq!(
        read_slot(&slot_path).unwrap().expect("slot rewritten").pid,
        std::process::id()
    );
    drop(taken);

    drop(live);
}

// ---------------------------------------------------------------------------
// 优雅停机收尾: 最后一桩审计 + 全量落盘
// ---------------------------------------------------------------------------

/// 停机触发后, 服务循环必须先收尾再退出: 写下 `gateway.shutdown` 审计事实,
/// 并把攒批中的全部观测事实 flush 到审计档案 (真文件断言, 不是内存观察)。
#[tokio::test]
async fn graceful_shutdown_records_the_final_audit_and_flushes_it_to_the_archive() {
    let dir = scratch_dir("shutdown");
    let audit_path = dir.join("daemon-audit.jsonl");
    let audit = Arc::new(FileAudit::new(audit_path.clone()));
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        ..GatewayServices::default()
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state = build_gateway_state_with_services(runtime, services);

    // 停机前先攒一桩观测事实 (只进攒批缓冲, 尚未落盘) —— 它必须随停机收尾
    // 的全量 flush 一起出现在档案里。
    state.observations.buffer_audit(
        "gateway.test_precursor",
        Some("buffered before shutdown".to_string()),
    );

    let server = tokio::spawn(serve_state_with_shutdown(listener, state, async move {
        let _ = stop_rx.await;
    }));

    // 等服务真起来 (健康探针)。
    let health_url = format!("http://127.0.0.1:{port}/health");
    let mut ready = false;
    for _ in 0..100 {
        let up = reqwest::get(&health_url)
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false);
        if up {
            ready = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(ready, "gateway must answer /health before shutdown");

    // 停机 (进程信号面的同义触发)。
    let _ = stop_tx.send(());
    server.await.expect("serve task").expect("graceful serve");

    // 收尾断言: 最后一桩审计事实 + 全部攒批事实已经写到档案文件 (落盘)。
    let contents = audit.contents();
    assert!(
        contents.contains("gateway.shutdown"),
        "收尾必须写下 gateway.shutdown 审计事实: {contents}"
    );
    assert!(
        contents.contains("graceful=1"),
        "收尾事实须标明优雅停机: {contents}"
    );
    assert!(
        contents.contains("gateway.test_precursor"),
        "停机前攒批的观测事实必须全量落盘: {contents}"
    );

    let _ = fs_err::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// 停机控制端点: 令牌门 (对抗面) + 三态口径不变
// ---------------------------------------------------------------------------

/// Audit archive fake recording facts in memory (gate-semantics assertions).
#[derive(Debug, Default)]
struct RecordingAudit {
    facts: Mutex<Vec<(String, Option<String>)>>,
}

impl RecordingAudit {
    fn detail_of(&self, event: &str) -> Option<String> {
        self.facts
            .lock()
            .unwrap()
            .iter()
            .find(|(name, _)| name == event)
            .and_then(|(_, detail)| detail.clone())
    }
}

#[async_trait::async_trait]
impl AuditCommand for RecordingAudit {
    async fn append_audit(&self, event: &str, detail: Option<&str>) {
        self.facts
            .lock()
            .unwrap()
            .push((event.to_string(), detail.map(str::to_string)));
    }
}

struct Harness {
    state: GatewayState,
    audit: Arc<RecordingAudit>,
}

async fn harness_with_gate(gate: GatewayTokenGate) -> Harness {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(audit.clone()),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = gate;
    Harness { state, audit }
}

fn shutdown_post(token: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/v1/admin/shutdown")
        .body(Body::empty())
        .unwrap();
    if let Some(token) = token {
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {token}").parse().expect("bearer header"),
        );
    }
    request
}

async fn json_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&body).into_owned())
}

/// 对抗面: 停机端点在 Enforced 门下 —— 缺令牌 401 auth_missing_key、伪令牌
/// 401 auth_invalid_key 且**不触发停机**; 正确令牌过门触发 (幂等: 首次 first,
/// 重复 repeat)。拒绝帧与审计都不得回显任何令牌值。
#[tokio::test]
async fn shutdown_endpoint_is_behind_the_token_gate_and_only_the_matching_token_trips_it() {
    let harness = harness_with_gate(GatewayTokenGate::Enforced(TOKEN.to_string())).await;
    let router = || canonical_router_with_state(harness.state.clone());

    // 缺令牌: 401 auth_missing_key, 闸不放。
    let (status, body) = json_body(router().oneshot(shutdown_post(None)).await.unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("auth_missing_key"), "{body}");
    assert!(!harness.state.shutdown.is_triggered(), "拒绝不得触发停机");

    // 伪令牌: 401 auth_invalid_key, 闸不放, 令牌值不回显。
    let (status, body) = json_body(
        router()
            .oneshot(shutdown_post(Some(WRONG_TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("auth_invalid_key"), "{body}");
    assert!(!body.contains(WRONG_TOKEN), "伪令牌不得回显: {body}");
    assert!(!body.contains(TOKEN), "期望令牌不得外泄: {body}");
    assert!(!harness.state.shutdown.is_triggered(), "拒绝不得触发停机");

    // 正确令牌: 过门触发; 重复调用幂等 (服务脚本重试安全)。
    let (status, body) =
        json_body(router().oneshot(shutdown_post(Some(TOKEN))).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("shutting_down"), "{body}");
    assert!(body.contains("\"trigger\":\"first\""), "{body}");
    assert!(harness.state.shutdown.is_triggered());

    let (status, body) =
        json_body(router().oneshot(shutdown_post(Some(TOKEN))).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"trigger\":\"repeat\""), "{body}");

    // 请求留痕入审计 (事实无令牌面)。
    harness.state.observations.flush().await;
    let detail = harness.audit.detail_of("gateway.shutdown_requested");
    assert!(
        detail.as_deref().unwrap_or("").contains("first=true"),
        "{detail:?}"
    );
}

/// 三态门口径照旧: 无令牌 (env 未设) = 放行 + 警告帧 + 审计; 逃生阀
/// (`none`) = 放行不逐次告警但留痕。新端点不引入第四种口径。
#[tokio::test]
async fn shutdown_endpoint_keeps_the_three_state_gate_semantics() {
    // 未设态: 放行 + 警告帧 + 审计。
    let harness = harness_with_gate(GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(harness.state.clone());
    let mut frames = harness.state.events.subscribe();
    let (status, _) = json_body(router.oneshot(shutdown_post(None)).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK, "未设态放行 (兼容路径零断)");
    assert!(harness.state.shutdown.is_triggered());
    let frame = frames.try_recv().expect("warning frame");
    assert_eq!(frame.event, "gateway_auth_unprotected");
    assert_eq!(frame.data["endpoint"], "/v1/admin/shutdown");
    assert_eq!(frame.data["mode"], "env_unset");
    harness.state.observations.flush().await;
    assert_eq!(
        harness
            .audit
            .detail_of("gateway.auth_unprotected")
            .as_deref(),
        Some("endpoint=/v1/admin/shutdown mode=env_unset")
    );

    // 逃生阀: 放行不逐次告警, 但留审计痕。
    let harness = harness_with_gate(GatewayTokenGate::UnprotectedDeclared).await;
    let router = canonical_router_with_state(harness.state.clone());
    let mut frames = harness.state.events.subscribe();
    let (status, _) = json_body(router.oneshot(shutdown_post(None)).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(harness.state.shutdown.is_triggered());
    assert!(frames.try_recv().is_err(), "明示不设防不逐次发警告帧");
    harness.state.observations.flush().await;
    assert_eq!(
        harness
            .audit
            .detail_of("gateway.auth_unprotected")
            .as_deref(),
        Some("endpoint=/v1/admin/shutdown mode=declared_none")
    );
}
