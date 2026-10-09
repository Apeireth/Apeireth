# 自省通道：结构化自述与敏感面边界

> 给维护 Apeireth 的人：模型如何**实测自身状态再发言**，以及自省的边界在哪里。
> 机制描述以真实代码为准（自省通道批，写作时点 v2.0.0-rc.2）。
> 用户向的旋钮说明见 [user-manual.md](user-manual.md)；外部工具面见 [mcp-external-tools.md](mcp-external-tools.md)。

```
[Document-Meta]
Document:        docs/02-guides/self-insight-channel.md
Version:         Self-Insight-Rev-A（自省通道批）
Last-Modified:   2026-10-11
Status:          🟢 活跃
```

## 1. 为什么有这条通道

产品哲学「不假装」要求模型能**实测自身状态再发言**——不是背诵配置文本，不是
猜，而是像查仪表盘一样查出自己当前的真实状态。内测实测里出现过这样的行为：
模型通过翻自己的配置文件做自我报告，并据此完成过自我纠错。这是魅力，也是
能力。但「靠翻文件撞见自己」是巧合，不是设计：翻到的是配置文本而非生效值，
翻到什么取决于文件恰好躺在哪，凭据还可能在同一视野里。

自省通道把这件事**设计化**：

1. **专用自述面**（[第 2 节](#2-专用自述面self_status)）：一个只读工具，一次调用
   返回一份结构化自述 JSON，字段逐项对账到运行时**真实生效值**与可查元数据；
2. **敏感面治理**（[第 4 节](#4-敏感面边界fail-closed)）：凭据与密钥面对文件
   读取类工具 fail-closed，配置文件里的密钥字段值脱敏；
3. **数据目录可见性保留**（[第 3 节](#3-数据目录可见性为什么保留)）：数据目录
   作为默认工作区继续可见——可见性本身是自省能力，边界画在凭据面而不是整个
   数据目录。

一句话：**自述走正门，敏感面关门**。

## 2. 专用自述面：`self_status`

`self_status`（能力 id `tool.self_status`）是只读档、默认可用、零审批的
**结构化自述**工具：无参数、无副作用、只回事实。它是"模型对自己说话"的正道——
比起翻配置文件，它的三个关键差别：

| 维度 | 翻配置文件 | `self_status` |
|---|---|---|
| 取值 | 配置文本（可能与生效值不符） | **真实生效值**（装配真正用的开关、进程内生效调参） |
| 形状 | 随文件布局漂移 | 冻结合同（六节恒在场，缺失显式 `null` + 原因） |
| 边界 | 凭据可能同视野 | 凭据只回存在性布尔，本体绝不回显 |

### 2.1 字段表（输出合同）

顶层六节**恒在场**（输出归一合同 [`OutputSchema`](../../crates/capabilities/tools/src/self_status.rs)
要求六节都是对象）；来源缺失/读取失败的**字段显式 `null` 并在同节 `reason`
给出原因**，整帧仍然成功——自述宁可如实说"这块我没测到"，不整帧失败，更不
编造数字。

| 节 | 字段 | 类型 | 语义 | 缺失时 |
|---|---|---|---|---|
| `identity` | `product_name` | string | 产品名 | 恒在场 |
| | `version` | string | 版本（workspace 单轴版本值） | 恒在场 |
| | `runtime_role` | string | 运行时角色（`gateway-sidecar`） | 恒在场 |
| `capabilities` | `<开关名>` | boolean | 能力名册全表：**实际用于装配的生效值**（filesystem / search / repo / shell / fetch / mcp / 记忆族 / 器官 / 自学习等 25 行），含授权层 `local_read_tools` 旋钮 | 恒在场 |
| `memory_ledger` | `sessions` | number \| null | 会话数（episode 流不同会话计数） | null + `reason` |
| | `memories` | number \| null | 记忆件数（episode 计数） | null + `reason` |
| | `protected` | number \| null | 保护件数（治理层保护标记计数） | null + `reason` |
| | `lessons` | number \| null | 教训数（反思沉淀教训计数） | null + `reason` |
| | `reason` | string \| null | 计数缺失/局部失败的原因 | 可用时 null |
| `tuning` | `values.*` | number | 四滑杆当前生效值（遗忘衰减 / 好奇强度 / 语气饱和 / 整合节奏） | null + `reason` |
| | `preset` | string | 取值命中的预设名（`effortless` / `balanced` / `deep_memory` / `custom`） | null + `reason` |
| | `self_learning` | boolean | 自学习（自动调参）开关生效值 | null + `reason` |
| `budget` | `max_rounds_per_turn` | number | 单回合轮数上限 | null + `reason` |
| | `max_tool_calls_per_round` | number | 单轮工具调用上限 | null + `reason` |
| | `source` | string | 取值口径（`configured` = 生效中的可配置旋钮 / `constant` = 读编译期常量回退） | null + `reason` |
| | `note` | string | 口径注记（旋钮解析路径 / 常量回退均如实注明） | null + `reason` |
| `workspace` | `root` | string \| null | 工作区根路径 | null + `reason` |
| | `credentials_present` | boolean \| null | **凭据存在性**（只回布尔，本体绝不回显） | null + `reason` |

计数只回数字，**永不回记忆内容原文**；`credentials_present` 只回存在与否，
**永不回凭据本体**——这是自述与泄密的分界线。

### 2.2 失败语义

- **输出合同**：结果经既有 `ToolOutcome { ok, output_text, meta }` 归一冻结，
  结构化侧过声明的 `OutputSchema`；形状漂移是**明面上的归一错误**，不是静默
  强转。
- **超时**：采集跑在阻塞池上，data 系探测卡住时执行链的到期计时器仍然生效，
  超时归既有 `timeout.*` code 族（可重试帧），不把回合一起挂死。
- **错误即帧**：失败帧完整关联调用 id、带稳定 code；data 系来源失败只把对应
  字段置 `null` + 原因，不整帧失败。

## 3. 数据目录可见性为什么保留

数据目录（`~/.apeireth` 或 `APEIRETH_DATA_DIR`）是产品的默认工作区：记忆库、
会话库、学习日志、反思教训、面板档案都住在这里。**保留可见性**是有意的：

1. **可见性是自省能力**。模型能读到自己的学习日志、教训沉淀、记忆计数，才能
   实测"我学到了什么、我错在哪"——内测里的自我纠错正是从这里长出来的。
2. **透明是治理的前提**。主人和模型对同一份持久档可见，"记录透明"才成立；
   把数据目录整体藏起来等于把产品状态变成黑盒，恰恰违背「不假装」。
3. **边界画在凭据面，不是整个数据目录**。见下一节：能看的是状态，不能看的是
   钥匙。

## 4. 敏感面边界（fail-closed）

文件读取类工具（`filesystem` 的 read/list/stat、`search`）对**凭据与密钥面**
fail-closed：

1. **路径级拒绝**（不读、不列、不搜、不 stat）：
   - `<data>/creds.json` 及其备份变体（文件凭据存储落盘件）；
   - 钥匙串导出物（导出转储、导出后端数据件，如 `*.keyring` / `*.keychain` /
     `*-keyring.bin` / `keychain-export*` / 主密钥件）；
   - `.env` 族、密钥材料（私钥/证书/密钥库件）、常见凭据文件、产品内部数据库。
2. **字段级脱敏**：配置文件**可以读**，但 key/token/secret 等密钥字段的
   **字段值**脱敏为 `[redacted]`——与既有启动日志脱敏同一语义（字段名保留，
   非敏感字段逐字节照读）。覆盖 JSON / YAML / TOML / env 等常见配置行，含
   单行内联 JSON。
3. **拒绝即帧**：拒绝信息不是一句空话，而是带稳定 code 的完整帧（`pre_deny`
   语义），并说明事实：**凭据面不可读（安全契约）**。

**零回归口径**：配置文件的非敏感字段、日志、计数照读照查；数据目录其余内容
的可见性不变。边界只收在凭据与密钥面。

## 5. 接线与治理

- **治理**（`build_production_governance`）：`tool.self_status` 恒定 grant、
  不挂审批——只读档、默认可用、零审批。会话级只读预设不拦它（它不在
  写/执行分类表里），未知能力依旧 fail-closed。
- **装配**（`ProductionModules`）：`tool.self_status` 与既有 5 个内置工具
  （filesystem / search / repo / shell / fetch）同列注册；能力名册在**任何字段
  移动之前**从实际装配用的配置投影，杜绝"名册与生效值漂移"。
- **安全语义**（能力安全描述）：`tool.self_status` 登记为只读、无外联、
  `may_access_credentials = false`（只回存在性布尔）。

## 6. 新符号名一览（自省通道批）

| 符号 | 位置 | 职责 |
|---|---|---|
| `SelfStatusTool` | `crates/capabilities/tools/src/self_status.rs` | `tool.self_status` 只读工具本体 |
| `SelfStatusSource` / `SelfStatusSnapshot` | 同上 | 自述来源 trait / 快照数据 |
| `StatusProbe<T>` | 同上 | data 系探测口（失败回原因，不整帧失败） |
| `render_snapshot` | 同上 | 字段合同的唯一投影实现 |
| `MemoryLedgerStats` / `CapabilitySwitch` / `TuningStatus` / `BudgetStatus` / `WorkspaceStatus` | 同上 | 各节数据形状 |
| `derive_tuning_preset` / `DISPOSITION_PRESETS` | 同上 | 四滑杆取值 → 预设名推导 |
| `redact_secret_field_values` / `credential_surface_refusal` | `crates/capabilities/tools/src/sensitive_path.rs` | 字段值脱敏 / 凭据面拒绝帧（pre_deny 语义） |
| `SqliteMemoryStore::ledger_counts` / `MemoryLedgerCounts` | `crates/engine/memory/src/lib.rs` | 记忆账本计数（只回计数） |
| `ProductionSelfStatusSource` / `roster_from_config` | `crates/engine/runtime-assembly/src/canonical/self_status_source.rs` | 生产自述来源 / 能力名册生效值投影 |
| `budget_status_from_configured` / `budget_status_from_constants` | 同上 | 预算一节取值口径（生效旋钮 / 常量回退） |
| `SelfStatusModule` | `crates/engine/runtime-assembly/src/canonical/tool_modules.rs` | 工具注册（与 5 内置工具同列），五段流水线包执行 |
| `self_status_ledger_probe` / `self_status_credentials_probe` | `crates/adapters/cli/src/lib.rs` | 组装根探测口（计数 / 凭据存在性） |

## 7. 测试证据

- 单元：`crates/capabilities/tools/src/self_status.rs`（字段真实 / 缺失显式
  null+原因 / 输出归一合同 / 超时归 `timeout.*` / 预设推导）、
  `crates/capabilities/tools/src/sensitive_path.rs`（凭据面路径拒绝 / 脱敏 /
  非敏感字段零回归 / 拒绝帧语义）、
  `crates/engine/memory/tests/memory_ledger_counts.rs`（计数准确、只回计数）。
- 端到端：`crates/engine/runtime-assembly/tests/self_status_runtime_e2e.rs`
  （装配注册 / 名册=生效值 / 计数对账真存储 / 凭据布尔不回显 / 缺失显式 null）、
  `crates/adapters/cli/tests/self_status_e2e.rs`（生产装配零审批默认可用 /
  env 生效值名册 / 认知库计数 / 凭据存在性 / 预算旋钮同源 / 旧工具零回归）。
- 敏感面拒绝与脱敏在 `filesystem` / `search` 工具测试内联覆盖（含
  pre_deny 帧文本与 `[redacted]` 断言）。
