//! TP20-N20 ApprovalBridge 集成测试 (standalone integration test).
//!
//! **目的**: companion `lib` 测试有 3 个 pre-existing E0599 错误 (tool_bridge.rs 调用 `.registry()`
//! 方法不存在 — 与本任务无关, 是 N2 OneRing task a284d5a7 在 master HEAD ff3f6d10 上未解决的
//! WIP 副作用). 本文件作为独立 integration test 验证 bridge 集成, 不依赖 `lib.rs` 内部测试模块.
//!
//! **覆盖**:
//! - t01: 缺字段的 ApprovalRequest → bridge 返回 MissingField (不 panic)
//! - t02: companion → bridge → orchestrator on_request 回调路由 (状态双向同步)
//! - t03: bridge 失败 (无回调) → 走 Reject 降级, 本地 status 写为 rejected
//! - t04: companion mark_approved → bridge.dispatch_response → orchestrator 收到
//! - t05: 0 装 PASS — bridge 不可达不影响主路径 (本地落库仍成功)
//! - t06: ApprovalRequest serde round-trip 字段透传保真
//! - t07: ApprovalRequest 未知字段进 extra (升级期兼容)

use apeireth_companion::approval_requests::{list, mark_approved, record_request};
use apeireth_memory::SqliteMemoryStore;
use apeireth_team_lead::{
    ApprovalBridge, ApprovalRequest as WireApprovalRequest,
    ApprovalResponse as WireApprovalResponse, InProcessBridge,
};
use serde_json::json;
use std::sync::Arc;

fn store() -> Arc<SqliteMemoryStore> {
    Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
}

// t01: 缺字段的 ApprovalRequest 通过 bridge 触发 MissingField, 不 panic
#[test]
fn t01_bridge_missing_field_not_panics() {
    let bridge = InProcessBridge::new();
    let bad = WireApprovalRequest {
        chain: "".into(),
        tool: "FileOperator".into(),
        args_preview: "{}".into(),
        reason: "x".into(),
        created_at: 0,
        extra: Default::default(),
    };
    let err = bridge.dispatch_request(bad).unwrap_err();
    assert!(
        matches!(
            err,
            apeireth_team_lead::ApprovalBridgeError::MissingField(_)
        ),
        "expected MissingField, got {err:?}"
    );
}

// t02: companion record_request + bridge.on_request 回调路由 (状态双向同步)
#[test]
fn t02_companion_to_orchestrator_two_way_sync() {
    let s = store();
    let bridge = Arc::new(InProcessBridge::new());
    // 注册回调: 模拟 orchestrator 自动批准
    bridge.on_request(|req| WireApprovalResponse {
        chain: req.chain.clone(),
        decision: "approved".into(),
        decided_at: 1_700_000_999,
        note: "auto-approve".into(),
        extra: Default::default(),
    });
    let bridge_ref: Arc<dyn ApprovalBridge> = bridge.clone();
    record_request(
        &s,
        "FileOperator",
        &json!({"op": "write", "path": "ok.txt"}),
        "x",
        Some(&bridge_ref),
    );
    // 回调返回 approved, apply_wire_response 写回本地 approved
    let approved = list(&s, Some("approved"));
    assert_eq!(
        approved.len(),
        1,
        "bridge.approved 必须写回本地 approved 列表"
    );
    assert!(approved[0].updated_at >= 1_700_000_999);
    // bridge 收到 1 请求 + 1 响应 (回调自动 dispatch)
    assert_eq!(bridge.received_requests().len(), 1);
    assert_eq!(bridge.received_responses().len(), 1);
}

// t03: bridge 无回调默认 reject — 0 装 PASS, 真实状态写回本地 (不假装已批准)
#[test]
fn t03_no_callback_default_rejects_writes_rejected_status() {
    let s = store();
    let bridge = Arc::new(InProcessBridge::new());
    let bridge_ref: Arc<dyn ApprovalBridge> = bridge.clone();
    record_request(
        &s,
        "ShellExec",
        &json!({"cmd": "rm -rf /"}),
        "高危, 需主人批准",
        Some(&bridge_ref),
    );
    // bridge 收到 1 个请求
    assert_eq!(bridge.received_requests().len(), 1);
    // 默认 reject → 本地状态变 rejected (append-only)
    let rejected = list(&s, Some("rejected"));
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].tool, "ShellExec");
    // pending 列表空 (被 reject 后移出 pending 视图)
    let pending = list(&s, Some("pending"));
    assert_eq!(pending.len(), 0);
}

// t04: companion mark_approved → bridge.dispatch_response → orchestrator 收到
#[test]
fn t04_mark_approved_dispatches_response_to_bridge() {
    let s = store();
    let bridge = Arc::new(InProcessBridge::new());
    let bridge_ref: Arc<dyn ApprovalBridge> = bridge.clone();
    // 先 record_request (无 bridge 时是 pending)
    record_request(&s, "ShellExec", &json!({"cmd": "ls"}), "需要批准", None);
    let pending = list(&s, Some("pending"));
    assert_eq!(pending.len(), 1);
    let chain = pending[0].chain.clone();

    // mark_approved 带 bridge, 触发 dispatch_response
    mark_approved(&s, &chain, Some(&bridge_ref)).unwrap();

    let responses = bridge.received_responses();
    let mark_resp = responses
        .iter()
        .find(|r| r.chain == chain && r.decision == "approved");
    assert!(
        mark_resp.is_some(),
        "mark_approved 必须 dispatch 1 个 approved 响应"
    );

    // 本地状态 approved
    let approved = list(&s, Some("approved"));
    assert_eq!(approved.len(), 1);
}

// t05: bridge 传 None (向后兼容老调用点) — 不影响本地落库
#[test]
fn t05_bridge_none_does_not_break_main_path() {
    let s = store();
    record_request(&s, "FileOperator", &json!({"op": "rm"}), "x", None);
    assert_eq!(list(&s, Some("pending")).len(), 1);
    let first = list(&s, Some("pending"))[0].clone();
    mark_approved(&s, &first.chain, None).unwrap();
    assert_eq!(list(&s, Some("approved")).len(), 1);
}

// t06: ApprovalRequest serde round-trip 字段透传保真
#[test]
fn t06_wire_request_serde_roundtrip() {
    let req = WireApprovalRequest {
        chain: "chain-001".into(),
        tool: "FileOperator".into(),
        args_preview: r#"{"op":"write"}"#.into(),
        reason: "needs owner".into(),
        created_at: 1_700_000_000,
        extra: {
            let mut m = serde_json::Map::new();
            m.insert("trace_id".into(), serde_json::json!(42));
            m
        },
    };
    let json = serde_json::to_string(&req).unwrap();
    // wire format snake_case
    assert!(json.contains("\"chain\""));
    assert!(json.contains("\"tool\""));
    assert!(json.contains("\"args_preview\""));
    assert!(json.contains("\"created_at\""));
    // 未知字段进 extra, 不丢
    assert!(json.contains("\"trace_id\""));

    let parsed: WireApprovalRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, req);
}

// t07: 未知字段进 extra, 升级期兼容
#[test]
fn t07_unknown_fields_go_to_extra() {
    let json = r#"{
        "chain": "c",
        "tool": "T",
        "args_preview": "{}",
        "reason": "x",
        "created_at": 0,
        "future_field_a": "future_value",
        "future_field_b": 99
    }"#;
    let req: WireApprovalRequest = serde_json::from_str(json).unwrap();
    assert_eq!(
        req.extra.get("future_field_a").unwrap(),
        &serde_json::json!("future_value")
    );
    assert_eq!(
        req.extra.get("future_field_b").unwrap(),
        &serde_json::json!(99)
    );
}
