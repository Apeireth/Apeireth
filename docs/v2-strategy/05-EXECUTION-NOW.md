[Document-Meta]
Document: 05-EXECUTION-NOW.md
Version: 2.0.0-V2
R-Cycle: v2-strategy
Last-Modified: 2026-08-04
Status: 🟡 DRAFT v2 (基于实测)
Author: Codex (策略分析)

---

# 本周立即执行 — 7 步走通 v2.0(v2)

> 目标:用 5-7 天时间,把 v2.0 战略从"文档"推到"代码"。
> v2 修正:**不是"砍到 18"**,而是"清理 4 个真小 + 新增 5 个 + 强化现有"。

---

## Step 1(Day 1):清理 4 个真小 crate

### 1.1 物理删除 apeireth-philosophy

**依据**:文件头自标 "⚠️ DEPRECATED",承诺"阶段 7+ 物理删除"。
**执行**:
```bash
rm -rf crates/apeireth-philosophy
# 编辑 Cargo.toml 移除 "crates/apeireth-philosophy"
# 跑 cargo check --workspace 确认通过
```

**验收**:`cargo check --workspace` 0 error,workspace members = 38

### 1.2 删除 apeireth-test

**依据**:lib.rs 自标 "R14 skeleton (Python mvp/ 接口兼容待 Phase 1)"——R14 已过,Phase 1 不存在。
**执行**:
```bash
rm -rf crates/apeireth-test
# 编辑 Cargo.toml 移除
```

**验收**:`cargo check --workspace` 0 error,workspace members = 37

### 1.3 apeireth-desktop 改名为 apeireth-tauri-stub

**依据**:lib.rs 占位 591B,main.rs 26KB 真 Tauri 代码;R17 已砍 Tauri 前端。
**执行**:
- 重命名目录 `crates/apeireth-desktop` → `crates/apeireth-tauri-stub`
- 更新 Cargo.toml 的 package name
- 在 README 标注"参考实现,不在产品里"
- workspace members 标记 DEPRECATED 但保留

**验收**:目录改名成功,cargo check 通过

### 1.4 apeireth-bench 扩充骨架

**依据**:现在只有 2.8KB,需要扩充为真做 SWE-bench/AgentBench 跑分。
**执行**:
- 不删,但要扩充到 ≥ 20KB
- 见 Step 6 详细任务

---

## Step 2(Day 2):建 `apeireth-mcp` skeleton

**优先级**:🔴 P0(战区 5 必须上车 MCP)

**新建 `crates/apeireth-mcp/`**:
```rust
// src/lib.rs
pub struct McpClient { /* ... */ }
impl McpClient {
    pub async fn connect_stdio(cmd: &str) -> Result<Self>;
    pub async fn initialize(&mut self) -> Result<ServerInfo>;
    pub async fn list_tools(&self) -> Result<Vec<Tool>>;
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value>;
}

pub struct McpServer { /* ... */ }
impl McpServer {
    pub fn new(name: impl Into<String>) -> Self;
    pub fn register_tool(&mut self, tool: Tool, handler: ToolHandler);
    pub async fn run_stdio(self) -> Result<()>;
}

// src/transport/stdio.rs
// src/transport/sse.rs
// src/protocol.rs (JSON-RPC 2.0)
// src/tool_bridge.rs (桥接到 apeireth-tool-registry)
```

**验收**:`cargo run -p apeireth-mcp --example hello` 启动 client + server 互相调用成功

---

## Step 3(Day 3):建 `apeireth-graph` skeleton

**优先级**:🔴 P0(战区 3 缺图编排)

**新建 `crates/apeireth-graph/`**:
```rust
// src/lib.rs
pub struct Graph { nodes: BTreeMap<NodeId, Node>, edges: Vec<Edge> }
pub trait Node: Send + Sync {
    fn id(&self) -> NodeId;
    fn run(&self, state: &mut State) -> Result<NodeOutput>;
}
impl Graph {
    pub fn add_node(&mut self, node: impl Node + 'static);
    pub fn add_edge(&mut self, from: NodeId, to: NodeId);
    pub async fn execute(&self, init_state: State) -> Result<FinalState>;
    pub async fn checkpoint(&self, state: &State) -> Result<Checkpoint>;
}

// src/checkpoint.rs
// src/state.rs
// src/executor.rs (集成 apeireth-supervisor)
```

**验收**:能跑通一个 3 节点的有向图(线性),checkpoint 写入成功

---

## Step 4(Day 4):强化 apeireth-memory 加向量检索

**优先级**:🟡 P1(战区 4 升级)

**执行**:
- 新建 `crates/apeireth-vector/`(可选,也可放 apeireth-memory 内)
- 集成 `sqlite-vec` 或 `lancedb`
- 实现 `Memory::semantic_search(query: &str, k: usize) -> Vec<MemoryItem>`
- 加 `Memory::extract_user_profile() -> UserProfile` (基于 LLM 调用)

**验收**:
- 1000 条记忆下,语义检索 P99 < 50ms
- 用户画像自动抽取 demo

---

## Step 5(Day 4-5):强化 apeireth-tool-registry 加小模型分类器

**优先级**:🔴 P0(战区 5 对标 VCP)

**依据**:VCP `dynamicToolRegistry.js` 74KB 有小模型分类器,Apeireth 缺。

**执行**:
- 在 apeireth-tool-registry 加 `Classifier` trait
- 默认实现用本地小模型(fastembed + cosine similarity)
- 实现 `classify_tool(tool: &Tool) -> Category` 9 类别(对标 VCP 7 类,加 safety + long-running)
- 跑 demo:注册 10 个工具,自动分类

**验收**:10 个工具自动分类准确率 ≥ 80%

---

## Step 6(Day 5-6):扩充 apeireth-bench

**依据**:现在 2.8KB 太小,需要真做 SWE-bench。

**执行**:
- 在 apeireth-bench 加 `src/swe_bench.rs`:SWE-bench Verified 跑分框架
- 加 `src/agent_bench.rs`:AgentBench 子集
- 加 `src/self_disable_bench.rs`:Self-Disable 攻击场景库(20 个 case)
- 加 `examples/swe_bench_smoke.rs`:1 个 example 跑通

**验收**:bench 框架能跑,Self-Disable 攻击场景 5+ 通过

---

## Step 7(Day 6-7):文档更新 + baseline 报告

### 7.1 更新顶层 README

- 第 1 段改成新定位(对标 VCP 全栈 Rust 重写)
- 加 v2-strategy 链接
- 加 GitHub topics

### 7.2 写 baseline 报告

`docs/v2-strategy/06-V2-BASELINE-2026-08.md`:
- 实测代码量统计(2.6MB Rust)
- 37 个有真实代码的 crate
- 4 个真小 crate 的处理方案
- 5 个新增 crate skeleton 状态
- v2.0.0-alpha 的"能跑什么 / 不能跑什么"

### 7.3 benchmark baseline

- 跑 SWE-bench smoke test(估计很低,因为才 skeleton)
- 跑 Self-Disable 攻击场景(估计 60% 通过)
- 跑 MCP client hello(100% 跑通)
- 跑 graph checkpoint smoke(100% 跑通)

---

## Week 1 时间表

| Day | 主要任务 | 交付物 |
|---|---|---|
| Day 1 | Step 1.1-1.4 清理 + Step 1.4 bench 准备 | workspace = 37 crate |
| Day 2 | Step 2 apeireth-mcp skeleton | MCP hello 跑通 |
| Day 3 | Step 3 apeireth-graph skeleton | graph 3 节点跑通 |
| Day 4 | Step 4 memory 向量 + Step 5 小模型分类器并行 | memory 检索 + 工具分类 |
| Day 5 | Step 6 bench 扩充 | SWE-bench 框架能跑 |
| Day 6 | Step 7.1-7.2 文档 | README 更新 + baseline 报告 |
| Day 7 | Step 7.3 benchmark 跑分 | baseline 数据 |

---

## Week 1 后立即开始的事

| 优先级 | 任务 |
|---|---|
| 🔴 P0 | apeireth-mcp 加 transport 实现(sse + http) |
| 🔴 P0 | apeireth-graph 加 checkpoint 持久化 |
| 🟡 P1 | apeireth-protocol 加 Gemini 协议 |
| 🟡 P1 | apeireth-api 加 Response replay cache |
| 🟢 P2 | apeireth-formal 加 Kani 验证 |

---

## 风险

| 风险 | 应对 |
|---|---|
| 7 天跑不完 | 优先级排序:MCP > graph > bench > vector |
| 物理删除 philosophy 引发意外 | git revert 一键回滚 |
| MCP 规范细节不清楚 | 跑官方 conformance test |
| 小模型分类器没训练数据 | 用 VCP 的 7 类关键词字典先 mock |

---

## 一句话

**一周内,把 v2.0 从"文档"推到"代码能跑 + 5 战区都有 P0 skeleton"**。

跑通 MCP hello + graph 3 节点 + memory 向量 + tool 分类 + SWE-bench 框架 + 清理 4 个真小 crate,这六件事是 v2.0 战略的**最小可证伪单元**。

---

_Last update_: 2026-08-04 (v2)
