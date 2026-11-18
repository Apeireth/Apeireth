//! `production.rs` 的纯装配辅助函数（B3b-2 拆出，装配根净增为 0）。
//!
//! 只是搬家：函数体逐字节不变，调用点仍按原顺序执行。

use std::sync::Arc;

use apeireth_plugin::{Plugin, ToolCapability};

use super::capability::CapabilityProvider;
use super::error::{RuntimeError, RuntimeResult};

/// 收集一个能力 provider 的工具模块与能力名册行。
pub(super) fn collect_tool_module(
    tool_modules: &mut Vec<Arc<dyn Plugin>>,
    capabilities: &mut Vec<Arc<dyn ToolCapability>>,
    provider: &dyn CapabilityProvider,
) -> RuntimeResult<()> {
    let module = provider
        .tool_module()
        .map_err(RuntimeError::misconfigured)?;
    capabilities.extend(module.tools());
    if !module.tools().is_empty() {
        tool_modules.push(Arc::new(module));
    }
    Ok(())
}
