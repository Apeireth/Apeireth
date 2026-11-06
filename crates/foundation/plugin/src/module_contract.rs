//! Versioned metadata and projections over the existing module/capability catalog.
use crate::{FrozenInvocation, ToolCapability, ToolExecutionContext, ToolExecutionPolicy};
use apeireth_core::kernel::{ModuleInstanceId, PluginId, SessionId, TaskId};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const CONTRACT_METADATA_KEY: &str = "tool.contract.v1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSurface {
    #[default]
    Compatibility,
    Logical,
    Internal,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    #[default]
    Direct,
    Agentic,
    Hybrid,
    Background,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    #[default]
    Compatibility,
    Pure,
    Orchestration,
    Protected,
}
/// Metadata is a restriction/classification, never an authorization grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleToolContract {
    pub version: u16,
    pub surface: ToolSurface,
    pub execution: ExecutionMode,
    pub effect: EffectClass,
    pub module_visible: bool,
    #[serde(default)]
    pub query_safe: bool,
}
impl Default for ModuleToolContract {
    fn default() -> Self {
        Self {
            version: 1,
            surface: ToolSurface::Compatibility,
            execution: ExecutionMode::Direct,
            effect: EffectClass::Compatibility,
            module_visible: false,
            query_safe: false,
        }
    }
}
impl ModuleToolContract {
    pub fn logical(execution: ExecutionMode, effect: EffectClass) -> Self {
        Self {
            surface: ToolSurface::Logical,
            execution,
            effect,
            module_visible: true,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if (self.query_safe
            && (self.surface != ToolSurface::Logical
                || self.effect != EffectClass::Pure
                || self.execution != ExecutionMode::Direct))
            || self.version != 1
            || (self.surface == ToolSurface::Compatibility && self != &Self::default())
            || (self.surface == ToolSurface::Internal && self.module_visible)
            || (self.surface != ToolSurface::Compatibility
                && self.effect == EffectClass::Compatibility)
        {
            return Err("invalid module tool contract".into());
        }
        Ok(())
    }
    pub fn model_visible(&self) -> bool {
        self.surface != ToolSurface::Internal
    }
}
/// A thin Logical Entry -> module Rust policy -> existing executor adapter.
/// Native implementation, frozen payload and wire ABI are retained verbatim.
pub struct ContractTool {
    tool: Arc<dyn ToolCapability>,
    contract: ModuleToolContract,
}
impl ContractTool {
    pub fn wrap(
        tool: Arc<dyn ToolCapability>,
        contract: ModuleToolContract,
    ) -> Result<Arc<dyn ToolCapability>, String> {
        contract.validate()?;
        Ok(Arc::new(Self { tool, contract }))
    }
}
#[async_trait]
impl ToolCapability for ContractTool {
    fn id(&self) -> &apeireth_core::kernel::CapabilityId {
        self.tool.id()
    }
    fn declaration(&self) -> NormalizedTool {
        self.tool.declaration()
    }
    fn module_contract(&self) -> ModuleToolContract {
        self.contract.clone()
    }
    fn model_visible(&self) -> bool {
        self.contract.model_visible() && self.tool.model_visible()
    }
    fn execution_policy(&self) -> ToolExecutionPolicy {
        self.tool.execution_policy()
    }
    fn module_result_delivery(
        &self,
        result: &ToolResult,
    ) -> crate::tool_execution::ModuleResultDelivery {
        self.tool.module_result_delivery(result)
    }
    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        self.tool.freeze_invocation(call)
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        if crate::system_one::requires_system_one(&self.contract) {
            return ToolResult::permanent_error(
                &call.id,
                "protected internal execution requires the Runtime context",
            )
            .with_name(&call.name);
        }
        self.tool.invoke(call).await
    }
    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        if crate::system_one::requires_system_one(&self.contract) {
            return ToolResult::permanent_error(
                &call.id,
                "protected internal execution requires the Runtime context",
            )
            .with_name(&call.name);
        }
        self.tool.invoke_frozen(call, frozen).await
    }
    async fn invoke_in_session(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ToolResult {
        if crate::system_one::requires_system_one(&self.contract) {
            return ToolResult::permanent_error(
                &call.id,
                "protected internal execution requires the Runtime context",
            )
            .with_name(&call.name);
        }
        self.tool.invoke_in_session(call, frozen, session).await
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        context: &ToolExecutionContext,
    ) -> ToolResult {
        if crate::system_one::requires_system_one(&self.contract) {
            if let Err(error) =
                crate::system_one::require_internal_review(self.id(), call, frozen, context)
            {
                return error;
            }
        }
        self.tool.invoke_with_context(call, frozen, context).await
    }
}
/// An accepted call is closed at the ToolResult boundary; this identifies the
/// owned work for a separate authorized status query. It conveys no authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleTaskHandle {
    pub version: u16,
    pub module: PluginId,
    pub instance: ModuleInstanceId,
    pub task: TaskId,
}
