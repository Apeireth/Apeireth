# 能力创造线 · 进度页（做到哪了）

[Document-Meta]
Document: docs/status/capability-creation-line-progress.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-08
Author: lead（插件/组件/能力开发线 总指挥）

> **本页是「做到哪了」的唯一权威页**（`AGENTS.md` §0.2：状态只引文件，不引对话记忆）。
> **目标形态** ⇒ [设计稿](../01-architecture/capability-creation-line-spec.md)；
> **授权** ⇒ [ADR-0025](../adr/0025-capability-creation-line-charter.md)（owner 2026-11-08：只拓新面 · 允许破棘轮 · 本轮只出设计与排期）。
> **重复审计** ⇒ 本页 §3（2026-11-08 定案：**真新面 0 / 站得住 14 族 / 移交 7 条**）。
> **工作线登记** ⇒ [workline-registry.md](workline-registry.md)。

---

## 0. 一句话

> **W0（登记批 + 重复审计批）已落地**：ADR + 设计稿 + 本进度页 + 族级守卫 + 工作线登记。
> **W1 落地 4 族**（C-B1 进程树 · C-B2 HTTP 写面 · C-H4 互联网检索 · C-H5 日历面）⇒ **PRODUCTION WIRED + 默认关**；
> **W2 落地 2 族接线**（C-H1 路由策略层 CLI 接入 + 自述探针 · C-H2 深度维接线 + 上限旋钮 + 自述真值）⇒ **PRODUCTION WIRED**，
> **但两族各有一条硬边界**（C-H1 信号需显式注入才生效 · C-H2 当前可达深度 ≤ 1 ⇒ 上限 > 1 不可达 ⇒ 不是有效防护）；
> **C-D1 账本加固 + 读路径并联完成，但仍未注册**（无生产激活口）⇒ 如实标 🟡 在途；
> **C-L1 端侧推理**待重复审计 · **池 B 7 条移交** · **存量接线 6 项移交**。

---

## 1. 波次看板

| 波 | 族数 | 状态 | 落地证据 | 还差什么 |
|---|---:|---|---|---|
| **W0** | — | ✅ **已落地** | 本页 + 设计稿 + [ADR-0025](../adr/0025-capability-creation-line-charter.md) + `scripts/check_creation_line.py` + 登记表新增行 + **重复审计**（本页 §3） | ❌ 待 owner 复核重复判定与池 A/B 划分（**唯一待办**） |
| **W1** | 7 | 🟡 **4 族已落地 / 3 族未接线** | C-B1 · C-B2 · C-H4 · C-H5 ⇒ **PRODUCTION WIRED + 默认关**（注册根 + 治理档 + 策展表 + 名册 + env 登记齐） | ❌ C-D1（读路径并联待 W2）· C-H1（缺隐私信号源）· C-H2（缺运行时喂深度点）未接线 |
| **W2** | 3 | 🟡 **2 族已接线 / 1 族仍不注册** | C-H1（CLI 接入 + 自述探针 + `privacy_source` 格）· C-H2（深度接线 + `APEIRETH_QUOTA_MAX_DEPTH` 旋钮 + 自述真值） | ❌ C-D1：账本读端加固 + 读路径并联**已完成**，但 `approve_grant`/`deny_grant` **全仓只有测试调用点** ⇒ 生产里授权永远停在 pending ⇒ **不注册**（激活口属宿主面） |
| **W2** | 1 | ❌ **未开工** | — | ❌ C-E1 未落地（技能/插件包 + 运行时验签） |
| **W3** | 2 | ❌ **未开工** | — | ❌ C-A1 · C-B4 未落地（平台依赖；引擎选型待定） |
| **W4** | 3 | ❌ **未开工** | — | ❌ C-B3 · C-D2 · C-G2 未落地 |
| **W5** | 1 | ❌ **未开工** | — | ❌ C-L1 端侧推理未落地（**新立项 · 开工前须先做重复审计**） |
| **W6** | 2 | ⛔ **未排期**（需 owner 逐项重拍） | — | ❌ C-D3 跨设备漫游 · C-F1 真多签 **待 owner 拍板**（D2 已裁「T0 不做」⇒ 本线不擅自推翻） |

**W0 的验收证据（实测）**：

```bash
python3 scripts/check_creation_line.py --check     # ⇒ 15 族逐条判定 + 先例互校，全绿
python3 scripts/check_doc_class.py --check         # ⇒ 0 违规（本线两份文档已在分类内）
python3 scripts/docs_links.py --check              # ⇒ 断链 0
python3 scripts/docs_index.py --check              # ⇒ 问题数 0
python3 scripts/arch_guard.py --strict             # ⇒ VIOLATIONS: 0（额度未动）
python3 scripts/docs_truth.py --verify --check     # ⇒ claims=N fail=0 stale=0
```

---

## 2. 族级看板（**池 A 15 条 · 状态由 `scripts/check_creation_line.py` 机器判定**）

> **重复判定列** = 2026-11-08 重复审计结论（§3）：**部分重复（需补强）** ⇒ 本族只补缺口、**不重建**已有面。
> **已有先例列** = 该族**必须复用/不得重建**的既有面（file:line）⇒ 守卫会**核验这些锚点仍在**；
> 先例被删 ⇒ 判定作废 ⇒ **必须重判重复性**（不许沿用旧判定）。
> ⚠️ 标「✅ 已落地」而探针未长 ⇒ **守卫红**（谎报）；标「❌ 未开工」而探针已长 ⇒ **守卫红**（已被他线实现，须认领或改判）。

| 族 | 能力 id / 面名 | 重复判定 | 已有先例（file:line，**不得重建**） | 波次 | 状态 | 治理档 | 还差什么 |
|---|---|---|---|---|---|---|---|
| C-A1 | `tool.external_gui`（含 C-A2） | 部分重复 | `fetch/accessibility.rs` AccessibilityTree · `gateway/im_bridge.rs` 动作链 | W3 | ❌ 未开工 | R0/R3 | ❌ 实现 + Windows 真机用例（Linux 须显式标未实现）+ locator 模型 |
| C-B1 | `tool.process_tree` | 部分重复 | `process/mod.rs` ProcessTreeContainment · `std_sub_supervisor.rs` 真 kill | W1 | ✅ 已落地 | R0/R3 | 已落地：库级内核 + 模型面工具 + 生产注册（production.rs:696）+ L3 + WRITE_OR_EXECUTE + 复用 `APEIRETH_ENABLE_SHELL`。⚠️ Windows 已实现**未上真机**（收容组归属一律 `Undetermined`）· macOS 显式未实现 | ❌ 实现 + 快照零审批断言 + 平台机制名不进公共契约 |
| C-B2 | `tool.http_write` | 部分重复 | `egress.rs` get_with_headers（**只有 GET**）· `sdk/client.rs` 真 POST | W1 | ✅ 已落地 | R2 | 已落地：写方法长在 `ControlledEgress`（**不自带第二 HTTP 客户端**）+ 生产注册（production.rs:669）+ L3 + WRITE_OR_EXECUTE + 2 个新 env 名。⚠️ **凭据绑定表默认无绑定** ⇒ 生产上只能发未鉴权请求（带 `credential_ref` 的调用拒绝即帧） | ❌ **扩 ControlledEgress 写方法** + 新增「无第二 HTTP 客户端」不变量测试 |
| C-B3 | `tool.script_exec`（跑命令半边移交 `tool.shell`） | 部分重复（半边移交） | `shell.rs` tool.shell（已接线） | W4 | ❌ 未开工 | R3 | ❌ 实现（仅语言代码解释执行）+ 四类逃逸拒绝用例 + **选型待定** |
| C-B4 | `tool.web_browser` | 部分重复 | `stealth_crawler.rs`（零 IO 未接线）· `fetch.rs` 受控 GET | W3 | ❌ 未开工 | R2/R3 | ❌ 实现 + 白名单 + 证据帧 + **引擎选型待定** |
| C-D1 | `tool.object_grant` | 部分重复 | `memory/scope.rs` MemoryScope · `governance/permission.rs` 授权底座 | W1 | 🟡 在途 | R0/R2 | **W2 已完成**：① 读路径并联（候选侧 SQL 并入被授 id + 协调器 4 处判定并联 + 管线可见域合并，**18 例**）；② **账本读端加固**（复核查出 3 个洞：旧 `active` 行**重放复活**已撤销授权 / **伪造 `active` 行跳过人批** / `version` 从不校验 ⇒ 修法：按 version 单调择一 + `active` 须有 pending 前身且 `decided_by` 非空 + 版本区间校验；顺带修「末行缺换行 ⇒ 末条授权静默消失」）；③ 元数据收口（`check` 认证闸 / 撤销归属校验 / subject 形状校验 / 审计写失败不静默 / 不回显域不可见对象的 partition）。**仍不注册**：`approve_grant`/`deny_grant` 无生产调用点 ⇒ 授权永远停在 pending ⇒ 注册它 = 注册一个「永远不能生效」的面；**待宿主面激活口**（面板/IM 审批卡回调） | ❌ 实现 + **撤销即时生效**测试 + **叠加/替代裁决** |
| C-D2 | 面名 `session_observability`（含 C-G4 + C-G3） | 部分重复 | `agent_trace.rs` TraceSpan · `durable/replay.rs` recover_attempt · `session_events.rs` marker | W4 | ❌ 未开工 | R0 | ❌ 实现（快照/恢复/统一时间线**共用一套事件面**）+ 两次导出逐字节等 |
| C-D3 | `transport.cross_device` | 部分重复 | `research_roaming_memory.rs` 合并/墓碑原语（未接线） | W6 | ⛔ 未排期 | R4 | ❌ **待 owner 重拍** + 先提取 merge/tombstone 为通用原语 |
| C-E1 | `tool.skill_package`（含 C-E2 插件那半边） | 部分重复 | `skill/library.rs` SkillLibrary · `plugin/manifest.rs` PluginManifest | W2 | ❌ 未开工 | R3 | ❌ 实现 + 越权包拒装 + 未签名拒装 + 卸载不留残影 |
| C-F1 | 面名 `multisig_verifier` | 部分重复 | `core/onion.rs` verify_multisig · `governance/chain.rs` PlaceholderMultiSignVerifier | W6 | ⛔ 未排期 | R4 | ❌ **待 owner 重拍**（D2 已裁 T0 不做） |
| C-G2 | `tool.replay_eval` | 部分重复 | `governance/eval.rs` EvalScore · `memory/d4_eval.rs` score | W4 | ❌ 未开工 | R0 | ❌ 实现（**复用既有打分器**，不新造第二套评分语义） |
| C-H1 | 面名 `routing_policy` | 部分重复 | `core/cost_router.rs` CostRouter · `runtime/provider.rs` ProviderRouter | W1 | ✅ 已落地 | R0 | **W2 已接线**：CLI 组合根读**显式声明**的隐私档（`privacy_signal_from_str`；不认识的值 ⇒ **拒路由**，不降级为普通档）⇒ 策略层进选路路径 + 自述面加 `privacy_source`（declared/defaulted）格 + 探针（谁被排除且为何）。⚠️ **两条硬边界**：① 仓内**无隐私信号真源**（唯一现成载体「规范请求 metadata」**会整包外发给厂商** ⇒ 已否决）⇒ **未注入 ⇒ 闸常态不生效**（漏接 = 静默失效）；② 本层是既有两层**之上**的第三层（**非**「合成一层」）⇒ **网关侧接入未交付**（规格 §4.1 已按实测校正） | ❌ 实现 + 决策可解释 + 缺价不编价 + **与既有两层同层** |
| C-H2 | 不新设 id（`budget.quota.depth`） | 部分重复 | `runtime/quota.rs` QuotaDimension · `tools/self_status.rs` 深度维占位 | W1 | ✅ 已落地 | R0 | **W2 已接线**：`Session` 加 `depth_quota` 槽 + `execute.rs` 四处检查点改 `check_with_depth` + 喂深度点（F1/F3/F4/F7）+ `APEIRETH_QUOTA_MAX_DEPTH` 旋钮（**严格解析**，非法值启动即报错）+ 自述面真值（上限/已耗=峰值/当前层）。⚠️ **硬边界**：深度 = **运行时调用树读数**，**当前可达深度 ≤ 1**（`DEFAULT_MAX_INVOCATION_DEPTH=1` + 子循环那条链是「平」的）⇒ **上限 > 1 不可达（配了也不触发）** ⇒ 该维**目前不是有效的深度防护**；F6（侧车 agent 那一跳）未喂（子 agent 自带 session ⇒ 父账本看不见，同 token/step/cost 既有缺口）；F4 **永久偏低 1**（存量平链读数，已逐字标注） | ❌ 实现 + 自述面深度维不再为 null |
| C-H4 | `tool.web_search` | 部分重复 | `sdk/client.rs` web_search（**契约已占名**）· `tools/search.rs` SearchTool（本地） | W1 | ✅ 已落地 | R2 | 已落地：生产注册（production.rs:678）+ 显式 **L2**（出网只读）+ env 登记 + 端点走配置面（`<data>/web_search.json`）。⚠️ **端点未配 ⇒ 面不注册**；结果正文走 `UntrustedEnvelope` 封套；条数上限按**端点原始条数**判（超限整单拒绝） | ❌ 实现 + **先对齐 SDK 契约占名** + 来源缺失即拒 + 双上限 |
| C-H5 | `tool.calendar` | **真新面** | **无先例**（⚠️「calendar 25 命中」全是 `canonical` 子串误判）· 边界对照 `engine/organ/src/care/tool.rs` CARE_SCHEDULE（**关怀例程 ≠ 主人日程**） | W1 | ✅ 已落地 | R2 | 已落地：3 文件拆分（mod/store/tool，均 < 800 行）+ 生产注册（production.rs:686）+ L3 + WRITE_OR_EXECUTE + `CalendarApprovalHook` 已挂治理链 + env 登记。账本 = `<data>/calendar/` 两本 JSONL（条目账 + 审计账），坏行按不存在并计数上报 | ❌ 实现（本地日程账 + 查询/创建/改期/取消，**零外部集成**）+ 写动作逐次人批 + **与 `tool.care_schedule` 边界写进族规格** |
| C-L1 | `provider.local_inference` | **待立项** | `runtime/provider.rs` ProviderRouter（归一化层；**无本地推理实现**） | W5 | ❌ 未开工 | R0/R2 | ❌ **先做重复审计** + 引擎选型 + 依赖闭包实测 |

---

## 3. 重复审计结论（**2026-11-08 定案** · 判据见设计稿 §1.1b）

> **方法**：按**语义重叠**（同一个 job 的既有实现面）判定，**不是**按同名符号。
> **关键结论**：**按名 0 命中 ≠ 无重复** —— 23 族**全部**存在语义先例 ⇒ **真新面 = 0**；
> 按本线自有判据（§1.1 ②③）复核后，**站得住 14 条**（含 3 处合并）、**移交 7 条**。

### 3.1 判定分布

| 判定 | 条数 | 说明 |
|---|---:|---|
| **真新面** | **1** | C-H5 日历面（仓内**零**实现；先前的「calendar 25 命中」是 `canonical` 的**子串误判**） |
| **部分重复（需补强）** | **14** | 池 A：写清「已有 X（file:line）· 缺 Y」⇒ 只补 Y |
| **纯重复（应移交）** | **7** | 池 B：等价面已存在 ⇒ 实为接线/搬运 ⇒ 不在本线 |

### 3.2 池 B 移交条目（**本线不做**）

| 条目 | 已有面（等价/已覆盖） | 移交去向 |
|---|---|---|
| C-C1 WS 全双工路由 | `protocol/ws_v1.rs` 8 帧协议（**LOCKED 集成点**）· `gateway/duplex_gateway.rs` + `barge_in.rs` | 存量线（网关路由 + SDK 传输各归其原线） |
| C-F2 R4 反思期装配 | `governance/chain.rs` `DelayedReflectionScheduler`（真实现，生产未注入） | 治理线（owner 2026-10-26 已裁「阶梯未到」） |
| C-H3 MCP 协议合规 | `plugin/mcp/subscribe.rs` 通知帧构造器 · `mcp_bridge/catalog.rs` 目录刷新 | MCP 线（**落点在他线在途段**） |
| C-B3 跑命令半边 | `tools/shell.rs` `tool.shell`（已生产接线） | `tool.shell` 存量线 |
| C-E2 发布包签名半边 | `scripts/release/cosign-*.sh` + `cosign.yml` + `docs/security/cosign-keys.md` | CI / 发布线 |
| C-F3 依赖+自家产物两段 | `cargo-audit.yml` + `cargo-deny.yml` + `deny.toml` + `cosign.yml` + `dependabot` | CI 线 |
| C-G1 升级+回滚半边 | `scripts/upgrade/*.sh` + `runtime-assembly/upgrade_cycle.rs` L0-L5 | 升级线 |

### 3.3 审计顺带抓到的**守卫缺陷（已修）**

| # | 缺陷 | 修法 |
|---|---|---|
| 1 | 探针只判「同名 0 命中」⇒ **判不了「是不是重复」** | 每族登记**「已有先例 file:line」**，守卫**核验先例仍在** ⇒ 先例变 ⇒ 判定作废须重判 |
| 2 | C-B2 探针 `Method::POST` 只匹配字面量（仓内写 `client.post(url)`）⇒ **假阴性** | 探针加 `\\.post\\(` |
| 3 | C-E2 探针含 crate 名 `ed25519_dalek` ⇒ **一行 `use` 就长** ⇒ **假阳性** | 改匹配**验签调用**而非 crate 名 |
| 4 | C-H3 探针 scope 只扫 `mcp_bridge` 而帧在 `foundation/plugin` ⇒ **scope 错位** | 该族已移交 ⇒ 探针随之移除 |

### 3.4 审计暴露的**本线自相矛盾（已消）**

> 上一版进度页 §4 移交清单写「R4 反思期装配 ⇒ 治理线 ⛔ 不做」，§2 族级看板却仍把 C-F2 挂在 W6 ⇒ **两处口径打架**。
> ⇒ 本版把 C-F2 移入**池 B**，两处口径统一。

---

## 3.5 W1 波次落地记录（2026-11-08）

| 项 | 值 |
|---|---|
| 落地族 | **4 族 PRODUCTION WIRED（默认关）**：`tool.process_tree` / `tool.http_write` / `tool.web_search` / `tool.calendar` |
| 生产装配 | 4 个 `ModuleFamily` 变体 + 4 段注册（`production.rs:669/678/686/696`）+ 4 个配置字段（`production_config.rs`） |
| 治理 | 显式档位：`http_write`/`calendar`/`process_tree` ⇒ **L3**；`web_search` ⇒ **L2**；`WRITE_OR_EXECUTE` 进 http_write/calendar/process_tree（web_search 只读**不进**） |
| 审批钩子 | `CalendarApprovalHook` + `ProcessTreeApprovalHook` **已挂治理链**（按开关装，关时连钩子都不装）；`http_write`/`web_search` 走**面级 `require_approval_for`**（每次停人批，无自动放行档） |
| env | **+4 个新名**（`APEIRETH_ENABLE_CALENDAR` / `APEIRETH_ENABLE_HTTP_WRITE` / `APEIRETH_HTTP_WRITE_ALLOWLIST` / `APEIRETH_ENABLE_WEB_SEARCH`）；`process_tree` **复用** `APEIRETH_ENABLE_SHELL` ⇒ 契约上限 **160 → 164** |
| 配置面（非 env） | `<data>/web_search.json`（检索端点）· `<data>/http_write.json`（凭据绑定表）⇒ **默认无内容 ⇒ 面不注册或写面拒绝即帧** |
| 策展表 | `tool_taxonomy.rs` 版本 3 → 4（+5 行；`tool.object_grant` 留行为**策展占位**，口径 = IMPLEMENTED，**不是已接线声明**） |
| 0 装诚实 | ① `tool.http_write` 的凭据绑定表默认无绑定 ⇒ 生产上只能发**未鉴权**请求；② `tool.web_search` 端点未配 ⇒ 面不注册；③ `tool.process_tree` 的 Windows 分支**未上真机**（收容组归属一律 `Undetermined`）；④ 3 族（对象级授权 / 路由策略层 / 配额深度维）**未接线** ⇒ 机制在、面不在 |

## 3.6 W2 波次落地记录（2026-11-08）

| 项 | 值 |
|---|---|
| 落地族 | **2 族接线**（C-H1 路由策略层 CLI 接入 + 自述探针 · C-H2 深度维接线 + 旋钮 + 自述真值）+ **1 族加固不注册**（C-D1） |
| 并行 | **8 路同跑**：4 路实现（按 crate 切写域：memory / runtime / core+plugin / runtime-assembly+cli ⇒ **零文件重叠**）+ 4 路**只读对抗复核**（C-D1 / C-H1 / C-H2 / 接线治理）⇒ **复核产出 3 个真洞**（账本重放复活 + 伪造 active 跳过人批 + version 不校验）与 2 条治理洞 |
| 对抗复核的硬产出 | ① C-D1：账本读端**从不校验 version** + 折算只看行序 ⇒ 旧 `active` 行重放可**复活已撤销授权**、伪造 `active` 行可**跳过人批直接生效** ⇒ 已修；② C-H1：复核证伪了规格里的「合成一层 + 网关侧接入」⇒ **规格已按实测校正**（不追认超载）；③ C-H2：运行时调用树深度读数**恒为 0/1** ⇒ 「> 1 的上限永远不可能触发」⇒ 深度维**不是有效防护**，已逐字标注；④ 治理：`full` 预设会把 RequireApproval **改写为放行** ⇒ 逐次人批**有条件** ⇒ 已把 `tool.process_tree`/`tool.http_write` 列入**人工审批地板**（存量零影响）⇒ 这两族**绝对**，另两族如实标**有条件** |
| 新 env 名 | **+1**（`APEIRETH_QUOTA_MAX_DEPTH`，严格解析）⇒ 契约上限 164 → **165**（授权 +16 内） |
| 额度 | crate **+0** · 依赖边 **+0** · env **+1** · 尺寸：7 个既有文件真增长 ⇒ 波次收口由 lead 重生成基线（**未抬 `max_file_loc`**，仍 6300） |
| 0 装诚实 | ① C-H1 闸门**未注入信号时不生效**（漏接 = 静默失效 ⇒ 自述面用 `privacy_source` 暴露）；② C-H2 **上限 > 1 不可达** ⇒ 不是有效防护；③ C-D1 **未注册**（无生产激活口）；④ `runtime/src/canonical/runtime.rs` 现 **1199 行**（**距棘轮阈值 1200 只差 1 行**）⇒ 下一波动它即进逐文件棘轮 |

## 4. 额度台账（**每波收口回填**）

| 棘轮 | 现值 | 本 ADR 授权上限 | W0 落库后 | 还差什么 |
|---|---:|---:|---:|---|
| `max_workspace_crates` | 31 | 35 | **31（未动）** | ❌ 合并后估算 +3~4 ⇒ 波次收口时抬（用不完就少抬） |
| `max_production_edges` | 118 | 128 | **118（未动）** | ❌ 估算 **+14 超授权 +10** ⇒ W2/W3 收口须复评或追加 ADR |
| `max_distinct_names`（env） | 160 | 176 | **164（本波已抬 +4：CALENDAR / HTTP_WRITE / HTTP_WRITE_ALLOWLIST / WEB_SEARCH）** | ✅ 在授权额度内（+16）⇒ 剩 +12 |
| `max_file_loc` | 6300 | 6300（不动） | 6300 | — （零余量 ⇒ 新面一律新开文件） |

---

## 5. 移交清单（**存量接线，不属本线**；owner 2026-11-08 裁「不动存量」）

| 项 | 实测依据 | 移交去向 | 本线状态 |
|---|---|---|---|
| 记忆 P0 列封缄接线 | `storage::crypto::classify_column` 已逐列定级；`memory→storage` 边已存在 | 存量安全线 | ⛔ 不做 |
| MCP 结果面凭据绊线（批次 3c-1b） | `merge-integration-closeout-2026-11-07.md` §5-9 | **在途线**（owner 在盯） | ⛔ 不碰 `production.rs` / `env-registry.toml` / `mcp_bridge` 在途段 |
| 网关路径任务源路由 | `CostRouter` 已在 CLI 侧生产在用 | 存量线 | ⛔ 不做 |
| 网关侧子代理编排端点 | `RuntimeSubLoopSpawner` 已生产在用（CLI 侧） | 存量线 | ⛔ 不做 |
| R4 反思期装配 | `DelayedReflectionScheduler` 库已实装、生产未装配 | 治理线 | ⛔ 不做（= 池 B C-F2） |
| ROADMAP §3 回填（G1/G2/G3/G5/G7 五行陈旧） | 实测：G2/G3/G5 已销账未回填；G7 能力早已上线只差接线 | 文档线 | ⛔ 不做（本线**不改他人行**） |

---

## 6. 本线纪律（**每批开工前先看**）

1. **不碰他线在途面**：`production.rs` / `env-registry.toml` / `mcp_bridge` 在途段 ⇒ 写域回避 + 排队。
2. **新面一律新开文件**（< 800 行）；既有大文件（`doc_ops.rs` 6255 等）**只读不写**。
3. **新增写能力默认关** + 关时**逐字节零变化**测试 + env 登记（含 purpose 与 evidence）。
4. **不自带第二套**治理 / HTTP 客户端 / 进程通道 / 路由语义 / 事件 schema / 评分器 / 可见性规则。
5. **平台机制名只落 backend 模块**（既有不变量 `public_contract_does_not_name_platform_sandbox_mechanisms`）。
6. **每族至少 1 个对抗测试**（越权 / 篡改 / 越界 / 超限 **必拒**）。
7. **0 装诚实**：未实现的平台/引擎**显式标未实现**，不得静默绿；本线最多承诺 `PRODUCTION WIRED`，
   **不承诺任何 `HARDWARE VALIDATED`**（本仓现值 0 条）。
8. **不重建已有面**：族规格的「已有先例」列即为禁建清单；先例变了 ⇒ **重判**，不许沿用旧判定。

---

## 7. 变更纪律

* **状态变了必须改本页**（并在 `docs/INDEX.csv` 重新生成）；
* 引用状态时**只引本页**，不引对话记忆（2026-10-28 教训：上下文压缩会丢掉「我做过什么」）；
* 每族落地 ⇒ 把**证据路径**写进第 2 节该行的「还差什么」列（清空即完成），
  并让 `scripts/check_creation_line.py` 的探针从基线长出来（**状态词与探针必须同时变**）。
