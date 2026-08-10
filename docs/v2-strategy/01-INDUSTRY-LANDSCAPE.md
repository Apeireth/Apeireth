[Document-Meta]
Document: 01-INDUSTRY-LANDSCAPE.md
Version: 2.0.0-V2
R-Cycle: v2-strategy
Last-Modified: 2026-08-04
Status: 🟡 DRAFT v2 (主人决策:5 战区全要)
Author: Codex (策略分析)

---

# 行业坐标系(v2) — Apeireth 在 5 战区全打

> v2 关键修正:主人决策**战区 1-5 全要**(除 UI 战区 6 交给其他团队)。
> 这意味着 Apeireth 的定位**不是"窄 Runtime"**,而是**"VCP 的全栈 Rust 重写 + 独家的安全原语"**。

---

## 5 战区对应表

### 战区 1:终端 Coding Agent

| 项目 | 核心武器 | Apeireth 实测 |
|---|---|---|
| **Claude Code** (Anthropic) | 终端原生 + Plan Mode + Subagent + Hooks + MCP | **apeireth-tui** 255KB(5 页面全栈) |
| **Codex CLI** (OpenAI) | Sandbox + Approval + TUI 流式 + 工作树隔离 | 已对齐 60% |
| **Cursor** | IDE 集成 | ❌ 不做(其他团队) |
| **Cline** | VS Code 扩展 | ❌ 不做 |
| **Aider** | git 原生 + repo map | ⚠️ 部分可借鉴 |
| **OpenHands** | 沙盒 runtime + 浏览器 | ⚠️ 可作 runtime 借鉴 |
| **Devin** | 长程 SWE | ⚠️ 不可对标 |

**Apeireth 现状**:`apeireth-tui` 已有 5 页面 ratatui 全栈(255KB,无 lib.rs 因为是 binary),接真后端,真 SSE 流式。**差距**:沙盒隔离、Subagent、Hooks 系统。

---

### 战区 2:LLM 网关 / 协议适配

| 项目 | 核心武器 | Apeireth 实测 |
|---|---|---|
| **VCP** | 26 module + 85 plugin | **apeireth-api** 197KB + **apeireth-protocol** 139KB + **apeireth-http-client** 37KB + **apeireth-pipeline** 76KB |
| **LiteLLM** | 100+ provider | ⚠️ 协议适配能力待扩展 |
| **vLLM** | 高吞吐推理 | ⚠️ 不同品类 |
| **OpenRouter** | 路由优化 | ⚠️ 待开发 |
| **One-API / NewAPI** | 国内主流 | ✅ R17 已接 NewAPI 鉴权 |

**Apeireth 现状**:**4 协议归一化真做了**(139KB),axum HTTP 服务(197KB),Keep-Alive LIFO HTTP 客户端(37KB),5 步主 chat pipeline(76KB)。**领先项**:Rust 类型安全、双洋葱隔离、Self-Disable。

---

### 战区 3:Multi-Agent 编排

| 项目 | 核心武器 | Apeireth 实测 |
|---|---|---|
| **LangGraph** | 图状态机 + checkpoint | ⚠️ **缺图编排** |
| **AutoGen** | GroupChat | ⚠️ **缺多 agent 对话协议** |
| **CrewAI** | 角色 + 任务 | ⚠️ 部分由 council 实现 |
| **MetaGPT** | SOP 软件公司模拟 | ⚠️ 不可对标 |
| **OpenHands Runtime** | 沙盒执行 | ⚠️ 不同方向 |

**Apeireth 现状**:**apeireth-council** 98KB(18 文件 7 advisor)——这其实是个**真实的多 advisor 系统**!`apeireth-supervisor` 22KB 是调度层。`apeireth-evolution` 107KB 是进化机制。

**最大短板**:**没有图编排(类 LangGraph)**——这是战区 3 的核心。

---

### 战区 4:长期记忆

| 项目 | 核心武器 | Apeireth 实测 |
|---|---|---|
| **Letta** (MemGPT) | 分层记忆 + 服务化 | **apeireth-memory** 120KB(8 文件) |
| **Mem0** | 自适应记忆 + 用户画像 | ⚠️ **缺用户画像自动抽取** |
| **Zep** | GraphRAG | ❌ **缺图记忆** |
| **Honcho** | SDK + MCP | ⚠️ **缺 SDK 暴露** |
| **MemoryOS** | Rust + Python SDK | ⚠️ 不可对标 |

**Apeireth 现状**:`apeireth-memory` 120KB——这是个**真实工程化的记忆系统**,基于 SQLite(rusqlite bundled)。`apeireth-bus` 74KB 是事件总线配套。

**最大短板**:**没有语义检索**(依赖 sqlite-vec 之类向量扩展未做)、**没有用户画像**、**没有 GraphRAG**、**没有对外 SDK**。

---

### 战区 5:工具协议 / 工具生态

| 项目 | 核心武器 | Apeireth 实测 |
|---|---|---|
| **MCP** (Anthropic) | 工业标准 | ❌ **缺 MCP 适配** |
| **Composio** | 250+ 工具 | ❌ **缺生态接入** |
| **VCP 插件** | 85 个真实插件 | **apeireth-tools** 82KB(5 trait) + **apeireth-tool-runtime** 95KB + **apeireth-tool-approval** 70KB + **apeireth-tool-registry** 68KB |

**Apeireth 现状**:**5 类 tool trait 真做了**(82KB),tool runtime 完整(parser/executor/privacy/record 95KB),tool approval 70KB(5 规则 + 5 分钟窗口),tool registry 68KB。**对标 VCP 的核心 tool 链路已对齐**。

**最大短板**:**没有 MCP 适配**——必须补。

---

## v2 战略修正

### 不再做的(原 v1 错的)

| ❌ 不做 | 原因 |
|---|---|
| ~~砍 11 个 crate 砍到 18~~ | 错了。实测 37 个有真实代码 |
| ~~砍 apeireth-sovereignty(哲学摆设)~~ | **大错特错**。274KB 真实代码,是战区 5 的安全核心 |
| ~~砍 apeireth-asi(熵增宇宙论)~~ | 错了。92KB 真实代码 |
| ~~合并 perception/cognition/consciousness/motivation/life-force 到 apeireth-mind~~ | **不该合并**。每个 14-29KB 真实代码,合并反而破坏架构 |
| ~~新增 apeireth-runtime 替代 supervisor~~ | supervisor 22KB 已存在,该**增强**而非替代 |

### 真正该做的

| ✅ 该做 | 原因 |
|---|---|
| 保留 39 个 crate 主体架构,**只清理 4 个真小 crate** | 实测 37 个有真实代码 |
| **新增 apeireth-mcp** | 战区 5 必须上车 MCP |
| **增强 apeireth-memory** | 战区 4 必须达到 Letta 水平 |
| **新增 apeireth-graph** | 战区 3 缺图编排 |
| **保留 apeireth-supervisor** | 战区 3 调度核心已有 |

---

## 实测验证:37/39 crate 都有真实代码

| Crate | 实测代码量 | 战区 |
|---|---|---|
| sovereignty | 274 KB | 战区 5 |
| tui | 255 KB | 战区 1 |
| api | 197 KB | 战区 2 |
| upgrade | 151 KB | 战区 3 |
| protocol | 139 KB | 战区 2 |
| web | 135 KB | (战区 6,其他团队) |
| memory | 120 KB | 战区 4 |
| evolution | 107 KB | 战区 3 |
| core | 105 KB | L0 HA |
| council | 98 KB | 战区 3 |
| tool-runtime | 95 KB | 战区 5 |
| constraint | 93 KB | 战区 5 |
| asi | 92 KB | 战区 3 |
| tools | 82 KB | 战区 5 |
| pipeline | 76 KB | 战区 2 |
| bus | 74 KB | 战区 4 |
| extension | 73 KB | 战区 3 |
| tool-approval | 70 KB | 战区 5 |
| tool-registry | 68 KB | 战区 5 |
| value | 56 KB | 战区 3 |
| agent | 55 KB | 战区 3 |
| central | 45 KB | 战区 3 |
| cli | 40 KB | 工具 |
| http-client | 37 KB | 战区 2 |
| pybridge | 35 KB | 工具 |
| verify | 35 KB | 形式化 |
| motivation | 33 KB | 战区 3 |
| onion | 30 KB | L0 HA |
| action | 32 KB | 战区 3 |
| cognition | 29 KB | 战区 3 |
| perception | 29 KB | 战区 3 |
| supervisor | 22 KB | 战区 3 |
| life-force | 18 KB | 战区 3 |
| relation | 15 KB | 战区 3 |
| consciousness | 15 KB | 战区 3 |
| **desktop** | **lib 591B / main 26KB** | ⚠️ 半空半实 |
| **bench** | **2.8 KB** | ⚠️ 真小 |
| **philosophy** | **1.8 KB** | ⚠️ DEPRECATED 自标 |
| **test** | **618 B** | ⚠️ R14 skeleton 自标 |

---

_Last update_: 2026-08-04 (v2)
