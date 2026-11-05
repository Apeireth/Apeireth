# MCP 外部工具接入指南（工具桥）

本文说明如何把一台外部 MCP 服务器接入运行时：配置面 → 连接与发现 → 动态工具注册 →
调用链与治理。实现位于 `crates/capabilities/tools/src/mcp_bridge/`（协议库
`apeireth_plugin::mcp` 只消费不改写），桥面装配位于
`crates/engine/runtime-assembly/src/canonical/production.rs`；生产接线（发现 → 登记 →
族授权 + 逐次人批 → 派发）位于
`crates/engine/runtime-assembly/src/canonical/external_tool_bridge.rs`，组合根的调用点在
`crates/adapters/cli/src/external_tools.rs`。

> **口径**：外部工具面落在 **PRODUCTION WIRED**（接入 canonical 运行时主路径）｜
> **DEFAULT ENABLED 不成立**（默认关，开关 `APEIRETH_ENABLE_MCP=1`）｜
> **HARDWARE VALIDATED 不成立**（2026-11-05 的 live 探针是仓外自建的最小 stdio 探针，
> **不是**真实第三方 MCP 服务器）。

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
    "enabled": true,
    "auto_authorize_tools": ["list_notes"],
    "deny_tools": ["delete_all"]
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
- `enabled`：显式给出，不设默认；
- `auto_authorize_tools`（可选）：**服务器侧工具名**白名单 —— 列内的工具进授权面
  （记成显式 grant）。**它不等于「免人批」**：外部工具一律逐次人批（见 §5），
  `full` 会话预设也不豁免；
- `deny_tools`（可选）：**服务器侧工具名**黑名单 —— 列内的工具被封（见 §6）。

两个可选列表都是 `#[serde(default)]` ⇒ **老配置零改动兼容**：不带这两个字段的旧配置
照样开。

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
**开关关 ⇒ 桥面根本不装配**（stderr 里没有 `[external-tools]` 行，见 §11）。

### 3. 连接、发现、注册（异步）

每个启用的服务器一个连接（生命周期机 + 断线重连预算）。发现入口是
`McpToolBridge::refresh()`：对每条链路跑 `tools/list`（链路断了就重开并重握手、
重发现），把结果与动态工具目录对账，返回需要注册/注销的增量。

**生产路径**（组合根已经在跑，你不用自己接）：启动期接线由
`crates/adapters/cli/src/external_tools.rs` 的 `wire_at_startup()` 调
`wire_external_tools()`（`external_tool_bridge.rs`），四步：

1. **发现**：`bridge.refresh()` —— **fail-all**：任一服务器失败 ⇒ 整次失败，
   **不半注册**（半注册 = 能力表与实况漂移，比不接更难查）；
2. **登记**：`RuntimeExternalToolRegistry::register` → `Runtime::register_dynamic_tool`
   ⇒ 进运行时能力表 ⇒ `tool_declarations()` 自动带上 `mcp:<server>:<tool>`；
3. **撤销**：目录里消失的条目进 `removed` ⇒ 登记面 `unregister`（模型目录随之少了它）；
4. **授权**：族授权 + 族逐次人批（见 §5）。

库级形状（自己接桥时用；`registry` 是任意实现了 `McpToolRegistry` 的登记面，
生产实现是 `RuntimeExternalToolRegistry`）：

```rust
let report = bridge.refresh().await?;            // 工具目录动态刷新
report.apply_to(&registry)?;                    // 落进登记面（同名拒绝 ⇒ 报错）
```

- 工具命名空间：模型侧 `mcp:<server>:<tool>`，能力 id `tool.mcp.<server>.<tool>`；
- **禁重复注册**：同名（或同能力 id）在目录层与注册层都被拒绝，绝不合并/遮蔽；
  （登记面另有**幂等**口径：同一件能力重复登记 = 成功，不是错 —— 周期刷新与启动期
  接线可能撞上同一件工具。）
- 服务器重连后 `refresh()` 会重新 `tools/list` 并对账：新增进 `registered`，
  消失进 `removed`，描述变化按「移除 + 新增」重新注册。

### 4. 周期刷新

默认 **60_000 ms**（`McpBridgeOptions::refresh_interval_ms`；`0` = 关闭周期刷新，
只在显式 `refresh()` 时刷新）。这是**程序化配置**：组合根按需构造 `McpBridgeOptions`，
**没有 env 旋钮**。

- **逐服务器隔离**：某服务器不可达 ⇒ 它本轮**不产出增量** ⇒ 它的工具**留在**能力表里
  （不可达不是「工具没了」的证据 ⇒ **不误撤**）；其余服务器的增量照常落地；
- **幂等**：同一件能力重复登记 = 成功；
- **可观测**：失败逐条进观察回调 ⇒ 记 stderr（`[external-tools] periodic refresh
  degraded: …`），**不静默**。

### 5. 治理映射（族授权 + 逐次人批）

外部工具的权限声明映射为 `PermissionPolicy` 显式授权；默认不授权 = 走审批：

- `McpPermissionMapping::authorize(policy, entry)`：显式授权一个外部工具；
- `McpPermissionMapping::require_approval(policy, entry)`：即使已授权也要审批；
- `APEIRETH_MCP_READONLY_PRESET=1`：放行**服务器声明为只读**的工具
  （`annotations.readOnlyHint == true`），仍走完整五段流水线；
- `McpToolDenyGuard`（单调 Guard，只拒不允许）：`deny_capability(...)` 硬拒指定能力；
  它的**生产注入口**是每服务器 `deny_tools` 配置面（见 §6）。

**族授权底座**：族内能力是运行时才发现的，无法逐个静态登记 ⇒ 授权落在**族**上：

- 族授权 `grant_family("tool.mcp")` + 族逐次人批 `require_approval_for_family("tool.mcp")`
  ⇒ 族内能力的裁决是 `RequireApproval`（**不是** `Allow`，也**不是** `Deny`）
  ⇒ 运行时挂起该次派发等人决定；
- 族根 `tool.mcp`，**段边界**判定：`tool.mcp.demo.lookup` 属于该族，
  `tool.mcpsomething` / `tool.mcp2.demo` 不属于；
- 未授权的能力 ⇒ `Decision::Deny`（fail-closed）。

**族审批地板**：预设层的 `human_approval_floor_applies` 给外部工具族单开一条分支 ——
族内能力**一律逐次人批**，`full` 会话预设**不豁免**，`standard` 的免审记忆**也不豁免**
（owner 拍板）。

**授权的落点**：生产组合根把族登记写进生产权限策略，同时给桥内闸**族放行**。桥内闸是
deny-only 的：拒绝即拒；放行或「要人批」一律**透传**给运行时治理链，桥内不重复要批
（重复要批会把已挂起的调用变成硬失败，断掉审批恢复路径）。

### 6. 拉黑一个工具（`deny_tools` 配置面）

`deny_tools` 是每服务器一条**黑名单**，值是**服务器侧工具名**（与
`auto_authorize_tools` 同一个名字面，不是模型面名）：

```json
[
  {
    "name": "notes",
    "transport": "stdio",
    "command": "notes-mcp-server",
    "enabled": true,
    "deny_tools": ["delete_all", "purge_cache"]
  }
]
```

生效路径：装配期 `McpToolBridge::new` 把每个服务器的 `deny_tools` 折进单调 deny 闸
（`seed_deny_list` ⇒ `McpToolDenyGuard::deny_server_tools`），闸**先于第一次发现**就武装好。

- **「不进目录」语义**：被封的工具**不进动态目录** ⇒ 不登记、不进模型工具目录、
  模型**看不见**它 ⇒ 也不会被 T1 挂起去问主人一个必然失败的问题；
- **闸挂在流水线 Stage 2**（Guard，早于 Stage 3 的真正执行）⇒ 拒绝即终局；
- **优先级**：**deny > 只读预设放行 > 族授权放行** —— 被封的工具即使被声明为只读、
  即使被 `auto_authorize_tools` 列进白名单、即使有族授权覆盖，一律封死；
- **坏配置拒开**：`deny_tools` 条目与 `auto_authorize_tools` 条目**同一语法函数**
  （`is_valid_mcp_name`）、**同一错误档位**；`enabled:false` 的服务器上的坏条目
  **同样拒开**（整份配置拒开，不静默跳过）；
- **已注册的怎么撤销**：靠**既有对账逻辑** —— 目录 `replace_server` 天然把不再出现的
  条目报成 `removed` ⇒ 登记面注销 ⇒ 模型目录随之少了它 ⇒ **无需特例**。

⚠️ **配置面：改文件 + 重启生效** —— `deny_tools` 是**装配期**读的配置，**没有**运行时
热更新面：改 `APEIRETH_MCP_SERVERS` 或数据目录的 `mcp-servers.json` 之后要**重启进程**
才生效。

⚠️ **主人可见的「当场拉黑」面仍不存在** —— 库 API 有
（`bridge.deny_guard().deny_capability("tool.mcp.notes.delete_all")`，下一轮刷新即生效），
但**没有**主人面的运行时封禁操作（那是另开批的活）。

本节的 `deny_tools` 配置面与本指南**同日（2026-11-05）**落地（提交 `6f1a543a`）。

### 7. 调用链

每次外部调用都穿五段流水线（`exec_pipeline`）：

1. pre 瀑布含风险映射（`mcp_risk_mapping`：未授权 → `pipeline.pre_ask` 待审批；
   放行与「要人批」都**透传**给运行时治理链）；
2. 单调 Guard（`mcp_tool_deny` → `pipeline.guard_deny`；deny 闸在这里）；
3. around：`apeireth_core::deadline` 超时（默认 30s，可配），MCP 调用超时归
   `timeout.*`（如 `timeout.deadline_expired`）；
4. post 纠错通道；
5. 输出归一合同：MCP 返回结构（`content` / `isError`）归一为
   `{text, content, is_error}` 再冻结成 `ToolOutcome`，形状漂移报
   `output_contract.*`。

### 8. 声明面 / 结果面有界收敛

服务器自己写的文字（工具描述、入参 schema、返回结果）默认会原样进模型上下文。
工具桥在模型面边界做**有界收敛**（`crates/capabilities/tools/src/mcp_bridge/redaction.rs`）：

| 面 | 上限 |
|---|---|
| 工具描述 | ≤ **500** 字符 |
| schema 内单个字符串 | ≤ **200** 字符 |
| 属性名 | ≤ **64** 字符 |
| schema 嵌套深度 | ≤ **8** |
| 每个对象的属性数 | ≤ **64**（`required` 属性优先保留） |
| schema 节点总预算 | **512** |
| 结果正文 | ≤ **64 Ki** 字符 |
| 结果块数 | ≤ **8** |
| 单块内联 data | ≤ **16 Ki** |

控制字符清洗**复用**治理层已有的字符类
（`crates/foundation/governance/src/tool_desc_audit.rs` 的 `ToolDescAuditor::is_dangerous_char`：
零宽族 + Bidi 覆写/隔离 + 软连字符 + C0/DEL/C1），再追加 LRM/RLM 与标签字符，
`\r` 另由控制字符清洗去掉。

⚠️ **诚实边界**：结果面**不做内容级凭据识别** ⇒ 远端回吐的凭据**仍会进模型上下文**；
防线是**有界 + 结构收敛 + 逐次人批**，不是识别。也不做**关键词黑名单**
（会误伤正常内容，换语种/拆分即绕过）。

### 9. 启动日志脱敏

`bridge.startup_log()` 给出每个服务器一行的脱敏摘要：URL 会记录（查询参数里的
token/密钥值、URL 密码、命令行里的密钥参数值一律 `[redacted]`）。适配器启动时
打印这些行即可，不要打印原始配置。

### 10. 名册口径

自报名册里有两行外部工具（`crates/engine/runtime-assembly/src/canonical/self_status_roster.rs`）：

- `mcp` = **实际登记进运行时能力表的外部工具件数 ≥ 1**（真接线口径：发现 → 登记 →
  族授权 + 逐次人批 → 派发）；未接线 / 发现失败 / 无服务器 ⇒ 恒 false（fail-closed）；
- `mcp_bridge` = **桥面已装配**（配置面事实，与件数无关）。

旧口径「服务器列表已加载」**不再**占用 `mcp` 行 —— 它冒充过「MCP 工具可用」。

### 11. 自查：三种启动情形

2026-11-05 实跑（live 探针 = 仓外自建的最小 stdio 探针，**不是**真实第三方 MCP 服务器）：

| 情形 | stderr |
|---|---|
| 开关开 + 服务器可达 | `[external-tools] wired: N registered, …, L live` |
| 开关关 | **无** `[external-tools]` 行 ⇒ 桥面不装配 |
| 开关开 + 服务器不可达 | `[external-tools] discovery failed: …`，**启动不中断** |

## 诚实边界

- **不是真服务器验证**：2026-11-05 的 live 探针是仓外自建的最小 stdio 探针
  ⇒ HARDWARE VALIDATED 不成立；
- **结果面不做凭据识别**：远端回吐的凭据仍会进模型上下文；
- **没有当场拉黑面**：主人面暂无运行时封禁操作，只有配置面（改文件 + 重启生效）
  与库 API（下一轮刷新生效）两条路。

## 测试

- `crates/capabilities/tools/tests/mcp_bridge_e2e.rs`：发现/注册、调用全链、治理拦截
  （未授权→审批、封禁→拒绝）、只读预设放行、超时、断线重连后重发现、重复名拒绝、
  输出归一、脱敏、坏配置拒开——全部跑进程内 JSON-RPC 测试服务端，禁真实网络；
- `crates/engine/runtime-assembly/tests/mcp_bridge_runtime_e2e.rs`：生产装配加载
  （含坏配置拒开）、动态注册 + 整轮调用、重复注册拒绝；
- `crates/capabilities/tools/tests/mcp_deny_guard.rs`：`deny_tools` 配置面专项 ——
  坏条目拒开、配置往返、闸先于第一次发现武装、被封工具不进目录也不进登记面、
  deny 压过族授权 / 只读预设 / 操作者白名单、被封工具在下一轮刷新后离开登记面、
  重连后仍是封的。

## 相关文档

- [MCP 外部工具桥四环 —— 结案记录（2026-11-05）](../status/CLOSEOUT-MCP-EXTERNAL-TOOLS-2026-11-05.md)：
  W9 结案记录（四环接通证据 / 四级口径落点 / 遗留项清单）。
