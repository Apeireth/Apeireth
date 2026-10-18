//! 创作工具面 W1 图像操作旋钮 (`APEIRETH_ENABLE_IMAGE_OPS`) 的治理接线回归:
//! 受控文件写族 —— 子依赖主 + `file_write` 唯一写总闸 (fail-closed, 单独设本
//! 旋钮而写总闸关 = 无效果) + grant + require_approval (输出写归档文件写入族,
//! 每次图像写入都停人工审批, 无自动放行档)。

use std::sync::Mutex;

use apeireth_cli::{
    build_production_governance_from_env, image_ops_enabled_from_env, ENABLE_FILE_WRITE_ENV,
    ENABLE_IMAGE_OPS_ENV,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceRequest};
use apeireth_tools_canonical::IMAGE_OPS_CAPABILITY_ID;
use serde_json::json;

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

fn args() -> serde_json::Value {
    json!({
        "input": "in.png",
        "output": "out.png",
        "operations": [{ "op": "grayscale" }]
    })
}

async fn verdict(
    governance: &apeireth_governance::GovernancePipeline,
) -> apeireth_governance::GovernanceVerdict {
    let capability = CapabilityId::new(IMAGE_OPS_CAPABILITY_ID).expect("capability id");
    let arguments = args();
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    governance.evaluate_verbose(&request).await
}

/// 开关 fail-closed: 只认显式 "1", 且**子依赖写总闸** —— 单独设本旋钮而
/// `APEIRETH_ENABLE_FILE_WRITE` 未开 = 无效果 (输出写归档文件写入族)。
#[test]
fn image_ops_knob_fails_closed_and_depends_on_the_write_gate() {
    let _lock = lock_env();
    let guard = EnvGuard::guard(&[ENABLE_IMAGE_OPS_ENV, ENABLE_FILE_WRITE_ENV]);

    guard.clear();
    assert!(!image_ops_enabled_from_env(), "默认关 (fail-closed)");

    std::env::set_var(ENABLE_IMAGE_OPS_ENV, "1");
    assert!(
        !image_ops_enabled_from_env(),
        "写总闸关时单独设本旋钮 = 无效果"
    );

    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");
    assert!(image_ops_enabled_from_env(), "旋钮 + 写总闸齐开才生效");

    std::env::set_var(ENABLE_IMAGE_OPS_ENV, " 1 ");
    assert!(image_ops_enabled_from_env(), "前后空白容忍 (语义真源同款)");

    for value in ["0", "true", "on", "11"] {
        std::env::set_var(ENABLE_IMAGE_OPS_ENV, value);
        assert!(!image_ops_enabled_from_env(), "must refuse {value:?}");
    }
}

/// 授权面同源: 开关开 = grant + require_approval —— 每次图像调用都停人工
/// 审批 (本地审批面板 / IM 审批卡同链, 无自动放行档); 开关关 = 不进授权面。
#[tokio::test]
async fn image_ops_grants_with_per_call_human_approval_when_wired() {
    let _lock = lock_env();
    let guard = EnvGuard::guard(&[ENABLE_IMAGE_OPS_ENV, ENABLE_FILE_WRITE_ENV]);

    // 双闸齐开: 授权面放行到人工审批 (不是放行执行, 也不是拒绝)。
    guard.clear();
    std::env::set_var(ENABLE_IMAGE_OPS_ENV, "1");
    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");
    let governance = build_production_governance_from_env();
    let result = verdict(&governance).await;
    assert!(
        matches!(result.decision, Decision::RequireApproval { .. }),
        "每次图像写入都停人工审批: {}",
        result.decision
    );
    assert_eq!(
        result.hook, "permission_governance",
        "审批判定来自授权层标记"
    );

    // 未开旋钮 (fail-closed): 不进授权面, 拒绝即帧。
    guard.clear();
    let governance = build_production_governance_from_env();
    let result = verdict(&governance).await;
    assert!(
        matches!(result.decision, Decision::Deny { .. }),
        "默认关不得授权: {}",
        result.decision
    );
}
