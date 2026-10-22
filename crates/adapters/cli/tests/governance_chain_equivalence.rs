//! R8-B2: 链入口与旧入口行为等价测试 (四态: Allow / Deny / RequireApproval / NotConfigured)。
//!
//! 「旧入口」= gate 直接交给 runtime (`with_governance(gate)`), 即 2026-10-21
//! 装配根切换前的形状; 「链入口」= `ChainGovernanceHook(GovernanceChain(gate))`,
//! 即切换后 `build_canonical_runtime_from_env` 的形状 (preset 包装层仍是
//! Gate 段内的 gate, 本测在裁决种类层验证 chain 适配器不改变任何判定语义)。
//!
//! 四态逐一对照: 默认链配置 (`require_human_authority = false`) 下 Gate 判定
//! 原样透传为同种类 `Decision`; 未配置态两入口都 fail-closed 拒绝。
//!
//! 已知元数据差异 (R8-B1 适配器交付映射, 非行为差异): `verdict.hook` 统一标
//! `governance_chain`, 拒绝理由加 `gate: ` 前缀。裁决种类与理由内容不变。

use std::sync::Arc;

use apeireth_cli::build_production_governance;
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, ChainGovernanceHook, Decision, DenyUnconfigured, GovernanceChain, GovernanceHook,
    GovernanceRequest,
};
use serde_json::Value;

/// 切换后的装配形状: chain (gate = 现有治理链) 包成 ChainGovernanceHook。
fn chain_entry(gate: Arc<dyn GovernanceHook>) -> Arc<dyn GovernanceHook> {
    Arc::new(ChainGovernanceHook::new(Arc::new(GovernanceChain::new(
        gate,
    ))))
}

/// 同一裸派发请求分别走旧入口与链入口, 回两份裁决。
async fn both_verdicts(
    old: &Arc<dyn GovernanceHook>,
    new: &Arc<dyn GovernanceHook>,
    capability: &str,
) -> (Decision, Decision) {
    let capability = CapabilityId::new(capability).expect("valid test capability");
    let arguments = Value::Null;
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    (
        old.evaluate_verbose(&request).await.decision,
        new.evaluate_verbose(&request).await.decision,
    )
}

fn kind(decision: &Decision) -> &'static str {
    match decision {
        Decision::Allow => "allow",
        Decision::Deny { .. } => "deny",
        Decision::RequireApproval { .. } => "require_approval",
    }
}

/// 四态之 Allow: 生产策略放行 `tool.repo`, 两入口裁决种类一致。
#[tokio::test]
async fn chain_entry_equivalent_for_allow() {
    let gate: Arc<dyn GovernanceHook> = Arc::new(build_production_governance(false));
    let old = Arc::clone(&gate);
    let new = chain_entry(gate);
    let (old_decision, new_decision) = both_verdicts(&old, &new, "tool.repo").await;
    assert_eq!(kind(&old_decision), "allow", "{old_decision}");
    assert_eq!(kind(&new_decision), kind(&old_decision), "{new_decision}");
}

/// 四态之 Deny: 生产策略拒绝 `tool.shell`, 两入口同为拒绝, 且链适配器只加
/// `gate: ` 前缀、原拒绝理由必须完整保留可追溯。
#[tokio::test]
async fn chain_entry_equivalent_for_deny() {
    let gate: Arc<dyn GovernanceHook> = Arc::new(build_production_governance(false));
    let old = Arc::clone(&gate);
    let new = chain_entry(gate);
    let (old_decision, new_decision) = both_verdicts(&old, &new, "tool.shell").await;
    assert_eq!(kind(&old_decision), "deny", "{old_decision}");
    assert_eq!(kind(&new_decision), "deny", "{new_decision}");
    let (
        Decision::Deny {
            reason: old_reason, ..
        },
        Decision::Deny {
            reason: new_reason, ..
        },
    ) = (&old_decision, &new_decision)
    else {
        unreachable!("kinds already asserted deny")
    };
    assert!(
        new_reason.contains(old_reason.as_str()),
        "chain reason must preserve the gate reason: {new_reason:?} vs {old_reason:?}"
    );
}

/// 四态之 RequireApproval: 本地读 opt-in 后 `tool.filesystem` descriptor =
/// [Read, Write], 无 TurnSecurityContext 的裸派发 fail-closed 升 RequireApproval
/// (H3 语义, 同 tests/production_governance.rs 口径), 两入口一致。
#[tokio::test]
async fn chain_entry_equivalent_for_require_approval() {
    let gate: Arc<dyn GovernanceHook> = Arc::new(build_production_governance(true));
    let old = Arc::clone(&gate);
    let new = chain_entry(gate);
    let (old_decision, new_decision) = both_verdicts(&old, &new, "tool.filesystem").await;
    assert_eq!(kind(&old_decision), "require_approval", "{old_decision}");
    assert_eq!(kind(&new_decision), "require_approval", "{new_decision}");
}

/// 四态之 NotConfigured: 未配置时旧入口 = runtime 默认 `DenyUnconfigured`,
/// 新入口 = chain 无 gate (`NotConfigured` 显式化); 两入口都必须 fail-closed
/// 拒绝, 裁决种类等价。
#[tokio::test]
async fn chain_entry_equivalent_for_not_configured() {
    let old: Arc<dyn GovernanceHook> = Arc::new(DenyUnconfigured);
    let new: Arc<dyn GovernanceHook> = Arc::new(ChainGovernanceHook::new(Arc::new(
        GovernanceChain::without_gate(),
    )));
    let (old_decision, new_decision) = both_verdicts(&old, &new, "tool.repo").await;
    assert_eq!(kind(&old_decision), "deny", "{old_decision}");
    assert_eq!(kind(&new_decision), "deny", "{new_decision}");
}
