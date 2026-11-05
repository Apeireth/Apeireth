//! 外部工具（协议桥）启动期接线面 —— CLI 组合根的薄封装。
//!
//! 接线四步（发现 → 撤销 → 登记 → 授权）的机制在
//! `apeireth_runtime_assembly::canonical::external_tool_bridge`；本模块只做
//! **组合根侧的两件事**：授权落点 + 失败不阻断启动。
//!
//! # 失败语义
//!
//! * **发现 fail-all**：任一服务器不可达 ⇒ 整次发现失败（不半注册）。
//! * **失败不阻断启动**：外部工具面不可用 ≠ 主机不可用 ⇒ 启动继续，但**如实
//!   记录**到 stderr，不静默吞掉（0 装诚实：主人要看得见「外部工具没接上」）。
//! * **授权先于登记**：族授权 + 族逐次人批落在生产权限策略里，让派发裁决是
//!   `RequireApproval` ⇒ 运行时挂起该次派发等人决定。

use std::sync::{Arc, Mutex};

use apeireth_governance::PermissionPolicy;
use apeireth_runtime::Runtime;
use apeireth_runtime_assembly::canonical::{
    authorize_external_tool_family, wire_external_tools, McpModule,
};
use apeireth_runtime_assembly::ProductionModules;
use apeireth_tools_canonical::mcp_bridge::McpToolBridge;

/// 装配期取出的外部工具桥句柄。
///
/// 桥与模块袋必须在**装配模块被 move 之前**取出（`register_into` 取走所有
/// 权）⇒ 组合根分两步：先取句柄，建成运行时后再接线。
pub struct ExternalToolHandles {
    bridge: Option<Arc<McpToolBridge>>,
    module_bag: Option<Arc<McpModule>>,
}

/// 从装配模块取出外部工具桥句柄（桥未装配时两项皆 `None`）。
pub fn take_handles(modules: &ProductionModules) -> ExternalToolHandles {
    ExternalToolHandles {
        bridge: modules.mcp_bridge().cloned(),
        module_bag: modules.mcp_module().cloned(),
    }
}

/// 启动期接线: 登记族授权 + 逐次人批标记, 再把发现的外部工具登记进能力表。
///
/// **不抛**：外部服务器不可达 ≠ 主机不可用 ⇒ 发现失败只如实记录到 stderr，
/// 启动继续（0 装诚实：主人要看得见「外部工具没接上」）。
pub async fn wire_at_startup(
    runtime: &Runtime,
    handles: &ExternalToolHandles,
    policy: &Mutex<PermissionPolicy>,
) {
    let Some(bridge) = handles.bridge.as_ref() else {
        return;
    };
    // 族授权 + 族逐次人批: 族内能力运行时才发现, 无法逐个静态登记 ⇒ 族根是
    // 它们唯一的共同前缀。人批权归运行时治理链 (owner 拍板: 一律逐次人批,
    // `full` 会话预设不豁免 —— 由预设层的族地板兜住)。
    authorize_external_tool_family(&mut policy.lock().expect("policy mutex"));
    let module_bag = handles.module_bag.as_ref().map(|bag| bag.as_ref());
    match wire_external_tools(runtime, bridge, module_bag, policy).await {
        Ok(wiring) => {
            if wiring.registered > 0 || wiring.unregistered > 0 {
                eprintln!(
                    "[external-tools] wired: {} registered, {} unregistered, {} live",
                    wiring.registered, wiring.unregistered, wiring.live
                );
            }
        }
        Err(reason) => {
            eprintln!("[external-tools] discovery failed: {reason}");
        }
    }
}
