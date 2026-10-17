//! 任务簿旋钮 (`APEIRETH_ENABLE_TASK_BOARD`, V6-a) 的治理接线回归:
//! 注册即闸 (默认关 fail-closed) + 写面审批档 (写面 = write_intent 同族每次
//! 人批, `task_list` 只读零审批; failed 只能人判, 无 failed/retry 工具面)。
//!
//! 覆盖: 旋钮链 (关 = 不在授权面, 拒绝即帧) / 任务工具治理档 (写面停审批、
//! 只读列放行、未知动作 fail-closed)。

use std::sync::Mutex;

use apeireth_cli::{
    build_production_governance_from_env, task_board_enabled_from_env, ENABLE_TASK_BOARD_ENV,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_runtime_assembly::{
    ACTION_TASK_ADD, ACTION_TASK_COMPLETE, ACTION_TASK_LIST, ACTION_TASK_REASSIGN,
    TASK_BOARD_CAPABILITY_ID,
};
use serde_json::json;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    previous: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn guard(keys: &'static [&'static str]) -> Self {
        let previous = keys
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        let guard = Self { previous };
        for key in keys {
            std::env::remove_var(key);
        }
        guard
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

async fn verdict(action: serde_json::Value) -> Decision {
    let governance = build_production_governance_from_env();
    let capability = CapabilityId::new(TASK_BOARD_CAPABILITY_ID).expect("capability id");
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &action,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    governance.evaluate(&request).await
}

/// 旋钮默认关 (fail-closed): 任务簿不在授权面, 派发即拒 (不是审批)。
#[tokio::test]
async fn the_task_board_knob_is_off_by_default_and_grants_nothing() {
    let _lock = lock_env();
    let _env = EnvGuard::guard(&[ENABLE_TASK_BOARD_ENV]);

    assert!(!task_board_enabled_from_env());
    let decision = verdict(json!({ "action": ACTION_TASK_LIST })).await;
    assert!(
        !matches!(decision, Decision::Allow),
        "未授权的任务簿调用不得放行: {decision:?}"
    );
}

/// 旋钮开 = 注册 + 授权 + 写面审批档 (任务工具治理档): 写面停审批,
/// 只读列放行, 未知动作 fail-closed。
#[tokio::test]
async fn enabling_the_knob_grants_the_face_and_stops_writes_at_approval() {
    let _lock = lock_env();
    let _env = EnvGuard::guard(&[ENABLE_TASK_BOARD_ENV]);
    std::env::set_var(ENABLE_TASK_BOARD_ENV, "1");
    assert!(task_board_enabled_from_env());

    for action in [ACTION_TASK_ADD, ACTION_TASK_REASSIGN, ACTION_TASK_COMPLETE] {
        let decision = verdict(json!({ "action": action })).await;
        let Decision::RequireApproval { reason } = decision else {
            panic!(
                "{action} 必须停人工审批 (任务簿写面 = 写文件, write_intent 同族): {decision:?}"
            );
        };
        assert!(reason.contains("write_intent 同族"), "{reason}");
    }
    assert_eq!(
        verdict(json!({ "action": ACTION_TASK_LIST })).await,
        Decision::Allow,
        "task_list 只读零审批"
    );
    // 未知动作 fail-closed 停审批 (不静默放行)。
    assert!(matches!(
        verdict(json!({ "action": "task_fail" })).await,
        Decision::RequireApproval { .. }
    ));
}
