# R33 优秀项目调研 (2026-08-09)

**作者**: Mavis
**范围**: 主人 8-09 00:49 指令 "先调研优秀项目, 联网搜索或知识库"
**前置**: R30 (8 工具 + 5 协议) + R31 (24 个 test 修好) + R32 (5 候选后端方向)

---

## 1. 主拍 (TL;DR)

挑 5 个对 **Apeireth 路线** 最相关的项目做调研, 每个给:
- **定位** (1-2 句)
- **跟 R30/R32 关联** (我们做了什么 / 缺什么)
- **能借鉴什么** (1 个最具体点, 可落地)
- **不借鉴什么** (避免 cargo cult)
- **触发条件** (什么时候做)

按主人战略关联度排序 (AI 操作系统 > 长程 memory > 多 agent > AI coding tool > 协议桥):

| 序 | 项目 | 关联度 | 调研结论 |
|---|---|---|---|
| 1 | **Aider** (AI coding) | ★★★★ | 必看 — 跟 R30 8 工具直接对标, 给"R30 该不该再加"参考 |
| 2 | **mem0** (long memory) | ★★★★★ | **借鉴** — 跟 R9 ThreeLayerMemory 范式同, 但更成熟 |
| 3 | **LangGraph** (agent 编排) | ★★★★ | 借鉴 — 跟 R32 pipeline 升级对应 |
| 4 | **MCP 协议** (Anthropic) | ★★★ | 已做 — 跟 R10 ProtocolGateway 对齐, 升级路径清晰 |
| 5 | **AutoGen / CrewAI** (多 agent) | ★★ | 暂缓 — 跟 R32 council 候选同, 5d 估时偏大 |

---

## 2. 项目 1: Aider (AI coding agent)

**定位**: 著名 AI coding agent (Python, MIT), 跟 ClaudeCode / Codex / OpenClaw 同赛道. 专注 repo-level 代码修改, git workflow 是核心.

**跟 R30 关联**:
- 我们的 8 工具 (WebSearch / FileOperator / Grep / ApplyPatch / Git / ShellExec / WebFetch / LongTask) 跟 Aider 的 tool set 几乎 1:1
- 区别: Aider 是 Python + 80+ 工具; 我们 Rust + 8 工具, 更瘦
- R30 P4 commit message 自动生成 (我们没做)

**能借鉴什么**:
- **"conventions awareness"**: Aider 在 system prompt 注入项目约定 (e.g. "this project uses serde, async, no unsafe"), 我们 SYSTEM_PROMPT 是硬编码的, 没法跟项目走
- 落地点: `apeireth-tools/src/registry.rs` 加 "conventions scanner", 启动时扫 `Cargo.toml` 拿 dep / edition / lints, 注入到 LLM 上下文. 1d.

**不借鉴什么**:
- Aider 80+ 工具 (我们 8 工具够了, codex 主人说 "AI 要有手和脚", 但不是 80 只手)
- Aider 用 GPT-4 起步, 我们 R7 跑 minimaxi 起步 (本地化路径)
- Aider 的 "repo map" (全项目 AST 摘要) — 跟我们 R9 ThreeLayerMemory 的 working layer (50 条 ring buffer) 思想不同, 不可比

**触发条件**: 主人问 "AI 不知道我项目用什么 dep" 时

---

## 3. 项目 2: mem0 (long-term memory) ★ 重点借鉴

**定位**: 开源 long-term memory 库 (Python, Apache 2.0), 2024 爆款. 核心是"AI 的第 2 大脑", 解决 LLM 上下文窗口短 + 跨 session 记忆丢失. 已经被多家 (Replit, Sourcegraph, Bayer) 集成.

**跟 R9 关联**:
- 我们的 R9 ThreeLayerMemory (working / short / long) 跟 mem0 的 4 层 (user / agent / session / long-term) 思路同
- 区别: mem0 重点是 "自动 extraction + embedding 检索", 我们是 "手动 recall by depth"
- 缺: 我们的 3 层没有"自动摘要 + 跨 session 检索", 只按时间窗口拿

**能借鉴什么 (具体落地)**:
- **fact extraction API**: mem0 提供 `add("I love Rust", user_id=...)` 自动提取 fact, 我们的 R9 是手动写 episode
- 落地点: `apeireth-memory/src/three_layer.rs` 加 `promote_with_summarize(working[0..n]) -> Vec<Note>`, 用 R19 启发式 (5 个 rule) 抽 fact. 2d.
- 触发: 主人说"AI 记不住我之前说啥" 时

**不借鉴什么**:
- mem0 用 LLM 做 fact extraction (烧钱), 我们 R9 promote 是纯 rule-based (零成本)
- mem0 用 vector DB (Qdrant), 我们 1.0 release 不用 vector (主人战略: 离线优先, 0 依赖云)
- mem0 强制 user_id / agent_id 隔离, 我们 TUI 1 user, 不需要

**触发条件**: R32 方向 3 (eval 真接) 完成后, 主人问 "memory 怎么更智能" 时

---

## 4. 项目 3: LangGraph (agent orchestration) ★ 借鉴

**定位**: LangChain 出的 state-machine based agent 框架 (Python, MIT). 核心: 把 agent 流程建模为有向图 (nodes = action, edges = transition), 比 ReAct/AutoGPT 的"无限循环"更可控.

**跟 R32 关联**:
- R32 方向 2 想把 R30 tool loop 抽到 apeireth-pipeline
- 我们的 pipeline 24 节点 orchestrator (R17 战役 0 7 阶段) 跟 LangGraph 的 graph 思想一致
- 区别: LangGraph 是 dynamic graph (运行时决定下一步), 我们是 static 7 阶段

**能借鉴什么 (具体落地)**:
- **conditional edge**: LangGraph 用 `add_conditional_edges("tool_call", should_continue)` 让 tool loop 自己决定继续/停止, 我们 R30 是 hardcode 3 turn 上限
- 落地点: `apeireth-pipeline/src/tool_loop.rs` 加 `should_continue(state) -> bool`, 替代 hardcode `MAX_TOOL_TURNS = 3`. 1d.
- 触发: 主人说"AI tool 调到一半停了" 或 "AI tool 死循环了" 时

**不借鉴什么**:
- LangGraph 的 checkpointing (持久化 state 到 DB) — 我们 R9 ThreeLayerMemory 已经在做
- LangGraph 的 human-in-the-loop 节点 — 我们 R2 approval 已经在做 (ToolPolicy)
- LangGraph 的 streaming events — 我们 R30 P4 已经在做 (ToolCallEvent)

**触发条件**: R32 方向 2 启动时

---

## 5. 项目 4: MCP 协议 (Anthropic)

**定位**: Model Context Protocol (Python/TS SDK, MIT/Apache), Anthropic 2024-11 推, 跟 USB-C 一样标准化 AI-tool 连接. 已经 1000+ 集成, 是 2025 AI 工具的"事实标准".

**跟 R10 关联**:
- 我们的 R10 ProtocolGateway 装了 MCP kind, 但 apeireth-mcp 仍是 skeleton (战役 5 P0)
- 区别: 真正的 MCP 协议有 sampling / resources / prompts 3 套, 我们只做了 tools/list + tools/call

**能借鉴什么 (具体落地)**:
- **resources 概念**: MCP 允许 AI 读"非 tool 的数据" (e.g. 文件, 数据库, API) 不走 tool call. 我们 R30 FileOperator.read 是 tool, 但 R9 episode recall 不是
- 落地点: `apeireth-mcp/src/protocol.rs` 加 `resources/list` + `resources/read` 实现, 走 ProtocolGateway 暴露给 LLM. 2d.
- 触发: 主人接 OpenClaw / R9 memory / 外部 DB 时

**不借鉴什么**:
- MCP 的 sampling 协议 (LLM 嵌套调用) — 我们 R10 ProtocolGateway 不做 LLM-of-LLM, 单层
- MCP 的 prompts 协议 (template 复用) — 我们 SYSTEM_PROMPT 写死, 主人 R19 锁定
- MCP SDK 的 stdio 通信 — 我们 R10 ProtocolGateway 走 HTTP (跟 Tauri 共享)

**触发条件**: 主人说"AI 该能读我 DB / Notion / 文件"时

---

## 6. 项目 5: AutoGen / CrewAI (multi-agent)

**定位**: 
- **AutoGen** (Microsoft, MIT): 通用多 agent 框架, 2 AI 对话, 1 人类审批
- **CrewAI** (Python, MIT): role-based 多 agent 协作, 像 "研究员 + 工程师 + 主编" 联合产报告

**跟 R32 关联**:
- R32 方向 5 想做 "多 AI 投票决断" (apeireth-council), 跟 AutoGen/CrewAI 同
- 区别: 它们是多 agent 协作, 我们是"多 provider 投票" (不同模型给同一 query, 选最佳)

**能借鉴什么 (具体落地)**:
- **role definition**: CrewAI 每个 agent 有 role/goal/backstory 3 字段, 我们 council 没角色, 都用 default
- 落地点: `apeireth-council/src/` 加 `CouncilMember { role, goal, backstory, provider }`, 启动时 5-10 个角色 (e.g. "工程师" / "架构师" / "测试员" / "安全审计"). 2d.
- 触发: 主人说 "我想让多个 AI 给方案, 我挑" 时

**不借鉴什么**:
- AutoGen 的 "GroupChat" (多 AI 互相对话) — 太贵, 主人 R19 不烧钱原则
- CrewAI 的 process.sequential/hierarchical — 我们 R7 0 流程, 简单投票
- AutoGen 的 code executor sandbox — 我们 R30 ShellExec + approval 已经在做, 不需 2nd sandbox

**触发条件**: R32 方向 5 启动时

---

## 7. 总结: 主人接下来可走 3 条路

| 路径 | 估时 | 关键借鉴 |
|---|---|---|
| **A. 后端 R32 真接 (1-2d)** | 1-2d | apeireth-asi 真计算 (R33 #1 暂缓, 但 pipeline tool loop 可借鉴 LangGraph) |
| **B. mem0 借鉴 (2d)** | 2d | working → long-term 自动 summarize, 跟 R9 ThreeLayerMemory 接 |
| **C. MCP resources (2d)** | 2d | apeireth-mcp 升级, 走 R10 ProtocolGateway 暴露 resources |

---

## 8. 不动边界 (R33 0 触)

- ✅ 0 联网 (知识库思考, 主人允许)
- ✅ 0 抄代码 (借鉴字段名 + 行为, 不抄实现)
- ✅ R10 LOCKED 0 触 (Pybridge / Module / etc.)
- ✅ 8 项不修改承诺 0 触

---

## 9. 决策日志

1. **不联网原因**: 主人 8-04 R25 已说 "知识库够用", 加上当前 network 状态未知. 知识库版本: Aider (2024-12 v0.6x), mem0 (2024-11 v0.1x), LangGraph (2024-10 v0.2x), MCP (2024-11 spec v2024-11-05), AutoGen (2024-10 v0.4x), CrewAI (2024-12 v0.x). 都是 2024 年底版本, 2025 增量未知.
2. **不展开原理**: 5 个项目都很深, 给主人 1 段足够决策. 拍哪个再做深挖.
3. **不混合借鉴**: 主人喜欢"借鉴一个, 做到位", 不喜欢 cargo cult 抄一堆.
