//! W3 三洋葱 L3-L5 **物理执行面接线**: 双洋葱判定 (`DoubleOnionGate`) 的治理钩子化。
//!
//! **架构位置** (2026-10-10): 生产治理管线 = `PermissionGovernanceHook` (授权) →
//! `CredentialDisclosureHook` → `PromptInjectionHook` → `BehaviorChainGuardHook`
//! (cli `build_production_governance_parts_with_dataset`)。本 hook 作为**末层纵深防御**
//! 追加 (旋钮 `APEIRETH_ENABLE_ONION_LAYER=1`, 默认关): 授权已放行的动作再过一次
//! 洋葱权威判定 —— 与生产注释 "Authorization is deliberately the first hook" 不冲突
//! (本层只**收紧**已放行的, 绝不把未授权变成待审批)。
//!
//! **判定映射** (`OnionVerdict` → `Decision`):
//! - `Allow` → [`Decision::Allow`] (不越权: 放行后仍由链上其余 hook / 工具冻结审批面决定);
//! - `BlockByPrinciple` (L5 → E 层兜底) → [`Decision::deny`] (最终拒, reason 直达模型);
//! - `BlockByHumanAuthority` (HA 离线 + 触 HA 层 = 物理隔离) → [`Decision::deny`]
//!   (v1 "物理隔离拒绝" 语义: 主人不在 = 没有可挂起的人, 故 Deny 而非 RequireApproval;
//!   日常审批面 = 工具冻结 (`ShellFrozenInvocation`) + 会话预设, 不归本层)。
//!
//! **防重造轮子**: 判定逻辑 0 重复 (全在 `apeireth_core::onion_gate::DoubleOnionGate`);
//! 命令级高危词表仍由 `tools::guardrail` 守门 (本层只做权限层映射, 不扫命令内容)。

use apeireth_core::kernel::CapabilityId;
use apeireth_core::onion_gate::{DoubleOnionGate, OnionAction, OnionVerdict, PermissionLayerKind};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use async_trait::async_trait;

/// capability → 权限洋葱层 (生产 composition 层分类; 执行核心不认识具体能力)。
///
/// 分层依据 (与 v1 `PermissionLayer` 语义对齐):
/// - **L1 受控写**: `tool.filesystem` (受 W1 沙箱工作区边界约束的读写);
/// - **L2 重要操作**: `tool.fetch` (受 `ControlledEgress` 出口管制) / 未知能力缺省;
/// - **L3 关键操作**: `tool.shell` / `tool.process` / `tool.supervisor` /
///   `tool.std_sub_supervisor` / MCP 动态能力 (W1 AppContainer 沙箱 + guardrail +
///   审批面多重把守的执行面)。
pub fn onion_layer_for_capability(capability: &CapabilityId) -> PermissionLayerKind {
    let id = capability.as_str();
    if id.starts_with("tool.mcp") {
        return PermissionLayerKind::L3;
    }
    match id {
        "tool.filesystem" => PermissionLayerKind::L1,
        "tool.fetch" => PermissionLayerKind::L2,
        "tool.shell" | "tool.process" | "tool.supervisor" | "tool.std_sub_supervisor" => {
            PermissionLayerKind::L3
        }
        _ => PermissionLayerKind::L2,
    }
}

/// 双洋葱权威判定治理 hook (末层纵深防御, 默认关注册)。
pub struct OnionLayerHook {
    gate: DoubleOnionGate,
}

impl OnionLayerHook {
    /// 用给定门的装配构造 hook。
    pub fn new(gate: DoubleOnionGate) -> Self {
        Self { gate }
    }

    /// 内部: 对一次 capability dispatch 做洋葱判定。
    fn judge(&self, capability: &CapabilityId) -> Decision {
        let layer = onion_layer_for_capability(capability);
        let action = OnionAction::new(capability.as_str(), "capability dispatch").touches(layer);
        match self.gate.unify_check(&action) {
            OnionVerdict::Allow { .. } => Decision::Allow,
            OnionVerdict::BlockByPrinciple { layer, reason } => {
                Decision::deny(format!("[双洋葱·原则层 {layer:?}] {reason}"))
            }
            OnionVerdict::BlockByHumanAuthority { reason } => {
                Decision::deny(format!("[双洋葱·HA 物理隔离] {reason}"))
            }
        }
    }
}

#[async_trait]
impl GovernanceHook for OnionLayerHook {
    fn name(&self) -> &str {
        "onion_layer"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch { capability, .. } = &request.action else {
            // completion 不触权限层, 本层不越权。
            return Decision::Allow;
        };
        self.judge(capability)
    }
}

#[cfg(test)]
mod onion_layer_tests {
    use super::*;
    use apeireth_core::onion::{HAMode, HumanAuthority};
    use apeireth_core::onion_gate::standard_double_onion_gate;

    fn capability(id: &str) -> CapabilityId {
        CapabilityId::new(id).expect("static capability id")
    }

    fn request_with<'a>(
        capability: &'a CapabilityId,
        session: &'a apeireth_core::kernel::SessionId,
        trace: &'a apeireth_core::kernel::TraceId,
        arguments: &'a serde_json::Value,
    ) -> GovernanceRequest<'a> {
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability,
                arguments,
            },
            *session,
            *trace,
            1,
        )
    }

    fn offline_gate() -> apeireth_core::onion_gate::DoubleOnionGate {
        use apeireth_core::onion_gate::single_human_authority;
        standard_double_onion_gate().with_human_authority(HumanAuthority {
            mode: HAMode::Offline,
            ..single_human_authority()
        })
    }

    #[tokio::test]
    async fn allowed_layer_under_online_ha_passes_through() {
        // W3 五件门③: 效果可见 = 已授权动作在 HA 在线时过洋葱层不被误伤。
        let hook = OnionLayerHook::new(standard_double_onion_gate());
        let capability = capability("tool.shell");
        let session = apeireth_core::kernel::SessionId::new();
        let trace = apeireth_core::kernel::TraceId::new();
        let args = serde_json::json!({});
        let decision = hook
            .evaluate(&request_with(&capability, &session, &trace, &args))
            .await;
        assert_eq!(decision, Decision::Allow);
    }

    #[tokio::test]
    async fn ha_offline_blocks_ha_layer_with_physical_reason() {
        // HA 离线 + L3 (触 HA 层) = 物理隔离拒绝 (v1 语义: Deny 而非挂起)。
        let hook = OnionLayerHook::new(offline_gate());
        let capability = capability("tool.shell");
        let session = apeireth_core::kernel::SessionId::new();
        let trace = apeireth_core::kernel::TraceId::new();
        let args = serde_json::json!({});
        let decision = hook
            .evaluate(&request_with(&capability, &session, &trace, &args))
            .await;
        match decision {
            Decision::Deny { reason } => assert!(reason.contains("HA 离线"), "{reason}"),
            other => panic!("expected Deny, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ha_offline_still_allows_non_ha_layer() {
        // HA 离线只挡需 HA 的层 (L1 不触 HA → 仍放行)。
        let hook = OnionLayerHook::new(offline_gate());
        let capability = capability("tool.filesystem");
        let session = apeireth_core::kernel::SessionId::new();
        let trace = apeireth_core::kernel::TraceId::new();
        let args = serde_json::json!({});
        let decision = hook
            .evaluate(&request_with(&capability, &session, &trace, &args))
            .await;
        assert_eq!(decision, Decision::Allow);
    }

    #[tokio::test]
    async fn completion_action_is_not_judged() {
        // completion 不触权限层 → Allow (不越权)。
        let hook = OnionLayerHook::new(standard_double_onion_gate());
        let session = apeireth_core::kernel::SessionId::new();
        let trace = apeireth_core::kernel::TraceId::new();
        let request = GovernanceRequest::new(
            Action::Completion {
                model: "m",
                message_count: 1,
            },
            session,
            trace,
            1,
        );
        assert_eq!(hook.evaluate(&request).await, Decision::Allow);
    }

    #[test]
    fn capability_layer_mapping_is_documented() {
        assert_eq!(
            onion_layer_for_capability(&capability("tool.filesystem")),
            PermissionLayerKind::L1
        );
        assert_eq!(
            onion_layer_for_capability(&capability("tool.fetch")),
            PermissionLayerKind::L2
        );
        assert_eq!(
            onion_layer_for_capability(&capability("tool.shell")),
            PermissionLayerKind::L3
        );
        assert_eq!(
            onion_layer_for_capability(&capability("tool.mcp.whatever")),
            PermissionLayerKind::L3
        );
        assert_eq!(
            onion_layer_for_capability(&capability("tool.unknown")),
            PermissionLayerKind::L2
        );
    }

    #[test]
    fn hook_name_is_stable() {
        assert_eq!(
            OnionLayerHook::new(standard_double_onion_gate()).name(),
            "onion_layer"
        );
    }
}
