//! W3-③ 星群正身 (逐域伙伴实例): 域 chat 面开放 + 按域运行时上下文的全链证据。
//!
//! 钉死的行为面:
//! 1. **域 chat 解禁全链**: 非默认域 `/v1/chat/completions` 走该域自己的伙伴
//!    运行时实例 —— 模型/记忆/配额/写闸全按域 (会话落本域账本, 配额记本域);
//! 2. **跨域配置互不影响 (对抗)**: 域 A 的模型配置改动不改变域 B 的回复;
//! 3. **域级性格/人设**: 域文档 persona 覆盖注入回合种子, 缺省逐字回落请求
//!    system (全局行为);
//! 4. **域事件过滤**: 域订阅只收本域在册会话的事件帧, 他域帧不投递;
//! 5. **默认域零回归**: 无覆盖时默认域 chat 行为与既有语义逐字一致;
//! 6. **坏域配置拒开**: 坏域 chat = 403 `domain_refused`, 他域不受牵连。

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{system_clock, CapabilityId, ModelId, PluginId, SessionId};
use apeireth_core::user_domain::{DomainRegistry, DomainRole, DomainUserId};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, ClientTokenConfig,
    ClientTokenSpec, ClientTokenTable, GatewayServices, GatewayState, GatewayTokenGate,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedUsage,
};
use apeireth_runtime::canonical::{
    DomainRuntimePlane, DomainSessionPlane, InMemorySessionStore, Runtime, Session, SessionManager,
};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const GATE_TOKEN: &str = "gate-token-0123456789abcdef";
const ALICE_TOKEN: &str = "alice-token-0123456789abcdef";
const BOB_TOKEN: &str = "bob-token-0123456789abcdef";
const STRANGER_TOKEN: &str = "stranger-token-0123456789abcdef";

const ALICE_CLIENT: &str = "tui";
const BOB_CLIENT: &str = "desk";
const STRANGER_CLIENT: &str = "stranger";

const MODEL_ALICE: &str = "model-alice";
const MODEL_BOB: &str = "model-bob";
const MODEL_GLOBAL: &str = "global-model";
const ALICE_PERSONA: &str = "ALICE-PERSONA-TEXT-温润如玉";
const CALLER_SYSTEM: &str = "CALLER-SYSTEM";

/// 录证 provider: 记录 (模型, 各消息文本) 并回显所服务的模型 ——
/// 「域 A 的模型配置不影响域 B 的回复」直接可读。
struct RecordingProvider {
    id: CapabilityId,
    calls: Mutex<Vec<(String, Vec<String>)>>,
}

impl RecordingProvider {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.recording").unwrap(),
            calls: Mutex::new(Vec::new()),
        })
    }

    fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProviderCapability for RecordingProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        [MODEL_ALICE, MODEL_BOB, MODEL_GLOBAL]
            .iter()
            .map(|model| ModelDescriptor::new(ModelId::new(*model).unwrap(), self.id.clone()))
            .collect()
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let texts = request
            .messages
            .iter()
            .map(|message| ContentPart::join_text(&message.content))
            .collect::<Vec<_>>();
        self.calls
            .lock()
            .unwrap()
            .push((request.model.clone(), texts));
        Ok(NormalizedResponse {
            id: "response-1".into(),
            model: request.model.clone(),
            content: format!("served:{}", request.model),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

/// 夹具插件: 只挂录证 provider (同一实例跨运行时共享)。
struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<RecordingProvider>,
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
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

fn manager() -> Arc<SessionManager> {
    Arc::new(SessionManager::new(
        Arc::new(InMemorySessionStore::new()),
        system_clock(),
    ))
}

async fn runtime_with(
    provider: Arc<RecordingProvider>,
    sessions: Arc<SessionManager>,
) -> Arc<Runtime> {
    Arc::new(
        Runtime::builder()
            .with_plugin(Arc::new(ProviderPlugin {
                manifest: PluginManifest::new(
                    PluginId::new("test.domain_partners").unwrap(),
                    "1.0.0",
                    "recording provider",
                )
                .declare_capability(
                    CapabilityId::new("provider.recording").unwrap(),
                    CapabilityKind::Provider,
                    "recording completion provider",
                )
                .unwrap(),
                provider,
            }))
            .with_session_manager(sessions)
            .with_default_model(MODEL_GLOBAL)
            .build()
            .await
            .unwrap(),
    )
}

struct Harness {
    state: GatewayState,
    provider: Arc<RecordingProvider>,
    alice_sessions: Arc<SessionManager>,
    bob_sessions: Arc<SessionManager>,
    registry: DomainRegistry,
}

/// 一份星群装配: 默认域 = 现运行时 (全局模型), alice/bob 各持伙伴运行时 +
/// 域文档覆盖 (alice: persona + model; bob: 仅 model)。用户表未配置
/// (角色面未激活, 本套件只判域)。
async fn harness() -> Harness {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "apeireth-domain-partner-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let registry = DomainRegistry::open(&root).unwrap();
    registry
        .create("user-alice", "Alice", DomainRole::Owner)
        .unwrap();
    registry
        .create("user-bob", "Bob", DomainRole::Member)
        .unwrap();
    let alice_id = DomainUserId::new("user-alice").unwrap();
    let bob_id = DomainUserId::new("user-bob").unwrap();
    registry
        .set_runtime_overrides(
            &alice_id,
            Some(ALICE_PERSONA.to_string()),
            Some(MODEL_ALICE.to_string()),
        )
        .unwrap();
    registry
        .set_runtime_overrides(&bob_id, None, Some(MODEL_BOB.to_string()))
        .unwrap();

    let provider = RecordingProvider::new();
    let alice_sessions = manager();
    let bob_sessions = manager();
    let runtime = runtime_with(Arc::clone(&provider), manager()).await;

    let mut managers = BTreeMap::new();
    managers.insert(alice_id, Arc::clone(&alice_sessions));
    managers.insert(bob_id, Arc::clone(&bob_sessions));
    managers.insert(
        apeireth_core::user_domain::DomainUserId::default_domain(),
        runtime.sessions_arc().clone(),
    );
    let sessions_plane = Arc::new(DomainSessionPlane::new(registry.clone(), managers));

    let mut instances = BTreeMap::new();
    instances.insert(
        DomainUserId::new("user-alice").unwrap(),
        runtime_with(Arc::clone(&provider), Arc::clone(&alice_sessions)).await,
    );
    instances.insert(
        DomainUserId::new("user-bob").unwrap(),
        runtime_with(Arc::clone(&provider), Arc::clone(&bob_sessions)).await,
    );
    let runtime_plane =
        DomainRuntimePlane::new(Arc::clone(&sessions_plane), Arc::clone(&runtime), instances);

    let mut state = build_gateway_state_with_services(runtime, GatewayServices::default());
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens = ClientTokenTable::from_config(ClientTokenConfig::new(vec![
        ClientTokenSpec {
            name: ALICE_CLIENT.to_string(),
            token: ALICE_TOKEN.to_string(),
            user_id: Some("user-alice".to_string()),
        },
        ClientTokenSpec {
            name: BOB_CLIENT.to_string(),
            token: BOB_TOKEN.to_string(),
            user_id: Some("user-bob".to_string()),
        },
        ClientTokenSpec {
            name: STRANGER_CLIENT.to_string(),
            token: STRANGER_TOKEN.to_string(),
            user_id: None,
        },
    ]))
    .expect("valid token table");
    state.domains = Some(sessions_plane);
    state.domain_runtimes = Some(Arc::new(runtime_plane));
    state.attach_domain_runtime_events();

    Harness {
        state,
        provider,
        alice_sessions,
        bob_sessions,
        registry,
    }
}

fn chat_request(token: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn chat_body(model: Option<&str>, system: Option<&str>) -> serde_json::Value {
    let mut messages = Vec::new();
    if let Some(system) = system {
        messages.push(serde_json::json!({"role": "system", "content": system}));
    }
    messages.push(serde_json::json!({"role": "user", "content": "你好"}));
    let mut body = serde_json::json!({ "messages": messages });
    if let Some(model) = model {
        body["model"] = serde_json::json!(model);
    }
    body
}

async fn router(harness: &Harness) -> axum::Router {
    canonical_router_with_state(harness.state.clone())
}

async fn send(router: &axum::Router, token: &str, body: serde_json::Value) -> serde_json::Value {
    let response = router
        .clone()
        .oneshot(chat_request(token, body))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

// ---------------------------------------------------------------------------
// 域 chat 解禁全链 (模型/记忆/配额/写闸全按域)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_default_domains_serve_chat_on_their_own_partner_runtime() {
    let harness = harness().await;
    let router = router(&harness).await;

    let value = send(&router, ALICE_TOKEN, chat_body(None, None)).await;

    // 模型面: 未显式点名模型 → 域覆盖模型 (而非全局默认/他域模型)。
    assert_eq!(value["model"], MODEL_ALICE);
    assert_eq!(
        value["choices"][0]["message"]["content"],
        format!("served:{MODEL_ALICE}")
    );
    let (model, _) = harness.provider.calls().last().unwrap().clone();
    assert_eq!(model, MODEL_ALICE, "伙伴回合必须跑在域模型上");

    // 写闸/配额面: 会话落**本域**账本, 配额记本域; 他域账本查无此会话。
    let session: SessionId = value["apeireth"]["session_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let in_alice = harness
        .alice_sessions
        .load(&session)
        .await
        .unwrap()
        .expect("会话必须落本域账本");
    assert!(in_alice.quota.consumed_tokens > 0, "配额必须记在本域账本上");
    assert!(
        harness.bob_sessions.load(&session).await.unwrap().is_none(),
        "本域会话不得出现在他域账本"
    );
}

// ---------------------------------------------------------------------------
// 跨域配置互不影响 (对抗)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn one_domains_model_config_never_changes_another_domains_reply() {
    let harness = harness().await;
    let router = router(&harness).await;

    // 两域各按自己的模型服务。
    let alice = send(&router, ALICE_TOKEN, chat_body(None, None)).await;
    let bob = send(&router, BOB_TOKEN, chat_body(None, None)).await;
    assert_eq!(
        alice["choices"][0]["message"]["content"],
        format!("served:{MODEL_ALICE}")
    );
    assert_eq!(
        bob["choices"][0]["message"]["content"],
        format!("served:{MODEL_BOB}")
    );

    // 对抗: 改写 alice 的域模型覆盖, bob 的回复逐字不变。
    harness
        .registry
        .set_runtime_overrides(
            &DomainUserId::new("user-alice").unwrap(),
            Some(ALICE_PERSONA.to_string()),
            Some(MODEL_GLOBAL.to_string()),
        )
        .unwrap();
    let alice_again = send(&router, ALICE_TOKEN, chat_body(None, None)).await;
    let bob_again = send(&router, BOB_TOKEN, chat_body(None, None)).await;
    assert_eq!(
        alice_again["choices"][0]["message"]["content"],
        format!("served:{MODEL_GLOBAL}"),
        "本域改动应即刻作用于本域"
    );
    assert_eq!(
        bob_again["choices"][0]["message"]["content"],
        format!("served:{MODEL_BOB}"),
        "域 A 的模型配置不得影响域 B 的回复"
    );
}

// ---------------------------------------------------------------------------
// 域级性格/人设 (覆盖注入 / 缺省回落)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn domain_persona_seeds_the_turn_and_falls_back_verbatim_when_absent() {
    let harness = harness().await;
    let router = router(&harness).await;

    // 有 persona 的域: 人设在前, 请求 system 在后。
    send(&router, ALICE_TOKEN, chat_body(None, Some(CALLER_SYSTEM))).await;
    let (_, texts) = harness.provider.calls().last().unwrap().clone();
    let joined = texts.join("\n");
    let persona_at = joined.find(ALICE_PERSONA).expect("人设必须进回合种子");
    let system_at = joined.find(CALLER_SYSTEM).expect("请求 system 必须保留");
    assert!(
        persona_at < system_at,
        "域人设在前, 请求 system 在后: {joined}"
    );

    // 无 persona 的域: 请求 system **逐字**回落 (默认域同款行为)。
    send(&router, BOB_TOKEN, chat_body(None, Some(CALLER_SYSTEM))).await;
    let (_, texts) = harness.provider.calls().last().unwrap().clone();
    let joined = texts.join("\n");
    assert!(joined.contains(CALLER_SYSTEM));
    assert!(
        !joined.contains(ALICE_PERSONA),
        "他域人设不得渗入本域回合: {joined}"
    );
}

// ---------------------------------------------------------------------------
// 域事件过滤
// ---------------------------------------------------------------------------

async fn collect_sse_until(response: axum::response::Response, marker: &str) -> String {
    let mut stream = http_body_util::BodyStream::new(response.into_body());
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
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
            Ok(Some(Err(_))) | Ok(None) => {}
            Err(_) => panic!("timed out waiting for SSE frames; collected={collected:?}"),
        }
    }
    collected
}

#[tokio::test]
async fn domain_event_subscriptions_only_receive_their_own_domains_frames() {
    let harness = harness().await;
    let router = router(&harness).await;

    // 两域各一条在册会话 (域账本立案)。
    let alice_session = Session::new(SessionId::new(), system_clock().as_ref());
    harness.alice_sessions.save(&alice_session).await.unwrap();
    let bob_session = Session::new(SessionId::new(), system_clock().as_ref());
    harness.bob_sessions.save(&bob_session).await.unwrap();

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/apeireth/events")
                .header("authorization", format!("Bearer {ALICE_TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 先发他域帧 (应被过滤), 再发本域帧 (应投递) —— 顺序使断言确定。
    harness
        .state
        .events
        .publish(apeireth_gateway::GatewayEvent::new(
            "turn_completed",
            serde_json::json!({ "session": bob_session.id, "marker": "BOB-FRAME" }),
        ));
    harness
        .state
        .events
        .publish(apeireth_gateway::GatewayEvent::new(
            "turn_completed",
            serde_json::json!({ "session": alice_session.id, "marker": "ALICE-FRAME" }),
        ));

    let collected = collect_sse_until(response, "ALICE-FRAME").await;
    assert!(collected.contains("ALICE-FRAME"), "{collected}");
    assert!(
        !collected.contains("BOB-FRAME"),
        "他域会话的事件帧不得投递给本域订阅: {collected}"
    );
}

#[tokio::test]
async fn the_domain_claim_query_is_checked_like_the_domain_header() {
    let harness = harness().await;
    let router = router(&harness).await;

    // `?domain=` 与域头同一语义: 自称他域 (存在) 与自称未知域 (不存在) 同帧。
    let mut frames = Vec::new();
    for claimed in ["user-bob", "nobody"] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/apeireth/events?domain={claimed}"))
                    .header("authorization", format!("Bearer {ALICE_TOKEN}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["error"]["code"], "domain_forbidden");
        frames.push(bytes.to_vec());
    }
    assert_eq!(frames[0], frames[1], "自称域帧必须恒定 (不泄露存在性)");

    // 自称本域 = 放行 (200 SSE)。
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/apeireth/events?domain=user-alice")
                .header("authorization", format!("Bearer {ALICE_TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// 默认域零回归 + 坏域配置拒开
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_default_domain_chat_surface_stays_on_the_injected_runtime() {
    let harness = harness().await;
    let router = router(&harness).await;

    // 无绑定用户 = 默认域: 无域覆盖 → 全局默认模型, 行为与既有语义一致。
    let value = send(
        &router,
        STRANGER_TOKEN,
        chat_body(None, Some(CALLER_SYSTEM)),
    )
    .await;
    assert_eq!(
        value["choices"][0]["message"]["content"],
        format!("served:{MODEL_GLOBAL}")
    );
    let (_, texts) = harness.provider.calls().last().unwrap().clone();
    let joined = texts.join("\n");
    assert!(joined.contains(CALLER_SYSTEM));
    assert!(!joined.contains(ALICE_PERSONA), "默认域不得被注入他域人设");

    // 请求显式点名模型仍然优先 (既有优先级: 请求 > 域 > 全局)。
    let value = send(&router, STRANGER_TOKEN, chat_body(Some(MODEL_BOB), None)).await;
    assert_eq!(
        value["choices"][0]["message"]["content"],
        format!("served:{MODEL_BOB}")
    );
}

#[tokio::test]
async fn a_broken_domain_config_refuses_the_chat_surface_without_touching_other_domains() {
    let harness = harness().await;
    // 坏域配置 (畸形域文档) —— 该域 chat 面拒开, 真因入审计。
    std::fs::write(
        harness
            .registry
            .layout()
            .domain_file(&DomainUserId::new("user-alice").unwrap()),
        b"{ not json",
    )
    .unwrap();
    let router = router(&harness).await;

    let response = router
        .clone()
        .oneshot(chat_request(ALICE_TOKEN, chat_body(None, None)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["error"]["code"], "domain_refused");

    // 他域照常服务 (一户坏配置不砖化整面)。
    let bob = send(&router, BOB_TOKEN, chat_body(None, None)).await;
    assert_eq!(
        bob["choices"][0]["message"]["content"],
        format!("served:{MODEL_BOB}")
    );
}
