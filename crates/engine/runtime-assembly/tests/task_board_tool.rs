//! V6-a 任务簿工具面 (`tool.task_board`) 的治理档与行为证明。
//!
//! 钉住: ①任务工具治理档 (写面 = write_intent 同族每次人批 / 只读列零审批 /
//! 未知动作 fail-closed); ②failed 只能人判 (工具面无 failed/retry 动作);
//! ③认领授权 (task_complete 自报必须是执行者本人); ④幂等投递单执行;
//! ⑤自述任务节计数真值; ⑥注册条件 + 名册行。

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_orchestration::task_queue::{TaskQueue, TaskQueueConfig, TaskStore};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    task_board_status_probe, ProductionBackends, ProductionModules, ProductionModulesConfig,
    SqliteTaskStore, TaskBoardApprovalHook, TaskBoardTool, ACTION_TASK_ADD, ACTION_TASK_COMPLETE,
    ACTION_TASK_LIST, ACTION_TASK_REASSIGN, TASK_BOARD_ACTIONS, TASK_BOARD_CAPABILITY_ID,
};
use serde_json::{json, Value};

fn queue() -> Arc<TaskQueue> {
    let store: Arc<dyn TaskStore> = Arc::new(SqliteTaskStore::in_memory().expect("store"));
    Arc::new(TaskQueue::open(store, TaskQueueConfig::default()).expect("queue opens"))
}

fn call(id: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "task_board".into(),
        arguments,
    }
}

async fn invoke(tool: &TaskBoardTool, id: &str, arguments: Value) -> (bool, Value) {
    let result = tool.invoke(&call(id, arguments)).await;
    let ok = result.is_ok();
    let value: Value = serde_json::from_str(&result.render()).unwrap_or(Value::Null);
    (ok, value)
}

fn dispatch(action: Option<&str>) -> GovernanceRequest<'static> {
    let capability: &'static CapabilityId = Box::leak(Box::new(
        CapabilityId::new(TASK_BOARD_CAPABILITY_ID).expect("capability id"),
    ));
    let arguments: &'static Value = Box::leak(Box::new(match action {
        Some(action) => json!({ "action": action }),
        None => json!({}),
    }));
    GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability,
            arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    )
}

/// 任务工具治理档: 写面 (增/改派/完成) = write_intent 同族每次人批,
/// `task_list` 只读零审批, 未知动作 fail-closed 停审批。
#[tokio::test]
async fn the_task_board_governance_tier_stops_writes_and_keeps_the_read_face_free() {
    let hook = TaskBoardApprovalHook::new();
    for action in [ACTION_TASK_ADD, ACTION_TASK_REASSIGN, ACTION_TASK_COMPLETE] {
        let decision = hook.evaluate(&dispatch(Some(action))).await;
        let Decision::RequireApproval { reason } = decision else {
            panic!(
                "{action} 必须停人工审批 (任务簿写面 = 写文件, write_intent 同族): {decision:?}"
            );
        };
        assert!(reason.contains("write_intent 同族"), "{reason}");
        assert!(reason.contains("无自动放行档"), "{reason}");
        assert!(reason.contains("failed 只能人判"), "{reason}");
    }

    // 只读列自家任务态: 与 self_status 同类只读面, 零审批。
    assert_eq!(
        hook.evaluate(&dispatch(Some(ACTION_TASK_LIST))).await,
        Decision::Allow
    );

    // 动作缺失/未知 fail-closed 停审批 (不静默放行)。
    for action in [None, Some("task_fail"), Some("rm -rf /")] {
        let decision = hook.evaluate(&dispatch(action)).await;
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "{action:?} 必须 fail-closed 停审批: {decision:?}"
        );
    }

    // 本钩子只管自己的面。
    let capability = CapabilityId::new("tool.fetch").expect("capability id");
    let arguments = json!({ "url": "https://example.com" });
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    assert_eq!(
        hook.evaluate(&request).await,
        Decision::Allow,
        "其它能力不受本钩子影响"
    );
}

/// failed 只能人判: 工具面闭合词表里**没有** failed/retry 动作, 调用即拒绝即帧。
#[tokio::test]
async fn the_task_board_face_has_no_failed_or_retry_action() {
    for forbidden in ["task_fail", "failed", "task_retry", "retry"] {
        assert!(
            !TASK_BOARD_ACTIONS.contains(&forbidden),
            "{forbidden} 不得进任务簿工具面 (failed 只能人判, 重试是显式人判动作)"
        );
    }
    let tool = TaskBoardTool::new(queue());
    let (ok, _) = invoke(
        &tool,
        "call_fail",
        json!({ "action": "task_fail", "task_id": "task-1" }),
    )
    .await;
    assert!(!ok, "未知动作必须拒绝即帧");
}

/// 工具面全回路: 增/列/改派/完成 over 持久队列 (真 sqlite 内存库)。
#[tokio::test]
async fn task_board_add_list_reassign_complete_round_trip() {
    let tool = TaskBoardTool::new(queue());
    let worker_session = SessionId::new();
    let (ok, added) = invoke(
        &tool,
        "call_add",
        json!({
            "action": ACTION_TASK_ADD,
            "title": "写周报",
            "payload": {"note": "opaque"},
            "assignee": worker_session.to_string(),
        }),
    )
    .await;
    assert!(ok, "{added}");
    assert_eq!(added["ok"], true);
    assert_eq!(added["deduplicated"], false);
    assert_eq!(added["tasks"][0]["title"], "写周报");
    assert_eq!(added["tasks"][0]["state"], "assigned");
    assert_eq!(added["tasks"][0]["assignee"], worker_session.to_string());
    let task_id = added["tasks"][0]["id"].as_str().expect("id").to_string();

    // 列 (只读面)。
    let (ok, listed) = invoke(&tool, "call_list", json!({ "action": ACTION_TASK_LIST })).await;
    assert!(ok, "{listed}");
    assert_eq!(listed["tasks"].as_array().expect("tasks").len(), 1);
    assert_eq!(listed["write_face"], "read_only");

    // 改派 (assign 即唤醒)。
    let (ok, reassigned) = invoke(
        &tool,
        "call_reassign",
        json!({
            "action": ACTION_TASK_REASSIGN,
            "task_id": task_id,
            "assignee": "worker-2",
        }),
    )
    .await;
    assert!(ok, "{reassigned}");
    assert_eq!(reassigned["tasks"][0]["assignee"], "worker-2");

    // 完成 = 自报: 非执行者拒 (委托者不能伪造工人状态)。
    let stranger_session = SessionId::new();
    let result = tool
        .invoke_in_session(
            &call(
                "call_complete_stranger",
                json!({ "action": ACTION_TASK_COMPLETE, "task_id": task_id }),
            ),
            None,
            Some(stranger_session),
        )
        .await;
    assert!(!result.is_ok(), "非执行者不得自报完成: {}", result.render());

    // 改派回原会话, 本人自报完成。
    let (ok, _) = invoke(
        &tool,
        "call_reassign_back",
        json!({
            "action": ACTION_TASK_REASSIGN,
            "task_id": task_id,
            "assignee": worker_session.to_string(),
        }),
    )
    .await;
    assert!(ok);
    let result = tool
        .invoke_in_session(
            &call(
                "call_complete",
                json!({ "action": ACTION_TASK_COMPLETE, "task_id": task_id }),
            ),
            None,
            Some(worker_session),
        )
        .await;
    assert!(result.is_ok(), "{}", result.render());
    let done: Value = serde_json::from_str(&result.render()).expect("json");
    assert_eq!(done["tasks"][0]["state"], "done");
    assert_eq!(done["write_face"], "task_book_write (write_intent 同族)");
}

/// 幂等投递: 同 key 双投递 = 单任务单执行, 第二次如实报去重。
#[tokio::test]
async fn duplicate_add_with_the_same_key_is_deduplicated_to_one_task() {
    let tool = TaskBoardTool::new(queue());
    let (ok, first) = invoke(
        &tool,
        "call_add_1",
        json!({ "action": ACTION_TASK_ADD, "title": "只干一次", "idempotency_key": "job-7" }),
    )
    .await;
    assert!(ok, "{first}");
    let (ok, second) = invoke(
        &tool,
        "call_add_2",
        json!({ "action": ACTION_TASK_ADD, "title": "只干一次 (重投)", "idempotency_key": "job-7" }),
    )
    .await;
    assert!(ok, "{second}");
    assert_eq!(second["deduplicated"], true);
    assert_eq!(second["tasks"][0]["id"], first["tasks"][0]["id"]);
    let (ok, listed) = invoke(&tool, "call_list", json!({ "action": ACTION_TASK_LIST })).await;
    assert!(ok);
    assert_eq!(
        listed["tasks"].as_array().expect("tasks").len(),
        1,
        "同 key 重复投递不产生第二个任务"
    );
    assert_eq!(listed["tasks"][0]["redelivered"], 1);
}

/// 自述任务节: 分态计数与队列真值同源 (pending/running/failed 不编造)。
#[tokio::test]
async fn self_status_task_section_reports_real_counts_from_the_queue() {
    let queue = queue();
    let tool = TaskBoardTool::new(Arc::clone(&queue));
    invoke(
        &tool,
        "call_add",
        json!({ "action": ACTION_TASK_ADD, "title": "待派的一件" }),
    )
    .await;
    let (_, added) = invoke(
        &tool,
        "call_add_2",
        json!({ "action": ACTION_TASK_ADD, "title": "在跑的一件", "assignee": "worker-a" }),
    )
    .await;
    let running_id = added["tasks"][0]["id"].as_str().expect("id").to_string();
    queue
        .begin_execution(
            &running_id,
            &apeireth_orchestration::task_queue::TaskCaller::agent("worker-a"),
        )
        .expect("begin");

    let source =
        apeireth_runtime_assembly::ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_task_board_probe(task_board_status_probe(queue));
    let self_status = apeireth_tools_canonical::SelfStatusTool::new(Arc::new(source));
    let result = self_status
        .invoke(&ToolCall {
            id: "call_status".into(),
            name: "self_status".into(),
            arguments: json!({}),
        })
        .await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).expect("self report json");
    assert_eq!(report["task_board"]["pending"], 1);
    assert_eq!(report["task_board"]["running"], 1);
    assert_eq!(report["task_board"]["failed"], 0);
    assert_eq!(report["task_board"]["reason"], Value::Null);
}

/// 注册条件 + 名册行: 队列注入才注册, 名册照实报生效值 (0 装诚实)。
#[tokio::test]
async fn task_board_registers_only_when_a_queue_is_supplied_and_the_roster_says_so() {
    let frozen = || -> Arc<dyn apeireth_core::kernel::Clock> {
        Arc::new(apeireth_core::kernel::VirtualClock::new(
            apeireth_core::kernel::Timestamp::from_epoch_millis(1_786_838_400_000)
                .unwrap()
                .as_datetime(),
        ))
    };
    let base = || ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    };

    // 关 = 不注册, 名册照实 false。
    let off = base();
    let modules =
        ProductionModules::build(off, ProductionBackends::default(), frozen()).expect("build");
    assert!(modules
        .capabilities()
        .iter()
        .all(|capability| capability.id().as_str() != TASK_BOARD_CAPABILITY_ID));

    // 开 (注入队列) = 注册 + 名册 true + 自述任务节可读真值。
    let mut on = base();
    on.task_board = Some(queue());
    let modules =
        ProductionModules::build(on, ProductionBackends::default(), frozen()).expect("build");
    let ids: Vec<String> = modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().to_string())
        .collect();
    assert!(
        ids.iter().any(|id| id == TASK_BOARD_CAPABILITY_ID),
        "missing {TASK_BOARD_CAPABILITY_ID}: {ids:?}"
    );
    let self_status = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status");
    let result = self_status
        .invoke(&ToolCall {
            id: "call_status".into(),
            name: "self_status".into(),
            arguments: json!({}),
        })
        .await;
    let report: Value = serde_json::from_str(&result.render()).expect("self report json");
    assert_eq!(report["capabilities"]["task_board"], true);
    assert_eq!(report["task_board"]["pending"], 0);
}
