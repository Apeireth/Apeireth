# v1 → v2 未吸收功能研究（2026-08-27）

> **现状 (2026-08-27)**：本文盘点 v1 legacy 86 crate 中**v2 工作区（15 active crate）未吸收或仅部分吸收**的功能，按不可或缺性分级（A/B/C/D = S1 关键 / S2 重要 / S3 一般 / S4 边缘），并给出每个的 v2 移植方案。**这不是 v1 形态平移**——按用户要求"重构版为了架构优化，不做 v1 一样的"。

```
[Document-Meta]
Document:        docs/04-internal/v2-unabsorbed-features.md
Version:         Design-1.0 (v2 首发)
Last-Modified:   2026-08-27
Status:          🟢 活跃 (v1→v2 未吸收功能盘查)
```

> 给谁看：架构决策者 + ROADMAP 维护者 + v2 实现者。
> 读法：先读 §0 范围 + §1 分级原则 → §2 A 类（S1 关键）逐项细看 → §3 B/C/D 类按优先级扫 → §4 移植路线图 → §5 待决策。
> 与 `scene-d-v2-plan.md` 关系：场景 D 已规划 3 例（主人偏好/自我诊断/多 agent 互审），本文**不重复**场景 D 范围；场景 D 例 3（多 agent 互审）直接消费本文 A1/A2 的 council + team-lead 实现。

---

## §0 范围

- **盘点对象**：`legacy/donor/` 77 crate + `legacy/archived/` 15 + `legacy/frozen/` 13
- **对照对象**：v2 工作区 `crates/{foundation/{core,protocol,plugin,governance,credentials}, engine/{runtime,provider,storage,memory}, capabilities/tools, adapters/{gateway,cli,sdk}}`
- **跳过**：已被 ROADMAP §4 P0/P1/P2/P3 覆盖的（governance 接线、credentials 接线、core drain、M1B 记忆移植）；纯 v1 内部工具（v1 集成测试、verify crate 等）
- **逐项核实**：本批对每个 S1/S2 项都读过 `legacy/donor/<crate>/src/lib.rs` 顶部 doc comment + 关键模块声明，引用附文件路径

---

## §1 分级原则

| 级别 | 含义 | 决策 |
|---|---|---|
| **A = S1 关键** | v1 哲学/愿景明文提到；v2 不吸收**长程任务跑不通**或**核心哲学层落地不了** | 必须进 v2 roadmap（具体 P 号见 §4） |
| **B = S2 重要** | 功能性组件；缺了 v2 能力下降但能跑 | 评估后进 roadmap |
| **C = S3 一般** | 实用工具；缺了 v2 用户体验下降但不影响核心 | 排到 P-arch 之后或独立 minor 版本 |
| **D = S4 边缘** | 特定场景才用；可推迟 | 文档保留指针，不移植 |

**核心判定问题**：
1. v1 哲学层（8 锚 + 13 键 + 三洋葱 + L0 HA + Self-Disable）落地是否依赖此功能？
2. v2 主链（gateway + canonical agent loop）能否承载完整长程任务（>3 轮，含多 agent / 多模态 / 长程评估）？
3. 用户拿 main 跑任务时，会因为缺这个功能而**拿不到 v1 能拿到的结果**吗？

---

## §2 A 类（关键）— 必须移植

### A1. apeireth-council — 7 强制 Advisor + 按住机制 + 多意见加权 synthesis

**v1 实现**：`legacy/donor/apeireth-council/src/lib.rs`（已实装，162 行 + 7 Advisor 子模块 + 30+ tests）

**职责**（v1 实测）：
- **7 强制 Advisor 领域**：safety / performance / philosophy / history / strategy / ethics / legal
- **3 生命周期**：persistent（常驻）/ ephemeral（临时）/ dynamic（按需生成）
- **按住机制**（per `docs/stage4/stage4-correction-v15-four-gates-permission-grant.md`）：
  - **30% 强反对**：≥30% Advisor 反对即 veto
  - **一致反对**：所有 Advisor 一致反对即 reject
  - **60s 裁决超时**：达不成 consensus 转 L0 HA 人工
- **多意见加权 synthesis**：每个 Advisor 给 verdict + 权重，runtime 综合
- **拟人化**：每个 Advisor 独立 session + persona + 立场 + 可辩论 3 轮
- **集成**：`SovereigntyHook` trait 与 `apeireth-sovereignty` 接口

**v2 现状**：**完全缺位**。

**v2 缺失的影响**：
- v2 治理 = external hook 闸（确定性字符串规则）—— 处理不了"伦理上对不对"
- 单 action 安全闸 = L1；多角度评审 = L3（场景 D 范畴但**粒度不同**于例 3 的多 agent 互审）
- council 适用**单次决策的多领域评审**；场景 D 例 3 适用**长程任务的多角色协作**——两个不同的 L3 用例

**v2-native 设计要点**（与场景 D 例 3 复用同一新 crate `crates/foundation/orchestration/`）：
- 7 Advisor **不**各自调独立 LLM（成本太高）—— 改用**多 instance 复用**：同一 provider 不同 model/temperature，7 个 system prompt 模板
- 按住机制实现为 `GovernanceDecision` 的扩展 4 态：Deny / Veto（30% Advisor 反对）/ Reject / DeferToHuman（60s 超时）
- Council 决策入 `Decision::reason` 字段，引用 13 键哲学标签（"violates NotSafe under S"，per `VERDICT_KEYS_BY_PRINCIPLE`）
- 多意见加权：每个 Advisor 输出 `AdvisorVerdict { stance, confidence, evidence }`，runtime 用 confidence 加权投票

**架构选择**：
- ✅ **不**单独开 `crates/engine/council/`—— 进 `orchestration` crate（与 team-lead 一起）
- ✅ **不**让 council 决策"通过/拒绝"硬动作—— 只返 opinion 进 governance pipeline
- ❌ **不**复制 v1 的 persona session 体系（v1 用独立 session，v2 简化成多 instance 复用）

### A2. apeireth-team-lead — 14 调度工具 + worktree 集成

**v1 实现**：`legacy/donor/apeireth-team-lead/src/lib.rs`（759 行 + 14 工具 + 8 bench + 5 fixtures）

**职责**（v1 实测）：
- **8 调度工具**：spawn_agent / send_to_agent / get_agent_output / wait_agent_idle / wait_agent / get_agent_status / list_agents / cancel_agent
- **3 worktree 工具**：get_task_info / check_merge / merge_worktree（v2 可砍，见下）
- **3 感知工具**：list_sessions / get_session_summary / search_sessions
- **Orchestrator trait** + 5 类错误（TeamNotFound / SpawnFailed / MidTaskFailed / ToolUnauthorized / HandoffFailed）
- **7 类 AgentStatus**：Idle / Spawning / Running / Idle / Failed / Cancelled / Done
- **3 类 AgentRole**：Planner / Implementer / Reviewer
- **SUPERVISOR_PROMPT 818 行**（v1 编译期嵌入 .md 文件）

**v2 现状**：**完全缺位**。场景 D 例 3 设计里"orchestrator 调度 subagent"是空话——没有 team-lead 实现，orchestrator 没法启动子 agent。

**v2 缺失的影响**：
- 场景 D 例 3（多 agent 互审）= 不可实现
- 场景 D 例 1（主人偏好持久化）可以用单 instance，但多 agent 是**真正的长程任务必备**

**v2-native 设计要点**（进 `crates/foundation/orchestration/`）：
- 8 调度工具的 v2 等价物：直接用 `Orchestrator::dispatch(SubagentSpec)` trait 调用，不暴露 14 个独立 tool（v1 把 orchestrator 当 tool 暴露给主 agent，v2 当 runtime 内部子系统）
- worktree 工具 **砍**（v2 不依赖 worktree 概念——单进程 gateway + adapter；v1 worktree 是为了多 reviewer 并行写代码）
- 感知工具 **移** 到 `crates/engine/memory`（list/get/search sessions 是 memory 的标准接口）
- Orchestrator trait 7 类状态 + 5 类错误 1:1 移植（这部分 v1 设计清晰）
- SUPERVISOR_PROMPT 砍——v2 prompt 由 orchestrator 动态构造（按 spec 拼装），不编译期嵌入

**架构选择**：
- ✅ 进 `orchestration` crate（与 council 一起）
- ✅ Orchestrator **不**对外暴露为 Tool（v1 错误地让主 agent 自己起子 agent——安全风险；v2 orchestrator 是 runtime 内部子系统）
- ✅ SubagentSpec 是结构化输入（per scene-d §3 决策）—— JSON schema 强约束

### A3. apeireth-perception — 多模态输入 + Attention 策略

**v1 实现**：`legacy/donor/apeireth-perception/src/lib.rs`（194 行 + 5 PerceptionInput + 2 Attention + 5 PerceptionChannel + PerceptionEvent）

**职责**（v1 实测）：
- `PerceptionInput` trait + 5 种输入：Text / Voice / Vision / Tactile / Command
- `Attention` trait + 2 种策略：TopK / Threshold
- `PerceptionChannel` trait + 5 通道（一对一对应输入类型）
- `PerceptionEvent`：cognition 的统一输入格式

**v2 现状**：**完全缺位**。v2 工作区只有 Text 输入（CLI body / HTTP body）。Voice/Vision/Tactile 没有。

**v2 缺失的影响**：
- 你设计 v2 长期愿景 = "Apeireth 是用户的伙伴，跨模态陪伴"——文本不是唯一通道
- 桌宠前端（companion-desktop）的 voice_session / screen_perception 留 legacy，没接 v2 runtime
- 没有 perception，cognition 的 Attention 机制也没法跑（legacy cognition 依赖 PerceptionEvent 输入）

**v2-native 设计要点**（进 `crates/engine/perception/`，与 runtime 同级）：
- **不**实现 Voice/Vision/Tactile 的完整 modality——v2 alpha 不阻塞，但要**Trait + 1 个 Text 实现**，Trait 接口预留
- `PerceptionInput::from_text(str)` 立即可用，voice/vision 是 forward-declared trait method（返回 NotImplemented）
- `PerceptionEvent` 沿用 v1 设计（统一格式），作为 runtime 的输入
- Attention 策略：`TopK(n)` 接收 PerceptionEvent stream → 选 top-n 进 runtime
- **0 装 PASS**：v2 alpha 只实现 Text modality，其他 4 种 trait method 显式 `unimplemented!()` + doc 标注

**架构选择**：
- ✅ `crates/engine/perception/` 新 crate——与 runtime 同级（perception 是输入层，runtime 是执行层）
- ✅ Trait-based，前向兼容，alpha 只 Text impl
- ❌ **不**复制 v1 完整 modality——v2 设计哲学是"先骨架后填肉"

### A4. apeireth-memory-extensions — 持久化记忆后端扩展

**v1 实现**：`legacy/donor/apeireth-memory-extensions/src/lib.rs`（文件存储 / MongoDB / SQLite 多后端 provider）

**职责**（v1 实测）：
- 3 种后端 provider：file / MongoDB / SQLite
- 持久化 apeireth-memory 的 domain 数据（Episode / Session / IdentityCard）
- Per-adapter trait 抽象

**v2 现状**：**仅 SQLite**（`crates/engine/storage`）。

**v2 缺失的影响**：
- 不致命——SQLite 是 99% 场景的合理选择
- 但 enterprise 用户需要 MongoDB（多 agent 跨服务共享）/ 文件存储（边缘部署）
- ROADMAP §3 P1 已隐含此需求（"apeireth-credentials 接 keyring/encrypted file"是类似模式）

**v2-native 设计要点**（扩展 `crates/engine/storage`，**不开新 crate**）：
- 在 storage crate 加 `MemoryBackend` trait（已有 SQLite impl，加 trait abstraction）
- `FileMemoryBackend` 实现 trait（keyring 加密 + AES-GCM）
- MongoDB 实现 trait（需要 mongo crate dep，体积大——**可选**）
- 13-crate 拓扑不破

**架构选择**：
- ✅ 进 `crates/engine/storage`（已有 SQLite，加 trait）
- ✅ 默认 impl = SQLite（向后兼容）
- ✅ File impl 必做（keyring 接入顺带做了）
- ❌ MongoDB 推后（不属于 P1-P3）

### A5. apeireth-tool-runtime — 工具调用解析与执行

**v1 实现**：`legacy/donor/apeireth-tool-runtime/src/lib.rs`（含 `ToolCallParser` + `ParsedToolCall` + `<<<[TOOL_REQUEST]>>>` marker 协议）

**v2 现状**：**完全缺位**。v2 工作区没有 `ToolCallParser`——因为 v2 走 provider 原生 `tool_calls` 协议（OpenAI tool_calls 数组），不解析 marker。

**v2 缺失的影响**：
- 当前零影响（v2 走 OpenAI/Anthropic 原生协议）
- 但 v2 的 `tool.fetch` / `tool.shell` 等需要解析模型输出中的 tool_calls 参数——这部分 v2 有，但没有统一的 `ParsedToolCall` 类型
- 边界：如果以后加新 provider（如自定义 JSON 协议），需要统一解析层

**v2-native 设计要点**：
- **不**复用 v1 `<<<[TOOL_REQUEST]>>>` marker（v1 marker 已被 v2 文档标注"deprecated"）
- 走 v2 已有的 `NormalizedTool::function(name, desc, schema_json)` — provider 转协议自动处理
- v2 缺的是 `ParsedToolCall` 统一表示：`(tool_name: CapabilityId, args: serde_json::Value, source: provider_name)`
- 放在 `crates/foundation/protocol/src/canonical/`（与 `NormalizedTool` 同级）

**架构选择**：
- ✅ 复用 `NormalizedTool` 协议层（已存在）
- ✅ 加 `ParsedToolCall` 结构（纯数据 + parse 辅助 trait）
- ❌ **不**保留 `<<<[TOOL_REQUEST]>>>` 解析——v1 协议作废

### A6. apeireth-tool-approval — 工具调用审批 v1 模型

**v1 实现**：`legacy/donor/apeireth-tool-approval/src/lib.rs`（5 规则：blacklist/trust/risk/frequency/whitelist + ApprovalDecision enum）

**v2 现状**：**部分缺位**。v2 有 `PermissionGovernanceHook`（upstream `873d2857`），但**机制不同**：
- v1 = 5 规则 ApprovalManager（blacklist/whitelist/risk/frequency/trust）
- v2 = PermissionPolicy（grant/require_approval/deny 列表）

**v2 缺失的影响**：
- v1 的 blacklist（黑名单工具永远拒）和 whitelist（16 个核心工具免审批）v2 没有直接等价——v2 PermissionPolicy 只看 grant 列表
- frequency 限制（"同一工具 5 分钟内只能用 N 次"）v2 没有
- trust 等级（不同工具不同 trust）v2 没有

**v2-native 设计要点**（扩展 `crates/foundation/governance`）：
- 在 `PermissionPolicy` 上加**频率限制**（per-capability `RateLimit { per_minute, per_hour }`）
- 加 **blacklist** 字段（不可逆，Policy 装配时静态绑）
- 加 **trust tier** 到 `CapabilityDescriptor::with_metadata("trust_tier", "low/medium/high/critical")`
- 现有 whitelist 行为保留在 PermissionPolicy 的 default grant 列表里

**架构选择**：
- ✅ 扩展现有 `PermissionPolicy` struct（不破坏 v2 接口）
- ✅ frequency/blacklist 是 v2-native 增强，**不**复制 v1 ApprovalManager（机制不同但等价能力）
- ✅ trust_tier 通过 `CapabilityDescriptor::with_metadata` 表达（不新增字段）

---

## §3 B 类（重要）— 评估后进 roadmap

### B1. apeireth-experience — 经验沉淀（提炼回流）

**职责**：从对话/动作/结果提炼经验条目 → 写入记忆 → 检索时按"经验是否相关"加权注入。

**v2 现状**：**仅 primitive**（`crates/engine/memory` 有 Episode 类型）。提炼、评分、注入管线未实现。

**B1 vs 场景 D 例 1（PreferenceStore）**：不同。PreferenceStore = 主人偏好；Experience = 客观经验（"上次这样做过，结果是 X"）。两者都进 memory crate，但分开。

**v2-native 设计**：进 `crates/engine/memory/src/experience.rs`（与 Episode 平级）。要 P3。

### B2. apeireth-state — 全局状态机 + 持久化

**职责**：会话级 + 跨会话级状态（"对话到达哪一步"、"主人在哪个场景"）。v1 用状态机管理。

**v2 现状**：**Session lifecycle 已有**（`crates/engine/runtime/src/canonical/session.rs`），但**全局状态机没有**。

**v2-native 设计**：进 `crates/engine/memory/src/state_machine.rs`。P3。

### B3. apeireth-cognition — Attention + 主动提议

**职责**：v1 的 cognition 是"AI 在什么时候主动开口"的机制（E7 emergence loop）。v2 砍了。

**v2 现状**：**缺位**。

**B3 vs 场景 D**：场景 D 是"长程任务里 AI 评审自己"，cognition 是"AI 主动 vs 被动响应"。两者不同。

**v2-native 设计**：进 `crates/engine/orchestration/cognition.rs`（与 council / team-lead 同 crate）。P6。

### B4. apeireth-sovereignty — HA 多签 + 物理隔离

**职责**：v1 sovereignty = L0 人类批准 + M-of-N 多签 + physical isolation check。v2 governance 的 `HumanAuthority` 部分覆盖。

**v2 现状**：**部分覆盖**（`onion.rs::HumanAuthority` 有 HA mode + RealHuman，但 M-of-N 多签未实现）。

**v2-native 设计**：扩展现有 `HumanAuthority` + 加 `MultiSign { required: u8, total: u8 }` 配置。P1（与 credentials 接线同批）。

### B5. apeireth-supervisor — 进程级监督树

**职责**：进程级（不是 AI agent 级）supervisor 树——5 sub-supervisor + RestartStrategy（"子进程崩溃就重启"）。

**v2 现状**：**缺位**。v2 只有 `ProcessExecutor`（单进程执行边界），没有 supervisor。

**v2 缺失的影响**：
- v1 是为了让 daemon 进程不崩
- v2 没有 daemon 进程（gateway / cli 是 short-lived 启动），supervisor 的需求低
- ROADMAP §4 P5 = "ProcessSupervisor"——已有此需求排期

**v2-native 设计**：进 `crates/capabilities/tools/src/process/supervisor.rs`（与 ProcessExecutor 同 crate）。P5。

### B6. apeireth-action — 动作空间 + 决策选择

**职责**：v1 action 模块 = "AI 在每个决策点选哪个动作"。与 motivation + cognition 联动。

**v2 现状**：**决策点在 `crates/engine/runtime/src/canonical/execute.rs`**（单 instance 编排）。没有显式 action 抽象。

**v2-native 设计**：进 `crates/engine/runtime/src/canonical/action.rs`（可选 — 如果 runtime 现在已经清晰，可不引入新抽象）。P-arch 后评估。

### B7. apeireth-eval — 评估 harness

**职责**：v1 eval = 系统化评估 harness（per-task pass/fail、regression suites）。

**v2 现状**：**Cargo test 部分覆盖**（cargo nextest + 各 crate tests/）。

**v2 缺失的影响**：
- cargo test 是单元/集成测试，不是 task-level eval
- 如果要"eval the assistant" 需要新 harness（eval suite per task）

**v2-native 设计**：新建 `crates/engine/eval/`——独立 crate。P-arch 后。

---

## §4 C/D 类（一般/边缘）— 简表

### C 类（一般，推后或独立 minor）

| crate | 职责 | v2 处理 |
|---|---|---|
| `apeireth-acp` | Agent Communication Protocol（Agent 间 IPC） | 场景 D 例 3 的 JSON 协议覆盖，不单独移植 |
| `apeireth-tool-filesystem` / `tool-shell` / `tool-search` / `tool-fetch` / `tool-codesearch` / `tool-browser` / `tool-image-gen` / `tool-image-process` | v1 分散工具 crate | v2 已合并到 `crates/capabilities/tools`，shell/fetch opt-in（已做） |
| `apeireth-eval` | 见 B7 | P-arch |
| `apeireth-wiki` | Markdown 知识库 | 独立 minor 版本 |
| `apeireth-i18n` | 国际化 | 独立 minor 版本 |
| `apeireth-experience` | 见 B1 | P3 |
| `apeireth-state` | 见 B2 | P3 |
| `apeireth-cognition` | 见 B3 | P6 |
| `apeireth-environment` | 环境变量管理 | 进 storage crate，P3 |
| `apeireth-context-fold` | 上下文折叠（长上下文管理） | 进 runtime，P3 |
| `apeireth-pipeline-g5` | 通用 5 阶段 pipeline | 场景 D 例 3 用，合并到 orchestration |
| `apeireth-blueprint-impl` | 蓝图实施 | 内部使用 |
| `apeireth-graph-primitive` | property graph primitive | 进 memory crate，P3 |
| `apeireth-vector` | 向量存储 | 进 memory crate，P3 |
| `apeireth-graph` | 图查询 | 进 memory crate，P3 |

### D 类（边缘，文档保留指针）

| crate | 职责 | v2 处理 |
|---|---|---|
| `apeireth-livekit` | LiveKit 实时通信 | 独立 minor，与 voice/companion-desktop 一起 |
| `apeireth-voice` | TTS/voice | 场景 D P7 |
| `apeireth-perception` Tactile/Vision | v2 alpha 不实现 | A3 trait 已留口 |
| `apeireth-bench` | 性能基准 | CI 内置，独立 minor |
| `apeireth-tool-image-gen` / `tool-image-process` | 图像处理 | 进 `tools` opt-in |
| `apeireth-pybridge` | PyO3 桥 | 0 装 PASS：v2 纯 Rust 不移植 |
| `apeireth-rate-limiter` | 限流 | 进 governance，B6 信任等级扩展时顺带做 |
| `apeireth-config` | 配置管理 | 进 `crates/engine/storage` 或单独 small crate |
| `apeireth-integration-e2e` | e2e 测试 | 进 tests/ |
| `apeireth-arbitration` | HASH-SQL 仲裁 | 关键功能！进 `crates/engine/memory/src/arbitration.rs`（与 audit hash chain 一致） |
| `apeireth-consciousness` | Cognitive-Dream 6 状态机 | 升级为 scene-d 例 2 SelfAssessmentCache 触发 |
| `apeireth-naming-v05` | 命名 V0.5 | 不移植——v2 走 kernel::ids |
| `apeireth-version` / `verify` / `upgrade` / `release-tools` | 版本/构建工具 | 独立工具链 |
| `apeireth-experience` | 经验沉淀 | 见 B1 |
| `apeireth-apeiron-lifecycle` 等 | 进化循环 | 见 B3 |
| `apeireth-test` / `apeireth-tui-e2e` | 测试工具 | 进 tests/ |

---

## §5 移植路线图（与 ROADMAP §4 对齐）

| 优先级 | 项 | 工作量 | 路线 |
|---|---|---|---|
| **P0 ✅ 已完成** | upstream `873d2857` governance 接线 | — | — |
| **P1** | A4 MemoryBackend trait (File impl + SQLite 增强) + B4 sovereignty 多签扩展 + core drain + credentials 接线 | 2-3 周 | 进 `crates/engine/storage` + `crates/foundation/governance` |
| **P2** | 13 键降级为哲学标准（已完成，2026-08-27 5 维分析拍板）+ scene-d 设计 + ROADMAP 同步 | 0（完成） | — |

**13 键降级拍板记录 (2026-08-27)**：5 维评分（1=强烈支持降级, 5=强烈支持接线）
- 安全性 **1** — self-introspection 是"AI 评 AI"路径, 被 prompt injection 影响后 verdict cache 同样被污染; external hook 0 模型参与, 0 污染路径
- 延迟 **1** — verdict cache lookup O(1) 命中, 但 cache miss 调 LLM O(seconds); hook 字符串匹配 O(μs); 6 数量级差
- 正确性覆盖 **2** — 13 键哲学层 vs hook 输入侧有少量互补, 但 v2 治理"走外部不靠 AI"已划走边界
- 审计/可观测 **2** — verdict cache append-only 可重放, hook decision 有 reason 字符串, 两者相当
- 场景 D 互补 **1** — 例 1 主人偏好 + 例 2 SelfAssessmentCache + 例 3 多 agent 互审已覆盖 self-introspection 所有应用场景; 13 键接进是边际冗余
- 加权 0.28/5 → **降级** (保持 L2 哲学标准, RUNTIME_ENFORCED=false 永久)
- 13 键仍用于: hook deny reason 引用 + CapabilityDescriptor risk 分级 + 哲学语义定义
| **P3** | A6 tool-approval frequency/blacklist + B1 experience + B2 state machine + C 类 memory 周边 | 3-4 周 | 进 `crates/engine/memory` |
| **P4** | A3 perception trait + Text impl（5 modality 的 forward-declared）| 1 周 | 进 `crates/engine/perception`（新 crate）|
| **P5** | A5 tool-runtime ParsedToolCall + B5 process supervisor + scene-d 例 2 (SelfAssessmentCache multi-instance) | 3-4 周 | 进 `crates/foundation/protocol` + `crates/capabilities/tools` + `crates/foundation/orchestration` |
| **P6** | A1 council + A2 team-lead + B3 cognition + scene-d 例 3 (orchestrator + 多 agent 互审) | 6-8 周 | 进 `crates/foundation/orchestration`（新 crate）|
| **P-arch 后** | B6 action / B7 eval / D 类 arbitration 等独立评估 | — | 独立 minor 版本 |

**总 v2 移植工作量**：15-23 周（约 4-6 个月一人），与 ROADMAP §4 P1-P6 整体时间线对齐。

---

## §6 与场景 D / ROADMAP 的关系

| 文档 | 关系 |
|---|---|
| `scene-d-v2-plan.md` | 场景 D 已规划 3 例（主人偏好/自我诊断/多 agent 互审），本文**不重复**——但本文 A1 council + A2 team-lead 是例 3 的实现基础 |
| `ROADMAP.md §4` | 本文 §5 的路线图对齐 ROADMAP P-arch/P1-P6；A1-A6 + B1-B7 给每条 P 填具体内容 |
| `v2-unabsorbed-features.md`（本文）| 是"哪些 v1 功能 v2 没吸收 + 为什么"的清单；scene-d 是"v2 怎么实现场景 D" |
| `legacy/donor/` | 全部保留作参考代码（per README v1.0 入口）；本文为每个未吸收项指向 legacy 源路径 |

---

## §7 待拍板决策

| # | 项 | 选项 | 我的推荐 |
|---|---|---|---|
| 1 | A4 MemoryBackend trait 形态 | (a) trait + SQLite impl（向后兼容）/ (b) enum-dispatch 静态分发 | (a) — trait 更利于 v2 plugin 架构 |
| 2 | A3 perception alpha 实现深度 | (a) 仅 Text + 4 trait forward-declared / (b) Text + 简单 Voice (audio file 解析) | (a) — P4 不阻塞 alpha；Voice 等 P7 |
| 3 | A1 council Advisor 是否每个独立 LLM 调用 | (a) 是（严格隔离）/ (b) 多 instance 复用（同 provider 不同 model/temperature） | (b) — 成本可控 + 隔离足够 |
| 4 | A2 team-lead worktree 工具 | (a) 完全砍 / (b) 砍但留 trait 扩展位 / (c) 全移植 | (b) — v2 alpha 不需要，但未来 P-arch 评估 |
| 5 | A6 frequency limit 触发粒度 | (a) 进程级 / (b) 用户级 / (c) Capability 级 | (c) — 粒度最细，避免误伤合法用例 |
| 6 | A1 council 决策与 governance Decision 关系 | (a) council 输出新 GovernanceDecision 4 态扩展 / (b) council 输出 Allow/Deny，runtime 转 4 态 | (a) — 清晰分层 |
| 7 | A4 File impl 加密 | (a) keyring + AES-GCM / (b) 纯 AES-GCM（keyring 是可选密钥源）/ (c) 不加密（文件系统权限） | (a) — 与 credentials 接线同路径 |

---

_本文 v2 首发 (2026-08-27) + rc.1 进展更新 (2026-08-27 收盘, HEAD `67c06d95`)：基于对 77 个 legacy/donor crate + 15 archived + 13 frozen 的 doc comment + 关键模块声明实测盘查（非子代理推理）。结论：v2 13-crate 工作区**缺 6 类 A 级功能 + 7 类 B 级 + 14 类 C 级 + 14 类 D 级**。A 级（council/team-lead/perception/memory-extensions/tool-runtime/tool-approval）必须进 v2 roadmap，P-arch / P1 / P3 / P5 / P6 排期落地。v2-native 设计原则贯穿：多 instance LLM 隔离（v1 全独立 session 太重）/ JSON 协议（v1 free text）/ 单一事实源 plugin registry（v1 多 registry）/ trait-based 多态（v1 if-else 散落）。**0 漂移承诺**: 9 哲学锚（升 8→9, 加 O-6 永远追求最优）/ 13 键 LOCKED 数据 / 3 项不可变脊柱语义 / workspace.version 0 改；本文新增 `crates/foundation/orchestration/`（P6）+ `crates/engine/perception/`（P4）+ 各 trait 扩展，**不**触碰 LOCKED public API。

**P-arch + RC 进展 (2026-08-27 收盘, HEAD `67c06d95`)**:
- ✅ O-6 哲学锚 #9 登记 + 12 项工程化兑现 (5 Refactor + 5 守门 workflow + 文档 + kernel re-export + 统一 error trait)
- ✅ **7/10 RC 真实现完成**: RC-1 MemoryBackend SqliteBackend / RC-2 Experience / RC-3 PreferenceStore / RC-4 SelfAssessmentStore / RC-8 SubSupervisor (改名 `TokioSubSupervisor` → `StdSubSupervisor`, 诚实反映 std::process) / RC-9 keyring / RC-10 File AES-GCM
- ⏳ **3/10 RC 待 LLM API key + 硬件**: RC-5 Orchestrator runtime LLM harness / RC-6 Council multi-LLM + 60s timeout / RC-7 Perception 真 modality (Whisper / screen capture)
- **cognitive module** (其他 dev 推, 3 commit `a699c5f5`/`1d227d6a`/`64e64f46`): hook ABI + 集成 + lifecycle invariants. 不在本工作范围.
- 详见 `docs/04-internal/HANDOFF-NOTES.md` (子代理 D 写, 接手人手册) + `CHANGELOG.md` (RC 进展时间表) + `ROADMAP.md` §3 (当前状态) + `docs/04-internal/v2.0.0-rc-roadmap.md` (10 RC 完整) + `docs/01-architecture/v2-arch-refactor-batch.md` (O-6 5 Refactor + 守门)

**接手人 5 条 actionable advice** (per 子代理 D handoff):
1. 优先做 RC-5/6/7 (需 LLM key), 不要重做 RC-1/2/3/4/8/9/10 (写真完成, 0 改)
2. 哲学锚编号 ledger 待核 (12 vs 23 项, 子代理 A/B 报告), 别重复兑现
3. 12 consumer 弃用迁移 (alpha `#[allow(deprecated)]` → v2.0 release 前删, rc 后必破)
4. RC-10 补 line header AAD tamper 保护 (子代理 C 建议 5)
5. cognitive module 不变量 (其他 dev 推, 看 commits `64e64f46` / `1d227d6a` / `a699c5f5`)

**子代理 4 项报告** (A/B/C/D, 全部采纳):
- A (`5dc29cb`): Send+Sync 注释 (commit `67fc66a0` 自动派生 + 注释说明, 不写 `#[derive(Send, Sync)]` 因 unsafe trait)
- B (`792f5a97`): v1 vs v2 41 项差异 (A6 / B7 / C14 / D14) + 5 风险 (RC-7 硬件依赖 / Council 60s timeout / 12 consumer 弃用 / schema 兼容 / O-6 守门回归)
- C (`9d60deea`): P0 build break (RC-2 untracked) + RC-8 命名错位 (commit `4e4fba89` 修)
- D (`4f56cf5a`): 接手人手册 `HANDOFF-NOTES.md` 11 节_