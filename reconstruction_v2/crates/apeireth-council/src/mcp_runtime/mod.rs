//! Local MCP runtime — minimal Prompt/Resource/JsonRpc primitives (v2 适配).
//!
//! **v2 适配**:
//! v1 依赖 `apeireth_mcp` crate. v2 没有. 在本模块本地定义等价 runtime,
//! 子模块布局 `prompts` / `resources` / `protocol` 与 v1 apeireth_mcp::prompts / resources / protocol 1:1 镜像,
//! 让 mcp_bridge.rs 业务代码 0 改即可编译.

#![allow(missing_docs)]

pub mod prompts;
pub mod resources;
pub mod protocol;
