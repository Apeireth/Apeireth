# Apeireth × SpectrAI：差距分析与前端设计方案

> 日期：2026-09-22
> 输入：`crates/` 现役 canonical 后端（route/事件/SSE 亲验）、`frontend/companion-desktop/`（runtime.ts 导出清单）、`frontend-handoff.md`、`ROADMAP.md` P8/P9、SpectrAI 66 张截图理解（`reference/spectrai/INDEX.md`、`design-tokens-observed.md`）
> 本文只做分析与设计，不改任何代码。

---

## 0. 结论摘要

1. **后端真实能力比文档印象强**：token 级真流式已于 2026-09-10 打通（`/v1/chat/completions?stream=true`，provider SSE → runtime sink → OpenAI 规格逐 chunk 转发，实测 210 帧增量）；治理面（审批 / grants / guard / audit / trace / memory 治理）是 SpectrAI **完全没有**的叙事资产。
2. **最大诚实差距：情感在场（presence）目前是断线的**。`presence.rs`/`companion_serve` 只在 legacy donor 里，tokens.json 与 `presence.ts` 引用的 PAD / initiative / dream / memory_recall 四事件在现役 gateway **不存在**。这不是"缺 UI"，是"缺数据契约"，方案 §4.6 给两条重接路线。
3. **SpectrAI 给我们的真正借鉴不是"功能更多"，而是三种交互范式**：分屏标签工作区、主从布局 + 空态引导、状态语义全局一致（badge 五色 / 警示条三色）。这三件套可以直接移植进我们的三调性体系。
4. **我们不做的**：工作流 DAG 编辑器、Agent Teams 编排、内置浏览器、IM 机器人——超出 companion 定位，明确记录为"不借鉴"。

---

## 1. 后端真实能力盘点（现役 canonical gateway，逐项亲验）

### 1.1 会话与对话

| 能力 | 端点 / 机制 | 状态 |
|---|---|---|
| 健康检查 | `GET /health` | ✅ 前端 `checkHealth` 已接 |
| 模型/提供商列表 | `GET /v1/models` `/v1/providers` | ✅ `listModels` 已接 |
| 运行时快照 | `GET /v1/runtime/snapshot`（别名 `/v1/apeireth/`） | ✅ 前端诊断弹窗已挂（P9） |
| 单轮对话 | `POST /v1/chat` | ✅ |
| 流式对话 | `POST /v1/chat/completions`（OpenAI SSE；role chunk → 逐 delta → finish；approval 时以 `finish_reason: approval_required` 终止） | ✅ **token 级真流式**（P8, 2026-09-10） |
| 会话管理 | `GET /v1/panel/sessions`；`session_settings.rs`（390 行，会话级设置读写） | ✅ 后端有；前端入口薄 |
| 打断 | `barge_in.rs`（253 行）：`interrupt` 事件广播，四类原因（语音插话 / 手动取消 / 新轮抢占 / 超时），会话隔离 | ✅ 后端有；**前端无消费方** |
| Workbench | `GET /v1/workbench/turn` | ✅ `fetchWorkbenchTurn` 已接 |

**诚实注**：事件总线 `/v1/apeireth/events` 上的 `turn_delta` 仍是整段一次（`events.rs` v1 honesty note 未更新）——**生命周期事件走总线，逐字渲染走 chat completions SSE**，两条通道各司其职。文档措辞需避免说"总线是流式通道"。

### 1.2 治理面（审批 / 安全 / 审计）——差异化资产

| 能力 | 端点 | 前端对接 |
|---|---|---|
| 审批 | `/v1/approvals`（list）`/v1/approvals/resolve` `/v1/apeireth/grant`；SSE `approval_required` / `approval_resolved` | `fetchCanonicalApprovals` / `resolveCanonicalApproval` / `approveFirstPendingApproval` 已存在，**但无独立审批中心视图** |
| 记忆治理 | `/v1/panel/memory/episodes`、`:id/forget|protect|unprotect`（带 expected_rev 乐观锁）、`/v1/memory/append` | ✅ `forgetMemoryEpisode`/`protectMemoryEpisode`/`unprotectMemoryEpisode` 已接 |
| 工具 | `/v1/tools/list` | ✅ ToolsView |
| 能力声明 | `/v1/apeireth/capabilities`；前端 `capabilitySupported/unavailableReason` 0 装语义 | ✅ |
| 授权 | `/v1/panel/grants(/revoke)` | ✅ `fetchGrants`/`revokeGrant` |
| 安全 | `/v1/safety/guard/status|events` + `evaluateGuard`（dry-run） | ✅ Guard 诊断已接 |
| 审计与 Trace | `/v1/panel/traces(/:id)`、`/v1/panel/audit` | ✅ ActivityView / `fetchTraceDetail` |

### 1.3 情感在场（北极星，**当前断线** → 2026-09-23 更新：已按路线 B 重接，见 §7）

- `crates/legacy/donor/apeireth-companion/presence.rs`、`companion_serve.rs` 是 v1 遗物；现役 gateway **无** emotion PAD / initiative / dream / memory_recall 事件。
- 现役已有种子：`ember_hud_driver.rs`（Ember HUD：4.0s 生理呼吸节律立方正弦调制、周边暗角辉光、四种认知姿态枚举 DeepCodingFocus/AttentivePresence/DreamingConsolidation/EmpatheticCare、WGSL uniform 输出）——**但 gateway 路由零消费方**。
- 前端 `presence.ts` 与 `subscribeCompanionEvents` 引用的是 legacy 契约。
- 结论：设计文档（tokens.json 时间线照明 / 金色纪律 / 三调性）是产品北极星，数据契约需要重接（§4.6）。

### 1.4 明确不存在的能力

终端 WebSocket / PTY、任务调度（cron）、看板数据模型、工作流 DAG、命令注册表、浏览器内嵌、IM 机器人、token 用量计费。其中 **终端** 与 **调度** 是后端有地基（ProcessExecutor、记忆 append 等）但无 API 面；其余是 scope 决策问题而非技术问题。

---

## 2. 差距矩阵：SpectrAI 功能面 × Apeireth 现状

| # | SpectrAI 功能面 | Apeireth 现状 | 判定 |
|---|---|---|---|
| 1 | 会话/对话 | chat + 真流式 + 打断后端就绪 | ✅ 有，前端补齐打断按钮即可 |
| 2 | 终端面板（xterm） | ProcessExecutor 在，无 PTY API、无前端视图 | ❌ 缺（提案，见 §5） |
| 3 | 看板（任务列） | 无数据模型 | ❌ 缺（可选，低优先） |
| 4 | 定时任务/事件钩子 | 无调度器 | ❌ 缺（可选，低优先） |
| 5 | 工作流 DAG 编辑器 | 无 | ⛔ 不借鉴（超 scope） |
| 6 | Agent Teams | 无（council/organ 是后台机制，按 P9 决策不做主人看板） | ⛔ 不借鉴 |
| 7 | 专家库/广场 | personas（前端本地） | 🟡 轻量做：persona 卡片管理即可，不做广场 |
| 8 | 资料库 | 记忆 episodes/graph 更深（forget/protect/链接） | ✅ 有且更强，UI 需主从化改造 |
| 9 | 内置浏览器 | 无 | ⛔ 不借鉴 |
| 10 | 命令面板（Ctrl+K） | 无 | ❌ 缺，高性价比 |
| 11 | 设置深度 | SettingsView 2314 行已厚；缺会话级入口、缺 per-能力模型分配 | 🟡 半缺 |
| 12 | 模型管理 | providers/models 列表 + admin config | 🟡 半缺：无"按功能分配模型"、无 Adapter 分组 |
| 13 | 技能/MCP | tools/list + capabilities | ✅ 等价覆盖 |
| 14 | 远程机器人 | 无 | ⛔ 不借鉴 |
| 15 | 记忆管理 | episodes/graph/protect/forget + 乐观锁 | ✅ 有且更强 |
| 16 | AI 管家（小黑） | personas + workbench turn | 🟡 轻量做：专注模式侧栏 persona 化 |
| 17 | 数据指挥中心（运行/等待/异常统计栏） | runtime snapshot + guard status + SSE 生命周期事件 | 🟡 数据够，缺聚合视图 |
| 18 | 审批 | **approval + grants + guard 三重治理，SpectrAI 没有** | ✅ **独家资产，缺的是 UI** |

**一句话**：SpectrAI 赢在"执行工作台广度"（终端/看板/调度），我们赢在"治理深度 + 情感在场叙事"。前端设计应该**扬长（治理中心 + presence 场景）补短（命令面板 + 聚合状态栏 + 主从布局范式）**，而不是逐项克隆 18 个功能面。

---

## 3. 前端现状摘要

- 18.4k 行，Svelte 5 + Tauri 2，bundled-backend 侧车（`apeireth gateway serve`）。
- 已建：黑洞 WebGL2 场景引擎（机位/缓动/降级）、行星层、四档时间线照明、三模式骨架（舰桥/深舱/临渊）、对话流式 + CoT 分流、MemoryView / ConversationsView / ActivityView / SettingsView / ToolsView。
- runtime.ts 已切 canonical 协议，`releaseContractManifest()` 双轨（legacy 别名保留）。
- 开场动画已封存（审美未过）。
- UI 点击流人工实测、macOS/Linux 未验（P8 遗留）。

---

## 4. 前端设计方案

> **v2 定稿更正（2026-09-22）**：本节 §4.1–§4.2 的"三模式 × 侧轨工作区"信息架构已被 **`docs/design/00-PHILOSOPHY.md`（显影式空间界面，主人已拍板定稿）取代**。新架构四要点：① **一份契约 × 四个投影 × 三档梯队**——微信式聊天壳是长寿骨骼（T0 无 WebGL 可跑老电脑），黑洞场景/桌宠/状态条是同一份 presence 数据的可插拔投影；② **会话即调性**——日常/工程/陪伴是会话属性而非全局模式开关，他是全频谱自动换挡，照看卷宗（审批/审计/记忆）与调性正交；③ **动作收敛为对话内卡片**——审批 = in-context 待签文书卡，治理中心降级为事后卷宗；④ **功能不随梯队/调性减配**。下文 §4.3 起的页面清单仍然有效（它们全是"照看卷宗"与壳的组装件），组件移植映射（§4.4）与冲突裁决（§4.5）不变——唯"蓝色选中换金色"一条已升级为范式级纪律（金色纪律在壳中，见 00-PHILOSOPHY §7）。presence 重接路线（§4.6）已被 00-PHILOSOPHY §10 的 `presence_state` 契约草案吸收。

### 4.1 总原则（已被 00-PHILOSOPHY 取代，保留存档）

SpectrAI 其实暗合我们的三调性登记（它有"在场页/监控页/档案页"之分），我们把它显式化：

| 调性 | 职能（SpectrAI 对应） | 承载页面 |
|---|---|---|
| **Scene 深空场景调**（金） | 会话/hero/聊天（对应 SpectrAI 主界面骨架） | 舰桥对话、快捷窗、观测、专注模式 |
| **Deep-Ops 数据深舱调**（模块识别色 ≤6，避开存在金） | 终端/看板/指挥中心/设置（对应 05-总控中枢） | **新增：治理中心（审批/授权/守卫）**、任务面板、系统状态、设置 |
| **Archive 纸面档案调**（纸白/墨黑/幽灵编号） | 资料库/记忆/百科（对应 01-百科） | 记忆星图列表态、他的日记 |

金色纪律不变：任何调性下"金色 = 他"；深舱识别色用既有提案蓝 `#2392fb`/绿 `#6fae5f`/工业黄 `#e8c33a` + 待补 3 色（建议：紫=情感/姿态、青=记忆、红=安全守卫）。

### 4.2 信息架构：三模式 × 侧轨工作区（存档，已被 00-PHILOSOPHY §2/§4 取代）

保留三模式作为**调性骨架**，吸收 SpectrAI 左图标轨 + 分屏标签组的骨架思想：

```
┌────────────────────────────────────────────┐
│  图标轨(48px)  │  主区(随模式切换)  │  右分屏标签组(可收起) │
│  舰桥          │  Scene: 对话        │  终端 / 审批 / Trace   │
│  工程(深舱)     │  Deep-Ops: 治理面板 │  资料库 / 任务          │
│  专注(临渊)     │  Scene(减光)       │  单栏，无分屏           │
│  档案          │  Archive: 记忆/日记 │  —                      │
└────────────────────────────────────────────┘
│  底部状态条：SSE 生命周期 ●backend_ready / turn 状态 / guard 摘要        │
```

- 分屏标签组**先只放"已存在数据"的标签**：审批中心、Trace 详情、记忆详情——不造空标签（0 装纪律）。
- 模式切换 = 调性切换 + 主区切换；右分屏可跨模式常驻（例如专注模式下挂着审批标签）。

### 4.3 页面清单（按施工顺序）

**P0 — 治理中心（Deep-Ops，差异化主打）**
一个视图内四 tab：① 审批收件箱（pending approvals 列表 + resolve 操作 + SSE `approval_required` 实时推入，**这正是 SpectrAI 完全没有的**）② 授权管理（grants 列表 + revoke）③ 守卫（guard status/events + dry-run 试算）④ 审计（traces + audit 主从）。
交互范式直接移植 SpectrAI「主从双栏 + 空态三件套」：左列表卡、右详情，空态文案明确写"批准一条工具调用后，这里会出现它的完整执行轨迹"。

**P0 — 命令面板（Ctrl+K）**
注册表驱动：切换模式 / 打开视图 / 切换时间线档位 / 切换 persona / 健康检查重连 / 审批快速通过（`approveFirstPendingApproval` 已备）。纯前端即可做，不依赖后端增量。这是 SpectrAI 体验里性价比最高的一件。

**P1 — 底部聚合状态条**
SpectrAI 右下"运行统计栏"的对应物：SSE 连接状态、当前 turn 状态（运行中可点=打断，接 barge_in）、guard 最近事件数、记忆 episode 计数。数据源全部已有（events + guard + snapshot）。

**P1 — 记忆/档案主从化改造**
MemoryView 从现有形态改为主从双栏（Archive 调）：左 episodes 列表（过滤器 chips 带计数——移植 SpectrAI 范式），右详情含 protect/forget 操作与 graph 链接；「他的日记」按设计文档走纸面调。

**P2 — 会话级设置入口**
`session_settings.rs` 后端已就绪，做一个 per-session 设置抽屉（SpectrAI 的抽屉交互：左滑出窄抽屉 + 背景压暗）。

**P3 — 任务面板/看板（可选）**
若做，复用看板列范式 + 状态映射小字；需要后端补任务数据模型，单独立项。

### 4.4 SpectrAI 14 条组件清单 → Apeireth 移植映射

| SpectrAI 组件 | 我们的落点 | 令牌化要点 |
|---|---|---|
| 1 左图标轨侧栏 | 三模式图标轨 | 图标轨用骨白 40% 灰，选中=存在金竖条 |
| 2 顶部命令搜索 | Ctrl+K 命令面板 | 面板底色用 deepOps.base |
| 3 hero 空态 | 舰桥空会话 | 已有基础，补"描边胶囊 + chips 上下文"层 |
| 4 主从双栏 | 治理中心/记忆/档案 | Archive 版用幽灵编号背景 |
| 5 分屏标签组 | 右工作区 | 标签页头 ≤6 识别色 |
| 6 设置行/设置卡 | SettingsView 规范化 | 描述行承担教学职责（已有） |
| 7 过滤器 chips（带计数） | 记忆/审批/审计列表 | 选中 chip 用 `bg-accent-gold/10`+金字，不用蓝 |
| 8 实体卡片行 | 工具/授权/persona | badge 五色语义表见下 |
| 9 空态三件套 | 全部主从视图 | 文案必须写"选择后这里会出现什么" |
| 10 双引导卡 | 治理中心空态 | 主卡=去对话触发一条审批，次卡=看审计 |
| 11 警示条三色 + kbd | 全局 | 黄=待配置（如未配置 provider）、红=守卫触发、绿=已同步；**多一档金=他在场** |
| 12 下拉/分段/滑杆 | 设置体系 | 分段选择器选中态用金 |
| 13 深色弹窗 + 快捷键提示 | 审批确认/危险操作 | 不可逆操作（forget/revoke）用红色边框卡而非二次弹窗（移植其 63 号截图范式） |
| 14 聊天面板 | 已有对话流 | 补欢迎区能力 chips 行 |

badge 五色语义（全局唯一一套）：绿=正常/蓝=信息/黄=待完善/红=危险/灰=禁用——与 §2.4 语义色提案对齐。

### 4.5 与现有视觉系统的冲突裁决

- SpectrAI 是"蓝 = 选中/主按钮"语言；**我们一切蓝选中替换为金选中**（金色纪律），蓝色只保留为 Deep-Ops 模块识别色之一。
- SpectrAI 的分屏标签组、统计栏偏"IDE 密集感"，与我们的"电影感，不是软件感"（§1 总纲）有张力 → 落地时**降信息密度**：状态条只留 4 个指标，标签组默认收起。
- 模拟态标注（§5.4）继续有效：任何依赖未实现 API 的入口显式标注"后端未开通"，不做假数据。

### 4.6 presence 重接：两条路线

| 路线 | 做法 | 代价 | 建议 |
|---|---|---|---|
| A 迁 legacy companion_serve | 把 donor 的 `:8090` 服务搬进主链，gateway 代理四事件 | 拖入 v1 债务（README 已标 deferred）；双服务运维 | 不推荐现在做 |
| B **gateway 内重实现（推荐）** | 借 `ember_hud_driver.rs` 已有认知姿态枚举做种子：runtime 每轮结束产出 PAD 粗估值 + initiative 事件 → 复用现有 events bus 新增 `presence_state` 事件（与 turn_* 同管道），前端 presence.ts 改订新契约 | 需定义 emotion 估算源（可先规则化：轮次时长/审批频率/记忆召回命中率 → PAD，诚实标注 v0 为启发式） | 选 B：一个事件类型 + 一个驱动模块，先让 HUD/照明有真数据，再谈精度 |

P7（voice/screen 连续感知）保持后置，presence v0 不依赖硬件。

---

## 5. 后端补强清单（提案，按优先级）

| 优先 | 缺口 | 建议端点 | 说明 |
|---|---|---|---|
| 高 | presence 事件 | events bus + `presence_state`（路线 B） | §4.6 |
| 高 | turn_delta 文档勘误 | — | events.rs honesty note 与 P8 真流式并存易误读，注释需改写 |
| 中 | 打断 HTTP 面 | `POST /v1/turn/interrupt`（接 barge_in） | 前端打断按钮的前提 |
| 中 | 审批负载文档化 | canonical_approval_lifecycle.rs 抽 API 契约段 | 治理中心 UI 的形状依据 |
| 低 | 终端 PTY WebSocket | `/v1/terminal/ws` | 依赖 ProcessExecutor 扩展，单独立项 |
| 低 | 任务调度 | `/v1/schedule/*` | 看板的前置；独立立项 |
| 中（2026-09-22 侦察增补） | 会话分支数据模型 | session 树（父会话/分支点/重说版本），分支点挂记忆 episodes 锚点 | 星野六机制 + Cherry Studio 佐证：对话掌控感是刚需；前端回溯/重说的后端前置 |

---

## 6. 施工顺序建议（2026-09-22 按 00-PHILOSOPHY 回改）

1. **presence_state 契约**（后端，00-PHILOSOPHY §10）——四个投影全压在这一个事件上，先于一切 UI。
2. **聊天壳骨骼**：消息列表主页 + 会话内卡片渲染（审批卡/工具卡/星尘卡）——长寿载体，T0 可用。
3. **治理卷宗**（原"治理中心"，现定位=事后卷宗；in-context 审批卡已在第 2 步进对话）。
4. **Ctrl+K 命令面板 + 打断按钮**（打断需后端 `POST interrupt`，可先用"前端切断渲染 + 新轮 preempt"近似）。
5. **底部状态条**（SSE 聚合，余光投影）。
6. **记忆卷宗主从化 + 日记纸面调**（Archive 调首次实拍校准，反哺 design tokens）。
7. T1/T2 显影接线（余烬点→光环→全参数，随 presence 数据变真而逐级点亮）→ 桌宠（常驻投影）→ 会话设置抽屉 → 可选的看板/终端立项。
8. **会话分支**（2026-09-22 侦察增补）：回溯/重说/分支树，分支点挂 episodes 锚点——需 §5 后端数据模型先行，列第二梯队 backlog。

---

*附：本文所有后端结论均亲验于 `crates/adapters/gateway/src/canonical_entry.rs`、`panels.rs`、`events.rs`、`barge_in.rs`、`ember_hud_driver.rs` 源码；前端对接状态以 `runtime.ts` 导出清单为准。*

---

## 7. K3 批次施工结果（2026-09-23 回写）

§6 施工顺序的 ①–⑦ 已全部落地并经主会话逐项视觉验收（CDP 截图亲审，真后端为主；验收登记 = `docs/04-internal/live-verification-ledger.md` #29/#30，工程日志 = `docs/04-internal/engineering-log-2026-09-22.md`）：

| §6 项 | 状态 | 落点与证据 |
|---|---|---|
| ① presence_state 契约 | ✅ | gateway `presence.rs`（60s 心跳 + 衰减 baseline + initiative 预算器，12 单测）；SSE 实收 2 帧心跳逐字段核契；commits `20cf3d1b`/`487b7141`/`fa011d07` |
| ② 聊天壳骨骼 | ✅ | 三栏主从（图标轨｜常驻列表栏｜聊天区）+ 个性化背景（默认静态图 / 主题 / 自定义上传 IndexedDB）+ accent 配色目录 + 会话内审批卡（PendingDocumentDock）；星尘卡蛰伏（总线无 memory_recall，0 装）；commits `5021157e`…`63557256` |
| ③ 治理卷宗 | ✅ | 左轨入口，四 tab（审批/授权/守卫/审计），Deep-Ops 调首次实拍；真后端 grants 3 条 / traces 46 / audit 100；commits `a2ecd244`/`dc821e55`/`40f78845` |
| ④ Ctrl+K + 打断 | ✅ | 19 条命令（导航 8 + 主题 7 + 动作 4），别名/拼音过滤，最近优先持久化；打断 = 前端切断收听（诚实文案「打断的是收听，不是他」），后端 interrupt HTTP 面仍列 §5；commits `d1252744`/`24c862ec` |
| ⑤ 底部状态条 | ✅ | 四指标（SSE/回合/守卫/记忆），安静态全灰与断线「重连中 + 不可用」级联均实测；commits `f9fecc02`/`6f4160b8` |
| ⑥ 记忆卷宗 + 日记 | ✅ | 记忆主从双栏（Archive 调首次实拍，inkGold 校准），forget 内联确认 + 409 冲突卡；日记 = 纸面空态契约页（后端无日记端点）；commits `104d9a2b`/`5497ae2a`/`78e01e36` |
| ⑦ T1/T2 显影接线 | ✅ | `presence.ts` 重写订 `presence_state`（具名 addEventListener），显影分级（heartbeat 余烬 / turn 光环 / ritual 契约空间），T2 PAD 映射增益 0.75 保守值；真后端 60s 心跳实测余烬 amplitude 0.65→0.2；commit `7026a3a7` |

**剩余梯队（未开工，等主人指令）**：桌宠（常驻投影，00-PHILOSOPHY §5 🟡 提案待校准）→ 会话设置抽屉（P2）→ 看板/终端（§5 独立立项）；**会话分支**（§6 第 8 项）仍列第二梯队 backlog，需后端数据模型先行。

**v0 诚实边界（施工期确立）**：`empathetic_care` / `ritual` 无生产者（契约空间保留）；initiative 只有预算器无生产者；prompt-overlay 召回命中不上总线（需 runtime 加 TraceEvent 变体）；审批卡对话内真后端全闭环与 UI 点击流人工实测仍挂账（台账 §2 #2/#4）；`subscribeCompanionEvents`（legacy 伴随体订阅）同患 mount 门恒假——点亮会唤醒「他说」主动开口整条链，需主人拍板后再动。
