//! `apeireth-companion::gh_accel` — GitHub 加速插件 (github-accel).
//!
//! 生态位: 插件 = 最小贡献单元 (工具注册 + 授权 + 生命周期);
//! 本体: `apeireth-tools::github_accel` (节点池拉取 + 本机实测 + 选最快, 见其文档).
//!
//! 装配 (docs/ref-gh-accel.md):
//! - on_load: 注册 `gh_accel` 工具 (Async, Network, Value) + 授权日常包
//! - on_unload: 注销工具 + 撤销授权 (真清理, 幂等)
//!
//! 0 假装: 只探测+给命令, 不执行不改环境; 节点为第三方免费服务, 结果=本次实测.

use std::sync::Arc;

use crate::plugin::Plugin;
use crate::tool_bridge::ToolBridge;

/// GitHub 加速插件: 节点池 → 实测 → 最快节点 → 加速 URL/命令.
pub struct GhAccelPlugin;

impl Plugin for GhAccelPlugin {
    fn id(&self) -> &str {
        "github-accel"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "GitHub 加速: xiake.pro 聚合节点池, 每次本机实测延迟选最快 (返回加速 URL 与 git/curl 命令)"
    }
    fn on_load(&self, bridge: &ToolBridge) -> Result<(), String> {
        bridge.registry.register(
            "gh_accel".to_string(),
            Arc::new(apeireth_tools::GhAccelTool),
        );
        bridge.packs.grant(crate::packs::PermissionPack::permanent(
            "GitHub加速授权",
            vec!["gh_accel".to_string()],
        ));
        Ok(())
    }
    fn on_unload(&self, bridge: &ToolBridge) -> Result<(), String> {
        bridge.registry.unregister("gh_accel");
        bridge.packs.revoke_by_name("GitHub加速授权");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::SqliteMemoryStore;

    #[tokio::test]
    async fn plugin_registers_tool_and_clean_unload() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let reg = crate::plugin::PluginRegistry::new();
        reg.install(&bridge, Arc::new(GhAccelPlugin)).unwrap();
        assert!(reg.is_installed("github-accel"));
        assert!(bridge.registry.list().iter().any(|n| n == "gh_accel"));
        // 授权包覆盖 → 免现场审批
        assert!(bridge
            .packs
            .check_and_consume("gh_accel", chrono::Utc::now().timestamp_millis()));
        // 工具本身 Low 风险 (不含 exec/file 等关键词) — 宪法硬门不拦
        use crate::constitution_gate::ConstitutionGate;
        assert!(ConstitutionGate::check("调用工具 gh_accel 探测 GitHub 加速节点").is_none());
        // 卸载 → 真清理
        reg.uninstall(&bridge, "github-accel").unwrap();
        assert!(!bridge.registry.list().iter().any(|n| n == "gh_accel"));
        assert!(!bridge
            .packs
            .check_and_consume("gh_accel", chrono::Utc::now().timestamp_millis()));
    }
}
