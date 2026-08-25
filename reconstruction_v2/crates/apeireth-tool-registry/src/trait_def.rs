//! **战役 2-1 / VCP §6.2.1 #12 + agentManager.js — Tool trait**
//!
//! **设计**:
//! - `Tool` async trait — 4 方法 (`name` / `kind` / `axes` / `call`)
//! - `call` 返 `Result<serde_json::Value, String>` — 简化, 跟 pipeline 错误流一致
//! - 所有实现必须 `Send + Sync` (因为 Registry 用 Arc<dyn Tool>)
//!
//! **字段级引用**:
//! - `agentManager.js:272-315 getAgentPrompt` — prompt 读取模式借鉴为 `call` 接口
//! - `Plugin.js:231-272 _executeStaticPluginCommand` — 静态插件执行模式借鉴为 call
//! - VCP `Plugin/<X>/plugin-manifest.json` 17KB 字段结构借鉴为 Tool 4 方法最小集

use async_trait::async_trait;
use serde_json::Value;

use crate::types::{ToolAxes, ToolKind};

/// **战役 2-1 — 工具 trait 最小集**
///
/// **4 方法**:
/// 1. `name()` — 工具唯一名 (e.g. `"FileOperator"`, 跟 VCP `Plugin/<X>/plugin-manifest.json:name`)
/// 2. `kind()` — 6 类之一 (VCP `pluginType` 字段)
/// 3. `axes()` — 5 轴正交属性
/// 4. `call(args)` — 异步执行, 返 JSON Value
///
/// **Send + Sync 约束**: Registry 用 `Arc<dyn Tool>` 跨线程共享, 必须 Send + Sync
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具唯一名 (e.g. `"FileOperator"`, `"WebSearch"`, `"DailyNoteWrite"`)
    fn name(&self) -> &str;

    /// 6 类 enum (VCP `pluginType` 字段级)
    fn kind(&self) -> ToolKind;

    /// 5 轴正交属性
    fn axes(&self) -> ToolAxes;

    /// 异步执行入口
    ///
    /// **args**: JSON Value (caller 端序列化, callee 端反序列化)
    ///
    /// **返**: `Ok(Value)` 成功 / `Err(String)` 失败 (简化, 跟 pipeline 一致;
    /// 不引入 thiserror 错误类型, 因为 Tool 是 user-defined interface)
    async fn call(&self, args: Value) -> Result<Value, String>;
}

/// 工具描述 (供 admin UI / list endpoint 用)
///
/// **借鉴**: VCP `Plugin/<X>/plugin-manifest.json` 字段 (description / version / author)
#[derive(Debug, Clone)]
pub struct ToolDescription {
    /// 工具名
    pub name: String,
    /// 6 类
    pub kind: ToolKind,
    /// 5 轴
    pub axes: ToolAxes,
    /// 简短描述 (≤ BRIEF token 预算)
    pub brief: String,
    /// 完整描述
    pub description: String,
    /// 版本
    pub version: String,
    /// 作者
    pub author: String,
}

impl ToolDescription {
    /// 默认空描述
    pub fn empty(name: impl Into<String>, kind: ToolKind, axes: ToolAxes) -> Self {
        Self {
            name: name.into(),
            kind,
            axes,
            brief: String::new(),
            description: String::new(),
            version: "0.1.0".to_string(),
            author: "unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AwaitingAxis, OutputAxis, ResidentAxis, TransportAxis, TriggerAxis};

    /// 测试用 mock Sync 工具
    struct MockSyncTool {
        name: String,
    }

    #[async_trait]
    impl Tool for MockSyncTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn kind(&self) -> ToolKind {
            ToolKind::Sync
        }
        fn axes(&self) -> ToolAxes {
            ToolAxes {
                trigger: TriggerAxis::OnDemand,
                awaiting: AwaitingAxis::Immediate,
                resident: ResidentAxis::Ephemeral,
                transport: TransportAxis::Local,
                output: OutputAxis::Value,
            }
        }
        async fn call(&self, _args: Value) -> Result<Value, String> {
            Ok(serde_json::json!({"status": "ok", "tool": self.name}))
        }
    }

    #[test]
    fn tool_trait_sync_basic() {
        // 同步工具可同步调 name/kind/axes
        let t = MockSyncTool {
            name: "test".to_string(),
        };
        assert_eq!(t.name(), "test");
        assert_eq!(t.kind(), ToolKind::Sync);
        let axes = t.axes();
        assert_eq!(axes.trigger, TriggerAxis::OnDemand);
    }

    #[tokio::test]
    async fn tool_trait_call_async() {
        // call 是 async
        let t = MockSyncTool {
            name: "echo".to_string(),
        };
        let result = t.call(serde_json::json!({"x": 1})).await.unwrap();
        assert_eq!(result["status"], "ok");
        assert_eq!(result["tool"], "echo");
    }

    #[test]
    fn tool_description_empty() {
        // 描述默认空
        let desc = ToolDescription::empty("foo", ToolKind::Async, ToolAxes::default());
        assert_eq!(desc.name, "foo");
        assert_eq!(desc.kind, ToolKind::Async);
        assert_eq!(desc.version, "0.1.0");
        assert_eq!(desc.author, "unknown");
        assert!(desc.brief.is_empty());
    }

    #[test]
    fn tool_trait_send_sync_compile_check() {
        // 编译期守: Tool 必须 Send + Sync (因为 Arc<dyn Tool> 跨线程)
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockSyncTool>();
    }
}
