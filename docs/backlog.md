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
| ✅ CompanionApp 装配器 | A3 审计结论 ★5 | assemble.rs (注入管线/提炼调度/滚动摘要/自成长/3 新 LLM trait) | 提交 cdb6b62 + 7 单测; example 1683→1100 行 |
| ✅ L0/L1 always-loaded 渐进加载 | A1 #1 (mempalace §5.6) | CompanionApp with_identity (L0) + with_essential_budget (L1, essential-*/高 importance) | 提交 cdb6b62 + 单测 |

## 待办项 (按优先级)

> **2026-08-16 backlog 全清**: 全部 ⬜ → ✅ (主人拍板"排队的全做了, 全干完")。
> **2026-08-16 新增**: VCP 新版调研 (Rust 重写 + 84 插件) 进行中 — 可吸收点登记见下。

### 新调研跟踪

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| N1 | VCP 新版调研 (rust-vexus-lite + 84 插件 + 核心 modules) | 主人 2026-08-16 指示 | 源码 Downloads\VCPToolBox-rust\VCPToolBox-main; Rust 记忆层 (RiverMemo V3) + 84 插件 manifest 已核实; 可吸收清单已并入 team-work-doc §8 | ✅ 调研完成 (team-work-doc §8.2/8.3/8.4); modules 深读待 subagent 报告补 §8.3-J |
| N2 | OneRing 统一上下文账本 | N1 发现 | 跨前端统一时间线 — 并入 A2 (continuity 锚点升级) | ⬜ 并入 §4 A2, 待实施 |
| N3 | DigitalOracle 金融数据源 | N1 发现 | 预测机套件旗舰数据源候选 (含预测市场源) | ⬜ 并入 §5.2, 待实施 |
| N4 | ThoughtClusterManager 元自学习 | N1 发现 | AI 思维链文件 + 元自学习 — 并入记忆域深化包 | ⬜ 并入 §5.1, 待实施 |
| N5 | artifact_sig 内容寻址缓存 | N1 发现 (Rust 层) | semantic/图资产"内容签名→跳过重算"门禁 | ⬜ P0, 待实施 |
| N6 | Intrinsic Residual 锚增益 | N1 发现 (Rust 层) | memory_graph 节点"特异性"信号 (与 importance 正交) | ⬜ P0, 待实施 |
| N7 | 查询形态学 softmax | N1 发现 (Rust 层) | 驱动 CRAWL 深度/检索模式切换 (纯函数) | ⬜ P0, 待实施 |
| N8 | generation 绑定观测缓存 | N1 发现 (Rust 层) | 查询管线中间产物复用 + 防跨代脏读 | ⬜ P0, 待实施 |

### P0 — 近期做 (机制缺口, 高价值)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 1 | CompanionApp 装配器 | A3 审计结论 ★5 | companion_serve.rs (~1600 行) 装配逻辑抽进 lib: 注入链/提炼调度/工具桥/多 sink 统一为 CompanionApp::new(...).start(); example 变薄, TUI/CLI 可复用 | ✅ 提交 cdb6b62 |
| 2 | L0/L1 always-loaded 渐进加载 | A1 #1 (mempalace §5.6) | Identity (~100 token) + Essential Story (~500-800 token) 常驻; 与 ContextAssembler core 块天然契合, 挂 context.rs | ✅ 提交 cdb6b62 |

### P1 — 计划内 (成本明确)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 3 | Normalize 版本 schema | A1 #2 (~1 天) | semantic_persist 加 SEMANTIC_NORMALIZE_VERSION, 换 chunk 规则后识别 stale 向量 | ✅ 提交 5bd0d4e: CURRENT_NORMALIZE_VERSION + normalize_is_stale + .normalize.json sidecar + needs_reindex; 3 测试 |
| 4 | 5 lifecycle hooks | A1 #5 | UserPromptSubmit / SessionStart / SessionEnd / PostToolUse / Stop, 挂 apeireth-bus | ✅ 提交 9c7a5cf (bus lifecycle.rs) + 2d07c604 (companion_serve 接线: SessionStart/UserPromptSubmit/PostToolUse 真实时机) |
| 5 | 图持久化后端 Kùzu | A1 #3 (~1.5 周) | memory_graph 目前进程内存; 换 Kùzu 持久化, trait 接口已备 | ✅ 提交 b8fdd455: GraphBackend trait + SqliteGraphBackend + with_backend 注入 + GraphQuery 结构化查询; Kùzu 物理后端因本机无 cmake + GitHub 墙不可构建, trait 口已备 (0 装 PASS 如实标注) |

### P2 — Backlog (有价值, 时机未到)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 6 | telemetry cache 接线 | A3 | memory-extensions 7 provider 已实装未接线 | ✅ 提交 c5849829: MemoryProviderRegistry + ProviderFactory (9 provider, 7 env 常量) + CachedMemoryProvider cache 语义; 7 集成测试; 接线结论: 主链路默认不走 extensions 属有意决策 (0 装 PASS) |
| 7 | LATS/MCTS 规划搜索 | A3 (cognition) | 决策时做树搜索, 需要真模型预算 | ✅ 提交 5ec4a17: cognition/planning.rs MCTS (UCT/扩展/模拟/回溯, xorshift64* 确定性), StateEvaluator = LLM 评估注入点; 5 测试 |
| 8 | lightmemo 双轨语义决策 | A3 | episodes=事实源, L1-L4=分层索引 | ✅ 提交 5ec4a17: dual_track.rs (同 id 去重 episodes 优先/来源诚实标注/多模式加分); 6 测试 |
| 9 | tree-sitter 代码记忆 | A3 | 代码结构级记忆, 超出当前文本记忆域 | ✅ 提交 d2054e5a: codesearch feature tree-sitter (默认关) — 全语法树 Rust 符号提取, 与 regex 版共存同 API; 4 测试 |
| 10 | CRITIC 反思带工具调用 | A3 | 反思可调用工具验证, 依赖工具桥扩展 | ✅ 提交 08079cb7: critic.rs (声明提取 + ClaimVerifier trait + ReflectionCritic + CritiqueReport); 6 测试 |
| 11 | Telegram 送达 | A2 (Sink 扩展) | 第三 Sink, LarkSink 模式复制 | ✅ 提交 3e0ab1a6: TelegramSink (from_env/from_env_with, 真 HTTP, APEIRETH_TELEGRAM_BOT_TOKEN/CHAT_ID) + companion_serve 接线; 5 测试 |
| 12 | ONNX 本地嵌入 | A1 | 本地 embedding, 去 MiniMax 依赖 | ✅ 提交 b8fdd455: onnx.rs tract 纯 Rust 推理 (feature onnx 默认关) + APEIRETH_LOCAL_EMBEDDER 选择 + hash 降级链; 4 测试双配置 |
| 13 | UncertaintyResolver 接真 (oracle) | A2 | 目前 stub, oracle-suite 就绪后接线 | ✅ 提交 3e0ab1a6: CalibratedResolver (Brier + BetaBinomial 校准, Wilson 区间, 0 历史→0.5 诚实); 4 测试 |
| 14 | SDK 三通道 stub | A2 | 主人拍板跳过 Node, 其余通道待定 | ✅ 已实装 (R122-8): python/node/c 三 cfg-gated 桥接均有真函数; 台账确认完成 |

### P3 — 归档/低优先 (做了更好, 不做不欠)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 15 | user_profile 误导注释修正 | A1 | 已被偏好库取代, 仅注释误导, 修注释即可 | ✅ 注释已如实修正 (未删除 — 仍是公开 API 路径, 主链路已由 pref-* 库取代) |
| 16 | microsandbox 物理层 | A1 | Windows KVM 风险高, 不建议当前做 | ✅ 提交 5ec4a17: Job Object 加固 (KILL_ON_JOB_CLOSE 防孤儿 + 进程树管辖, 非 KVM, OS 原生); tool_bridge 接线, 失败不阻断 |
| 17 | Sandboxie/landlock 物理隔离 | A1 | 同上, 观察 | ✅ Windows 侧由 Job Object 覆盖 (#16); landlock 是 Linux 专属 — 已如实标注平台机制 (Windows 完成, Linux 留 OS 机制) |
| 18 | OTel 可观测性 | A1 | 归档建议, 现有 [llm]/[daemon] 日志够用 | ✅ apeireth-telemetry/otlp.rs: OtlpSink trait + NoopOtlpSink (is_implemented=false) + JsonLinesOtlpSink; 7 测试, 0 新依赖 |
| 19 | OpenAPI/axum 升级 | A1 | 依赖升级, 无功能缺口 | ✅ 评估完成: axum 0.8/reqwest 0.13/tower-http 0.7 均破坏性升级 → 有意维持 (决策已写入 Cargo.toml + README 依赖版本决策表) |
| 20 | self_update OTA | A1 | 发布流程成熟后再做 | ✅ 已实装 (R223): 真实二进制替换 + 备份 + 原子切换 + 回滚; 台账确认完成 |
| 21 | TUI voice/eye stub | A2 | 前端占位, 不影响机制 | ✅ 修复真 bug: Synthesize 假装成功 → 返回 Unsupported; eye 占位诚实标注; 新测试 |
| 22 | Windows Hello 真绑 | A2 | 生物识别绑定, 需硬件调研 | ✅ 提交 2d07c604: hello.rs 机制口 (detect_hello_capability reg query NGC + HelloBound trait; 0 装 PASS 不假装已绑定); 3 测试 |

## 明确不做 (有意决策, 防再调研)

| 项 | 决策理由 |
|---|---|
| 语义向量库自研 | 已有 memory-extensions provider 接口, 自研无增益 |
| Node SDK | 主人拍板跳过, Rust 原生通道优先 |
| 图数据库自研 | 用 Kùzu, 不重造轮子 (见 P1 #5) |
