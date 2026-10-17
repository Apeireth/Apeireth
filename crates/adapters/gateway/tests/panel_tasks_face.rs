//! 任务面板面的面契约: `GET /v1/panel/tasks` 的一次快照形状 (行 + 时间线 +
//! 分态计数) + 显式空态 + W2-② 令牌门 + 未接线 501 诚实降级 +
//! `POST /v1/panel/tasks/{task_id}/human-judge-retry` 人判重试链 +
//! **面板边界** (无增删改派口) + `tasks.read` / `tasks.human_judge_retry` 入能力清单。
//!
//! 这里钉七件事:
//! 1. **快照形状** —— `{tasks:[{id,title,state,attempts,assignee,created_at_ms,
//!    updated_at_ms,payload,timeline}], counts:{pending..blocked,total}}`, 状态
//!    六态冻结词表, 时间线按任务过滤 (kind/at_ms/detail);
//! 2. **显式空态** —— `tasks: []` / `counts` 全 0, 不缺字段、不 501;
//! 3. **令牌门同面** —— 读与人判动作同过 W2-② 门 (401 矩阵 + 真令牌 200);
//! 4. **未接线如实** —— 端口未注入 = 501 `unsupported` (不假装有数据源);
//! 5. **人判重试链** —— POST 到端口的 `human_judge_retry`, 署名只来自解析链
//!    (不可自称), 缺省理由如实记录; 任务不存在 404 / 非 failed 409 拒绝即帧;
//! 6. **面板边界** —— 路由表无增/删/改派/完成/fail 口 (防越权, 走工具面);
//! 7. **能力清单入册** —— `tasks.read` (只读) + `tasks.human_judge_retry` (动作)。
//!
//! 真数据面 (真 `TaskQueue`: 人判重试 = 队列真转移 + 真审计) 的端到端钉在
//! 组合根侧 `crates/adapters/cli/tests/task_panel_real_chain.rs` (同一端口)。

use std::sync::{Arc, Mutex};

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices, GatewayState,
    GatewayTokenGate, HumanJudgeRetryResponse, TaskBoardPanel, TaskBoardSnapshotDto, TaskCountsDto,
    TaskPanelDto, TaskPanelError, TaskTimelineDto,
};
use apeireth_runtime::canonical::Runtime;
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const TOKEN: &str = "panel-tasks-token-0123456789abcdef";
const FORGED_TOKEN: &str = "forged-token-fedcba9876543210";

/// 端口桩: 回一份固定快照, 记录人判动作的入参 (面形状与门语义不需要真队列)。
#[derive(Default)]
struct StubTaskBoard {
    snapshot: TaskBoardSnapshotDto,
    retried: Mutex<Vec<(String, String, String)>>,
    retry_result: Mutex<Option<Result<TaskPanelDto, TaskPanelError>>>,
}

#[async_trait]
impl TaskBoardPanel for StubTaskBoard {
    async fn task_snapshot(&self) -> Result<TaskBoardSnapshotDto, String> {
        Ok(self.snapshot.clone())
    }

    async fn human_judge_retry(
        &self,
        task_id: &str,
        judge: &str,
        reason: &str,
    ) -> Result<TaskPanelDto, TaskPanelError> {
        self.retried.lock().expect("锁").push((
            task_id.to_string(),
            judge.to_string(),
            reason.to_string(),
        ));
        match self.retry_result.lock().expect("锁").take() {
            Some(result) => result,
            None => Ok(sample_task(task_id, "pending")),
        }
    }
}

fn sample_task(id: &str, state: &str) -> TaskPanelDto {
    TaskPanelDto {
        id: id.to_string(),
        title: "校对发布清单".to_string(),
        state: state.to_string(),
        attempts: 2,
        assignee: Some("worker-a".to_string()),
        created_at_ms: 1_700_000_000_000,
        updated_at_ms: 1_700_000_000_500,
        payload: serde_json::json!({"module": "release", "step": 3}),
        timeline: vec![
            TaskTimelineDto {
                kind: "enqueued".to_string(),
                at_ms: 1_700_000_000_000,
                detail: None,
            },
            TaskTimelineDto {
                kind: "failure_judged".to_string(),
                at_ms: 1_700_000_000_500,
                detail: Some(r#"{"judge":"owner","reason":"验收不过"}"#.to_string()),
            },
        ],
    }
}

fn sample_snapshot() -> TaskBoardSnapshotDto {
    TaskBoardSnapshotDto {
        tasks: vec![
            sample_task("task-1", "failed"),
            TaskPanelDto {
                assignee: None,
                timeline: Vec::new(),
                attempts: 0,
                ..sample_task("task-2", "pending")
            },
        ],
        counts: TaskCountsDto {
            pending: 1,
            assigned: 0,
            running: 0,
            done: 0,
            failed: 1,
            blocked: 0,
            total: 2,
        },
    }
}

async fn harness(services: GatewayServices, gate: GatewayTokenGate) -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.expect("runtime"));
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = gate;
    state
}

fn request(method: &str, uri: &str, token: Option<&str>, body: Body) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
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

fn get(uri: &str, token: Option<&str>) -> Request<Body> {
    request("GET", uri, token, Body::empty())
}

fn post(uri: &str, token: Option<&str>, json: serde_json::Value) -> Request<Body> {
    let mut request = request("POST", uri, token, Body::from(json.to_string()));
    request.headers_mut().insert(
        "content-type",
        "application/json".parse().expect("content type"),
    );
    request
}

async fn status_and_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn stub_services() -> GatewayServices {
    GatewayServices {
        tasks: Some(Arc::new(StubTaskBoard {
            snapshot: sample_snapshot(),
            ..StubTaskBoard::default()
        })),
        ..GatewayServices::default()
    }
}

/// 1. 快照形状: 行字段集合与计数字段集合逐字钉死, 状态六态词表, 时间线按任务过滤。
#[tokio::test]
async fn task_snapshot_shape_is_the_stable_panel_contract() {
    let state = harness(stub_services(), GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) =
        status_and_body(router.oneshot(get("/v1/panel/tasks", None)).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    let tasks = body["tasks"].as_array().expect("tasks array");
    assert_eq!(tasks.len(), 2);
    let mut keys: Vec<&str> = tasks[0]
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "assignee",
            "attempts",
            "created_at_ms",
            "id",
            "payload",
            "state",
            "timeline",
            "title",
            "updated_at_ms",
        ],
        "任务行字段集合 = 合同九字段, 不多不少"
    );
    assert_eq!(tasks[0]["id"], "task-1");
    assert_eq!(tasks[0]["state"], "failed");
    assert_eq!(tasks[0]["attempts"], 2);
    assert_eq!(tasks[0]["assignee"], "worker-a");
    assert_eq!(tasks[0]["payload"]["module"], "release");
    // 状态六态冻结词表 (行值与计数键都出自此表)。
    let counts = &body["counts"];
    let mut count_keys: Vec<&str> = counts
        .as_object()
        .expect("counts object")
        .keys()
        .map(String::as_str)
        .collect();
    count_keys.sort_unstable();
    assert_eq!(
        count_keys,
        vec!["assigned", "blocked", "done", "failed", "pending", "running", "total",]
    );
    assert_eq!(counts["failed"], 1);
    assert_eq!(counts["total"], 2);
    for state in [
        "pending", "assigned", "running", "done", "failed", "blocked",
    ] {
        assert!(counts.get(state).is_some(), "六态计数缺 {state}");
    }
    // 时间线: 按任务过滤, kind/at_ms/detail 形状。
    let timeline = tasks[0]["timeline"].as_array().expect("timeline");
    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[1]["kind"], "failure_judged");
    assert_eq!(timeline[1]["at_ms"], 1_700_000_000_500i64);
    assert!(timeline[1]["detail"]
        .as_str()
        .expect("detail")
        .contains("验收不过"));
    assert!(timeline[0]["detail"].is_null(), "无附加字段不造 detail");
}

/// 2. 空态 = 显式空 (行空数组 + 计数全 0, 字段一个不少, 不 501)。
#[tokio::test]
async fn empty_task_board_is_an_explicit_empty_state() {
    let services = GatewayServices {
        tasks: Some(Arc::new(StubTaskBoard {
            snapshot: TaskBoardSnapshotDto {
                tasks: Vec::new(),
                counts: TaskCountsDto::default(),
            },
            ..StubTaskBoard::default()
        })),
        ..GatewayServices::default()
    };
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) =
        status_and_body(router.oneshot(get("/v1/panel/tasks", None)).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["tasks"], serde_json::json!([]));
    assert_eq!(body["counts"]["total"], 0);
    assert_eq!(body["counts"]["failed"], 0);
}

/// 3. 令牌门同面: 读与人判动作同过 W2-② 门 (401 矩阵 + 真令牌 200)。
#[tokio::test]
async fn both_task_panel_endpoints_sit_behind_the_gateway_token_gate() {
    let state = harness(
        stub_services(),
        GatewayTokenGate::Enforced(TOKEN.to_string()),
    )
    .await;
    let router = canonical_router_with_state(state);

    let cases: Vec<(&str, Request<Body>)> = vec![
        ("读面缺令牌", get("/v1/panel/tasks", None)),
        (
            "人判面缺令牌",
            post(
                "/v1/panel/tasks/task-1/human-judge-retry",
                None,
                serde_json::json!({}),
            ),
        ),
    ];
    for (label, request) in cases {
        let (status, text) = status_and_body(router.clone().oneshot(request).await.unwrap()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{label}: {text}");
        assert!(text.contains("auth_missing_key"), "{label}: {text}");
    }

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(get("/v1/panel/tasks", Some(FORGED_TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_invalid_key"), "{text}");
    assert!(!text.contains(TOKEN), "401 帧不得回显令牌: {text}");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(get("/v1/panel/tasks", Some(TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(text.contains("\"tasks\""), "{text}");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(post(
                "/v1/panel/tasks/task-1/human-judge-retry",
                Some(TOKEN),
                serde_json::json!({"reason": "修好了再试"}),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
}

/// 4. 未接线 = 501 `unsupported` 诚实降级 (不假装有数据源)。
#[tokio::test]
async fn task_panel_degrades_to_501_without_the_port() {
    let state = harness(
        GatewayServices::default(),
        GatewayTokenGate::UnprotectedDefault,
    )
    .await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(get("/v1/panel/tasks", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{text}");
    assert!(text.contains("\"unsupported\""), "{text}");
    assert!(text.contains("tasks.read"), "{text}");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(post(
                "/v1/panel/tasks/task-1/human-judge-retry",
                None,
                serde_json::json!({}),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{text}");
    assert!(text.contains("tasks.human_judge_retry"), "{text}");
}

/// 5. 人判重试链: POST 到端口, 署名只来自解析链 (不可自称), 缺省理由如实;
///    任务不存在 404 / 非 failed 409 (拒绝即帧, 不静默)。
#[tokio::test]
async fn human_judge_retry_reaches_the_port_with_a_chain_resolved_judge() {
    let board = Arc::new(StubTaskBoard {
        snapshot: sample_snapshot(),
        ..StubTaskBoard::default()
    });
    let services = GatewayServices {
        tasks: Some(board.clone()),
        ..GatewayServices::default()
    };
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);

    // 带理由: 端口收到 (任务 id, 解析署名, 理由); 响应回重试后的任务行。
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(post(
                "/v1/panel/tasks/task-1/human-judge-retry",
                None,
                serde_json::json!({"reason": "修好了再试"}),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: HumanJudgeRetryResponse = serde_json::from_str(&text).expect("响应信封");
    assert_eq!(body.task.id, "task-1");
    assert_eq!(body.task.state, "pending", "人判重试 = Failed → Pending");

    // 缺省理由如实记录 (不编造调用方没说的话)。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(post(
                "/v1/panel/tasks/task-2/human-judge-retry",
                None,
                serde_json::json!({}),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let calls = board.retried.lock().expect("锁").clone();
    assert_eq!(calls[0].0, "task-1");
    assert_eq!(calls[0].2, "修好了再试");
    assert_eq!(calls[1].0, "task-2");
    assert_eq!(calls[1].2, "驾驶舱任务面板人判重试", "缺省理由 = 显式常量");
    // 署名只来自解析链: 未设防口径的过门身份如实为 anonymous (请求无从自称)。
    assert_eq!(calls[0].1, "anonymous");
    assert_eq!(calls[1].1, "anonymous");

    // 任务不存在 → 404 not_found (不静默、不代建)。
    *board.retry_result.lock().expect("锁") = Some(Err(TaskPanelError::NotFound {
        task_id: "task-404".to_string(),
    }));
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(post(
                "/v1/panel/tasks/task-404/human-judge-retry",
                None,
                serde_json::json!({}),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{text}");
    assert!(text.contains("not_found"), "{text}");

    // 非 failed 态 → 409 conflict (只有 failed 能人判重试)。
    *board.retry_result.lock().expect("锁") = Some(Err(TaskPanelError::InvalidState {
        task_id: "task-2".to_string(),
        state: "running".to_string(),
    }));
    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(post(
                "/v1/panel/tasks/task-2/human-judge-retry",
                None,
                serde_json::json!({}),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{text}");
    assert!(text.contains("conflict"), "{text}");
    assert!(text.contains("只有 failed 能重试"), "{text}");
}

/// 6. 面板边界: 路由表无增/删/改派/完成/fail 口 (防越权, 这些是工具面的写面)。
#[tokio::test]
async fn task_panel_face_exposes_no_add_reassign_delete_or_fail_routes() {
    let state = harness(stub_services(), GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let forbidden: Vec<(&str, Request<Body>)> = vec![
        (
            "增 (POST 列表口)",
            post("/v1/panel/tasks", None, serde_json::json!({"title": "x"})),
        ),
        (
            "删 (DELETE 行口)",
            request("DELETE", "/v1/panel/tasks/task-1", None, Body::empty()),
        ),
        (
            "改派",
            post(
                "/v1/panel/tasks/task-1/reassign",
                None,
                serde_json::json!({"assignee": "worker-b"}),
            ),
        ),
        (
            "完成",
            post(
                "/v1/panel/tasks/task-1/complete",
                None,
                serde_json::json!({}),
            ),
        ),
        (
            "人判 fail 不进面板 (failed 只能人判进入, 但入口不在本面)",
            post(
                "/v1/panel/tasks/task-1/human-judge-fail",
                None,
                serde_json::json!({}),
            ),
        ),
    ];
    for (label, request) in forbidden {
        let (status, _) = status_and_body(router.clone().oneshot(request).await.unwrap()).await;
        assert!(
            matches!(
                status,
                StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
            ),
            "{label} 不得有面板路由 (得到 {status})"
        );
    }
    // 唯一动作面照常在 (边界不误伤)。
    let (status, _) = status_and_body(
        router
            .clone()
            .oneshot(post(
                "/v1/panel/tasks/task-1/human-judge-retry",
                None,
                serde_json::json!({}),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// 7. 能力清单入册: `tasks.read` 只读 + `tasks.human_judge_retry` 动作,
///    `supported/available` 随接线状态。
#[tokio::test]
async fn task_panel_capabilities_are_registered_with_wiring_truth() {
    let state = harness(stub_services(), GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (_, text) = status_and_body(
        router
            .oneshot(get("/v1/apeireth/capabilities", None))
            .await
            .unwrap(),
    )
    .await;
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    let groups = body["capabilities"].as_array().expect("groups");
    let group = groups
        .iter()
        .find(|group| group["name"] == "tasks")
        .expect("tasks 分组入册");
    let read = &group["capabilities"][0];
    assert_eq!(read["id"], "tasks.read");
    assert_eq!(read["read"], true);
    assert_eq!(read["write"], false);
    assert_eq!(read["supported"], true);
    assert_eq!(read["available"], true);
    let judge = &group["capabilities"][1];
    assert_eq!(judge["id"], "tasks.human_judge_retry");
    assert_eq!(judge["read"], false);
    assert_eq!(judge["write"], true);
    assert_eq!(judge["operations"], serde_json::json!(["retry"]));

    // 未接线: 清单如实报不支持 (不虚报可用)。
    let state = harness(
        GatewayServices::default(),
        GatewayTokenGate::UnprotectedDefault,
    )
    .await;
    let router = canonical_router_with_state(state);
    let (_, text) = status_and_body(
        router
            .oneshot(get("/v1/apeireth/capabilities", None))
            .await
            .unwrap(),
    )
    .await;
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    let groups = body["capabilities"].as_array().expect("groups");
    let group = groups
        .iter()
        .find(|group| group["name"] == "tasks")
        .expect("tasks 分组在册");
    for cap in group["capabilities"].as_array().expect("caps") {
        assert_eq!(cap["supported"], false);
        assert_eq!(cap["available"], false);
        assert!(cap["reason"].as_str().is_some(), "不可用须给机器可读真因");
    }
}
