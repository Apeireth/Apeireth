//! W2-③ 多客户端会话复用: 并发会话路由面的面契约。
//!
//! 这里钉五件事 (桌面/TUI/IM/web 多端同时在线同一个后端):
//! 1. 事件扇出 —— 每客户端独立订阅, 同一会话的事件扇出多端, 会话间隔离;
//! 2. 同会话并发回合 —— **排队 + 审计** (第二回合排队等第一个跑完, 不发 busy
//!    帧, 排队事实入 `gateway.session_client` 审计行);
//! 3. 会话账本零交叠写 —— 对抗并发压测 (多客户端回合 + 设置补丁同时砸一个
//!    会话, 消息一条不丢、转写与账本 lockstep、设置改动不被回合覆写);
//! 4. 会话归属 —— 过门客户端名与会话事件关联 (`gateway.session_client`);
//! 5. 令牌隔离 —— 客户端 A 的令牌不能冒名 B 的审计名 (name↔token 绑定校验,
//!    无绑定的自称名 fail-closed)。
//!
//! 外加面板一致性: `/v1/panel/*` 每请求一次快照读 (会话列表/内存计数)。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{CapabilityId, ModelId, PluginId, SessionId, Timestamp};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand, ClientIdentity,
    ClientTokenConfig, ClientTokenSpec, ClientTokenTable, EpisodeDto, GatewayServices,
    GatewayState, GatewayTokenGate, GraphEdgeDto, GraphNodeDto, MemoryGraphDto, MemoryQuery,
    SessionQuery, SessionSummaryDto, CLIENT_NAME_CLAIM_HEADER,
};
use apeireth_governance::AllowAll;
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, NormalizedFinishReason, NormalizedRequest,
    NormalizedResponse, NormalizedUsage,
};
use apeireth_runtime::canonical::{
    InMemorySessionStore, Runtime, Session, SessionEventKind, SessionStore,
};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tokio::sync::{mpsc, Semaphore};
use tower::ServiceExt;

const MODEL: &str = "fake-model-1";
/// 测试令牌 (强制态过门用; 形似凭据以便泄漏显形, 非真实秘密)。
const TOKEN: &str = "multi-client-token-0123456789abcdef";
const TOKEN_A: &str = "tok-desktop-a-0123456789abcdef";
const TOKEN_B: &str = "tok-desktop-b-0123456789abcdef";

// ---------------------------------------------------------------------------
// 确定性夹具 (provider / 插件 / 审计档案)
// ---------------------------------------------------------------------------

fn test_clock() -> Arc<VirtualClock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

fn response_for(call: usize) -> NormalizedResponse {
    NormalizedResponse {
        id: format!("response-{call}"),
        model: MODEL.to_string(),
        content: format!("reply-{call}"),
        finish_reason: Some(NormalizedFinishReason::Stop),
        usage: NormalizedUsage::new(3, 2),
        tool_calls: Vec::new(),
        raw_metadata: serde_json::Map::new(),
    }
}

/// 确定性纯文本 provider: 每轮固定收尾文本 (回合消息数可预测)。
struct PlainProvider {
    id: CapabilityId,
    calls: AtomicUsize,
}

#[async_trait]
impl ProviderCapability for PlainProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new(MODEL).unwrap(),
            self.id.clone(),
        )]
    }

    async fn complete(
        &self,
        _request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(response_for(call))
    }
}

/// 可停闸 provider: 进场即报到, 除非测试放行, 一直等 —— 回合时长由测试控制
/// (排队/交叠语义的确定性钉法)。
struct GatedProvider {
    id: CapabilityId,
    calls: AtomicUsize,
    entered: mpsc::UnboundedSender<usize>,
    release: Arc<Semaphore>,
}

#[async_trait]
impl ProviderCapability for GatedProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new(MODEL).unwrap(),
            self.id.clone(),
        )]
    }

    async fn complete(
        &self,
        _request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.send(call).ok();
        let permit = self.release.acquire().await.expect("release channel open");
        permit.forget();
        Ok(response_for(call))
    }
}

/// 夹具插件: 只挂一个 provider。
struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<dyn ProviderCapability>,
}

#[async_trait]
impl Plugin for ProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider)]
    }
}

fn provider_plugin(provider: Arc<dyn ProviderCapability>) -> ProviderPlugin {
    ProviderPlugin {
        manifest: PluginManifest::new(
            PluginId::new("test.multi_client_provider").unwrap(),
            "1.0.0",
            "deterministic provider",
        )
        .declare_capability(
            provider.id().clone(),
            CapabilityKind::Provider,
            "deterministic completion provider",
        )
        .unwrap(),
        provider,
    }
}

/// 审计档案 fake: 记录观测攒批面落下的每一条事实。
#[derive(Debug, Default)]
struct RecordingAudit {
    facts: Mutex<Vec<(String, Option<String>)>>,
}

impl RecordingAudit {
    fn details_of(&self, event: &str) -> Vec<String> {
        self.facts
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| name == event)
            .filter_map(|(_, detail)| detail.clone())
            .collect()
    }
}

#[async_trait]
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
    store: Arc<dyn SessionStore>,
    router: axum::Router,
}

/// 一份网关装配: 真运行时 + 强制态令牌门 + 显式客户端令牌表。
async fn harness(provider: Arc<dyn ProviderCapability>, clients: &[(&str, &str)]) -> Harness {
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(test_clock())
            .with_session_store(Arc::clone(&store))
            .with_governance(Arc::new(AllowAll))
            .with_plugin(Arc::new(provider_plugin(provider)))
            .with_default_model(MODEL)
            .build()
            .await
            .unwrap(),
    );
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(Arc::clone(&audit) as Arc<dyn AuditCommand>),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.to_string());
    state.client_tokens = table(clients);
    let router = canonical_router_with_state(state.clone());
    Harness {
        state,
        audit,
        store,
        router,
    }
}

fn table(entries: &[(&str, &str)]) -> ClientTokenTable {
    ClientTokenTable::from_config(ClientTokenConfig::new(
        entries
            .iter()
            .map(|(name, token)| ClientTokenSpec {
                name: (*name).to_string(),
                token: (*token).to_string(),
                user_id: None,
            })
            .collect(),
    ))
    .expect("valid table")
}

fn plain() -> Arc<dyn ProviderCapability> {
    Arc::new(PlainProvider {
        id: CapabilityId::new("provider.fake").unwrap(),
        calls: AtomicUsize::new(0),
    })
}

fn chat_body(session: SessionId, input: &str) -> Body {
    Body::from(
        serde_json::to_vec(&serde_json::json!({
            "session": session,
            "input": input,
        }))
        .expect("body"),
    )
}

fn request(method: &str, uri: &str, token: Option<&str>, body: Body) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body)
        .expect("request");
    if let Some(token) = token {
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {token}").parse().expect("bearer"),
        );
    }
    request
}

fn with_claim(mut request: Request<Body>, claimed: &str) -> Request<Body> {
    request.headers_mut().insert(
        CLIENT_NAME_CLAIM_HEADER,
        claimed.parse().expect("claim header"),
    );
    request
}

/// 收一个并发任务的响应 (10 秒兜底: 队列语义若死锁, 测试要报死而不是挂死)。
async fn finish(
    job: tokio::task::JoinHandle<axum::http::Response<Body>>,
) -> axum::http::Response<Body> {
    tokio::time::timeout(Duration::from_secs(10), job)
        .await
        .expect("request finished within 10s (排队语义不得死锁)")
        .expect("request task not panicked")
}

async fn status_and_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn wait_writers_in_flight(state: &GatewayState, session: SessionId, want: usize) {
    for _ in 0..400 {
        if state
            .runtime
            .sessions()
            .write_gate()
            .writers_in_flight(&session)
            .await
            == want
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("writers_in_flight never reached {want}");
}

/// 读 SSE 帧直到 `marker` 出现 (或 5 秒超时)。
async fn collect_sse_until(response: axum::response::Response, marker: &str) -> String {
    let mut stream = http_body_util::BodyStream::new(response.into_body());
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let next =
            tokio::time::timeout_at(deadline, tokio_stream::StreamExt::next(&mut stream)).await;
        match next {
            Ok(Some(Ok(frame))) => {
                let bytes = frame.into_data().unwrap_or_default();
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if collected.contains(marker) {
                    break;
                }
            }
            Ok(Some(Err(_))) => {}
            Ok(None) => break,
            Err(_) => panic!("timed out waiting for SSE marker {marker:?}; got {collected:?}"),
        }
    }
    collected
}

fn message_texts(session: &Session) -> Vec<String> {
    session
        .messages
        .iter()
        .map(|message| ContentPart::join_text(&message.content))
        .collect()
}

// ---------------------------------------------------------------------------
// 1. 事件扇出: 每客户端独立订阅, 同会话事件扇出多端, 会话间隔离
// ---------------------------------------------------------------------------

#[tokio::test]
async fn two_clients_on_one_session_each_get_the_sessions_event_fan_out() {
    let bus = apeireth_gateway::EventBus::new(16);
    let s = SessionId::new();
    let t = SessionId::new();
    let mut desktop = bus.subscribe_session(s);
    let mut tui = bus.subscribe_session(s);
    let mut other = bus.subscribe_session(t);
    assert_eq!(bus.session_connections(&s), 2, "两客户端独立订阅同一会话");
    assert_eq!(desktop.connections(), 2);

    bus.publish(apeireth_gateway::GatewayEvent::new(
        "turn_started",
        serde_json::json!({ "session": s, "marker": "alpha" }),
    ));
    bus.publish(apeireth_gateway::GatewayEvent::new(
        "turn_started",
        serde_json::json!({ "session": t, "marker": "beta" }),
    ));

    // 同会话事件扇出多端: 两个订阅端都收到 s 的帧。
    let frame = desktop.recv().await.unwrap().expect("frame");
    assert_eq!(frame.data["marker"], "alpha");
    let frame = tui.recv().await.unwrap().expect("frame");
    assert_eq!(frame.data["marker"], "alpha");

    // 会话隔离: t 的帧只进 t 的订阅端。
    let frame = other.recv().await.unwrap().expect("frame");
    assert_eq!(frame.data["marker"], "beta");
    assert!(
        tokio::time::timeout(Duration::from_millis(100), desktop.recv())
            .await
            .is_err(),
        "s 的订阅端不得收到 t 的帧 (会话过滤)"
    );

    // 一端断开不动另一端 (独立订阅): 留下的客户端照收扇出。
    drop(tui);
    assert_eq!(bus.session_connections(&s), 1, "退订即回收计数");
    bus.publish(apeireth_gateway::GatewayEvent::new(
        "turn_completed",
        serde_json::json!({ "session": s, "marker": "gamma" }),
    ));
    let frame = desktop.recv().await.unwrap().expect("frame");
    assert_eq!(frame.data["marker"], "gamma", "剩余订阅端照收扇出");

    // 会话帧面: 订阅期间留存最近一帧, 全员退订即不留存、不回放。
    assert_eq!(
        desktop.snapshot().expect("snapshot").data["marker"],
        "gamma"
    );
    drop(desktop);
    assert_eq!(bus.session_connections(&s), 0);
    assert!(
        bus.session_snapshot(&s).is_none(),
        "无订阅者即无帧面留存 (有界, 不回放)"
    );
}

#[tokio::test]
async fn the_sse_endpoint_can_scope_a_connection_to_one_session() {
    let harness = harness(plain(), &[("tui", "tok-tui-0123456789abcdef")]).await;
    let s = SessionId::new();
    let t = SessionId::new();
    let router = harness.router.clone();

    let response = router
        .oneshot(request(
            "GET",
            &format!("/v1/apeireth/events?session={s}"),
            Some("tok-tui-0123456789abcdef"),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 他端 (无过滤) 的帧与他会话的帧都不进本连接; 本会话帧按序到。
    harness
        .state
        .events
        .publish(apeireth_gateway::GatewayEvent::new(
            "turn_started",
            serde_json::json!({ "session": t, "marker": "beta" }),
        ));
    harness
        .state
        .events
        .publish(apeireth_gateway::GatewayEvent::new(
            "turn_started",
            serde_json::json!({ "session": s, "marker": "alpha-1" }),
        ));
    harness
        .state
        .events
        .publish(apeireth_gateway::GatewayEvent::new(
            "turn_completed",
            serde_json::json!({ "session": s, "marker": "alpha-2" }),
        ));

    let collected = collect_sse_until(response, "alpha-2").await;
    assert!(collected.contains("alpha-1"), "{collected}");
    assert!(
        !collected.contains("beta"),
        "会话过滤: 他会话帧不得进本连接: {collected}"
    );

    // 订阅动作落会话归属审计 (客户端名 ↔ 会话)。
    harness.state.observations.flush().await;
    let rows = harness.audit.details_of("gateway.session_client");
    assert!(
        rows.iter().any(|row| row.contains(&format!("session={s}"))
            && row.contains("client=tui")
            && row.contains("action=events.subscribe")),
        "订阅审计必须带客户端名与会话: {rows:?}"
    );
}

#[tokio::test]
async fn an_invalid_scoped_session_id_is_refused_with_an_explicit_frame() {
    let harness = harness(plain(), &[]).await;
    let response = harness
        .router
        .clone()
        .oneshot(request(
            "GET",
            "/v1/apeireth/events?session=not-a-uuid",
            Some(TOKEN),
            Body::empty(),
        ))
        .await
        .unwrap();
    let (status, text) = status_and_body(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{text}");
    assert!(text.contains("invalid_request"), "{text}");
}

// ---------------------------------------------------------------------------
// 2. 同会话并发回合: 排队 + 审计 (不发 busy 帧)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_second_turn_on_a_busy_session_queues_instead_of_being_rejected() {
    let (entered_tx, mut entered_rx) = mpsc::unbounded_channel();
    let release = Arc::new(Semaphore::new(0));
    let provider = Arc::new(GatedProvider {
        id: CapabilityId::new("provider.fake").unwrap(),
        calls: AtomicUsize::new(0),
        entered: entered_tx,
        release: Arc::clone(&release),
    });
    let harness = harness(provider, &[("desktop-a", TOKEN_A), ("desktop-b", TOKEN_B)]).await;
    let session = SessionId::new();
    let router = harness.router.clone();

    // 客户端 A 先发: 回合进入 provider (持会话写闸)。
    let turn_a = tokio::spawn({
        let router = router.clone();
        async move {
            router
                .oneshot(request(
                    "POST",
                    "/v1/chat",
                    Some(TOKEN_A),
                    chat_body(session, "alpha"),
                ))
                .await
                .unwrap()
        }
    });
    tokio::time::timeout(Duration::from_secs(5), entered_rx.recv())
        .await
        .expect("turn A entered the provider (5s 内报到)")
        .expect("entry signal");
    // 客户端 B 同会话再发: 排队 (在场计数 2), 不被 busy 帧拒绝。
    let turn_b = tokio::spawn({
        let router = router.clone();
        async move {
            router
                .oneshot(request(
                    "POST",
                    "/v1/chat",
                    Some(TOKEN_B),
                    chat_body(session, "bravo"),
                ))
                .await
                .unwrap()
        }
    });
    wait_writers_in_flight(&harness.state, session, 2).await;
    assert!(
        !turn_b.is_finished(),
        "第二回合必须排队等第一个跑完 (排队语义), 不是立即返回 busy"
    );

    // 放行两个回合: 先后完成, 全部 200 (没有 busy 帧)。
    release.add_permits(2);
    let (status_a, body_a) = status_and_body(finish(turn_a).await).await;
    let (status_b, body_b) = status_and_body(finish(turn_b).await).await;
    assert_eq!(status_a, StatusCode::OK, "{body_a}");
    assert_eq!(
        status_b,
        StatusCode::OK,
        "排队回合最终完成, 不是 4xx busy: {body_b}"
    );

    // 会话账本: 两个回合按入场次序连续落账 (alpha 的整回合在 bravo 之前)。
    let stored = harness
        .store
        .load(&session)
        .await
        .unwrap()
        .expect("session");
    assert_eq!(
        message_texts(&stored),
        vec!["alpha", "reply-0", "bravo", "reply-1"],
        "回合串行: 每回合消息连续, 次序即入场次序 (零交叠写)"
    );
    let appended = stored
        .log
        .iter()
        .filter(|entry| matches!(entry.event, SessionEventKind::MessageAppended { .. }))
        .count();
    assert_eq!(appended, 4, "账本与转写 lockstep");

    // 排队即审计: B 的 `gateway.session_client` 行带 queued=1 与排队位次。
    harness.state.observations.flush().await;
    let rows = harness.audit.details_of("gateway.session_client");
    let turn_rows: Vec<&String> = rows
        .iter()
        .filter(|row| row.contains("action=turn"))
        .collect();
    assert_eq!(turn_rows.len(), 2, "两个回合各一行归属审计: {rows:?}");
    assert!(
        turn_rows
            .iter()
            .any(|row| row.contains("client=desktop-a") && row.contains("queued=0")),
        "{rows:?}"
    );
    assert!(
        turn_rows.iter().any(|row| row.contains("client=desktop-b")
            && row.contains("queued=1")
            && row.contains("position=1")),
        "排队事实必须留痕 (排队+审计): {rows:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. 会话账本零交叠写 (对抗并发压测)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn concurrent_turns_and_settings_writes_never_interleave_the_session_ledger() {
    let harness = harness(plain(), &[("desktop-a", TOKEN_A), ("desktop-b", TOKEN_B)]).await;
    let session = SessionId::new();
    // 会话先落账 (补丁与回合谁先到都不该 404, 排队语义只管写次序)。
    harness
        .store
        .save(&Session::new(session, test_clock().as_ref()))
        .await
        .unwrap();
    let router = harness.router.clone();

    let mut jobs = Vec::new();
    // 6 个回合 (两个客户端各 3 个) + 3 个设置补丁同时砸同一个会话。
    for n in 0..6 {
        let router = router.clone();
        let token = if n % 2 == 0 { TOKEN_A } else { TOKEN_B };
        jobs.push(tokio::spawn(async move {
            router
                .oneshot(request(
                    "POST",
                    "/v1/chat",
                    Some(token),
                    chat_body(session, &format!("msg-{n}")),
                ))
                .await
                .unwrap()
        }));
    }
    for _ in 0..3 {
        let router = router.clone();
        jobs.push(tokio::spawn(async move {
            router
                .oneshot(request(
                    "PATCH",
                    &format!("/v1/sessions/{session}/settings"),
                    Some(TOKEN_A),
                    Body::from(r#"{"model":"fake-model-1","approval_remember":true}"#.to_string()),
                ))
                .await
                .unwrap()
        }));
    }
    for job in jobs {
        let (status, text) = status_and_body(finish(job).await).await;
        assert_eq!(status, StatusCode::OK, "{text}");
    }

    let stored = harness
        .store
        .load(&session)
        .await
        .unwrap()
        .expect("session");
    assert_eq!(
        stored.messages.len(),
        12,
        "6 回合 × (user + assistant) 一条不丢"
    );
    for pair in stored.messages.chunks(2) {
        assert_eq!(pair.len(), 2);
        assert_eq!(
            pair[0].role,
            MessageRole::User,
            "回合内的 user 紧邻自己的回复"
        );
        assert_eq!(pair[1].role, MessageRole::Assistant);
        assert!(
            ContentPart::join_text(&pair[0].content).starts_with("msg-"),
            "每个回合的 user 消息紧邻自己的回复 (零交叠)"
        );
    }
    let appended_seqs: Vec<usize> = stored
        .log
        .iter()
        .filter_map(|entry| match &entry.event {
            SessionEventKind::MessageAppended { seq, .. } => Some(*seq),
            _ => None,
        })
        .collect();
    assert_eq!(
        appended_seqs,
        (0..12).collect::<Vec<_>>(),
        "账本追加序号连续 (零交叠写)"
    );
    assert!(
        stored.revision >= 12,
        "revision 单调前进, 无回退覆写: {}",
        stored.revision
    );

    // 设置补丁的写不被回合覆写 (读改写全程持证, 零交叠)。
    assert_eq!(
        stored.settings.model.as_deref(),
        Some("fake-model-1"),
        "设置改动不得被在途回合的保存吃掉"
    );
    assert!(stored.settings.approval_remember, "同上");

    // 归属审计: 6 个回合 + 3 个补丁各有 `gateway.session_client` 行。
    harness.state.observations.flush().await;
    let rows = harness.audit.details_of("gateway.session_client");
    assert_eq!(
        rows.iter()
            .filter(|row| row.contains("action=turn") && row.contains(&format!("session={session}")))
            .count(),
        6,
        "{rows:?}"
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.contains("action=settings.patch")
                && row.contains(&format!("session={session}")))
            .count(),
        3,
        "{rows:?}"
    );
}

/// 回合半途的设置补丁与回合的账本写互不吞并 (丢更新的确定性钉法)。
#[tokio::test]
async fn a_settings_patch_during_a_running_turn_preserves_both_writes() {
    let (entered_tx, mut entered_rx) = mpsc::unbounded_channel();
    let release = Arc::new(Semaphore::new(0));
    let provider = Arc::new(GatedProvider {
        id: CapabilityId::new("provider.fake").unwrap(),
        calls: AtomicUsize::new(0),
        entered: entered_tx,
        release: Arc::clone(&release),
    });
    let harness = harness(provider, &[("desktop-a", TOKEN_A)]).await;
    let session = SessionId::new();
    let router = harness.router.clone();

    let turn = tokio::spawn({
        let router = router.clone();
        async move {
            router
                .oneshot(request(
                    "POST",
                    "/v1/chat",
                    Some(TOKEN_A),
                    chat_body(session, "alpha"),
                ))
                .await
                .unwrap()
        }
    });
    tokio::time::timeout(Duration::from_secs(5), entered_rx.recv())
        .await
        .expect("turn entered the provider (5s 内报到)")
        .expect("entry signal");

    // 回合半途的设置补丁: 排队等回合, 不与回合交叠写账本。
    let patch = tokio::spawn({
        let router = router.clone();
        async move {
            router
                .oneshot(request(
                    "PATCH",
                    &format!("/v1/sessions/{session}/settings"),
                    Some(TOKEN_A),
                    Body::from(r#"{"model":"fake-model-1","approval_remember":true}"#.to_string()),
                ))
                .await
                .unwrap()
        }
    });
    wait_writers_in_flight(&harness.state, session, 2).await;

    release.add_permits(1);
    let (turn_status, turn_body) = status_and_body(finish(turn).await).await;
    let (patch_status, patch_body) = status_and_body(finish(patch).await).await;
    assert_eq!(turn_status, StatusCode::OK, "{turn_body}");
    assert_eq!(patch_status, StatusCode::OK, "{patch_body}");

    let stored = harness
        .store
        .load(&session)
        .await
        .unwrap()
        .expect("session");
    assert_eq!(
        message_texts(&stored),
        vec!["alpha", "reply-0"],
        "回合的账本写完整 (补丁不吃掉消息)"
    );
    assert_eq!(
        stored.settings.model.as_deref(),
        Some("fake-model-1"),
        "补丁的写完整 (回合不覆写设置) —— 读改写全程持证即零交叠"
    );
    assert!(stored.settings.approval_remember);
}

// ---------------------------------------------------------------------------
// 4/5. 会话归属 (gateway.session_client) 与令牌隔离 (name↔token 绑定)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_gateway_session_client_audit_binds_the_client_name_to_the_session() {
    let harness = harness(plain(), &[("tui", "tok-tui-0123456789abcdef")]).await;
    let session = SessionId::new();

    let (status, _) = status_and_body(
        harness
            .router
            .clone()
            .oneshot(request(
                "POST",
                "/v1/chat",
                Some("tok-tui-0123456789abcdef"),
                chat_body(session, "hello"),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    harness.state.observations.flush().await;
    let rows = harness.audit.details_of("gateway.session_client");
    let turn_row = rows
        .iter()
        .find(|row| row.contains("action=turn") && row.contains(&format!("session={session}")))
        .unwrap_or_else(|| panic!("缺会话归属审计行: {rows:?}"));
    assert!(
        turn_row.contains("client=tui"),
        "归属行必须署令牌绑定的客户端名: {turn_row}"
    );

    // 与会话事件关联: 同一 session id 出现在回合事件审计行里。
    let turn_events = harness.audit.details_of("chat.turn.completed");
    assert!(
        turn_events
            .iter()
            .any(|row| row.contains(&format!("session={session}"))),
        "归属行与回合事件共享会话键: {turn_events:?}"
    );
}

#[tokio::test]
async fn a_client_cannot_claim_another_clients_audit_name() {
    let harness = harness(plain(), &[("desktop-a", TOKEN_A), ("desktop-b", TOKEN_B)]).await;
    let session = SessionId::new();
    let router = harness.router.clone();

    // A 的令牌 + 自称 B: name↔token 绑定不一致 → 401 (冒名 fail-closed)。
    let forged = with_claim(
        request(
            "POST",
            "/v1/chat",
            Some(TOKEN_A),
            chat_body(session, "spoof"),
        ),
        "desktop-b",
    );
    let (status, text) = status_and_body(router.clone().oneshot(forged).await.unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_invalid_key"), "{text}");
    assert!(!text.contains(TOKEN_A), "401 帧不得回显令牌: {text}");

    // A 不自称: 署名只能是令牌绑定的 A。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "POST",
                "/v1/chat",
                Some(TOKEN_A),
                chat_body(session, "honest"),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    harness.state.observations.flush().await;
    let rejected = harness.audit.details_of("gateway.auth_rejected");
    assert!(
        rejected
            .iter()
            .any(|row| row.contains("reason=client_name_mismatch")
                && row.contains("client=desktop-a")
                && row.contains("claimed=desktop-b")),
        "冒名拒绝必须留痕: {rejected:?}"
    );
    let rows = harness.audit.details_of("gateway.session_client");
    assert!(
        rows.iter().any(|row| row.contains("client=desktop-a")),
        "{rows:?}"
    );
    assert!(
        rows.iter().all(|row| !row.contains("client=desktop-b")),
        "冒名者的名绝不进审计署名: {rows:?}"
    );
}

#[tokio::test]
async fn a_claimed_name_without_a_token_binding_is_refused() {
    // 未设防口径: 无令牌即无绑定, 自称名不可校验 → 拒 (fail-closed)。
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let runtime = Arc::new(
        Runtime::builder()
            .with_session_store(store)
            .with_default_model(MODEL)
            .build()
            .await
            .unwrap(),
    );
    let audit = Arc::new(RecordingAudit::default());
    let services = GatewayServices {
        audit_commands: Some(Arc::clone(&audit) as Arc<dyn AuditCommand>),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::UnprotectedDefault;
    let router = canonical_router_with_state(state.clone());

    let claimed = with_claim(
        request("GET", "/v1/panel/sessions", None, Body::empty()),
        "desktop-b",
    );
    let (status, text) = status_and_body(router.oneshot(claimed).await.unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_invalid_key"), "{text}");

    state.observations.flush().await;
    let rejected = audit.details_of("gateway.auth_rejected");
    assert!(
        rejected
            .iter()
            .any(|row| row.contains("reason=client_name_unverifiable")),
        "{rejected:?}"
    );
}

// ---------------------------------------------------------------------------
// 面板一致性: /v1/panel/* 每请求一次快照读 (会话列表/内存计数)
// ---------------------------------------------------------------------------

/// 版本化会话快照 fake: 每次读返回一个自洽快照 (revision == message_count ==
/// 版本), 并计数调用 —— 响应里的行混了两个版本即撕裂读。
#[derive(Debug, Default)]
struct VersionedSessions {
    calls: AtomicUsize,
    version: AtomicUsize,
}

#[async_trait]
impl SessionQuery for VersionedSessions {
    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let version = self.version.load(Ordering::SeqCst) as u64;
        Ok(vec![SessionSummaryDto {
            id: "00000000-0000-0000-0000-000000000009".to_string(),
            title: Some("snapshot".to_string()),
            created_at: 0,
            updated_at: version as i64,
            message_count: version as usize,
            revision: version,
        }])
    }
}

/// 版本化内存计数 fake: 每次读推进版本并返回**同一版本**的 nodes/edges 计数
/// (两组计数若来自不同读, 数目就会对不上)。
#[derive(Debug, Default)]
struct VersionedMemory {
    calls: AtomicUsize,
    version: AtomicUsize,
}

#[async_trait]
impl MemoryQuery for VersionedMemory {
    async fn list_episodes(
        &self,
        _session: Option<&str>,
        _query: Option<&str>,
        _limit: usize,
    ) -> Result<Vec<EpisodeDto>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }

    async fn memory_graph(&self) -> Result<MemoryGraphDto, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let version = self.version.fetch_add(1, Ordering::SeqCst) + 1;
        let nodes = (0..version)
            .map(|n| GraphNodeDto {
                id: format!("n-{n}"),
                label: format!("n-{n}"),
                kind: "episode".to_string(),
            })
            .collect();
        let edges = (0..version)
            .map(|n| GraphEdgeDto {
                from: format!("n-{n}"),
                to: format!("n-{n}"),
                weight: 1.0,
                label: None,
            })
            .collect();
        Ok(MemoryGraphDto { nodes, edges })
    }
}

#[tokio::test]
async fn panel_session_reads_are_one_consistent_snapshot_per_request() {
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let runtime = Arc::new(
        Runtime::builder()
            .with_session_store(store)
            .with_default_model(MODEL)
            .build()
            .await
            .unwrap(),
    );
    let sessions = Arc::new(VersionedSessions::default());
    let services = GatewayServices {
        sessions: Some(Arc::clone(&sessions) as Arc<dyn SessionQuery>),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.to_string());
    let router = canonical_router_with_state(state.clone());

    // 写入不息 (版本推进), 两个客户端并发读: 每个响应的行必须自洽
    // (revision == message_count == 同一版本), 且每请求恰好一次快照读。
    sessions.version.store(7, Ordering::SeqCst);
    let mut readers = Vec::new();
    for _ in 0..2 {
        let router = router.clone();
        readers.push(tokio::spawn(async move {
            router
                .oneshot(request(
                    "GET",
                    "/v1/panel/sessions",
                    Some(TOKEN),
                    Body::empty(),
                ))
                .await
                .unwrap()
        }));
    }
    for reader in readers {
        let (status, text) = status_and_body(finish(reader).await).await;
        assert_eq!(status, StatusCode::OK, "{text}");
        let body: serde_json::Value = serde_json::from_str(&text).unwrap();
        let row = &body["sessions"][0];
        assert_eq!(
            row["revision"], row["message_count"],
            "一行即一版快照 (无撕裂读): {text}"
        );
        assert_eq!(row["revision"], 7, "静止窗口两客户端读数一致: {text}");
    }
    assert_eq!(
        sessions.calls.load(Ordering::SeqCst),
        2,
        "每请求恰好一次快照读 (不拼两次读)"
    );

    // 版本推进后, 新请求读到新快照 (读数单调前进, 不回退)。
    sessions.version.store(9, Ordering::SeqCst);
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/sessions",
                Some(TOKEN),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(text.contains("\"revision\":9"), "{text}");
}

#[tokio::test]
async fn panel_memory_counts_are_read_as_one_snapshot_per_request() {
    let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
    let runtime = Arc::new(
        Runtime::builder()
            .with_session_store(store)
            .with_default_model(MODEL)
            .build()
            .await
            .unwrap(),
    );
    let memory = Arc::new(VersionedMemory::default());
    let services = GatewayServices {
        memory: Some(Arc::clone(&memory) as Arc<dyn MemoryQuery>),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.to_string());
    let router = canonical_router_with_state(state.clone());

    // 每次读推进版本: 一次请求只许读一次 —— nodes/edges 计数若混了两个版本
    // 的读, 两组计数就对不上。
    for _ in 0..3 {
        let (status, text) = status_and_body(
            router
                .clone()
                .oneshot(request(
                    "GET",
                    "/v1/panel/graph",
                    Some(TOKEN),
                    Body::empty(),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{text}");
        let body: serde_json::Value = serde_json::from_str(&text).unwrap();
        let nodes = body["nodes"].as_array().expect("nodes").len();
        let edges = body["edges"].as_array().expect("edges").len();
        assert_eq!(nodes, edges, "内存计数同出一版快照 (一次读): {text}");
    }
    assert_eq!(
        memory.calls.load(Ordering::SeqCst),
        3,
        "每请求恰好一次快照读"
    );
}

// ---------------------------------------------------------------------------
// 干净收尾: 会话关闭面 (身份类型在请求扩展中下行) 的最小钉
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_gate_passes_the_token_bound_identity_down_to_handlers() {
    // 绑定面的直接证据: 处理器拿到的署名键就是令牌绑定名 (不是请求自称)。
    let identity = ClientIdentity::authenticated("tui");
    assert!(identity.authenticated);
    assert_eq!(identity.client, "tui");
    assert!(!ClientIdentity::anonymous().authenticated);
    assert_eq!(ClientIdentity::UNATTRIBUTED, "unattributed");
}
