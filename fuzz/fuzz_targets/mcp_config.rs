//! fuzz: MCP 配置与工具清单解析（外部输入面）。
//!
//! 两个靶子：
//! - `McpServerConfig::from_json_value` —— 配置文件解析（fail-closed 承诺）；
//! - `parse_tool_list` —— **MCP 服务器回给我们的工具清单**，这是真正的外部输入，
//!   由第三方进程决定内容。
//!
//! 判据：不 panic。两者都返回结构化错误是正常输出。
#![no_main]

use apeireth_tools_canonical::mcp_bridge::config::McpServerConfig;
use apeireth_tools_canonical::mcp_bridge::connection::parse_tool_list;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<Value>(data) else {
        return;
    };
    let _ = McpServerConfig::from_json_value(value.clone());
    let _ = parse_tool_list(value);
});
