//! 书→技能沉淀 (I1) 旋钮的治理接线回归: `APEIRETH_ENABLE_SKILL_DISTILL`
//! (file_write 总闸族, fail-closed) / `APEIRETH_ENABLE_SKILL_SEARCH` (只读档)。
//!
//! 覆盖: 旋钮链 (沉淀子依赖写总闸, 单独设 = 无效果) / 治理档 (每次沉淀都
//! 停人工审批, 理由载荷 = 技能卡摘要; 检索只读零审批) / 文档随行 (治理档
//! 语义在用户手册与 API 契约面同文)。

use std::sync::Mutex;

use apeireth_cli::{
    build_production_governance_from_env, skill_distill_enabled_from_env,
    skill_search_enabled_from_env, ENABLE_FILE_WRITE_ENV, ENABLE_SKILL_DISTILL_ENV,
    ENABLE_SKILL_SEARCH_ENV,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernancePipeline, GovernanceRequest};
use apeireth_tools_canonical::{SKILL_DISTILL_CAPABILITY_ID, SKILL_SEARCH_CAPABILITY_ID};
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

fn skill_env_guard() -> EnvGuard {
    EnvGuard::guard(&[
        ENABLE_FILE_WRITE_ENV,
        ENABLE_SKILL_DISTILL_ENV,
        ENABLE_SKILL_SEARCH_ENV,
    ])
}

async fn verdict(
    governance: &GovernancePipeline,
    capability_id: &str,
    arguments: Value,
) -> apeireth_governance::GovernanceVerdict {
    let capability = CapabilityId::new(capability_id).expect("capability id");
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

fn distill_args() -> Value {
    json!({
        "card": {
            "name": "换元法",
            "summary": "换元后微分元同步替换",
            "triggers": ["换元"],
            "body": "步骤一二三",
            "source": { "kind": "book", "title": "讲义" }
        },
        "text": "来源正文"
    })
}

/// 旋钮链 (fail-closed): 沉淀子依赖写总闸 —— 单独设本旋钮而写总闸关 = 无
/// 效果; 检索只读档独立。
#[test]
fn skill_knobs_chain_behind_the_file_write_master_gate() {
    let _lock = lock_env();
    let guard = skill_env_guard();

    guard.clear();
    assert!(!skill_distill_enabled_from_env(), "默认关 (fail-closed)");
    assert!(!skill_search_enabled_from_env(), "默认关 (fail-closed)");

    // 单独设沉淀旋钮而写总闸关 = 无效果 (file_write 总闸族)。
    std::env::set_var(ENABLE_SKILL_DISTILL_ENV, "1");
    assert!(!skill_distill_enabled_from_env(), "沉淀旋钮不得越过写总闸");

    // 写总闸同开才生效。
    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");
    assert!(skill_distill_enabled_from_env(), "写总闸同开才生效");

    // 检索只读档: 不依赖写总闸。
    guard.clear();
    std::env::set_var(ENABLE_SKILL_SEARCH_ENV, "1");
    assert!(skill_search_enabled_from_env(), "只读检索独立于写总闸");
}

/// 治理档: 沉淀每次人批 (审批档钩子承载, 理由载荷 = 技能卡摘要); 检索只读
/// 零审批 (grant 即全权); 未开旋钮 = 不在授权面 (拒绝即帧)。
#[tokio::test]
async fn governance_tiers_match_the_two_skill_tools() {
    let _lock = lock_env();
    let guard = skill_env_guard();
    guard.clear();
    std::env::set_var(ENABLE_FILE_WRITE_ENV, "1");
    std::env::set_var(ENABLE_SKILL_DISTILL_ENV, "1");
    std::env::set_var(ENABLE_SKILL_SEARCH_ENV, "1");

    let governance = build_production_governance_from_env();

    // 沉淀: 每次都停人工审批, 载荷 = 技能卡摘要 (审批档钩子唯一承载)。
    let result = verdict(&governance, SKILL_DISTILL_CAPABILITY_ID, distill_args()).await;
    let Decision::RequireApproval { reason } = result.decision else {
        panic!("每次沉淀都必须停人工审批: {:?}", result.decision);
    };
    assert_eq!(result.hook, "skill_distill_write_risk");
    assert!(reason.contains("技能卡沉淀"), "{reason}");

    // 检索: 只读档零审批 (授权层放行, 无 ask 档)。
    let result = verdict(
        &governance,
        SKILL_SEARCH_CAPABILITY_ID,
        json!({ "query": "换元" }),
    )
    .await;
    assert!(
        result.is_allowed(),
        "只读检索零审批: {} / {:?}",
        result.hook,
        result.decision
    );

    // 旋钮关: 两级都不在授权面 (拒绝即帧)。
    guard.clear();
    let governance = build_production_governance_from_env();
    for capability in [SKILL_DISTILL_CAPABILITY_ID, SKILL_SEARCH_CAPABILITY_ID] {
        let result = verdict(&governance, capability, distill_args()).await;
        assert!(
            matches!(result.decision, Decision::Deny { .. }),
            "{capability} 未授权必须拒绝: {:?}",
            result.decision
        );
        assert_eq!(result.hook, "permission_governance");
    }
}

/// 文档随行: 治理档语义 (沉淀 = 写文件面 file_write 总闸族 + 审批档; 检索 =
/// 只读档) 在用户手册与 API 契约面同文。
#[test]
fn docs_state_the_skill_distill_and_search_governance_tiers() {
    let user_manual = include_str!("../../../../docs/02-guides/user-manual.md");
    assert!(
        user_manual.contains("tool.skill_distill"),
        "user manual must carry the skill distill tool"
    );
    assert!(
        user_manual.contains("tool.skill_search"),
        "user manual must carry the skill search tool"
    );
    assert!(
        user_manual.contains("每次沉淀都停人工审批"),
        "user manual must state the distill approval tier"
    );
    let api = include_str!("../../../../docs/03-reference/api.md");
    assert!(
        api.contains("`tool.skill_distill`") && api.contains("`tool.skill_search`"),
        "api reference must carry the two skill tool rows"
    );
}
