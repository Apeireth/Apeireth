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

### 1.1b 重复判定三档（**2026-11-08 重复审计定案**；审计底稿见进度页 §3）

| 判定 | 定义 | 处置 |
|---|---|---|
| **真新面** | 仓内**无任何**实现该族 job 的面（库级实现 / 脚本 / CI 层实现**都算**已有面） | 本线开工 |
| **部分重复（需补强）** | 已有面实现了该族 job 的**可识别子集** ⇒ 必须写清「**已有 X（file:line）· 缺 Y**」 | 本线开工，但**只补 Y**，**不重建 X**；族规格必须带「已有先例」栏 |
| **纯重复（应移交）** | 已有面与该族**等价**（同 job + 同目标域 + 同治理档）⇒ 实为接线/搬运 | **移交存量线**，本线不做 |

> ⚠️ **按名 0 命中 ≠ 无重复**（审计实测：23 族**全部**存在语义先例，**真新面 = 0**）
> ⇒ ⇒ **「符号 0 命中」只能证明「没造过同名物」，不能证明「没人做过这件事」**。

### 1.1c 能力 id 命名空间（**生产惯例，权威源 `foundation/plugin/src/capability.rs:25` `CapabilityKind`**）

| Kind | 保留前缀 | 语义 | 例 |
|---|---|---|---|
| Tool | `tool.` | 模型可调用 | `tool.shell` |
| Provider | `provider.` | 补全服务 | `provider.anthropic` |
| Memory | `memory.` | 持久召回 | `memory.sqlite` |
| Transport | `transport.` | 外部进程/对端通道 | `transport.mcp` |
| Observer | `observer.` | 观测运行时 | `observer.tracing` |
| Scheduler | `scheduler.` | 决定何时跑 | `scheduler.cron` |
| Extension | `extension.` | 逃生舱（**不是垃圾桶**） | — |

⇒ **纪律**：新族的能力 id **必须**落在上述保留前缀内；**不是模型面能力**的（治理面 / 观测面 / 运行时机制面）
⇒ **不新设能力 id**，只标**面名**（照 `module.mcp` 的先例）。

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

## 3. 能力族总表（**重复判定后** · 2026-11-08 重复审计定案）

> **重复判定口径**：见 §1.1b。**真新面 = 0**（23 族全部与既有面部分重叠）⇒
> 本表已把 **14 条新面条目**（覆盖原 18 族，含 3 处合并）与 **5 条移交条目**分开列。
> **能力 id 已按 §1.1c 命名空间校正**，且**避开既有占名**（SDK 契约 `client.rs:76/79`、`ws_v1.rs:141`）。

### 3.1 池 A —— 本线新面（**只补缺口，不重建已有面**）

| 族 | 能力 id（校正后） | 重复判定 | **已有先例**（file:line） | **本族真正新增的那一面** | 波次 | 治理档 | 依赖影响 |
|---|---|---|---|---|---|---|---|
| **C-A1**（含 C-A2） | `tool.external_gui`（含 C-A2） | 部分重复 | `fetch/accessibility.rs:96,205`（HTML **无障碍树**，生产接线）· `gateway/im_bridge.rs:626,684`（GUI 动作链，**无生产构造点**） | **原生 OS 应用**的控件树取数（UIA/AX/AT-SPI）+ **跨应用合成输入**；读面产 **locator** ⇒ 动作面消费 **同一 locator 模型** | W4 | R0/R3 | crate+1 · env+2 · edge+2 · dep+1~2 |
| **C-B1** | `tool.process_tree` | 部分重复 | `process/mod.rs:101` `IsolationCapability::ProcessTreeContainment`（杀整树档位）· `std_sub_supervisor.rs:167,180`（真 `child.kill()`） | **快照枚举**（pid/父/命令/状态/是否逃逸进程组）+ **按 pid 幂等取消一棵树** | W1 | R0/R3 | env+1 |
| **C-B2** | `tool.http_write` | 部分重复（两半未合流） | `egress.rs:515,523`（`ControlledEgress` **只有 GET**）· `sdk/client.rs:752`（真 POST + Bearer） | 把「**写方法 + 鉴权头 + 受控出网闸**」并成一条**产品**工具面（egress 长出 POST） | W1 | R2 | env+1 · edge+1 |
| **C-B3**（收窄） | `tool.script_exec`（**弃用 `code_exec`**：SDK 契约已占名） | 部分重复（半边移交） | `shell.rs:195` `tool.shell`（**已接线**的沙箱命令执行面：隔离档位+审批+对象判定+输出边界） | 只保留「**语言代码在沙箱内解释执行**」那一面；**「跑命令」整半边移交 `tool.shell` 线** | W5 | R3 | env+2 · edge+1 · dep 选型待定 |
| **C-B4** | `tool.web_browser`（**避开 `BrowserTool`**：它在 `architecture_invariants.rs:101` 禁名单里） | 部分重复（薄） | `stealth_crawler.rs:22,83,201`（反检测配置 + UA 选取 + 抽取，**零 IO 未接线**）· `fetch.rs:210`（单 URL 受控 GET） | **真浏览器会话驱动**（导航/点击/表单/JS/多步会话）；`stealth_crawler` 先定性再复用 | W4 | R2/R3 | crate+1 · env+2 · edge+2 · dep 选型待定 |
| **C-D1** | `tool.object_grant` | 部分重复 | `governance/permission.rs:135,177` + `gateway/panels.rs:782`（能力/族级授权 + `/v1/panel/grants`）· `memory/scope.rs:17` `MemoryScope`（**域级**可见性） | **按记忆对象 id 授权**（一条 episode/fact 只读授予某主体）+ 读路径过滤 + **撤销即时生效**（复用 `panel/grants/revoke` 的热撤销语义） | W1 | R0/R2 | env+1 |
| **C-D2**（含 C-G4 + C-G3） | 面名 `session_observability`（**不新设能力 id**：非模型面） | 部分重复 | `memory/agent_trace.rs:184,263`（span 树 + 落库，未接线）· `runtime/trace.rs:164,173`（turn 内）· `runtime-assembly/startup.rs:118`（**生产接线**）· `orchestration/durable/replay.rs:219,351`（真崩溃恢复）· `runtime/session_events.rs:188-194`（崩溃**可检测** marker） | **会话层快照+恢复契约 + 会话层恢复执行器 + 跨源统一时间线视图**（三块拼图并成**一套事件面 + 一套 span 面**，不落三次 schema） | W5 | R0 | env+1 · edge+1 |
| **C-D3** | `transport.cross_device` | 部分重复（薄） | `memory/research_roaming_memory.rs:95,160,180`（副本**合并 + 墓碑**原语，research 专用未接线） | 同步游标 + 冲突策略 + 传输 + 跨设备身份/加密漫游；**先提取 merge/tombstone 为通用原语** | W6 | R4 | crate+1 · env+2 · edge+2 · dep+1 |
| **C-E1**（含 C-E2 插件那半边） | `tool.skill_package` | 部分重复 | `tools/skill/library.rs:163,194,256,283` `SkillLibrary`（单卡 + 修订归档 + 库扫描）· `plugin/manager.rs:40` + `manifest.rs:17`（插件 manifest + 生命周期） | 「**包**」这一层：manifest + 工具声明 + **权限声明** + **可签名可校验** + 安装/卸载/版本固定（**两动作同族**，避免先出无签名格式再改 schema） | W3 | R3 | crate+1 · env+2 · edge+2 · dep+1 |
| **C-F1** | 面名 `multisig_verifier`（治理面，**不新设能力 id**） | 部分重复（薄） | `core/onion.rs:94,185` `verify_multisig`（M-of-N 计数 + offline 熔断，**签名体是自造字符串格式**）· `governance/chain.rs:201,214` `MultiSignVerifier` + `PlaceholderMultiSignVerifier`（全拒 fail-closed） | **密码学验签原语**（Ed25519）+ 一个**真** `MultiSignVerifier` 实现（替换占位器） | W6 | R4 | env+1 · edge+1 · dep+1 |
| **C-G2** | `tool.replay_eval` | 部分重复（中） | `governance/eval.rs:13,137,147,201` `EvalScore`/`TaskSummary`/`summarize`/`to_eval_scores` · `memory/d4_eval.rs:293` `score()` · `runtime/event_log.rs:227` `fold_surface` | 「**回放一段会话 → 产出评估结论**」这条链（把 replay + event log/trace 接进**既有打分器**） | W5 | R0 | env+1 |
| **C-H1** | 面名 `routing_policy`（**不新设能力 id**：选路不是模型面能力） | 部分重复 | `core/cost_router.rs:508,602,373` `CostRouter`/`candidates`（四档成本 × 模型档案矩阵）· `runtime/provider.rs:95,136` `ProviderRouter`（fallback + 健康度） | **统一策略层**（任务画像 × 隐私 × 成本）+ **网关侧接入**（gateway 对 `CostRouter` 零引用）⇒ **与既有两层合成一层**，不造第三套路由语义 | W1 | R0 | env+1 |
| **C-H2** | **不新设能力 id**（沿用既有自述面路径 `budget.quota.depth`） | 部分重复（占位完备） | `runtime/quota.rs:47` `QuotaDimension`（三维真计量）+ `:20-21` 明写「depth 无调用树跟踪」· `tools/self_status.rs:320-322,2262,3428`（深度维**诚实占位 + 自述格 + 测试钉死**） | **深度维真计量**（调用树深度跟踪）—— **接口位/自述格/测试位都已留好** ⇒ 成本最低 | W1 | R0 | env+1 |
| **C-H5** | `tool.calendar` | **真新面** | **无任何先例**（⚠️ 先前「calendar 25 命中」全是 `canonical` 的**子串误判**；仓内**零**日历实现）；`tool.care_schedule` 是**代理自身的关怀例程**（`crates/engine/organ/src/care/tool.rs:40`，add/remove/list），**不是主人日程** ⇒ 边界不同 | 主人**日程**面（本地日程账 + 查询/创建/改期/取消 + 逐次人批）；**先本地、零外部集成** | W1 | R2 | env+1 · edge+1 |
| **C-H4** | `tool.web_search`（⚠️ **SDK 契约已占同名** `client.rs:76` ⇒ 须先对齐口径） | 部分重复（薄） | `sdk/client.rs:76,226,656`（SDK 契约已占名 + `/v1/tools/web_search/invoke` + 真 HTTP）· `tools/fetch.rs:210`（受控 GET）· `tools/search.rs:67` `SearchTool`（**本地**字面搜索，不用网络） | **检索引擎后端 + 排序 + 引用标注**（仓内既无搜索引擎也无检索 API） | W1 | R2 | env+1 |

### 3.2 池 B —— **移交存量线**（本线不做；owner 裁「只拓新面」）

| 族 | 原拟能力 id | 判定 | 已有面（等价或已覆盖） | 移交去向 |
|---|---|---|---|---|
| **C-C1** | `gw.duplex_ws` / `sdk.stream` | **纯接线（整族移交）** | `protocol/ws_v1.rs:63` `WsFrame`（8 帧 + `validate`，**LOCKED 集成点**）· `gateway/duplex_gateway.rs:104,131` + `barge_in.rs:101,194,258`（双工/打断控制器，**未挂路由**） | 存量线（网关 `/v1/ws` 路由 + SDK `invoke_stream` 传输各归其原线） |
| **C-F2** | `gov.reflection_period` | **纯接线（整族移交）** | `governance/chain.rs:370` `DelayedReflectionScheduler`（**真实现**）+ `:99` `GovernancePhase::Reflection` 编排位（**生产未注入**） | 治理线（owner 2026-10-26 已裁「反思期属阶梯未到」） |
| **C-H3** | `tool.mcp_protocol_compliance` | **纯接线（整族移交）** | `plugin/mcp/subscribe.rs:421,473`（`list_changed` → `ToolEventKind::ListChanged` + 通知帧构造器）· `schema.rs:37` + `lifecycle.rs:86-112` · `mcp_bridge/catalog.rs:193` `replace_server` | MCP 线（**落点在他线在途段**；`transport.rs:961` 现为单值 `Accept`） |
| **C-E2 半边** | （发布包签名） | **整半边移交** | `scripts/release/cosign-sign-all.sh` + `cosign-verify.sh` + `.github/workflows/cosign.yml` + `docs/security/cosign-keys.md`（8 包签名+验签+公钥发布/撤销） | CI / 发布线（本线只保留「**插件/技能包运行时验签**」那一面，已并入 C-E1） |
| **C-F3 半边** | `sec.supply_chain`（依赖 + 自家产物两段） | **整半边移交** | `cargo-audit.yml` + `cargo-deny.yml` + `deny.toml` + `cosign.yml` + `dependabot-upgrade.yml` | CI 线（本线只保留「**第三方插件/MCP 服务器的可验证来源**」那一面） |
| **C-G1 半边** | `tool.ota_update`（升级 + 回滚） | **整半边移交** | `scripts/upgrade/*.sh`（备份+停服+换装+起服+健康检查+回滚，`--dry-run`）· `runtime-assembly/upgrade_cycle.rs:158` `UpgradeCycle` L0-L5 | 升级线（本线只保留「**灰度放量 + 运行期 OTA 面**」） |
| **C-B3 半边** | （跑命令） | **整半边移交** | `tools/shell.rs:195` `tool.shell`（已生产接线） | `tool.shell` 存量线（本线只保留「语言代码解释执行」） |

> ⇒ **池 A = 14 条新面条目**（覆盖原 18 族：3 处合并）｜**池 B = 7 条移交条目**（覆盖原 5 族 + 3 个半边）。

### 3.3 额度重算（**合并后**）

| 棘轮 | 现值 | 授权上限 | 合并后估算 | 结论 |
|---|---:|---:|---:|---|
| `max_workspace_crates` | 31 | 35 | **+3**（automation / web / trust） | ✅ 在额度内 |
| `max_production_edges` | 118 | 128 | **+11** | ⚠️ 接近上限 ⇒ W3 收口复评 |
| `max_distinct_names`（env） | 160 | 176 | **+16** | ⚠️ 接近上限 ⇒ W3 收口复评 |
| `max_file_loc` | 6300 | 6300（不动） | ±0 | 新面一律新开文件（< 800 行） |

## 4. 族规格（逐族 · 池 A 14 条 + C-L1 新立项）

> **每族的「已有先例」见 §3.1**（重复判定依据，file:line 可复核）；本节只写**新增面的语义与验收**。
> **共同纪律**：落既有 crate 的新文件（`max_file_loc` 零余量）· 复用既有 `ToolCapability` / 治理链 / 审批链 ·
> **新增写能力默认关** + 关时逐字节零变化 + env 登记 · 每族至少 1 个对抗测试。

### 4.0 池 A 速览

| 族 | 能力 id / 面名 | 波次 | 治理档 | 开关口径（env 名 ⇒ 新增数） | 落点 | 验收判据（可机器判） |
|---|---|---|---|---|---|---|
| C-A1 | `tool.external_gui`（含 C-A2） | W4 | R0/R3 | **新名 2**：`APEIRETH_ENABLE_GUI_READ` + `APEIRETH_ENABLE_GUI_ACTION` | 新 crate `crates/capabilities/automation/` | `rg -c 'native_control_tree\ |
| C-B1 | `tool.process_tree` | W1 | R0/R3 | **复用** `APEIRETH_ENABLE_SHELL`（注册即闸 ⇒ shell 关 = 无进程可看，语义自洽）⇒ **0 新名** | `crates/capabilities/tools/src/process/snapshot.rs` | `rg -c 'process_tree_snapshot' crates/` > 0；快照面**零审批**断言；平台机制名不进公共契约 |
| C-B2 | `tool.http_write` | W1 | R2 | **新名 1**：`APEIRETH_ENABLE_HTTP_WRITE`（默认关 + 零变化测试 + 登记） | `crates/capabilities/tools/src/http_write.rs` + 扩 `crates/capabilities/tools/src/egress.rs` | `rg -c 'Method::POST' crates/` > 0；**新面文件内 `reqwest::` = 0 命中**（照既有 fetch 不变量）；白名单外必拒；响应体上限生效 |
| C-B3 | `tool.script_exec`（跑命令半边移交 `tool.shell`） | W5 | R3 | **新名 2**：`APEIRETH_ENABLE_SCRIPT_EXEC` + `APEIRETH_SCRIPT_RUNTIME` | `crates/capabilities/tools/src/script_exec.rs` | `rg -c 'script_exec' crates/` > 0（**且 `code_exec` 不得出现在本族**）；四类逃逸（断网/越界/超时/超内存）各有拒绝用例 |
| C-B4 | `tool.web_browser` | W4 | R2/R3 | **新名 1**：`APEIRETH_ENABLE_WEB_BROWSER` | 新 crate `crates/capabilities/web/` | `rg -c 'web_browser' crates/` > 0；域名白名单外必拒；动作必带证据帧；**不得出现 `BrowserTool` 字样**（守卫禁名单） |
| C-D1 | `tool.object_grant` | W1 | R0/R2 | **存在性即开关**（grant 账存在才装）⇒ **0 新名** | `crates/engine/memory/src/object_grant.rs` | `rg -c 'object_grant' crates/engine/memory/src/` > 0；撤销后**立即**读不到；授权语义**叠加于** `MemoryScope`（不替代） |
| C-D2 | 面名 `session_observability`（含 C-G4 + C-G3） | W5 | R0 | **新名 1**：`APEIRETH_ENABLE_SESSION_OBSERVABILITY` | `crates/engine/runtime/src/canonical/session_observability.rs` | `rg -c 'session_observability' crates/` > 0；同一回合同一时间线**两次导出逐字节等**；恢复后副作用不重复 |
| C-D3 | `transport.cross_device` | W6 | R4 | **新名 2**：`APEIRETH_ENABLE_CROSS_DEVICE` + `APEIRETH_ROAMING_IDENTITY_FILE` | 新 crate | ⛔ **待 owner 逐项重拍**（阶梯 T1 + 跨主体披露） |
| C-E1 | `tool.skill_package`（含 C-E2 插件那半边） | W3 | R3 | **新名 2**：`APEIRETH_ENABLE_SKILL_PACKAGE` + `APEIRETH_SKILL_PACKAGE_TRUST_DIR` | 新 crate `crates/foundation/trust/` | `rg -c 'skill_package' crates/` > 0；越权包拒装；**未签名包拒装**；卸载后能力表同步移除 |
| C-F1 | 面名 `multisig_verifier` | W6 | R4 | **新名 1**：`APEIRETH_ENABLE_MULTISIG` | `crates/foundation/governance/src/chain.rs` 邻域 | ⛔ **待 owner 逐项重拍**（D2 已裁 T0 不做） |
| C-G2 | `tool.replay_eval` | W5 | R0 | **新名 1**：`APEIRETH_ENABLE_REPLAY_EVAL` | `crates/engine/runtime/src/canonical/replay_eval.rs` | `rg -c 'replay_eval' crates/` > 0；**复用既有 `EvalScore` 打分器**（不新造第二套评分语义） |
| C-H1 | 面名 `routing_policy` | W1 | R0 | **复用** `APEIRETH_COST_ROUTING`（扩 purpose 写「策略层」面）⇒ **0 新名** | `crates/foundation/core/src/routing_policy.rs` | `rg -c 'routing_policy' crates/foundation/core/src/` > 0；决策必含 `selection_reason`/`fallback_reason`；缺价不编价；**与 `CostRouter`/`ProviderRouter` 同层，不造第三套** |
| C-H2 | 不新设 id（沿用 `budget.quota.depth`） | W1 | R0 | **复用** 配额家族既有 env（扩 purpose）⇒ **0 新名** | `crates/engine/runtime/src/canonical/quota.rs` | `rg -c 'depth_quota' crates/engine/runtime/src/` > 0；自述面深度维**不再为 null** |
| C-H4 | `tool.web_search` | W1 | R2 | **新名 1**：`APEIRETH_ENABLE_WEB_SEARCH` | `crates/capabilities/tools/src/web_search.rs` | `rg -c 'tool\.web_search' crates/` > 0；**先对齐 SDK 契约占名**（否则 SDK 面继续说假话）；来源缺失即拒；结果双上限 |
| C-H5 | `tool.calendar` | W1 | R2 | **新名 1**：`APEIRETH_ENABLE_CALENDAR` | **新名 1**：`APEIRETH_ENABLE_CALENDAR` | `crates/capabilities/tools/src/calendar.rs` |
| C-L1 | `provider.local_inference` | W5 | R0/R2 | **新名 1**：`APEIRETH_ENABLE_LOCAL_INFERENCE` | 新 crate（**大闭包 ⇒ 隔离**） | `rg -c 'local_inference' crates/` > 0；**开工前须先做重复审计**；断网可用；模型来源/许可显式标注 |


### 4.1 语义与机制的要点（逐族）

#### C-A1 `tool.external_gui` —— 外部 GUI 能力面（读 + 动作，**一套 locator**）

* **语义**：原生应用**窗口 → 控件树 → 稳定定位**（只读）⇒ 同一套 **locator** 供动作面消费（点击/输入/滚动）。
* **复用**：`fetch/accessibility.rs` 的 `AccessibilityTree`（**HTML** 无障碍树）**只借鉴形状，不复用语义**（原生 ≠ HTML）⇒
  ⚠️ **命名必须区分**（既有 `AccessibilityTree` 不改，新面用 `native_control_tree` 语汇）。
* **平台**：Windows UIA 先行；macOS/Linux **显式标未实现**（0 装诚实）。平台机制名**只落 backend 模块**（既有不变量）。
* **治理**：读 = R0；动作 = R3 ⇒ **预览票 + 逐次人批**（照 `device_control` 三拍先例）。

#### C-B2 `tool.http_write` —— 写方法必须长在受控出网上

* **硬约束**：`architecture_invariants.rs:144` 禁止第二 HTTP 客户端 ⇒ `egress.rs` 的 `ControlledEgress`
  **长出写方法**（与 `get_with_headers` 同族），新工具面**只调它**。
* ⚠️ `egress.rs` 在 egress 不变量守卫扫描面内 ⇒ **只加传输语义，不引入反向依赖**。

#### C-D1 `tool.object_grant` —— 对象级授权是**叠加**，不是替代

* 域级可见性已有（`MemoryScope`）；**能力级/族级授权**已有（`permission.rs` + `/v1/panel/grants`）。
* 新面 = **对象 id 粒度**的只读授予 + 撤销即时生效（复用既有热撤销语义）。
* ⚠️ **必须先裁决**：对象级 grant 是 `MemoryScope` 的**叠加**还是**替代** ⇒ 否则出现两套可见性规则。

#### C-E1 `tool.skill_package` —— 包 = manifest + 声明 + 权限 + 签名（**一次成型**）

* 既有：`SkillLibrary`（单卡 + 修订归档）、`PluginManifest` + `PluginManager`（生命周期）。
* 新面：**「包」这一层**（工具声明 + **权限声明** + 版本固定 + 安装/卸载 + **可签名可校验**）。
* ⚠️ **权限声明不得扩大宿主授权**；**未签名/验签失败 = 拒装，不降级放行**。

#### C-H1 `routing_policy` —— 与既有两层**合成一层**，不造第三套

* 既有：`CostRouter`（成本档 × 模型档案）+ `ProviderRouter`（fallback + 健康度）。
* 新面 = **统一策略层**（任务画像 × 隐私 × 成本）+ **网关侧接入**（gateway 对 `CostRouter` 零引用）。

## 5. 批次与依赖（**重复审计后重切**）

| 波 | 族 | 依赖前波 | 破棘轮净增（crate/edge/env/dep） | 共同收口 |
|---|---|---|---|---|
| **W0** | 登记 + **重复审计** + 汇总表 | — | 0 / 0 / 0 / 0 | doc class + links + index + 族级守卫 全绿 |
| **W1**（补强批 · **0 新外部 dep**） | C-B1 进程树快照 · C-B2 HTTP 写面 · C-D1 对象级授权 · C-H1 路由策略层 · C-H2 配额深度维 · C-H4 互联网检索 · **C-H5 日历面**（**7 条**） | W0 | 0 / +2 / **+3** / 0 | `arch_guard` 0 违规 · 全量门禁绿 · claims 同步 |
| **W2**（信任面 · +1 dep） | C-E1 技能/插件包（含运行时验签） | W1 | +1 / +3 / +2 / +1 | 同上 |
| **W3**（平台面 · +1~2 dep） | C-A1 外部 GUI（原生控件树 + 动作）· C-B4 浏览器会话 | W2 | +2 / +4 / +3 / +1~2 | 同上 |
| **W4**（机制面） | C-B3 脚本解释执行 · C-D2 会话可观测与恢复 · C-G2 回放评估（**3 条**） | W3 | 0~1 / +3 / +2 / 选型定 | 同上 |
| **W5**（大闭包 · **新立项**） | C-L1 端侧推理（`provider.local_inference`，断网可用） | W4 | +1 / +2 / +2 / 闭包大 | 同上 |
| **W6**（**待 owner 逐项重拍**） | C-D3 跨设备漫游 · C-F1 真多签（**2 条**） | W5 | +1 / +2 / +2 / +2 | 同上 |

> **W5 的 C-L1 是本轮新增立项**（上一版 23 族未含端侧推理）⇒ **开工前必须先做一次重复审计**
> （仓内 `provider.rs` 归一化层已有，但**无任何本地推理实现** ⇒ 预期为真新面，仍须核）。

> **额度对账**（合并后）：crate **+4**（≤ 授权 +4）· edge **+15**（⚠️ **超授权 +10** ⇒ W2/W3 收口须复评）·
> env **+16**（⚠️ **已顶格授权 +16** ⇒ W2/W3 收口须复评或追加 ADR）· dep **+4~6**。
> ⇒ ⇒ **每波收口回填契约并复评额度**（[ADR-0025 §4.3](../adr/0025-capability-creation-line-charter.md)）。

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
| **池 B 移交的 7 条**（C-C1 WS 路由 · C-F2 反思期装配 · C-H3 MCP 合规 · C-B3 跑命令半边 · C-E2 发布包签名半边 · C-F3 依赖/产物两段 · C-G1 升级+回滚半边） | **重复审计判定 = 等价面已存在**（§3.2 有 file:line）⇒ 本线**不重建**，只做 §3.1 那一面缺口 |
| **重建既有面**（自造第二套路由语义 / 第二套事件 schema / 第二套评分器 / 第二套可见性规则 / 第二套 HTTP 客户端） | 既有面已覆盖 ⇒ 重造 = 概念重复（`duplicate` 权重 3 的棘轮项） |
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

## 9. 复核命令（族级探针 · **已按审计结论校正**）

> ⚠️ **上一版探针的 3 个缺陷已修**（审计 §3.2）：
> ① C-B2 探针 `Method::POST` 只匹配字面量（仓内 SDK 写 `client.post(url)`）⇒ **假阴性** ⇒ 已加 `\.post\(`；
> ② C-E2 探针含 crate 名 `ed25519_dalek` ⇒ **一行 `use` 就长** ⇒ **假阳性** ⇒ 已改为匹配**验签调用**；
> ③ C-H3 探针 scope 只扫 `mcp_bridge` 而帧在 `foundation/plugin` ⇒ **scope 错位** ⇒ 该族已移交，探针随之移除。
> 另：**按名 0 命中 ≠ 无重复** ⇒ 每个族另有**「已有先例 file:line」**随族登记（见 §3.1），
> 守卫会**同时核验先例仍在**（先例被删 ⇒ 判定作废，必须重判重复性）。

```bash
# 池 A（14 条 + 1 新立项）
rg -n 'process_tree_snapshot|snapshot_process_tree' crates/ --glob '*.rs'            # C-B1
rg -n 'Method::POST|Method::PUT|\.post\(' crates/ --glob '*.rs'                     # C-B2
rg -n 'object_grant|MemoryGrant' crates/ --glob '*.rs'                               # C-D1
rg -n 'routing_policy|RoutingPolicy' crates/foundation/core/src/                     # C-H1
rg -n 'depth_quota|DepthQuota' crates/engine/runtime/src/                            # C-H2
rg -n 'tool\.web_search|WebSearchTool|web_search_tool' crates/ --glob '*.rs'        # C-H4
rg -n 'skill_package|SkillPackage|verify_package_signature' crates/ --glob '*.rs'      # C-E1
rg -n 'native_control_tree|external_gui|external_control_tree' crates/ --glob '*.rs'  # C-A1
rg -n 'web_browser|BrowserSession' crates/ --glob '*.rs'                             # C-B4
rg -n 'script_exec' crates/ --glob '*.rs'                                            # C-B3
rg -n 'session_observability|SessionObservability' crates/ --glob '*.rs'             # C-D2
rg -n 'replay_eval|ReplayEval' crates/ --glob '*.rs'                                 # C-G2
rg -n 'Ed25519Verifier|verify_multisig_ed25519|MultiSigThreshold' crates/ --glob '*.rs'  # C-F1
rg -n 'cross_device|CrossDevice|sync_cursor' crates/ --glob '*.rs'                    # C-D3
rg -n 'local_inference|LocalInferenceProvider' crates/ --glob '*.rs'                 # C-L1（新增立项）
```

## 10. 风险台账

| # | 风险 | 触发 | 处置 |
|---|---|---|---|
| R1 | 在途线与本线抢 `production.rs` / `env-registry.toml` | W1 注册新面时 | 写域回避 + 排队；注册面留给在途线收口后再接线 |
| R2 | 新外部包触发 `deny.toml multiple-versions = "deny"` | W2/W3/W4 | 引入前跑 `cargo deny check`；冲突则隔离新 crate + 钉版本 |
| R3 | 额度（edge/env）估算超授权（§5 已预警 **edge +15 / env +18**） | W3 收口 | 按实际重算；不够则**追加 ADR**（owner 拍板），不擅自抬 |
| R4 | 新面绕过治理链（造出越权面） | 任一写面 | 新面一律复用既有 `ToolCapability` + 治理链 + 审批链；对抗测试钉死 |
| R5 | 第三方内容进模型上下文（未标注来源） | C-H4 / C-C1 | 一律走既有 `UntrustedEnvelope` 封套；来源缺失即拒 |
| R6 | Windows-only 能力在 Linux CI 假装通过 | C-A1 | 未实现平台**显式标未实现**，不得静默绿 |
| R7 || 中性表述违规（第三方产品名进入注释） | C-A1/C-B2 借鉴既有实现时 | **新名 2**：`APEIRETH_ENABLE_GUI_READ` + `APEIRETH_ENABLE_GUI_ACTION` | 新写代码零第三方产品名；既有违规文件**只在自己写域内**顺手清（他域报备） |
| R8 | **能力 id 越界/撞名**（`CapabilityKind` 七前缀 + SDK 契约已占名） | 新族落 id 时 | 落 id 前查 `tool_taxonomy.rs` 策展表 + `sdk/client.rs:76,79` + `ws_v1.rs:141`；**已避**：`code_exec`→`script_exec`、`BrowserTool`→`web_browser` |
| R9 | **重复面漏判**（探针只判「同名」，判不了「同 job」） | 任何新族开工前 | 族规格必须带「**已有先例 file:line**」；守卫核验先例仍在；先例变了 ⇒ **重判**，不许沿用旧判定 |
| R10 | **合并族三处落地三次 schema**（C-D2 吸收了 3 族） | W4 开工 | 三件事**共用一套事件面 + 一套 span 面**；schema 变更一次到位，禁止「先出无签名包再补签名」式演进 |
