# Apeireth Crate Index (1.0)

> 对齐 master 实际代码 (2026-08-18)。85 crates，按字母序。完整描述来自各 crate Cargo.toml。

| Crate | Description |
|---|---|
| \$n\ | Apeireth 1.0 release 必做 #10: i18n 骨架 (1:1 翻译 v0.9.21 商业版 out/main 中 i18next@^26.0.5 + react-i18next@^17.0.3 集成面; 5 语言 en/zh-CN/ja/fr/de, TOML 格式 locales/*.toml 编译期 hardcode, 12 类别 69 keys 100% 翻译覆盖: 5 nav + 6 tools + 9 organs + 5 R-Measure + 6 哲学锚 + 8 承诺 + 5 Provider + 4 SDK + 3 observability + 5 鉴权 + 10 通用 + 3 readiness) |
| \$n\ | Apeireth 纯 Rust SDK 客户端 (1.0 release #2 install), 0 PyO3, 0 .venv |
| \$n\ | Apeireth 动机器官 (A11.2 落点 — R14 Phase 4 动机/价值器官: MotivationDrive trait + SGI 单字段 (sgi_current+sgi_history 二元) + C-SGI-1~7 七条硬约束编译时 hardcode + E 层校验 + ReflectionAuditor 告警) |
| \$n\ | Apeireth 感知器官 (A9 落点 — R14 Phase 3 外部输入接入层: 信号/IO/Token 流 → 统一 PerceptionEvent) |
| \$n\ | Apeireth 记忆子系统 (Episode/Note/Session SQLite 存储 + BM25 检索) — R14 Phase 1 主目标 (V1130 wallclock 2.5s) |
| \$n\ | Apeireth 价值器官 (A11.3 落点 — R14 Phase 4 动机/价值评估: ValueEvaluation + ValuePrioritization + 5 层原则洋葱一致性 + motivation_score 0.85 门槛) |
| \$n\ | Apeireth 升级器官 (A15 落点 — R14 Phase 5 OTA 升级 + sandbox-validator + 5 重治理) |
| \$n\ | Apeireth 生命力维 (A13 落点 — R14 Phase 4 维度 1 穿透架构: 持续力 + 反思期 + SGI 单字段 + 涌现 + 内稳态) |
| \$n\ | Apeireth 双洋葱统一体 trait abstraction layer — 原则洋葱 5 层 (E/S/A/M/O) + 权限洋葱 6 层 (L0-L5) + 电子环网络 (R14 Phase 4 P16, ADR-0001) |
| \$n\ | Apeireth 统一凭据存取层 (TP3/N21): 按服务名读写凭据 (CredentialsStore trait + 文件形态后端) + 权限洋葱衔接 (高危凭据走审批门 trait 口) + 脱敏红线 (明文不入日志/错误). TP20-S3 塞缝批: 加 KeyringBackend trait + 平台 keyring 后端 + EncryptedFileBackend fallback (chacha20poly1305) + SecretBuf zeroize + 审计 (name_hash 不含明文) |
| \$n\ | Apeireth 向量检索子系统 (VectorStore trait + SqliteVecBackend) — V2 P1 战区 4 skeleton (docs/v2-strategy/05 §Step 4) |
| \$n\ | Apeireth 行动器官 (A11.1 落点 — R14 Phase 4: 改变环境 + 工具执行 + 表达 + 沉默 = 不行动也是行动) |
| \$n\ | Apeireth 性能基准 (criterion benchmarks, V1130 wallclock 2.5s target) — R14 Phase 1 性能验证 |
| \$n\ | Apeireth 演化器官 — 6 状态机 + trait fail-6 + Learning/Abstraction/SelfModification/Extension + 与 apeireth-council 集成 — R14 round5-01 (backend_engineer2) |
| \$n\ | Apeireth 意识子系统 (A12 — Cognitive-Dream 6 状态机: Awake/Reflecting/Dreaming/Meditating/SelfDisabling/Recovering) |
| \$n\ | Apeireth 约束器官 (P12 — v4.1 新增: 12 键 verdict cache 复用 + 5 重守门 trait (编译时/运行时/多AI/物理隔离/反思期)) |
| \$n\ | Apeireth 智囊团 7 强制 Advisor + 按住机制 + 拟人化 synthesis + mock LLM provider — R14 Phase 5 P22 (架构师2 落点, P15 7 强制顾问) |
| \$n\ | Apeireth 主路径核心类型 (Episode/Note/Session/IdentityCard) — R14 Phase 1 入口 |
| \$n\ | Apeireth 主权 + MEWG 五重治理 (Multi-Evidence Weighted Governance: MEWG/多人/多AI/物理多签/反思期) — R14 Phase 5, 纯 Rust trait + mock, 无 PyO3/外部 SDK |
| \$n\ | Apeireth 专用 rate limiter (R20 阶段 6 估补, token/leaky/fixed/sliding window 4 算法 + 5 storage stub, 0 真接 R20 阶段 6 skeleton) |
| \$n\ | Apeireth 自研 API 接入平台 — 直连 Anthropic + OpenAI 协议双标准 (R17 重构, 不再依赖 NewAPI) |
| \$n\ | Apeireth 自研 HTTP 客户端 — Keep-Alive LIFO 池 (复刻开源 agentOptions 5 字段 (战役 1-2 / 借鉴 §6.2.2 #14) |
| \$n\ | Apeireth ASI 北极星指标 (V0.5 5 维 + V1136 真测 7 子测度) — R14 Phase 2 Rust 重设计 |
| \$n\ | Apeireth CentralAI aggregate root, lifecycle coordinator, and PID 1 supervisor entry |
| \$n\ | Apeireth CLI (CliRunner, 暴露 Rust 子系统给终端) — R14 Phase 0 接口规范对照 |
| \$n\ | Apeireth cognitive organ (cognitive organ ) + R172 consciousness bridge |
| \$n\ | Apeireth companion organ (A12.5) - 长期跨 session 用户关系器官, 承载''用户是 AI 伙伴'' 语义. 基于 apeireth-graph-primitive, 提供 Partner / Bond / Milestone / Timeline / Companion 核心类型. |
| \$n\ | Apeireth cross-crate regression verification mechanism |
| \$n\ | Apeireth experience layer (R173/Stage2 §3): LLM Wiki + Knowledge Graph + VCP association network |
| \$n\ | Apeireth host infrastructure facade: secure keyring and cross-platform machine identity |
| \$n\ | Apeireth image generation tool (ImageGenProvider trait, Mock + OpenAI DALL-E + Stability AI + MiniMax-Image providers, compatible adapter layer) |
| \$n\ | Apeireth Library Stage 5 governance — policy framework + formal verification + cross-crate consistency (R127 P5-2, per decision-33 §1.4 + decision-55 §2.3) |
| \$n\ | Apeireth OpenClaw-mode gateway: single long-lived daemon hosting multiple Node adapters (TUI/HTTP/Desktop/Mobile/CLI), Agent workspace, Skills, and DM access security. Borrows the 'single process, multi-LLM, multi-channel' architecture from OpenClaw, lifted to Rust with compile-time-guaranteed single-instance mode. |
| \$n\ | Apeireth Privacy Guard (VCP 模式 3/8): PII 检测 + 脱敏 + 审计 |
| \$n\ | Apeireth process-level supervisor: PID 1 + 5 sub-supervisors + 3 restart strategies + actor mailbox + AI 自驱心跳 |
| \$n\ | Apeireth PyO3 桥 (Python 3.13.14 <-> Rust) — R14 Phase 3 (暴露 Rust crate 给 Python mvp/) — ADR 0007 compat-components-layer + ADR 0008 feature-gating-pybridge (round9-11 qa_engineer) |
| \$n\ | Apeireth R137: filesystem extension (sandbox + atomic write + fsnotify + file lock + doc parsing) |
| \$n\ | Apeireth R138: shell extension (real sandbox seccomp/JobObject + russh SSH + persistent tasks + streaming + multi-sig + calculator), extending apeireth-tools/code_exec and long_task |
| \$n\ | Apeireth R139: browser tool extension (Playwright accessibility tree + CLI/SKILL + MCP dual mode), HTTP fetch by default, optional CDP via chromiumoxide |
| \$n\ | Apeireth R140: code search + knowledge graph (regex + Aho-Corasick + symbol extraction), 15 MCP tools, borrows codebase-memory-mcp design |
| \$n\ | Apeireth R141: image processing tool (multimodal router, OCR placeholder, image hash, EXIF) |
| \$n\ | Apeireth R144: context folding (FoldStrategy + FoldMarker + cross-session token accumulator), borrows ContextFoldingV2 design (origin: open-source) |
| \$n\ | Apeireth R145 VSearch: 全文 + 聚合 + TF-IDF 排序内存搜索. 区别于 apeireth-tool-codesearch (regex/AST) + apeireth-tool-image-process (perceptual). 上升为 Rust 编译期保证, 字段级复刻 VSearch/VSearch+ (origin: open-source). |
| \$n\ | Apeireth R145: HASH-SQL 仲裁 + 唯一事实时间线. 跨前端/群聊/邮箱/Agent 通讯 1 套 SQL, content_hash 决定 canonical order. 上升为 Rust 编译期保证, 字段级复刻 HASH-SQL 仲裁 (开源项目 origin). |
| \$n\ | Apeireth R147: end-to-end runtime orchestration - HeartbeatScheduler + AsyncTaskStore + ChanneledBus + ArbitrationLog + SearchEngine + GroupChat + EmotionEngine integrated runtime. Borrows 'self-driven living day' concept (origin: open-source), lifted to Rust compile-time guaranteed unified runtime. |
| \$n\ | Apeireth R149 unified fetch engine: HTTP+search+deep+Bilibili+anime. 吸收 7 个 fetch/search plugins (UrlFetch+TavilySearch+AnySearch+VSearch+FlashDeepSearch+BilibiliFetch+AnimeFinder), 上升为 Rust 类型系统 + 多 provider 单 crate. |
| \$n\ | Apeireth R152: Temporal-style workflow engine (Activity trait + WorkflowRunner + EventHistory). Borrows temporalio/temporal (13K stars) design, self-impl 0 引外部 dep. |
| \$n\ | Apeireth R17 战役 1-1: LLM 协议归一化层 (OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini), 字段级借鉴开源 protocol-bridge 真代码 |
| \$n\ | Apeireth R17 战役 1-3 主 chat 管线 (借鉴 §6.2.2 #15/#17/#19/#20: token 预算三层 / placeholder 递归 / Force-Translate / 15s 抑制窗口) |
| \$n\ | Apeireth R17 战役 2-1: 工具注册中心 (6 类 enum + 5 轴正交 + token 预算三层 + notify 热加载 + 异步任务推送, 借鉴 §6.2.1 #12/#13 + §6.2.2 #15 + agentManager.js chokidar (origin: open-source)) |
| \$n\ | Apeireth R17 战役 2-2: 工具运行时 (parser + executor + record + privacy, 借鉴开源 runtime loop + toolCallRecordStore + toolResultPrivacyGuard + §6.2.2 #18 (origin: open-source)) |
| \$n\ | Apeireth R17 战役 2-3: 工具审批 (5 规则 + 5 分钟窗口 + fuzzy matching 集成, 借鉴自 toolApprovalManager.js (origin: open-source)) |
| \$n\ | Apeireth R17 战役 2-4: Agent 管理系统 (alias 解析 + LRU cache + notify 热加载, 字段级复刻 agentManager.js 339 行) |
| \$n\ | Apeireth R17 战役 2-5: 工具集成 (5 trait 真实现: web_search / file_ops / git_ops / code_exec / tool_result, FileOperator 68KB 真代码字段级复刻 (origin: open-source)) |
| \$n\ | Apeireth R20 阶段 1: Team Lead (1:1 翻译 v0.9.21 商业版 out/main/agent/AgentMCPServer.js Orchestrator 缺 P0, A 改 13:34 的版本同步) |
| \$n\ | Apeireth R20 阶段 4 估补 (RIVAL §2.4): 蓝图实装 = 4 风险类 (K-1/K-2/K-3/K-4) + 4 决策表 (D-01..D-04) + 6 实战模板 (A-F) + 5 R-Measure (R-1..R-5) + 3 评估指标 (Q1/Q2/Q3). 1 crate 打包, 跟 V0.5 命名 (apeireth-naming-v05) 互不冲突. |
| \$n\ | Apeireth R20 阶段 4 估补: V0.5 命名规范 (4 类 × 6 维 = 24 维) + R126 P1-4 V0.5.30 扩展 (5 new meta-dim + 1 derived overall = 30 维). 1:1 翻译 v1077 V0.5 17 维 LOCKED 升级到 24 维 v2 命名空间 (per docs/architecture-v4-1-living-intelligence-update §13). 4 大类 (PC 0.40 / RC 0.30 / HG 0.15 / GP 0.15) × 6 维度 (level/domain/modality/safety/completeness/lineage) = 24 base 维 + 5 new meta-dim (Robustness+SelfImprovement+Adversarial+CiPassRate+VerifierConsistency) + 1 derived overall (MetaOverall) = 30 维, sum=1.00 守门, 编译期 hardcode enum, encode/decode/validate/sum_guard 完整. 0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor + 8 项不修改承诺. 借鉴 ID: R126-v05-30-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10 |
| \$n\ | Apeireth R20 阶段 5 集成测试 e2e (主仓 + API + TUI 三层端到端, 60+ 测试, 不碰 24 LOCKED) |
| \$n\ | Apeireth R20 阶段 6 flesh out: Lark/Feishu SDK (1:1 翻译 v0.9.21 商业版 out/main 中 `@larksuiteoapi/node-sdk@^1.59.0` 集成面; STUB 模式 (LarkClientImpl 8 工具返 NotImplemented) + 真接实现 (LarkRealImpl 5 端点 HTTP, reqwest 0.12 + rustls-tls, wiremock 0.6 测); STUB_MODE 编译期 hardcode=true 守门, 切 false 需 6 哲学锚 + 主人审) |
| \$n\ | Apeireth R20 阶段 6 flesh out: LiveKit Server SDK 真接实现 (1:1 翻译 livekit-server-sdk 0.6+ Twirp API: server_url / api_key / room / track / participant / event 6 端点, 走 reqwest 0.12 + rustls-tls HTTP, wiremock 0.6 测; STUB 守门 6 核心 API + 5 K-1 强校验 + 8 tool whitelist 编译期 hardcode) |
| \$n\ | Apeireth R21 借鉴 Golutra #6: 9 Tauri state 模式 (OnceLock + Arc + Mutex) 转 TUI 等价物 (ratatui state 共享框架). 3 模式 (OnceLockState / MutexState / RwLockState) + 9 器官 state 共享 (heart/brain/hand/eye/ear/memory/voice/body/mind) + 1 完整状态共享例子 + 25+ 集成测试. 0 真接 tokio/async, 留 R21 续真接. 0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor + 8 项不修改承诺 |
| \$n\ | Apeireth R35 telemetry umbrella (cache + metric + trace + observability facade) |
| \$n\ | Apeireth relation subsystem (A12) - 4 relation kinds (Symbiosis/Coordination/Embedding/SelfRelation) + relation decision tree + property graph (R154: RelationGraph with adjacency indexes + BFS/DFS iterators + shortest path + predicate query). 0 引 external dep. |
| \$n\ | Apeireth repository scanning and quality analysis facade |
| \$n\ | Apeireth terminal sandboxes (R173 / Stage2 §3): 6 backend - Local/Docker/SSH/Daytona/Modal/Singularity |
| \$n\ | Apeireth TUI (R19 + R155) - ratatui 终端版, 5 nav 页面 (舰桥/对话/生长/历史/设置) + 9 器官 + 30+ 后端 crate 全接 + R155 RuntimeBridge 拉 apeireth-runtime 7 模块 (HeartbeatScheduler/AsyncTaskStore/ChanneledBus/ArbitrationLog/SearchEngine/GroupChat/EmotionEngine) 状态供 TUI main loop 渲染 |
| \$n\ | Apeireth TUI 5 nav + 9 器官 端到端集成测试 (R20 阶段 5 估补, ratatui TestBackend 测 TUI 设计契约, 干 TUI 不干前端) |
| \$n\ | Apeireth v2.0 战区 5 P0: Model Context Protocol skeleton (client/server + JSON-RPC 2.0 + stdio/SSE transport + tool-registry bridge, 字段级参考 MCP 2025-03-26 规范) |
| \$n\ | Apeireth v2.0 P0 deterministic graph orchestration and checkpoints |
| \$n\ | Apeireth voice subsystem |
| \$n\ | Apeireth Web 前端 — Leptos 0.7 SSR + WASM hydration, 让主人能在浏览器真用 Apeireth Council 7 advisor (R18) |
| \$n\ | apeireth-acp — R23 6 module acp 子模块: Agent Communication Protocol 抽象 + 信封 + 路由 |
| \$n\ | apeireth-bus — 5 层通信总线 (L0 inproc / L1 UDS / L2 pipe / L3 gRPC / L4 WebSocket) + pub-sub/req-rep/streaming + 反背压 + Trace ID 链路追踪 (round15-02) |
| \$n\ | apeireth-config — R23 6 module config 子模块: 强类型配置项 |
| \$n\ | apeireth-cron — R23 6 module cron 子模块: 计划任务声明 + 时间窗校验 |
| \$n\ | apeireth-eval — R23 6 module eval 子模块: 评测维度 + 分数聚合 |
| \$n\ | apeireth-extension — 6 类扩展 (sync/async/static/service/messagePreprocessor/hybrid) + extension.toml 严格 schema + 审核后注册 + 沙盒 + 调用审计 (P28 round 5-03) |
| \$n\ | apeireth-provider \u2014 R35: 6 Provider + R176: LlmFacade + http_dispatch |
| \$n\ | apeireth-skills — R23 6 module (cron/skills/acp/config/test/eval) skills 子模块: 可复用能力声明 + 描述 + 输入/输出 schema + 版本 |
| \$n\ | apeireth-test — R23 6 module test 子模块: 测试用例描述 + 重试 + 报告 |
| \$n\ | R179 P0-3: LLM 抽象接口 (ChatMessage / LlmRequest / LlmProvider / LlmError). 拆 apeireth-memory <-> apeireth-api 编译期边. |
| \$n\ | R20 阶段 6 估补: 通用 5 阶段 pipeline 框架 (placeholder, 整合 #3 B-7 R21 续补范畴, 真实实现待 R21+ 重建) |
| \$n\ | release-tools — TP20-S5 塞缝批: 发布期供应链验证 (cargo vet/audit/deny + CycloneDX SBOM) 的工程化载体 |
| \$n\ | TP27 标的元数据资产 (N3 金融源, FinanceDatabase 30 万标的入库套件 — 标的清单/行业/交易所/可信度 T0) |
| \$n\ | TP28 Markdown 知识库 (llm_wiki 模式, 文件树 + 索引 + 检索) |
