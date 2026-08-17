# R189 GitHub 优秀项目调研 — tool 栈 (registry/runtime/approval/fetch)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R189
> **日期**: 2026-08-13
> **范围**: apeireth-tools (复数) + 9 个 tool-* 子 crate
> **状态**: 调研为升级预备. R179/R181/R174/R140 已分别调研过 browser/codesearch/fetch/filesystem/shell.

---

## 0. 现状

### 核心 (3 文件 396KB)
- apeireth-tool-registry (143KB): async_task 17KB + classifier 49KB + lib 9KB
- apeireth-tool-runtime (171KB): executor 13KB + fuzzy 12KB + lib 10KB
- apeireth-tool-approval (81KB): approval_bridge 8KB + decision 5KB + fuzzy_bridge 4KB + 3 more

### 具体工具
- apeireth-tool-filesystem (R140)
- apeireth-tool-shell (R140)
- apeireth-tool-browser (R179 调研 + chromiumoxide 升级)
- apeireth-tool-codesearch (R181 调研 + ast-grep 升级)
- apeireth-tool-fetch (R176 + R174 已真接, 11 文件 58KB: anysearch 16KB / bilibili / anime / deep / cache / engine / search_aggregator / http_fetch / html_extract)
- apeireth-tool-image-gen (R173 deferred)
- apeireth-tool-image-process (R173 deferred)
- apeireth-tool-search (R174 fetch 中合并)

### 入口
- apeireth-tools (总入口, 类似 central registry)

**已实现能力**:
- 3 核心 + 9 工具, 工具栈结构清晰
- classifier 49KB 智能分类
- fuzzy 12KB 模糊匹配
- approval 决策机制

---

## 1. LLM 工具协议 SOTA

### 1.1 Model Context Protocol (modelcontextprotocol) — **RECOMMENDED 行业标准**

- **GitHub**: https://github.com/modelcontextprotocol
- **License**: MIT
- **定位**: Anthropic 主导的 LLM 工具协议
- **核心能力**:
  - JSON-RPC 2.0 基础
  - Tools / Resources / Prompts / Sampling
  - stdio / HTTP/SSE 传输
  - 我们 apeireth-mcp 已经在用 (R115)

**学习点**: 我们 tool-runtime 应该对齐 MCP 协议

### 1.2 OpenAI Function Calling / Tools — **学习**

- 我们已经对齐
- **学习点**: schema 标准化

### 1.3 LangChain Tools (再, R188 提过) — 学习

### 1.4 VCP Variable & Command Protocol — **学习**

- VCP 自己的协议 (R185 调研)
- **学习点**: 串指令 / 元指令 / 时间预约

### 1.5 A2A (Agent-to-Agent, Google 2025+) — **学习**

- 跨 agent 通信
- **价值**: 我们 council + 跨用户 agent 互联

---

## 2. 工具执行 SOTA

### 2.1 wasmtime (再, R178 提过) — 沙箱执行

- 我们 sandbox 当前用
- 字节码隔离

### 2.2 Firecracker / microsandbox (再, R178 提过) — VM 执行

- R178 推荐 microsandbox 真接
- **价值**: tool-runtime 升级到 micro-VM

### 2.3 Docker / bollard (再) — 容器执行

- 我们 sandbox 当前 stub

### 2.4 eBPF (aya-rs/aya-rust) — **学习 (高级)**

- 内核级工具执行
- 极高性能, 但学习成本高
- **价值**: 长期 sandbox 升级

### 2.5 process-execution (各种) — 备选

- std::process + seccomp/AppArmor

---

## 3. 工具发现 / 模糊匹配 SOTA

### 3.1 LangChain Tool Selector — 学习

- LLM-driven 工具选择
- 我们 fuzzy.rs 类似

### 3.2 fzf (再, R183 提过) — 学习

- 模糊匹配 SOTA
- 我们 fuzzy 借鉴

### 3.3 nucleo / fuzzy-matcher (Rust) — **学习**

- 模糊匹配 Rust 实现
- 我们 fuzzy 升级

### 3.4 tantivy (quickwit-oss/tantivy) — **学习 (Rust 全文搜索)**

- 13K+ stars, Apache 2.0
- 全文搜索 SOTA
- **价值**: tool-registry 工具描述全文检索

### 3.5 meilisearch (meilisearch/meilisearch) — 学习

- 50K+ stars, MIT
- 工业级搜索
- 不集成 (Rust 服务重)

---

## 4. 工具权限 / 审批 SOTA

### 4.1 我们 Self-Disable + Permission Onion — **业界独一档**

- L0-L5 权限洋葱
- 物理多签
- 形式化验证

### 4.2 Anthropic Tool Use Permissions — 学习

- 用户授权机制
- 我们 tool-approval 借鉴

### 4.3 OpenAI Function Calling with user confirmation — 学习

- 用户确认模式

### 4.4 VCP VCPNotify + VCPLog (R185 提过) — 学习

- 三套通知物理隔离
- 我们 tool-approval 加类似机制

### 4.5 Kubernetes RBAC — 学习

- 角色权限
- 工业级
- **学习点**: 我们 approval 角色化

---

## 5. 工具分类 / Routing SOTA

### 5.1 Semantic Router (再, R184 提过) — **学习**

- 语义级工具选择
- 我们 tool-registry classifier 借鉴

### 5.2 LangChain AgentExecutor — 学习

- 工具 + agent 循环

### 5.3 DSPy (再, R184 提过) — 学习

- type-safe tool signature

### 5.4 Gorilla (ShishirGPatil/gorilla) — **学习**

- LLM for API call
- 论文: Gorilla: Large Language Model Connected with Massive APIs
- **学习点**: 工具选择 SOTA 模型

---

## 6. 升级方案 (R189+ 实施)

### 6.1 短期 (1-2 days)

1. **nucleo / fuzzy-matcher**: tool-registry fuzzy 升级
2. **tantivy 评估**: 工具描述全文检索
3. **MCP 协议对齐**: tool-runtime 完整支持 MCP

### 6.2 中期 (3-5 days)

4. **microsandbox 真接** (R178 路线): tool-runtime sandbox 升级
5. **Gorilla 风格工具选择**: classifier 加 LLM-driven 选择
6. **三套通知物理隔离** (R185 路线): tool-approval 加 AI/UI/Both

### 6.3 长期 (持续)

7. **A2A 协议对齐**: 跨 agent 工具调用
8. **eBPF 沙箱**: 高性能 sandbox
9. **Anthropic Skills 渐进式披露**: 工具按需注入

---

## 7. 依赖增量

| crate | 体积 | License | 必需 |
|---|---|---|---|
| nucleo | ~50KB | MIT | 短期 |
| fuzzy-matcher | ~30KB | MIT | 短期 |
| tantivy (评估) | ~10MB | Apache 2.0 | 中期 |
| microsandbox (R178) | ~5MB | Apache 2.0 | 中期 |

**总增加**: ~80KB (短期), ~15MB (中长期)

---

## 8. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| sovereignty (R178) | tool 沙箱隔离 |
| tool-* (9 个) | 具体工具 |
| core (R188) | 工具调用 verdict 守门 |
| council (R180) | advisor 可调用工具 |
| bus (R188) | 工具调用事件 |
| pipeline (R184) | 工具作为 stage |

---

## 9. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- tool 公开 API: 0 改 (新能力在子模块内, 通过 trait 抽象)

---

## 10. 参考链接

- MCP: https://github.com/modelcontextprotocol
- OpenAI Function Calling: https://platform.openai.com/docs/guides/function-calling
- LangChain Tools: https://python.langchain.com/docs/concepts/tools
- wasmtime: https://github.com/bytecodealliance/wasmtime
- Firecracker: https://github.com/firecracker-microvm/firecracker
- microsandbox: https://github.com/zerocore-ai/microsandbox
- aya-rs: https://github.com/aya-rs/aya-rust
- nucleo: https://github.com/helix-editor/nucleo
- fuzzy-matcher: https://github.com/althonos/fuzzy-matcher
- tantivy: https://github.com/quickwit-oss/tantivy
- meilisearch: https://github.com/meilisearch/meilisearch
- Semantic Router: https://github.com/aurelio-ai/semantic-router
- Gorilla: https://github.com/ShishirGPatil/gorilla
- Anthropic Skills: https://www.anthropic.com/news/skills
- VCP Protocol: vcptoolbox.com/learn-vcp
- A2A: https://github.com/google/A2A