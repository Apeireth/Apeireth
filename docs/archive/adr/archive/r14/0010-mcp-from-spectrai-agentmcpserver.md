# ADR-0010: apeireth-mcp 来自 SpectrAI AgentMCPServer 翻译

```
[Document-Meta]
Document: docs/adr/0010-mcp-from-spectrai-agentmcpserver.md
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
> **性质**: 第十个 ADR — 记录 `apeireth-mcp` 从 LOCKED 0 代码 → 通过 1:1 翻译 SpectrAI v0.9.21 `AgentMCPServer.ts` (893 LOC) 实现的工程期设计决策，附带 14 个 MCP 工具语义 (8 supervisor + 3 worktree + 3 认知) + mid-task bug 3 处修法合并到 `apeireth-mcp::team` 模块的归属决策。
>
> **依据**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §5.2 + §8 决策清单 #2 + §9.2 + ADR 0007 (兼容组件层) + ADR 0009 (integration rebase skip 策略) + APEIRETH-CONVENTIONS §9 6 锚穿透。
>
> **约束**: ❌ 不修改任何 LOCKED 文档 / Cargo.toml / crates/ 源码；仅新增命名空间 `docs/adr/0010-*.md` 独立 ADR。

---

## 状态

✅ **A 方案已拍板** (2026-08-05 13:34): `apeireth-team-lead` 新 crate 命名采纳, 主人拍板 (Mavis 默认 A); 待 architect 复核 Cargo.toml 加 workspace member。

---

## 背景（Context）

### Apeireth `apeireth-mcp` 现状（R19+ 状态，事实证据）

| 维度 | 状态 | 引用 |
|---|---|---|
| **代码量** | 2135 LOC | R19+ 实测（`crates/apeireth-mcp/`） |
| **架构** | 3 transport (http_streamable / sse / stdio) + JSON-RPC 2.0 protocol + `ToolHandler` 桥 | `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §6.2 |
| **缺口** | 服务端 0 业务工具 (only `ToolHandler` trait 表面) | 同上 |
| **LOCKED 历史** | §14.4 crate 候选 + §16.5 对比表都提及，**0 代码** | ADR 0007 §事实证据 |
| **R19+ 填坑目标** | 通过 SpectrAI `AgentMCPServer.ts` 翻译补 14 个业务工具 | 蓝图 §5.2 第 6 行 |

### SpectrAI `AgentMCPServer.ts` 现状（v0.9.21 事实证据）

| 维度 | 数值 | 引用 |
|---|---|---|
| **文件 LOC** | 893 | `.minimax-agent-cn\spectrai\spectrai-source\src\main\agent\AgentMCPServer.ts` |
| **架构** | Node.js MCP server (stdio + JSON-RPC 2.0) + WebSocket 桥 (AgentBridge.ts 130 LOC) 跨进程调主进程 | grep 实测 |
| **工具总数** | 14 业务工具（按蓝图 §B.2 分类: 8 supervisor + 3 worktree + 3 认知）| 蓝图 §B.2 |
| **mid-task bug 3 处** | ① SessionManagerV2:642 throw; ② AgentManagerV2:281 catch+success; ③ child session 状态窗口期 | 蓝图 §4 |

### 关键不假装（Key Honesty Points）

- 🔴 **`apeireth-mcp` 服务端 0 业务工具**（本 ADR 之前）
- 🔴 **mid-task bug 3 处是真根因**（改 1 处不完整会引入新 bug，必 3 处一起改）
- 🟡 **`rmcp` crate 是官方 Rust MCP SDK**（不在 7 LOCKED 范围；需 Cargo.toml workspace 加依赖，**架构师拍板**）
- 🟡 **`apeireth-team-lead` 命名是新命名**（避免与进程级 `apeireth-supervisor` PID 1 冲突；蓝图 §5.1 决策）

---

## 决策（Decision）

**正式确立 "`apeireth-mcp` 来自 SpectrAI `AgentMCPServer.ts` 翻译" 路径**，按 5 项硬决策：

### 决策 1: 保留 `apeireth-mcp` 现有 2135 LOC 架构

> 3 transport (http_streamable / sse / stdio) + JSON-RPC 2.0 + `ToolHandler` trait 表面 **全部不动**。
> 新增模块 `apeireth-mcp::team` 作为 14 业务工具的载体，**不修改** `McpServer` 核心实现。

### 决策 2: 新增模块 `apeireth-mcp::team`（估 600-800 LOC）

**14 个业务工具**按 3 类翻译：

| 类 | 工具数 | 工具名 | 1:1 翻译源 |
|---|---|---|---|
| **supervisor** | 8 | `spawn_agent` / `send_to_agent` / `get_output` / `wait_idle` / `wait` / `get_status` / `list` / `cancel` | `AgentMCPServer.ts:457-609` |
| **supervisor worktree** | 3 | `merge` / `info` / `check` | `AgentMCPServer.ts:665-720, 832-870` |
| **认知** | 3 | `list_sessions` / `get_summary` / `search_sessions` | `AgentMCPServer.ts:611-665` |

**实现要点**：
- 使用官方 Rust MCP SDK `rmcp` crate（`#[tool]` 宏 + `#[tool_handler]` 简化注册）
- 14 工具全部 `async fn`，统一 `Result<Value, McpError>` 返回
- 单元测试 200 LOC（每工具 1 happy path + 1 error path）
- 集成测试 50 LOC（端到端 spawn → send → wait_idle → cancel）

### 决策 3: mid-task bug 3 处修法合并到 `apeireth-mcp::team`

> **不** 改 `apeireth-agent` / `apeireth-session`（它们是高层抽象）。
> **不** 改 `apeireth-protocol`（协议层不应该懂业务状态）。
> **改** `apeireth-mcp::team` 内部，让 14 工具自己实现"永 panic 不出" + "跨 session 引用先验状态" + "事件驱动状态同步"。

**3 处修法**（蓝图 §4.4）：

| 修法 | 翻译到 Rust |
|---|---|
| **修法 1**: sendMessage 永不 panic | `async fn send_to_agent() -> Result<Value, McpError>`; 终态返 `McpError::SessionClosed` 而非 `panic!` |
| **修法 2**: 所有跨 session 引用都先验状态 | `team::state::AgentState::verify_alive(session_id)` 在每次 send 前调; 状态来自 `tokio::sync::watch` |
| **修法 3**: 状态变更用事件驱动 | 14 工具订阅 `tokio::sync::broadcast<SessionEvent>`; 不轮询 |

### 决策 4: 使用官方 Rust MCP SDK `rmcp` crate

| 选项 | 优点 | 缺点 | 决策 |
|---|---|---|---|
| **A: 自己实现 JSON-RPC 2.0** | 零依赖 | 200+ LOC boilerplate; 协议升级手动 | ❌ |
| **B: 用 `mcp-rs` 社区 crate** | 简单 | 半年没更新; 不支持 streamable HTTP | ❌ |
| **C: 用官方 `rmcp` crate** | Anthropic 维护; MCP 协议 strict | 依赖树增 1 crate; 需 Cargo.toml workspace 加 | ✅ |

**约束**: `rmcp` 不在 7 LOCKED 范围（Cargo.toml 是 M-marked，**架构师拍板** Cargo.toml 改；本文档仅记录决策）。

### 决策 5: 新增 crate `apeireth-team-lead`（避免命名冲突）

> **不** 复用 `apeireth-supervisor`（PID 1 进程级 supervisor，含义不同）。
> **不** 复用 `apeireth-council`（7 advisor voting，不是 supervisor 调度 sub-agents）。
> **新建** `apeireth-team-lead` crate，1:1 翻译 `supervisorPrompt.ts` 808 LOC 818 行 markdown。

**理由**: 蓝图 §5.1 决策 #3 已采纳"1:1 翻译" + "新命名避免冲突"；本文档固化。

---

## 后果（Consequences）

### 正面

- ✅ **填 `apeireth-mcp` LOCKED 0 代码坑**（ADR 0007 §事实证据 标记的 MISSING 项填上）
- ✅ **14 业务工具端到端可测**（单元测试 200 LOC + 集成测试 50 LOC 覆盖）
- ✅ **mid-task bug 3 处修真**（父进程不再"以为成功实际丢消息"；蓝图 §4.2 描述的撕裂状态消失）
- ✅ **保留 `apeireth-mcp` 2135 LOC 架构**（3 transport + protocol 不动；新模块解耦）
- ✅ **使用官方 `rmcp` crate**（Anthropic 维护；MCP 协议 strict 升级跟得上）

### 负面

- ⚠️ **失去：跨进程 MCP server 兼容**（WebSocket 桥 `AgentBridge.ts` 130 LOC 砍掉，改 in-process）
- ⚠️ **失去：未来第三方 MCP 客户端走 stdio 跨进程的能力**（除非保留 893 LOC `AgentMCPServer` 跨进程版）
- ⚠️ **新增 crate `apeireth-team-lead` 850 LOC**（依赖树增 1 crate；架构师拍板 Cargo.toml）
- ⚠️ **Rust `rmcp` crate API 可能变化**（官方 crate 早期；季度 minor 升级 breaking）

### 中和

- 🛡️ **`apeireth-mcp` 现有架构**（3 transport）**完全不动**；新模块独立
- 🛡️ **mid-task bug 修法只影响 `apeireth-mcp::team` 内部**；不外溢
- 🛡️ **Cargo.toml 改动**（加 `rmcp` 依赖 + 加 `apeireth-team-lead` workspace member）**架构师拍板**
- 🛡️ **不修改 LOCKED**（7 项 LOCKED + 8 项附加 LOCKED 全守）

---

## 备选方案（Alternatives Considered）

### 选项 A: 砍 `apeireth-mcp` 整个 crate

- ✅ 零代码
- ❌ ADR 0007 §10 "兼容组件层 4 项硬规则" 失去 1/3 表面
- ❌ 5 Provider 走 4 协议端点 + MCP 客户端调用第三方工具 的能力丧失
- ❌ R11 baseline 跟 MCP 相关的 V1136=0.9063 无法重现

### 选项 B: 只翻译 8 supervisor 工具，不做 worktree + 认知

- ✅ LOC 减半（400 vs 700）
- ❌ 蓝图 §5.2 决策 #6 "P0 全部 14 工具" 失守
- ❌ supervisor worktree 是 GitWorktreeService 翻译的入口（蓝图 §4.5）
- ❌ 认知工具是 minimax m3 hallucination 缓解的关键（蓝图 §4.5）

### 选项 C: 完整 14 工具 + mid-task bug 修法 + `rmcp` + `apeireth-team-lead` 新 crate（本决策）

- ✅ 蓝图 §5.2 决策 #2+#6 全部采纳
- ✅ ADR 0007 LOCKED 设想填坑
- ✅ mid-task bug 一次性解决
- ⚠️ Cargo.toml 增 2 项（`rmcp` 依赖 + `apeireth-team-lead` workspace member）

---

## 实施路径（Implementation Path）

| 阶段 | 任务 | Owner | 依赖 |
|---|---|---|---|
| R19+ 阶段 1 sprint | `apeireth-mcp::team` 8 supervisor 工具实现 | mcp-integration-expert | 本 ADR + `rmcp` crate 集成 |
| R19+ 阶段 1 sprint | mid-task bug 修法 1+2 合并到 `team` 模块 | mcp-integration-expert | 8 supervisor 工具骨架 |
| R19+ 阶段 2 sprint | `apeireth-mcp::team` 3 worktree 工具 + 3 认知工具 | mcp-integration-expert | 阶段 1 + `apeireth-git` (P1) |
| R19+ 阶段 2 sprint | mid-task bug 修法 3 合并（事件驱动状态）| mcp-integration-expert | 阶段 1 收尾 |
| R19+ 阶段 2 启动前 | 新建 `apeireth-team-lead` crate 1:1 翻译 supervisorPrompt | technical_writer | 蓝图 §5.2 决策 #3 |
| R19+ 阶段 3 | 14 工具单元测试 + 集成测试 | qa_engineer | 14 工具实装 |
| R19+ 阶段 4 收产品 | 端到端 demo（TUI 用 14 工具跑团队协作）| leader | 14 工具全测 |

---

## 关键不假装（Key Honesty Points）

- 🔴 **`apeireth-mcp` 服务端 0 业务工具**（本 ADR 之前；不假装已实装）
- 🟡 **`rmcp` crate 加 Cargo.toml workspace**（**架构师拍板**，本文档仅记录决策）
- 🟡 **`apeireth-team-lead` 新 crate 850 LOC**（避免命名冲突；需 Cargo.toml workspace 加 member）
- 🟡 **mid-task bug 修法 3 处一起改**（改 1 处不完整会引入新 bug；不可跳）
- 🟢 **`apeireth-mcp` 现有 2135 LOC 架构**（3 transport）**完全不动**（不重写已有代码）
- 🟢 **8 supervisor / 3 worktree / 3 认知 = 14 工具**（蓝图 §B.2 已锁；逐个测试覆盖）

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
| 现有 ADR 0001~0009 | 不动 |
| 现有 stage4-* / README.md / CHANGELOG.md | 不动 |

---

## 哲学 anchor

| 锚 | 来源 | 本 ADR 落地 |
|---|---|---|
| **S-1 主 22:33** | 北极星导向 | 14 工具覆盖 supervisor 8/8 = 团队协作全场景；服务 ASI 北极星 |
| **S-2 主 17:43** | 实事求是 | `apeireth-mcp` 现状 2135 LOC 0 业务工具 = 实事求是，不假装已实装 |
| **O-5 主 17:58** | 不假装 | mid-task bug 是 P0 急救；3 处修法合并到 `team` 模块 = 不假装 1 处能解决 |
| **O-2 主 19:33** | 走在前人经验上 | 工具语义 1:1 翻译，可对照 SpectrAI v0.9.21 测试 |
| **O-3 主 23:44** | 干到底 | 实施路径 5 阶段 + owner 明确 + 14 工具逐个测试 |
| **O-4 主 00:56** | 任何人都能接手 | 决策 5 项 + 后果 3 维度 + 备选 3 方案 + 实施 5 阶段 + 关键不假装 6 条 |

---

## 关联文档

- **前置**: [ADR 0007 兼容组件层](0007-compat-components-layer.md) + [ADR 0009 integration rebase skip](0009-integration-rebase-skip-policy.md)
- **蓝图**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §5.2 第 6 行 + §8 决策清单 #2 + §9.2 阶段 1 路线
- **mid-task bug 详**: 蓝图 §4（3 处根因 + 修法表）
- **源码引用**: `AgentMCPServer.ts:457-870`（14 工具定义完整清单）

---

_ADR 0010 草拟 (technical_writer) — `apeireth-mcp` 来自 SpectrAI `AgentMCPServer.ts` 翻译路径正式确立, 不修改任何 LOCKED 文档 / 现有架构 / 现有 ADR._
_5 项硬决策 + 3 类工具 14 个 + mid-task bug 3 处修法合并 + `apeireth-team-lead` 新命名._
_主哲学 6 锚穿透. 任何接手者能查. 矩阵不可摘要替代._

---

## 拍板记录

- **2026-08-05 13:34** - 主人拍板 A 方案：`apeireth-team-lead`（新 crate 命名）
  - 理由：明确"团队 + leader"角色，跟 `apeireth-supervisor`（进程监督）区分
  - 决策者：Mavis 默认 + 主人拍板
  - 影响：解锁 ADR-0011, ADR-0010 §8 决策清单, ARCHITECTURE.md §5.2 映射表
  - 后续：等 Cargo.toml 完工（code_reviewer 在改），立即创建 `crates/apeireth-team-lead/`
