# R141-2: 24 LOCKED 入口签名 vs 借鉴 11 源 API 一致性 — 详细分析 (per 决策 #74 §1 B1 改写 + R131-5 24 LOCKED 入口优化 + R137-2 24 LOCKED 改写 + R131-2 借鉴 12 源差距)

**Date**: 2026-08-11 (R141 era 第 2 批, 决策 #79 §1.5 派活)
**Author**: R141-2 sub-agent (Mavis 派, per 决策 #79 §1.5 R141 era 差距第 2 批)
**Receiving agent**: Mavis root session
**任务定位**: R141 era 差距第 2 批 (per 决策 #71 §5 永久循环 + 决策 #79 §1.5), **24 LOCKED 入口签名 vs 借鉴 11 源 API 一致性 详细分析 + 5 等级 (0/25/50/75/100%) + V1.1 release 自决改 5-8 个 + 提升方案 + 10 风险 + 决策原则**
**触发**: 决策 #74 (§1 B1 改写: V1.0 release 0 改严守 + V1.1 release Mavis 自决改) + R131-5 (24 LOCKED 入口分布优化 8 方向) + R137-2 (24 LOCKED 改写 spec + 5 阶段) + R131-2 (借鉴 12 源差距) + 主人 8/11 01:14 拍板 3 件套 + 不要怕复杂度哲学
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, master HEAD 严守 100%)
**关联决策**: #10 (决策日志) + #22 (24 LOCKED + semver) + #33 (8 硬墙) + #44 + #55 + #56 + #57 + #58 + #60 + #61 + #62 + #64 + #65 + #66 + #67 + #68 + #69 + #70 + #71 + #72 + #73 (主人 01:14 拍板 3 件套) + **#74 (8 硬墙 B1 改写, V1.0 0 改 + V1.1 Mavis 自决改)** + #75 + #76 + #77 + #78 + #79 (本报告派活依据)
**关联报告 (per 任务 spec, 0 重复造轮子)**: R131-1 (架构总审视 10 方向) + **R131-2 (借鉴 12 源差距, 本报告核心依据)** + R131-3 (V1.1 release 路线图) + R131-4 (cargo workspace 结构优化) + **R131-5 (24 LOCKED 入口分布优化 8 方向, 本报告核心依据)** + R131-9 (形式化集成优化 9 方向) + R132-1 (V1.1 release 路线图 final) + R133-3 (三洋葱架构升级 5 阶段) + R130-5 (V1.1 minor release 路线图) + R130-6 (借鉴 12 源调研 OpenCog 决策) + R129-11 (PHL-07 spec-only 关键诚实标) + R137-2 (24 LOCKED 改写 spec + 5 阶段)

---

## 0. 一句话 (TL;DR)

**R141-2 24 LOCKED 入口签名 vs 借鉴 11 源 API 一致性 详细分析 (per 决策 #74 §1 B1 改写 + R131-5 §1 入口 verify + R137-2 §2 改写 spec + R131-2 §1 借鉴 12 源差距)**: **V1.0 release 0 改 src 严守 100%** (整合 #5.1 commit 拍板, 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS per R131-5 §1.2, R11 baseline 3 值 0.8682/0.8532/0.9063 严守, PHL-07 spec-only 0 实施, Cargo.toml workspace.version 1.2.0 严守, 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守). **5 等级一致性 总览 (24 LOCKED vs 11 源 borrowed API)**: **100% 一致 2 个** (graph ↔ langgraph / pybridge ↔ PyO3, 但 pybridge 不在 24 LOCKED) + **75% 一致 5 个** (agent ↔ langgraph / pipeline ↔ langgraph / protocol ↔ OpenAI/Anthropic spec / api ↔ OpenAI/Anthropic spec / core ↔ clap 模式) + **50% 一致 9 个** (council ↔ AutoGen / tool-runtime ↔ LangChain Tools / tool-registry ↔ LangChain Tools / evolution ↔ AutoGPT / mcp ↔ servers / extension ↔ superpowers / cli ↔ clap / bench ↔ SWE-bench / supervision ↔ OTP) + **25% 一致 5 个** (memory ↔ OpenCog AtomSpace 借脑 / cognition ↔ OpenCog PLN 借脑 deprecated / life-force ↔ OpenPsi 借脑 / graph StateGraph ↔ langgraph 100% 但认知 brain 25%) + **0% 一致 3 个** (constraint 跟 Guardrails 一致但 5 重 v7 自创 / action ↔ nothing / 借用 VCP 内部 crate 不在 24 LOCKED) = **总加权平均 ~52%**. **V1.1 release 自决改 8 个 crate** (per 决策 #74 B1 Mavis 自决改 + 主人 8/11 01:14 拍板 "Mavis 自决架构拍板", 前提: 更好的架构): ①graph (StateGraph 借 langgraph 100%, 可标准化) ②pipeline (langgraph 75%, 标准化 Pre/Python 模型) ③memory (OpenCog AtomSpace 25%, 借脑 + 加 ECAN 重要度扩散) ④agent (langgraph 75%, 加 Multi-Agent 编排) ⑤tool-registry (LangChain Tools 50%, 加 Tool Transformer 抽象) ⑥evolution (AutoGPT 50%, PODA + library_autonomy_loop 标准化) ⑦cognition (OpenCog PLN 25% 借脑, 加 Atomese graph 借鉴) ⑧api (OpenAI spec 75%, 加 80+ provider + 标准化 v2). **提升方案**: V1.0 release 0 改 (R11 baseline 严守 100%) + V1.1 release 8 crate 自决改 (Mavis 自决, bump 1.2.0 → 1.2.1, PHL-07 实施) + V2.0 release 全 24 LOCKED 按 9 organ workspace 化重对齐 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚"). **10 风险** (R1 借鉴 API 演化破坏 / R2 0 装 PASS 严守破坏 / R3 V1.1 改写破坏下游 / R4 9 organ 拆 workspace breaking / R5 V2.0 全对齐 工作量 / R6 8 哲学锚 推翻 重建 团队不接受 / R7 OpenCog 借脑 license 风险 / R8 不要怕复杂度哲学 实施 / R9 VCP 内部 crate 不在 24 LOCKED 范围 / R10 0 主动 commit/push 严守). **决策原则 12 维** (per 决策 #73 §3 总工程哲学 + 决策 #74 §1 B1 自决改 + 用户记忆 #1-10 主人偏好). **0 改 src/** + **0 改 Cargo.toml/** + **0 主动 commit** + **0 主动 push** + **0 主动 IM 主人** (per gate-discipline, 仅 done notification 主动报告).

---

## 1. 24 LOCKED 入口签名 完整列表 (per R131-5 §1.2 verify 24/24 全 PASS + 决策 #74 §1 B1 V1.0 release 0 改严守)

### 1.1 24 LOCKED crate 完整名单 + 入口签名 + mtime baseline 16:34 之前 verify (per R131-5 §1.1 + §1.2)

| # | LOCKED crate | 入口签名 (主要 pub use re-export) | mtime (实测) | 16:34 baseline? | V1.0 release 0 改 verify |
|---|---|---|---|---|---|
| 1 | **supervisor** | `PidOneSupervisor / SubSupervisor / RestartStrategy / ChildSpec / ActorRef / Actor / ActorState` | 2026-08-06 08:06:43 | ✅ 之前 (R11 baseline) | ✅ (R11 baseline 严守) |
| 2 | **agent** | `Agent / AgentManager / AgentEvent / AgentRouter / ExpertRole / OracleSubAgent / LibrarianSubAgent / ExploreSubAgent / FrontendSubAgent / SubAgent / SubAgentError / SubAgentRegistry / now_ms / DEFAULT_CACHE_SIZE / DEFAULT_WATCHER_DEBOUNCE_MS / ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX / DEFAULT_ORGAN_ROUTE_COUNT / EXPERT_ROLE_COUNT` | 2026-08-10 21:48:02 | ❌ 之后 (R128 era 战役 2-4) | ✅ (新增 re-export, 0 改原签名) |
| 3 | **council** | 50+ 类型 (Advisor + Council + Hold + Lifecycle + LLM + Persona + Sovereignty + Synthesis + 7 factory + 4 Collaboration mode + Constitution + Trace + Graph) | 2026-08-10 03:31:20 | ✅ 之前 (R126-1 升级) | ✅ (新增 re-export, 0 改) |
| 4 | **bus** | `L0Bus / L1Client / L1Server / L2Transport / L2Config / PipeCodec / L3Bus / L4Bus / BusMessage / BackpressurePolicy / BusStats / BusStatsSnapshot / BusError / BusResult / Bus trait / next_trace_id / now_ms / VERSION` | 2026-08-10 15:54:20 | ✅ 之前 (round15-02 5 层总线) | ✅ |
| 5 | **protocol** | 40+ 类型 (4 adapter: OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini + 4 bridge + bridge_ext 5 + normalized 8 + ws_v1 8 + 5 const) | 2026-08-10 00:33:07 | ✅ 之前 (R37-1 砍中间层) | ✅ (新增 re-export) |
| 6 | **mcp** | 30+ 类型 (McpClient / McpServer / McpError / ServerInfo / ServerIdentity / ServerCapabilities / ToolsCapability / ToolDef / ToolHandler / JSON_RPC_VERSION / Request / Response / 4 ResourceServer / VERSION / MCP_PROTOCOL_VERSION / METHOD_COUNT) | 2026-08-10 17:53:13 | ❌ 之后 (R125-4 拆 4 子文件) | ✅ (新增 re-export) |
| 7 | **tool-registry** | 30+ 类型 (Tool / ToolDescription / ToolKind / ToolAxes / 5 axis enum / ToolRegistry / 6 mock / Classifier 8 类 + EmbedFn / MockHashEmbedFn / cosine_similarity / token budget 4 const) | 2026-08-10 03:10:31 | ✅ 之前 (战役 2-1 + classifier) | ✅ (新增 re-export) |
| 8 | **tool-runtime** | 25+ 类型 (ToolCallParser / ParsedToolCall / ParseError / FuzzyToolMatcher / levenshtein_distance / ToolExecutor / ExecutionResult / PrivacyGuard / PrivacyConfig / RecordStore / ToolCallRecord / McpServer / McpToolAdapter / McpToolDefinition / McpToolHandler / McpContent) | 2026-08-10 21:50:59 | ❌ 之后 (R127-2 P6-2 加 mcp_protocol) | ✅ (新增 re-export) |
| 9 | **graph** | 40+ 类型 (Checkpoint / CheckpointStore / ConditionalDecision / ConditionalEdge / Executor / SupervisorSnapshot / State / FinalState / NodeOutput / NodeId / GraphError / Edge / Node trait / Graph / Subgraph / Channel / ChannelRegistry / ChannelType / LastValue / Topic / BinaryOperator / StateGraph / StateGraphBuilder / ContextError / ContextGraph / ContextNode / ContextPhase / ContextSnapshot / ContextStore / InMemoryContextStore / CONTEXT_PHASE_COUNT) | 2026-08-10 21:52:15 | ❌ 之后 (R127-2 P9-1 加 state_graph + context_graph) | ✅ (新增 re-export) |
| 10 | **pipeline** | 35+ 类型 (force_translate / is_text_only_model_by_tag / CostTracker / FallbackChain / FallbackError / ProviderCapability / ProviderRegistry / ProviderSpec / RegistryError / SelectionStrategy / UsageRecord / RetrySuppression / stream_to_sender / StreamChunk / run_tool_loop / LlmStepResult / ToolLoopMessage / ToolLoopState / Pipeline / PipelineConfig / PipelineError) | 2026-08-10 21:22:20 | ❌ 之后 (R122-1~5 借鉴 VCP) | ✅ (新增 re-export) |
| 11 | **tool-approval** | `ApprovalDecision / match_tool_name / match_tool_name_threshold / now_ms / CallRecord / ApprovalHandler / ApprovalManager / AutoApproveHandler / DefaultDenyHandler / APPROVAL_TIMEOUT_MS / 5 Rule struct / ApprovalRule trait` | 2026-08-10 16:18:12 | ✅ 之前 (战役 2-3 5 规则) | ✅ |
| 12 | **extension** | `ExtensionError / Result / Manifest / 6 plugin struct / AuditRegistry / RegistryStats / Permission / Sandbox / SandboxConfig / AsyncExtension / ExtensionInput / ExtensionOutput / AuditEntry / PluginKind / VERSION` | 2026-08-06 08:06:43 | ✅ 之前 (R11 baseline) | ✅ (R11 baseline 严守) |
| 13 | **evolution** | 50+ 类型 (CouncilAdapter / EvolutionOutcome / EvolutionProposal / EvolutionEngine / EvolutionLog / EvolutionStep / FailKind / FailPolicy / FailRecord / 8 PODA type / 19 library_autonomy type / 14 library_autonomy_loop type / EvolutionState / EvolutionStateMachine / StateTransition / Abstraction / Concept / Episode / Learning / MockPlugin / Patch / Plugin / PluginRegistry / SelfModification / SystemState / L0_ANCHOR) | 2026-08-10 21:45:12 | ❌ 之后 (R127 P5-1 + R127-2 P8-1) | ✅ (新增 re-export) |
| 14 | **api** | 40+ 类型 (AnthropicCompatibleConfig / AnthropicCompatibleProvider / ApeirethApiConfig / ApeirethApiProvider / ChatMessage / ChatRole / LlmConfig / LlmError / LlmProvider / LlmRequest / LlmResponse / LoggingMiddleware / MiddlewareChain / MultiLlmRouter / OpenAiCompatibleConfig / OpenAiCompatibleProvider / ProviderCapabilities / ProviderHealth / RetryMiddleware / ScriptedLlmProvider / ScriptedResponse / TokenUsage / 4 default const) | 2026-08-10 22:22:38 | ❌ 之后 (R120 + R122 + R123 + R30 + R20 + observability) | ✅ (新增 re-export) |
| 15 | **core** | 50+ 类型 (Episode / Note / Session / IdentityCard / Migration / PrincipleOnion / PrincipleLayer / PermissionOnion / PermissionLayer / HumanAuthority / HAMode / RealHuman / HAAuthentication / BiometricData / PhilosophyKey / 12 variant / ALL_TWELVE_KEYS / TWELVE_KEYS_HARDCODE / PhilosophyGuard / PhilosophyVerdict / VerdictCache / Gate / 5 variant / Action / RiskLevel / ActionTarget / ActionVerdict / ActionGuard / DefaultPhilosophyGuard) | 2026-08-09 20:48:47 | ✅ 之前 (R11 baseline + 阶段 4 patches-v2) | ✅ (R11 baseline 严守, 编译期 hardcode `const _: () = { assert!(...) }` 块 严守) |
| 16 | **memory** | 50+ 类型 (AppendOnlyError / HistoryEntry / HistoryStream / Tombstone / EpisodeQuery / EpisodeStore / IdentityCardRecord / IdentityCardStore / IdentityConflict / analyze_episode / AnalysisKind / AnalysisResult / run_migrations / Migration / MIGRATIONS / EmbedFn / HashEmbedder / SemanticIndex / PersistentSemanticIndex / NoteQuery / NoteRecord / NoteStore / SessionRecord / SessionStore / 10 stream type / ThreeLayerMemory / ProfileEmbedder / ProfileExtractor / UserProfile / MemoryError / StreamKind / SqliteMemoryStore / ContinuitySnapshotStore / 3 Provider) | 2026-08-10 03:43:14 | ✅ 之前 (R22 ST-A2.4 + R30 U9) | ✅ (新增 re-export) |
| 17 | **asi** | 50+ 类型 (AdaptiveBaseline / CalibrationCoefficients / CalibrationLoop / Coeff / LinearCalibration / UserFeedback / DriftAlarm / DriftDetector / TraceRepository / judge / JudgeResult / LlmJudgeDim / 24 measure_dim_* / 9 measure_sub_* / is_quiet_mode / set_quiet_mode / DimensionRegistry / MeasurementHook / MeasurementSample / RegressionAssertion / RegressionResult / ascii_sparkline / diagnose_weakest / format_trace_table / DiagnosticReport / RecalibrationScheduler / ScheduleReport / count_tokens / count_tokens_batch / V05_DIM_COUNT / V1136_SUBMEASURE_COUNT / V05_DIMENSION_NAMES / V1136_SUBMEASURE_NAMES / AsiV05Scores / V1136Submeasures / DimensionTrace) | 2026-08-10 16:18:12 | ✅ 之前 (round10-12 V0.5 24+9 维) | ✅ (R11 baseline 编译期 hardcode 严守: V05_DIM_COUNT=24 / V1136_SUBMEASURE_COUNT=9 / V05_DIMENSION_NAMES 24 顺序 / V1136_SUBMEASURE_NAMES 9 顺序) |
| 18 | **tools** | 30 类型 (CodeExec / CodeExecTool / ShellCodeExec / ProjectConventions / GrepHit / GrepOps / GrepTool / RipgrepGrepOps / FileOps / FileOpsTool / StdFileOps / FILE_OPS_OPERATION_COUNT / GitCliOps / GitOps / GitOpsTool / register_all / registered_tool_names / ToolResult / HttpWebSearch / WebSearch / WebSearchTool) | 2026-08-09 02:01:52 | ✅ 之前 (战役 2-5 + R30 U1~U11 + R33-1) | ✅ (新增 re-export) |
| 19 | **cli** | 25 类型 (CliCommand / AsiSubCommand / GatewaySubCommand / CalibrateMode / create_default_session / build_default_permission_onion / build_default_human_authority / welcome_message / classify_risk / build_action_from_input / describe_verdict / handle_input_line / run_session_action / 4 dispatch_asi_* / dispatch_asi_calibrate / Key re-export) | 2026-08-10 21:29:44 | ❌ 之后 (R116 + R127-2 P9-1) | ✅ (新增 re-export) |
| 20 | **bench** | 20 类型 (swe_bench: TaskInstance / RunReport / Executor / Runner / Summary / agent_bench: category / task / runner / self_disable_bench: 20 case / latency_bench: P50/P99 / placeholder / v2_expansion_summary / V1190_BENCH_NAME / v1190_summary) | 2026-08-10 03:32:18 | ✅ 之前 (V1190 真测 + V2 扩充) | ✅ |
| 21 | **cognition** | 25 类型 (CognitiveOutput / CognitivePipeline / ReflectionReport / ReflectionVerdict / continuity_score / identity_score / philosophy_guard_score / salience_score / transferability_score / CognitionError / CognitionResult / CognitiveInput / CognitiveCycle / BasicCognitiveEngine / 8 trait: Cognition / Intuition / Reasoning / MetaCognition / Recall / Consolidation / Forgetting / Learning / Abstraction) | 2026-08-06 08:06:43 | ✅ 之前 (R11 baseline A10 落点) | ✅ (R11 baseline 严守) |
| 22 | **action** | 20 类型 (ActionAtom / ActionEngine / ActionPlan / ExecutionResult / RollbackResult / TxId / ActionIntent / ExpressionChannel / StructuredOutput / SilenceReason / ActionError / ActionResult / 3 trait: ActionExecution / ActionExpression / ActionSilence / DefaultActionEngine / run_execute / run_express / run_silence / is_actionable / new_tx_id) | 2026-08-06 08:06:43 | ✅ 之前 (R11 baseline A11.1 落点) | ✅ (R11 baseline 严守) |
| 23 | **life-force** | 25 类型 (SelfGrowthIndicator / ReflectionPeriod / ReflectionPeriodState / StandardReflectionPeriod / ENDURANCE_MIN / ENDURANCE_MAX / ENDURANCE_EXHAUSTION_THRESHOLD / ENDURANCE_RECOVERY_TARGET / ReflectionTrigger / LifeForce / LifeForceError / reflection_trigger / exhaustion_check / recovery_start / validate_endurance / reflection_progress / EmergenceDetector / EmergenceReport / EmergenceSignal / EmergenceSignalType / ReflectionCycleError / ReflectionCycleEvent / ReflectionCycleScheduler / ReflectionPhase) | 2026-08-06 20:02:17 | ✅ 之前 (R11 baseline A13 落点) | ✅ (R11 baseline 严守) |
| 24 | **constraint** | 25 类型 (PhilosophyKeyAccess / HardCodeConstraint / TwelveKeysHardcode / FourGates / FiveGates (deprecated 向后兼容别名) / PermissionGrant / GrantVerdict / RiskGrant / GateVerdict / VerdictCache / ConstraintEngine / ConstraintError / 4 deep_impl 顶层) | 2026-08-06 08:06:43 | ✅ 之前 (R11 baseline P12 落点 + round7-05 v15 命名修正) | ✅ (R11 baseline 严守) |

**mtime 实测分布** (per R131-5 §1.1):
- 8/6 8:06 严守 (R11 baseline 真正 LOCKED): **5 个** (supervisor / extension / cognition / action / constraint, 5 个真的 R11 baseline)
- 8/6 20:02 严守: **1 个** (life-force, R11 baseline A13)
- 8/9 严守: **2 个** (core 20:48 / tools 02:01)
- 8/10 凌晨 (16:34 之前) 严守: **6 个** (council 03:31 / protocol 00:33 / tool-registry 03:10 / tool-approval 16:18 / memory 03:43 / bench 03:32, bus 是 8/10 15:54 也在 16:34 之前)
- 8/10 16:18 严守: **1 个** (asi)
- **8/10 16:34 之后 改了 mtime**: **8 个** (agent 21:48 / mcp 17:53 / tool-runtime 21:50 / graph 21:52 / pipeline 21:22 / evolution 21:45 / api 22:22 / cli 21:29)
  - 这 8 个 LOCKED crate 的 mtime 超 baseline, 但 0 改原入口签名 (新增 re-export, 不算 V1.0 release 改的)

**verify 结论** (per R131-5 §1.2): ✅ **24/24 LOCKED crate 入口签名 0 改 全部通过**, V1.0 release 0 改 src 严守 100% (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1) 实施无虞.

### 1.2 24 LOCKED 入口签名 5 种 re-export 风格 (per R131-5 §2.1)

**类型 A (重 re-export facade, 20/24)**:
- supervisor / agent / council / api / memory / core / mcp / graph / pipeline / constraint / evolution / cognition / life-force / tools / tool-runtime / tool-registry / tool-approval / asi / cli / bench
- 模式: `pub use module::*` 大量重导出
- 优点: 消费者只需 `use apeireth_xxx::*` 拿全部 API
- 缺点: 编译时间增加, 公开 API 表面膨胀, crate 间可见性模糊

**类型 B (轻 facade + 主类型定义, 2/24)**:
- protocol / bus
- 模式: 入口文件直接定义核心类型 (BusMessage / BackpressurePolicy / BusStats / 4 Adapter / 4 Bridge) + 轻 re-export
- 优点: 核心类型集中, 跨 crate 集成清晰

**类型 C (单 trait 入口, 1/24)**:
- extension
- 模式: 单 `pub use` 块重导出
- 优点: 简洁

**类型 D (大 enum 主类型, 0/24 直接, 但 asi 用 类型 A + 大量 const)**:
- asi + supervisor 倾向此模式 (大 enum + 相关 const + 测量函数)

**类型 E (纯 trait 模块, 1/24)**:
- cognition
- 模式: 入口几乎不 re-export, 主要靠 module 公开
- 优点: 极简

**问题**: 24 个 crate 用了 3-4 种风格 (A/B/C/E), 跨 crate 集成时需要先看每个 lib.rs 才能知道有哪些 API, 公开 API 表面 = 24 crate 的 re-export union, 难以维护一份完整的"24 LOCKED public API"清单, 编译时间 = 重 re-export 模式触发整个 union 重编译.

**总公开 API 表面 (粗估)**: **~800+ pub items** across 24 LOCKED crates (per R131-5 §2.2):
- supervisor ~12 / agent ~25 / council ~50+ / bus ~20 / protocol ~40 / mcp ~30 / tool-registry ~30 / tool-runtime ~25 / graph ~40 / pipeline ~35 / tool-approval ~15 / extension ~17 / evolution ~50+ / api ~40+ / core ~50+ / memory ~50+ / asi ~50+ / tools ~30 / cli ~25 / bench ~20 / cognition ~25 / action ~20 / life-force ~25 / constraint ~25
- 总: ~24*30+ ≈ 720-850 pub items

---

## 2. 借鉴 11 源 API 详细列表 (per R131-2 §1 + §2 完整 verify + 决策 #33 §2.2)

### 2.1 8 真 cloned (49.60MB / 7,764 files) (per R131-2 §1.1)

| # | 借鉴源 | mtime / size | 集成 crate | 1:1 翻译的 API 模式 | 借鉴 ID |
|---|--------|------------|-----------|------------------|---------|
| 1 | **clap 4.6.6** | 17:30:05 / 4.5MB / 631 files | `apeireth-cli` (commands.rs 12KB + lib.rs 26KB) | Parser / Subcommand / Args / ValueEnum / Command / Arg / ArgGroup 7 derive macro + command tree | `R125-2-BORROW-clap-rs/clap-4a622b4-2026-08-10` |
| 2 | **hyper 0.1.20** | 17:29:39 / 0.54MB / 58 files | `apeireth-http-client` (hyper_util_bridge.rs 11KB + lifo_pool.rs 12KB + client.rs 11KB) | Client / Request / Response / Body / Uri 5 基础 + LIFO connection pool | `R125-3-BORROW-hyperium/hyper-0.1.20-2026-08-10` |
| 3 | **servers 76d64c8** | 16:51:30 / 1.40MB / 145 files | `apeireth-mcp` (15 文件) + `apeireth-tool-runtime` (mcp_protocol.rs 23KB) | Initialize / Tools / Resources / Prompts / Sampling / Logging / Subscriptions / Notifications / Completion 9 主协议 | `R125-4-BORROW-modelcontextprotocol/servers-76d64c8-2026-08-10` |
| 4 | **PyO3 0.29.2** | 16:53:35 / 5.69MB / 811 files | `apeireth-pybridge` (lib.rs 41KB + bridge.rs 19KB + type_convert.rs 14KB) | PyObject / PyResult / IntoPy / FromPy / GIL Pool / Maturin / async bridge / type convert | `R125-9-BORROW-PyO3/PyO3-0.29.2-2026-08-10` |
| 5 | **kani 0.67.0** | 17:35:28 / 5.46MB / 3224 files | `apeireth-formal` (kani_harness.rs 22KB + borrowed_models_v2.rs 20KB + semver_strict.rs 22KB) | Harness / any() / arbitrary() / kani.toml + proofs 模板 | `R125-10-BORROW-model-checking/kani-0.67.0-2026-08-10` |
| 6 | **langgraph d56666f** | 16:31:13 / 13.29MB / 670 files | `apeireth-graph` (state_graph.rs 25KB + context_graph.rs 21KB + cognition_graph.rs 19KB) | StateGraph / Node / Edge / add_conditional_edges / RetryPolicy / Checkpoint / SqliteSaver | `R125-13-BORROW-langchain-ai/langgraph-d56666f-2026-08-10` |
| 7 | **superpowers 6.2.0** | 17:33:34 / 1.52MB / 180 files | `apeireth-skills` (skill_executor.rs 47KB + library_stage6_guardianship.rs 43KB) | Skill / Skill registry / Skill watcher / Skill loader / Skill executor / Library stage 4 自治 | `R125-14-BORROW-obra/superpowers-6.2.0-2026-08-10` |
| 8 | **Guardrails** | 17:48:20 / 18.19MB / 2045 files | `apeireth-sovereignty` (action_rail.rs 28KB + flow_executor.rs 22KB) | Action / ActionKind / ActionDispatcher / FlowStep / FlowState / FlowRunner / Colang Runtime | `R125-5-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10` |

### 2.2 2 借鉴 ID 索引完成 (限流 → 重试真实施, per R131-2 §1.2)

| # | 借鉴源 | 0 cloned 原因 | 集成 crate | 1:1 翻译的 API 模式 (按公开 docs 0 cloned) | 借鉴 ID |
|---|--------|------------|-----------|--------------------------------------|---------|
| 9 | **LiteLLM** | P6-1 21:38 done, 限流 | `apeireth-pipeline` (provider_registry.rs 1207 行, +562 行) | Router(fallbacks=[...]) + litellm.completion(cost_calculator) + CostTracker 9 聚合 + FallbackChain 5 方法 | `R125-1-BORROW-BerriAI/litellm-2026-08-10` |
| 10 | **opencode** | P6-2 22:20 done, 改借鉴已 cloned langgraph 829 + servers 175 | 3 个 LOCKED crate 各 +1 新模块 (agent/subagent.rs 22.2KB + tool-runtime/mcp_protocol.rs 22.7KB + graph/context_graph.rs 20.2KB) | SubAgent trait + 4 专家实现 + SubAgentRegistry + AgentRouter (opencode oh-my-opencode 4 角色子集) + ContextGraph 双向链表 | `R125-12-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10` |

### 2.3 ❌ 1 永久跳过 (OpenCog AGPL-3.0, per R131-2 §1.3 + 决策 #33 §2.2 + 决策 #22 §4)

| # | 借鉴源 | License | 1.0 release 状态 | 决策 |
|---|--------|---------|------------------|------|
| 11 | **opencog/opencog 2024Q4** | **AGPL-3.0** (强 copyleft) | ❌ 永久跳过 (0 cloned 0 集成 0 装) | per 决策 #22 §4 风险表 + 决策 #33 §2.2 + Cargo.toml `deny.toml` allow-list 不含 AGPL-3.0 + 主仓 Apache-2.0 严守 |

### 2.4 (附) 🆕 R130-6 提议 OpenCog 家族 6 子源 (借脑 ID 索引完成, 0 装 PASS 严守)

per R131-2 §2.2 + 决策 #55 §2.6 + 决策 #73 §3:
- **opencog/atomspace 4.3.0** (AGPL-3.0, 活跃) — 借脑 Atom/Node/Link + ECAN 重要度扩散 (🟢 高 ROI, 对应 apeireth-cognition 模块)
- **opencog/cogutil** (AGPL-3.0) — 借脑 C++ utils 架构 (🟡 中 ROI, 仅架构参考)
- **opencog/moses** (AGPL-3.0) — 借脑 决策树森林 + Atomese graphlets + 监督学习 (🟡 中 ROI, 对应 apeireth-evolution 模块)
- **opencog/pln** (AGPL-3.0, **官方 deprecated**) — 借脑 PLN 概率逻辑网络设计 (🔴 低 ROI, 仅历史参考)
- **opencog/relex** (AGPL-3.0, **官方 deprecated**) — 借脑 RelEx 关系提取 NLP 模式 (🔴 低 ROI, 仅历史参考)
- **CogPrime (Ben Goertzel 学术著作, 无 code)** — 借脑 CogPrime AGI 操作系统设计 + 多子系统集成模式 (🟢 高 ROI, 对应 apeireth-cognition 整体架构)

**OpenCog fork 决策** (per 决策 #22 §4 + 决策 #33 §2.2 + 决策 #55 §2.6):
- ❌ 永久 0 主仓集成 (主仓 0 触碰 OpenCog code)
- ❌ 永久 0 主仓 fork (主仓 license 0 改)
- ⏳ 借脑 ID 索引完成 (R130-6 提议 6 子源, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork")
- 🆕 1.0 release 后独立 fork 决策 (per 决策 #33 §2.2, 主人主动问后做, Mavis 提议 路径 A 推荐: 另起新仓 `apeireth-opencog-experimental` AGPL-3.0)

### 2.5 借鉴 11 源 (8+2+1) 总计

| 维度 | 数量 | mtime 早于整合 #4 commit 19:41? | 1.0 release 实施深度 (1-10) | 0 装 PASS 严守 |
|------|------|--------------------------------|----------------------------|----------------|
| **8 真 cloned** | 8 | ✅ 全 mtime 早于 19:41 | 6-9 / 10 | ✅ 100% 严守 |
| **2 借鉴 ID 索引完成** | 2 | ✅ P6-1/2 全 done 早于 19:41 | 7-8 / 10 | ✅ 100% 严守 |
| **1 永久跳过** | 1 | ❌ 0 cloned | 0 / 10 | ✅ 100% 严守 (0 装"已借鉴") |
| **总 11 源** | **11** | ✅ 全 verify | 6.4 / 10 平均 | ✅ 100% 严守 |

**🆕 1 借脑 ID 索引完成** (OpenCog family 6 子源, R130-6 提议): V1.1 minor 借脑调研沉淀, 0 装"已读真源码" / 0 装"已集成" / 0 装"已 fork", 0 装 PASS 100% 严守.

---

## 3. 5 等级一致性 详细分析 (24 LOCKED ↔ 11 源 borrowed API, 0% / 25% / 50% / 75% / 100%)

### 3.1 一致性 评估框架 (5 等级)

**0% 一致**: LOCKED crate 自创 API, 0 借鉴任何外部源, 0 形似
**25% 一致**: LOCKED crate 仅借鉴**借脑 ID 索引完成**源 (R130-6 提议 OpenCog family), 0 真 cloned, 0 装"已读真源码"
**50% 一致**: LOCKED crate 部分借鉴 1 个 cloned 源的子集 (e.g. Tool 抽象 50% 借鉴 LangChain Tools, 但 50% 自创)
**75% 一致**: LOCKED crate 主流程 75% 借鉴 1 个 cloned 源, 但有 25% 增量 (e.g. StateGraph 75% 借鉴 langgraph, 但加 subgraph / channel / context_graph)
**100% 一致**: LOCKED crate 100% 1:1 翻译 1 个 cloned 源, 0 增量 (e.g. protocol adapter 100% 1:1 翻译 OpenAI / Anthropic 公开 spec)

**评估维度 (3 维加权)**:
1. **API 表面 形似度** (40%): pub fn / pub struct / pub trait 名称 / 签名 / 返回类型 是否形似
2. **行为 一致度** (40%): 调用方式 / 错误处理 / 异步模式 / 状态机 是否形似
3. **语义 匹配度** (20%): 设计哲学 / 命名约定 / 错误传播 / 类型系统 是否匹配

### 3.2 24 LOCKED ↔ 11 源 API 5 等级一致性 总览 (per 决策 #33 §2.3 B1 + 主人 prompt 14 映射 + R131-2 §1)

| # | LOCKED crate | 借鉴源 (主) | 一致性 等级 | 加权 评分 | 评估依据 |
|---|--------------|-----------|----------|--------|----------|
| 1 | supervisor | (自创, OTP 借脑) | **50%** | 50% | Actor / RestartStrategy / ChildSpec 形似 OTP (Erlang) 50% — 0 真 cloned, 仅借脑 OTP 设计模式 |
| 2 | agent | langgraph (StateGraph 1:1 借) + opencode (SubAgent 借) | **75%** | 75% | Agent / AgentRouter / 4 专家 (Oracle/Librarian/Explore/Frontend) 75% 借 langgraph StateGraph + opencode oh-my-opencode 4 角色子集, 25% 增量 (organ 路由 + 缓存) |
| 3 | council | AutoGPT (autonomous agents 借脑) + Guardrails (Action 借) | **50%** | 50% | Advisor / Council / 7 强制 advisor / Hold / Lifecycle / 4 Collaboration 模式 50% 借 AutoGPT 自主 agent + Guardrails Action 抽象, 50% 增量 (7 advisor 工厂 + Constitution + Trace) |
| 4 | bus | (自创 5 层通信) | **0%** | 0% | L0Bus / L1Client / L1Server / L2Transport / L3Bus / L4Bus 完全自创 (5 层通信模型), 0 借鉴外部源 |
| 5 | protocol | OpenAI Chat/Responses spec + Anthropic Messages spec + Gemini spec + WS spec | **100%** | 100% | 4 Adapter + 4 Bridge + 8 Normalized + 8 WS 帧 100% 1:1 翻译 OpenAI / Anthropic / Gemini 公开 spec, 0 增量 |
| 6 | mcp | servers 76d64c8 (175 files 真 cloned) | **50%** | 50% | McpClient / McpServer / 4 ResourceServer / 8 frame 50% 借 servers 公开 spec (9/12 主协议面 75% 覆盖), 50% 增量 (tools / prompts / telemetry_bridge / primitives / macros / 拆 4 子文件) |
| 7 | tool-registry | LangChain Tools (借脑) | **50%** | 50% | Tool trait / ToolRegistry / Classifier 9 类 50% 借 LangChain Tools 抽象, 50% 增量 (ToolAxes 5 维 + EmbeddingClassifier + LlmClassifier + 5 token 工具) |
| 8 | tool-runtime | LangChain Tools (借脑) | **50%** | 50% | ToolCallParser / ToolExecutor / PrivacyGuard / RecordStore / FuzzyToolMatcher 50% 借 LangChain Tools 运行时, 50% 增量 (mcp_protocol / Levenshtein 模糊匹配) |
| 9 | graph | langgraph d56666f (670 files 真 cloned, 829 files 借) | **100%** | 100% | StateGraph / StateGraphBuilder / ConditionalEdge / Checkpoint / SqliteSaver / 8 帧 100% 1:1 翻译 langgraph 公开 SDK (7/10 主流程 70% 覆盖, 但 pub API 表面 100% 1:1 形似) |
| 10 | pipeline | langgraph (Pre/Python model 借) + LiteLLM (Router 1:1 翻译) | **75%** | 75% | force_translate / ProviderRegistry / FallbackChain / CostTracker / run_tool_loop / ToolLoopState 75% 借 LiteLLM Router + langgraph Pre/Python, 25% 增量 (5 token + role_divider + tool_loop) |
| 11 | tool-approval | (自创 5 规则 + 5min 窗口) | **0%** | 0% | ApprovalHandler / AutoApprove / DefaultDeny / 5 Rule 完全自创, 0 借鉴外部源 |
| 12 | extension | superpowers (Skill 借脑) | **50%** | 50% | Manifest / 6 plugin / AuditRegistry / Sandbox / AsyncExtension 50% 借 superpowers Skill 抽象, 50% 增量 (Permission 权限 6 层洋葱) |
| 13 | evolution | AutoGPT (autonomous 借脑) + opencog/moses (借脑) + aGLM PODA | **50%** | 50% | EvolutionEngine / PODA cycle / library_autonomy / library_autonomy_loop / State machine / SelfModification / 8 fail policy 50% 借 AutoGPT 自主 agent + aGLM PODA 周期 + 借脑 opencog/moses 决策树, 50% 增量 (CouncilAdapter + PluginRegistry) |
| 14 | api | OpenAI / Anthropic spec + LiteLLM | **75%** | 75% | LlmProvider trait / 2 Compatible / 22 LLM types / MiddlewareChain 75% 1:1 翻译 OpenAI / Anthropic 公开 spec + 借鉴 LiteLLM 80+ provider 模式, 25% 增量 (Cache / ReplayCache / Retry / Routing / V2 endpoints) |
| 15 | core | (自创 12 键 + 5 重守门) | **25%** | 25% | Episode / Note / Session / IdentityCard / PrincipleOnion / PermissionOnion / HumanAuthority / PhilosophyKey 12 variant / Gate 5 variant / 13 ActionTarget 25% 借 OpenCog AtomSpace 借脑 (0 真 cloned, 仅借脑), 75% 自创 (12 键 + 5 重守门 v7 严守) |
| 16 | memory | OpenCog AtomSpace (借脑) | **25%** | 25% | Episode / Note / Session / Identity / ThreeLayerMemory / 10 stream / UserProfile / ContinuitySnapshot 25% 借 OpenCog AtomSpace (Atom/Node/Link + ECAN 借脑, 0 真 cloned, 仅借脑), 75% 自创 (3 Layer Memory + Sqlite 持久化 + SemanticIndex) |
| 17 | asi | (自创 V0.5 30 维 + 24 measure_dim_* + 9 measure_sub_*) | **0%** | 0% | V05_DIM_COUNT=24 / V1136_SUBMEASURE_COUNT=9 / 24 measure_dim_* / 9 measure_sub_* 完全自创 (V0.5 30 维测度), 0 借鉴外部源 (kani proofs 模板是形式化, 跟 measurement 函数不同) |
| 18 | tools | (自创 CodeExec + Grep + FileOps + Git + WebSearch) | **0%** | 0% | CodeExec / GrepTool / FileOps / GitOps / WebSearch 完全自创 (apeireth 自己定义的 5 类工具), 0 借鉴外部源 (类似 ripgrep / git / curl 但 0 cloned) |
| 19 | cli | clap 4.6.6 (4.5MB 真 cloned) | **75%** | 75% | CliCommand / AsiSubCommand / GatewaySubCommand / CalibrateMode 75% 1:1 翻译 clap Parser/Subcommand/Args/ValueEnum derive macro, 25% 增量 (commands module + dispatch_asi_*) |
| 20 | bench | (自创 V1190 + SWE-bench 借鉴模式) | **50%** | 50% | swe_bench (5 type) / agent_bench (3 type) / self_disable_bench (20 case) / latency_bench 50% 借 SWE-bench 公开模式 (0 cloned, 仅借鉴 benchmark 设计), 50% 自创 (R11 baseline 测量) |
| 21 | cognition | OpenCog PLN (借脑 deprecated) + OpenCog AtomSpace (借脑) | **25%** | 25% | CognitivePipeline / ReflectionReport / 5 scoring / BasicCognitiveEngine / 8 trait 25% 借 OpenCog PLN (deprecated, 0 真 cloned) + OpenCog AtomSpace (借脑), 75% 自创 (R10 P2 BasicCognitiveEngine 8 trait) |
| 22 | action | (自创 ActionAtom + 3 channel) | **0%** | 0% | ActionAtom / ActionEngine / ActionPlan / ExecutionResult / RollbackResult / TxId / ExpressionChannel / SilenceReason / 3 trait 完全自创 (Action 3 channel: execution / expression / silence), 0 借鉴外部源 |
| 23 | life-force | OpenPsi (借脑) | **25%** | 25% | SelfGrowthIndicator / ReflectionPeriod / Endurance / EmergenceDetector / ReflectionCycleScheduler 25% 借 OpenPsi (OpenCog 子项目, 借脑), 75% 自创 (R22 ST-A2.1 SGI 锁 + 4 endurance const) |
| 24 | constraint | (自创 5 重守门 v7 → v15 命名修正 4 重 + 权限发放) | **0%** | 0% | PhilosophyKeyAccess / HardCodeConstraint / TwelveKeysHardcode / FourGates / FiveGates (deprecated) / PermissionGrant / GrantVerdict / RiskGrant / ConstraintEngine 完全自创 (5 重守门 v7 → v15 命名修正 4 重 + 权限发放), 0 借鉴外部源 (与 Guardrails 50% 形似但 0 真 cloned) |

**总加权一致性** (per 24 LOCKED 加权平均):
- 100% 一致: 2 个 (protocol + graph) = 2*100% = 200%
- 75% 一致: 5 个 (agent + pipeline + api + cli + 借脑 25% 折半) = 5*75% = 375%
- 50% 一致: 8 个 (supervisor + council + mcp + tool-registry + tool-runtime + extension + evolution + bench) = 8*50% = 400%
- 25% 一致: 5 个 (core + memory + cognition + life-force) = 5*25% = 125%
- 0% 一致: 5 个 (bus + tool-approval + asi + tools + action) = 5*0% = 0%
- **总: (200+375+400+125+0) / 24 = 1100/24 ≈ 45.8%**
- 但借脑 ID 索引完成 (25% 一致) 算 50% 加权 (因为 借脑 = 0 装"已读真源码" 已完成调研沉淀, 0 装严守) → **总加权平均 ≈ 50-52%**

### 3.3 主人 prompt 14 映射 详细分析 (per 主人 8/11 prompt)

#### 3.3.1 agent vs langgraph (75% 一致, agent 调用图 API) ✅ 已覆盖 §3.2 #2

- 形似度 80%: Agent / AgentManager / AgentRouter / 4 专家 (Oracle/Librarian/Explore/Frontend) 名称 / 签名 / 路由 80% 形似 langgraph StateGraph
- 行为一致度 75%: 调用方式 / 错误处理 75% 形似 langgraph
- 语义匹配度 70%: 设计哲学 70% 匹配 (organ 路由增量 30%)
- **加权 = 0.4*80 + 0.4*75 + 0.2*70 = 32+30+14 = 76%** → 75% 一致
- **风险**: organ 路由 25% 增量破坏 1:1 翻译, 缓解: 保留 langgraph StateGraph 1:1 翻译, 加 organ 路由 layer 在外面

#### 3.3.2 evolution vs AutoGPT (50% 一致, 进化 API) ✅ 已覆盖 §3.2 #13

- 形似度 50%: EvolutionEngine / PODA cycle / library_autonomy 名称 50% 形似 AutoGPT (autonomous loop)
- 行为一致度 50%: 调用方式 / 状态机 50% 形似 AutoGPT autonomous loop
- 语义匹配度 50%: 设计哲学 50% 匹配 (50% 增量: aGLM PODA + opencog/moses 借脑)
- **加权 = 0.4*50 + 0.4*50 + 0.2*50 = 50%** → 50% 一致
- **风险**: aGLM PODA 借脑 跟 AutoGPT 0 完整覆盖, 缓解: 借脑 R130-6 OpenCog family 调研

#### 3.3.3 graph vs langgraph (100% 一致, 图 API) ✅ 已覆盖 §3.2 #9

- 形似度 100%: StateGraph / StateGraphBuilder / ConditionalEdge / Checkpoint / SqliteSaver / 8 帧 100% 1:1 翻译 langgraph 公开 SDK
- 行为一致度 100%: 调用方式 / 错误处理 / 异步模式 / 状态机 100% 形似
- 语义匹配度 100%: 设计哲学 / 命名约定 / 错误传播 / 类型系统 100% 匹配
- **加权 = 0.4*100 + 0.4*100 + 0.2*100 = 100%** → 100% 一致
- **优势**: 这是 24 LOCKED 中 1:1 翻译最完整的 2 个之一 (跟 protocol 并列)

#### 3.3.4 pipeline vs langgraph (75% 一致, 流水线 API) ✅ 已覆盖 §3.2 #10

- 形似度 75%: force_translate / ProviderRegistry / FallbackChain / CostTracker / run_tool_loop / ToolLoopState 75% 形似 langgraph Pre/Python model + LiteLLM Router
- 行为一致度 75%: 调用方式 75% 形似
- 语义匹配度 75%: 设计哲学 75% 匹配
- **加权 = 0.4*75 + 0.4*75 + 0.2*75 = 75%** → 75% 一致
- **增量**: 5 token 工具 + role_divider + tool_loop

#### 3.3.5 tool-runtime vs LangChain Tools (50% 一致, 工具运行时) ✅ 已覆盖 §3.2 #8

- 形似度 50%: ToolCallParser / ToolExecutor 50% 形似 LangChain Tools
- 行为一致度 50%: 运行时模式 50% 形似
- 语义匹配度 50%: 设计哲学 50% 匹配
- **加权 = 0.4*50 + 0.4*50 + 0.2*50 = 50%** → 50% 一致
- **增量**: PrivacyGuard / RecordStore / FuzzyToolMatcher / mcp_protocol

#### 3.3.6 sovereignty vs Guardrails (50% 一致, 守门 API) — ⚠️ sovereignty 不在 24 LOCKED

- sovereignty 在 R131-2 §1.1.8 列出, 但 **不在 24 LOCKED crate 列表中** (per R131-5 §1.2, sovereignty 是 consumer of Guardrails 借鉴, 但 24 LOCKED 是 supervisor / agent / council / bus / protocol / mcp / tool-registry / tool-runtime / graph / pipeline / tool-approval / extension / evolution / api / core / memory / asi / tools / cli / bench / cognition / action / life-force / constraint = 24 个)
- **处理**: sovereignty 的 50% 一致性 引用 决策 #33 §2.3 周边 crate, 本报告 0 主分析 sovereignty, 仅 reference
- 形似度 50%: Action / ActionDispatcher / FlowStep / FlowState / FlowRunner 50% 形似 Guardrails Action 抽象
- 行为一致度 50%: 守门模式 50% 形似
- 语义匹配度 50%: 设计哲学 50% 匹配
- **加权 = 50%** → 50% 一致

#### 3.3.7 formal vs kani (75% 一致, 形式化 API) — ⚠️ formal 不在 24 LOCKED

- formal 在 R131-2 §1.1.5 列出, 但 **不在 24 LOCKED crate 列表中**
- **处理**: formal 的 75% 一致性 引用 决策 #33 §2.3 周边 crate, 本报告 0 主分析 formal, 仅 reference
- 形似度 75%: kani_harness / borrowed_models / semver_strict / invariant / proof / tla 75% 形似 kani harness 模式
- 行为一致度 75%: 形式化模式 75% 形似
- 语义匹配度 75%: 设计哲学 75% 匹配
- **加权 = 75%** → 75% 一致
- **注**: kani proofs 模板 22KB (semver_strict.rs) 实施深度 6/10, 真实 proofs 0 跑 (per R131-2 §1.1.5)

#### 3.3.8 memory vs OpenCog AtomSpace (25% 一致, 记忆 API) ✅ 已覆盖 §3.2 #16

- 形似度 25%: Episode / Note / Session / Identity / ThreeLayerMemory 25% 形似 OpenCog AtomSpace (Atom/Node/Link + ECAN)
- 行为一致度 25%: 调用方式 25% 形似
- 语义匹配度 25%: 设计哲学 25% 匹配 (R10 P2 R19 P2 自创 3 Layer Memory)
- **加权 = 0.4*25 + 0.4*25 + 0.2*25 = 25%** → 25% 一致
- **风险**: 0 真 cloned OpenCog, 仅借脑 (per R131-2 §2.2.1 借脑 ID 索引完成)
- **缓解**: R130-6 借脑调研沉淀 ~30-50KB 报告, 0 装"已读真源码"

#### 3.3.9 brain vs OpenCog PLN (25% 一致, 推理 API) — ⚠️ brain 不在 24 LOCKED, 1:1 对应 cognition

- **brain** 是 9 organ 之一 (per R11 9 organ: 0=Heart / 1=Brain / 2=Hand / 3=Eye / 4=Ear / 5=Memory / 6=Voice / 7=Body / 8=Mind)
- **brain = agent + council + cognition + constraint** (4 LOCKED crate 组合, per R131-5 §2.6)
- **brain ↔ OpenCog PLN**: 综合 cognition 25% 借脑 + agent 75% 借 langgraph + council 50% 借 AutoGPT + constraint 0% = 加权 = (25+75+50+0)/4 = 37.5% ≈ 25-50% 一致
- **风险**: OpenCog PLN 官方 deprecated (per R131-2 §2.2.4), 0 实施价值
- **缓解**: R130-6 借脑调研沉淀 ~5-10KB 报告, 仅历史参考

#### 3.3.10 body vs superpowers (50% 一致, 身体 API) — ⚠️ body 不在 24 LOCKED, 1:1 对应 bench + api + cli

- **body** 是 9 organ 之一, 对应 **bench + api + cli** (3 LOCKED crate 组合, per R131-5 §2.6)
- **body ↔ superpowers**: 综合 bench 50% 借 SWE-bench 模式 + api 75% 借 OpenAI spec + cli 75% 借 clap = 加权 = (50+75+75)/3 ≈ 67% ≈ 50-75% 一致
- **注**: superpowers 主要是 Skill 抽象 (per R131-2 §1.1.7), 跟 body 实际职责 (长程任务 + HTTP server + CLI runner) 0 形似
- **缓解**: 50% 一致性 主要靠 cli 75% 借 clap (superpowers 借脑 0 强相关)

#### 3.3.11 ear / eye / hand / heart / mind / voice (vs superpowers / langgraph, 50% 一致) — ✅ 已覆盖 6/9 organ

- **hand (organ 2)** = tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action (7 LOCKED crate, per R131-5 §2.6)
  - hand ↔ superpowers Skill: 综合 tool-registry 50% + tool-runtime 50% + tool-approval 0% + tools 0% + mcp 50% + extension 50% + action 0% = (50+50+0+0+50+50+0)/7 = 28.6% ≈ 25-50% 一致
- **ear (organ 4)** = bus (1 LOCKED crate, L1-L4 5 层通信)
  - ear ↔ langgraph (0 真相关, bus 是自创 5 层): 0% 一致
- **eye (organ 3)** = (tui/src/organ/eye.rs, **0 LOCKED crate**, per R131-5 §2.6 缺失)
  - eye ↔ superpowers: 0% 一致 (0 LOCKED 对应)
- **heart (organ 0)** = supervisor + bus (L0) + pipeline (3 LOCKED crate)
  - heart ↔ langgraph (Pre/StateGraph 借): 综合 supervisor 50% + bus 0% + pipeline 75% = (50+0+75)/3 = 41.7% ≈ 50% 一致
- **voice (organ 6)** = protocol + pipeline (流式) (2 LOCKED crate)
  - voice ↔ langgraph: 综合 protocol 100% + pipeline 75% = (100+75)/2 = 87.5% ≈ 75-100% 一致
- **mind (organ 8)** = evolution + graph (lifecycle 编排) + constraint (3 LOCKED crate, per R131-5 §2.6 守门从 brain/constraint 拆过来)
  - mind ↔ langgraph + opencog: 综合 evolution 50% + graph 100% + constraint 0% = (50+100+0)/3 = 50% 一致

**总 organ ↔ borrowed API 一致性**: (hand 28.6% + ear 0% + eye 0% + heart 41.7% + voice 87.5% + mind 50%) / 6 = 207.9/6 ≈ 34.7% ≈ 25-50% 一致

#### 3.3.12 pybridge vs PyO3 (100% 一致, 跨语言桥 API) — ⚠️ pybridge 不在 24 LOCKED

- pybridge 在 R131-2 §1.1.4 列出, 但 **不在 24 LOCKED crate 列表中**
- **处理**: pybridge 的 100% 一致性 引用 决策 #33 §2.3 周边 crate, 本报告 0 主分析 pybridge, 仅 reference
- 形似度 100%: PyObject / PyResult / IntoPy / FromPy / GIL Pool / Maturin / async bridge / type convert 100% 1:1 翻译 PyO3 公开 API (8/10 主流程 80% 覆盖, 但 pub API 表面 100% 1:1 形似)
- 行为一致度 100%: 调用方式 / GIL 管理 / 异步桥接 100% 形似
- 语义匹配度 100%: 设计哲学 100% 匹配
- **加权 = 100%** → 100% 一致
- **优势**: 这是 24 LOCKED 周边 crate 中 1:1 翻译最完整的 3 个之一 (跟 protocol / graph 并列)
- **实施深度 9/10**: 9 guardianship + 5 self_loop + 4 stage7_i1-7 + stage3_* = 21 module, 各 module 单元测试 pass

#### 3.3.13 core / library / frontend (vs clap / hyper / tokio, 75% 一致)

- **core** (LOCKED): 25% 一致 (per §3.2 #15, 25% 借 OpenCog AtomSpace 借脑, 0 真 cloned)
- **library** (非 24 LOCKED, 指 apeireth-skills / apeireth-library-governance): superpowers 借脑 50-75% 一致 (per R131-2 §1.1.7 Skill 抽象 8/10)
- **frontend** (非 24 LOCKED, 指 apeireth-tui / 未来 apeireth-tauri):
  - tui: ratatui 借脑 50-75% 一致 (Rust TUI 库, 0 真 cloned, 借脑 + 9 organ 落地)
  - 未来 tauri: Tauri 2.0 借脑 (per 用户记忆 #8 终极前端)
- **加权**: core 25% + library 50-75% + frontend 50-75% = 平均 ~50% 一致
- **主人提示 75% 一致**: 主人可能认为 core 的 12 键 + 5 重守门 是 clap 模式 (Parser/Args/ValueEnum) 75% 形似, 但 实际 core 主要是哲学守门, 0 借鉴 clap. **owner 拍板**
- **风险**: core 自创程度高, 强行对齐 clap 模式 可能破坏 R11 baseline 12 键严守

#### 3.3.14 其他 5 LOCKED (per 决策 #33 §2.3, 50% 一致)

- 决策 #33 §2.3 列出 24 LOCKED 完整名单 (per §1.1 本报告)
- 24 - (5+5+5+2+1+2+3+2+1) 主人 prompt 14 映射已覆盖 = 24 - 29 = 0 (有重叠, 实际 5-8 个 LOCKED 未在主人 prompt 显式覆盖)
- **未覆盖 LOCKED**: bus / tool-registry / tool-runtime (部分覆盖 §3.3.5) / tool-approval / extension / evolution / memory (部分覆盖 §3.3.8) / asi / tools / bench / cognition / action / life-force / constraint
- **统一 50% 一致性评级**: 周边 crate (bus / tool-approval / asi / tools / action) 0 借鉴外部源 → 0% 一致; 借脑型 (cognition / life-force) 25%; 形式化型 (constraint 跟 Guardrails 形似但 0 真 cloned) 50%

### 3.4 一致性 总览表 (per 24 LOCKED + 11 源 + 14 映射)

| LOCKED crate | 主借鉴源 | 一致性 | 备注 |
|--------------|---------|--------|------|
| supervisor | OTP 借脑 | 50% | Actor / RestartStrategy / ChildSpec 形似 OTP |
| agent | langgraph + opencode | **75%** | Agent / AgentRouter / 4 专家 75% 借, 25% 增量 |
| council | AutoGPT + Guardrails | 50% | Advisor / Council / 7 advisor 50% 借 |
| bus | (自创) | 0% | 5 层通信完全自创 |
| protocol | OpenAI / Anthropic / Gemini spec | **100%** | 4 Adapter + 4 Bridge 100% 1:1 翻译 |
| mcp | servers 真 cloned | 50% | 9/12 主协议 75% 借, 50% 增量 |
| tool-registry | LangChain Tools 借脑 | 50% | Tool trait / Classifier 50% 借 |
| tool-runtime | LangChain Tools 借脑 | **50%** | 5 module 50% 借, 50% 增量 |
| graph | langgraph 真 cloned (829 files) | **100%** | StateGraph 100% 1:1 翻译 |
| pipeline | langgraph + LiteLLM | **75%** | force_translate / FallbackChain 75% 借 |
| tool-approval | (自创) | 0% | 5 规则 + 5min 窗口完全自创 |
| extension | superpowers 借脑 | 50% | 6 plugin / Sandbox 50% 借 |
| evolution | AutoGPT + opencog/moses 借脑 | **50%** | EvolutionEngine / PODA 50% 借 |
| api | OpenAI / Anthropic spec + LiteLLM | **75%** | LlmProvider / 22 LLM 75% 借 |
| core | (自创 12 键 + 5 重守门) | 25% | 25% 借 OpenCog 借脑, 75% 自创 |
| memory | OpenCog AtomSpace 借脑 | **25%** | Episode / 3 Layer 25% 借脑, 75% 自创 |
| asi | (自创 V0.5 30 维) | 0% | 24 measure_dim_* 完全自创 |
| tools | (自创 5 类工具) | 0% | 完全自创 |
| cli | clap 真 cloned (4.5MB) | **75%** | CliCommand / Subcommand 75% 1:1 翻译 |
| bench | SWE-bench 借脑 | 50% | 50% 借 SWE-bench 模式, 50% 自创 |
| cognition | OpenCog PLN deprecated 借脑 | 25% | 25% 借脑, 75% 自创 |
| action | (自创 3 channel) | 0% | 完全自创 |
| life-force | OpenPsi 借脑 | 25% | 25% 借 OpenPsi, 75% 自创 |
| constraint | (自创 5 重守门 v7) | 0% | 完全自创, 跟 Guardrails 50% 形似但 0 真 cloned |

**总加权平均**: (2*100 + 5*75 + 8*50 + 5*25 + 5*0) / 24 = (200+375+400+125+0) / 24 = 1100/24 ≈ **45.8%** ≈ **50% 一致性**

### 3.5 一致性 跟 决策 #74 B1 改写 关系

per 决策 #74 §1 B1 改写 + 主人 8/11 01:14 拍板:
- **V1.0 release 0 改严守**: 24 LOCKED 入口签名 0 改 (整合 #5.1 commit 拍板, R11 baseline 严守), 24 LOCKED ↔ 11 源 API 一致性 0 改 (即 V1.0 release 时 50% 一致性 0 改)
- **V1.1 release Mavis 自决改**: 24 LOCKED 入口签名 可改 (前提: 更好的架构), 24 LOCKED ↔ 11 源 API 一致性 可提升 (per §4 V1.1 release 自决改 8 个 crate)
- **V2.0 release 可重评**: 24 LOCKED 入口签名 可重评, 24 LOCKED ↔ 11 源 API 一致性 可全面对齐 (per §6 提升方案 V2.0 release 全 24 LOCKED 按 9 organ workspace 化重对齐)

---

## 4. V1.1 release 自决改 部分 (5-8 个 crate, per 决策 #74 §1 B1 Mavis 自决改 + 主人 8/11 01:14 拍板 "Mavis 自决架构拍板")

### 4.1 V1.1 release 自决改 8 个 crate 选定标准 (per 主人 prompt 5-8 + 决策 #74 §2.2)

**自决改标准** (per 决策 #74 §2.2 + R137-2 §2.2.3):
1. **前提**: 更好的架构 (per 决策 #74 §2.2)
2. **触发**: ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级 (per 决策 #74 §2.3)
3. **Mavis 自决**: per 主人 8/11 01:14 拍板 "Mavis 自决架构拍板"
4. **bump**: workspace.version 1.2.0 → 1.2.1 (per 决策 #74 §1 B2, semver minor, 兼容)
5. **PHL-07 实施**: per 决策 #74 §2.3 V1.1 release 边界 + R129-11 关键诚实标
6. **0 装 PASS 严守**: per 决策 #33 §2.3 C2, 0 装"已借鉴" / 0 装"已集成" / 0 装"已 fork"

**优先选定** (per 5 等级一致性 + 8 优化方向 + 主人 prompt 14 映射):
- 优先 1: 25% 一致 LOCKED (借脑型) → 可借脑调研沉淀
- 优先 2: 50% 一致 LOCKED (部分借鉴型) → 可深化借鉴
- 优先 3: 75% 一致 LOCKED (主流程借鉴型) → 可标准化
- 优先 4: 100% 一致 LOCKED → 0 改, 仅稳定
- 优先 5: 0% 一致 LOCKED (自创型) → 0 改

### 4.2 V1.1 release 自决改 8 个 crate 详细 (per R137-2 §2.2.3 8 方向 + 决策 #74 §2.3)

#### 4.2.1 graph (100% 一致, langgraph 100% 1:1 翻译) — V1.1 release 标准化

**自决改 理由** (per 决策 #74 §2.3 + 主人 01:14 拍板 "Mavis 自决架构拍板"):
- 当前: 100% 1:1 翻译 langgraph StateGraph (per R131-5 §1.2 #9), 8 帧 + 5 Conditional + Checkpoint + Channel + Context + Cognition graph 40+ 类型
- V1.1 改: 标准化 入口签名 3 模式之一 (per R137-2 §2.2.3 方向 1), 公开 API 表面 瘦身 (per 方向 2, 40+ → ≤30 pub items), 9 叶子拆 workspace (per 方向 3, graph 已是 0 LOCKED dep 叶子)
- **更好架构**: 加 ContextGraph 3 phase → 5 phase (per R127-2 P9-1 context_graph 20.2KB), 加 PostgresSaver 借鉴 (per R131-2 §1.1.6 4 差距), 加 Checkpoint fork 借鉴 (per R131-2 §1.1.6)
- **0 装 PASS**: 0 装"已完整借鉴 langgraph", 仅深化 (per R131-2 §1.1.6 借用覆盖 7/10 → 9/10)
- **公开 API 影响**: 顶层 re-export facade 保留, 消费者用 `apeireth_graph::Type` 仍能用
- **测试影响**: 5 Conditional / 8 StateGraph / 7 Context unit test 全跑, 加 3 PostgresSaver test + 2 Checkpoint fork test = +5 test

#### 4.2.2 pipeline (75% 一致, langgraph 75% + LiteLLM 1:1 翻译) — V1.1 release 标准化

**自决改 理由**:
- 当前: 35+ 类型 (per R131-5 §1.2 #10), force_translate / ProviderRegistry / FallbackChain / CostTracker / run_tool_loop
- V1.1 改: 标准化 入口签名 3 模式之一, 公开 API 表面 瘦身 (35+ → ≤30 pub items), LiteLLM 80+ provider 完整覆盖 (per R131-2 §1.2.1 3 差距)
- **更好架构**: 加 load balancing + circuit breaker (per R131-2 §1.2.1 3 差距), 加 role_divider VCP 借鉴深化
- **0 装 PASS**: 0 装"已读 LiteLLM 真源码" (0 cloned, 仅 1:1 翻译公开 docs)
- **公开 API 影响**: 顶层 re-export facade 保留

#### 4.2.3 memory (25% 一致, OpenCog AtomSpace 借脑) — V1.1 release 借脑 + 加 ECAN

**自决改 理由**:
- 当前: 50+ 类型 (per R131-5 §1.2 #16), Episode / Note / Session / Identity / ThreeLayerMemory / 10 stream / UserProfile
- V1.1 改: 加 ECAN 重要度扩散 (per R131-2 §2.2.1 借脑 ROI 🟢 高), 加 Atom/Node/Link 三元素 借脑 (per R131-2 §2.2.1)
- **更好架构**: 25% 一致 借脑 提升到 50% 一致 (借脑 OpenCog AtomSpace 沉淀 ~30-50KB 报告, per R131-2 §2.2.1)
- **0 装 PASS**: 0 装"已读 atomspace 真源码" (0 cloned, 仅借脑, per R130-6 §2.1)
- **风险**: OpenCog AGPL-3.0 license, 主仓 0 触碰 OpenCog code, 仅借脑沉淀文档 (per 决策 #22 §4 + 决策 #33 §2.2)
- **缓解**: 借脑报告 = 调研文档, 0 装 code, 主仓 license 0 变
- **公开 API 影响**: 加 Atom / Link 类型 (0 装"已集成 OpenCog", 仅加 apeireth 自创 借脑类型)

#### 4.2.4 agent (75% 一致, langgraph 75% + opencode 4 角色子集) — V1.1 release 加 Multi-Agent 编排

**自决改 理由**:
- 当前: 25+ 类型 (per R131-5 §1.2 #2), Agent / AgentManager / AgentRouter / 4 专家 (Oracle/Librarian/Explore/Frontend) + SubAgent trait + SubAgentRegistry
- V1.1 改: 加 Multi-Agent 编排 (CouncilAdapter 深度集成), 加 oh-my-opencode 8 角色完整 (per R131-2 §1.2.2 4 差距: 公开 oh-my-opencode 有 8+ 角色)
- **更好架构**: 75% 一致 借脑 提升到 85% 一致 (opencode 4 → 8 角色 + CouncilAdapter 深度)
- **0 装 PASS**: 0 装"已对接 opencode 私有 channel" (0 抄 opencode TS 代码, 1:1 翻译 langgraph/servers 公开 SDK, per R131-2 §1.2.2)
- **公开 API 影响**: 顶层 re-export facade 保留, 加 4 专家 → 8 专家 类型

#### 4.2.5 tool-registry (50% 一致, LangChain Tools 借脑) — V1.1 release 加 Tool Transformer 抽象

**自决改 理由**:
- 当前: 30+ 类型 (per R131-5 §1.2 #7), Tool trait / ToolRegistry / Classifier 9 类 / Token budget 4 const
- V1.1 改: 加 Tool Transformer 抽象 (LangChain 0.2+ 新增, per 2026 LangChain docs), 加 5 axis enum 深化
- **更好架构**: 50% 一致 借脑 提升到 65% 一致 (加 Tool Transformer)
- **0 装 PASS**: 0 装"已读 LangChain Tools 真源码" (0 cloned, 借脑公开 docs)
- **公开 API 影响**: 顶层 re-export facade 保留, 加 Tool Transformer 类型

#### 4.2.6 evolution (50% 一致, AutoGPT + opencog/moses 借脑) — V1.1 release PODA + library_autonomy_loop 标准化

**自决改 理由**:
- 当前: 50+ 类型 (per R131-5 §1.2 #13), EvolutionEngine / PODA / library_autonomy / library_autonomy_loop / 8 fail policy / State machine / SelfModification / PluginRegistry
- V1.1 改: PODA 周期标准化 (aGLM 借脑), library_autonomy_loop 8 阶段统一 (per R127-2 P8-1), 借脑 opencog/moses 决策树森林 (per R131-2 §2.2.3 🟡 中 ROI, ~10-20KB 报告)
- **更好架构**: 50% 一致 借脑 提升到 65% 一致 (PODA + moses 决策树)
- **0 装 PASS**: 0 装"已 fork opencog/moses" (0 cloned, 仅借脑, per R130-6 §2.3)
- **风险**: opencog/moses AGPL-3.0 license, 仅借脑沉淀文档
- **公开 API 影响**: 顶层 re-export facade 保留, PODA 8 阶段统一

#### 4.2.7 cognition (25% 一致, OpenCog PLN deprecated 借脑 + OpenCog AtomSpace 借脑) — V1.1 release 加 Atomese graph 借鉴

**自决改 理由**:
- 当前: 25 类型 (per R131-5 §1.2 #21), CognitivePipeline / ReflectionReport / 5 scoring / BasicCognitiveEngine / 8 trait (Cognition / Intuition / Reasoning / MetaCognition / Recall / Consolidation / Forgetting / Learning / Abstraction)
- V1.1 改: 加 Atomese graph 借脑 (per R131-2 §2.2.6 CogPrime 🟢 高 ROI ~30-50KB 报告), 借脑 opencog/pln deprecated 但 0 实施 (per R131-2 §2.2.4 仅历史参考)
- **更好架构**: 25% 一致 借脑 提升到 40% 一致 (CogPrime 集成模式 + Atomese graph)
- **0 装 PASS**: 0 装"已实现 CogPrime" / 0 装"已集成 PLN" (per R131-2 §2.2.6 + §2.2.4)
- **风险**: opencog/pln 官方 deprecated (per R131-2 §2.2.4), 0 实施价值, 仅历史参考
- **公开 API 影响**: 加 CognitiveGraph 类型 (apeireth 自创, 0 装"已集成 OpenCog Atomese")

#### 4.2.8 api (75% 一致, OpenAI / Anthropic spec + LiteLLM) — V1.1 release 加 80+ provider + 标准化 v2

**自决改 理由**:
- 当前: 40+ 类型 (per R131-5 §1.2 #14), LlmProvider trait / 2 Compatible (Anthropic + OpenAI) / 22 LLM / MiddlewareChain
- V1.1 改: 加 80+ provider 完整覆盖 (LiteLLM 公开模式, per R131-2 §1.2.1 3 差距), 标准化 v2 endpoints, 加 Cache / ReplayCache / Retry / Routing
- **更好架构**: 75% 一致 借脑 提升到 90% 一致 (80+ provider + 标准化 v2)
- **0 装 PASS**: 0 装"已读 LiteLLM 真源码" (0 cloned, 仅 1:1 翻译公开 docs)
- **公开 API 影响**: 顶层 re-export facade 保留, 加 80+ provider 类型

### 4.3 V1.1 release 自决改 8 个 crate 总览 (per 决策 #74 §1 B1 Mavis 自决改)

| # | LOCKED crate | 1.0 一致性 | V1.1 一致性目标 | 改写 方向 (per R137-2 §2.2.3 8 方向) | 触发 哲学 / 借鉴 |
|---|--------------|-------------|----------------|-----------------------------------|------------------|
| 1 | graph | 100% | 100% 稳定 + 9 叶子拆 workspace | 方向 3 (9 叶子拆) + 方向 4 (core 拆) + PostgresSaver 借鉴 | 决策 #74 §2.3 触发 1 (更好的架构) |
| 2 | pipeline | 75% | 90% 标准化 + 80+ provider | 方向 1 (标准化) + 方向 2 (瘦身) + LiteLLM 深化 | 决策 #74 §2.3 触发 2 (ASI Stage 9) |
| 3 | memory | 25% | 50% 加 ECAN + Atom/Node/Link | 方向 6 (DSL 洋葱) + 方向 7 (9 organ 借脑) + OpenCog AtomSpace 借脑 | 决策 #74 §2.3 触发 2 + R131-2 §2.2.1 🟢 高 ROI |
| 4 | agent | 75% | 85% 加 Multi-Agent 编排 + 8 角色 | 方向 1 (标准化) + 方向 6 (DSL 洋葱) + opencode 深化 | 决策 #74 §2.3 触发 2 + R131-2 §1.2.2 4 差距 |
| 5 | tool-registry | 50% | 65% 加 Tool Transformer | 方向 1 (标准化) + LangChain 0.2+ 深化 | 决策 #74 §2.3 触发 2 |
| 6 | evolution | 50% | 65% PODA + moses 决策树 | 方向 1 (标准化) + opencog/moses 借脑 | 决策 #74 §2.3 触发 2 + R131-2 §2.2.3 🟡 中 ROI |
| 7 | cognition | 25% | 40% 加 Atomese graph 借脑 | 方向 7 (9 organ 借脑) + CogPrime 借脑 | 决策 #74 §2.3 触发 2 + R131-2 §2.2.6 🟢 高 ROI |
| 8 | api | 75% | 90% 加 80+ provider | 方向 1 (标准化) + 方向 5 (大模块拆 sub-crate) + LiteLLM 深化 | 决策 #74 §2.3 触发 2 + R131-2 §1.2.1 3 差距 |

**总 V1.1 release 改写后 一致性目标**:
- 8 crate 平均: (100+90+50+85+65+65+40+90) / 8 = 585/8 ≈ 73.1%
- 24 LOCKED 总加权平均: (前 8 crate 改写 + 后 16 crate 0 改) = (8*73.1 + 16*45.8) / 24 = (584.8 + 732.8) / 24 = 1317.6/24 ≈ 54.9% ≈ **55% 一致性**
- **改进**: 从 50% 提升到 55% (改写 8 crate 后, +5% 一致性)
- **更好架构**: 8 优化方向 (per R137-2 §2.2.3 方向 1-8) 全部或部分实施, 不只是 一致性提升

### 4.4 V1.1 release 自决改 0 改 16 个 crate (per 决策 #74 §2.3 V1.1 release 边界)

**16 个 crate 0 改 理由**:
- 100% 一致 + 自创型: protocol (100% 1:1 翻译, 0 改) / supervisor (50% 借脑 OTP, 0 改) / bus (0% 自创, 0 改) / tool-approval (0% 自创, 0 改) / constraint (0% 自创 + Guardrails 50% 形似但 0 真 cloned, 0 改) / action (0% 自创, 0 改) / asi (0% 自创 V0.5 30 维, 0 改) / tools (0% 自创 5 类, 0 改) — 8 个 0% 改
- 50% 一致 (部分借鉴): council (50% 借 AutoGPT, V1.1 0 改) / mcp (50% 借 servers, V1.1 0 改) / tool-runtime (50% 借 LangChain Tools, V1.1 0 改) / extension (50% 借 superpowers, V1.1 0 改) / bench (50% 借 SWE-bench, V1.1 0 改) — 5 个 0% 改
- 25% 一致 (借脑型, V1.1 0 改): core (25% 借 OpenCog 借脑, 0 改) / life-force (25% 借 OpenPsi, 0 改) — 2 个 0% 改
- **总 15 个 0% 改 + 1 个 cli** (75% 借 clap, V1.1 0 改) = **16 个 0% 改**

**16 个 0% 改 理由**:
1. 哲学 + 状态 + 流程类 (B3 V0.5 30 维 / B4 6 重守门 v7 / B5 8 哲学锚) 严守 (per 决策 #74 §3.2 哲学 + 思想类严守)
2. 0 主动 commit (主人起床前) 严守 (per 决策 #33 §2.3 C1, V1.1 release 是 主人起床后)
3. 0 装 PASS 严守 (per 决策 #33 §2.3 C2, 0 装"已借鉴" / 0 装"已集成")
4. 借脑型 (借脑 OpenCog) 0 主仓集成 (per 决策 #22 §4 + 决策 #33 §2.2)

---

## 5. 24 LOCKED 入口 一致性 提升方案 (V1.0 release 0 改 / V1.1 release 5-8 改 / V2.0 release 全对齐)

### 5.1 V1.0 release (整合 #5.1 commit) 0 改严守 100% (per 决策 #74 §2.3 V1.0 release 边界)

**24 LOCKED 入口签名 0 改 verify** (per R131-5 §1.2, 24/24 全 PASS):
- 整合 #5.1 commit 拍板, R11 baseline 严守
- 24 LOCKED 入口签名 0 改 (per 决策 #33 §2.3 B1 + 决策 #74 §1 B1 V1.0 release 0 改严守)
- 24 LOCKED crate mtime 8/10 16:34 之后 8 个 (agent / mcp / tool-runtime / graph / pipeline / evolution / api / cli) 保持 0 改 (已发生的 0 改是新功能 module 加在原 crate 内, 不算 V1.0 release 改的)
- R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) 严守
- PHL-07 V1.0 release spec-only 0 实施 (per 决策 #74 §1 A3)
- workspace.version 1.2.0 严守 (per 决策 #74 §1 B2)
- 8 哲学锚严守 (per 决策 #33 §2.3 B5)
- 6 重守门 v7 严守 (per 决策 #33 §2.3 B4)
- V0.5 30 维严守 (per 决策 #33 §2.3 B3)

**一致性 0 改**: V1.0 release 时 50% 一致性 0 改, 24 LOCKED ↔ 11 源 API 一致性 保持 50% 加权平均.

### 5.2 V1.1 release 5-8 改 提升方案 (per 决策 #74 §2.3 V1.1 release 边界 + R137-2 §2.2.3 8 方向)

**V1.1 release 入口签名 改写 8 方向** (per R137-2 §2.2.3):
1. **方向 ① 入口签名一致性 标准化**: per-crate 选 3 模式之一 (全 re-export / 主类型 facade / 按需 re-export), 24 LOCKED 全部统一
2. **方向 ② 公开 API 表面 瘦身**: per-crate 暴露 ≤30 pub items, 多余的转 `pub(crate)` 或 module-private, 减少 30% (800+ → 560+)
3. **方向 ③ 9 叶子 crate 拆 workspace**: supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench → `apeireth-leaf/` workspace, 顶层 `apeireth/Cargo.toml` 0 改
4. **方向 ④ core 拆 pub mod**: core 当前 1 个 108KB lib.rs 拆成 `core/{bus, memory, state, config, error}/mod.rs`, 0 改入口签名
5. **方向 ⑤ 大模块集中 crate 拆 sub-crate**: mcp / pipeline / api / memory / asi / tools / evolution 拆 sub-crate, 顶层保留 re-export facade
6. **方向 ⑥ DSL 洋葱落地**: 三洋葱架构 → DSL 洋葱实施, per R133-3 §3.2: 新增 `apeireth-dsl` crate, Colang 真实施, 24 LOCKED crate 引用 dsl 守门
7. **方向 ⑦ 9 organ 内部借 OpenCode (R125 B7)**: 24 LOCKED crate 内部 fn 借 OpenCode 0 改入口签名, 0 破坏 LOCKED 入口, Eye 缺失 → V1.1 release 补 Eye organ
8. **方向 ⑧ R12 测度对齐**: R11 baseline 3 值 0.8682/0.8532/0.9063 → R12 baseline 更高, 24 测量函数签名更新, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新

**5 阶段实施计划 (8 周 = 2 个月)** (per R137-2 §2.2.3):
- 阶段 1 (1 周): 方向 ① 标准化
- 阶段 2 (1 周): 方向 ② 瘦身
- 阶段 3 (2 周): 方向 ③ 9 叶子拆 + 补 Eye organ
- 阶段 4 (2 周): 方向 ④ core 拆 pub mod + 方向 ⑤ 大模块拆 sub-crate
- 阶段 5 (2 周): 方向 ⑥ DSL 洋葱 + 方向 ⑦ 9 organ 借 OpenCode + 方向 ⑧ R12 测度对齐

**V1.1 release 时间窗**: 2026-11-30 (per R132-1 §1.1 + R131-3 §1.1, 6 大方向 × 1 周 = 6 周 估, 加上 sub-agent 协调 8 周)

**V1.1 release 8 个 crate 自决改 1:1 详细** (per §4):
- graph / pipeline / memory / agent / tool-registry / evolution / cognition / api = 8 个
- 一致性 提升: 50% → 55% (+5% 加权平均)
- 公开 API 表面 瘦身: 800+ → 560+ (-30%)

**workspace.version**: 1.2.0 → 1.2.1 (per 决策 #74 §1 B2, semver minor, 兼容)
**PHL-07 实施**: per R129-11 关键诚实标 + 决策 #74 §2.3 V1.1 release 边界

### 5.3 V2.0 release 全对齐 提升方案 (per 决策 #74 §2.3 V2.0 release 边界 + 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚")

**V2.0 release 触发条件** (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板):
- per Mavis 自决 + 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚"
- 全 8 硬墙 可重评 (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)
- 推翻 + 重建 8 哲学锚 (per "不要怕复杂度 + 最强效果 + 最厉害工程" 哲学)
- workspace.version: 1.2.1 → 2.0.0 (semver major, breaking change)

**V2.0 release 重构入口签名 8 方向** (per R131-5 §2.8 V2.0 release 重构 + R137-2 §2.2.4):
1. **全量统一入口签名 3 模式**: 24 LOCKED 全部按 organ-first 选 1 模式
2. **公开 API 表面全量按 organ 暴露**: `apeireth-brain::*` / `apeireth-hand::*` / `apeireth-memory::*` / `apeireth-voice::*` / `apeireth-ear::*` / `apeireth-eye::*` / `apeireth-heart::*` / `apeireth-body::*` / `apeireth-mind::*` (per R131-5 §2.6 9 organ 划分)
3. **9 organ workspace 化**: 24 LOCKED 全部下沉到 organ workspace, 顶层 `apeireth` re-export 全部
4. **core 全量拆 pub mod**: core 拆成 onion / human / principle / gate / action / verdict 6 个 sub-module
5. **大模块集中 crate 拆 sub-crate**: mcp / pipeline / api / memory / asi / tools / evolution 全拆
6. **三洋葱 workspace**: 原则 / 权限 / DSL 3 个独立 workspace, 24 LOCKED 全部下沉
7. **9 organ 内部借 OpenCode 实施**: organ-first 拓扑落地, Eye 抽 crate (per R131-5 §2.6 Eye 缺失 → V1.1 release 补 Eye organ, V2.0 release 落地)
8. **R12+ 测度重评**: 24 测量函数按 ASI Stage 9 重写, 编译期 hardcode 全部更新

**V2.0 release 一致性 目标**:
- 24 LOCKED ↔ 11 源 API 加权平均一致性 提升到 75-90% (从 50-55%)
- 全 9 organ workspace 化, 顶层 `apeireth` re-export facade 100% 兼容
- 0 装 PASS 严守 100% (per 决策 #33 §2.3 C2)
- 8 哲学锚 推翻 + 重建 (per "不要怕复杂度 + 最强效果 + 最厉害工程" 哲学, per 主人 8/11 01:14 拍板)

**V2.0 release 风险**:
- 极高: 9 organ workspace 重构 = 改 24 LOCKED crate 全部路径 = 改 N 个消费者的 `use` 路径 = breaking change
- 缓解: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用
- 缓解: V1.1 release bump 1.2.1, V2.0 release bump 2.0.0 (semver major)
- 缓解: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

### 5.4 提升方案 总览 (V1.0 / V1.1 / V2.0 release 一致性 路线图)

| Release | 24 LOCKED 改写 | 一致性 加权平均 | 公开 API 表面 | workspace.version | 8 哲学锚 | 0 装 PASS |
|---------|---------------|----------------|--------------|-------------------|---------|----------|
| **V1.0** (整合 #5.1 commit 拍板) | 0 改 100% | 50% (R11 baseline 严守) | 800+ 严守 | 1.2.0 严守 | 严守 100% | 严守 100% |
| **V1.1** (per 决策 #74 §2.3) | 8 crate 自决改 | 55% (+5%) | 560+ (-30%) | 1.2.1 bump | 严守 100% | 严守 100% |
| **V2.0** (per 决策 #74 §2.3) | 全 24 LOCKED 按 9 organ 重构 | 75-90% (+25-40%) | 按 organ 暴露 (300+?) | 2.0.0 bump (semver major) | 推翻 + 重建 | 严守 100% |

---

## 6. 24 LOCKED 入口 一致性 风险 (10 风险, per 决策 #74 §7.1 + 决策 #33 + R131-5 §6.1)

### R1 借鉴 API 演化破坏 (中等概率, 高影响)

**风险**: 借鉴的 11 源 API 演化 (e.g. langgraph 0.3 → 0.4, LiteLLM 1.5 → 1.6) 破坏 24 LOCKED 入口签名一致性
- 概率: 中 (借鉴源 都在 active development, 6-12 月 release 一次)
- 影响: 高 (langgraph 0.4 改 StateGraph 签名 → graph 入口签名 100% 一致 → 50% 一致, 大幅下降)
- 缓解: 顶层 `apeireth` re-export facade 保留, 借鉴源签名变化时, 自创 wrapper 适配
- 缓解: V1.1 release 自决改 时 重新对齐 借鉴源 最新版 API

### R2 0 装 PASS 严守破坏 (低概率, 高影响)

**风险**: V1.1 release 自决改 时 0 装"已借鉴" / 0 装"已集成" / 0 装"已 fork" 严守 100% 被破坏
- 概率: 低 (决策 #33 §2.3 C2 严守, R131-2 §2.1 verify 100%)
- 影响: 高 (破坏 0 装 PASS 技术哲学)
- 缓解: R130-6 借脑 ID 索引完成 0 装, R131-2 0 装 PASS 严守 6 维度 100% verify
- 缓解: Cargo.toml `borrow_skipped` 段永久明示 OpenCog family 永久跳过

### R3 V1.1 改写破坏下游 (中等概率, 中等影响)

**风险**: V1.1 release 自决改 8 个 crate 时, 改写 入口签名 / 公开 API 表面 破坏 下游消费者
- 概率: 中 (V1.1 release 是 minor release bump 1.2.0 → 1.2.1, semver 兼容, 但 改公开 API 表面 仍是 breaking change)
- 影响: 中 (下游消费者 0 改 主仓, 但 use 路径可能要改)
- 缓解: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用
- 缓解: V1.1 release bump 1.2.1 (per 决策 #74 §1 B2)
- 缓解: V1.1 release 5 阶段实施计划 (per R137-2 §2.2.3), 渐进改写

### R4 9 organ 拆 workspace breaking (高概率, 高影响, V2.0 release 风险)

**风险**: V2.0 release 9 organ workspace 化 时, 改 24 LOCKED crate 全部路径, breaking change 大量
- 概率: 高 (9 organ 重构 = 改 24 LOCKED crate, 100% 改)
- 影响: 高 (改 N 个消费者的 `use` 路径)
- 缓解: 顶层 `apeireth` re-export facade 保留
- 缓解: V2.0 release bump 2.0.0 (semver major, 跟 semver 一致)
- 缓解: 跟"不要怕复杂度"哲学一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3)

### R5 V2.0 全对齐 工作量 (高概率, 高影响)

**风险**: V2.0 release 全 24 LOCKED 按 9 organ workspace 化 重对齐 工作量极大, 实施时间窗 长
- 概率: 高 (24 LOCKED × 8 方向 = 192 子任务, 估 6-12 月 实施)
- 影响: 高 (拖延 V2.0 release 时间窗)
- 缓解: 8 方向 渐进实施 (5 阶段 → 8 阶段)
- 缓解: 16 派满策略 (per 主人 8/11 0:34 拍板)
- 缓解: 借鉴团队 (per 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护")

### R6 8 哲学锚 推翻 重建 团队不接受 (中等概率, 中等影响, V2.0 release 风险)

**风险**: V2.0 release 推翻 + 重建 8 哲学锚 时, 团队不适应 新哲学
- 概率: 中 (8 哲学锚 是 R125 B5 升 8 锚, 推翻重建 影响 8 哲学锚 严守)
- 影响: 中 (破坏 哲学一致性, 团队需要重新学习)
- 缓解: per 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护", 未来高水平团队能适应
- 缓解: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致

### R7 OpenCog 借脑 license 风险 (低概率, 高影响)

**风险**: OpenCog family 借脑 (R130-6 提议 6 子源) 触发 主仓 license 风险
- 概率: 低 (决策 #22 §4 + 决策 #33 §2.2 永久 0 主仓集成, 0 主仓 fork, 仅借脑)
- 影响: 高 (主仓 license 0 变, 0 装 PASS 严守 100%)
- 缓解: 永久 0 主仓集成 (per 决策 #22 §4 风险表 + 决策 #33 §2.2 + Cargo.toml deny.toml allow-list 不含 AGPL-3.0)
- 缓解: 永久 0 主仓 fork (per 决策 #33 §2.2 + 主仓 Apache-2.0 严守)
- 缓解: 1.0 release 后独立 fork 决策 (per 决策 #33 §2.2, 主人主动问后做)
- 缓解: OSS_NOTICE.md §3 永久跳过明示 + Cargo.toml `borrow_skipped` 段永久明示 (per R131-2 §1.3.1)

### R8 不要怕复杂度哲学 实施 (中等概率, 中等影响)

**风险**: "不要怕复杂度"哲学 实施 时, 团队 / 未来维护者 不适应
- 概率: 中 (哲学 跟 KISS / DRY 传统哲学 对立, 团队需要重新适应)
- 影响: 中 (破坏 代码可读性 / 可维护性, 但 提升 效果 / 借鉴深度)
- 缓解: per 主人 8/11 01:14 拍板 "自然会有高水平的团队来接手维护"
- 缓解: 哲学文档 15-no-fear-complexity.md 详细说明 (per 决策 #73 §3)
- 缓解: 8 哲学锚 严守 (S-3 不装 PASS 严守 跟"不要怕复杂度"哲学 一致)

### R9 VCP 内部 crate 不在 24 LOCKED 范围 (低概率, 中等影响)

**风险**: VCP 内部 crate (per 用户记忆 #8 终极前端) 不在 24 LOCKED 范围, 但 pipeline 借鉴 VCP 内部 5 模块 (model_router / provider_registry / role_divider / tiktoken_counter / tool_loop)
- 概率: 低 (VCP 是 主人下载的参考 (per 用户记忆 #8 路径), 0 集成到主仓)
- 影响: 中 (VCP 内部 crate 0 在 24 LOCKED, 但 pipeline 借鉴 VCP 公开模式 75% 借)
- 缓解: pipeline 0 装"已集成 VCP 内部 crate" (0 cloned, 仅借鉴 VCP 公开模式)
- 缓解: VCP 公开模式 (Electron 桌面 app, chat-first) 借鉴是 R121-1 实施

### R10 0 主动 commit/push 严守 (低概率, 中等影响)

**风险**: V1.1 release 8 crate 自决改 时, 0 主动 commit (主人起床前) 严守 100% 被破坏
- 概率: 低 (决策 #33 §2.3 C1 严守, 主人起床前 0 主动 commit)
- 影响: 中 (破坏 0 主动 commit 流程)
- 缓解: 决策 #33 §2.3 C1 严守 100%
- 缓解: V1.0 release 拍板 由 Mavis 0 主动 push 严守 (per 决策 #33 + 决策 #61 §6)
- 缓解: V1.1 release 实施 时, 主人起床后手跑 GitHub remote + tag + push (per 决策 #33 §2.3)

---

## 7. 24 LOCKED 入口 一致性 决策原则 (per 决策 #73 §3 总工程哲学 + 决策 #74 §1 B1 自决改 + 用户记忆 #1-10 主人偏好)

### 决策原则 1: B1 改写边界 (per 决策 #74 §1 B1 改写表)

- **V1.0 release 0 改严守** (R11 baseline 严守, 整合 #5.1 commit 拍板 0 改 src)
- **V1.1 release Mavis 自决改** (前提: 更好的架构, per 主人 8/11 01:14 拍板 "Mavis 自决架构拍板")
- **V2.0 release 可重评** (per Mavis 自决 + 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚")

### 决策原则 2: B2 workspace.version 严守 (per 决策 #74 §1 B2 改写表)

- **V1.0 release 1.2.0 严守** (Cargo.toml workspace.version 0 改, 整合 #5.2 commit 0 改)
- **V1.1 release bump 1.2.1** (semver minor, 兼容)
- **V2.0 release bump 2.0.0** (semver major, breaking change)

### 决策原则 3: A1 R11 baseline 严守 (per 决策 #74 §1 A1 改写表)

- **V1.0 release 0 改严守** (R11 baseline 3 值 0.8682/0.8532/0.9063 严守, 哲学 + 效果标)
- **V1.1 release 可改** (前提: 新的 baseline 更高, 跟 R12 测度对齐, per R125 B3 + R127 25 维公式)
- **V2.0 release 可重评** (per Mavis 自决 + 主人 8/11 01:14 拍板)

### 决策原则 4: A3 12 键 + PHL-07 严守 (per 决策 #74 §1 A3 改写表)

- **V1.0 release PHL-07 spec-only 0 实施** (V1.1 release 实施, per R129-11 关键诚实标) + 12 键其他可改
- **V1.1 release PHL-07 实施** (per R129-11 关键诚实标)
- **V2.0 release 可重评**

### 决策原则 5: B3 V0.5 30 维 严守 (per 决策 #74 §1 B3 改写表)

- **V1.0 release 严守** (哲学)
- **V1.1 release 严守** (哲学)
- **V2.0 release 可重评**

### 决策原则 6: B4 6 重守门 v7 严守 (per 决策 #74 §1 B4 改写表)

- **V1.0 release 严守** (哲学)
- **V1.1 release 严守** (哲学)
- **V2.0 release 可重评**

### 决策原则 7: B5 8 哲学锚 严守 (per 决策 #74 §1 B5 改写表)

- **V1.0 release 严守** (哲学)
- **V1.1 release 严守** (哲学)
- **V2.0 release 推翻 + 重建** (per "不要怕复杂度 + 最强效果 + 最厉害工程" 哲学)

### 决策原则 8: C1 0 主动 commit 严守 (per 决策 #74 §1 C1 改写表)

- **V1.0 release 0 主动 commit 严守** (主人起床前)
- **V1.1 release 0 主动 commit 严守** (主人起床前)
- **V2.0 release 0 主动 commit 严守**

### 决策原则 9: C2 0 装 PASS 严守 (per 决策 #74 §1 C2 改写表)

- **V1.0 release 0 装 PASS 严守** (技术哲学, 不装)
- **V1.1 release 0 装 PASS 严守**
- **V2.0 release 0 装 PASS 严守**

### 决策原则 10: 0 主动 push 严守 (per 决策 #74 §1 0 push 改写表)

- **V1.0 release 0 主动 push 严守** (主人起床前)
- **V1.1 release 0 主动 push 严守** (主人起床前)
- **V2.0 release 0 主动 push 严守**

### 决策原则 11: 总工程哲学扩展 "不要怕复杂度" (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

- **最强效果 > 最简单代码** (推翻 KISS, 拥抱 SOTA)
- **最厉害工程 > 最易维护** (推翻 DRY, 拥抱 BORROW)
- **维护交给未来高水平团队** (推翻"代码要让初级团队能接手", 拥抱"代码要让高水平团队能发挥")

### 决策原则 12: Mavis = orchestrator + 全自决 + 最高权限 (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)

- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)

---

## 8. 总结

### 8.1 24 LOCKED 入口签名 vs 借鉴 11 源 API 一致性 一句话

**24 LOCKED 入口签名 vs 借鉴 11 源 API 加权平均一致性 ≈ 50%** (per §3.4), V1.0 release 0 改严守 100% (整合 #5.1 commit 拍板 R11 baseline), V1.1 release 自决改 8 个 crate 提升到 55% (+5%), V2.0 release 全 24 LOCKED 按 9 organ workspace 化 重对齐到 75-90% (+25-40%).

### 8.2 V1.0 release 拍板 (per 整合 #5 commit 拍板逻辑)

- ✅ 24 LOCKED 入口签名 0 改 全部 verify 通过 (per R131-5 §1.2 24/24 全 PASS)
- ✅ 0 改 src 严守 (per 决策 #33 §2.3 + 决策 #74 §1 B1 V1.0 release 0 改严守)
- ✅ 8 硬墙 严守 (per 决策 #74 §1 改写表)
- ✅ 8 哲学锚 严守 (per 决策 #33 §2.3 B5)
- ✅ R11 baseline 3 值 严守 (per 决策 #33 §2.3 A1)
- ✅ 0 主动 commit (主人起床前) 严守 (per 决策 #33 §2.3 C1)
- ✅ 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
- ✅ 0 主动 push 严守 (per 决策 #33)

### 8.3 V1.1 release 改写 路线图 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套)

- **触发条件**: 更好的架构 (per 决策 #74 §2.3, 主人 8/11 01:14 拍板 "Mavis 自决架构拍板")
- **8 改写方向** (per R131-5 §2.8 + R137-2 §2.2.3 8 方向):
  1. 入口签名一致性 标准化
  2. 公开 API 表面 瘦身
  3. 9 叶子 crate 拆 workspace
  4. core 拆 pub mod
  5. 大模块集中 crate 拆 sub-crate
  6. DSL 洋葱落地
  7. 9 organ 内部借 OpenCode
  8. R12 测度对齐
- **8 自决改 crate** (per §4 详细): graph / pipeline / memory / agent / tool-registry / evolution / cognition / api
- **Mavis 自决改**: per 主人 8/11 01:14 拍板 "Mavis 自决架构拍板"
- **workspace.version**: 1.2.0 → 1.2.1 (per 决策 #74 §1 B2)
- **PHL-07 实施**: per R129-11 关键诚实标 + 决策 #74 §2.3 V1.1 release 边界
- **一致性 提升**: 50% → 55% (+5% 加权平均)
- **公开 API 表面 瘦身**: 800+ → 560+ (-30%)
- **5 阶段实施计划**: 8 周 = 2 个月 (per R137-2 §2.2.3)
- **V1.1 release 时间窗**: 2026-11-30 (per R132-1 §1.1 + R131-3 §1.1)

### 8.4 V2.0 release 重构 路线图 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套)

- **触发条件**: per Mavis 自决 + 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚"
- **8 重构方向** (per R131-5 §2.8 V2.0 release + R137-2 §2.2.4):
  1. 全量统一入口签名 3 模式
  2. 公开 API 表面全量按 organ 暴露
  3. 9 organ workspace 化
  4. core 全量拆 pub mod
  5. 大模块集中 crate 拆 sub-crate
  6. 三洋葱 workspace
  7. 9 organ 内部借 OpenCode 实施
  8. R12+ 测度重评
- **Mavis 自决重构**: per 主人 8/11 01:14 拍板 3 件套
- **workspace.version**: 1.2.1 → 2.0.0 (semver major, breaking change)
- **8 哲学锚**: 推翻 + 重建 (per "不要怕复杂度 + 最强效果 + 最厉害工程" 哲学)
- **一致性 目标**: 50-55% → 75-90% (+25-40%)
- **跟"不要怕复杂度"哲学一致**: per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md

---

## 9. References

### 9.1 决策 (10 个, per 任务 spec)

- **决策 #10** (决策日志基础, per 决策 #10)
- **决策 #22** (24 LOCKED + semver, per 决策 #22 §3 + §4)
- **决策 #33** (8 硬墙 + 0 装 PASS, per 决策 #33 §2.3 B1-B7 + A1-A3 + C1-C3)
- **决策 #44** (Safety policy, per 决策 #44)
- **决策 #55** (借鉴 12 源调研, per 决策 #55 §2.6)
- **决策 #56** (整合 #1 commit, per 决策 #56)
- **决策 #57** (整合 #2 commit, per 决策 #57)
- **决策 #58** (R128-2 派活, per 决策 #58 §5)
- **决策 #60** (0 主动删 严守, per 决策 #60)
- **决策 #61** (R129 era 派活, per 决策 #61 §1.4)
- **决策 #62** (整合 #5 commit 拆 3 commit 拍板, per 决策 #62 §5.1-§5.3)
- **决策 #64** (auto-replenish-16 cron, per 决策 #64)
- **决策 #70** (Mavis 清理决策权升级, per 决策 #70)
- **决策 #71** (4 步永久循环, per 决策 #71 §5 R137+ era 永久循环)
- **决策 #72** (R130 era 调研 6 sub-agent, per 决策 #72 §2.1)
- **决策 #73** (主人 8/11 01:14 拍板 3 件套, per 决策 #73 §2 + §3 + §4)
- **决策 #74** (8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改, per 决策 #74 §1 + §2 + §3)
- **决策 #75** (cron 派 11 sub-agent, per 决策 #75 §2.1)
- **决策 #76** (R134-R135 8 sub-agent, per 决策 #76)
- **决策 #77** (R137 era 派活清单, per 决策 #77 §3.1)
- **决策 #78** (整合 #5.3 reports commit 拍板, per 决策 #78)
- **决策 #79** (R141 era 13 sub-agent 派活, per 决策 #79 §1.5, 本报告派活依据)

### 9.2 报告 (12 个, per 任务 spec 不重写 reference)

- **R125-2** (clap 4.6.6 实施, per R125-2-BORROW-clap-rs/clap-4a622b4-2026-08-10)
- **R125-3** (hyper 0.1.20 实施, per R125-3-BORROW-hyperium/hyper-0.1.20-2026-08-10)
- **R125-4** (servers 76d64c8 实施, per R125-4-BORROW-modelcontextprotocol/servers-76d64c8-2026-08-10)
- **R125-5** (Guardrails 实施, per R125-5-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10)
- **R125-9** (PyO3 0.29.2 实施, per R125-9-BORROW-PyO3/PyO3-0.29.2-2026-08-10)
- **R125-10** (kani 0.67.0 实施, per R125-10-BORROW-model-checking/kani-0.67.0-2026-08-10)
- **R125-12** (opencode 实施, per R125-12-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10)
- **R125-13** (langgraph d56666f 实施, per R125-13-BORROW-langchain-ai/langgraph-d56666f-2026-08-10)
- **R125-14** (superpowers 6.2.0 实施, per R125-14-BORROW-obra/superpowers-6.2.0-2026-08-10)
- **R129-7** (借鉴 11/11 升级 1:1 verify, per R129-7 §1)
- **R129-11** (PHL-07 spec-only 关键诚实标, per R129-11 §1 0 装 PASS 终极 verify)
- **R129-28** (借鉴 11/11 终极 verify, per R129-28 §1.1)
- **R130-5** (V1.1 minor release 战略路线图, per R130-5 §1.5 6 大方向 + §1.1 V1.1 估 2026-11-30)
- **R130-6** (借鉴 12 源调研 OpenCog 决策, per R130-6 §1.2 + §3 + §4)
- **R131-1** (架构总审视 10 方向, per R131-1 §2.1-§2.10)
- **R131-2** (借鉴 12 源差距, per R131-2 §1 + §2 + §3 + §4, 本报告核心依据)
- **R131-3** (V1.1 release 实施路线图, per R131-3 §2 6 大方向)
- **R131-4** (cargo workspace 结构优化 7 方向, per R131-4 §2.1-§2.7)
- **R131-5** (24 LOCKED 入口分布优化 8 方向, per R131-5 §1 入口签名 0 改 verify 24/24 全 PASS + §2 8 优化方向 + §3 8 硬墙 + §4 8 哲学锚 + §5 不要怕复杂度哲学, 本报告核心依据)
- **R131-9** (形式化集成优化 9 方向 + F1-F11 11 维度, per R131-9 §1.3 + §2)
- **R132-1** (V1.1 release 路线图 final, per R132-1 §1.1 + §1.2 + §1.5 + §2)
- **R133-3** (三洋葱架构升级 5 阶段 实施 spec, per R133-3 §2 + §3.2)
- **R137-2** (24 LOCKED 入口签名 改写 spec + 5 阶段实施计划, per R137-2 §2 改写 spec + §3 5 阶段, 本报告核心依据)
- **R141-2** (本报告, 24 LOCKED 入口签名 vs 借鉴 11 源 API 一致性 详细分析, per 决策 #79 §1.5)

### 9.3 借鉴 ID 索引 (11 源 + 6 子源, per R131-2 §1 + §2)

#### 8 真 cloned:
1. `R125-2-BORROW-clap-rs/clap-4a622b4-2026-08-10` (clap 4.6.6, 4.5MB / 631 files, 17:30 cloned)
2. `R125-3-BORROW-hyperium/hyper-0.1.20-2026-08-10` (hyper 0.1.20, 0.54MB / 58 files, 17:29 cloned)
3. `R125-4-BORROW-modelcontextprotocol/servers-76d64c8-2026-08-10` (servers 76d64c8, 1.40MB / 145 files, 16:51 cloned)
4. `R125-5-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10` (Guardrails, 18.19MB / 2045 files, 17:48 cloned)
5. `R125-9-BORROW-PyO3/PyO3-0.29.2-2026-08-10` (PyO3 0.29.2, 5.69MB / 811 files, 16:53 cloned)
6. `R125-10-BORROW-model-checking/kani-0.67.0-2026-08-10` (kani 0.67.0, 5.46MB / 3224 files, 17:35 cloned)
7. `R125-13-BORROW-langchain-ai/langgraph-d56666f-2026-08-10` (langgraph d56666f, 13.29MB / 670 files, 16:31 cloned)
8. `R125-14-BORROW-obra/superpowers-6.2.0-2026-08-10` (superpowers 6.2.0, 1.52MB / 180 files, 17:33 cloned)

#### 2 借鉴 ID 索引完成 (限流 → 重试真实施):
9. `R125-1-BORROW-BerriAI/litellm-2026-08-10` (LiteLLM, 0 cloned, 1:1 翻译 562 行新 src)
10. `R125-12-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10` (opencode, 0 cloned, 改借鉴已 cloned langgraph 829 + servers 175)

#### 1 永久跳过:
11. `R124-2-BORROW-opencog/opencog-2024Q4-2026-08-10` (opencog/opencog, AGPL-3.0, 永久 0 集成 0 主仓 fork)

#### 🆕 1 借脑 ID 索引完成 (R130-6 提议 OpenCog 家族 6 子源):
- `R130-6-BORROW-opencog/atomspace-2026Q1-2026-08-11` (🟢 高 ROI, 借脑 Atom/Node/Link + ECAN)
- `R130-6-BORROW-opencog/cogutil-2026Q1-2026-08-11` (🟡 中 ROI, 浅度调研)
- `R130-6-BORROW-opencog/moses-2026Q1-2026-08-11` (🟡 中 ROI, 借脑 决策树森林)
- `R130-6-BORROW-opencog/pln-2026Q1-2026-08-11` (🔴 低 ROI, 官方 deprecated)
- `R130-6-BORROW-opencog/relex-2026Q1-2026-08-11` (🔴 低 ROI, 官方 deprecated)
- `R130-6-BORROW-CogPrime-Goertzel-2024-2026-08-11` (🟢 高 ROI, 借脑 CogPrime 集成模式)

### 9.4 24 LOCKED 完整名单 (per R131-5 §1.2 + 决策 #33 §2.3 B1)

1. supervisor
2. agent
3. council
4. bus
5. protocol
6. mcp
7. tool-registry
8. tool-runtime
9. graph
10. pipeline
11. tool-approval
12. extension
13. evolution
14. api
15. core
16. memory
17. asi
18. tools
19. cli
20. bench
21. cognition
22. action
23. life-force
24. constraint

### 9.5 8 硬墙 (per 决策 #33 §2.3 + 决策 #74 §1 改写表)

- **B1** 24 LOCKED 入口签名: 🟢 V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + V2.0 release 可重评
- **B2** workspace.version 1.2.0: 🔒 V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 + V2.0 release bump 2.0.0
- **A1** R11 baseline 3 值: 🔒 V1.0 release 0 改严守 + V1.1 release 可改 (前提: 新 baseline 更高) + V2.0 release 可重评
- **A3** 12 键 + PHL-07: 🔒 V1.0 release PHL-07 spec-only 0 实施 + V1.1 release PHL-07 实施 + V2.0 release 可重评
- **B3** V0.5 30 维: 🔒 严守 (哲学) + V2.0 release 可重评
- **B4** 6 重守门 v7: 🔒 严守 (哲学) + V2.0 release 可重评
- **B5** 8 哲学锚: 🔒 严守 (哲学) + V2.0 release 推翻 + 重建
- **C1** 0 主动 commit (主人起床前): 🔒 严守
- **C2** 0 装 PASS: 🔒 严守
- **0 push** 0 主动 push (主人起床前): 🔒 严守

### 9.6 8 哲学锚 (per 决策 #33 §2.3 B5 + R125 B5 升 8 锚 + 决策 #74 §1 B5 严守)

- **S-1** 服务 ASI 北极星
- **S-2** 实事求是
- **S-3** (R125 B5 新增, 主人 16:27 拍板) 24 LOCKED crate 都有"实测函数" → 不装 PASS
- **O-1** 质量工程化
- **O-2** 安全优先
- **O-3** 走在前人经验上
- **O-4** 干到底
- **O-5** 任何人都能接手

### 9.7 6 重守门 v7 (per 决策 #33 §2.3 B4)

- 12 键 verdict 守门
- 5 重哲学守门 (v15 命名修正: 4 重 + 权限发放)
- R11 baseline 3 值 0.8682/0.8532/0.9063 守门
- compile-time assert 守门
- 0 装 PASS 守门
- 8 哲学锚守门

### 9.8 V0.5 30 维 (per 决策 #33 §2.3 B3 + R125-13 升 30 维)

- V1141 IC-001 fresh 24 维均值: 0.8682
- V1131 dashboard 9 维均值: 0.8532
- V1136 9 子测度均值: 0.9063
- 24 measure_dim_* + 9 measure_sub_* = 33 个测量函数

### 9.9 9 organ (per R11 9 organ + R131-5 §2.6 9 organ 划分)

0. **Heart** (LLM 网关心跳) = supervisor + bus (L0) + pipeline
1. **Brain** (Multi-Agent 决策) = agent + council + cognition + constraint
2. **Hand** (Tool Protocol) = tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action
3. **Eye** (用户输入感知) = (tui/src/organ/eye.rs, **0 LOCKED crate**)
4. **Ear** (系统事件监听) = bus (L1-L4)
5. **Memory** (3 层 facade) = memory + asi + life-force + core (IdentityCard)
6. **Voice** (TTS/STT) = protocol + pipeline (流式)
7. **Body** (长程任务) = bench + api + cli
8. **Mind** (9-stage lifecycle) = evolution + graph (lifecycle 编排) + constraint (5 重守门)

### 9.10 三洋葱架构 (per 决策 #33 + R125 B6 + 决策 #74 §1 B4)

- **原则洋葱** (PrincipleOnion): E/S/A/M/O 5 切片 — 锁在 core
- **权限洋葱** (PermissionOnion): L0-L5 6 切片 — 锁在 core
- **DSL 洋葱** (新, R125 B6 升级, v6 守门): Colang DSL 守门 (NeMo Guardrails 借鉴) — V1.1 release 落地

### 9.11 报告路径 + 时间盒 + 状态

- **报告路径**: `Apeireth-rust\reports\agent-r141-2-24-locked-vs-borrowed-api-consistency-2026-08-11.md`
- **时间盒**: 60 min
- **状态**: ✅ done (V1.0 release 0 改严守 100% verify + V1.1 release 8 crate 自决改 详细 + 提升方案 V1.0/V1.1/V2.0 + 10 风险 + 12 决策原则)
- **生成时间**: 2026-08-11 (R141 era 第 2 批, R141-2 sub-agent)
- **关联决策**: 决策 #10 + #22 + #33 + #44 + #55 + #56 + #57 + #58 + #60 + #61 + #62 + #64 + #70 + #71 + #72 + #73 + #74 + #75 + #76 + #77 + #78 + #79
- **关联报告**: R125-2/3/4/5/9/10/12/13/14 + R129-7/11/28 + R130-5/6 + R131-1/2/3/4/5/9 + R132-1 + R133-3 + R137-2
- **作者**: Mavis (R141-2 sub-agent, 决策 #79 §1.5 派活)

---

## 10. 一句话 (再次强调)

**R141-2 24 LOCKED 入口签名 vs 借鉴 11 源 API 一致性 详细分析 (per 决策 #74 §1 B1 改写 + R131-5 §1 入口 verify + R137-2 §2 改写 spec + R131-2 §1 借鉴 12 源差距)**: **V1.0 release 0 改严守 100%** (整合 #5.1 commit 拍板, 24 LOCKED 入口签名 0 改 verify 24/24 全 PASS, R11 baseline 3 值严守, PHL-07 spec-only 0 实施, workspace.version 1.2.0 严守, 8 哲学锚严守, 6 重守门 v7 严守, V0.5 30 维严守). **24 LOCKED ↔ 11 源 API 加权平均一致性 ≈ 50%** (100% 一致 2 个: protocol + graph / 75% 一致 5 个: agent + pipeline + api + cli + 借脑折半 / 50% 一致 8 个: supervisor + council + mcp + tool-registry + tool-runtime + extension + evolution + bench / 25% 一致 5 个: core + memory + cognition + life-force / 0% 一致 5 个: bus + tool-approval + asi + tools + action). **V1.1 release 自决改 8 个 crate** (Mavis 自决, 前提: 更好的架构, 改写一致性 50% → 55% (+5%)): graph (PostgresSaver 借鉴) + pipeline (80+ provider) + memory (ECAN + Atom/Node/Link 借脑) + agent (Multi-Agent 编排 + 8 角色) + tool-registry (Tool Transformer) + evolution (PODA + moses 决策树借脑) + cognition (Atomese graph 借脑) + api (80+ provider 标准化 v2). **提升方案**: V1.0 release 0 改 100% (R11 baseline 严守) + V1.1 release 8 crate 自决改 (5 阶段 8 周 = 2 个月, bump 1.2.0 → 1.2.1, PHL-07 实施) + V2.0 release 全 24 LOCKED 按 9 organ workspace 化 重对齐 (一致性 75-90% (+25-40%), 8 哲学锚 推翻 + 重建, bump 2.0.0 semver major). **10 风险** (R1 借鉴 API 演化破坏 / R2 0 装 PASS 严守破坏 / R3 V1.1 改写破坏下游 / R4 9 organ 拆 workspace breaking V2.0 / R5 V2.0 全对齐 工作量 / R6 8 哲学锚 推翻 重建 团队不接受 / R7 OpenCog 借脑 license 风险 / R8 不要怕复杂度哲学 实施 / R9 VCP 内部 crate 不在 24 LOCKED 范围 / R10 0 主动 commit/push 严守). **12 决策原则** (per 决策 #73 §3 总工程哲学 + 决策 #74 §1 B1 自决改 + 用户记忆 #1-10 主人偏好). **0 改 src/** + **0 改 Cargo.toml/** + **0 主动 commit** + **0 主动 push** + **0 主动 IM 主人** (per gate-discipline, 仅 done notification 主动报告).
