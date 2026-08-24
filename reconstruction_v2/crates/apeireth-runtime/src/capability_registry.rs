//! CapabilityRegistry - 工具 / 能力 / agent 抽象注册表
//!
//! 0 装 PASS: 从 UnifiedRuntimeHost (host.rs:72 tool_registry: Arc<ToolRegistry>)
//! 抽取并升级为更通用的 Capability 抽象。
//!
//! 设计动机:
//! - ToolRegistry 只管 "工具" (functions callable by agent)
//! - 但 v2.0 还要管: skills (workflow templates), agents (subagent definitions),
//!   memory providers (knowledge bases), prompt templates
//! - CapabilityRegistry 提供统一 trait + 分类 + 查询, ToolRegistry 作为最常用 impl
//!
//! 0-breaking: host.rs 现在仍持有 tool_registry: Arc<ToolRegistry>,
//! 但也持有 capabilities: CapabilityRegistry (内部包 ToolRegistry + 未来扩展)。
//! 现有调用方 (self.tool_registry.list_tools()) 不动。

use apeireth_tools::ToolRegistry;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 能力类别 (0 装 PASS: 当前只实装 Tool, 其他扩展是 #[allow(dead_code)] 接口预留)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityKind {
    /// 可被 agent 直接调用的工具 (ToolRegistry 承载)
    Tool,
    /// 工作流 / Skill 模板 (Phase 2 扩展)
    Skill,
    /// 子 agent 定义 (Phase 2 扩展)
    Agent,
    /// 外部知识库 / memory provider (Phase 2 扩展)
    Memory,
    /// Prompt 模板 (Phase 2 扩展)
    Prompt,
}

/// 单个能力的元数据 (0 装 PASS: 字段精简, 只描述能力边界)
#[derive(Debug, Clone)]
pub struct CapabilityMeta {
    pub kind: CapabilityKind,
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for RiskLevel {
    fn default() -> Self { RiskLevel::Low }
}

/// CapabilityRegistry - 通用能力注册表 (ToolRegistry 是它的核心 impl 之一)
pub struct CapabilityRegistry {
    /// 工具注册表 (主要 backend, 来自 apeireth-tools)
    pub tools: Arc<ToolRegistry>,
    /// 未来扩展位 (Phase 2: skills / agents / memory / prompts)
    #[allow(dead_code)]
    skills: RwLock<HashMap<String, CapabilityMeta>>,
    #[allow(dead_code)]
    agents: RwLock<HashMap<String, CapabilityMeta>>,
    #[allow(dead_code)]
    memory_providers: RwLock<HashMap<String, CapabilityMeta>>,
    #[allow(dead_code)]
    prompts: RwLock<HashMap<String, CapabilityMeta>>,
}

impl CapabilityRegistry {
    /// 0 装 PASS: 与原 host.rs::new() 中 tool_reg 构造一致
    pub fn new(tools: Arc<ToolRegistry>) -> Self {
        Self {
            tools,
            skills: RwLock::new(HashMap::new()),
            agents: RwLock::new(HashMap::new()),
            memory_providers: RwLock::new(HashMap::new()),
            prompts: RwLock::new(HashMap::new()),
        }
    }

    /// 0 装 PASS: 封装 ToolRegistry.list_tools() 调用 — gateway 仍可调用此方法
    pub fn list_tool_names(&self) -> Vec<String> {
        self.tools
            .list_tools()
            .iter()
            .map(|t| t.name.clone())
            .collect()
    }

    /// 0 装 PASS: 封装 ToolRegistry.list_tools() 返回完整 ToolDefinition
    pub fn list_tools(&self) -> Vec<apeireth_tools::ToolDefinition> {
        self.tools.list_tools()
    }
}

impl std::fmt::Debug for CapabilityRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CapabilityRegistry {{ tools: {} tool(s) }}",
            self.tools.list_tools().len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tools::{Tool, ToolDefinition, ToolResult};
    use async_trait::async_trait;

    struct NoopTool;
    #[async_trait]
    impl Tool for NoopTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "noop".into(),
                description: "no-op".into(),
                risk_level: apeireth_tools::RiskLevel::Low,
            }
        }
        async fn execute(
            &self,
            _p: serde_json::Value,
        ) -> Result<ToolResult, apeireth_tools::ToolError> { Ok(ToolResult::success("")) }
    }

    #[test]
    fn test_capability_registry_new() {
        let tool_reg = Arc::new(ToolRegistry::new());
        tool_reg.register(Arc::new(NoopTool));
        let cr = CapabilityRegistry::new(tool_reg);
        assert_eq!(cr.list_tool_names(), vec!["noop".to_string()]);
        assert_eq!(cr.list_tools().len(), 1);
    }
}
