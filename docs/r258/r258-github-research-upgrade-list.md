# R258: GitHub 调研 + 借鉴升级清单（紧凑版）

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 每个模块对标 1-2 个 GitHub 优秀项目 + 升级点

> 主人 8/14 拍板: 调研差不多就停，不再无限做 web search。本文档基于已掌握的生态印象 + 本仓 _archived/_frozen 评估。够用即可。

---

## §1 模块对标清单（紧凑）

| 模块 | 当前状态 | GitHub 对标 | 借鉴点 |
|---|---|---|---|
| browser (apeireth-tool-browser) | 已 R140 真接 (chromiumoxide) | chromiumoxide, headless_chrome, fantoccini | chromiumoxide 已用, 0 再升级空间 |
| shell (apeireth-tool-shell) | 已 R140 真接 (tokio::process) | tokio, duct, subprocess | 已是最简 0-dep 实现, 无升级 |
| codesearch (apeireth-tool-codesearch) | R251 in-process 模式匹配 (0 ast-grep) | ripgrep, ast-grep, tantivy | 已自研 regex + 多 pattern, ripgrep 仍是 fallback |
| http (apeireth-http-client) | 已用 (reqwest 兼容层) | reqwest, hyper | reqwest 已收, 0 升级 |
| fetch (apeireth-tool-fetch) | R149-R252 全 provider (HTTP/AnySearch/Tavily/Brave/Serper/Bilibili/Deep) | none (自研聚合) | 已自研 search_aggregator, 0 缺 |
| LLM gateway | R255-R257 真接 (OpenAI Chat + Bearer) | async-openai, rig | 自研 LlmWorker (1 file), 已够用 |
| multi-agent (apeireth-council) | 30 文件, 7 advisor + collaboration/voting/debate/planner_executor | autogen-rs, langgraph-rust | 自研 depth 已超 autogen-rs (后者是骨架) |
| memory (apeireth-memory) | 15 文件 4900+ 行 (semantic_persist + three_layer + episode) | letta, mem0 | 自研 SQLite + vec 完整 |
| skills (apeireth-skills) | 11 文件, skill_executor 1349 行 + semver_strict | Anthropic skills, OpenAI skills | 已对齐 Anthropic skills spec |
| graph (apeireth-graph) | 17 文件, conditional/subgraph/cognition_graph 全齐 | langgraph-rust (Python 禁), rustworkx | 自研 depth 远超 langgraph-rust |
| tool-registry / tool-runtime / tool-approval | 已 4 件套对齐 | MCP (官方 spec) | 0 升级, 已是 MCP-aligned |
| HTTP fetch + multi-search | 已 R252 8 provider | tavily-python, brave-search | 自研聚合已超两者单体 |
| Self-Disable (apeireth-sovereignty) | 32 文件 + Kani proofs | 业界无对照 |
| Pipeline (5-stage) | g5_runtime_bridge + apeireth-pipeline | langgraph, llamaindex | 自研深度 |

**结论**: 大多数模块已经借鉴过 GitHub 优秀项目且自研深度已超。**真正的升级空间在 _frozen/_archived crate 的复活**。

---

## §2 _frozen + _archived 复活评估（按价值排序）

### Tier A: 强复活候选（实际复用）

| crate | 行数 | 现状 | 复活价值 | 路径 |
|---|---|---|---|---|
| **apeireth-tracing** | 3685 行, 9 文件 | 当前 supervisor telemetry 只有 metrics (counter/gauge/histogram), 缺 trace/span/context propagation | ★★★★★ | R259 复活: 把 span + trace + propagation + sampler 集成进 supervisor, 替代 supervisor::otel_metrics 局部实现 |
| **apeireth-task** | 1328 行, 5 文件 | 当前 runtime 用 HeartbeatScheduler (单层 time-driven), 缺 DAG + multi-stage | ★★★★ | R260 复活: 把 DAG scheduler 接到 runtime, 多步计划任务可声明式编排 |
| **apeireth-sandbox** | 2228 行, 2 文件 (real.rs 1320 行) | 当前 tool-shell 无隔离, 风险大 | ★★★★★ | R261 复活: tool-shell 调 sandbox 跑命令, 替代裸 exec |
| **apeireth-cache** | 4060 行, 10 文件 (LRU/TTL/shard/redis/stats) | 当前 tool-fetch cache.rs 118 行 (单层), memory 也无统一 cache | ★★★★ | R262 复活: 整合进 memory + tool-fetch + runtime 的 cache 调用 |

### Tier B: 中等复活候选（按需）

| crate | 行数 | 复活价值 | 路径 |
|---|---|---|---|
| **apeireth-update** | 3040 行, cosign+endpoint+signature | ★★★ | R263: GitHub Releases 集成 + cosign 验证 (未来用) |
| **apeireth-oauth** | 3472 行, device_code+flow+provider | ★★ | R264: 给 provider 加 OAuth 流程 (未来用) |
| **apeireth-tree-sitter** | 1720 行, AST+highlight+LSP | ★★ | R265: 给 codesearch 加 AST 模式 (codesearch 已自研, 锦上添花) |
| **apeireth-metrics** | 已由 supervisor 替代 | ★ | 0 复活 (重复) |
| **apeireth-credentials** | 7 文件 | ★ | R266: 整合进 LlmWorker + provider |

### Tier C: 不复活（永久 freeze）

| crate | 原因 |
|---|---|
| apeireth-image-prompt | 主人明确: 生图功能仅 stub, 后端最后 |
| apeireth-tauri-stub | 主人明确: TUI 主线, Tauri 后做 |
| apeireth-observability | 已由 supervisor telemetry + tracing 替代 |
| apeireth-plugin | 主人明确: plugin 整合进一体化模块, 不独立 |
| apeireth-keyring | 主人明确: 后端最后做 |
| apeireth-machine-id | 同上 |
| _archived/apeireth-protocol-bridge | 已被 api 替代 |
| _archived/apeireth-formal | 已被 sovereignty Kani proofs 替代 |
| _archived/apeireth-integration-r20-stage4 | R20 老测试, 历史归档 |
| _archived/apeireth-repo-analyzer, repo-scan | codesearch 已替代 |
| _archived/apeireth-rollback | 不需要 (升级用 git revert) |
| _archived/apeireth-memory-dailynote, lightmemo | 已被 apeireth-memory 替代 |
| _archived/apeireth-sdk-lark, livekit, sandbox, voice | 主人明确: 后端最后做, 外部 SDK 仅 stub |

---

## §3 R259+ 候选顺序（按"一体化优美 + 优先级"）

| R | 主题 | 评估等级 | 依赖 |
|---|---|---|---|
| R259 | **apeireth-tracing 复活** (span + trace + propagation 接入 supervisor) | ★★★★★ | 0 |
| R260 | **apeireth-task 复活** (DAG scheduler 接入 runtime) | ★★★★ | R259 |
| R261 | **apeireth-sandbox 复活** (tool-shell 隔离执行) | ★★★★★ | 0 |
| R262 | **apeireth-cache 复活** (统一 cache layer) | ★★★★ | R259 |
| R263 | apeireth-update (cosign + autoupdate) | ★★★ | R259 |
| R264 | apeireth-oauth (provider OAuth flow) | ★★ | R261 |
| R265 | apeireth-tree-sitter (AST codesearch) | ★★ | 0 |
| R266 | apeireth-credentials (整合 provider auth) | ★ | R259 |
| R267 | TUI 接 MiniMax API end-to-end (runtime_bridge dispatch_llm_task) | ★★★★★ | R257 ✅ |
| R268 | Self-Disable Kani 实战触发链 (从 verdict 到 auto-disable) | ★★★★ | 0 |
| R269 | Council 跨多 model 决策 (single LLMBackend → multi) | ★★★★ | R257 ✅ |

---

## §4 GitHub 调研结论

1. **大多数模块已借鉴且自研深度超 GitHub 同类**: council, graph, memory, fetch, skills — 不再 web search 重复调研
2. **真正的升级空间在 frozen 复活**: 13 个 frozen + 14 个 archived, 其中 6 个有强复活价值
3. **主人终极目标 = 全做 + 一体化**: R259-R269 按 Tier A 顺序推
4. **不再无限调研**: 本文档是最终状态, 下次再查就只看 Tier A 候选是否还有遗漏

---

## §5 主哲学锚对齐

- **S-1 北极星**: 借鉴上升, 不重复造轮子, _frozen 复活就是借鉴上升的体现
- **S-2 实事求是**: 调研差不多就停 (主人 8/14 拍板), 不假装还要无限调研
- **O-1 安全优先**: R261 sandbox 复活是 O-1 的核心载体
- **O-3 干到底**: Tier A 6 个目标全部要干完
- **O-5 不假装**: 调研"够用即可", 不假装做了完整 GitHub 调研报告
