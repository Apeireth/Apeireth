//! 受控文件写入旋钮 (`APEIRETH_ENABLE_FILE_WRITE` / 子档) 的治理接线回归
//! (件一风险映射 + 件二设置开关的 Rust 侧即效接线)。
//!
//! 覆盖: 开关 → 风险档位解析 (子档依赖主开关, fail-closed) / 默认档每次写入
//! 都停人工审批 (与本地审批面板 / IM 审批卡同链) / 自动放行档只放行修改类
//! 补丁 (创建/删除停人工审批, 「删除/新建永不自动放行」) / 未开主开关 =
//! 写入工具未授权。

use std::sync::Mutex;

use apeireth_cli::{
    build_production_governance_from_env, file_write_risk_level_from_env, FileWriteRiskLevel,
    ENABLE_FILE_WRITE_AUTO_PASS_ENV, ENABLE_FILE_WRITE_ENV,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, Decision, GovernancePipeline, GovernanceRequest, TurnSecurityContext,
};
use apeireth_tools_canonical::{APPLY_PATCH_CAPABILITY_ID, GIT_WRITE_BOUNDARY_NOTE};
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

async fn verdict(
    governance: &GovernancePipeline,
    arguments: Value,
    context: Option<&TurnSecurityContext>,
) -> apeireth_governance::GovernanceVerdict {
    let capability = CapabilityId::new(APPLY_PATCH_CAPABILITY_ID).expect("capability id");
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

fn update_args() -> Value {
    json!({ "patch": "*** Begin Patch\n*** Update File: a.txt\n@@\n-old\n+NEW\n*** End Patch" })
}

fn add_args() -> Value {
    json!({ "patch": "*** Begin Patch\n*** Add File: created.txt\n+hello\n*** End Patch" })
}

fn delete_args() -> Value {
    json!({ "patch": "*** Begin Patch\n*** Delete File: a.txt\n*** End Patch" })
}

/// 设置开关即效接线 (Rust 侧): 开关组合 → 风险档位, 子档依赖主开关。
#[test]
fn file_write_env_knobs_map_to_the_governance_risk_levels() {
    let _lock = lock_env();
    let guard = EnvGuard::guard(&[ENABLE_FILE_WRITE_ENV, ENABLE_FILE_WRITE_AUTO_PASS_ENV]);

    guard.clear();
    assert_eq!(
        file_write_risk_level_from_env(),
        FileWriteRiskLevel::Disabled,
        "默认关 (fail-closed)"
    );

    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");
    assert_eq!(
        file_write_risk_level_from_env(),
        FileWriteRiskLevel::RequireApprovalEveryWrite,
        "主开关 = 默认审批级 (每次写入都要人批)"
    );

    std::env::set_var(ENABLE_FILE_WRITE_AUTO_PASS_ENV, "1");
    assert_eq!(
        file_write_risk_level_from_env(),
        FileWriteRiskLevel::AutoPassReadFileEdits,
        "子档随主开关生效"
    );

    // 子档依赖主开关: 单独设子档 = 无效果 (同 shellSandbox 嵌套依赖模式)。
    std::env::remove_var(ENABLE_FILE_WRITE_ENV);
    assert_eq!(
        file_write_risk_level_from_env(),
        FileWriteRiskLevel::Disabled,
        "子档不得越过主开关"
    );
}

/// 默认审批级: 每次写入都停在 [`Decision::RequireApproval`] (授权标记层,
/// 与本地审批面板 / IM 审批卡同链), 且授权判定先于内容风险。
#[tokio::test]
async fn default_level_routes_every_write_to_human_approval() {
    let _lock = lock_env();
    let guard = EnvGuard::guard(&[ENABLE_FILE_WRITE_ENV, ENABLE_FILE_WRITE_AUTO_PASS_ENV]);
    guard.clear();
    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");

    let governance = build_production_governance_from_env();
    for arguments in [update_args(), add_args(), delete_args()] {
        let result = verdict(&governance, arguments, None).await;
        assert!(
            matches!(result.decision, Decision::RequireApproval { .. }),
            "{}",
            result.decision
        );
        assert_eq!(
            result.hook, "permission_governance",
            "默认档的审批判定来自授权层标记"
        );
    }
}

/// 自动放行档: 修改类补丁放行 (写入风险钩子不拦), 创建/删除停人工审批
/// (同一条审批链, 「删除/新建永不自动放行」单调生效)。
#[tokio::test]
async fn auto_pass_level_keeps_creates_and_deletes_on_the_approval_chain() {
    let _lock = lock_env();
    let guard = EnvGuard::guard(&[ENABLE_FILE_WRITE_ENV, ENABLE_FILE_WRITE_AUTO_PASS_ENV]);
    guard.clear();
    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");
    std::env::set_var(ENABLE_FILE_WRITE_AUTO_PASS_ENV, "1");

    let governance = build_production_governance_from_env();
    for arguments in [add_args(), delete_args()] {
        let result = verdict(&governance, arguments, None).await;
        assert!(
            matches!(result.decision, Decision::RequireApproval { .. }),
            "{}",
            result.decision
        );
        assert_eq!(
            result.hook, "apply_patch_write_risk",
            "创建/删除必须停在写入风险钩子的人工审批档"
        );
    }

    // 修改类补丁越过写入风险钩子 (自动放行); 携带写意图的回合上整体放行。
    let context = write_intent_context();
    let result = verdict(&governance, update_args(), Some(&context)).await;
    assert_ne!(
        result.hook, "apply_patch_write_risk",
        "修改类补丁不得被写入风险钩子拦下"
    );
    assert!(
        result.is_allowed(),
        "写意图下的修改类补丁应放行: {} / {:?}",
        result.hook,
        result.decision
    );
}

/// 未开主开关: 写入工具不在授权面 (fail-closed, 拒绝即帧)。
#[tokio::test]
async fn disabled_level_leaves_the_write_tool_unauthorized() {
    let _lock = lock_env();
    let guard = EnvGuard::guard(&[ENABLE_FILE_WRITE_ENV, ENABLE_FILE_WRITE_AUTO_PASS_ENV]);
    guard.clear();

    let governance = build_production_governance_from_env();
    let result = verdict(&governance, update_args(), None).await;
    assert!(
        matches!(result.decision, Decision::Deny { .. }),
        "{}",
        result.decision
    );
    assert_eq!(result.hook, "permission_governance");
}

/// git 写边界声明同样落在文档面 (件三: 工具描述与 docs 同文)。
#[test]
fn docs_state_the_git_write_boundary() {
    let user_manual = include_str!("../../../../docs/02-guides/user-manual.md");
    assert!(
        user_manual.contains("git 提交等写操作**不提供工具**"),
        "user manual must state the git write boundary"
    );
    let api = include_str!("../../../../docs/03-reference/api.md");
    assert!(
        api.contains("git 提交等写操作不提供工具，属设计边界"),
        "api reference must state the git write boundary"
    );
    // 与工具描述的边界文案同源 (同一事实, 两种语言面)。
    assert!(GIT_WRITE_BOUNDARY_NOTE.contains("none exists"));
}

/// 写意图上下文 (修改类请求) —— 自动放行档测试用。
fn write_intent_context() -> TurnSecurityContext {
    use apeireth_guard::{IntentInput, IntentInterpreter, RuleIntentInterpreter};
    let intent = RuleIntentInterpreter.interpret(IntentInput {
        session_id: SessionId::new().to_string(),
        trace_id: String::new(),
        user_request: "修改 a.txt 中的这一行实现".to_string(),
        created_at_ms: 0,
    });
    TurnSecurityContext::new(intent.intent_id.clone(), "").with_intent(intent)
}
