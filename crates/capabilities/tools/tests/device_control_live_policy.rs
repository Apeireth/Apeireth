//! 设备受控执行面 (`tool.device_control`) 的**宿主授权同源**行为钉子
//! (2026-11-01 安全审计 P3-6)。
//!
//! 原先错在哪: 治理钩子 (分级审批档) 与 stage-1 闸 (风险映射) 各持一份**工具
//! 自建**的策略, 与治理链正在评估的那一份**不是同一个对象** ⇒ 面板上的热撤销
//! (`PermissionPolicy::revoke`) 对这个面**无效**, 且从外面**看不出来**。
//!
//! 现在钉住的行为 (口径钉死处都断言精确值, 演进须改测试):
//! * 宿主策略装进共享槽 ⇒ **撤销 ⇒ 两道闸都拒绝即帧 (不动设备)**;
//! * 宿主策略**未装入** ⇒ 沿用本面原有口径 (分级档位照旧) ⇒ 零回归。
//!
//! ⚠️ **宿主拍板链不动**: execute 拍的放行证据仍只认宿主决定账 / 推送账
//! (`device/approval.rs`); 本文件只钉**授权闸**, 不碰那条链。

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, Permission, PermissionPolicy,
};
use apeireth_protocol::canonical::ToolCall;
use serde_json::{json, Value};

use apeireth_tools_canonical::device::device_control::{
    DeviceControlApprovalHook, DeviceControlRiskMappingHook, DEVICE_CONTROL_CAPABILITY_ID,
};
use apeireth_tools_canonical::live_policy::LivePermissionPolicySlot;
use apeireth_tools_canonical::{PreExecuteHook, PreExecuteRequest, PreVerdict};

// ---------------------------------------------------------------------------
// 夹具 (策略 / 槽 / 请求)
// ---------------------------------------------------------------------------

/// 宿主策略: 授权 `tool.device_control` (可随时热撤销)。
fn host_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(Permission::ExecuteTool(
            DEVICE_CONTROL_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 槽: 宿主策略**尚未装入** ⇒ 各面**不表态** ⇒ 零回归。
fn empty_slot() -> LivePermissionPolicySlot {
    Arc::new(Mutex::new(None))
}

/// 槽: 装入宿主策略 (与治理链同源 ⇒ 热撤销可见)。
fn slot_with(policy: &Arc<Mutex<PermissionPolicy>>) -> LivePermissionPolicySlot {
    Arc::new(Mutex::new(Some(Arc::clone(policy))))
}

/// 热撤销 (面板 `POST /v1/panel/grants/revoke` 同款)。
fn revoke(policy: &Arc<Mutex<PermissionPolicy>>) {
    assert!(
        policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .revoke(&Permission::ExecuteTool(
                DEVICE_CONTROL_CAPABILITY_ID.to_string()
            )),
        "热撤销必须真的撤掉那份授权"
    );
}

fn dispatch(args: Value) -> GovernanceRequest<'static> {
    let capability: &'static CapabilityId = Box::leak(Box::new(
        CapabilityId::new(DEVICE_CONTROL_CAPABILITY_ID).expect("capability id"),
    ));
    let arguments: &'static Value = Box::leak(Box::new(args));
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

/// execute 拍 + 可逆动作 (reversible 档 = 审批档: 每次人批, 可配自动档)。
fn execute_args() -> Value {
    json!({ "action": "execute", "device": "lamp", "device_action": "power_on" })
}

fn pre_request() -> (ToolCall, CapabilityId) {
    (
        ToolCall {
            id: "call_1".into(),
            name: "device_control".into(),
            arguments: execute_args(),
        },
        CapabilityId::new(DEVICE_CONTROL_CAPABILITY_ID).expect("capability id"),
    )
}

// ---------------------------------------------------------------------------
// 用例
// ---------------------------------------------------------------------------

/// ① 宿主策略**未装入** ⇒ 分级审批档钩子沿用本面原有口径 (reversible 每次人批)。
#[tokio::test]
async fn approval_hook_keeps_its_grade_tier_when_the_host_policy_is_not_installed() {
    let hook = DeviceControlApprovalHook::with_slot(empty_slot());

    let decision = hook.evaluate(&dispatch(execute_args())).await;
    assert!(
        matches!(decision, Decision::RequireApproval { .. }),
        "reversible 档必须照旧每次人批 (槽为 None ⇒ 不表态): {decision:?}"
    );

    // plan / preview 是零副作用读面 ⇒ 照旧放行。
    let decision = hook
        .evaluate(&dispatch(
            json!({ "action": "plan", "device": "lamp", "device_action": "power_on" }),
        ))
        .await;
    assert!(matches!(decision, Decision::Allow), "{decision:?}");
}

/// ② 宿主撤销授权 ⇒ 分级审批档钩子**拒绝即帧** (热撤销对本面生效)。
#[tokio::test]
async fn approval_hook_denies_after_the_host_revokes() {
    let policy = host_policy();
    let hook = DeviceControlApprovalHook::with_slot(slot_with(&policy));

    // 撤销前: 照旧按分级索要审批。
    let decision = hook.evaluate(&dispatch(execute_args())).await;
    assert!(
        matches!(decision, Decision::RequireApproval { .. }),
        "撤销前照旧按分级审批: {decision:?}"
    );

    revoke(&policy);

    let decision = hook.evaluate(&dispatch(execute_args())).await;
    let Decision::Deny { reason } = decision else {
        panic!("撤销后必须拒绝即帧: {decision:?}");
    };
    assert!(reason.contains(DEVICE_CONTROL_CAPABILITY_ID), "{reason}");
    assert!(reason.contains("撤销"), "{reason}");
}

/// ③ stage-1 闸: 宿主策略**未装入** ⇒ 沿用本面自建口径 (放行) ⇒ 零回归。
#[test]
fn stage_one_gate_keeps_its_verdict_when_the_host_policy_is_not_installed() {
    let hook = DeviceControlRiskMappingHook::with_slot(empty_slot());
    let (call, capability) = pre_request();
    assert_eq!(
        hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
        PreVerdict::Allow,
        "宿主策略未装入时 stage-1 必须照旧放行 (零回归)"
    );
}

/// ④ stage-1 闸: 宿主撤销授权 ⇒ 拒绝即帧 (热撤销对本面生效, 不动设备)。
#[test]
fn stage_one_gate_denies_after_the_host_revokes() {
    let policy = host_policy();
    let hook = DeviceControlRiskMappingHook::with_slot(slot_with(&policy));
    let (call, capability) = pre_request();

    assert_eq!(
        hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
        PreVerdict::Allow,
        "宿主已授权时 stage-1 放行"
    );

    revoke(&policy);

    let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
    let PreVerdict::Ask { reason } = verdict else {
        panic!("撤销后 stage-1 必须拒绝即帧: {verdict:?}");
    };
    assert!(reason.contains(DEVICE_CONTROL_CAPABILITY_ID), "{reason}");
    assert!(reason.contains("撤销"), "{reason}");
}

/// ⑤ `new(policy)` 构造器: **签名不变**, 读的仍是自己那份策略。
///
/// 口径钉死: 该策略说「否」⇒ **拒绝即帧** (fail-closed), 该策略说「行」⇒ 放行。
#[test]
fn risk_mapping_new_constructor_still_reads_the_policy_it_was_given() {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    let hook = DeviceControlRiskMappingHook::new(Arc::clone(&policy));
    let (call, capability) = pre_request();

    let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
    assert!(
        matches!(verdict, PreVerdict::Ask { .. }),
        "构造时给的那份策略未授权 ⇒ 必须拒绝即帧 (fail-closed): {verdict:?}"
    );

    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(Permission::ExecuteTool(
            DEVICE_CONTROL_CAPABILITY_ID.to_string(),
        ));
    assert_eq!(
        hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
        PreVerdict::Allow,
        "构造时给的那份策略被授予后必须放行 (证明读的是它)"
    );
}
