# 能力创造线 · 设计稿（v2.0 上线前「全能能力」目标形态）

[Document-Meta]
Document: docs/01-architecture/capability-creation-line-spec.md
Version: 1.0
Last-Modified: 2026-11-08
Class: SPEC
Status: 🟢 活跃（设计稿）
Author: lead（插件/组件/能力开发线 总指挥）

> **本稿是「目标形态」，不是「做到哪了」**。做到哪了 ⇒ [进度页](../status/capability-creation-line-progress.md)（唯一权威）。
> **授权依据**：[ADR-0025](../adr/0025-capability-creation-line-charter.md)（owner 2026-11-08 裁定：只拓新面 · 允许破棘轮 · 本轮只出设计与排期）。
> **工作线登记**：[docs/status/workline-registry.md](../status/workline-registry.md)。
>
> 📍 **开工前先认领工作线**（`AGENTS.md` §0）：本线 ↔ 本设计稿 ↔ 进度页 ↔ 守卫命令，四件齐了才准开工。

---

## 0. 一句话

> **本线只做「仓内现在没有的面」**：18 个新能力族分 6 个波次推进；
> 每族都有 0 命中取证、治理档、依赖影响与**可机器判**的验收判据；
> **存量接线一律移交**（[ADR-0025 §5](../adr/0025-capability-creation-line-charter.md)）。

---

## 1. 口径与判据

### 1.1 「新面」判据（本线唯一口径）

| 判据 | 内容 |
|---|---|
| ① 新能力 id / 新模型面工具名 | 仓内 `rg` **0 命中** |
| ② 既有族上**新增执行语义** | 新动作 / 新档位 / 新维度 / 新传输 / 新协议面 |
| ③ 新**可查询或可操作对象** | 新快照面 / 新账 / 新协议面 / 新可查询视图 |

⇒ **纯「把已有实现接进生产装配根」≠ 新面** ⇒ 归存量线，本线不碰（§7 移交清单）。

### 1.2 四级口径（承 `system-capabilities.md`，不新造）

`NOT IMPLEMENTED < IMPLEMENTED < PRODUCTION WIRED < HARDWARE VALIDATED`；
`DEFAULT ENABLED` **不是一级**，是横切属性。

**本线的交付口径**：每族**最多到 `PRODUCTION WIRED`**；`HARDWARE VALIDATED` 需真机证据，
本仓现值 **0 条**（51 条 claim 无一达到）⇒ 本线**不承诺**任何 `HARDWARE VALIDATED`。

### 1.3 默认档纪律（不放宽）

* **新增写能力一律默认关**（`default = "off"` + 关时**逐字节零变化**测试）；
* **只读面零审批**；写面逐次人批；**族授权 = 按族一次授权、跨族仍逐次人批**；
* 新 env 名必须登记 `docs/architecture/env-registry.toml`（owner/default/level/purpose/evidence）。

---

## 2. 事实基线（实测 2026-11-08，命令见 §9）

| 指标 | 实测值 | 取证 |
|---|---:|---|
| 生产可达静态能力 id | **54** + 1 动态族（`tool.mcp.<server>.<tool>`） | 独立复算 ≡ 仓内策展表 `TOOL_TAXONOMY` 57−3 |
| **默认开** | **6**（`filesystem`/`search`/`repo`/`repo_map`/`self_status`/`trait_report`） | 其余 **48 条默认关** |
| 行为族 | **45**（32 工具侧 + 13 认知侧） | `runtime-assembly/src/canonical.rs:371-468` |
| 生产**不可达**能力 id | **13**（`tool.memory.*` 7 + `tool.mesh.*` 3 + 3 条库级保留面） | `MemoryModule::new` / `MeshModule::new` 全仓只在 tests 被调 |
| `HARDWARE VALIDATED` | **0** | 51 claim = 38 PRODUCTION WIRED / 12 IMPLEMENTED / 1 NOT IMPLEMENTED |
| 棘轮 | `crates=31/31` · `edges=118/118` · `env=160/160` · `max_src_loc=6255/6300` | `python3 scripts/arch_guard.py --strict` ⇒ `VIOLATIONS: 0` |

---

## 3. 八面能力地图（总表）

> 治理档：**R0** 只读零审批 · **R1** 审计留痕 · **R2** 白名单+逐次人批 · **R3** 硬门+逐次人批 · **R4** 硬门+反思期+多签。
> 依赖影响列：`env` = 新开关名 · `edge` = 新依赖边 · `crate` = 新 crate · `dep` = 新外部包。

| 面 | 族 id | 能力 id（提案） | 一句话 | 波次 | 治理档 | 依赖影响 |
|---|---|---|---|---|---|---|
| **A 感知** | C-A1 | `tool.ui_control_tree` | 只读 UI 控件树（窗口→控件→稳定定位） | W4 | R0 | crate+1 · env+1 · edge+1 · dep+1 |
| **A 感知** | C-A2 | `tool.gui_action` | GUI 动作层（点击/输入/滚动，带预览票） | W4 | R3 | env+1 · edge+1 |
| **B 行动** | C-B1 | `tool.process_tree` | 进程树快照（只读）+ 按树取消 | W1 | R0/R3 | env+1 |
| **B 行动** | C-B2 | `tool.http_write` | 带鉴权的 HTTP 写面（POST/PUT/PATCH，域名白名单） | W1 | R2 | env+1 · edge+1 |
| **B 行动** | C-B3 | `tool.code_exec` | 沙箱代码执行面（受限语言 + 资源封顶 + 每次人批） | W5 | R3 | env+2 · edge+1 · dep 选型待定 |
| **B 行动** | C-B4 | `tool.browser` | 浏览器自动化面（受控导航 + 文本抽取 + 交互动作 + 截图证据） | W4 | R3 | crate+1 · env+2 · edge+2 · dep 选型待定 |
| **C 通联** | C-C1 | `gw.duplex_ws` / `sdk.stream` | 网关 WS 全双工路由 + SDK 流式传输 | W2 | R0/R2 | env+1 · edge+2 · dep+1 |
| **D 记忆** | C-D1 | `mem.object_grant` | 记忆**对象级**只读授权（一条记忆授予一主体，可即时撤销） | W1 | R0/R2 | env+1 |
| **D 记忆** | C-D2 | `rt.checkpoint_resume` | 上下文语义分页 + 检查点恢复（中断后续跑） | W1 | R0 | env+1 · edge+1 |
| **D 记忆** | C-D3 | `rt.cross_device_sync` | 跨设备会话/记忆增量同步 + 加密漫游 | **W6** | R4 | crate+1 · env+2 · edge+2 · dep+1 |
| **E 自进** | C-E1 | `pkg.skill_package` | 技能包格式（manifest+工具声明+权限声明）+ 安装/卸载 | W3 | R3 | crate+1 · env+2 · edge+2 · dep+1 |
| **E 自进** | C-E2 | `pkg.signature` | 包/插件签名验签（Ed25519）+ 未签名拒装 | W3 | R3 | env+1 · edge+1 |
| **F 治理** | C-F1 | `gov.multisig` | Ed25519 真多签（替代占位器） | **W6** | R4 | env+1 · edge+1 · dep+1 |
| **F 治理** | C-F2 | `gov.reflection_period` | R4 反思期生效路径（冷静期 + 回滚窗口） | **W6** | R4 | env+1 |
| **F 治理** | C-F3 | `sec.supply_chain` | 供应链验签（MCP 服务器 / 外部二进制 / 包） | W3 | R2/R3 | env+1 · edge+1 |
| **G 运维** | C-G1 | `tool.ota_update` | OTA 灰度 + 一键回滚（版本轴 + 备份联动） | **W6** | R4 | env+2 · edge+1 |
| **G 运维** | C-G2 | `tool.replay_eval` | 会话回放评估器（回放判定质量，不重放副作用） | **W6** | R0 | env+1 |
| **G 运维** | C-G3 | `obs.timeline` | tracing 时间线（跨模态事件时间线 + 导出） | W5 | R0 | env+1 · edge+1 |
| **G 运维** | C-G4 | `rt.crash_recovery` | 崩溃恢复（会话/任务账一致性恢复） | W5 | R0 | env+1 |
| **H 智囊** | C-H1 | `rt.routing_policy` | 模型路由策略层（任务画像 × 隐私 × 成本 决策表） | W1 | R0 | env+1 |
| **H 智囊** | C-H2 | `quota.depth` | 配额**深度维**（推理深度/思考量）真执行 | W1 | R0 | env+1 |
| **H 智囊** | C-H3 | `tool.mcp_protocol_compliance` | MCP 合规三件套（分页 / 通知消费 / http 合规头） | W1 | R0 | env+1 |
| **H 智囊** | C-H4 | `tool.web_search` | 互联网检索面（配置端点 + 有界收敛 + 来源标注） | W1 | R2 | env+1 |

> **合计 23 族**（W1 = 8 · W2 = 1 · W3 = 3 · W4 = 3 · W5 = 3 · W6 = 5）。
> **额度需求合计**（对 ADR-0025 授权上限）：crate **+4**（≤35）· edge **+12**（≤128）· env **+18**（⚠️ **超授权 +16** ⇒ W3 收口须复评）· dep **+8**。

---

## 4. 族规格（逐族）

### W1 — 0 新外部 dep 档（7 族）

> 共同机制：**落既有 crate 的新文件**（`max_file_loc` 零余量 ⇒ 既有大文件只读不写）；
> 复用既有 `ToolCapability` / `ExtensionDescriptor` / 治理链 / 审批链；**0 新 crate / 0 新依赖边 / 0 新外部包**。

#### C-B1 `tool.process_tree` — 进程树快照与取消

* **语义**：列出受控子进程树快照（pid / 父 pid / 命令 / 状态 / 是否逃逸进程组）；按 pid 取消一棵子进程树（幂等）。
* **机制**：读 `/proc`（Linux）/ `tasklist`（Windows，0 新 dep）· 复用 `ProcessExecutor` + `IsolationCapabilities.process_tree_containment` 档位。
* **落点**：`crates/capabilities/tools/src/process/snapshot.rs`（新文件）。
* **治理**：快照 = R0 零审批；取消 = R3（不可逆）⇒ 逐次人批 + `tool.shell` 族授权。
* **开关**：复用 `APEIRETH_ENABLE_SHELL`（注册即闸 ⇒ shell 关 = 无进程可看，语义自洽）⇒ **0 新 env 名**。
* **验收**：`rg -c 'process_tree_snapshot' crates/` > 0；`cargo test -p apeireth-tools-canonical --test process_executor --locked` 绿；
  快照面零审批断言（无审批请求发生）。

#### C-B2 `tool.http_write` — 带鉴权的 HTTP 写面

* **语义**：对**配置白名单域名**发起 POST/PUT/PATCH/DELETE；请求体/响应体有界；凭据从既有凭据链取（不回显）。
* **机制**：**扩既有 `ControlledEgress` 加写方法**（`get_with_headers` 的同族扩展）⇒ **新工具面不得自带 HTTP 客户端**
  （既有不变量 `fetch_tool_uses_controlled_egress_and_no_second_http_client` 钉死 `fetch.rs` 内 `reqwest::` 为 0 命中
  ⇒ 同理新面必须走受控出网唯一通道）；域名白名单照既有 egress 模型；凭据走既有凭据链（不回显、不进日志）。
* **落点**：`crates/capabilities/tools/src/http_write.rs`（新文件）· **出网闸扩写方法**落 `crates/capabilities/tools/src/egress.rs`
  ⚠️ **该文件在 egress 不变量守卫的扫描面内**（`egress.rs` **不得**出现 `apeireth_runtime`/`apeireth_gateway`/`apeireth_provider`/`FetchTool`/`BrowserTool`/`ToolRegistry`/`GovernancePipeline` 字样）⇒ 扩写方法**只加传输语义，不引入反向依赖**。
* **治理**：R2 ⇒ 域名白名单 + **每次逐次人批** + 族授权；**默认关**。
* **开关**：**1 个新 env 名**（`APEIRETH_ENABLE_HTTP_WRITE`）⇒ 登记 + 默认关 + 零变化测试。
* **验收**：`rg -c 'Method::POST' crates/` 由 0 → >0（现为 **0 命中**）；`cargo test -p apeireth-tools-canonical --test http_write --locked` 绿；
  **新增不变量测试**：新面文件内 `reqwest::` = 0 命中（照既有 fetch 不变量同款）；非白名单域名拒绝即帧；响应体上限生效。

#### C-D1 `mem.object_grant` — 记忆对象级授权

* **语义**：把**一条**记忆（episode/fact）以只读授予某主体；被授方可读不可写；**撤销即时生效**；全程审计。
* **机制**：grant 账（append-only）+ 读路径过滤 + 复用 `governance::permission` 族授权/审批底座；**存在性即开关**（grant 账存在才装）⇒ **0 新 env 名**。
* **落点**：`crates/engine/memory/src/object_grant.rs`（新文件）。
* **治理**：读 = R0；授权/撤销 = R2（跨主体披露）⇒ 逐次人批 + 审计。
* **验收**：`rg -c 'MemoryGrant|object_grant' crates/engine/memory/src/` > 0；撤销后读取**立即**拒绝（测试钉死）；审计行存在。

#### C-D2 `rt.checkpoint_resume` — 检查点恢复

* **语义**：回合/长任务在关键点落**检查点**（上下文语义分页 + 状态快照）；中断后可从检查点续跑，**不重放已生效副作用**。
* **机制**：复用既有 `compaction_checkpoint` 压缩检查点 + `TaskBoard` 任务账；恢复时按幂等键判定是否已生效。
* **落点**：`crates/engine/runtime/src/canonical/checkpoint.rs`（新文件）。
* **治理**：R0（读/恢复本身不改外部世界）；恢复若触发外部动作 ⇒ 走既有审批链。
* **验收**：`rg -c 'restore_from_checkpoint|CheckpointStore' crates/` > 0；「恢复后副作用不重复」测试钉死。

#### C-H1 `rt.routing_policy` — 模型路由策略层

* **语义**：任务画像（能力需求 × 隐私等级 × 成本档）× 候选模型 ⇒ 可解释选路决策（选谁 / 为什么 / 退路是什么）。
* **机制**：复用 `apeireth-core::cost_router`（四档成本分级已实装）**扩一维**（隐私/能力），**不新造第二套路由器**。
* **落点**：`crates/foundation/core/src/routing_policy.rs`（新文件）。
* **治理**：R0（决策是只读计算）；**默认关**（沿 `APEIRETH_COST_ROUTING` 现状）⇒ 复用其开关名，扩 purpose ⇒ **0 新 env 名**。
* **验收**：`rg -c 'RoutingPolicy|routing_policy' crates/foundation/core/src/` > 0；决策**可解释**（必含 `selection_reason` / `fallback_reason`，照既有 cost_router 口径）；
  缺价不编价（无价候选不参与比较）。

#### C-H2 `quota.depth` — 配额深度维

* **语义**：配额四维中的**深度维**（推理深度 / 思考量）从「只有类型」变「真执行」——超限即帧 + 自述真值。
* **机制**：复用 `runtime::canonical::quota` 既有计量 + `self_status` 已留好的深度维自述格（现为「如实标记未接线」）。
* **治理**：R0；拒绝面属既有治理链，**不新增档位**。
* **验收**：`rg -c 'DepthQuota|depth_quota' crates/engine/runtime/src/` > 0；自述面深度维**不再标 null**（测试钉死）。

#### C-H3 `tool.mcp_protocol_compliance` — MCP 合规三件套

* **语义**：① `tools/list` 发 cursor + 消费 `nextCursor` 分页；② 发 `_meta.progressToken` + 声明 `tools.listChanged` + **消费通知帧并刷新目录**（现状：只计数丢弃）；③ http 面发 `Accept: application/json, text/event-stream` + `Mcp-Session-Id` + `MCP-Protocol-Version`。
* **机制**：帧构造器**已存在**（`plugin/src/mcp/schema.rs`；`subscribe.rs:421` 已认 `list_changed` ⇒ `ToolEventKind::ListChanged`）⇒ 桥侧只差「发」与「消费」。
* **落点**：`crates/capabilities/tools/src/mcp_bridge/`（**注意：本文件所在目录有他线在途改动 ⇒ 本族排在在途收口之后**）。
* **治理**：R0（目录刷新）；派发沿 MCP 族既有逐次人批地板。
* **验收**：`rg -c 'nextCursor|progressToken|Mcp-Session-Id' crates/capabilities/tools/src/mcp_bridge/` 由 **0** → >0；
  `cargo test -p apeireth-tools-canonical --test mcp_bridge_e2e --locked` 绿；claims `tool.mcp_bridge` 的「2 类实现缺口硬挡」降为 0 并 bump `verified_at`。

#### C-H4 `tool.web_search` — 互联网检索面

* **语义**：对**配置的检索端点**（自建/自控，如私有 SearXNG 实例）发起查询；结果**有界收敛** + **来源标注** + 未标来源的片段不得进模型上下文。
* **机制**：复用 `reqwest`（锁内既有）+ 既有受控出网通道 + 既有 `UntrustedEnvelope` 封套（第三方内容一律按不可信处理）。
* **落点**：`crates/capabilities/tools/src/web_search.rs`（新文件）。
* **治理**：R2（出网 + 第三方内容）⇒ 端点白名单 + 逐次人批 + 族授权；**默认关**。
* **验收**：`rg -c 'tool\.web_search' crates/` > 0；结果条数/字节**双上限**；来源标注缺失即拒（测试钉死）；默认关时零行为变化。

### W2 — +1 小 dep 档（1 族）

#### C-C1 `gw.duplex_ws` / `sdk.stream` — 全双工路由与流式 SDK

* **语义**：网关暴露 `/v1/ws`，承载既有 8 帧协议（`Auth/Ping/Pong/UserInput/AssistantTextChunk/AssistantAudioChunk/BargeInInterrupt/StreamEnd`）；SDK `invoke_stream` 走真 WS。
* **机制**：协议类型**已在** `foundation/protocol/src/ws_v1.rs`（库级）；网关侧 `duplex_gateway.rs` + `barge_in.rs`（库级）⇒ 缺**路由 + 传输**。
* **依赖**：`axum 0.7.9` 的 `ws` feature ⇒ **+1 新外部包 `tokio-tungstenite`**（现锁内 **0 命中**）；SDK 侧同包。
* **落点**：`crates/adapters/gateway/src/duplex_route.rs`（新文件）+ `crates/adapters/sdk/src/stream.rs`（新文件）。
* **治理**：连接/心跳 = R0；`ToolInvoke` 帧 = R2 逐次人批；**默认关**（沿既有 `voice.duplex` 的 `not_assembled` 口径）。
* **验收**：`rg -c '/v1/ws' crates/adapters/gateway/src/` 由 **0** → >0；`sdk.stream` 走真 WS（回环实测）；网关 `voice.duplex` 不再报 `not_assembled`。

### W3 — 信任面档（3 族）

#### C-E1 `pkg.skill_package` — 技能包格式与安装

* **语义**：最小技能包 = manifest + 工具声明 + **权限声明** + 签名；本地安装/校验/卸载；越权包拒装。
* **机制**：复用 `foundation/plugin` 的 `PluginManifest` / `ExtensionDescriptor` / `PluginManager` 生命周期（ADR-0008 单 ABI 家族）。
* **落点**：新 crate `crates/foundation/trust/`（`apeireth-trust`）——签名验签与包格式的依赖闭包**必须隔离**（避免污染主 lock）。
* **治理**：安装 = R3（引入可执行第三方代码）⇒ **硬门 + 逐次人批**；**权限声明逐条过治理**，包声明不得扩大宿主授权。
* **验收**：`rg -c 'SkillManifest|skill_manifest' crates/` 由 0 → >0；越权包拒装测试钉死；卸载后能力表**同步移除**（不留残影）。

#### C-E2 `pkg.signature` — 包/插件签名验签

* **语义**：Ed25519 签名验签 + 信任根管理；**未签名 / 验签失败 = 拒装**（不降级为「提示后放行」）。
* **依赖**：**+1 新外部包 `ed25519-dalek`**（现锁内 **0 命中**；仓内 `id_ed25519` 命中全为凭据形状检测，非验签）。
* **治理**：R3。
* **验收**：`rg -c 'ed25519::|Ed25519Signature' crates/` 由 0 → >0；篡改包**必拒**（测试钉死）；失败路径**无降级分支**。

#### C-F3 `sec.supply_chain` — 供应链验签

* **语义**：MCP 服务器清单 / 外部二进制清单（`tool.external.*` 依赖的系统二进制）可带签名与摘要；未登记者按既有治理档处理。
* **落点**：`crates/foundation/trust/`（与 C-E2 同 crate）。
* **治理**：R2/R3。
* **验收**：篡改的外部二进制**不被派发**；清单登记面可查。

### W4 — 感知/行动档（2 族）

#### C-A1 `tool.ui_control_tree` — 只读 UI 控件树

* **语义**：窗口 → 控件树 → 稳定定位（含去重/回退）；**只读不执行**。
* **依赖**：平台异构三套（Windows UIA COM / macOS AX / Linux AT-SPI）⇒ **新 crate**（`apeireth-automation`）+ 新外部包；
  **Windows 先行、macOS/Linux 显式标「未实现」**（0 装诚实，不假装三平台齐）。
* **治理**：R0 零审批。
* **验收**：Windows 真机跑通一条控件树用例；**Linux 环境跑同一测试必须显式标未实现**（不得静默通过）。

#### C-A2 `tool.gui_action` — GUI 动作层

* **语义**：点击/输入/滚动，**必须带预览票**（预览帧 → 逐次人批 → 执行）；动作目标来自 C-A1 的稳定定位。
* **治理**：R3（不可逆/可见副作用）⇒ **硬门 + 逐次人批**；预览票一次性、载荷被改拒绝即帧（照 `device_control` 三拍先例）。
* **验收**：无预览票的动作**必拒**；动作计数进自述节（照既有创作节口径）。

#### C-B4 `tool.browser` — 浏览器自动化面

* **语义**：**最小版本** = 受控导航（域名白名单）+ 正文文本抽取 + 交互动作（点击/输入）+ **截图证据**（每次动作留证据帧）；**不承诺浏览器全等价**（不做多标签/扩展/下载管理）。
* **机制（引擎选型待定，W4 前二选一）**：① 内嵌无头内核（**新外部包 + 大闭包**，D19 教训：须隔离 crate + 钉版本）；② 驱动**宿主已装**浏览器（走其调试协议，**0 新包**但依赖宿主装浏览器 ⇒ 部署面变复杂）。
* **落点**：新 crate `crates/capabilities/web/`（与端侧推理同属「大闭包隔离」范式）⇒ 与 C-B3 共享 crate 名不合适 ⇒ **C-B3 落 `crates/capabilities/tools/src/code_exec.rs`**（复用既有进程隔离面），**C-B4 落新 crate**。
* **治理**：导航/抽取 = R2（出网 + 第三方内容）⇒ 白名单 + 逐次人批；**交互动作 = R3**（可见副作用）⇒ **预览票 + 逐次人批**；第三方内容一律走既有 `UntrustedEnvelope` 封套。
* **验收**：域名白名单外**必拒**；抽取结果**有界**且带来源；动作必带证据帧；**默认关时零行为变化**。

### W5 — 大闭包/运维档（3 族）

#### C-B3 `tool.code_exec` — 沙箱代码执行面

* **语义**：受限语言的代码执行（默认无网、工作区限域、CPU/内存/时长三重封顶），**每次逐次人批**；输出有界。
* **机制（**选型待定，两案在 W5 前必须二选一**）**：① 子进程解释器（复用既有 `ProcessExecutor` 隔离面，0 新包，但依赖宿主装解释器）；
  ② 进程内绑定（锁内 `pyo3 0.29.3` 已在树内，但**进程内跑不可信代码 = 越权面** ⇒ 须另起隔离进程）。
* **治理**：R3。
* **验收**：网断/文件越界/超时限/超内存**四种逃逸各有拒绝用例**；输出封顶生效。

#### C-G3 `obs.timeline` — tracing 时间线

* **语义**：跨模态事件时间线（回合/工具/审批/记忆写入）+ 导出；**只读**。
* **机制**：复用既有 `ToolEventV1` + `RuntimeEventSink` + `NervousBus` 事件族（**不新造事件总线**）。
* **验收**：`rg -c 'tracing_timeline|TimelineSpan' crates/` 由 0 → >0；导出可复现（同一回合同一时间线两次导出逐字节等）。

#### C-G4 `rt.crash_recovery` — 崩溃恢复

* **语义**：崩溃后**任务账/会话账/运行中状态**的一致性恢复（running→pending + 幂等单执行）。
* **机制**：复用既有 `TaskQueue` 崩溃恢复语义（V6-a 已有）扩到会话层。
* **验收**：崩溃注入（子进程 kill）后重启 ⇒ 账本一致 + 无重复执行（测试钉死）。

### W6 — 需 owner 逐项重新拍板档（6 族）

> ⚠️ **这 6 族不是「已排期」**，是「**候选**」——其中多签 / 漫游 / 全流程 OTA 属
> 2026-10-26 D2 已裁「T0 不做」的产品范围，**本线不擅自推翻** ⇒ 逐项重拍后才进排期。

| 族 | 能力 id | 为什么需重拍 | 前置 |
|---|---|---|---|
| C-D3 | `rt.cross_device_sync` | 阶梯 T1（Noise 漫游）+ 跨主体披露（R4） | W5 |
| C-F1 | `gov.multisig` | D2 已裁「物理多签 v2.1 不做」；占位器 `NotConfigured` 是**正确的 0 装** | W5 |
| C-F2 | `gov.reflection_period` | owner 2026-10-26 已裁「反思期属阶梯未到」⇒ 装配它 = 改产品范围 | W5 |
| C-G1 | `tool.ota_update` | R4 系统级控制面；「T0 轻量版」（主人同意 + 自动备份 + 一键回滚）需 owner 明确授权 | W5 |
| C-G2 | `tool.replay_eval` | 回放语义须先定义「哪些副作用不重放」 | W5 |
| C-B3（依赖侧） | — | C-B3 本身在 W5；此处仅提示其「解释器选型」需 owner 认可 | — |

---

## 5. 批次与依赖

| 波 | 族 | 依赖前波 | 破棘轮净增（crate/edge/env/dep） | 共同收口 |
|---|---|---|---|---|
| **W0** | ADR + 本设计稿 + 进度页 + 登记表 | — | 0 / 0 / 0 / 0 | doc class + links + index + docs_truth 全绿 |
| **W1** | C-B1 · C-B2 · C-D1 · C-D2 · C-H1 · C-H2 · C-H3 · C-H4（**8 族**） | W0 | 0 / 0 / +3 / 0 | `arch_guard` 0 违规 · 全量门禁绿 · claims 同步 |
| **W2** | C-C1 | W1 | 0 / +2 / +1 / +1 | 同上 |
| **W3** | C-E1 · C-E2 · C-F3 | W2 | +1 / +4 / +4 / +1 | 同上 |
| **W4** | C-A1 · C-A2 · C-B4（**3 族**） | W3 | +1~2 / +4 / +3 / +1~2 | 同上 |
| **W5** | C-B3 · C-G3 · C-G4 | W4 | 0~1 / +3 / +3 / 选型定 | 同上 |
| **W6** | C-D3 · C-F1 · C-F2 · C-G1 · C-G2（**5 族**） | W5 + **owner 逐项重拍** | +1 / +4 / +5 / +2 | 同上 |

> **额度对账**：合计 crate **+3~4**（≤ 授权 +4）· edge **+15**（⚠️ **超授权 +10** ⇒ **W5/W6 需按实际重算额度**，
> 或在 W3 收口时**重评授权额度**）· env **+18**（⚠️ **超授权 +16** ⇒ 同上）· dep **+5~7**。
> ⇒ ⇒ **W3 收口时必须回填契约并复评额度**（[ADR-0025 §4.3](../adr/0025-capability-creation-line-charter.md)）。

---

## 6. 验收判据（**每波机器可判**）

```bash
# ① 棘轮（额度 + 违规）
python3 scripts/arch_guard.py --strict     # ⇒ VIOLATIONS: 0；crates ≤35 / edges ≤128 / env ≤176 / max_src_loc ≤6300
# ② 声明真源
python3 scripts/docs_truth.py --verify --check   # ⇒ claims=N fail=0 stale=0
# ③ 文档纪律
python3 scripts/check_doc_class.py --check   # ⇒ 0 违规
python3 scripts/docs_links.py --check        # ⇒ 断链 0
python3 scripts/docs_index.py --check        # ⇒ 问题数 0
# ④ 工程门禁
cargo test --workspace --all-targets --locked --no-fail-fast -j 2          # ⇒ 0 failed
cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings        # ⇒ 0 警告
cargo fmt --all -- --check                                                 # ⇒ 0 diff
# ⑤ 中性表述 + secret
pwsh scripts/check-neutral-terms.ps1          # ⇒ 0 命中
pwsh scripts/secret-scan.ps1 -Mode scan-all   # ⇒ No secrets found
```

**族级判据**：每族的三条 0 命中取证命令（§4 各族「验收」行）**必须由 0 变 >0**，且**每族至少 1 个对抗测试**
（越权/篡改/越界/超限 **必拒**）。

---

## 7. 不做清单（**明确排除**，防后人重复讨论）

| 不做 | 理由 |
|---|---|
| 存量接线（记忆 P0 列封缄 / 网关成本路由 / 网关子代理端点 / R4 反思期装配） | owner 裁定「不动存量」⇒ 移交（[ADR-0025 §5](../adr/0025-capability-creation-line-charter.md)） |
| 改 LOCKED 5 项（9 哲学锚本体 / 14 键 PHL-01..08 / 3 不可变脊柱 / `workspace.version` / R11 baseline） | LOCKED；workspace.version 仅 release authority 可改 |
| 接回 13/14 键 runtime 强制 | 2026-10-11 PHL-08 前后已拍板降级为哲学标准 |
| 改既有大文件（`doc_ops.rs` 6255 等） | `max_file_loc` 零余量 ⇒ 新面一律新开文件（< 800 行） |
| 宣称任何 `HARDWARE VALIDATED` | 本仓现值 0 条；本线最多到 `PRODUCTION WIRED` |
| 替他人线改 `production.rs` / `env-registry.toml` / `mcp_bridge` 在途面 | 他线在途（owner 在盯）⇒ 本线写域回避 |
| 顺手重构 / 扩大范围 | 施工纪律 §六.3「顺手修三判」 |

---

## 8. 写域与战场纪律

### 8.0 既有架构不变量（**实现前必读**，会当场拦人）

`crates/capabilities/tools/tests/architecture_invariants.rs`（168 行，源级守卫）已钉死四条：

| 不变量 | 钉死什么 | 对本线新面的约束 |
|---|---|---|
| `fetch_tool_uses_controlled_egress_and_no_second_http_client` | `src/fetch.rs` 内 `reqwest::` / `ProcessExecutor` / `is_ssrf` 等 **0 命中**，且必须走 `ControlledEgress::get_with_headers` | **C-B2** 不得自带 HTTP 客户端 ⇒ 必须**扩 `ControlledEgress` 加写方法**（受控出网唯一通道） |
| `egress_infrastructure_does_not_depend_on_runtime_gateway_or_provider` | `src/egress.rs` 不得出现 `apeireth_runtime`/`apeireth_gateway`/`apeireth_provider`/`FetchTool`/`BrowserTool`/`ToolRegistry`/`GovernancePipeline` | **C-B2** 扩 egress 时**只加传输语义、不引入反向依赖** |
| `public_contract_does_not_name_platform_sandbox_mechanisms` | `process/mod.rs` 公共契约面（去注释后）不得出现 `JobObject`/`cgroup`/`seccomp`/`Landlock`/`namespace`/`setrlimit`/`Seatbelt`/`sandbox-exec`/`CreateProcessW`/`RestrictedToken` | **C-B1**（进程树快照）**C-A1**（平台 UIA）**C-B4**（浏览器）⇒ **平台机制名只落 backend 模块**，公共契约只写平台无关的能力档位 |
| `process_infrastructure_does_not_create_a_shell_backdoor` | `process/**` 不得出现 `cmd /c`/`sh -c`/`shell_words`/`command: String` | 新进程面**不得**接受自由命令串 ⇒ 一律结构化参数 |

> ⇒ **这四条是新面的「隐性契约」**：设计里没写，实现批就会当场撞红 ⇒ 每批开工前先读该文件。

### 8.1 写域
| 波 | 允许写域（**建议**） | 禁碰 |
|---|---|---|
| W1 | `crates/capabilities/tools/src/{process/snapshot.rs,http_write.rs,web_search.rs}` · `crates/engine/memory/src/object_grant.rs` · `crates/engine/runtime/src/canonical/checkpoint.rs` · `crates/foundation/core/src/routing_policy.rs` · `crates/engine/runtime/src/canonical/quota.rs` · `crates/capabilities/tools/src/mcp_bridge/`（**待他线收口**） | `production.rs` / `env-registry.toml` / `mcp_bridge`（在途段）· 既有大文件 |
| W2 | `crates/adapters/gateway/src/duplex_route.rs` · `crates/adapters/sdk/src/stream.rs` · 对应 `Cargo.toml` | 同上 |
| W3 | 新 crate `crates/foundation/trust/**` | 同上 |
| W4 | 新 crate `crates/capabilities/automation/**` | 同上 |
| W5 | `crates/engine/runtime/src/canonical/` 新文件 · `crates/capabilities/tools/src/code_exec.rs` | 同上 |
| W6 | 逐项 ADR 后再定 | 同上 |

**并发纪律**：同文件区施工必须排队；跨范围顺手修 = 报备 + 最小改动 + 说明；
**0 冲突 ≠ 合并正确**（施工纪律 §二.5）⇒ 合并后逐文件 diff 复核。

---

## 9. 复核命令（族级 0 命中取证速查）

```bash
# 感知/行动
rg -n 'UiAutomation|ui_automation|element_tree|ControlTree' crates/ --glob '*.rs'   # 0
rg -n 'ClickAction|click_at|mouse_click|GuiAction' crates/ --glob '*.rs'                # 0
rg -n 'ProcessTreeSnapshot|process_tree_snapshot|snapshot_process_tree' crates/ --glob '*.rs'  # 0
rg -n 'playwright|puppeteer|chromium|headless_browser|selenium' crates/ --glob '*.rs'   # 仅 1 处第三方借鉴注释
rg -n 'Method::POST|Method::PUT' crates/ --glob '*.rs'                                  # 0
# 通联/记忆/自进/治理/运维/智囊
rg -n '/v1/ws' crates/adapters/gateway/src/                                              # 0
rg -n 'tokio-tungstenite' crates/ --glob '*.rs'                                         # 0
rg -n 'MemoryGrant|memory_grant|episode_grant|GrantScope' crates/ --glob '*.rs'        # 0
rg -n 'restore_from_checkpoint|CheckpointStore' crates/ --glob '*.rs'                   # 0
rg -n 'SkillManifest|skill_manifest' crates/ --glob '*.rs'                             # 0
rg -n 'ed25519::|Ed25519Signature|verify_multisig' crates/ --glob '*.rs'               # 0
rg -n 'ota_update|OtaUpdate|canary_release|gradual_rollout' crates/ --glob '*.rs'      # 0
rg -n 'replay_evaluator|ReplayScorer' crates/ --glob '*.rs'                            # 0
rg -n 'tracing_timeline|TimelineSpan|timeline_span' crates/ --glob '*.rs'               # 0
rg -n 'RoutingPolicy|TaskProfile|routing_policy|task_profile' crates/foundation/core/src/ crates/engine/provider/src/  # 0
rg -n 'DepthQuota|depth_quota' crates/ --glob '*.rs'                                    # 0
rg -n 'nextCursor|progressToken|Mcp-Session-Id' crates/capabilities/tools/src/mcp_bridge/  # 0
rg -n 'brave|duckduckgo|searx|bing\.com' crates/ --glob '*.rs'                          # 0
```

> ⚠️ **落地前必重跑**：在途线正在改 `mcp_bridge/**` ⇒ C-H3 的三条命令**必须在其收口后重跑**再开工。

---

## 10. 风险台账

| # | 风险 | 触发 | 处置 |
|---|---|---|---|
| R1 | 在途线与本线抢 `production.rs` / `env-registry.toml` | W1 注册新面时 | 写域回避 + 排队；注册面留给在途线收口后再接线 |
| R2 | 新外部包触发 `deny.toml multiple-versions = "deny"` | W2/W3/W4 | 引入前跑 `cargo deny check`；冲突则隔离新 crate + 钉版本 |
| R3 | 额度（edge/env）估算超授权（§5 已预警 **edge +15 / env +18**） | W3 收口 | 按实际重算；不够则**追加 ADR**（owner 拍板），不擅自抬 |
| R4 | 新面绕过治理链（造出越权面） | 任一写面 | 新面一律复用既有 `ToolCapability` + 治理链 + 审批链；对抗测试钉死 |
| R5 | 第三方内容进模型上下文（未标注来源） | C-H4 / C-C1 | 一律走既有 `UntrustedEnvelope` 封套；来源缺失即拒 |
| R6 | Windows-only 能力在 Linux CI 假装通过 | C-A1 | 未实现平台**显式标未实现**，不得静默绿 |
| R7 | 中性表述违规（第三方产品名进入注释） | C-A1/C-B2 借鉴既有实现时 | 新写代码零第三方产品名；既有违规文件**只在自己写域内**顺手清（他域报备） |
