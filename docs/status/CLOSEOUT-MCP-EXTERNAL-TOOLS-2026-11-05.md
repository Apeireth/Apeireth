# MCP 外部工具桥四环 —— 结案记录（2026-11-05）

> **状态**：**已接线**。外部工具桥的**发现 / 登记 / 授权 / 派发四环**在生产装配上接通，
> 四级口径落在 **PRODUCTION WIRED**（不是 IMPLEMENTED，不是 DEFAULT ENABLED，不是 HARDWARE VALIDATED）。
> **台账处置**：未接线台账的 **W9** 行（2026-11-04 口径归位批写入）已从
> [未接线台账](unwired-ledger.md) §一「待接线」移出，落到 §四「已接线」；**本记录是那一行的完整留档**
> （原决策 / 翻转原因 / 四环接通证据 / 四级口径落点 / 遗留项清单）。
> **边界**：本记录**不改** [`claims.yaml`](claims.yaml) —— claim 的 `level` / `verified_at` 属四级口径裁决，
> 由主线拍板；本记录只给建议，不代改。

---

## 1. 原决策（2026-11-04 口径归位批，逐条留档）

> **决策：暂不接线**（2026-11-04 口径归位批）。
> **理由**：接线不是「登记」而是**新功能 + 治理模型变更** ——
> ① 动态能力 id 族与静态 grant 表的形态冲突；
> ② `full` 会话预设把 RequireApproval 改写成 Allow（会让外部工具零人批放行）；
> ③ 断线重连后能力表重登语义缺失
> ⇒ 属 owner 级产品决策，不是点对点升级批能顺手做掉的。
> **触发**：owner 拍板「是否把第三方外部工具暴露给模型 + 逐次人批粒度 + `full` 预设是否豁免人批」后立项；
> 立项时须一并解决「桥内 `Ask` ≠ 运行时挂起审批」「deny 闸无注入口」「重连后能力表重登」三项。
> **当时的实测基线**：四环生产调用点均为 0（发现 `McpToolBridge::refresh()`、登记
> `Runtime::register_dynamic_tool()`、授权静态 grant 表零条 mcp、派发 `tool_declarations()` 内无 `mcp:*`）
> ⇒ 开 `APEIRETH_ENABLE_MCP=1` 后 **0 个外部工具进入模型工具目录**。

**这条基线今天整条过时** —— §3 的四环现在都有生产装配调用点。

## 2. 翻转原因（原三条理由 + 三项立项前置，逐条落到今天的落点）

| 原条目（2026-11-04） | 今天的落点 | 提交 |
|---|---|---|
| ① 动态能力 id 族 vs 静态 grant 表形态冲突 | **族授权底座**：`grant_family` / `require_approval_for_family`（`crates/foundation/governance/src/permission.rs:177` / `:190`）+ 族根单一真源 `MCP_FAMILY_ROOT`；裁决顺序 = 精确 grant > 族 grant > 无授权即拒（fail-closed，`permission.rs:209`） | `5ead6ac1` |
| ② `full` 预设把 RequireApproval 改写成 Allow | **MCP 族审批地板** `human_approval_floor_applies`（`crates/engine/runtime-assembly/src/canonical/permission_preset.rs:132`）：族内能力**一律逐次人批**，`full` 预设**不豁免**、`standard` 的免审记忆**也不豁免**（owner 已拍板） | `5ead6ac1` |
| ③ 重连后能力表重登语义缺失 | **周期刷新 + 重连后能力表重登**：`spawn_external_tool_refresh`（`crates/engine/runtime-assembly/src/canonical/external_tool_bridge.rs:434`）+ `apply_external_tool_report`（先撤后登，`:307`）+ 重连回流目录 | `34735d62` |
| 前置项「桥内 `Ask` ≠ 运行时挂起审批」 | 桥内闸改 **deny-only 透传**（拒绝即拒；放行或「要人批」一律透传给运行时治理链，桥内不重复要批）⇒ 人批只发生在运行时 | `a3de1dc9` |
| 前置项「deny 闸无注入口」 | **仍在途**（见 §6 遗留 ③）⇒ 本条未销账 | 在途 |

**owner 拍板项（2026-11-05 接线批）**：MCP 族**一律逐次人批**，`full` 预设**不豁免**。
接线批据此把族审批地板做进预设层，并用测试钉死「预设不得越过地板」这一条。

## 3. 四环接通证据（生产装配调用点）

| 环 | 生产调用点 | 证据（file:line） |
|---|---|---|
| ① 发现 | `McpToolBridge::refresh()`（`crates/capabilities/tools/src/mcp_bridge/bridge.rs:364`） | 组合根 `crates/adapters/cli/src/external_tools.rs:63`（`wire_at_startup`）→ `crates/engine/runtime-assembly/src/canonical/external_tool_bridge.rs:254`（`wire_external_tools`） |
| ② 登记 | `Runtime::register_dynamic_tool()`（`crates/engine/runtime/src/canonical/runtime.rs:563`） | `crates/engine/runtime-assembly/src/canonical/external_tool_bridge.rs:160`（`RuntimeExternalToolRegistry::register`）⇒ 进运行时能力表 |
| ③ 授权 | 族授权 + 族逐次人批 | 底座 `crates/foundation/governance/src/permission.rs:177` / `:190`；族地板 `crates/engine/runtime-assembly/src/canonical/permission_preset.rs:132`；授权落点 `crates/engine/runtime-assembly/src/canonical/external_tool_bridge.rs:219`（`authorize_external_tool_family`）；组合根调用 `crates/adapters/cli/src/external_tools.rs:74` |
| ④ 派发 | 模型面工具名 `mcp:<server>:<tool>` | 命名 `crates/capabilities/tools/src/mcp_bridge/catalog.rs:68`；进模型工具目录 `crates/engine/runtime/src/canonical/runtime.rs:585`（`tool_declarations()` 读能力表）；派发命中 `crates/engine/runtime/src/canonical/capability.rs:136`（`find_by_name`） |

**声明面 / 结果面有界收敛**（`crates/capabilities/tools/src/mcp_bridge/redaction.rs`）：描述 ≤500 字符
（`:51`）、schema 深度 ≤8、属性 ≤64、结果正文 ≤64 KiB（`:95`）⇒ 第三方原文不再裸奔进模型。
**名册口径已归位**（`crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:121` / `:122`）：
`mcp` 行 = **实际登记进运行时能力表的外部工具件数 ≥ 1**；`mcp_bridge` 行 = 桥面已装配（配置面事实）。

## 4. live 实测（2026-11-05 实跑，非第三方服务器）

探针 = **仓内自建的最小 stdio 探针**（换行分隔 JSON-RPC，只实现 `initialize` / `tools/list` / `tools/call`），
放在仓外临时目录；**不是真实第三方 MCP 服务器**。

| 情形 | 命令 | 实际输出（stderr） |
|---|---|---|
| A 开关开 + 探针可达 | `APEIRETH_ENABLE_MCP=1 APEIRETH_MCP_SERVERS='[{"name":"probe","transport":"stdio","command":"<探针>","enabled":true}]' apeireth session` | `[external-tools] wired: 1 registered, 0 unregistered, 1 live` + `canonical runtime ready`（exit 0） |
| B 开关关 | 同上但**不设** `APEIRETH_ENABLE_MCP` | **无** `[external-tools]` 行 ⇒ 桥面不装配 |
| C 开关开 + 服务器不可达 | 命令指向不存在的可执行文件 | `[external-tools] discovery failed: mcp server "probe" is unreachable: … No such file or directory (os error 2)`，**启动不中断**（`canonical runtime ready` 仍打印，exit 0） |

相关测试（2026-11-05 实跑，全绿）：`cargo test --locked -j 2 -p apeireth-runtime-assembly --test mcp_bridge_runtime_e2e
--test mcp_bridge_refresh --test self_status_roster_families --test capability_switch_chains
-p apeireth-tools-canonical --test mcp_bridge_e2e` ⇒ 4 / 10 / 14 / 11 / 28 套件，0 failed。

## 5. 四级口径落点

| 级 | 成立？ | 依据 |
|---|---|---|
| IMPLEMENTED | 成立（已被上一级覆盖） | 库级可跑、测试通过 |
| **PRODUCTION WIRED** | **成立** | 四环各有生产装配调用点（§3）+ live 实测跑过（§4 情形 A：1 registered 进能力表） |
| DEFAULT ENABLED | **不成立** | 默认关；开关 `APEIRETH_ENABLE_MCP=1`（`crates/adapters/cli/src/lib.rs:186`） |
| HARDWARE VALIDATED | **不成立** | 真服务器端到端未实测 —— live 探针是仓内自建的最小 stdio 探针（§4） |

**claim 建议（只建议，主线裁决）**：claim `tool.mcp_bridge` 的 `level` 落 **PRODUCTION WIRED**（与上表同），
`default` 保持 OFF，`switch_env` 保持 `APEIRETH_ENABLE_MCP`；`notes` 里必须同时写死
**HARDWARE VALIDATED 不成立**与 §6 的 7 条遗留 —— 缺一条就变成「库级已接线」的误读。

## 6. 遗留项清单（7 条，**未销账**，不得写成「全部解决」）

| # | 遗留 | 证据（file:line） | 性质 / owner |
|---|---|---|---|
| ① | **结果面不做内容级凭据识别** —— 远端回吐的凭据**仍会进模型上下文**；现有防线是「有界 + 收敛 + 逐次人批」，不是识别 | `crates/capabilities/tools/src/mcp_bridge/redaction.rs` 的 `clean_text`（`:137`）只清控制字符 / 隐形字符，无凭据内容识别；结果面只做长度收敛（`:95`） | 设计边界（安全线知悉） |
| ② | **`sweep_output_for_egress` 仍未接线到工具输出面** ⇒ **不能**声称「MCP 结果已脱敏」 | 全仓唯一调用点是其自身单测 `crates/capabilities/tools/src/redaction_bridge.rs:971`（位于 `#[cfg(test)]` 模块，模块起点 `:654`）；生产侧零调用点 | 未接线（出境线，非 MCP 域） |
| ③ | **deny 闸的操作面注入口仍缺** ⇒ 主人暂无「事后拉黑单个外部工具」的手段 | `McpToolDenyGuard::deny_capability()` 的生产调用点为 0（仅 `crates/capabilities/tools/tests/mcp_bridge_e2e.rs:212` 与自身单测 `crates/capabilities/tools/src/mcp_bridge/governance.rs:655`） | **在途**：另一条并行施工线正在补（工作树未提交 —— 每服务器 `deny_tools` 配置面 → 装配期 `seed_deny_list` 播种；**未进 HEAD，本记录按在途登记，不计入销账**（四级口径：**PRODUCTION WIRED** 的四环证据不含此面）。owner：安全线 + 并行线 |
| ④ | **名册不独立复核「族授权 + 逐次人批已就位」** —— 那层在组合根的策略里，名册够不着 ⇒ 名册只报可观测代理「已登记 ≥1」 | `crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:121`；策略落点在 `crates/adapters/cli/src/external_tools.rs:74` | 读数边界（运行时线） |
| ⑤ | **工具数 N 本身不在名册**（`CapabilitySwitch` 只有布尔）⇒ 计数面在接线日志（stderr） | `crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:121`；计数日志 `crates/adapters/cli/src/external_tools.rs:79`（`[external-tools] wired: …`） | 读数边界（运行时线） |
| ⑥ | **真服务器端到端未实测** ⇒ HARDWARE VALIDATED 不成立 | §4 探针性质说明（仓内自建最小 stdio 探针 ≠ 真实第三方 MCP 服务器） | 实测缺口（安全线） |
| ⑦ | **周期刷新默认 60s 是程序化配置，没有 env 旋钮**（未登记的开关名不得进代码） | 默认值 `crates/capabilities/tools/src/mcp_bridge/config.rs:49`（`DEFAULT_REFRESH_INTERVAL_MS = 60_000`）+ 落点 `:421`；生产侧只经 `bridge.options().refresh_interval_ms` 读取（`crates/adapters/cli/src/external_tools.rs:124`）；仓内 MCP 侧 env 只有 `APEIRETH_ENABLE_MCP`（`crates/adapters/cli/src/lib.rs:186`）与 `APEIRETH_MCP_SERVERS`（`crates/capabilities/tools/src/mcp_bridge/config.rs:26`） | 设计边界（tools 线） |

**另记（不属遗留，属口径变更）**：桥级闸与生产策略**不同源** —— 生产装配给桥的是桥内自建策略，
本批用桥级族放行（`McpFamilyGrants`）达到等效效果（见
`crates/engine/runtime-assembly/src/canonical/external_tool_bridge.rs` 的「授权的两道闸」段）。

## 7. 验证记录（2026-11-05 实跑）

见主线交付记录；本记录只登记与本条直接相关的三项：MCP 相关 5 个测试套件全绿（§4）、
live 探针三种情形（§4）、四级口径落点（§5）。

## 8. 边界声明（0 装诚实）

- 本记录登记的是**生产装配已接通**，不是「外部工具面已做完」：§6 的 7 条遗留逐条有效。
- §4 的 live 探针**不是**真实第三方 MCP 服务器 ⇒ 不得据此声称 HARDWARE VALIDATED。
- 结果面**没有**内容级凭据识别，§6 ① 有效；**不得**声称「MCP 结果已脱敏」（§6 ②）。
- `tool.mcp_bridge` 的 claim `level` / `verified_at` 由主线裁决，本记录不改 `claims.yaml`。
- 本记录与台账 §四那一行是同一事实的两面：台账给「结论 + 指针」，本记录给「证据 + 遗留」。
