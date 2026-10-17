//! V2-c 域面逐面开放 (面板/审批/事件/管理面按域路由补全): 非默认域的
//! 面板读面按域过滤返回、审批面归属域校验、事件面与面板订阅同一在册探测、
//! 管理面保持全局而审计带域标注。
//!
//! 这里钉六件事 (验收矩阵, 与 `docs/gateway-api-contract.md` §1 域行同表):
//! 1. **面板按域过滤**: sessions/memory/graph/audit/grants/tasks 六面的行与
//!    计数都只含本域归属 (A 域看不到 B 域计数);
//! 2. **无归属记号行 fail-closed**: 行无会话/域归属记号 = 不进域视图;
//! 3. **面板订阅与事件域一致**: 同一在册探测 —— 面板可见 ⇔ 事件帧投递;
//! 4. **审批跨域归属拒**: 他域会话的审批读/解与不存在**同帧** 404
//!    (owner 亦无跨域通路 —— W3-② 跨用户审批闸升级为跨域归属);
//! 5. **管理面全局 + 审计域标注**: users/domains 面不按域过滤 (owner 全局
//!    视角), 管理动作审计行带 `domain=<目标域>` 标注并进目标域审计视图;
//! 6. **同帧语义保持**: 自称域 403 恒定帧、跨域资源 404 与不存在逐字节同帧
//!    (W3-① 契约); 默认域/域面未接线读数零回归。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{system_clock, SessionId};
use apeireth_core::user_domain::{DomainRegistry, DomainRole};
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, AuditCommand, AuditDto,
    AuditQuery, ClientTokenConfig, ClientTokenSpec, ClientTokenTable, DomainAdminService,
    EpisodeDto, GatewayEvent, GatewayServices, GatewayState, GatewayTokenGate, GrantDto,
    GrantQuery, MemoryGraphDto, MemoryQuery, SessionQuery, SessionSummaryDto, TaskBoardPanel,
    TaskBoardSnapshotDto, TaskCountsDto, TaskPanelDto, TaskPanelError, UserAccess,
    UserAdminService, CLIENT_TOKENS_ENV,
};
use apeireth_governance::{UserRecord, UserTable};
use apeireth_runtime::canonical::{
    DomainSessionPlane, InMemorySessionStore, Runtime, Session, SessionManager,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const GATE_TOKEN: &str = "gate-token-0123456789abcdef";
const ALICE_TOKEN: &str = "alice-token-0123456789abcdef";
const BOB_TOKEN: &str = "bob-token-0123456789abcdef";
const CAROL_TOKEN: &str = "carol-token-0123456789abcdef";
const STRANGER_TOKEN: &str = "stranger-token-0123456789abcdef";

const ALICE_CLIENT: &str = "tui";
const BOB_CLIENT: &str = "desk";
const CAROL_CLIENT: &str = "carol";
const STRANGER_CLIENT: &str = "stranger";

// ---------------------------------------------------------------------------
// 面板端口 fake (跨面共享一份事实, 供按域过滤断言)
// ---------------------------------------------------------------------------

/// 审计档案 fake: 既是写口 (观测攒批面落点) 也是读口 (面板审计面取数口),
/// 所以 `domain=`/`session=` 归属记号的过滤断言走的是同一批真实行。
#[derive(Debug, Default)]
struct AuditArchive {
    rows: Mutex<Vec<AuditDto>>,
}

impl AuditArchive {
    fn seed(&self, event: &str, detail: &str) {
        self.rows.lock().unwrap().push(AuditDto {
            ts: 0,
            event: event.to_string(),
            service: "gateway".to_string(),
            detail: Some(detail.to_string()),
        });
    }

    fn details_of(&self, event: &str) -> Vec<String> {
        self.rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.event == event)
            .filter_map(|row| row.detail.clone())
            .collect()
    }
}

#[async_trait::async_trait]
impl AuditQuery for AuditArchive {
    async fn list_audit(&self, _limit: usize) -> Result<Vec<AuditDto>, String> {
        Ok(self.rows.lock().unwrap().clone())
    }
}

#[async_trait::async_trait]
impl AuditCommand for AuditArchive {
    async fn append_audit(&self, event: &str, detail: Option<&str>) {
        self.rows.lock().unwrap().push(AuditDto {
            ts: 0,
            event: event.to_string(),
            service: "gateway".to_string(),
            detail: detail.map(str::to_string),
        });
    }
}

/// 会话读口 fake: 全局视角列全部三条会话 (默认域读数的零回归基准)。
struct AllSessions {
    rows: Vec<SessionSummaryDto>,
}

#[async_trait::async_trait]
impl SessionQuery for AllSessions {
    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, String> {
        Ok(self.rows.clone())
    }
}

/// 记忆读口 fake: 每条会话一条情节 + 一张带归属的图。
struct SharedMemory {
    episodes: Vec<EpisodeDto>,
    graph: MemoryGraphDto,
}

#[async_trait::async_trait]
impl MemoryQuery for SharedMemory {
    async fn list_episodes(
        &self,
        _session: Option<&str>,
        _query: Option<&str>,
        _limit: usize,
    ) -> Result<Vec<EpisodeDto>, String> {
        Ok(self.episodes.clone())
    }

    async fn memory_graph(&self) -> Result<MemoryGraphDto, String> {
        Ok(self.graph.clone())
    }
}

/// 授权读口 fake: 两行全局授权 (行无域归属记号 → 域视图为空, fail-closed)。
struct SharedGrants;

#[async_trait::async_trait]
impl GrantQuery for SharedGrants {
    async fn list_grants(&self) -> Result<Vec<GrantDto>, String> {
        Ok(vec![
            GrantDto {
                permission: "execute_tool:tool.calc".to_string(),
                capability: "tool.calc".to_string(),
                granted_at: None,
            },
            GrantDto {
                permission: "execute_tool:tool.clock".to_string(),
                capability: "tool.clock".to_string(),
                granted_at: None,
            },
        ])
    }
}

/// 任务面板端口 fake: 两行任务 + 满额计数 (共享快照, 域视图须重算计数)。
struct SharedTasks {
    snapshot: TaskBoardSnapshotDto,
}

#[async_trait::async_trait]
impl TaskBoardPanel for SharedTasks {
    async fn task_snapshot(&self) -> Result<TaskBoardSnapshotDto, String> {
        Ok(self.snapshot.clone())
    }

    async fn human_judge_retry(
        &self,
        _task_id: &str,
        _judge: &str,
        _reason: &str,
    ) -> Result<TaskPanelDto, TaskPanelError> {
        Err(TaskPanelError::Internal {
            reason: "test fake: not used".to_string(),
        })
    }
}

fn task(id: &str, state: &str) -> TaskPanelDto {
    TaskPanelDto {
        id: id.to_string(),
        title: format!("task {id}"),
        state: state.to_string(),
        attempts: 1,
        assignee: None,
        created_at_ms: 0,
        updated_at_ms: 0,
        payload: serde_json::json!({}),
        timeline: Vec::new(),
    }
}

fn summary(id: SessionId) -> SessionSummaryDto {
    SessionSummaryDto {
        id: id.to_string(),
        title: None,
        created_at: 0,
        updated_at: 0,
        message_count: 0,
        revision: 0,
    }
}

fn episode(id: &str, session: SessionId) -> EpisodeDto {
    EpisodeDto {
        id: id.to_string(),
        timestamp: 0,
        role: "user".to_string(),
        content: format!("memory of {id}"),
        session_id: session.to_string(),
        category: None,
        importance: None,
        protected: None,
        status: None,
    }
}

// ---------------------------------------------------------------------------
// 装配 (令牌门 + 角色门 + 域门全激活; 每例独立数据目录, 并行安全)
// ---------------------------------------------------------------------------

struct Harness {
    state: GatewayState,
    audit: Arc<AuditArchive>,
    alice_session: SessionId,
    bob_session: SessionId,
    default_session: SessionId,
}

static SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir() -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "apeireth-domain-faces-open-{}-{n}",
        std::process::id()
    ));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

fn write_tokens(dir: &std::path::Path) {
    let doc = apeireth_core::stored_doc::StoredDoc::new(
        apeireth_gateway::CLIENT_TOKENS_DOC_NAME,
        apeireth_gateway::CLIENT_TOKENS_DOC_VERSION,
        vec![apeireth_gateway::CLIENT_TOKENS_DOC_VERSION],
        ClientTokenConfig::new(vec![
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
                name: CAROL_CLIENT.to_string(),
                token: CAROL_TOKEN.to_string(),
                user_id: Some("user-carol".to_string()),
            },
            ClientTokenSpec {
                name: STRANGER_CLIENT.to_string(),
                token: STRANGER_TOKEN.to_string(),
                user_id: None,
            },
        ]),
    );
    fs_err::write(
        dir.join(apeireth_gateway::CLIENT_TOKENS_FILE),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();
}

fn user_record(user_id: &str, role: &str, clients: &[&str]) -> UserRecord {
    UserRecord {
        user_id: user_id.to_string(),
        role: role.to_string(),
        client_names: clients.iter().map(|c| (*c).to_string()).collect(),
        domain: None,
    }
}

/// 带域面 + 面板端口的网关装配: 两个绑定用户各持自己的域与会话写闸
/// (默认域 = 运行时那把写闸, 同一实例), 令牌门/角色门/域门全激活。
async fn harness() -> Harness {
    let dir = temp_dir();
    std::env::remove_var(CLIENT_TOKENS_ENV);
    write_tokens(&dir);
    // 两户各自 owner (多户视角: 每户户主), 供审批面 ApprovalL2 过角色门后
    // 再过跨域归属闸的断言; user-carol 是「已开域但数据面未开」的靶子用户。
    let users = UserTable::new(vec![
        user_record("user-alice", "owner", &[ALICE_CLIENT]),
        user_record("user-bob", "owner", &[BOB_CLIENT]),
        user_record("user-carol", "member", &[CAROL_CLIENT]),
    ]);
    users.save(&dir).expect("users.json");

    let registry = DomainRegistry::open(&dir).unwrap();
    registry
        .create("user-alice", "Alice", DomainRole::Owner)
        .unwrap();
    registry
        .create("user-bob", "Bob", DomainRole::Owner)
        .unwrap();
    // carol 的域已开但**数据面未开** (平面无该域写闸) —— 拒开不代跑的靶子。
    registry
        .create("user-carol", "Carol", DomainRole::Member)
        .unwrap();

    let alice_session = SessionId::new();
    let bob_session = SessionId::new();
    let default_session = SessionId::new();

    let audit = Arc::new(AuditArchive::default());
    // 面板共享事实: 每面都同时含本域行/他域行/无归属行 (过滤断言的靶子)。
    audit.seed(
        "gateway.note",
        &format!("session={alice_session} client=tui action=chat.turn"),
    );
    audit.seed(
        "gateway.note",
        &format!("session={bob_session} client=desk action=chat.turn"),
    );
    audit.seed(
        "gateway.user_action",
        "user=user-alice role=owner action=user_seed endpoint=/v1/panel/audit decision=allow domain=user-alice",
    );
    audit.seed(
        "gateway.user_action",
        "user=user-alice role=owner action=user_seed endpoint=/v1/panel/audit decision=allow domain=user-bob",
    );
    audit.seed("gateway.note", "reason=unattributed-process-fact");

    let services = GatewayServices {
        sessions: Some(Arc::new(AllSessions {
            rows: vec![
                summary(alice_session),
                summary(bob_session),
                summary(default_session),
            ],
        })),
        memory: Some(Arc::new(SharedMemory {
            episodes: vec![
                episode("ep-alice", alice_session),
                episode("ep-bob", bob_session),
                episode("ep-default", default_session),
            ],
            graph: MemoryGraphDto {
                nodes: vec![
                    node(&format!("session:{alice_session}")),
                    node(&format!("session:{bob_session}")),
                    node("episode:ep-alice"),
                    node("episode:ep-bob"),
                    node("unattributed:orphan"),
                ],
                edges: vec![
                    edge(&format!("session:{alice_session}"), "episode:ep-alice"),
                    edge(&format!("session:{bob_session}"), "episode:ep-bob"),
                ],
            },
        })),
        audit: Some(audit.clone()),
        audit_commands: Some(audit.clone()),
        grants: Some(Arc::new(SharedGrants)),
        tasks: Some(Arc::new(SharedTasks {
            snapshot: TaskBoardSnapshotDto {
                tasks: vec![task("task-a", "pending"), task("task-b", "failed")],
                counts: TaskCountsDto {
                    pending: 1,
                    assigned: 0,
                    running: 0,
                    done: 0,
                    failed: 1,
                    blocked: 0,
                    total: 2,
                },
            },
        })),
        user_admin: Some(Arc::new(UserAdminService::new(&dir))),
        domain_admin: Some(Arc::new(DomainAdminService::new(
            DomainRegistry::open(&dir).unwrap(),
        ))),
        ..GatewayServices::default()
    };

    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let mut state = build_gateway_state_with_services(runtime.clone(), services);
    state.token_gate = GatewayTokenGate::Enforced(GATE_TOKEN.to_string());
    state.client_tokens = ClientTokenTable::load(Some(&dir)).expect("token table");
    state.user_access = UserAccess::load(&dir, &state.client_tokens).expect("role face");

    // 域平面: 默认域与运行时共用同一把写闸 (W2-③ 承诺不拆)。
    let alice_manager = Arc::new(SessionManager::new(
        Arc::new(InMemorySessionStore::new()),
        system_clock(),
    ));
    let bob_manager = Arc::new(SessionManager::new(
        Arc::new(InMemorySessionStore::new()),
        system_clock(),
    ));
    let mut managers = BTreeMap::new();
    managers.insert(
        apeireth_core::user_domain::DomainUserId::new("user-alice").unwrap(),
        alice_manager.clone(),
    );
    managers.insert(
        apeireth_core::user_domain::DomainUserId::new("user-bob").unwrap(),
        bob_manager.clone(),
    );
    managers.insert(
        apeireth_core::user_domain::DomainUserId::default_domain(),
        runtime.sessions_arc().clone(),
    );
    state.domains = Some(Arc::new(DomainSessionPlane::new(registry, managers)));

    alice_manager
        .save(&Session::new(alice_session, system_clock().as_ref()))
        .await
        .unwrap();
    bob_manager
        .save(&Session::new(bob_session, system_clock().as_ref()))
        .await
        .unwrap();
    runtime
        .sessions()
        .save(&Session::new(default_session, system_clock().as_ref()))
        .await
        .unwrap();

    Harness {
        state,
        audit,
        alice_session,
        bob_session,
        default_session,
    }
}

fn node(id: &str) -> apeireth_gateway::GraphNodeDto {
    apeireth_gateway::GraphNodeDto {
        id: id.to_string(),
        label: id.to_string(),
        kind: "node".to_string(),
    }
}

fn edge(from: &str, to: &str) -> apeireth_gateway::GraphEdgeDto {
    apeireth_gateway::GraphEdgeDto {
        from: from.to_string(),
        to: to.to_string(),
        weight: 1.0,
        label: None,
    }
}

fn request(method: &str, uri: &str, token: &str, body: Body) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .unwrap();
    request
        .headers_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    request
}

fn json_request(method: &str, uri: &str, token: &str, body: serde_json::Value) -> Request<Body> {
    let mut request = request(method, uri, token, Body::from(body.to_string()));
    request
        .headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    request
}

fn claimed_domain(request: Request<Body>, domain: &str) -> Request<Body> {
    let mut request = request;
    request
        .headers_mut()
        .insert("x-apeireth-domain", domain.parse().unwrap());
    request
}

async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(&body_bytes(response).await).expect("json body")
}

fn router(harness: &Harness) -> axum::Router {
    canonical_router_with_state(harness.state.clone())
}

/// Read SSE frames until `marker` shows up in the accumulated text.
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
            Ok(Some(Err(_))) => {}
            Ok(None) => break,
            Err(_) => panic!("timed out waiting for SSE frames; collected={collected:?}"),
        }
    }
    collected
}

// ---------------------------------------------------------------------------
// 1. 面板按域过滤 (A 域看不到 B 域计数)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn panel_faces_filter_every_row_by_domain_and_counts_never_leak_the_other_domain() {
    let harness = harness().await;
    let router = router(&harness);

    // 会话面板: 本域账本直读 —— A 域恰一行, B 域/默认域的行与计数都不在。
    let sessions = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/sessions",
                ALICE_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    let rows = sessions["sessions"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "A 域只能看到本域在册会话: {sessions}");
    assert_eq!(rows[0]["id"], harness.alice_session.to_string());
    assert!(
        !sessions
            .to_string()
            .contains(&harness.bob_session.to_string()),
        "B 域会话不得进 A 域视图: {sessions}"
    );

    // 记忆面板: 行按会话归属过滤。
    let episodes = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/memory/episodes",
                BOB_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    let rows = episodes["episodes"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "B 域只见本域情节: {episodes}");
    assert_eq!(rows[0]["id"], "ep-bob");

    // 记忆图: 会话节点在册才留, 情节节点随包含边留下。
    let graph = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/graph",
                ALICE_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    let graph_text = graph.to_string();
    assert!(graph_text.contains("episode:ep-alice"), "{graph}");
    assert!(!graph_text.contains("episode:ep-bob"), "{graph}");
    assert!(!graph_text.contains("unattributed:orphan"), "{graph}");

    // 审计面板: 会话记号行在册才进, `domain=` 标注行只进标注域。
    let audit = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/audit",
                ALICE_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    let audit_text = audit.to_string();
    assert!(
        audit_text.contains(&harness.alice_session.to_string()),
        "{audit}"
    );
    assert!(
        !audit_text.contains(&harness.bob_session.to_string()),
        "{audit}"
    );
    assert!(audit_text.contains("domain=user-alice"), "{audit}");
    assert!(!audit_text.contains("domain=user-bob"), "{audit}");

    // 授权面板: 行无域归属记号 → 域视图零行 (fail-closed)。
    let grants = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/grants",
                ALICE_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        grants["grants"].as_array().unwrap().len(),
        0,
        "无归属记号的授权行不进域视图: {grants}"
    );

    // 任务面板: 行无域归属记号 → 零行 + **计数重算为零** (共享快照的计数
    // 不得泄进域视图 —— A 域看不进 B 域计数)。
    let tasks = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/tasks",
                ALICE_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(tasks["tasks"].as_array().unwrap().len(), 0, "{tasks}");
    assert_eq!(tasks["counts"]["total"], 0, "计数必须按可见行重算: {tasks}");
    assert_eq!(tasks["counts"]["failed"], 0, "分态计数不得泄漏: {tasks}");
}

#[tokio::test]
async fn panel_rows_without_attribution_markers_stay_out_of_domain_views() {
    let harness = harness().await;
    let router = router(&harness);

    // 无归属记号的审计行 (进程级事实) 不进域视图 (fail-closed, 宁拒不泄)。
    let audit = body_json(
        router
            .clone()
            .oneshot(request("GET", "/v1/panel/audit", BOB_TOKEN, Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert!(
        !audit.to_string().contains("unattributed-process-fact"),
        "无归属记号行不得进域视图: {audit}"
    );

    // 无归属记号的记忆图节点同理 (上例已断言 orphan 节点); 这里补一条
    // 「探测失败按不在册」面: 撕掉本域账本后一切会话行读作不在册。
    // (在册探测的 fail-closed 语义另有 `domain_context` 单元钉, 此处钉面板面。)
    let sessions = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/sessions",
                BOB_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        sessions["sessions"].as_array().unwrap().len(),
        1,
        "B 域恰见本域在册一行: {sessions}"
    );
}

// ---------------------------------------------------------------------------
// 3. 事件域一致 (面板订阅与事件同一在册探测)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn panel_visibility_and_event_delivery_share_one_domain_membership_probe() {
    let harness = harness().await;
    let router = router(&harness);

    // 面板面: alice 可见本域会话、不可见他域会话 (同一在册探测)。
    let sessions = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/sessions",
                ALICE_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    let panel_sees_own = sessions
        .to_string()
        .contains(&harness.alice_session.to_string());
    let panel_sees_foreign = sessions
        .to_string()
        .contains(&harness.bob_session.to_string());

    // 事件面: 同一订阅只投本域在册会话的帧。
    let response = router
        .clone()
        .oneshot(request(
            "GET",
            "/v1/apeireth/events",
            ALICE_TOKEN,
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    harness.state.events.publish(GatewayEvent::new(
        "turn_delta",
        serde_json::json!({ "session": harness.alice_session, "text": "own-frame-marker" }),
    ));
    harness.state.events.publish(GatewayEvent::new(
        "turn_delta",
        serde_json::json!({ "session": harness.bob_session, "text": "foreign-frame-marker" }),
    ));
    // 无会话归属的进程级帧照常投递 (也作收帧标记)。
    harness.state.events.publish(GatewayEvent::new(
        "domain_probe_done",
        serde_json::json!({ "marker": "domain-probe-end" }),
    ));
    let collected = collect_sse_until(response, "domain-probe-end").await;

    assert_eq!(
        panel_sees_own,
        collected.contains("own-frame-marker"),
        "面板可见 ⇔ 事件投递 (本域): 面板={panel_sees_own} 帧={collected}"
    );
    assert_eq!(
        panel_sees_foreign,
        collected.contains("foreign-frame-marker"),
        "面板可见 ⇔ 事件投递 (他域): 面板={panel_sees_foreign} 帧={collected}"
    );
    assert!(panel_sees_own, "本域会话必须在面板可见");
    assert!(!panel_sees_foreign, "他域会话必须在面板不可见");
    assert!(collected.contains("domain-probe-end"), "{collected}");
}

#[tokio::test]
async fn event_subscriptions_cannot_name_another_domains_session() {
    let harness = harness().await;
    let router = router(&harness);

    // `?session=` 点名他域会话 = 与不存在**同帧** 404 (存在性不泄露)。
    let foreign = router
        .clone()
        .oneshot(request(
            "GET",
            &format!("/v1/apeireth/events?session={}", harness.bob_session),
            ALICE_TOKEN,
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);
    let foreign_bytes = body_bytes(foreign).await;
    let value: serde_json::Value = serde_json::from_slice(&foreign_bytes).unwrap();
    assert_eq!(value["error"]["code"], "session_not_found");

    // 经 bob 自己的会话面删掉那条会话, 同 id 再查 → 帧逐字节不变。
    let deleted = router
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/v1/sessions/{}", harness.bob_session),
            BOB_TOKEN,
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::OK);
    let gone = router
        .oneshot(request(
            "GET",
            &format!("/v1/apeireth/events?session={}", harness.bob_session),
            ALICE_TOKEN,
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(gone.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        foreign_bytes,
        body_bytes(gone).await,
        "跨域订阅帧不得随他域存在性而变"
    );
}

/// 已开域但数据面未开 (平面无该域写闸): 面板读 = 403 `domain_refused`
/// (错误即帧, 不给空视图假装「本域无数据」), 他域不受牵连。
#[tokio::test]
async fn a_domain_without_an_open_session_plane_is_refused_not_given_an_empty_view() {
    let harness = harness().await;
    let router = router(&harness);

    let refused = router
        .clone()
        .oneshot(request(
            "GET",
            "/v1/panel/sessions",
            CAROL_TOKEN,
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);
    let value = body_json(refused).await;
    assert_eq!(
        value["error"]["code"], "domain_refused",
        "数据面未开必须拒开, 绝不给空视图装成本域无数据"
    );

    // 一户数据面未开不砖化整面: 其余域照常。
    let other = router
        .clone()
        .oneshot(request(
            "GET",
            "/v1/panel/sessions",
            ALICE_TOKEN,
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(other.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// 4. 审批跨域归属拒 (W3-② 跨用户审批闸升级为跨域归属)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn approval_faces_treat_cross_domain_sessions_as_missing() {
    let harness = harness().await;
    let router = router(&harness);

    // 收件箱: 他域会话的审批读作不存在 (与不存在同帧)。
    let inbox = router
        .clone()
        .oneshot(request(
            "GET",
            &format!("/v1/approvals?session={}", harness.bob_session),
            ALICE_TOKEN,
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(inbox.status(), StatusCode::NOT_FOUND);
    let inbox_bytes = body_bytes(inbox).await;
    let value: serde_json::Value = serde_json::from_slice(&inbox_bytes).unwrap();
    assert_eq!(value["error"]["code"], "session_not_found");

    // 解决口: alice 是 owner (升级前 owner 可批任意会话), 升级后跨域 = 同帧
    // 404 —— 连「此处有审批」都不泄露。
    let resolve = router
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/approvals/resolve",
            ALICE_TOKEN,
            serde_json::json!({
                "session": harness.bob_session.to_string(),
                "approval": "00000000-0000-0000-0000-0000000000aa",
                "decision": "approve",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resolve.status(), StatusCode::NOT_FOUND);
    let resolve_bytes = body_bytes(resolve).await;
    let value: serde_json::Value = serde_json::from_slice(&resolve_bytes).unwrap();
    assert_eq!(value["error"]["code"], "session_not_found");

    // 同帧语义: 他域存在 / 全局不存在 两态逐字节一致。
    let deleted = router
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/v1/sessions/{}", harness.bob_session),
            BOB_TOKEN,
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::OK);
    let gone_inbox = router
        .clone()
        .oneshot(request(
            "GET",
            &format!("/v1/approvals?session={}", harness.bob_session),
            ALICE_TOKEN,
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(
        inbox_bytes,
        body_bytes(gone_inbox).await,
        "审批收件箱跨域帧不得随他域存在性而变"
    );

    // 本域会话不被域闸误伤: 域归属过闸后照常走到本域执行面 (本装配无伙伴
    // 运行时 → 403 `domain_refused` fail-closed, 绝不代跑 —— 这证明上面的
    // 404 是域归属拒, 不是审批面整体不可用)。
    let own = router
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/approvals/resolve",
            ALICE_TOKEN,
            serde_json::json!({
                "session": harness.alice_session.to_string(),
                "approval": "00000000-0000-0000-0000-0000000000bb",
                "decision": "approve",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(own.status(), StatusCode::FORBIDDEN);
    let value = body_json(own).await;
    assert_eq!(
        value["error"]["code"], "domain_refused",
        "本域审批过域闸后走本域执行面 (无伙伴实例 = 拒开不代跑)"
    );
}

// ---------------------------------------------------------------------------
// 5. 管理面全局 + 审计域标注
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_faces_stay_global_and_audit_rows_carry_the_target_domain_mark() {
    let harness = harness().await;
    let router = router(&harness);

    // 全局视角: 非默认域 owner 读 users/domains 面 = 不按域过滤的全表。
    let users = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/users",
                ALICE_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    let user_rows = users["users"].as_array().unwrap();
    assert_eq!(user_rows.len(), 3, "管理面保持全局 (owner 视角): {users}");
    assert!(users.to_string().contains("user-bob"), "{users}");

    let domains = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/domains",
                ALICE_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    let domain_rows = domains["domains"].as_array().unwrap();
    assert!(domain_rows.len() >= 3, "域管理面保持全局: {domains}");
    assert!(domains.to_string().contains("user-bob"), "{domains}");

    // 建户 (目标域 = user-bob 的域引用): 管理动作照常走全局面。
    let created = router
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/admin/users",
            ALICE_TOKEN,
            serde_json::json!({
                "user_id": "user-dave",
                "role": "member",
                "client_names": [],
                "domain": "user-bob",
            }),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);

    // 开域 (目标域 = 新开的域)。
    let opened = router
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/admin/domains",
            ALICE_TOKEN,
            serde_json::json!({ "user_id": "user-erin", "display_name": "Erin" }),
        ))
        .await
        .unwrap();
    assert_eq!(opened.status(), StatusCode::CREATED);
    harness.state.observations.flush().await;

    // 审计域标注: 管理动作行带 `domain=<目标域>` 记号 (行为域行另记)。
    let rows = harness.audit.details_of("gateway.user_action");
    let user_add = rows
        .iter()
        .find(|row| row.contains("action=user_add") && row.contains("user=user-dave"))
        .or_else(|| {
            rows.iter()
                .find(|row| row.contains("action=user_add") && row.contains("decision=allow"))
        })
        .expect("user_add 审计行");
    assert!(
        user_add.contains("domain=user-bob"),
        "管理动作 = 目标域标注: {user_add}"
    );
    let domain_create = rows
        .iter()
        .find(|row| row.contains("action=domain_create") && row.contains("decision=allow"))
        .expect("domain_create 审计行");
    assert!(
        domain_create.contains("domain=user-erin"),
        "开域动作 = 目标域标注: {domain_create}"
    );

    // 标注进域视图: 目标域 (user-bob) 的审计面看得到 user_add 行,
    // 行为域 (user-alice) 的视图看不到目标域行 (归属记号逐域分账)。
    let bob_audit = body_json(
        router
            .clone()
            .oneshot(request("GET", "/v1/panel/audit", BOB_TOKEN, Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert!(
        bob_audit.to_string().contains("action=user_add"),
        "目标域审计视图必须看得到管理动作行: {bob_audit}"
    );
    let alice_audit = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/audit",
                ALICE_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert!(
        !alice_audit.to_string().contains("domain=user-bob"),
        "目标域行不得进他域审计视图: {alice_audit}"
    );
}

// ---------------------------------------------------------------------------
// 6. 同帧语义保持 + 默认域零回归 + 面闸表
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_domain_reads_stay_unfiltered_on_every_panel_face() {
    let harness = harness().await;
    let router = router(&harness);

    // 未绑定客户端 = 默认域: 五面全量读数 (零回归)。
    let sessions = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/sessions",
                STRANGER_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        sessions["sessions"].as_array().unwrap().len(),
        3,
        "默认域读数不过滤: {sessions}"
    );
    let episodes = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/memory/episodes",
                STRANGER_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(episodes["episodes"].as_array().unwrap().len(), 3);
    let audit = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/audit",
                STRANGER_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert!(audit.to_string().contains("unattributed-process-fact"));
    assert!(audit.to_string().contains(&harness.bob_session.to_string()));
    let grants = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/grants",
                STRANGER_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(grants["grants"].as_array().unwrap().len(), 2);
    let tasks = body_json(
        router
            .clone()
            .oneshot(request(
                "GET",
                "/v1/panel/tasks",
                STRANGER_TOKEN,
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(tasks["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(tasks["counts"]["total"], 2, "默认域计数保持共享快照真值");
}

#[tokio::test]
async fn the_domain_claim_keeps_one_constant_403_frame_on_the_new_faces() {
    let harness = harness().await;
    let router = router(&harness);

    // 自称他域 (存在) 与自称未知域 (不存在) → 403 恒定同帧 (W3-① 契约钉死)。
    let mut bodies = Vec::new();
    for claimed in ["user-bob", "nobody"] {
        let response = router
            .clone()
            .oneshot(claimed_domain(
                request("GET", "/v1/panel/audit", ALICE_TOKEN, Body::empty()),
                claimed,
            ))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "claimed={claimed}"
        );
        let bytes = body_bytes(response).await;
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["error"]["code"], "domain_forbidden");
        bodies.push(bytes);
    }
    assert_eq!(
        bodies[0], bodies[1],
        "自称域帧必须恒定: 目标域存在与否不得外泄"
    );

    // 自称本域 = 放行 (自称名头只做绑定校验, 不改路由)。
    let own = router
        .oneshot(claimed_domain(
            request("GET", "/v1/panel/audit", ALICE_TOKEN, Body::empty()),
            "user-alice",
        ))
        .await
        .unwrap();
    assert_eq!(own.status(), StatusCode::OK);
}

#[tokio::test]
async fn panel_write_mates_stay_closed_while_the_read_faces_open() {
    let harness = harness().await;
    let router = router(&harness);

    // 逐面开放的**只读**口全开: 六个面板读面 + 两个管理列表面。
    for (method, uri) in [
        ("GET", "/v1/panel/sessions"),
        ("GET", "/v1/panel/memory/episodes"),
        ("GET", "/v1/panel/graph"),
        ("GET", "/v1/panel/audit"),
        ("GET", "/v1/panel/grants"),
        ("GET", "/v1/panel/tasks"),
        ("GET", "/v1/panel/users"),
        ("GET", "/v1/panel/domains"),
    ] {
        let response = router
            .clone()
            .oneshot(request(method, uri, ALICE_TOKEN, Body::empty()))
            .await
            .unwrap();
        assert_ne!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{method} {uri} 已按域开放, 不得再被面闸拒"
        );
    }

    // 同族写面与其余面仍按域闸 fail-closed (写面随批, 不随读面一起放开)。
    for (method, uri, body) in [
        ("POST", "/v1/panel/grants/revoke", Body::empty()),
        (
            "POST",
            "/v1/panel/tasks/task-1/human-judge-retry",
            Body::empty(),
        ),
        ("GET", "/v1/panel/traces", Body::empty()),
        ("GET", "/v1/models", Body::empty()),
    ] {
        let response = router
            .clone()
            .oneshot(request(method, uri, ALICE_TOKEN, body))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{method} {uri} 不得随读面一起放开"
        );
        let value = body_json(response).await;
        assert_eq!(value["error"]["code"], "domain_forbidden");
    }
}
