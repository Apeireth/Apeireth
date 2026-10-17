//! 任务面板面的真队列链 (组合根侧): `CliTaskBoard` 附到**真**持久任务队列
//! (V6-a `TaskQueue`) 上 → `GET /v1/panel/tasks` 一次快照读 +
//! `POST /v1/panel/tasks/{task_id}/human-judge-retry` 人判重试真转移。
//! 面形状 / 令牌门 / 未接线 501 / 面板边界的契约钉在
//! `crates/adapters/gateway/tests/panel_tasks_face.rs` (同一端口)。
//!
//! 这里钉四件事 (真队列零桩, 面板→API→队列整链):
//! 1. **真值快照** —— 六态分组计数与队列 `counts` 同真值, 行字段上真记录,
//!    时间线是该任务的事件账投影;
//! 2. **人判重试真转移** —— POST → `TaskQueue::human_judge_retry`: `Failed` →
//!    `Pending` + `retry_judged` 审计 (署名来自解析链), **不自动唤醒** (防雪崩);
//! 3. **拒绝即帧** —— 非 failed 态 409 / 任务不存在 404, 队列状态分毫不动;
//! 4. **附从句柄零扰动** —— 面板句柄不重复崩溃恢复 (在跑任务不被重派,
//!    事件账不假增)。

use std::sync::Arc;

use apeireth_cli::gateway_panels::CliTaskBoard;
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices,
    GatewayTokenGate,
};
use apeireth_orchestration::task_queue::{
    EnqueueOutcome, InMemoryTaskStore, TaskCaller, TaskEvent, TaskQueue, TaskQueueConfig, TaskSpec,
    TaskState, TaskStore,
};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// 网关 HTTP 入口装配 (任务面板口指向同一任务店 = 与工具面同队同账)。
async fn router_over(store: Arc<dyn TaskStore>) -> axum::Router {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let services = GatewayServices {
        tasks: Some(Arc::new(CliTaskBoard::attach(store))),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::UnprotectedDefault;
    canonical_router_with_state(state)
}

async fn call(
    router: axum::Router,
    method: &str,
    uri: &str,
    body: Body,
) -> (StatusCode, serde_json::Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .unwrap();
    request
        .headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let body = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    (status, body)
}

async fn snapshot(router: axum::Router) -> (StatusCode, serde_json::Value) {
    call(router, "GET", "/v1/panel/tasks", Body::empty()).await
}

async fn retry(
    router: axum::Router,
    task_id: &str,
    reason: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let payload = match reason {
        Some(reason) => serde_json::json!({ "reason": reason }),
        None => serde_json::json!({}),
    };
    call(
        router,
        "POST",
        &format!("/v1/panel/tasks/{task_id}/human-judge-retry"),
        Body::from(payload.to_string()),
    )
    .await
}

/// 1. 真值快照: 六态计数与队列同真值, 行字段上真记录, 时间线是事件账投影。
#[tokio::test]
async fn the_panel_snapshot_projects_the_real_queue_truth() {
    let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());
    let queue = TaskQueue::open(Arc::clone(&store), TaskQueueConfig::default()).unwrap();
    let human = TaskCaller::human("owner");
    let worker = TaskCaller::agent("worker-a");

    let EnqueueOutcome::Enqueued { task: failed, .. } = queue
        .enqueue(TaskSpec::new(
            "校对发布清单",
            serde_json::json!({"step": 3}),
        ))
        .unwrap()
    else {
        panic!("必须是新任务");
    };
    queue
        .assign(&failed.id, "worker-a", &worker)
        .expect("assign");
    queue.begin_execution(&failed.id, &worker).expect("begin");
    queue
        .human_judge_fail(&failed.id, &human, "验收不过")
        .expect("fail");
    let EnqueueOutcome::Enqueued { task: pending, .. } = queue
        .enqueue(TaskSpec::new("整理题库", serde_json::Value::Null))
        .unwrap()
    else {
        panic!("必须是新任务");
    };
    let EnqueueOutcome::Enqueued { task: running, .. } = queue
        .enqueue(TaskSpec::new("跑批", serde_json::Value::Null).with_assignee("worker-a"))
        .unwrap()
    else {
        panic!("必须是新任务");
    };
    queue.begin_execution(&running.id, &worker).expect("begin");
    queue.complete(&running.id, &worker).expect("complete");

    let (status, body) = snapshot(router_over(Arc::clone(&store)).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // 分态计数 = 队列真值 (一次快照读里同出)。
    let counts = queue.counts().unwrap();
    assert_eq!(body["counts"]["failed"], counts.failed);
    assert_eq!(body["counts"]["pending"], counts.pending);
    assert_eq!(body["counts"]["done"], counts.done);
    assert_eq!(body["counts"]["total"], 3);

    let tasks = body["tasks"].as_array().expect("tasks");
    assert_eq!(tasks.len(), 3);
    let failed_row = tasks
        .iter()
        .find(|row| row["id"] == failed.id.as_str())
        .expect("failed 行");
    assert_eq!(failed_row["title"], "校对发布清单");
    assert_eq!(failed_row["state"], "failed");
    assert_eq!(failed_row["assignee"], "worker-a");
    assert_eq!(failed_row["attempts"], 1);
    assert_eq!(failed_row["payload"]["step"], 3);

    // 时间线 = 该任务的事件账投影 (入队 / 指派 / 执行 / 人判失败都在)。
    let kinds: Vec<&str> = failed_row["timeline"]
        .as_array()
        .expect("timeline")
        .iter()
        .map(|event| event["kind"].as_str().expect("kind"))
        .collect();
    assert!(kinds.contains(&"enqueued"), "{kinds:?}");
    assert!(kinds.contains(&"assigned"), "{kinds:?}");
    assert!(kinds.contains(&"execution_started"), "{kinds:?}");
    assert!(kinds.contains(&"failure_judged"), "{kinds:?}");
    let judged = failed_row["timeline"]
        .as_array()
        .expect("timeline")
        .iter()
        .find(|event| event["kind"] == "failure_judged")
        .expect("人判失败事件");
    assert!(
        judged["detail"]
            .as_str()
            .expect("detail")
            .contains("验收不过"),
        "{judged}"
    );
    // 只投影本任务的事件 (不串线)。
    assert!(failed_row["timeline"]
        .as_array()
        .expect("timeline")
        .iter()
        .all(|event| event["detail"]
            .as_str()
            .map(|detail| !detail.contains(&pending.id))
            .unwrap_or(true)));
    let _ = pending;
}

/// 2. 人判重试链 (面板→API→队列真重试): Failed → Pending + `retry_judged` 审计,
///    不自动唤醒 (显式动作防雪崩)。
#[tokio::test]
async fn human_judge_retry_is_a_real_queue_transition_with_audit() {
    let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());
    let queue = TaskQueue::open(Arc::clone(&store), TaskQueueConfig::default()).unwrap();
    let human = TaskCaller::human("owner");
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("修好了再试", serde_json::Value::Null).with_assignee("worker-a"))
        .unwrap()
    else {
        panic!("必须是新任务");
    };
    queue
        .human_judge_fail(&task.id, &human, "验收不过")
        .expect("fail");
    let wakes_before = queue
        .journal()
        .unwrap()
        .iter()
        .filter(
            |event| matches!(event, TaskEvent::WakeEmitted { task_id, .. } if task_id == &task.id),
        )
        .count();

    let (status, body) = retry(
        router_over(Arc::clone(&store)).await,
        &task.id,
        Some("缺口已补"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["task"]["id"], task.id.as_str());
    assert_eq!(
        body["task"]["state"], "pending",
        "人判重试 = Failed → Pending"
    );

    // 队列真值: 状态真转移 + retry_judged 审计带署名与理由。
    let record = queue.get(&task.id).expect("get");
    assert_eq!(record.state, TaskState::Pending);
    let journal = queue.journal().unwrap();
    let retried = journal
        .iter()
        .find_map(|event| match event {
            TaskEvent::RetryJudged {
                task_id,
                judge,
                reason,
                ..
            } if task_id == &task.id => Some((judge.clone(), reason.clone())),
            _ => None,
        })
        .expect("retry_judged 审计");
    assert_eq!(retried.0, "anonymous", "署名来自解析链 (未设防口径如实)");
    assert_eq!(retried.1, "缺口已补");
    // 显式动作不自动唤醒 (回队待派, 再派走 assign) —— 防雪崩: 重试前后唤醒数不变。
    let wakes_after = journal
        .iter()
        .filter(
            |event| matches!(event, TaskEvent::WakeEmitted { task_id, .. } if task_id == &task.id),
        )
        .count();
    assert_eq!(wakes_after, wakes_before, "人判重试不自动唤醒");
}

/// 3. 拒绝即帧: 非 failed 态 409 / 不存在 404, 队列状态分毫不动。
#[tokio::test]
async fn retry_refuses_non_failed_and_missing_tasks_without_touching_the_queue() {
    let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());
    let queue = TaskQueue::open(Arc::clone(&store), TaskQueueConfig::default()).unwrap();
    let worker = TaskCaller::agent("worker-a");
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("在跑任务", serde_json::Value::Null).with_assignee("worker-a"))
        .unwrap()
    else {
        panic!("必须是新任务");
    };
    queue.begin_execution(&task.id, &worker).expect("begin");
    let journal_before = queue.journal().unwrap().len();

    let (status, body) = retry(router_over(Arc::clone(&store)).await, &task.id, None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body["error"]["message"]
        .as_str()
        .expect("message")
        .contains("只有 failed 能重试"));

    let (status, body) = retry(router_over(Arc::clone(&store)).await, "task-404", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

    // 队列分毫不动: 状态照旧, 事件账不增 (拒绝不落假账)。
    let record = queue.get(&task.id).expect("get");
    assert_eq!(record.state, TaskState::Running);
    assert_eq!(queue.journal().unwrap().len(), journal_before);
}

/// 4. 附从句柄零扰动: 面板句柄不重复崩溃恢复 (在跑/在派任务不被重派,
///    事件账不假增)。
#[tokio::test]
async fn the_attached_panel_handle_never_disturbs_the_live_queue() {
    let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());
    let queue = TaskQueue::open(Arc::clone(&store), TaskQueueConfig::default()).unwrap();
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("在派任务", serde_json::Value::Null).with_assignee("worker-a"))
        .unwrap()
    else {
        panic!("必须是新任务");
    };
    let journal_before = queue.journal().unwrap().len();

    // 装配面板句柄 + 读一帧快照。
    let (status, _) = snapshot(router_over(Arc::clone(&store)).await).await;
    assert_eq!(status, StatusCode::OK);

    let record = queue.get(&task.id).expect("get");
    assert_eq!(
        record.state,
        TaskState::Assigned,
        "附从句柄不重派在派任务 (恢复只归主句柄的 open)"
    );
    assert_eq!(record.redelivered, 0, "不假记重复投递");
    assert_eq!(
        queue.journal().unwrap().len(),
        journal_before,
        "不产生任何恢复审计"
    );
}
