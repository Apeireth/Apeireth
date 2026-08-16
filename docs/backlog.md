# 调研/审计 Backlog 台账

> 规范 00 (文档同步自觉) 的落地载体: 凡调研/审计发现的、当下不做的项, 必须显式登记于此,
> 不得散落在聊天记录里丢失。本文件是唯一权威台账, 完成即划 ✅ 并注明提交/文档位置。

## 审计来源索引

| 代号 | 审计 | 日期 | 结果去向 |
|---|---|---|---|
| A1 | 代码 TODO 全量审计 (mempalace/VCP/Zep/Mem0 等借鉴点落地核对) | 2026-08-16 | 本节 |
| A2 | Handoff 交接审计 (docs/CONTEXT-HANDOVER.md 逐项核对) | 2026-08-16 | 本节 |
| A3 | 记忆域深度调研 (memory-research.md §五 backlog) | 2026-08-16 | 本节 |

## 已完成项 (✅)

| 项 | 来源 | 完成方式 | 证据 |
|---|---|---|---|
| ✅ 记忆分层排名 (imp×3+access×0.3+group+recency) | A3 记忆 v2 升级包 | memory_extractor.rs rank_memory_entries | 提交 + 单测 |
| ✅ 提炼器对账化 (ADD/UPDATE/DELETE + tomb 逻辑删除) | A3 (Mem0 借鉴) | memory_extractor.rs apply_reconcile | 提交 + 单测 |
| ✅ 反思触发 (周期 OR importance 和 > 150) | A3 (反思机制) | reflection.rs importance_surge | 提交 + 单测 |
| ✅ 做梦去重 (文本级近重复) | A3 (做梦质量) | dream.rs dedup_textual | 提交 + 单测 |
| ✅ 滚动摘要持久化 + sum-* 链 | A3 (上下文管理) | companion_serve.rs summarize_dialog | 提交 |
| ✅ 事实图 (双时态边 + 带权链接 + crawl 检索 + 注入) | A3 (Zep+A-MEM) | memory_graph.rs | 提交 + 3 单测 |
| ✅ 状态感知 (时刻/节律/目标/约定/情绪注入) | A2 模块 1 | companion_serve.rs inject_state | 提交 |
| ✅ 目标驱动 (5 工具 + 权限洋葱) | A2 模块 6 | goal_tools.rs + tool_bridge.rs with_goals | 提交 + 单测 |
| ✅ 主动送达 (SSE 广播 + Lark) | A2 模块 4 | MultiSink | 提交 |
| ✅ 上下文滚动摘要 | A2 模块 3 | companion_serve.rs | 提交 |
| ✅ 深度反思 (ReflectionReflector trait) | A2 模块 5 | reflection.rs | 提交 |
| ✅ 统一注入管线 ContextAssembler (核心块保护 + 预算截断) | A3 (L0/L1 前置) | context.rs | 提交 986358e + 3 单测 |

## 待办项 (按优先级)

### P0 — 近期做 (机制缺口, 高价值)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 1 | CompanionApp 装配器 | A3 审计结论 ★5 | companion_serve.rs (~1600 行) 装配逻辑抽进 lib: 注入链/提炼调度/工具桥/多 sink 统一为 CompanionApp::new(...).start(); example 变薄, TUI/CLI 可复用 | ⬜ |
| 2 | L0/L1 always-loaded 渐进加载 | A1 #1 (mempalace §5.6) | Identity (~100 token) + Essential Story (~500-800 token) 常驻; 与 ContextAssembler core 块天然契合, 挂 context.rs | ⬜ |

### P1 — 计划内 (成本明确)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 3 | Normalize 版本 schema | A1 #2 (~1 天) | semantic_persist 加 SEMANTIC_NORMALIZE_VERSION, 换 chunk 规则后识别 stale 向量 | ⬜ |
| 4 | 5 lifecycle hooks | A1 #5 | UserPromptSubmit / SessionStart / SessionEnd / PostToolUse / Stop, 挂 apeireth-bus | ⬜ |
| 5 | 图持久化后端 Kùzu | A1 #3 (~1.5 周) | memory_graph 目前进程内存; 换 Kùzu 持久化, trait 接口已备 | ⬜ |

### P2 — Backlog (有价值, 时机未到)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 6 | telemetry cache 接线 | A3 | memory-extensions 7 provider 已实装未接线 | ⬜ |
| 7 | LATS/MCTS 规划搜索 | A3 (cognition) | 决策时做树搜索, 需要真模型预算 | ⬜ |
| 8 | lightmemo 双轨语义决策 | A3 | episodes=事实源, L1-L4=分层索引 | ⬜ |
| 9 | tree-sitter 代码记忆 | A3 | 代码结构级记忆, 超出当前文本记忆域 | ⬜ |
| 10 | CRITIC 反思带工具调用 | A3 | 反思可调用工具验证, 依赖工具桥扩展 | ⬜ |
| 11 | Telegram 送达 | A2 (Sink 扩展) | 第三 Sink, LarkSink 模式复制 | ⬜ |
| 12 | ONNX 本地嵌入 | A1 | 本地 embedding, 去 MiniMax 依赖 | ⬜ |
| 13 | UncertaintyResolver 接真 (oracle) | A2 | 目前 stub, oracle-suite 就绪后接线 | ⬜ |
| 14 | SDK 三通道 stub | A2 | 主人拍板跳过 Node, 其余通道待定 | ⬜ |

### P3 — 归档/低优先 (做了更好, 不做不欠)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 15 | user_profile 误导注释修正 | A1 | 已被偏好库取代, 仅注释误导, 修注释即可 | ⬜ |
| 16 | microsandbox 物理层 | A1 | Windows KVM 风险高, 不建议当前做 | ⬜ |
| 17 | Sandboxie/landlock 物理隔离 | A1 | 同上, 观察 | ⬜ |
| 18 | OTel 可观测性 | A1 | 归档建议, 现有 [llm]/[daemon] 日志够用 | ⬜ |
| 19 | OpenAPI/axum 升级 | A1 | 依赖升级, 无功能缺口 | ⬜ |
| 20 | self_update OTA | A1 | 发布流程成熟后再做 | ⬜ |
| 21 | TUI voice/eye stub | A2 | 前端占位, 不影响机制 | ⬜ |
| 22 | Windows Hello 真绑 | A2 | 生物识别绑定, 需硬件调研 | ⬜ |

## 明确不做 (有意决策, 防再调研)

| 项 | 决策理由 |
|---|---|
| 语义向量库自研 | 已有 memory-extensions provider 接口, 自研无增益 |
| Node SDK | 主人拍板跳过, Rust 原生通道优先 |
| 图数据库自研 | 用 Kùzu, 不重造轮子 (见 P1 #5) |
