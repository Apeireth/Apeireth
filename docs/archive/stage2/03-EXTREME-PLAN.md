[Document-Meta]
Document: 03-EXTREME-PLAN.md
Version: 2.0.0-V2
R-Cycle: v2-strategy
Last-Modified: 2026-08-04
Status: 🟡 DRAFT v2 (基于实测)
Author: Codex (策略分析)

---

# 极致版 18 个月路线图(v2,基于实测)

> v2 关键修正:基于实测代码量(2.6MB Rust, 37 个 crate 有真实代码),
> 战略从"做 Runtime"修正为"**5 战区全打,对标 VCP 但用 Rust 重写 + 独家安全原语**"。

---

## 阶段 0(Month 0-1):清理与强化

### 任务清单

#### 0.1 真正可清理的 4 个 crate

| Crate | 实测 | 处理 |
|---|---|---|
| **apeireth-philosophy** | 1.8KB,文件头自标 "⚠️ DEPRECATED",等阶段 7+ 物理删除 | ✅ **按文件头承诺物理删除** |
| **apeireth-test** | 618B,自标 "R14 skeleton (Python mvp/ 接口兼容待 Phase 1)" | ⚠️ 评估:是否还需要独立 test crate? 多数测试在各 crate 内,这个可删 |
| **apeireth-bench** | 2.8KB,只有 1 个文件 | ⚠️ **不删,但需扩展** —— 应当承担 SWE-bench/AgentBench 跑分,不能继续 2.8KB |
| **apeireth-desktop** | lib.rs 591B 占位,但 main.rs 26KB 真 Tauri 代码 + tauri.conf.json + gen/icons | ⚠️ **保留 main.rs 部分**,但 lib.rs 占位删,改名为 `apeireth-tauri-stub` 或并入 main binary |

#### 0.2 必须新增的 5 个 crate

| 新 crate | 战区 | 职责 |
|---|---|---|
| 🆕 **apeireth-mcp** | 战区 5 | MCP 客户端 + server(接入生态的命脉) |
| 🆕 **apeireth-graph** | 战区 3 | 图编排(LangGraph 风格) |
| 🆕 **apeireth-vector** | 战区 4 | 向量检索后端(sqlite-vec 或 LanceDB) |
| 🆕 **apeireth-sdk** | 战区 1/4/5 | 多语言 SDK 统一测试 |
| 🆕 **apeireth-formal** | 战区 5 | 形式化验证(Kani + Creusot) |

#### 0.3 必须**加强**的现有 crate(不合并,只补全)

| Crate | 现状 | 需补 |
|---|---|---|
| **apeireth-memory** | 120KB,SQLite 持久化 | 加向量检索、用户画像、跨会话图 |
| **apeireth-supervisor** | 22KB,调度核心 | 加图编排能力 |
| **apeireth-tool-registry** | 68KB,工具注册 | 加小模型分类器(对标 VCP) |
| **apeireth-tui** | 255KB,5 页面 | 加工作树隔离、Subagent、Hooks |
| **apeireth-protocol** | 139KB,4 协议 | 加 Gemini / Cohere(对标 VCP 三协议) |
| **apeireth-extension** | 73KB,15 文件 | 评估是否真在做扩展点 |

### 阶段 0 成果
- 物理删除 apeireth-philosophy(履行文件头承诺)
- 评估是否删除 apeireth-test
- 重命名 apeireth-desktop → apeireth-tauri-stub(只保留 main.rs)
- 加强 apeireth-bench → 真做 SWE-bench
- 新增 5 个 crate skeleton

---

## 阶段 1(Month 2-4):补齐短板

### 1A:MCP 全适配(Month 2-3)

**新建 `apeireth-mcp`**:
- 实现 MCP client + server
- 支持 stdio + SSE + HTTP streamable 三种 transport
- 把现有 apeireth-tool-registry 桥接到 MCP
- **超 VCP 的设计**:Type-safe tool schema(Rust trait 编译期保证)

**指标**:
- 100% MCP 规范测试通过
- 能被 Claude Desktop / Cursor / Cline 识别
- tool 调用延迟 < 5ms(本地)/ < 50ms(含 transport)

### 1B:Memory 升级(Month 3-4)

**升级 `apeireth-memory`**:
- 加 **apeireth-vector** 集成(sqlite-vec 或 LanceDB)
- 实现**语义检索 + 时间检索 + 标签检索** 3 维度
- 加**用户画像自动抽取**(基于 LLM)
- 加**记忆压缩**(每 1000 轮自动摘要)
- 实现 **Memory MCP Server**(把记忆暴露成 MCP server)

**指标**:
- 100k tokens 检索 P99 < 100ms
- 用户画像准确率 ≥ 80%

### 1C:TUI 增强(Month 2-4,贯穿)

**升级 `apeireth-tui`**:
- 加 **工作树隔离**(基于 git worktree)
- 加 **Subagent 调度**(基于 apeireth-supervisor)
- 加 **Hooks 系统**(模仿 Claude Code)
- 加 **Plan Mode**(先给计划再执行)

---

## 阶段 2(Month 5-8):Multi-Agent + 图编排

### 2A:图编排(Month 5-6)

**新建 `apeireth-graph`**:
- 实现 LangGraph 风格的图状态机
- 支持 checkpoint(状态保存)
- 支持时间回溯(回到任意节点)
- 集成 apeireth-supervisor 作为执行器

**指标**:
- 支持节点并行执行
- 支持动态图重写
- P99 checkpoint 写入 < 10ms

### 2B:Multi-Agent 协作模式(Month 6-8)

**升级 `apeireth-council`**:
- 实现 4 种协作模式:Planner + Executor / Debate / Voting / Hierarchical
- 加图编排支持
- 实现"角色宪法"(每个 advisor 自己的约束)
- 加 reasoning trace 可视化

**指标**:
- 3 个 advisor 协作完成任务的 demo
- SWE-bench Verified ≥ 50%

---

## 阶段 3(Month 9-12):生态接入 + 标杆

### 3A:SDK 多语言(Month 9-10)

**新建 `apeireth-sdk`**:
- Python(PyO3 + maturin)
- TypeScript(napi-rs)
- Go(cgo)
- C-ABI

**指标**:
- `pip install apeireth` 一行可用
- `npm install @apeireth/sdk` 一行可用
- 100% API 覆盖 + 完整类型提示

### 3B:框架适配器(Month 10-11)

**新建 `apeireth-integrations/`**:
- langgraph adapter
- autogen adapter
- claude-code adapter
- openhands adapter

**关键策略**:**做"下面那一层"**——不当另一个 LangGraph,做 LangGraph 下面那一层。

### 3C:3 个标杆 demo(Month 11-12)

1. **apeireth-personal-assistant** — 跑一周不重启
2. **apeireth-coding-agent** — SWE-bench Verified 跑分
3. **apeireth-multi-agent-team** — 3 agent 协作

**指标**:
- SWE-bench Verified ≥ 60%
- GitHub trending
- 真实用户跑满一周

---

## 阶段 4(Month 13-18):登顶

### 4A:形式化验证(Month 13-15)

**新建 `apeireth-formal`**:
- Kani 验证 apeireth-runtime 关键不变量
- Creusot 翻译 runtime 关键路径为 SMT
- 公开"宪法检查器"

**发布物**:
- 一篇学术论文:Apeireth: A Formally Verifiable High-Autonomy Agent Platform
- verified-runtime 子 crate(关键路径 100% 验证)

### 4B:标准提案(Month 15-17)

**主导/参与 2 个标准**:
1. **Agent Capability Token (ACT)**
2. **Memory Interchange Format (MIF)**

**指标**:
- 至少 1 个标准被 2+ 主流项目采用

### 4C:商业化(Month 17-18)

**3 层模式**:
1. 开源核心(Apache 2.0)
2. Apeireth Cloud(企业版)$99/agent/月
3. 白标 SDK

**关键策略**:**先开源占生态,再云服务变现,最后做标准**(参考 MongoDB 路径)。

---

## 5 战区成功定义

### 战区 1:Terminal Agent
- SWE-bench Verified ≥ 60%(对标 Claude Code / Devin)
- GitHub stars ≥ 30k

### 战区 2:LLM Gateway
- 支持 ≥ 5 个 provider(对标 LiteLLM 100+ 是不可能的,做精)
- 协议归一化领先 VCP 1 步

### 战区 3:Multi-Agent
- ≥ 5 个 advisor 协作完成 demo
- 图编排能力对标 LangGraph

### 战区 4:Memory
- 100k tokens P99 < 100ms(对标 Letta)
- 用户画像自动抽取准确率 ≥ 80%

### 战区 5:Tool Protocol
- MCP 100% 兼容
- ≥ 30 个内置工具(从 VCP 借鉴 + 自研)
- Self-Disable 准确率 ≥ 99.99%

### 跨战区独有护城河
- 形式化验证覆盖率 ≥ 80%(全行业第一)
- 双洋葱架构的编译期证明(全行业唯一)

---

## 风险与对策

| 风险 | 对策 |
|---|---|
| MCP 标准变化 | 跟 Anthropic 团队建立直接联系 |
| Rust 人才稀缺 | 双轨——核心用 Rust,外围 SDK 用 Python/TS |
| LangGraph 等快速跟进 | 形式化验证 + 双洋葱是护城河 |
| 5 战区资源不够 | 严格优先级——先 2+3+5(MCP 必须上车),后 1+4 |
| 商业化路径不明 | 先开源社区,再做企业版 |
| 测试覆盖回归 | CI gate:必须 ≥ 2265 |

---

## 总结(v2)

| 维度 | v1.0.0 现状 | 18 个月后目标 |
|---|---|---|
| 定位 | "高自主性长程 agent 平台" | **"VCP 的全栈 Rust 重写 + 独家安全原语 + 双洋葱 + 形式化"** |
| Crate 数 | 39(含 4 个真小) | **44(39 + 5 新增)** |
| 代码量 | 2.6MB Rust | **5MB+ 真正生产代码** |
| 测试 | 2265 | **5000+(5 战区都覆盖)** |
| 战区覆盖 | 1-5 都有但都浅 | **5 战区都达到对应王者前 3** |
| 独有武器 | Self-Disable + 双洋葱(已实现) | **+ 形式化验证 + Multi-Agent + MCP** |
| 学术阵地 | 无 | **至少 1 篇顶会论文** |

**这才是"极致版 v2"。** 关键修正:不是"做窄 Runtime",而是"做 VCP 的全栈 Rust 重写并超越"。

---

_Last update_: 2026-08-04 (v2)
