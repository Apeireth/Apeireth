//! 授权读面投影: 面板「已授权」词表 + 族授权行。
//!
//! **读面不自算第二套口径**: 「是否已授权」直接问执法面
//! ([`PermissionPolicy::decision_for_capability`]), 故面板显示的授权状态与实际
//! 放行/拒绝**结构上同源** —— 执法面加了族授权 (段边界前缀) 时, 读数自动同口径,
//! 不必再改读面。
//!
//! 反面教材 (仓内既有教训): 模型侧调用名是 `ext.nmap` 这类短名, **能力 id 是
//! `tool.external.nmap`**; 治理授权按能力 id 判定, **按模型名授权等于没授**。
//! 读面若把「按模型名」也算 granted, 就是「显示已授权、实际被拒」的读数陷阱。
//!
//! 本模块是纯投影 (零写面): 只读治理底座的同一份 [`PermissionPolicy`]。

use apeireth_gateway::GrantDto;
use apeireth_governance::{Decision, Permission, PermissionPolicy};

/// MCP 外部工具**族根** (段边界前缀: `tool.mcp.<server>.<tool>` 命中,
/// `tool.mcpsomething` 不命中 —— 边界是点号)。
///
/// TODO(收口): 族根常量下沉到 `apeireth_tools_canonical::mcp_bridge` 后, 本处
/// 改为转发该常量 (共享单一真源), 字面量不留第二份。
const MCP_TOOL_FAMILY_ROOT: &str = "tool.mcp";

/// 面板「已授权」读数 —— **执法面同一个判定**, 读面不自算第二套口径。
///
/// 执法面 = [`PermissionPolicy::decision_for_capability`]: 精确能力 id 命中
/// (或族命中, 或 admin override) 放行; 标了逐次人批 ⇒ 停审批; 其余拒绝。
/// 读面词表:
/// * `granted` —— 执法面放行;
/// * `granted-approval` —— 已授权, 但每次调用停人工审批;
/// * `granted-by-model-name` —— 历史遗留的「按模型面工具名」授权: **执法面不认**
///   (执法只认能力 id), 故**不**记作 `granted`, 只留痕给人看;
/// * `none` —— 未授权。
pub(super) fn permission_state(
    policy: &PermissionPolicy,
    capability_id: &str,
    model_name: &str,
) -> &'static str {
    match policy.decision_for_capability(capability_id) {
        Decision::Allow => "granted",
        Decision::RequireApproval { .. } => "granted-approval",
        Decision::Deny { .. } => {
            if policy_grants_model_name(policy, model_name) {
                "granted-by-model-name"
            } else {
                "none"
            }
        }
    }
}

/// 是否存在「按模型面工具名」的授权 (`Permission::ExecuteTool(模型面名)`)。
///
/// 执法面**不认**这种授权 (执法只认能力 id), 读面只留痕, 不当作已授权。
fn policy_grants_model_name(policy: &PermissionPolicy, model_name: &str) -> bool {
    policy.iter().any(|permission| {
        matches!(permission, Permission::ExecuteTool(name) if name.as_str() == model_name)
    })
}

/// 族授权在授权面上的行 (族是独立集合, 不塞进 `Permission` 枚举 —— 枚举非
/// `#[non_exhaustive]`, 加变体会破下游穷尽匹配)。
///
/// `capability` 放族根 id (机器可用: 未来按族撤销按它寻址), `permission` 放
/// 人类可读标签 (面板行的标题面)。
pub(super) fn family_grants(policy: &PermissionPolicy) -> Vec<GrantDto> {
    granted_family_roots(policy)
        .into_iter()
        .map(|family| GrantDto {
            permission: family_grant_label(&family),
            capability: family,
            granted_at: None,
        })
        .collect()
}

/// 面板读面: 已授权的**能力族** (族根集合)。
///
/// 真源 = 治理底座 [`PermissionPolicy`] 的族集合 (与执法面同一份对象, 读面
/// **不建第二份真源**)。走底座的只读访问器 `granted_families()`。
fn granted_family_roots(policy: &PermissionPolicy) -> Vec<String> {
    policy.granted_families().map(str::to_string).collect()
}

/// 族授权在面板上的可读标签 (族根 + 覆盖范围)。
fn family_grant_label(family: &str) -> String {
    if family == MCP_TOOL_FAMILY_ROOT {
        format!("工具族 `{family}`（含该族全部 MCP 外部工具）")
    } else {
        format!("工具族 `{family}`（含该族全部能力 id）")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 读数 = 执法判定: 精确能力 id 命中 ⇒ `granted`; 「按模型面工具名」的授权
    /// 执法面不认 ⇒ 读数**不**记 `granted`, 只留痕 (`granted-by-model-name`)。
    #[test]
    fn readout_follows_the_enforcement_verdict() {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool("tool.repo".to_string()));
        assert_eq!(permission_state(&policy, "tool.repo", "repo"), "granted");
        assert_eq!(permission_state(&policy, "tool.shell", "shell"), "none");

        policy.grant(Permission::ExecuteTool("shell".to_string()));
        assert_eq!(
            permission_state(&policy, "tool.shell", "shell"),
            "granted-by-model-name"
        );
    }

    /// 逐次人批档: 已授权但每次调用停审批 —— 读数不塌成 plain `granted`。
    #[test]
    fn readout_marks_the_approval_gate() {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool("tool.apply_patch".to_string()));
        policy.require_approval_for("tool.apply_patch");
        assert_eq!(
            permission_state(&policy, "tool.apply_patch", "apply_patch"),
            "granted-approval"
        );
    }

    /// 族授权: 执法面按段边界放行 ⇒ 读数同口径 `granted`; 段边界外
    /// (`tool.mcpsomething`) 不命中; 族行在授权面可见。
    #[test]
    fn family_grant_is_granted_and_visible() {
        let mut policy = PermissionPolicy::new();
        policy.grant_family(MCP_TOOL_FAMILY_ROOT);
        assert_eq!(
            permission_state(&policy, "tool.mcp.demo.lookup", "mcp:demo:lookup"),
            "granted"
        );
        assert_eq!(
            permission_state(&policy, "tool.mcpsomething", "mcp:something"),
            "none"
        );

        let rows = family_grants(&policy);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].capability, "tool.mcp");
        assert!(rows[0].permission.contains("tool.mcp"));
    }

    /// 无族授权时授权面不多行 (读面不编数)。
    #[test]
    fn no_family_grant_means_no_family_row() {
        let policy = PermissionPolicy::new();
        assert!(family_grants(&policy).is_empty());
    }
}
