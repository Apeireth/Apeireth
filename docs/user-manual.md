# Apeireth 用户手册（user-manual, 2026-08-16）

> **给谁看**: 运行 Apeireth 的主人/用户。回答「它在做什么、我怎么控制它」。
> **0 假装**: 机制描述均以真实代码为准并标注来源；**不虚构交互**（没有弹窗就说没有弹窗）；未实装的如实列在 §8。
> 跑起来见 [quick-start.md](quick-start.md)；装配裁剪见 [capability-packs.md](capability-packs.md)。

---

## 1. Apeireth 是什么

**基地，不是 AI 本身**：Apeireth 是给 LLM 的「操作系统」——提供器官 + 工具 + 记忆 + 关系可能性；不定义 AI 是什么（team-work-doc §1.1）。你接入一个 LLM（默认 MiniMax），基地给它：记忆、工具、安全边界、主动陪伴的节奏、与你的关系存档。

两个常驻形态（命令见 quick-start）：

- **companion_serve**：OpenAI 兼容端点（默认 8090）——任何兼容前端接上即拥有全部能力；对话 + 记忆生命周期 + SSE 主动送达（`examples/companion_serve.rs` 模块头）。
- **companion_daemon**：心跳驱动的常驻问候进程 + 多通道送达。

## 2. 记忆与连续性

### 2.1 存哪里

SQLite 库，默认 `%APPDATA%\apeireth\memory.sqlite`（`APEIRETH_MEMORY_PATH` 可改）。**统一锚点**：`APEIRETH_CONTINUITY_ID`（默认 `companion-main`）——记忆/日志/目标/反思共用同一身份锚；工程上做最大努力的记录 + 迁移，不假装灵魂同一（team-work-doc §1.1）。

### 2.2 记忆生命周期（自动，无需操作）

| 机制 | 何时发生 | 做什么 |
|---|---|---|
| 对话后提炼 | 节流默认 600s（`APEIRETH_EXTRACT_INTERVAL_SECONDS`）+ 6h 批量 | LLM 提炼 facts/preferences/commitments/emotional/graph（带 importance），Mem0 式对账 ADD/UPDATE/DELETE（`memory_extractor.rs`，maintenance-guide 模块地图） |
| 做梦（dream） | 6h 无互动（安静期 `APEIRETH_DREAM_QUIET_SECONDS`） | 记忆合并 + LLM 摘要写回（`dream.rs`） |
| 反思（reflection） | 周期默认 24h（`APEIRETH_REFLECT_PERIOD_HOURS`） | 4 阶段反思写回反思记录（`reflection.rs`） |
| 记忆注入 | 每次对话前 | 反幻觉注入：闭世界证据编号列表 + 禁止 AI 声称记得未提供的记忆（`memory_injection.rs`） |
| 统一时间线 | 每条发言 | OneRing 账本：SSE/Web/Lark/Telegram/CLI 的发言归入同一 continuity 锚点逐条留痕（`onering.rs`，backlog N2 吸收） |

### 2.3 你能控制的

- `APEIRETH_SEED_MEMORY` / `APEIRETH_SEED_DEMO=1`：种子记忆（演示）。
- 记忆可浏览/审计：`audit_log` 工具（append-only 留痕查询，masked 脱敏不还原）。

## 3. 工具与审批（能力不失控）

### 3.1 基地自带工具（真实注册清单，来源：`tool_bridge.rs:380-431` + `with_goals`）

`recall_memory` / `save_memory`（记忆读写）、`WebSearch` / `WebFetch` / `Grep` / `Git` / `FileOperator` / `ShellExec`（基地 4 真工具系）、`propose_capability`（能力提案）、`simulate` / `forecast`（预测机沙盘 + 可证伪预测）、`audit_log`（审计）、`save/list/verify_experience`（经验库）、`propose/approve_principle`（动态原则）、`goal_create/status/complete/pause/block`（长目标，需 `with_goals` 接线）。

### 3.2 审批怎么运作（用户视角，真实机制）

1. **日常包默认放行**：只读 + 记忆写类 9 工具永久免审（`packs.rs:149-164`）。
2. **白名单**：16 个内置工具直接放行（`tool_bridge.rs:438-455`）。
3. **风险类要主人批准**：`system/network/file/shell/exec/patch/task` 类调用 → 产生一条**授权请求**（`apreq-*`，同参数去重）→ **前端轮询 `GET /v1/apeireth/approval-requests` 展示 + 一键批准**——这就是「主人批准」的真实载体，没有弹窗，就是前端页面上的批准操作（`approval_requests.rs`，maintenance-guide 模块地图）。
4. **签包免审**：你可以预签权限包（一次强确认 → 有效期内不限次）；`APEIRETH_GRANT="工具:小时"` 可启动即授权（maintenance-guide §四）。
5. **完整闸门链**（8 闸：主权/洋葱/宪法硬门/动态原则/LLM 评审/权限包/审批规则/执行）见 [plugin-authoring-guide.md](plugin-authoring-guide.md) §2.1。

### 3.3 动态原则（你批准的规矩）

AI 可提案原则（`propose_principle`），你用 **master token**（companion_serve 的 `APEIRETH_MASTER_TOKEN`）批准（`approve_principle`）后成为执行期拦截规则——AI 永远不接触 master token（team-work-doc §1.3 洋葱安全）。

## 4. 主动陪伴（涌现）

基地会**主动**开口，但受三层门控（来源：`organs.rs` 模块头，诚实标注原文）：

- **是否开口**：情绪（consciousness，情绪很低时不出声）/ 审议（council 7 强制 Advisor 多视角审议，裁决拒绝则不开口）/ 演化（evolution，主动策略连续被忽略则退回 Draft——「我不该这么频繁」）。
- **怎么开口**：`tone()` 三层合成——关系基线（Bond）× 情绪语气（7 档确定性映射）× 审议措辞强度（坚定/从容/留余地/克制 4 档）。
- **边界（Boundaries）**：涌现循环受边界约束（`emergence.rs`）。
- 送达节流：`ThrottledUtterance`（最短间隔 `APEIRETH_MIN_LLM_INTERVAL_SECS`，默认 60s）。

## 5. 送达通道

| Sink | 说明 | 配置 |
|---|---|---|
| `ConsoleSink` | 控制台输出 | 默认 |
| `LarkSink` | 飞书 IM | `APEIRETH_LARK_APP_ID/SECRET/RECEIVE_ID`（可选 `BASE_URL`） |
| `TelegramSink` | Telegram Bot API | `APEIRETH_TELEGRAM_BOT_TOKEN/CHAT_ID` |
| `BroadcastSink` / `MultiSink` | 多通道广播/组合 | 代码装配（`daemon.rs:389-446`） |

**出站隐私**：送达前经 `apeireth_guard::redact_text` 脱敏（Mask 策略）（`daemon.rs:476` + guard crate pii/redactor：邮件/电话/SSN/卡号/IP/带凭据 URL/7 类密钥前缀/敏感 env 键值 8 类检测）。

## 6. 安全（不堆关键词，靠机制）

安全 = 能力限制 + 洋葱门 + 宪法评审 + 主人批准 + 熔断（team-work-doc §1.1）：

- **洋葱门**（SecurityGate）：风险级别裁决先于执行。
- **宪法硬门**（ConstitutionGate）：编译期 15 条规则表，零 token 成本，LLM 评审前拦截（`constitution_gate.rs`）。
- **宪法评审**（真 LLM）：Medium+ 风险按 E 层原则判案；评审失败 → 保守拒绝（不放过）。
- **主权熔断**（SovereigntyGate）：违规累计 → 冻结循环，全工具拒绝。
- **执行体隔离**：高危工具 per-call 子进程执行（exec_worker，Windows Job Object 加固）。
- **溢出保护**：超大工具输出 spill 到私有文件，不撑爆上下文。

## 7. 套件与生态

三件套（教育/渗透/预测机）与三档装配见 [capability-packs.md](capability-packs.md)；社区插件见 [plugin-authoring-guide.md](plugin-authoring-guide.md)。

## 8. 未接清单（0 装 PASS，用户视角）

| 项 | 状态 |
|---|---|
| LLM 动态措辞（tone::ToneRefiner） | trait 口已留，实现未接；渲染层现用 Bond 静态语调 + tone() 三层确定性合成（`organs.rs` 头部诚实标注） |
| Windows Hello / YubiKey 真生物识别签包 | 机制口已备（`hello.rs` 检测 + 绑定 trait），真绑定是平台 SDK 下一步（`packs.rs` 头部） |
| 沙盒物理层参数化（Sandboxie/landlock） | Layer 2 运行时隔离已生效；参数化是 B3 任务范围（suites.toml packs.sandbox note） |
| AI 能力「部署→监控→回滚」完整闭环 | 提案→评审→批准→激活已落地；后半段见 capability/deploy 现状（team-work-doc §4 A1） |
| 预测机数据源 adapter trait | 未接；现数据源 = 工具插件形态（plugin-authoring-guide §5.1） |
