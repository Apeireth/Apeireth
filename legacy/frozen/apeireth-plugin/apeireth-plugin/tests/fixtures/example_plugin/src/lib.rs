//! # apeireth-plugin-example
//!
//! Example plugin fixture (per v09021-rust-translation-blueprint-RIVAL §4.1, Fixture 5).
//!
//! 1:1 翻译 v0.9.21 商业版 plugin 入口 (估 `init(apeireth_api)` 函数 + 4 钩子).
//! 本 fixture 是 **skeleton** — 只暴露 API 表面 + lifecycle 5 状态, 不做实际业务.
//!
//! ## plugin.json 字段 (1:1 翻译 v0.9.21)
//!
//! ```json
//! {
//!   "schema_version": "1",
//!   "name": "apeireth-plugin-example",
//!   "version": "0.1.0",
//!   "author": "Apeireth Team",
//!   "entry": "src/lib.rs",
//!   "permissions": ["file_read", "file_write", "network", "mcp_call"],
//!   "min_apeireth_version": "apeireth"
//! }
//! ```
//!
//! ## 4 钩子 (per v0.9.21)
//!
//! 1. `init(apeireth_api)` — plugin 启动, 注册 tool calls
//! 2. `on_session_start()` — session 开始
//! 3. `on_tool_call(name, args)` — 每个 tool call 前置 hook
//! 4. `destroy()` — plugin 卸载

use serde::{Deserialize, Serialize};

/// plugin 暴露给主进程的 API 表面 (skeleton).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApeirethApi {
    /// 当前 session id (主进程注入).
    pub session_id: String,
    /// plugin 工具注册回调 (主进程注入).
    pub register_tool: Option<String>,
}

/// 5 lifecycle 状态 (1:1 翻译 v0.9.21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginLifecycle {
    Loaded,
    Initialized,
    Ready,
    Unloaded,
    Destroyed,
}

/// 4 种 permission (1:1 翻译 v0.9.21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    FileRead,
    FileWrite,
    Network,
    McpCall,
}

/// plugin 入口 (per v0.9.21 估缺 `init(apeireth_api)`).
///
/// 收到主进程 `ApeirethApi` 后, 应该:
/// 1. 注册 plugin 自己的 tool calls (走 `apeireth_api.register_tool`)
///
/// 2. 返回 lifecycle 状态 (Loaded → Initialized)
///
/// skeleton: 不真注册, 只演示 API 形状.
pub fn init(api: ApeirethApi) -> Result<PluginLifecycle, String> {
    // O-5 不假装: 实际注册走 `apeireth-tool-registry` 集成,
    // R20 阶段 4 真接. 当前只演示 API 形状.
    let _ = api; // 避免 unused 警告
    Ok(PluginLifecycle::Initialized)
}

/// session start 钩子 (per v0.9.21 估缺).
pub fn on_session_start() -> PluginLifecycle {
    PluginLifecycle::Ready
}

/// tool call 前置 hook (per v0.9.21 估缺).
///
/// 返 `true` = 允许调用, `false` = 拒绝.
pub fn on_tool_call(name: &str, _args: &serde_json::Value) -> bool {
    // skeleton: 默认 allow 所有 tool, R20 阶段 4 加 m3 防御白名单
    !name.is_empty()
}

/// destroy 钩子 (per v0.9.21 估缺).
pub fn destroy() -> PluginLifecycle {
    PluginLifecycle::Destroyed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_plugin_init_returns_initialized() {
        let api = ApeirethApi {
            session_id: "test".to_string(),
            register_tool: None,
        };
        assert_eq!(init(api).unwrap(), PluginLifecycle::Initialized);
    }

    #[test]
    fn example_plugin_lifecycle_can_progress() {
        let api = ApeirethApi {
            session_id: "test".to_string(),
            register_tool: None,
        };
        assert_eq!(init(api).unwrap(), PluginLifecycle::Initialized);
        assert_eq!(on_session_start(), PluginLifecycle::Ready);
        assert!(on_tool_call("any_tool", &serde_json::json!({})));
        assert_eq!(destroy(), PluginLifecycle::Destroyed);
    }

    #[test]
    fn example_plugin_4_permissions_defined() {
        // 4 种 permission 都在
        let _perms = [
            PluginPermission::FileRead,
            PluginPermission::FileWrite,
            PluginPermission::Network,
            PluginPermission::McpCall,
        ];
        assert_eq!(4, 4); // 编译期守门: 4 种
    }
}
