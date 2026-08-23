# Apeireth 2.0 (终极收敛架构 / Reconstruction V2)

> **哲学基石**：Apeiron（无名与涌现） · 0 装 PASS（绝对诚信） · 基地不是 AI 本身（LLM 只是租客） · 机制而非补丁

本项目是基于 Apeireth 1.0 全量 50 万行代码与 9,400+ 篇架构文档进行**深度重构与终极收敛**的现代化 AGI / 伴侣操作系统。

---

## 一、 架构总览：从 85+ 碎片 Crate 到 10 大高内聚核心模块

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              Apeireth 2.0 终极收敛架构                                 │
└────────────────────────────────────────────────────────────────────────────────────────┘

  【应用与接入】   apeireth-cli / TUI            │  Desktop / Web Client (Tauri 2 + Svelte 5)
  ────────────────────────────────────────────────────────────────────────────────────────
  【网关与服务】   apeireth-gateway (Axum Server / S4 出站真拦截 / 全双工 SSE / MCP 协议宿主)
  ────────────────────────────────────────────────────────────────────────────────────────
  【伴侣智能核】   apeireth-companion (生命器官 / 涌现 E7 / 好奇心 E4 / 世界模型 W1-W3 / 情绪 PAD)
  ────────────────────────────────────────────────────────────────────────────────────────
  【执行与工具】   apeireth-tools (统一插件化工具集 / Windows JobObject + 最小权限 Token 沙箱)
  ────────────────────────────────────────────────────────────────────────────────────────
  【调度运行时】   apeireth-runtime (自我驱动日循环 / 异步任务分发 / 监督者树 / OTel 链路追踪)
  ────────────────────────────────────────────────────────────────────────────────────────
  【多协议引擎】   apeireth-protocol (OpenAI / Anthropic / Gemini / MiniMax 归一化网关)
  ────────────────────────────────────────────────────────────────────────────────────────
  【治理与安全】   apeireth-governance (统一 ABAC 门控 / 13 哲学键 / 三重洋葱 / 不可变审计链)
  ────────────────────────────────────────────────────────────────────────────────────────
  【统一持久化】   apeireth-storage (SQLite 读写分离池 / 内存高速因果图 / 向量索引 / V1-V7 迁移)
  ────────────────────────────────────────────────────────────────────────────────────────
  【核心与契约】   apeireth-core (领域实体 / 八大哲学锚 / 零开销事件总线 / 虚拟时钟)
```

---

## 二、 10 大核心 Crate 矩阵与职责

| Crate | 核心职责 | 合并与收敛来源 |
|---|---|---|
| **`apeireth-core`** | 领域实体（Episode/Note/Session/IdentityCard）、八大哲学锚、13 哲学键定义、零开销事件总线、时钟抽象 | 纯净底座，0 内部依赖 |
| **`apeireth-governance`** | 统一安全门控（V1+V2+V3 AND 门）、ABAC 权限包、三重洋葱（Principle/Permission/DSL）、自禁用防护、哈希审计链 | 收敛 `constraint`, `sovereignty`, `guard`, `action`, `council`, `onion`, `credentials`, `arbitration` |
| **`apeireth-storage`** | 读写分离 SQLite 连接池、写入队列通道（杜绝锁死）、内存高速因果与实体图谱、时序有效性过滤、向量索引 | 收敛 `memory`, `graph`, `graph-primitive`, `vector`, `context-fold`, `experience` |
| **`apeireth-protocol`** | 4 大 LLM 协议归一化（OpenAI Chat/Responses, Anthropic, Gemini, MiniMax）与 WebSocket 8 帧协议 | 收敛 `protocol`, `provider` |
| **`apeireth-tools`** | 插件化工具注册表、内置实用工具（Shell/Fetch/Browser/Search/FileSystem/Image/Repo）、沙箱隔离执行器 | 收敛 10+ 个 `tool-*` 子包 |
| **`apeireth-runtime`** | 心跳与日循环调度、`AsyncTaskStore` 协程分发、Supervisor 进程守护、全链路 Trace ID | 收敛 `runtime`, `supervisor`, `cron`, `host`, `central` |
| **`apeireth-companion`** | 生命体伙伴核心：涌现循环、Plutchik 8 维情绪与 PAD 调制、W1-W3 世界模型、E4 好奇心引擎、W6 意图预测、TP34 全流式 CoT 拆解 | 重构优化版 `apeireth-companion` |
| **`apeireth-gateway`** | 统一 REST/WS 服务、S4 出站 Default-Deny 网络拦截器、全双工 Presence 事件流、MCP Server/Client | 收敛 `api`, `gateway`, `acp`, `mcp`, `http-client` |
| **`apeireth-cli`** | 统一命令行终端与服务拉起入口 | `apeireth-cli` |
| **`apeireth-sdk`** | 多语言 / 跨平台客户端抽象与绑定接口 | `apeireth-sdk` |

---

## 三、 关键架构革新（破除 1.0 暗伤）

1. **破除 Crate 碎片化**：
   - 从 85 个分散 Crate 收敛为 10 个高内聚模块，编译耗时与链接开销降低 70%+；
   - 彻底消除数千行重复的 `Arc<dyn Trait>` 桥接与跨包包装器。
2. **安全防线真正实装（0 虚招，真防御）**：
   - **S4 出站白名单拦截**：在 `apeireth-gateway` 中直接实现物理 Socket/Reqwest 级域名白名单拦截，未授权外部请求直接 Fail-Closed；
   - **真正的 Windows 最小权限沙箱**：集成 `RestrictedToken`（剥离管理员权限）+ `JobObject`（硬限制 CPU/内存）。
3. **高并发读写分离存储**：
   - 采用只读连接池 + 专属单通道写协程（Write-Channel），保证高频 Tick、做梦与用户并发请求下 **0 `database is locked`**。
4. **全链路端到端流式（True Streaming）**：
   - 采用 TP34 状态机，原生实现 Token 级流式输出、`<think>` / `<!-- -->` 实时分离流、Presence 情绪事件流，毫秒级响应前端视觉。

---

## 四、 扩展性设计规范（如何扩展未来生态）

- **新增工具**：实现 `apeireth_tools::Tool` trait，并通过 `registry.register(MyTool)` 即可热插拔注册，无需新增 Crate。
- **新增 LLM 协议**：实现 `apeireth_protocol::ProtocolAdapter` trait，自动接入多模型路由。
- **新增存储后端**：实现 `apeireth_storage::MemoryStore` 或 `GraphStore` trait，即可平滑切换外部数据库（PostgreSQL/Redis/Qdrant）。
- **新增治理规则**：在 `apeireth_governance::RuleEngine` 中声明自定义策略即可，无需修改底层洋葱架构。
