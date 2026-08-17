# VCP 缺口升级计划（v2 战略 07）

```
[Document-Meta]
Document: 07-VCP-GAP-UPGRADE-PLAN.md
Version: 1.0.0-V2
R-Cycle: v2-strategy
Last-Modified: 2026-08-05
Status: DRAFT v2 (基于源码级对比 + 实测代码量)
Author: Codex (策略分析) + Leader 拍板待定
依据: docs/v2-strategy/02-VCP-DEEP-COMPARISON.md (v2) + docs/17-APEIRETH-VS-VCP-CONSUMER-PLAN.md + docs/18-VCP-BORROW-RETROSPECTIVE.md
基线: Apeireth HEAD v1.0.0 (2026-08-04, 39 crate / 2265 tests)
```

> **主哲学 anchor 6 全贯穿自检**
> S-1 主 22:33 北极星导向 - 升级服务 ASI 北极星（V0.5 24 维 -> ASI 更精准测量）
> S-2 主 17:43 实事求是 - 不假装"已对齐 VCP"，明确生态断层，通过 MCP 桥接而非自研
> O-5 主 17:58 不假装 - 不假装"已经够用"，不假装"能 1:1 替代 VCP"
> O-2 主 19:33 走在前人经验上 - 每条都字段级引用 VCP 真代码（文件 + 行号）
> O-3 主 23:44 干到底 - 给出 P0/P1/P2 优先级 + DoD，不空谈
> O-4 主 00:56 任何人都能接手 - 表格化 + 每条带目标文件路径 + 借鉴 ID

---

## §0. 与既有文档的关系（不重复造轮子）

| 本文档职责 | 引用 |
|---|---|
| **总体战区 / 5 战场 / 身份定位** | `00-VISION.md` |
| **战区对位 / 数字对比** | `02-VCP-DEEP-COMPARISON.md` |
| **18 个月时间表** | `03-EXTREME-PLAN.md` |
| **Crate 重组（删 4 / 增 5 / 强化）** | `04-CRATE-CONSOLIDATION.md` |
| **TUI 9 器官升级路线图** | `06-TUI-UPGRADE-ROADMAP.md` (Step 1 ✅ round17-25 / Step 2-3 ⏸️ 暂存) |
| **VCP 真源码逐文件调研记录** | `docs/17-APEIRETH-VS-VCP-CONSUMER-PLAN.md` |
| **VCP 借鉴 19 文件清单** | `docs/18-VCP-BORROW-RETROSPECTIVE.md` |
| **哲学层 v4 / v4.1 / 双洋葱** | `docs/architecture-v4-living-intelligence.md` + `docs/r14-design/onion-wall-architecture-2026-07-31.md` |

**本文档新增内容**：在既有战略基础上，**逐项**给出 VCP 源文件 + 行号 + 目标 Apeireth crate/文件 + 借鉴 ID + DoD，让 R18+ 施工团队能直接按表领活儿。

---

## §1. TL;DR

**Apeireth 现状**（v1.0.0，HEAD `3cab8f32`）：
- 39 workspace members（37 个真实代码 / 4 个真小），2.6 MB Rust
- 战区 2（LLM 网关）= VCP `chatCompletionHandler.js` 59KB 的 7.6 倍（449KB）
- 战区 5（工具协议）= VCP `dynamicToolRegistry.js` + `Plugin.js` 的强类型 Rust 替身
- **Self-Disable / 双洋葱 / 编译期形式化 = 全行业唯一护城河**

**真正缺的 13 项**（按优先级）：
- **P0（4 项）**：MCP 客户端/服务端、图编排（LangGraph 风格）、向量检索后端、小模型分类器
- **P1（5 项）**：Response Replay Cache、语义模型路由、角色划分标记、tiktoken 精确计数、多语言 SDK
- **P2（4 项）**：日志回放、Kani 形式化验证、浏览器自动化（via MCP）、多模态生成（via MCP）

**明确不做（4 项）**：
- 自研 Web Admin UI（已砍，交给别的团队）
- 自研 Tauri 桌面端（R17 战役 3 已砍）
- 砍 5 个哲学器官（每个 14-33KB 真实代码，合并丢清晰度）
- 重写 9 键哲学守门（V3 LOCKED，仅在 v4.1 提议 12 键 v2 不冻结）

**关键战略判断**（引自 `00-VISION.md`）：
> Apeireth 不要做第 11 个 LLM 网关，也不要做"Agent Runtime"——要做"VCP 的 Rust 重写 + 独家的形式化安全 + Self-Disable + 双洋葱"，在 5 个战场上同时打，但用 Rust 类型系统和哲学架构形成 VCP 永远无法复制的差异化。

---

## §2. 缺口总览（13 项 + 4 项不做）

| # | 优先级 | 缺口 | VCP 源文件 | Apeireth 现状 | 目标 crate / 文件 |
|---|--------|------|-----------|------------|------------------|
| 1 | P0 | **MCP 全适配** | VCP 无 | `apeireth-mcp` 仅 Dockerfile（README §v2 新增 Features）| `crates/apeireth-mcp/src/` 新建 |
| 2 | P0 | **图编排（LangGraph）** | VCP 无 | `apeireth-graph` 仅 Dockerfile | `crates/apeireth-graph/src/` 新建 |
| 3 | P0 | **向量检索后端** | `KnowledgeBaseManager.js` + `Vexus-Lite` | `apeireth-vector` 缺 `workspace.members` | `crates/apeireth-vector/src/` 启用 + `apeireth-memory` 集成 |
| 4 | P0 | **小模型工具分类器** | `dynamicToolRegistry.js:40-80 CATEGORY_RULES` | `apeireth-tool-registry` 68KB 无分类器 | `apeireth-tool-registry/src/categorizer.rs` 新建 |
| 5 | P1 | **Response Replay Cache** | `chatCompletionHandler.js:73-156 ResponseReplayCache` | `apeireth-api` 197KB 无 | `apeireth-api/src/replay_cache.rs` 新建 |
| 6 | P1 | **语义模型路由** | `semanticModelRouter.js` 17KB + `VCPModelAuto` | `apeireth-asi` 92KB 是 ASI 测量非路由 | `apeireth-pipeline/src/model_router.rs` 新建 |
| 7 | P1 | **角色划分标记** | `roleDivider.js` 16KB `<ROLE_DIVIDE_*>` | `apeireth-pipeline` 无 | `apeireth-pipeline/src/role_divider.rs` 新建 |
| 8 | P1 | **tiktoken 精确计数** | `finalContextStore.js` 11KB + tiktoken | `apeireth-pipeline/src/token_budget.rs` 用 VCP 同源启发式 | 引入 `tiktoken-rs` crate，替换 `token_pieces()` |
| 9 | P1 | **多语言 SDK** | VCP 无 | `apeireth-sdk` 仅 Dockerfile | `crates/apeireth-sdk/` 新建（PyO3 + napi-rs + cgo） |
| 10 | P2 | **日志回放** | `vcpLogReplayManager.js` 19KB | 无 | `apeireth-memory/src/replay.rs` 新建 |
| 11 | P2 | **形式化验证（Kani）** | VCP 无 | `apeireth-formal` skeleton + `PermissionLayerConfig` POD | `apeireth-formal/src/kani_harness.rs` 实装 |
| 12 | P2 | **浏览器自动化** | `browserRuntimeManager.js` 26KB + `ChromeBridge` plugin | 无 | 不自研，通过 `apeireth-mcp` 接 Playwright MCP server |
| 13 | P2 | **多模态生成** | 9 个 Gen 插件（ComfyUI / Flux / Doubao / GPT-Image / Gemini / Agnes / AgnesVideo / DMX / NanoBanana） | 无 | 不自研，通过 `apeireth-mcp` 接对应 MCP server |

### 不做的 4 项（守住护城河，不被 VCP 生态绑架）

| 不做项 | 原因 | 引用 |
|---|---|---|
| 自研 Web Admin UI | 战区 6 已砍，R17 战役 3 主人决策 | `00-VISION.md` §战区 6 |
| 自研 Tauri 桌面端 | R17 战役 3 已砍 | 同上 |
| 砍 5 个哲学器官（perception/cognition/consciousness/motivation/life-force）| 每个 14-33KB 真实代码，合并丢清晰度 | `04-CRATE-CONSOLIDATION.md` §5 |
| 重写 V3 9 键哲学守门 | V3 LOCKED，v4.1 提议 12 键 v2 不冻结 | `architecture-v4-1-living-intelligence-update.md` §15 |

---

## §3. P0 阻塞级（不开车上不了战场，4 项）

### P0-1. MCP 全适配（命脉）

| 项 | 值 |
|---|---|
| **VCP 源** | 无（VCP 没做 MCP）|
| **Apeireth 现状** | `apeireth-mcp` 仅 Dockerfile（README §v2 新增 Features）|
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-mcp\src\` 新建 |
| **依赖** | `rmcp` (官方 Rust MCP SDK) 或自实现 stdio/SSE/HTTP-streamable 三 transport |
| **借鉴 ID** | BORROW-MCP-001（无 VCP 源，新增能力）|
| **DoD** | 100% MCP 规范测试通过；能被 Claude Desktop / Cursor / Cline 识别；tool 调用延迟 < 5ms（本地）/ < 50ms（含 transport）|
| **关键决策** | **type-safe tool schema**（Rust trait 编译期保证）= VCP 永远做不到的护城河 |

### P0-2. 图编排（LangGraph 风格）

| 项 | 值 |
|---|---|
| **VCP 源** | 无（VCP 没做图编排）|
| **Apeireth 现状** | `apeireth-graph` 仅 Dockerfile（README §v2 新增 Features）|
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-graph\src\` 新建 |
| **依赖** | 自实现 + 集成 `apeireth-supervisor` 作为执行器 |
| **借鉴 ID** | BORROW-LANGGRAPH-001（参考 LangGraph / AutoGen 设计）|
| **DoD** | 支持节点并行执行；支持动态图重写；支持 checkpoint（状态保存）；P99 checkpoint 写入 < 10ms；支持时间回溯 |
| **关键决策** | 与 `apeireth-council` 7 advisor 集成，每个 advisor 可作为图节点 |

### P0-3. 向量检索后端

| 项 | 值 |
|---|---|
| **VCP 源** | `KnowledgeBaseManager.js:25-95`（better-sqlite3 + Vexus-Lite Rust NAPI）|
| **Apeireth 现状** | `apeireth-vector` 缺 `workspace.members`（README §v2 标 缺 workspace.members, T2 顺手补）|
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-vector\src\` 启用 + `apeireth-memory/src/` 集成 |
| **依赖** | `sqlite-vec` 0.32（锁版本）或 `LanceDB` |
| **借鉴 ID** | BORROW-VECTOR-001（字段级镜像 VCP dimension / tagIndex / writeLease 模式，但用 Rust trait 不用 NAPI）|
| **DoD** | 100k tokens 检索 P99 < 100ms；语义检索 + 时间检索 + 标签检索 3 维度统一；用户画像准确率 ≥ 80% |
| **关键决策** | VCP 用 Vexus-Lite NAPI + better-sqlite3 双进程双写租约，Apeireth 直接用 `rusqlite` 单进程单写，**不需要写租约** |

### P0-4. 小模型工具分类器

| 项 | 值 |
|---|---|
| **VCP 源** | `dynamicToolRegistry.js:40-80 CATEGORY_RULES`（7 类：search/file_code/image_media/memory_knowledge/agent_task/communication/data）|
| **Apeireth 现状** | `apeireth-tool-registry` 68KB 无分类器，需手动注册 |
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-tool-registry\src\categorizer.rs` 新建 |
| **借鉴 ID** | BORROW-CATEGORIZER-001（7 类 1:1 镜像 VCP `CATEGORY_RULES`）|
| **DoD** | 7 类分类与 VCP 同源（编译期 hardcode `CATEGORY_COUNT = 7`）；支持小模型 endpoint（OpenAI compat）；支持主备模型降级 |
| **关键决策** | VCP 用 JS 字符串匹配 + RAG 嵌入；Apeireth 用 Rust enum + 编译期 hardcode + 可选 LLM endpoint |

---

## §4. P1 重要级（功能完整度，5 项）

### P1-1. Response Replay Cache

| 项 | 值 |
|---|---|
| **VCP 源** | `chatCompletionHandler.js:73-156 ResponseReplayCache`（key = `clientIp::messageId`，LRU 100 entries）|
| **Apeireth 现状** | `apeireth-api` 197KB 无 |
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-api\src\replay_cache.rs` 新建 |
| **借鉴 ID** | BORROW-REPLAY-001 |
| **DoD** | 相同 `client_ip::message_id` 命中缓存时跳过工具链；status_code 200-499 才缓存；LRU 上限可配；`__vcpReplayCacheRecorderInstalled` 防止双装 |
| **关键决策** | VCP 用 `Map<key, {statusCode, headers, chunks}>`；Apeireth 用 `Arc<DashMap<Key, Entry>>`（tokio 友好）|

### P1-2. 语义模型路由

| 项 | 值 |
|---|---|
| **VCP 源** | `semanticModelRouter.js` 17KB + `VCPModelAuto`（客户端不指定模型，按余弦相似度 + preset 路由）|
| **Apeireth 现状** | `apeireth-asi` 92KB 是 ASI 测量公式（V0.5 24 维），不是模型路由 |
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-pipeline\src\model_router.rs` 新建 |
| **借鉴 ID** | BORROW-MODEL-ROUTER-001 |
| **DoD** | 客户端可省略 `model` 字段；按 query 语义 + preset 表自动选模型；路由决策可审计（写 `action_stream`）|
| **关键决策** | VCP 用纯 JS cosine；Apeireth 用 embedding service + 编译期 hardcode preset 表 |

### P1-3. 角色划分标记

| 项 | 值 |
|---|---|
| **VCP 源** | `roleDivider.js` 16KB（`<ROLE_DIVIDE_*>` 标记 + 递归切分）|
| **Apeireth 现状** | `apeireth-pipeline` 无 |
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-pipeline\src\role_divider.rs` 新建 |
| **借鉴 ID** | BORROW-ROLE-DIVIDER-001 |
| **DoD** | 解析 `<ROLE_DIVIDE_USER>` / `<ROLE_DIVIDE_SYSTEM>` / `<ROLE_DIVIDE_TOOL>` 标记；递归切分 nested；与 `apeireth-protocol::NormalizedMessage` 集成 |

### P1-4. tiktoken 精确 token 计数

| 项 | 值 |
|---|---|
| **VCP 源** | `finalContextStore.js` 11KB + tiktoken（精确 token 计数）|
| **Apeireth 现状** | `apeireth-pipeline/src/token_budget.rs` 用 VCP 同源启发式 `token_pieces()`（拉丁 1 word + CJK 1 char）|
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-pipeline\src\token_budget.rs` 替换 |
| **依赖** | `tiktoken-rs` crate（tiktoken Rust 绑定）|
| **借鉴 ID** | BORROW-TOKEN-COUNT-001 |
| **DoD** | 与 OpenAI tiktoken 1:1 一致；cl100k_base / o200k_base 支持；3 const 真值不变（`LIGHT_LIST=15` / `DEFAULT_BRIEF=6` / `MAX_INJECTION_CHARS=16000`）|
| **关键决策** | 保留 VCP 启发式作为 fallback，精确计数作为 P1 选项（编译期 feature flag）|

### P1-5. 多语言 SDK

| 项 | 值 |
|---|---|
| **VCP 源** | 无（VCP 是 JS+Python 混合，用户改源码接入）|
| **Apeireth 现状** | `apeireth-sdk` 仅 Dockerfile（README §v2）|
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-sdk\` 新建 |
| **依赖** | `PyO3` + `maturin`（Python） / `napi-rs`（TS） / `cgo`（Go） / `C-ABI` |
| **借鉴 ID** | BORROW-SDK-001（无 VCP 源，新增能力）|
| **DoD** | `pip install apeireth` 一行可用；`npm install @apeireth/sdk` 一行可用；100% API 覆盖 + 完整类型提示；与 `apeireth-bus` 5 层通信总线对接 |
| **关键决策** | 不与 `apeireth-pybridge`（已有）冲突——SDK 是外向接口，pybridge 是内向 Rust-Python 桥 |

---

## §5. P2 加分级（形式化 + 生态桥接，4 项）

### P2-1. 日志回放

| 项 | 值 |
|---|---|
| **VCP 源** | `vcpLogReplayManager.js` 19KB（重放历史对话用于调试 / 复现 / 训练）|
| **Apeireth 现状** | `apeireth-memory` 120KB 无 replay API |
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-memory\src\replay.rs` 新建 |
| **借鉴 ID** | BORROW-REPLAY-LOG-001 |
| **DoD** | 按 `session_id` 重放 episode 流；按时间范围重放；可注入 mock LLM 响应 |

### P2-2. 形式化验证（Kani）

| 项 | 值 |
|---|---|
| **VCP 源** | 无（VCP 无形式化）|
| **Apeireth 现状** | `apeireth-formal` skeleton + `PermissionLayerConfig` POD + `double_onion_sample` Kani harness |
| **目标路径** | `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-formal\src\kani_harness.rs` 实装 |
| **借鉴 ID** | BORROW-FORMAL-001（无 VCP 源，引入 Kani 验证双锁 AND 门不变量）|
| **DoD** | Kani harness 覆盖 5 大 Self-Disable 机制；Creusot 验证 6 历史流 append-only 触发器；CI 跑通 `cargo kani` |
| **关键决策** | 这是 VCP 永远追不上的护城河——VCP 只能测试，Rust + Kani 能证明 |

### P2-3. 浏览器自动化（via MCP）

| 项 | 值 |
|---|---|
| **VCP 源** | `browserRuntimeManager.js` 26KB + `Plugin/ChromeBridge` 真跑 |
| **Apeireth 现状** | 无 |
| **策略** | **不自研**，通过 P0-1 MCP 接 Playwright MCP server |
| **借鉴 ID** | BORROW-MCP-PLAYWRIGHT-001（间接通过 MCP）|
| **DoD** | `apeireth-mcp` 能加载 Playwright MCP server 配置文件；tool call 转 MCP 标准协议 |

### P2-4. 多模态生成（via MCP）

| 项 | 值 |
|---|---|
| **VCP 源** | 9 个 Gen 插件（ComfyUI / Flux / Doubao / GPT-Image / Gemini / Agnes / AgnesVideo / DMX / NanoBanana）|
| **Apeireth 现状** | 无 |
| **策略** | **不自研**，通过 P0-1 MCP 接对应 MCP server |
| **借鉴 ID** | BORROW-MCP-MULTIMODAL-001（间接通过 MCP）|
| **DoD** | MCP 接入 ≥ 3 个多模态 MCP server；`apeireth-tools` 暴露统一 multimodal interface |
| **关键决策** | 多模态生成是 API 服务，Apeireth 应做协议网关而非 GPU 推理基础设施 |

---

## §6. 借鉴账本（接续 docs/18 §3.2 13 + 7 项）

**既有 19 项借鉴（已完成）**：见 `docs/18-VCP-BORROW-RETROSPECTIVE.md` §3.2（13 项）+ §3.3（7 项新增）。

**本文档新增 13 项借鉴 ID**：

| ID | 借鉴项 | VCP 源 | 目标 | 状态 |
|---|---|---|---|---|
| BORROW-CATEGORIZER-001 | 小模型工具分类器 | `dynamicToolRegistry.js:40-80` | `apeireth-tool-registry/src/categorizer.rs` | 待开工 |
| BORROW-REPLAY-001 | Response Replay Cache | `chatCompletionHandler.js:73-156` | `apeireth-api/src/replay_cache.rs` | 待开工 |
| BORROW-MODEL-ROUTER-001 | 语义模型路由 | `semanticModelRouter.js` + `VCPModelAuto` | `apeireth-pipeline/src/model_router.rs` | 待开工 |
| BORROW-ROLE-DIVIDER-001 | 角色划分标记 | `roleDivider.js` | `apeireth-pipeline/src/role_divider.rs` | 待开工 |
| BORROW-TOKEN-COUNT-001 | tiktoken 精确计数 | `finalContextStore.js` + tiktoken | `apeireth-pipeline/src/token_budget.rs` 替换 | 待开工 |
| BORROW-VECTOR-001 | 向量检索后端 | `KnowledgeBaseManager.js` + Vexus-Lite | `apeireth-vector/` + `apeireth-memory/` 集成 | 待开工 |
| BORROW-REPLAY-LOG-001 | 日志回放 | `vcpLogReplayManager.js` | `apeireth-memory/src/replay.rs` | 待开工 |
| BORROW-MCP-001 | MCP 全适配 | 无 | `apeireth-mcp/src/` 新建 | Dockerfile only |
| BORROW-MCP-PLAYWRIGHT-001 | Playwright via MCP | `browserRuntimeManager.js` 间接 | `apeireth-mcp` 加载 Playwright MCP server | 待开工 |
| BORROW-MCP-MULTIMODAL-001 | 多模态 via MCP | 9 个 Gen 插件间接 | `apeireth-mcp` 加载多模态 MCP server | 待开工 |
| BORROW-SDK-001 | 多语言 SDK | 无 | `apeireth-sdk/` 新建 | Dockerfile only |
| BORROW-LANGGRAPH-001 | 图编排 | 无（参考 LangGraph）| `apeireth-graph/src/` 新建 | Dockerfile only |
| BORROW-FORMAL-001 | Kani 形式化验证 | 无 | `apeireth-formal/src/kani_harness.rs` | skeleton |

**累计借鉴 ID**：19（既有）+ 13（本文新增）= **32 项**。

---

## §7. 验证指标（DoD 矩阵）

### P0 完成判定（Month 2-4 完成）

| 指标 | 目标 | 验证方式 |
|---|---|---|
| MCP 规范测试 | 100% 通过 | `cargo test -p apeireth-mcp` |
| MCP 客户端延迟 | < 5ms（本地）| bench |
| MCP 端到端延迟 | < 50ms（含 transport）| bench |
| 图编排 checkpoint 写入 | P99 < 10ms | bench |
| 图编排并行节点 | ≥ 4 节点并发 | `cargo test -p apeireth-graph` |
| 向量检索 100k tokens | P99 < 100ms | bench（`lancedb` / `sqlite-vec`）|
| 用户画像准确率 | ≥ 80% | 标注测试集 |
| 小模型分类器 7 类 | 100% 覆盖 | `cargo test -p apeireth-tool-registry` |
| **SWE-bench Verified 阶段 0 目标** | ≥ 5% | `cargo bench -p apeireth-bench`（README v2 §SWE-bench）|

### P1 完成判定（Month 5-8 完成）

| 指标 | 目标 | 验证方式 |
|---|---|---|
| Replay Cache 命中率 | ≥ 30%（重复请求）| shadow 测试 |
| 模型路由决策可审计 | 100% 写 `action_stream` | e2e 测试 |
| 角色划分解析 | 100% 通过 fixture | `cargo test` |
| tiktoken 精度 | 与 OpenAI 1:1（误差 < 0.1%）| 对照测试 |
| Python SDK `pip install` | 一行可用 | 集成测试 |
| TS SDK `npm install` | 一行可用 | 集成测试 |
| **SWE-bench Verified 阶段 2 目标** | ≥ 50% | bench |

### P2 完成判定（Month 9-12 完成）

| 指标 | 目标 | 验证方式 |
|---|---|---|
| 日志回放 | session_id 范围 100% 还原 | fixture 测试 |
| Kani harness 覆盖率 | 5 Self-Disable 机制 100% | `cargo kani` |
| Playwright MCP 接入 | 浏览器自动化 5 个场景跑通 | e2e |
| 多模态 MCP 接入 | ≥ 3 个 server | e2e |
| **SWE-bench Verified 阶段 3 目标** | ≥ 60% | bench |
| **V0.5 24 维公式 R18 真测** | 三值更新（V1141/V1131/V1136）| dashboard |

### 守住不做的 4 项

| 不做项 | 验证方式 |
|---|---|
| 不自研 Web Admin | `git grep -i "vue\|admin"` 应只在 `apeireth-web/` 出现，且仅 Dockerfile |
| 不自研 Tauri | `git grep -i "tauri"` 应只在 `apeireth-desktop/` 出现，且标记 DEPRECATED |
| 不砍 5 个哲学器官 | 5 个 crate 各 ≥ 10KB，`cargo build -p` 全过 |
| 不重写 V3 9 键 | `git grep "PhilosophyKey"` 应只在原文件出现 |

---

## §8. 与 R18+ 施工期衔接

本文档与 `03-EXTREME-PLAN.md` 的 5 阶段对应：

| R18+ 阶段 | 本文档对应 P0/P1/P2 项 |
|---|---|
| 阶段 0（Month 0-1）清理与强化 | P0-1 MCP skeleton 启用；P0-3 vector workspace member 补 |
| 阶段 1A（Month 2-3）MCP 全适配 | P0-1 完整实现 |
| 阶段 1B（Month 3-4）Memory 升级 | P0-3 + P0-4 |
| 阶段 1C（Month 2-4）TUI 增强 | 引 `06-TUI-UPGRADE-ROADMAP.md` |
| 阶段 2A（Month 5-6）图编排 | P0-2 |
| 阶段 2B（Month 6-8）Multi-Agent | 强化 `apeireth-council`（非本文档项） |
| 阶段 3A（Month 9-10）SDK 多语言 | P1-5 |
| 阶段 3B（Month 10-11）框架适配器 | 走 MCP |
| 阶段 4（Month 12-18）| P2 全做 |

---

## §9. 引用清单（必读 + 字段级引用）

### Apeireth 既有文档（必读）

| 文档 | 路径 | 用途 |
|---|---|---|
| `00-VISION.md` | `docs/v2-strategy/00-VISION.md` | 5 战区定位 |
| `02-VCP-DEEP-COMPARISON.md` | `docs/v2-strategy/02-VCP-DEEP-COMPARISON.md` | 数字对比 |
| `03-EXTREME-PLAN.md` | `docs/v2-strategy/03-EXTREME-PLAN.md` | 18 个月时间表 |
| `04-CRATE-CONSOLIDATION.md` | `docs/v2-strategy/04-CRATE-CONSOLIDATION.md` | crate 重组 |
| `06-TUI-UPGRADE-ROADMAP.md` | `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` | TUI 9 器官 |
| `17-APEIRETH-VS-VCP-CONSUMER-PLAN.md` | `docs/17-APEIRETH-VS-VCP-CONSUMER-PLAN.md` | VCP 12 周调研 |
| `18-VCP-BORROW-RETROSPECTIVE.md` | `docs/18-VCP-BORROW-RETROSPECTIVE.md` | VCP 19 项借鉴 |
| `architecture-v4-living-intelligence.md` | `docs/architecture-v4-living-intelligence.md` | 哲学层 v4 |
| `architecture-v4-1-living-intelligence-update.md` | `docs/architecture-v4-1-living-intelligence-update.md` | 哲学层 v4.1 |
| `onion-wall-architecture-2026-07-31.md` | `docs/r14-design/onion-wall-architecture-2026-07-31.md` | 双洋葱 |
| `borrowed-from-projects.md` | `docs/stage3-blueprints/borrowed-from-projects.md` | 30 项调研账本 |
| `apeireth-tool-registry/lib.rs` | `crates/apeireth-tool-registry/src/lib.rs` | 编译期 hardcode 范例 |

### VCP 真源（字段级引用）

| VCP 源文件 | 路径 | 本文档引用行 |
|---|---|---|
| `dynamicToolRegistry.js` | `research/source/vcptoolbox/modules/dynamicToolRegistry.js` | 7-21 / 40-80 |
| `chatCompletionHandler.js` | `research/source/vcptoolbox/modules/chatCompletionHandler.js` | 22-28 / 73-156 |
| `semanticModelRouter.js` | `research/source/vcptoolbox/modules/semanticModelRouter.js` | 全文件 |
| `roleDivider.js` | `research/source/vcptoolbox/modules/roleDivider.js` | 全文件 |
| `finalContextStore.js` | `research/source/vcptoolbox/modules/finalContextStore.js` | tiktoken 集成 |
| `KnowledgeBaseManager.js` | `research/source/vcptoolbox/KnowledgeBaseManager.js` | 25-95 |
| `vcpLogReplayManager.js` | `research/source/vcptoolbox/modules/vcpLogReplayManager.js` | 全文件 |
| `browserRuntimeManager.js` | `research/source/vcptoolbox/modules/browserRuntimeManager.js` | 全文件 |
| `Plugin/ChromeBridge/` | `research/source/vcptoolbox/Plugin/ChromeBridge/` | 浏览器自动化 |
| 9 个 Gen 插件 | `research/source/vcptoolbox/Plugin/*Gen*/` | 多模态 |

---

## §10. 一句话总结（v2）

**Apeireth 不是要变成第 11 个 VCP，是要做"VCP 的 Rust 重写 + 编译期钉死 + 双洋葱形式化 + Self-Disable 护城河"——13 项缺口按 P0/P1/P2 顺序补，前 4 项 P0 决定能否上战场，最后 4 项 P2 通过 MCP 桥接而非自研，借力 85 个 VCP 插件生态而非对抗。**

_Last update_: 2026-08-05 (v2.0.0 草案)
_Status_: DRAFT v2，待 Leader 拍板（活跃任务 4494b133 in_progress 与本任务并行，需增量同步）
