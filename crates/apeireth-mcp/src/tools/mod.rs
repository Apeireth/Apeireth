//! R125-4: MCP tools protocol — module root (拆分自原 R65 1 大文件)
//!
//! **拆分架构** (借鉴 `modelcontextprotocol/servers/src/everything/tools/<tool_name>.ts` 多文件模式):
//! - `types.rs` — `Tool` / `ToolContent` / `ToolCallResult` 3 个核心 struct + 4 错误码常量
//! - `server.rs` — `ToolServer` trait + `handle_tools_list` / `handle_tools_call` 2 个 JSON-RPC handler
//! - `naming.rs` — `is_valid_tool_name` kebab-case 校验 (借鉴 VCP toolCallParser.js)
//! - `browser.rs` — R123-3 浏览器自动化 skeleton (0 改)
//! - `mod.rs` (本文件) — 入口, 子 mod 声明 + re-export
//!
//! **MCP 协议 (per modelcontextprotocol/specification 2025-03-26)**:
//! - `tools/list` — 客户端列 server 端 tools (name / description / inputSchema)
//! - `tools/call` — 客户端按 name + arguments 调 tool, server 返 content[] (text/image/resource blocks)
//!
//! **入口签名 0 改 (per 8 硬墙 #3)**: `Tool` / `ToolContent` / `ToolCallResult` /
//! `ToolServer` / `handle_tools_list` / `handle_tools_call` / `is_valid_tool_name` /
//! `TOOL_NOT_FOUND` / `TOOL_INVALID_ARGS` / `TOOL_CALL_FAILED` / `TOOL_INTERNAL` 全部
//! `pub use` 从子 mod, 等价于原 mod.rs 公共 API (调用方 0 感知改动).
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `resources.rs` / `protocol.rs` / `ResourceServer` (LOCKED, R33-3 R33-3-1 0 触碰)
//! - 0 引入 I/O / 网络 (server 注入, 0 真接)
//! - 0 业务耦合 (apeireth-mcp 0 依赖 tui/api, 任意 server impl 都能挂)

pub mod browser; // R123-3: P2-12 浏览器自动化 skeleton (0 改, R125-4 例外授权)
pub mod naming; // R125-4: 拆分自原 mod.rs — `is_valid_tool_name`
pub mod server; // R125-4: 拆分自原 mod.rs — `ToolServer` trait + handlers
pub mod types; // R125-4: 拆分自原 mod.rs — `Tool` / `ToolContent` / `ToolCallResult` + 错误码

// ============================================================
// Re-exports (per 8 硬墙 #3 入口签名 0 改)
// ============================================================
// 重新导出全部 pub items, 等价于原 mod.rs 公共 API.
// 调用方 `apeireth_mcp::tools::Tool` / `apeireth_mcp::tools::handle_tools_list` 等
// 仍可访问, 0 感知内部拆分.
pub use naming::is_valid_tool_name;
pub use server::{handle_tools_call, handle_tools_list, ToolServer};
pub use types::{
    Tool, ToolCallResult, ToolContent, TOOL_CALL_FAILED, TOOL_INTERNAL, TOOL_INVALID_ARGS,
    TOOL_NOT_FOUND,
};

#[cfg(test)]
mod tests {
    //! R125-4: 拆 module 后, 验证公共 API 仍可访问 (test_tools_module_split_works).
    //!
    //! **目的**: 确保 `apeireth_mcp::tools::Tool` 等 pub items 仍可被外部 crate
    //! 通过 `apeireth_mcp::tools::*` 访问, 0 感知内部 4 文件拆分.
    use super::*;
    use crate::protocol::{Id, JsonRpcRequest};
    use serde_json::json;

    /// **拆 module 后, 所有 tool 仍可调** (test_tools_module_split_works)
    ///
    /// 通过 `apeireth_mcp::tools::Tool` / `apeireth_mcp::tools::handle_tools_list` /
    /// `apeireth_mcp::tools::handle_tools_call` 入口, 验证拆分前 + 拆分后行为一致.
    #[test]
    fn test_tools_module_split_works() {
        // 1) 验证 Tool 仍可通过 `apeireth_mcp::tools::Tool` 访问
        let t = Tool::new("split-echo").with_description("post-split tool");
        assert_eq!(t.name, "split-echo");
        assert_eq!(t.description.as_deref(), Some("post-split tool"));

        // 2) 验证 handle_tools_list 仍可调
        struct MinimalServer;
        impl ToolServer for MinimalServer {
            fn list(&self) -> Vec<Tool> {
                vec![Tool::new("x")]
            }
            fn call(
                &self,
                name: &str,
                _arguments: &serde_json::Value,
            ) -> Result<ToolCallResult, crate::protocol::JsonRpcError> {
                if name == "x" {
                    Ok(ToolCallResult::ok(vec![ToolContent::text("ok")]))
                } else {
                    Err(crate::protocol::JsonRpcError::new(
                        TOOL_NOT_FOUND,
                        format!("not found: {name}"),
                    ))
                }
            }
        }
        let req = JsonRpcRequest::new("tools/list", None, Id::Num(1));
        let resp = handle_tools_list(&req, &MinimalServer);
        assert!(resp.error.is_none());

        // 3) 验证 handle_tools_call 仍可调
        let req = JsonRpcRequest::new(
            "tools/call",
            Some(json!({"name": "x", "arguments": {}})),
            Id::Num(2),
        );
        let resp = handle_tools_call(&req, &MinimalServer);
        assert!(resp.error.is_none());

        // 4) 验证 is_valid_tool_name 仍可调
        assert!(is_valid_tool_name("post-split"));
        assert!(!is_valid_tool_name("Post_Split"));
    }
}
