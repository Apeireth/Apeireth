//! git 写分级旋钮 (`APEIRETH_ENABLE_GIT_WRITE` / `_AUTO_PASS_L1` 子档) 的治理
//! 接线回归 (D-P1 分级风险映射)。
//!
//! 覆盖: 开关 → 分级档位解析 (子档依赖主开关, fail-closed) / 默认档 L1·L2
//! 都停人工审批 (L1 理由带分级标注, 与本地审批面板 / IM 审批卡同链) /
//! 「自动放行 L1」档只放 L1 可逆本地写 (L2 永远人批) / L3 毁灭拒绝即帧 /
//! 未开主开关 = 工具未授权。

use std::sync::Mutex;

use apeireth_cli::{
    build_production_governance_from_env, git_write_risk_level_from_env, GitWriteRiskLevel,
    ENABLE_GIT_WRITE_AUTO_PASS_L1_ENV, ENABLE_GIT_WRITE_ENV,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, GovernancePipeline, GovernanceRequest, TurnSecurityContext};
use apeireth_tools_canonical::GIT_WRITE_CAPABILITY_ID;
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

fn git_write_env_guard() -> EnvGuard {
    EnvGuard::guard(&[ENABLE_GIT_WRITE_ENV, ENABLE_GIT_WRITE_AUTO_PASS_L1_ENV])
}

async fn verdict(
    governance: &GovernancePipeline,
    command: &str,
    context: Option<&TurnSecurityContext>,
) -> apeireth_governance::GovernanceVerdict {
    let capability = CapabilityId::new(GIT_WRITE_CAPABILITY_ID).expect("capability id");
    let arguments: Value = json!({ "command": command });
    let mut request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    if let Some(context) = context {
        request = request.with_security_context(context);
    }
    governance.evaluate_verbose(&request).await
}

/// 写意图上下文 (运行 git 命令提交/切分支类请求) —— 自动放行档测试用。
///
/// 任务意图须覆盖「命令执行 + 修改」面: git 写命令是进程执行面 (与 shell 工具
/// 同一意图口径), 意图对齐门对未请求的执行面照常抬审批 —— 那是独立的纵深
/// 一层, 不归分级档位管。
fn write_intent_context() -> TurnSecurityContext {
    use apeireth_guard::{IntentInput, IntentInterpreter, RuleIntentInterpreter};
    let intent = RuleIntentInterpreter.interpret(IntentInput {
        session_id: SessionId::new().to_string(),
        trace_id: String::new(),
        user_request: "运行 git 命令把这些修改提交到本地仓库".to_string(),
        created_at_ms: 0,
    });
    TurnSecurityContext::new(intent.intent_id.clone(), "").with_intent(intent)
}

/// 设置开关即效接线: 开关组合 → 分级档位, 子档依赖主开关 (fail-closed)。
#[test]
fn git_write_env_knobs_map_to_the_governance_risk_levels() {
    let _lock = lock_env();
    let guard = git_write_env_guard();
    guard.clear();

    assert_eq!(
        git_write_risk_level_from_env(),
        GitWriteRiskLevel::Disabled,
        "默认关 (fail-closed)"
    );

    std::env::set_var(ENABLE_GIT_WRITE_ENV, "1");
    assert_eq!(
        git_write_risk_level_from_env(),
        GitWriteRiskLevel::RequireApprovalEveryWrite,
        "主开关开 = 默认档 (每次人批)"
    );

    std::env::set_var(ENABLE_GIT_WRITE_AUTO_PASS_L1_ENV, "1");
    assert_eq!(
        git_write_risk_level_from_env(),
        GitWriteRiskLevel::AutoPassReversibleL1,
        "子档开 = 自动放行 L1 档"
    );

    // 单独设子档 = 无效果 (依赖主开关)。
    std::env::remove_var(ENABLE_GIT_WRITE_ENV);
    assert_eq!(
        git_write_risk_level_from_env(),
        GitWriteRiskLevel::Disabled,
        "子档依赖主开关"
    );
}

/// 默认档: L1 / L2 都停人工审批, 理由带分级标注 (审批卡载荷同源)。
#[tokio::test]
async fn default_tier_keeps_every_git_write_at_human_approval() {
    let _lock = lock_env();
    let guard = git_write_env_guard();
    guard.clear();
    std::env::set_var(ENABLE_GIT_WRITE_ENV, "1");

    let governance = build_production_governance_from_env();
    for (command, label) in [
        ("git commit -m x", "L1"),
        ("git add a.txt", "L1"),
        ("git push origin main", "L2"),
        ("git fetch origin", "L2"),
    ] {
        let result = verdict(&governance, command, None).await;
        assert!(
            matches!(
                result.decision,
                apeireth_governance::Decision::RequireApproval { .. }
            ),
            "{command} 默认档必须停人工审批: {:?}",
            result.decision
        );
        assert_eq!(result.hook, "git_write_tier", "{command}");
        let reason = result.decision.reason().unwrap_or_default();
        assert!(
            reason.contains(label),
            "{command} 理由须带 {label} 分级标注: {reason}"
        );
    }
}

/// 「自动放行 L1」档: 只放 L1 可逆本地写; L2 永远人批; L3 拒绝即帧。
#[tokio::test]
async fn auto_pass_l1_tier_releases_only_reversible_local_writes() {
    let _lock = lock_env();
    let guard = git_write_env_guard();
    guard.clear();
    std::env::set_var(ENABLE_GIT_WRITE_ENV, "1");
    std::env::set_var(ENABLE_GIT_WRITE_AUTO_PASS_L1_ENV, "1");

    let governance = build_production_governance_from_env();
    let context = write_intent_context();

    // L1 放行 (写意图上下文内)。
    for command in ["git add a.txt", "git commit -m x", "git switch main"] {
        let result = verdict(&governance, command, Some(&context)).await;
        assert!(
            result.is_allowed(),
            "{command} 自动放行 L1 档应放行: {:?} / {}",
            result.decision,
            result.hook
        );
    }

    // L2 永远人批 (自动放行档也不放)。
    for command in [
        "git push origin main",
        "git remote add origin https://host/x.git",
    ] {
        let result = verdict(&governance, command, Some(&context)).await;
        assert!(
            matches!(
                result.decision,
                apeireth_governance::Decision::RequireApproval { .. }
            ),
            "{command} 永远人工审批: {:?}",
            result.decision
        );
    }

    // L3 毁灭: 拒绝即帧, 不进审批。
    for command in [
        "git reset --hard",
        "git push --force origin main",
        "git commit --amend",
    ] {
        let result = verdict(&governance, command, Some(&context)).await;
        assert!(
            matches!(result.decision, apeireth_governance::Decision::Deny { .. }),
            "{command} 必须拒绝即帧: {:?}",
            result.decision
        );
    }
}

/// 未开主开关: git 写工具不在授权面 (fail-closed, 拒绝即帧)。
#[tokio::test]
async fn disabled_level_leaves_the_git_write_tool_unauthorized() {
    let _lock = lock_env();
    let guard = git_write_env_guard();
    guard.clear();

    let governance = build_production_governance_from_env();
    let result = verdict(&governance, "git commit -m x", None).await;
    assert!(
        matches!(result.decision, apeireth_governance::Decision::Deny { .. }),
        "{}",
        result.decision
    );
    assert_eq!(result.hook, "permission_governance");
}
