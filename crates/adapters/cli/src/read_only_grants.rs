//! 默认可用的**只读**工具授予（自省 / 代码概览 / 性格周报）。
//!
//! # 为什么这些工具默认可用、零审批
//!
//! 它们的共同点是：**只读、不外联、不执行**——
//!
//! - `tool.repo`：仓库只读概览；
//! - `tool.self_status`：自述面（模型可实测自身状态再发言，不靠翻文件撞见自己）；
//! - `tool.trait_report`：性格周报（只读成长计既有账的数字与调整原因）；
//! - `tool.repo_map`：代码地图（读工作区内的源文件，抽符号签名成大纲）。
//!
//! 「默认可用」不等于「无限制」：每个工具自己还有限域（工作区根）、
//! 预算（文件数 / 字节 / token）与只读约束 —— 授予层只回答
//! 「这件事本身要不要人批」，而**只读不批**是本仓既有口径。
//!
//! # 为什么单独一个模块
//!
//! 尺寸棘轮（`cli/src/lib.rs` 在基线边缘，只减不增）：
//! 新增一个只读工具时不该把装配根又顶上去一行。授予清单与理由放在这里，
//! 装配根只留一次调用。

use apeireth_governance::{Permission, PermissionPolicy};

/// 把默认可用的只读工具授予写进策略。
///
/// 返回同一个 `policy`（链式便于在装配根一行调用）；
/// `enable_local_read_tools` 管的是**本地文件系统与搜索**这两把更宽的钥匙，
/// 单独一档（legacy opt-in），不与上面的自省/概览工具混在一起。
pub fn grant_read_only_tools(
    mut policy: PermissionPolicy,
    enable_local_read_tools: bool,
) -> PermissionPolicy {
    policy.grant(Permission::ExecuteTool("tool.repo".to_string()));
    policy.grant(Permission::ExecuteTool("tool.self_status".to_string()));
    policy.grant(Permission::ExecuteTool("tool.trait_report".to_string()));
    // 代码地图：只读 + 工作区限域 + 三重预算封顶（见 `repo_map_tool` 模块）。
    policy.grant(Permission::ExecuteTool(
        apeireth_tools_canonical::repo_map_tool::REPO_MAP_CAPABILITY_ID.to_string(),
    ));
    if enable_local_read_tools {
        policy.grant(Permission::ExecuteTool("tool.filesystem".to_string()));
        policy.grant(Permission::ExecuteTool("tool.search".to_string()));
    }
    policy
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_governance::Permission;

    fn granted(policy: &PermissionPolicy, capability: &str) -> bool {
        policy.has(&Permission::ExecuteTool(capability.to_string()))
    }

    #[test]
    fn read_only_tools_are_granted_by_default() {
        let policy = grant_read_only_tools(PermissionPolicy::new(), false);
        for capability in [
            "tool.repo",
            "tool.self_status",
            "tool.trait_report",
            "tool.repo_map",
        ] {
            assert!(granted(&policy, capability), "{capability} 应默认可用");
        }
    }

    #[test]
    fn local_read_tools_stay_behind_the_legacy_opt_in() {
        let closed = grant_read_only_tools(PermissionPolicy::new(), false);
        assert!(
            !granted(&closed, "tool.filesystem"),
            "legacy 开关关时不授予"
        );
        assert!(!granted(&closed, "tool.search"), "legacy 开关关时不授予");

        let open = grant_read_only_tools(PermissionPolicy::new(), true);
        assert!(granted(&open, "tool.filesystem"));
        assert!(granted(&open, "tool.search"));
    }
}
