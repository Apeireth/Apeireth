# MCP 外部工具接入指南（工具桥）

本文说明如何把一台外部 MCP 服务器接入运行时：配置面 → 连接与发现 → 动态工具注册 →
调用链与治理。实现位于 `crates/capabilities/tools/src/mcp_bridge/`（协议库
`apeireth_plugin::mcp` 只消费不改写），生产装配位于
`crates/engine/runtime-assembly/src/canonical/production.rs`。

## 「接入一个服务器」使用说明

### 1. 写服务器配置

主入口是环境变量 `APEIRETH_MCP_SERVERS`，值为 JSON 数组：

```json
[
  {
    "name": "notes",
    "transport": "stdio",
    "command": "notes-mcp-server",
    "args": ["--root", "D:/data/notes"],
    "enabled": true
  },
  {
    "name": "remote-tools",
    "transport": "http",
    "url": "https://tools.example.test/mcp",
    "enabled": true
  }
]
```

- `name`：服务器名（`[a-z0-9][a-z0-9_-]*`），它会命名空间化每个工具
  （模型侧名字 `mcp:<server>:<tool>`，能力 id `tool.mcp.<server>.<tool>`）；
- `transport`：`stdio`（子进程 + 换行分隔 JSON-RPC）/ `http`（HTTP JSON-RPC
  直答）/ `sse`（事件流传响应 + 消息端点收请求）；
- `stdio` 必须给 `command`（可选 `args`），且不得带 `url`；`http` / `sse`
  必须给 `url`（http(s)），且不得带 `command`；
- `enabled`：显式给出，不设默认。

次入口是数据目录里的 `mcp-servers.json`，走存储文档信封
（`{"name":"mcp-servers","version":1,"compatible_versions":[1],"body":{"servers":[...]}}`），
用 `StoredDoc` 单文档拒开语义加载：坏 JSON / 串档 / 版本不可读一律**拒绝打开**，
不静默回退默认值。写入用 `apeireth_core::stored_doc::save_single`：

```rust
use apeireth_core::stored_doc;
use apeireth_tools_canonical::mcp_bridge::{doc_compat, server_list_path, McpServerConfig, McpServerSpec};

let body = McpServerConfig::new(vec![McpServerSpec::stdio("notes", "notes-mcp-server")]);
stored_doc::save_single(&server_list_path(&data_dir), &doc_compat(), body, stored_doc::DEFAULT_DOC_MODE)?;
```

优先级：`APEIRETH_MCP_SERVERS`（若设置则必须合法）> 数据目录文件 > 空列表。
任一入口解析/校验失败都是启动错误（fail-closed）。

### 2. 打开生产槽位

`ProductionModulesConfig.mcp` 默认 `false`（轻默认）；置 `true` 并给出数据目录后，
装配时会加载服务器列表并构造工具桥：

```rust
let config = ProductionModulesConfig {
    mcp: true,
    mcp_data_dir: Some(data_dir.clone()),
    ..ProductionModulesConfig::default()
};
let modules = ProductionModules::build(config, backends, clock)?;
let bridge = modules.mcp_bridge().expect("mcp slot enabled").clone();
```

CLI 适配器同款旋钮：`APEIRETH_ENABLE_MCP=1` 打开槽位，数据目录取面板持久档同位目录。

### 3. 连接、发现、注册（异步）

每个启用的服务器一个连接（生命周期机 + 断线重连预算）。发现入口是
`McpToolBridge::refresh()`：对每条链路跑 `tools/list`（链路断了就重开并重握手、
重发现），把结果与动态工具目录对账，返回需要注册/注销的增量：

```rust
let report = bridge.refresh().await?;            // 工具目录动态刷新
report.apply_to(mcp_module.as_ref())?;            // 落进 module bag（同名拒绝）
for tool in &report.registered {
    runtime.register_dynamic_tool("module.mcp", tool.clone())?;
}
```

- 工具命名空间：模型侧 `mcp:<server>:<tool>`，能力 id `tool.mcp.<server>.<tool>`；
- **禁重复注册**：同名（或同能力 id）在目录层与注册层都被拒绝，绝不合并/遮蔽；
- 服务器重连后 `refresh()` 会重新 `tools/list` 并对账：新增进 `registered`，
  消失进 `removed`，描述变化按「移除 + 新增」重新注册。

### 4. 治理映射（默认审批级）

外部工具的权限声明映射为 `PermissionPolicy` 显式授权；默认不授权 = 走审批：

- `McpPermissionMapping::authorize(policy, entry)`：显式授权一个外部工具；
- `McpPermissionMapping::require_approval(policy, entry)`：即使已授权也要审批；
- `APEIRETH_MCP_READONLY_PRESET=1`：放行**服务器声明为只读**的工具
  （`annotations.readOnlyHint == true`），仍走完整五段流水线；
- `McpToolDenyGuard`（单调 Guard，只拒不允许）：`deny_capability(...)` 硬拒指定能力。

### 5. 调用链

每次外部调用都穿五段流水线（`exec_pipeline`）：

1. pre 瀑布含风险映射（`mcp_risk_mapping`：未授权 → `pipeline.pre_ask` 待审批）；
2. 单调 Guard（`mcp_tool_deny` → `pipeline.guard_deny`）；
3. around：`apeireth_core::deadline` 超时（默认 30s，可配），MCP 调用超时归
   `timeout.*`（如 `timeout.deadline_expired`）；
4. post 纠错通道；
5. 输出归一合同：MCP 返回结构（`content` / `isError`）归一为
   `{text, content, is_error}` 再冻结成 `ToolOutcome`，形状漂移报
   `output_contract.*`。

### 6. 启动日志脱敏

`bridge.startup_log()` 给出每个服务器一行的脱敏摘要：URL 会记录（查询参数里的
token/密钥值、URL 密码、命令行里的密钥参数值一律 `[redacted]`）。适配器启动时
打印这些行即可，不要打印原始配置。

## 测试

- `crates/capabilities/tools/tests/mcp_bridge_e2e.rs`：发现/注册、调用全链、治理拦截
  （未授权→审批、封禁→拒绝）、只读预设放行、超时、断线重连后重发现、重复名拒绝、
  输出归一、脱敏、坏配置拒开——全部跑进程内 JSON-RPC 测试服务端，禁真实网络；
- `crates/engine/runtime-assembly/tests/mcp_bridge_runtime_e2e.rs`：生产装配加载
  （含坏配置拒开）、动态注册 + 整轮调用、重复注册拒绝。
