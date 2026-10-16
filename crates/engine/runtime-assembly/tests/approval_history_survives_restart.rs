//! C-05 审批持久化证据: 审批历史随**生产会话库**跨"重启"存活。
//!
//! 外部审计 C-05（标注 ⚠未二次核验）称 `InMemorySessionStore` 为默认、重启丢
//! 审批历史。读码实况: 组合根（`build_canonical_runtime_with_sessions_from_env`）
//! 把生产运行时组装到 `SqliteSessionStore`（CLI 与 `gateway serve` 同一条装配
//! 线，桌面侧车即其子进程）; `InMemorySessionStore` 只是内核 builder 的惰性缺省
//! （测试/零装配用）。本套件把审计主张正面钉住: 在文件型生产会话库里，
//! 审批记录（待批 + 已决）与审批事件链在关闭重开（= 进程重启）后原样存活。

use apeireth_core::kernel::{
    system_clock, ApprovalId, CapabilityId, RequestId, SessionId, Timestamp, TraceId,
};
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime::canonical::{
    approval_arguments_summary, approval_command_text, ApprovalStatus, FrozenTurnContinuation,
    PendingApproval, Session, SessionEventKind, SessionStore,
};
use apeireth_runtime_assembly::SqliteSessionStore;

/// 一条 `Remove-Item x.txt`（删除族）待批记录, 形状与执行面冻结出的记录一致。
fn pending_approval(
    session_id: SessionId,
    approval_id: ApprovalId,
    request_id: RequestId,
    trace_id: TraceId,
) -> PendingApproval {
    let call = ToolCall {
        id: "call_1".into(),
        name: "shell".into(),
        arguments: serde_json::json!({ "command": "Remove-Item x.txt" }),
    };
    let created = Timestamp::from_epoch_millis(1_700_000_000_000).unwrap();
    PendingApproval {
        approval_id,
        session_id,
        request_id,
        trace_id,
        round: 1,
        capability_id: CapabilityId::new("tool.example").unwrap(),
        tool_name: "shell".into(),
        tool_call: call.clone(),
        effective_invocation: None,
        governance_hook: "hook".into(),
        governance_reason: "approval list matched [family:删除族]".into(),
        command_text: approval_command_text("shell", &call, None),
        arguments_summary: approval_arguments_summary("shell", &call),
        authorization_families: vec!["删除族".to_string()],
        operation_fingerprint: "fp-approval-history".into(),
        created_at: created,
        expires_at: Timestamp::from_epoch_millis(1_700_000_300_000).unwrap(),
        status: ApprovalStatus::Pending,
        continuation: FrozenTurnContinuation::start_of_round(request_id, trace_id, "m", 1),
        human_reason: None,
        generated_text: String::new(),
        rounds_used: 1,
    }
}

/// 待批记录 + `ApprovalRequired` 事件落进文件库, 关闭重开（= 进程重启）后
/// 原样读回: 审批历史不丢、状态不漂移。
#[tokio::test]
async fn pending_approval_history_survives_a_session_store_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.sqlite3");
    let clock = system_clock();
    let session_id = SessionId::new();
    let approval_id = ApprovalId::new();
    let (request_id, trace_id) = (RequestId::new(), TraceId::new());

    // 第一代进程: 冻结一条待批记录并落盘。
    {
        let store = SqliteSessionStore::open(&path).await.unwrap();
        let mut session = Session::new(session_id, clock.as_ref());
        session.record(
            request_id,
            trace_id,
            SessionEventKind::ApprovalRequired {
                hook: "hook".into(),
                action: "capability_dispatch".into(),
                reason: "approval list matched [family:删除族]".into(),
                round: 1,
                approval_id,
            },
            clock.as_ref(),
        );
        session.approvals.insert(
            approval_id,
            pending_approval(session_id, approval_id, request_id, trace_id),
        );
        session.active_approval_id = Some(approval_id);
        store.save(&session).await.unwrap();
    }

    // 第二代进程: 重开文件库（= 重启）后审批历史仍在。
    let store = SqliteSessionStore::open(&path).await.unwrap();
    let reloaded = store
        .load(&session_id)
        .await
        .unwrap()
        .expect("session survives");
    assert_eq!(
        reloaded.approvals.len(),
        1,
        "审批历史必须跨重启存活（审计 C-05 主张的反证）"
    );
    let restored = &reloaded.approvals[&approval_id];
    assert_eq!(restored.status, ApprovalStatus::Pending, "状态不漂移");
    assert_eq!(restored.tool_name, "shell");
    assert_eq!(
        restored.operation_fingerprint, "fp-approval-history",
        "冻结指纹原样读回"
    );
    assert_eq!(
        restored.effective_authorization_families(),
        vec!["删除族".to_string()],
        "审批族标注随记录存活"
    );
    assert_eq!(
        reloaded.active_approval_id,
        Some(approval_id),
        "待批指针存活"
    );
    assert_eq!(reloaded.events.len(), 1, "审批事件链完整");
    match &reloaded.events[0].event {
        SessionEventKind::ApprovalRequired {
            approval_id: event_id,
            ..
        } => assert_eq!(*event_id, approval_id),
        other => panic!("expected ApprovalRequired, got {other:?}"),
    }
}

/// 决议后（终态 + `ApprovalResolved` 事件 + 人工理由）再重启一轮, 审批链
/// 两段历史（待批 → 已决）都完整: 用户重启后审批历史不消失。
#[tokio::test]
async fn resolved_approval_history_survives_another_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sessions.sqlite3");
    let clock = system_clock();
    let session_id = SessionId::new();
    let approval_id = ApprovalId::new();
    let (request_id, trace_id) = (RequestId::new(), TraceId::new());

    // 第一代进程: 待批落盘。
    {
        let store = SqliteSessionStore::open(&path).await.unwrap();
        let mut session = Session::new(session_id, clock.as_ref());
        session.record(
            request_id,
            trace_id,
            SessionEventKind::ApprovalRequired {
                hook: "hook".into(),
                action: "capability_dispatch".into(),
                reason: "approval list matched [family:删除族]".into(),
                round: 1,
                approval_id,
            },
            clock.as_ref(),
        );
        session.approvals.insert(
            approval_id,
            pending_approval(session_id, approval_id, request_id, trace_id),
        );
        session.active_approval_id = Some(approval_id);
        store.save(&session).await.unwrap();
    }

    // 第二代进程: 人工批准并执行收束, 决议落盘。
    {
        let store = SqliteSessionStore::open(&path).await.unwrap();
        let mut session = store.load(&session_id).await.unwrap().unwrap();
        let mut resolved = session.approvals[&approval_id].clone();
        resolved.status = ApprovalStatus::Consumed;
        resolved.human_reason = Some("确认删除".into());
        session.approvals.insert(approval_id, resolved);
        session.active_approval_id = None;
        session.record(
            request_id,
            trace_id,
            SessionEventKind::ApprovalResolved {
                approval_id,
                decision: "approved".into(),
                round: 1,
                human_reason: Some("确认删除".into()),
            },
            clock.as_ref(),
        );
        store.save(&session).await.unwrap();
    }

    // 第三代进程: 再次重启, 审批链两段历史齐全。
    let store = SqliteSessionStore::open(&path).await.unwrap();
    let reloaded = store
        .load(&session_id)
        .await
        .unwrap()
        .expect("session survives");
    let resolved = &reloaded.approvals[&approval_id];
    assert_eq!(resolved.status, ApprovalStatus::Consumed, "终态不漂移");
    assert_eq!(resolved.human_reason.as_deref(), Some("确认删除"));
    assert_eq!(reloaded.active_approval_id, None, "收束后无待批指针");
    assert_eq!(reloaded.events.len(), 2, "审批事件链两段齐全");
    assert!(matches!(
        reloaded.events[0].event,
        SessionEventKind::ApprovalRequired { .. }
    ));
    assert!(matches!(
        reloaded.events[1].event,
        SessionEventKind::ApprovalResolved { .. }
    ));
    assert_eq!(
        store.list().await.unwrap().len(),
        1,
        "面板面 list() 同样看到这条会话"
    );
}
