//! worktree 两级工具旋钮 (`APEIRETH_ENABLE_WORKTREE_PLAN` / `_APPLY`) 的治理
//! 接线回归 (总装批件一)。
//!
//! 覆盖: 开关 → 生效值映射 (子依赖主 + file_write 唯一写总闸, fail-closed) /
//! `tool.worktree_apply` 每次落盘停人工审批 (理由载荷 = 改单摘要) /
//! `tool.worktree_plan` 只读零审批 / 未开开关 = 工具未授权 (拒绝即帧)。

use std::sync::Mutex;

use apeireth_cli::{
    build_production_governance_from_env, worktree_apply_enabled_from_env,
    worktree_plan_enabled_from_env, ENABLE_FILE_WRITE_ENV, ENABLE_WORKTREE_APPLY_ENV,
    ENABLE_WORKTREE_PLAN_ENV,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, GovernancePipeline, GovernanceRequest};
use apeireth_tools_canonical::{WORKTREE_APPLY_CAPABILITY_ID, WORKTREE_PLAN_CAPABILITY_ID};
use serde_json::{json, Value};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    keys: &'static [&'static str],
    previous: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn guard(keys: &'static [&'static str]) -> Self {
        let previous = keys
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        Self { keys, previous }
    }

    fn clear(&self) {
        for key in self.keys {
            std::env::remove_var(key);
        }
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

fn worktree_env_guard() -> EnvGuard {
    EnvGuard::guard(&[
        ENABLE_FILE_WRITE_ENV,
        ENABLE_WORKTREE_PLAN_ENV,
        ENABLE_WORKTREE_APPLY_ENV,
    ])
}

async fn verdict(
    governance: &GovernancePipeline,
    capability_id: &str,
    arguments: &Value,
) -> apeireth_governance::GovernanceVerdict {
    let capability = CapabilityId::new(capability_id).expect("capability id");
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    governance.evaluate_verbose(&request).await
}

/// 一份通过计划拍校验的改单 + 补丁 (add 一件) —— 与 `worktree_plan` 产物同形。
fn valid_apply_arguments() -> Value {
    json!({
        "plan": {
            "goal": "新增一个说明文件",
            "files": [{ "path": "note.txt", "action": "add", "expect": "文件建成并含一行说明" }]
        },
        "patch": "*** Begin Patch\n*** Add File: note.txt\n+hello\n*** End Patch",
        "verify": []
    })
}

/// 开关即效接线: 子依赖主 + file_write 唯一写总闸, fail-closed。
#[test]
fn worktree_env_knobs_are_chained_behind_the_file_write_gate() {
    let _lock = lock_env();
    let guard = worktree_env_guard();
    guard.clear();

    assert!(
        !worktree_plan_enabled_from_env() && !worktree_apply_enabled_from_env(),
        "默认关 (fail-closed)"
    );

    // 只开写总闸: 两级仍关 (开关各自 opt-in)。
    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");
    assert!(
        !worktree_plan_enabled_from_env() && !worktree_apply_enabled_from_env(),
        "写总闸开不等于 worktree 面自动开"
    );

    // 计划拍主开关: 依赖写总闸 (子依赖主 + file_write)。
    std::env::set_var(ENABLE_WORKTREE_PLAN_ENV, "1");
    assert!(
        worktree_plan_enabled_from_env(),
        "写总闸 + 计划拍旋钮 = 计划拍生效"
    );
    assert!(!worktree_apply_enabled_from_env(), "落盘拍未开 = 不生效");

    // 落盘拍: 依赖计划拍主开关 + 写总闸 (两级成对)。
    std::env::set_var(ENABLE_WORKTREE_APPLY_ENV, "1");
    assert!(
        worktree_apply_enabled_from_env(),
        "两级旋钮齐 + 写总闸 = 落盘拍生效"
    );

    // 单独设落盘拍旋钮 = 无效果 (子依赖主)。
    std::env::remove_var(ENABLE_WORKTREE_PLAN_ENV);
    assert!(
        !worktree_plan_enabled_from_env() && !worktree_apply_enabled_from_env(),
        "落盘拍依赖计划拍主开关"
    );

    // 单独设两级旋钮而写总闸关 = 无效果 (子依赖 file_write)。
    std::env::set_var(ENABLE_WORKTREE_PLAN_ENV, "1");
    std::env::remove_var(ENABLE_FILE_WRITE_ENV);
    assert!(
        !worktree_plan_enabled_from_env() && !worktree_apply_enabled_from_env(),
        "两级都依赖 file_write 唯一写总闸"
    );
}

/// `tool.worktree_apply` 审批级: 每次落盘停人工审批 (无自动放行档), 理由载荷 =
/// 改单摘要; 计划拍校验不过的调用拒绝即帧 (不进审批)。
#[tokio::test]
async fn worktree_apply_sits_at_require_approval_with_the_change_order_summary() {
    let _lock = lock_env();
    let guard = worktree_env_guard();
    guard.clear();
    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");
    std::env::set_var(ENABLE_WORKTREE_PLAN_ENV, "1");
    std::env::set_var(ENABLE_WORKTREE_APPLY_ENV, "1");

    let governance = build_production_governance_from_env();
    let result = verdict(
        &governance,
        WORKTREE_APPLY_CAPABILITY_ID,
        &valid_apply_arguments(),
    )
    .await;
    assert!(
        matches!(
            result.decision,
            apeireth_governance::Decision::RequireApproval { .. }
        ),
        "每次落盘都必须停人工审批: {:?}",
        result.decision
    );
    assert_eq!(result.hook, "worktree_apply_write_risk");
    let reason = result.decision.reason().unwrap_or_default();
    assert!(
        reason.contains("每次都要人批") && reason.contains("验证命令"),
        "理由载荷必须是改单摘要: {reason}"
    );

    // 计划拍校验不过 = 拒绝即帧 (不进审批)。
    let drifted = json!({
        "plan": {
            "goal": "改单与补丁漂移",
            "files": [{ "path": "note.txt", "action": "add", "expect": "建" }]
        },
        "patch": "*** Begin Patch\n*** Add File: other.txt\n+x\n*** End Patch",
        "verify": []
    });
    let result = verdict(&governance, WORKTREE_APPLY_CAPABILITY_ID, &drifted).await;
    assert!(
        matches!(result.decision, apeireth_governance::Decision::Deny { .. }),
        "改单漂移必须拒绝即帧: {:?}",
        result.decision
    );
    let reason = result.decision.reason().unwrap_or_default();
    assert!(reason.contains("改单"), "拒绝理由指明漂移: {reason}");
}

/// `tool.worktree_plan` 是只读改单面: 零审批放行 (计划不落盘)。
#[tokio::test]
async fn worktree_plan_is_read_only_and_needs_no_approval() {
    let _lock = lock_env();
    let guard = worktree_env_guard();
    guard.clear();
    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");
    std::env::set_var(ENABLE_WORKTREE_PLAN_ENV, "1");

    let governance = build_production_governance_from_env();
    let arguments = json!({
        "goal": "新增一个说明文件",
        "files": [{ "path": "note.txt", "action": "add", "expect": "文件建成并含一行说明" }]
    });
    let result = verdict(&governance, WORKTREE_PLAN_CAPABILITY_ID, &arguments).await;
    assert!(
        result.is_allowed(),
        "只读改单面零审批: {:?} / {}",
        result.decision,
        result.hook
    );
}

/// 未开开关: 两级工具不在授权面 (fail-closed, 拒绝即帧)。
#[tokio::test]
async fn disabled_switches_leave_the_worktree_tools_unauthorized() {
    let _lock = lock_env();
    let guard = worktree_env_guard();
    guard.clear();

    let governance = build_production_governance_from_env();
    for capability_id in [WORKTREE_APPLY_CAPABILITY_ID, WORKTREE_PLAN_CAPABILITY_ID] {
        let result = verdict(&governance, capability_id, &valid_apply_arguments()).await;
        assert!(
            matches!(result.decision, apeireth_governance::Decision::Deny { .. }),
            "{capability_id} 未开开关必须拒绝即帧: {:?}",
            result.decision
        );
        assert_eq!(result.hook, "permission_governance", "{capability_id}");
    }
}
