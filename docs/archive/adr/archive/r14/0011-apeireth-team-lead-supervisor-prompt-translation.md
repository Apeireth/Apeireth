# ADR-0011: apeireth-team-lead 新 crate 命名 + supervisorPrompt 1:1 翻译

```
[Document-Meta]
Document: docs/adr/0011-apeireth-team-lead-supervisor-prompt-translation.md
Version: Manual-Rev-A
R-Cycle: R19+
Commit: <commit 4 份文档时回填>
Last-Modified: 2026-08-05
Status: ✅ A 方案已拍板 (2026-08-05)
```

> 决策: Mavis 默认 A 方案 apeireth-team-lead, 主人 2026-08-05 13:34 拍板采纳。

> **状态**: ✅ A 方案已拍板 (2026-08-05)
> **日期**: 2026-08-05
> **决策者**: Mavis + 主人 + 架构师
> **作者**: technical_writer
> **性质**: 第十一个 ADR — 记录 Apeireth R19+ 通过 1:1 翻译 SpectrAI v0.9.21 `supervisorPrompt.ts` (808 LOC) 实现"团队 leader 给子 agent 的 prompt 构造器"的工程期设计决策, 核心是**新 crate 命名** (A/B/C 三选一) + **`buildAwarenessPrompt` / `buildSupervisorPrompt` + 14 supervisor 工具描述 1:1 翻译** + **不与 `apeireth-supervisor` 命名冲突**。
>
> **依据**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §5.2 第 7 行 + §8 决策清单 #3 + §9.2 + ADR 0010 (`apeireth-mcp` 来自 SpectrAI 翻译) + ADR 0007 (兼容组件层) + ADR 0009 (integration rebase skip 策略) + APEIRETH-CONVENTIONS §9 6 锚穿透。
>
> **约束**: ❌ 不修改任何 LOCKED 文档 / Cargo.toml / crates/ 源码；仅新增命名空间 `docs/adr/0011-*.md` 独立 ADR。

---

## 状态

✅ **A 方案已拍板** (2026-08-05 13:34): 命名 `apeireth-team-lead` 采纳 (Mavis 推荐, 主人拍板); 待 architect 复核 Cargo.toml 加 workspace member。

---

## 背景（Context）

### 关键问题：翻译成什么命名空间的 Rust crate？

主人在 2026-08-04 R19 阶段 3 战略会上拍板：**1:1 翻译 supervisorPrompt.ts，不按 minimax 习惯改写**（保守先，主人自己用 m3 测后迭代）。但**新 crate 叫什么名字**是开放问题，三个候选：

| 选项 | 命名 | 强调点 | Mavis 评估 |
|---|---|---|---|
| **A** | `apeireth-team-lead` | 团队 + leader 角色 | **Mavis 推荐**（跟 `apeireth-supervisor` 进程监督区分清晰） |
| **B** | `apeireth-spectrAI-bridge` | SpectrAI 血统 | ❌ 跟 6 compat 平台"平台+桥接"风格冲突，且锁死来源命名 |
| **C** | `apeireth-spawn-agent` | spawn_agent 工具语义 | ❌ 丢失"leader/team"语义，且 14 工具之一以偏概全 |

**为什么这是问题**：
- 现有 `apeireth-supervisor` crate (550 LOC) 是**进程监督**（PID 1 / 5 sub-supervisor 监控 child actor 生命周期）
- 新需求是**团队 leader 角色**（构造 prompt 告诉子 agent "你是 leader, 用这些工具, 走这套规则"）
- 两者**职责完全不同**（一个是 OS-level 进程管理, 一个是 agent-level 角色提示词）
- 但命名都带"supervisor/lead"关键字, 容易混淆

### SpectrAI `supervisorPrompt.ts` 现状（v0.9.21 事实证据）

| 维度 | 数值 | 引用 |
|---|---|---|
| **文件 LOC** | 808 | `.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\supervisorPrompt.ts` |
| **核心函数** | `buildAwarenessPrompt(context)` + `buildSupervisorPrompt(tools, advisors)` | grep 实测 |
| **14 supervisor 工具** | spawn_agent / send_input / wait_idle / cancel / list_agents / get_status / etc. | 蓝图 §B.2 8 supervisor 工具类 |
| **prompt 风格** | Claude 习惯（XML 标签 / `<system>` / tool_use 描述） | minimax 习惯 vs Claude 习惯有差异 |
| **7 advisor voting 触发** | `buildSupervisorPrompt` 内嵌 advisor 投票触发逻辑 | 蓝图 §5.2 第 7 行 |

### 主人 2026-08-04 R19 战略会拍板事项

| 决策 | 内容 | 影响 |
|---|---|---|
| **1:1 翻译** | 不按 minimax 习惯改写 supervisorPrompt.ts | 保守先, 主人用 m3 测后迭代 prompt 敏感度 |
| **深度集成** | 不做 yinta 那种 patch fork | 直接并入 `apeireth` 主项目, 跟 `apeireth-mcp` 同等级 |
| **1-2 周时间窗** | R19 阶段 3 实施 | 跟 `apeireth-mcp` (R19 阶段 2) 串联 |
| **baseline 不掉** | V1141=0.8682 / V1131=0.8532 / V1136=0.9063 | 翻译后 R-Measure 不能掉 |

### `apeireth-supervisor` vs 新 crate 职责对比（决策核心）

| 维度 | `apeireth-supervisor` (现有 550 LOC) | 新 crate (待命名) |
|---|---|---|
| **职责** | 进程监督（PID 1 / 监控 child actor 退出码 / 重启策略） | 团队 leader 角色（构造 prompt / 调度 sub-agent 工具） |
| **层级** | OS-level（tokio task / 进程） | Agent-level（LLM prompt 字符串） |
| **依赖** | `tokio::process` / `nix` 等 | `apeireth-protocol` / `apeireth-agent` / `apeireth-mcp` |
| **调用方** | 系统启动 / SupervisorManager | 任何要"扮演 leader"的 agent |
| **类比** | systemd / launchd | PM 给团队的需求文档 |

**关键**: 两 crate 命名必须区分, 否则 future maintainer 一看 "supervisor" 就以为是 PID 1, 走错代码。

---

## 决策（Decision）

**正式确立 "`apeireth-team-lead` 新 crate + supervisorPrompt 1:1 翻译" 路径**, 按 5 项硬决策:

### 决策 1: 新建 crate `apeireth-team-lead` (A 方案, Mavis 默认推荐)

> 命名明确"团队 + leader 角色", 跟 `apeireth-supervisor` 进程监督**语义零冲突**。
> 跟 `apeireth-council` 审议庭也清晰区分 (council = 7 advisor voting, team-lead = 1 leader 调度 sub-agents)。

**命名理由（4 条）**:
1. 跟现有"功能 + 角色"约定一致 (`apeireth-supervisor` 监督员, `apeireth-team-lead` 团队 leader)
2. "team" 表征 multi-agent 协作, "lead" 表征 role
3. 不锁死来源 (不像 `apeireth-spectrAI-bridge` 写死 SpectrAI)
4. 不以偏概全 (不像 `apeireth-spawn-agent` 只覆盖 14 工具之一)

**估 LOC**:
- 核心 lib: 600 LOC
- 14 工具 prompt 描述: 200 LOC
- 单元测试: 50 LOC (每工具 1 happy path)
- **合计 ~850 LOC**

### 决策 2: 1:1 翻译 supervisorPrompt.ts 808 行, 不按 minimax 习惯改写

> 主人 2026-08-04 R19 拍板: 保守先, 主人用 m3 测后迭代 prompt 敏感度。

**翻译范围**:
- `buildAwarenessPrompt(context)` 函数全翻译 (估 250 LOC Rust)
- `buildSupervisorPrompt(tools, advisors)` 函数全翻译 (估 350 LOC Rust)
- 14 supervisor 工具的 prompt 描述 (估 200 LOC Rust)
- 7 advisor voting 触发逻辑 (估 100 LOC Rust trait 注入)
- **不翻译**: SpectrAI 自己的 TypeScript 类型声明 / 测试代码 (TypeScript-only)

**翻译原则**:
- ✅ 保留所有 XML 标签 (`<system>`, `<tools>`, `<advisors>` 等)
- ✅ 保留所有 Claude 习惯措辞 ("You are a supervisor agent...")
- ✅ 保留所有 prompt 工程细节 (few-shot examples, edge case 警告)
- ❌ 不按 minimax 习惯改成 "你是 AI 助手..." 风格
- ❌ 不"优化"成中文 (原文是英文, 翻译后保持英文)

### 决策 3: 集成位置 `apeireth-mcp::team` 通过 trait 调 `apeireth-team-lead`

> `apeireth-mcp::team` (ADR 0010 已确立) **不直接 hold prompt 模板**, 而是通过 trait 桥接。
> 这样 14 supervisor 工具的实现 (在 `apeireth-mcp::team`) 和 prompt 构造 (在 `apeireth-team-lead`) **解耦**。

**trait 定义** (估 50 LOC, 放 `apeireth-team-lead/src/lib.rs`):

```rust
pub trait SupervisorPromptBuilder: Send + Sync {
    fn build_awareness_prompt(&self, ctx: &AgentContext) -> String;
    fn build_supervisor_prompt(&self, tools: &[ToolMeta], advisors: &[AdvisorMeta]) -> String;
}
```

**调用方** (`apeireth-mcp::team`):
- 14 supervisor 工具 (spawn_agent / send_input / etc.) 各自实现 `ToolHandler` trait
- 第一次调用某工具时, 调 `SupervisorPromptBuilder` 构造该 agent 的 system prompt
- 后续调用复用已构造的 prompt (缓存到 `Arc<str>`)

### 决策 4: 不与 `apeireth-supervisor` 冲突, 命名空间严格分离

| crate | 职责 | 依赖 | 调用方 |
|---|---|---|---|
| `apeireth-supervisor` (现有 550 LOC) | 进程监督 (PID 1 / 5 sub-supervisor) | `tokio::process` | SupervisorManager / system bootstrap |
| `apeireth-team-lead` (新 850 LOC) | 团队 leader 角色 (prompt 构造) | `apeireth-protocol` / `apeireth-agent` / `apeireth-mcp` | `apeireth-mcp::team` 14 工具 |

**强约束**: `apeireth-team-lead` **不依赖** `apeireth-supervisor` (避免循环依赖风险)。

### 决策 5: 7 advisor voting 触发逻辑通过 trait 注入

> 7 advisor voting 来自 `apeireth-council` crate (尚未实现, R21+ P2 计划)。
> 本 ADR 阶段**不假装 council 已实现**, 而是通过 trait 抽象。

**trait 定义** (估 30 LOC):

```rust
#[async_trait]
pub trait AdvisorVotingTrigger: Send + Sync {
    async fn should_trigger_vote(&self, agent_state: &AgentState) -> bool;
    async fn collect_votes(&self, question: &str) -> Vec<AdvisorVote>;
}
```

**默认实现** (本 ADR 阶段):
- `AlwaysTrigger` — 总是触发 (用于 baseline 测试)
- `NeverTrigger` — 从不触发 (用于单元测试)
- `RealCouncilTrigger` — `apeireth-council` 真实 LLM 集成 (R21+ P2 接入)

---

## 后果（Consequences）

### 正面

- ✅ **填 Apeireth LOCKED 0 代码坑** (LOCKED 文档 §14.4 提"prompt 构造"但无 Rust 实现)
- ✅ **`apeireth-team-lead` 850 LOC 端到端可测** (单元测试 50 LOC + 集成测试 50 LOC)
- ✅ **14 supervisor 工具的 prompt 1:1 翻译** (跟 SpectrAI 原文对照可测, prompt 敏感度可量化)
- ✅ **跟 `apeireth-supervisor` 命名零冲突** (两个 crate 职责/层级/依赖完全分离)
- ✅ **跟 `apeireth-council` 7 advisor voting 解耦** (trait 注入, 不阻塞 R19 阶段 3 推进)
- ✅ **保留 `apeireth-mcp` 现有 2135 LOC 架构** (新 crate 独立, 通过 trait 集成)

### 负面

- ⚠️ **失去: 跨 prompt 框架兼容** (SpectrAI 的 Claude 习惯 prompt 不直接兼容 minimax 习惯, 需主人测后迭代)
- ⚠️ **失去: prompt 自动 A/B 测试能力** (本 ADR 阶段 1:1 翻译, 没有 A/B 框架, 后续 R21+ P3 加)
- ⚠️ **新增 crate `apeireth-team-lead` 850 LOC** (依赖树增 1 crate, 架构师拍板 Cargo.toml)
- ⚠️ **m3 minimax 习惯 vs Claude 习惯的 prompt 敏感度未知** (主人决定保守先 1:1 翻译, 但实际效果要测)

### 中和

- 🛡️ **`apeireth-supervisor` 现有 550 LOC 架构完全不动** (新 crate 独立, 无交叉)
- 🛡️ **`apeireth-mcp` 现有 2135 LOC 架构完全不动** (新 crate 通过 trait 集成, 不修改 McpServer)
- 🛡️ **7 advisor voting 触发逻辑抽象为 trait** (本 ADR 阶段不假装 council 已实现, R21+ 接入)
- 🛡️ **Cargo.toml 改动** (加 `apeireth-team-lead` workspace member) **架构师拍板**
- 🛡️ **不修改 LOCKED** (7 项 LOCKED + 8 项附加 LOCKED 全守)

---

## 备选方案（Alternatives Considered）

### 选项 A: `apeireth-team-lead` (Mavis 推荐) ✅

**优点**:
- ✅ 明确"团队 + leader"角色, 一眼能懂是 agent-level role
- ✅ 跟 `apeireth-supervisor` (进程监督) 区分清晰 (team-lead 是 agent role, supervisor 是 OS process)
- ✅ 跟 `apeireth-council` (审议庭) 区分清晰 (council = 7 advisor voting, team-lead = 1 leader 调度 sub-agents)
- ✅ 命名符合现有"功能 + 角色"约定 (`apeireth-supervisor` 监督员, `apeireth-team-lead` 团队 leader)
- ✅ 不锁死来源 (未来从 SpectrAI 演进到其他 prompt 源, 命名不变)
- ✅ 不以偏概全 (不只覆盖 spawn_agent 工具)

**缺点**:
- ⚠️ "team-lead" 在英语里略带管理学色彩 (但 agent 团队协作场景下可接受)

**决策**: **Mavis 默认推荐 A 方案, 等主人拍板**。

### 选项 B: `apeireth-spectrAI-bridge` ❌

**优点**:
- ✅ 强调 SpectrAI 血统, 方便追溯 (1:1 翻译来源清晰)
- ✅ 跟 "bridge" 命名风格部分一致 (`apeireth-pybridge` 也有 bridge 后缀)

**缺点**:
- ❌ 跟 "6 compat 平台" 风格冲突 (`apeireth-api` / `apeireth-mcp` / `apeireth-pybridge` 等都是"平台/协议 + 桥接"风格, 而非"特定来源 + bridge")
- ❌ "SpectrAI" 命名锁死来源, 限制未来演进 (如果未来合并 minimax 自研 prompt 源, 命名尴尬)
- ❌ "bridge" 暗示"跨语言/跨进程桥接" (但本 ADR 是 1:1 Rust 翻译, 不跨语言)
- ❌ 跟 `apeireth-pybridge` 职责混淆 (pybridge 是真的跨语言 PyO3 桥, spectrAI-bridge 不是)

**决策**: ❌ 不推荐 (锁死来源 + 风格冲突)。

### 选项 C: `apeireth-spawn-agent` ❌

**优点**:
- ✅ 强调 spawn_agent 工具语义 (14 工具中最核心的)

**缺点**:
- ❌ 丢失"leader/team"语义 (spawn_agent 只是 14 工具之一, 不能代表整个 crate 职责)
- ❌ 跟 `apeireth-agent` 单数形式混淆 (一个叫 `agent`, 一个叫 `spawn-agent`, 未来维护者搞不清谁调用谁)
- ❌ "spawn_agent" 只是 14 个工具之一, 以此命名过窄 (本 crate 实际管理 14 工具的 prompt 描述, 不只 spawn)
- ❌ 暗示 crate 是"工具实现"而非"prompt 构造" (实际本 crate 核心是 prompt, 工具实现在 `apeireth-mcp::team`)

**决策**: ❌ 不推荐 (以偏概全 + 命名混淆)。

### 选项 D: 复用 `apeireth-supervisor` (加 sub-module) — 没列入 A/B/C 但讨论过

**思路**: 在 `apeireth-supervisor` 下加 `supervisor::prompt` 子模块, 不新建 crate。

**否决理由**:
- ❌ `apeireth-supervisor` 是 OS-level (tokio task / 进程), 加 agent-level prompt 进去破坏单一职责
- ❌ 命名"supervisor"在两个 crate 里含义不同 (PID 1 监督 vs team leader), future maintainer 困惑
- ❌ 跟 ADR 0010 §决策 5 "不复用 apeireth-supervisor" 一致 (那时已经否决过)

**决策**: ❌ 不考虑 (单一职责原则)。

---

## 实施路径（Implementation Path）

| 阶段 | 任务 | Owner | 依赖 | 估时 |
|---|---|---|---|---|
| **R19 阶段 3.1** | 主人对命名 A/B/C 拍板 | 主人 | 本 ADR | 1 天 |
| **R19 阶段 3.2** | 架构师拍板 Cargo.toml (加 `apeireth-team-lead` workspace member) | 架构师 | 主人拍板 | 1 天 |
| **R19 阶段 3.3** | 创建 `crates/apeireth-team-lead/Cargo.toml` + `src/lib.rs` | rust-coder | 架构师拍板 | 0.5 天 |
| **R19 阶段 3.4** | 1:1 翻译 `buildAwarenessPrompt` (估 250 LOC) | rust-coder | 3.3 | 2 天 |
| **R19 阶段 3.5** | 1:1 翻译 `buildSupervisorPrompt` (估 350 LOC) | rust-coder | 3.4 | 3 天 |
| **R19 阶段 3.6** | 14 supervisor 工具 prompt 描述翻译 (估 200 LOC) | rust-coder | 3.5 | 2 天 |
| **R19 阶段 3.7** | 7 advisor voting trait 定义 + 默认实现 (估 100 LOC) | rust-coder | 3.5 | 1 天 |
| **R19 阶段 3.8** | 跟 `apeireth-mcp::team` trait 集成 (估 50 LOC in mcp side) | rust-coder | 3.5+3.7 | 1 天 |
| **R19 阶段 3.9** | 单元测试 50 LOC + 集成测试 50 LOC | rust-coder | 3.8 | 1 天 |
| **R19 阶段 3.10** | R-Measure baseline 验证 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063 不能掉) | technical_writer | 3.9 | 1 天 |
| **R19 阶段 3.11** | 主人 m3 实测 + prompt 迭代 | 主人 | 3.10 | 持续 |
| **合计** | | | | **~13.5 天 (~2-3 周)** |

**关键里程碑**:
- **M1** (3.1): 命名拍板 → 解锁 Cargo.toml
- **M2** (3.5): prompt 核心翻译完成 → 解锁 14 工具描述
- **M3** (3.8): trait 集成完成 → 解锁 R-Measure 验证
- **M4** (3.10): R-Measure baseline 不掉 → R19 阶段 3 收尾

---

## 关键不假装（Key Honesty Points）

- 🔴 **`apeireth-team-lead` 当前 0 代码**（本 ADR 之前；不假装已实装）
- 🔴 **SpectrAI v0.9.21 supervisorPrompt.ts 808 LOC 翻译完成度未知**（主人 m3 测后才知道 prompt 敏感度）
- 🟡 **`apeireth-council` 7 advisor voting 当前 0 代码**（本 ADR 通过 trait 注入默认实现, R21+ P2 真实 LLM 集成）
- 🟡 **m3 minimax 习惯 vs Claude 习惯的 prompt 敏感度未知**（主人决定保守先 1:1 翻译, 但实际效果要测）
- 🟡 **Cargo.toml 改动**（加 `apeireth-team-lead` workspace member）**架构师拍板**（本文档仅记录决策）
- 🟡 **跟 `apeireth-mcp::team` 集成的 trait API 最终形态**（本 ADR 阶段定下大方向, 实施时 rust-coder 跟 mcp side 对齐）
- 🟢 **`apeireth-supervisor` 现有 550 LOC 完全不动**（不假装要改 supervisor, 两个 crate 解耦）
- 🟢 **`apeireth-mcp` 现有 2135 LOC 架构完全不动**（不假装要重写 mcp, 通过 trait 集成）

---

## 不修改承诺

| ❌ 不修改 | 原因 |
|---|---|
| 阶段 1+2+3 LOCKED 文档 | 主人明确沉淀 |
| v2 / v4 / v4.1 LOCKED | 哲学层纲领 |
| 阶段 4 核心文档 LOCKED (`6ca80776`) | 蓝图 §10 已锁 |
| 阶段 5 施工文档 LOCKED (631 行) | 阶段 5 实施时再引用 |
| v6 基础架构 | 主 AI 团队已 LOCKED |
| R11 baseline (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 主人 2026-07-31 明确不动 |
| APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md | 顶层规范 |
| START-CONSTRUCTION.md | 顶层手册 |
| `apeireth-legacy/` | R17 finalize 后归档, 不删 |
| workspace version 1.0.0 (semver 严格) | 不动 |
| 现有 ADR 0001~0010 | 不动 |
| 现有 stage4-* / README.md / CHANGELOG.md | 不动 |
| `apeireth-supervisor` 现有 550 LOC | 本 ADR 阶段零修改 |
| `apeireth-mcp` 现有 2135 LOC | 本 ADR 阶段零修改 |

---

## 哲学 anchor

| 锚 | 来源 | 本 ADR 落地 |
|---|---|---|
| **S-1 主 22:33** | 6 anchor ASI 完整性 | team-lead 是"团队"角色核心, 14 工具 prompt + 7 advisor voting 覆盖 team 协作全场景, 服务 ASI 北极星 |
| **S-2 主 17:43** | 6 anchor 实验室 | 1:1 翻译 supervisorPrompt.ts 808 行, 跟 SpectrAI 原文对照可测, 实验室态度 |
| **O-5 主 17:58** | 6 anchor 12 急救 | supervisorPrompt 包含 P0 mid-task bug 修法的 prompt 描述 (父进程不再"以为成功实际丢消息") |
| **O-2 主 19:33** | 6 anchor 4 分类 | team-lead ≠ supervisor (进程) ≠ council (审议) ≠ agent (单数), 4 分类清晰 |
| **O-3 主 23:44** | 6 anchor 决策清单 | ADR 模板 + A/B/C/D 4 备选方案 + 5 项硬决策 + 实施 11 阶段 + 关键不假装 8 条 |
| **O-4 主 00:56** | 6 anchor 12 统一 | 跟现有 12 子规范统一 (`apeireth-supervisor` 监督员, `apeireth-team-lead` 团队 leader, 同构命名) |

---

## 关联文档

- **前置**: [ADR 0007 兼容组件层](0007-compat-components-layer.md) + [ADR 0009 integration rebase skip](0009-integration-rebase-skip-policy.md) + [ADR 0010 apeireth-mcp 来自 SpectrAI 翻译](0010-mcp-from-spectrai-agentmcpserver.md)
- **蓝图**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §5.2 第 7 行 + §8 决策清单 #3 + §9.2 阶段 3 路线
- **Tauri 资产**: `docs/stage4/tauri-assets-from-spectrAI-2026-08-05.md`
- **架构集成**: `ARCHITECTURE.md` §5.2 集成映射表 (team-lead 行待补)
- **源码引用**: `supervisorPrompt.ts:1-808` (核心 prompt 构造器完整)
- **后续** (R21+ P2): `apeireth-council` 7 advisor voting 真实 LLM 集成, 替换默认 trait 实现

---

_ADR 0011 草拟 (technical_writer) — `apeireth-team-lead` 新 crate 命名 + supervisorPrompt 1:1 翻译路径正式确立, 不修改任何 LOCKED 文档 / 现有架构 / 现有 ADR / 现有 crate._
_5 项硬决策 + 4 类备选方案 (A 推荐) + 实施 11 阶段 + 关键不假装 8 条 + 不修改承诺 14 项._
_主哲学 6 锚穿透. 任何接手者能查. 等主人对命名 A/B/C/D 拍板._

---

## 拍板记录

- **2026-08-05 13:34** - 主人拍板 A 方案：`apeireth-team-lead`（新 crate 命名）
  - 理由：明确"团队 + leader"角色，跟 `apeireth-supervisor`（进程监督）区分
  - 决策者：Mavis 默认 + 主人拍板
  - 影响：解锁 ADR-0011, ADR-0010 §8 决策清单, ARCHITECTURE.md §5.2 映射表
  - 后续：等 Cargo.toml 完工（code_reviewer 在改），立即创建 `crates/apeireth-team-lead/`
