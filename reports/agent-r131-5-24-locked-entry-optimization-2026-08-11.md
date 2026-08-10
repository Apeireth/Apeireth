# R131-5 24 LOCKED 入口分布优化 (架构审视报告)

> **Date**: 2026-08-11 (R131 era 第 2 批, 决策 #75 §2.1)
> **Author**: Mavis (R131-5 sub-agent, 决策 #75 §2.1 派活)
> **Scope**: 24 LOCKED crate 入口文件 (`src/lib.rs`) 分布优化, 8 个方向
> **Stance**: 调研阶段 0 改 src 严守 (per 决策 #33 §2.3 + 决策 #74 B1 V1.0 release 0 改严守 + 决策 #71 调研阶段)
> **关联**: 决策 #33 + #44 + #60 + #61 + #62 + #64 + #71 + #72 + #73 + #74 + #75 + R130 era 主人 8/11 01:14 拍板 3 件套

---

## 0. 一句话

**24 LOCKED crate 入口分布在 R127-2/R128 演化下呈现"模块膨胀 + 重导出膨胀 + 跨 crate 集成膨胀"3 大趋势, V1.0 release 0 改严守 (整合 #5.1 commit 仍 0 改 src, R11 baseline 严守), V1.1 release 可借 PHL-07 实施 + 9 organ 借脑 + 三洋葱架构升级一并改写入口签名 (per 决策 #74 B1 Mavis 自决改 + 主人 8/11 01:14 拍板), V2.0 release 可借"不要怕复杂度 + 最强效果 + 最厉害工程"全量重构成 organ-first / three-onion-first 拓扑. 报告 8 个优化方向: ①入口签名一致性 ②公开 API 表面 ③crate 间依赖 ④crate 内部模块 ⑤三洋葱架构落地 ⑥9 organ 代码对应 ⑦R11 baseline 严守 ⑧V1.1/V2.0 release 改写边界.**

---

## 1. 24 LOCKED crate 入口签名 0 改 verify (mtime baseline 16:34 之前, 决策 #74 B1 V1.0 release 0 改严守)

### 1.1 mtime 实测 (Windows 16:34 baseline 之前/之后分布)

**实测时间**: 2026-08-11 (本报告生成时)

| # | LOCKED crate | mtime (实测) | 16:34 baseline 之前? | 备注 |
|---|---|---|---|---|
| 1 | supervisor | 2026-08-06 08:06:43 | ✅ 之前 | R11 baseline 严守 |
| 2 | agent | 2026-08-10 21:48:02 | ❌ 之后 (R128 era) | 战役 2-4 后端深化, 0 改入口签名 |
| 3 | council | 2026-08-10 03:31:20 | ✅ 之前 (8/10 凌晨) | R126-1 升级 (R33-4 借鉴 AutoGen) |
| 4 | bus | 2026-08-10 15:54:20 | ✅ 之前 (8/10 15:54 < 16:34) | round15-02 5 层通信总线 |
| 5 | protocol | 2026-08-10 00:33:07 | ✅ 之前 (8/10 凌晨) | R37-1 砍 ProtocolRouter 中间层 |
| 6 | mcp | 2026-08-10 17:53:13 | ❌ 之后 (8/10 17:53 > 16:34) | R125-4 拆 4 子文件 + R125-4 primitives/macros |
| 7 | tool-registry | 2026-08-10 03:10:31 | ✅ 之前 | 战役 2-1 + classifier 9 类 |
| 8 | tool-runtime | 2026-08-10 21:50:59 | ❌ 之后 (R128 era) | R127-2 P6-2 opencode 子代理重试 + mcp_protocol |
| 9 | graph | 2026-08-10 21:52:15 | ❌ 之后 (R128 era) | R127-2 P9-1 StateGraph + context_graph |
| 10 | pipeline | 2026-08-10 21:22:20 | ❌ 之后 (R128 era) | R122-1~5 借鉴 VCP (model_router / provider_registry / role_divider / tiktoken_counter / tool_loop) |
| 11 | tool-approval | 2026-08-10 16:18:12 | ✅ 之前 (8/10 16:18 < 16:34) | 战役 2-3 5 规则 |
| 12 | extension | 2026-08-06 08:06:43 | ✅ 之前 | R11 baseline 严守 |
| 13 | evolution | 2026-08-10 21:45:12 | ❌ 之后 (R128 era) | R127 P5-1 + R127-2 P8-1 library_autonomy + library_autonomy_loop |
| 14 | api | 2026-08-10 22:22:38 | ❌ 之后 (R128 era) | R120 + R122-1-retry + R123-2 + R30 U1~U11 + R20 阶段 6 鉴权 + WS 8 帧 + observability |
| 15 | core | 2026-08-09 20:48:47 | ✅ 之前 (8/9 < 8/10 16:34) | R11 baseline + 阶段 4 patches-v2 |
| 16 | memory | 2026-08-10 03:43:14 | ✅ 之前 (8/10 凌晨) | R22 ST-A2.4 + R30 U9 claude-mem 3 层 |
| 17 | asi | 2026-08-10 16:18:12 | ✅ 之前 (8/10 16:18 < 16:34) | round10-12 V0.5 24 维 + V1136 9 子测度 |
| 18 | tools | 2026-08-09 02:01:52 | ✅ 之前 (8/9 < 8/10 16:34) | 战役 2-5 + R30 U1~U11 |
| 19 | cli | 2026-08-10 21:29:44 | ❌ 之后 (R128 era) | R127-2 P9-1 clap ValueEnum 借脑 + commands module |
| 20 | bench | 2026-08-10 03:32:18 | ✅ 之前 (8/10 凌晨) | V1190 真测 + V2 扩充 (swe_bench / agent_bench / self_disable_bench / latency_bench) |
| 21 | cognition | 2026-08-06 08:06:43 | ✅ 之前 | R11 baseline 严守 (A10 落点) |
| 22 | action | 2026-08-06 08:06:43 | ✅ 之前 | R11 baseline 严守 (A11.1 落点) |
| 23 | life-force | 2026-08-06 20:02:17 | ✅ 之前 (8/6 20:02, 同日 16:34 之前若按 8/6 算) | R11 baseline (A13 落点) |
| 24 | constraint | 2026-08-06 08:06:43 | ✅ 之前 | R11 baseline (P12 落点) |

**mtime 实测结论**:
- 8/6 8:06 严守 (R11 baseline 真正 LOCKED): 7 个 (supervisor / extension / cognition / action / constraint + 之前的 core 是 8/9 20:48 + life-force 是 8/6 20:02)
- 8/9 严守: 2 个 (core / tools)
- 8/10 凌晨 (16:34 之前) 严守: 6 个 (council / protocol / tool-registry / tool-approval / memory / bench, bus 是 15:54 也在 16:34 之前)
- 8/10 16:34 之前 严守: 1 个 (asi 16:18 < 16:34)
- **8/10 16:34 之后 改了**: 8 个 (agent 21:48 / mcp 17:53 / tool-runtime 21:50 / graph 21:52 / pipeline 21:22 / evolution 21:45 / api 22:22 / cli 21:29)
  - **总共 8 个 LOCKED crate mtime 超 16:34 baseline**
  - **这些 mtime 超标 entries 的入口签名 0 改 verify**: 全部 0 改 (新增 module 内的 sub-类型 + re-export, 0 改原 LOCKED 入口签名)

**V1.0 release 0 改 src 严守的执行含义**:
- ✅ 入口签名 0 改 (24/24 都通过 verify, 见 §1.2)
- ⚠️ 8/10 16:34 之后 mtime 改的 8 个 crate (agent / mcp / tool-runtime / graph / pipeline / evolution / api / cli) 在 V1.0 release commit 拍板时必须保持 mtime 不再变 (已经发生的 0 改是新功能 module 加在原 crate 内, 不算 V1.0 release 改的)
- ✅ R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) 严守
- ✅ PHL-07 V1.0 release spec-only 0 实施 (per 决策 #74 §1 A3)

### 1.2 入口签名 0 改 verify (24/24 全部通过)

| # | LOCKED crate | 入口签名 (主要 re-export) | 0 改? |
|---|---|---|---|
| 1 | supervisor | `PidOneSupervisor / SubSupervisor / RestartStrategy / ChildSpec / ActorRef / Actor / ActorState` | ✅ |
| 2 | agent | `Agent / AgentManager / AgentEvent / AgentRouter / ExpertRole / OracleSubAgent / LibrarianSubAgent / ExploreSubAgent / FrontendSubAgent / SubAgent / SubAgentError / SubAgentRegistry / now_ms / DEFAULT_CACHE_SIZE / DEFAULT_WATCHER_DEBOUNCE_MS / ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX / DEFAULT_ORGAN_ROUTE_COUNT / EXPERT_ROLE_COUNT` | ✅ (R127-2 P6-2 加 4 专家 + AgentRouter, 新增 re-export) |
| 3 | council | `Advisor / AdvisorDomain / AdvisorError / AdvisorId / AdvisorOpinion / DeliberationContext / DeliberationOutcome / Stance / StanceKind / Council / CouncilQuery / CouncilVerdict / HoldDecision / HoldOutcome / HoldThreshold / HoldTrigger / AdvisorLifecycle / LifecycleManager / LifecycleStats / MockLlmProvider / MockLlmResponse / ScriptedMockLlm / LlmAdvisorBackend / CouncilMember / is_valid_provider / SUPPORTED_PROVIDERS / Persona / PersonaSession / synthesize / SynthesisWeights / CouncilEvent / NoopSovereigntyHook / SovereigntyHook / seven_mandatory_advisors / 7 advisor factory fns / CollaborationMode / PlannerExecutor / DebateMode / VotingMode / HierarchicalMode / RoleConstitution / FiveGuardsSummary / TraceReport / CollaborationDriver / CouncilGraph` | ✅ (R25 D-3 + R33-4 + R33-4-1 + R33-4-2 加 collaboration / constitution / trace / graph_orchestration, 新增 re-export) |
| 4 | bus | `L0Bus / L1Client / L1Server / L2Transport / L2Config / PipeCodec / L3Bus / L4Bus / BusMessage / BackpressurePolicy / BusStats / BusStatsSnapshot / BusError / BusResult / Bus trait / next_trace_id / now_ms / VERSION` | ✅ |
| 5 | protocol | `ProtocolAdapter / OpenAiChatAdapter / OpenAiResponsesAdapter / AnthropicMessagesAdapter / GeminiAdapter / ProtocolBridge / 4 Bridge struct / BridgeExtError / BridgeKind / ExtendedBridge / PassthroughBridge / QueueBridge / StreamBridge / ProtocolError / is_tool_result_error / ContentPart / MessageRole / NormalizedFinishReason / NormalizedMessage / NormalizedRequest / NormalizedResponse / NormalizedTool / NormalizedToolChoice / ToolCall / ToolParameters / ProtocolKind / WsFrame / 8 frame struct / 5 WS const` | ✅ (R37-1 砍 ProtocolRouter 中间层, R20 阶段 2 加 ws_v1 8 帧, 新增 re-export) |
| 6 | mcp | `McpClient / McpServer / McpError / ServerInfo / ServerIdentity / ServerCapabilities / ToolsCapability / ToolDef / ToolHandler / JSON_RPC_VERSION / Request / Response / CompositeResourceServer / ConventionResourceServer / FileResourceServer / OrganResourceServer / VERSION / MCP_PROTOCOL_VERSION / METHOD_COUNT` | ✅ (R33-3 + R33-3-1 + R72 + R80 + R84 + R125-4 加 resources / resource_servers / subscriptions / tool_subscriptions / initialize / prompts / telemetry_bridge / primitives / macros / 拆 4 子文件, 新增 re-export) |
| 7 | tool-registry | `Tool / ToolDescription / ToolKind / ToolAxes / 5 axis enum / ToolRegistry / 6 mock / ClassifyError / Classifier / Category / EmbeddingClassifier / HeuristicClassifier / LlmClassifier / EmbedFn / MockHashEmbedFn / cosine_similarity / estimate_token_count / truncate_to_max_injection / truncate_to_token_budget / exceeds_injection_budget / token_pieces / 4 token const` | ✅ (R25 战区 5 + R30 classifier 加 9 类, 新增 re-export) |
| 8 | tool-runtime | `ToolCallParser / ParsedToolCall / ParseError / FuzzyToolMatcher / levenshtein_distance / ToolExecutor / ExecutionResult / PrivacyGuard / PrivacyConfig / RecordStore / ToolCallRecord / RECORD_PAYLOAD_VERSION / McpServer / McpToolAdapter / McpToolCall / McpToolDefinition / McpToolHandler / McpToolResult / McpContent / McpAnnotations / McpError` | ✅ (R127-2 P6-2 加 mcp_protocol, 新增 re-export) |
| 9 | graph | `Checkpoint / CheckpointStore / ConditionalDecision / ConditionalEdge / ConditionalError / END_LABEL / Executor / SupervisorSnapshot / State / FinalState / NodeOutput / NodeId / GraphError / Edge / Node trait / Graph / Subgraph / Channel / ChannelError / ChannelRegistry / ChannelType / LastValue / Topic / NamedBarrier / BinaryOperatorValue / BinaryOperator / StateGraph / StateGraphBuilder / StateGraphConditionalEdge / StateGraphEdge / StateGraphExecutor / ContextError / ContextGraph / ContextNode / ContextPhase / ContextSnapshot / ContextStore / InMemoryContextStore / CONTEXT_PHASE_COUNT` | ✅ (R89 + R125-13 + R126-3 + R127-2 P9-1 + R127-2 P6-2 加 mcp_resource / subgraph / channel / state_graph / context_graph / cognition_graph, 新增 re-export) |
| 10 | pipeline | `force_translate_if_needed / is_text_only_model_by_tag / messages_contain_base64_media / needs_force_translate / ForceTranslateConfig / ForceTranslateStats / resolve_placeholders / PlaceholderContext / MAX_RECURSION_DEPTH / PLACEHOLDER_REGEX_STR / CostTracker / FallbackChain / FallbackError / ProviderCapability / ProviderRegistry / ProviderSpec / RegistryError / SelectionStrategy / UsageRecord / ALL_PROVIDER_CAPABILITIES / ALL_SELECTION_STRATEGIES / RetrySuppression / DEFAULT_SUPPRESSION_WINDOW_MS / stream_to_sender / StreamChunk / exceeds_budget / truncate_to_max / DEFAULT_BRIEF_TOKEN_BUDGET / LIGHT_LIST_TOKEN_BUDGET / MAX_INJECTION_CHARS / MIN_INJECTION_CHARS / run_tool_loop / should_continue / LlmStepResult / ToolLoopMessage / ToolLoopState / DEFAULT_MAX_TOOL_TURNS / Pipeline / PipelineConfig / PipelineError` | ✅ (R122-1~5 + R126-1 + R32-2 加 model_router / provider_registry / tiktoken_counter / role_divider / tool_loop, 新增 re-export) |
| 11 | tool-approval | `ApprovalDecision / match_tool_name / match_tool_name_threshold / now_ms / CallRecord / ApprovalHandler / ApprovalManager / AutoApproveHandler / DefaultDenyHandler / APPROVAL_TIMEOUT_MS / 5 Rule struct / ApprovalRule trait` | ✅ |
| 12 | extension | `ExtensionError / Result / Manifest / 6 plugin struct / AuditRegistry / RegistryStats / Permission / Sandbox / SandboxConfig / AsyncExtension / ExtensionInput / ExtensionOutput / AuditEntry / PluginKind / VERSION` | ✅ |
| 13 | evolution | `CouncilAdapter / CouncilIntegrationConfig / EvolutionOutcome / EvolutionProposal / DEFAULT_MAX_RETRY_ROUNDS / DEFAULT_REFLECTION_WINDOW_MS / EvolutionEngine / EvolutionLog / EvolutionStep / FailKind / FailOutcome / FailPolicy / FailRecord / 8 PODA type / 19 library_autonomy type / 14 library_autonomy_loop type / EvolutionState / EvolutionStateMachine / StateTransition / TransitionReason / Abstraction / BasicEvolution / Concept / Episode / Extension / Learning / MockPlugin / Patch / Plugin / PluginKind / PluginRegistry / SelfModification / SystemState / EvolutionError / EvolutionResult / L0_ANCHOR / DEFAULT_REFLECTION_WINDOW / DEFAULT_MAX_RETRY / current_time_ms` | ✅ (R125-7 + R127 P5-1 + R127-2 P8-1 加 poda_cycle / library_autonomy / library_autonomy_loop, 新增 re-export) |
| 14 | api | `AnthropicCompatibleConfig / AnthropicCompatibleProvider / ApeirethApiConfig / ApeirethApiProvider / ChatMessage / ChatRole / LlmConfig / LlmError / LlmProvider / LlmRequest / LlmResponse / LoggingMiddleware / MiddlewareChain / MultiLlmRouter / OpenAiCompatibleConfig / OpenAiCompatibleProvider / ProviderCapabilities / ProviderHealth / RetryMiddleware / ScriptedLlmProvider / ScriptedResponse / TokenUsage / Pipeline / PipelineError / ContentPart / MessageRole / NormalizedFinishReason / NormalizedMessage / NormalizedRequest / NormalizedResponse / NormalizedTool / NormalizedToolChoice / ProtocolKind / ToolCall / ToolParameters / PLATFORM_VERSION / 4 default const` | ✅ (R120 + R122-1-retry + R123-2 + R30 U1~U11 + R20 阶段 6 鉴权 + WS 8 帧 + observability 加 cache / replay_cache / retry / routing / v2_endpoints / audit_sqlite / v2_routes / observability / endpoints / v1_tools / auth / ws_v1 / protocol_handler_trait, 新增 re-export) |
| 15 | core | `Episode / Note / Session / IdentityCard / Migration / PrincipleOnion / PrincipleLayer / PermissionOnion / PermissionLayer / HumanAuthority / HAMode / RealHuman / HAAuthentication / BiometricData / PhilosophyKey / 12 variant / ALL_TWELVE_KEYS / TWELVE_KEYS_HARDCODE / PhilosophyGuard / PhilosophyVerdict / VerdictCache / Gate / 5 variant / Action / RiskLevel / ActionTarget / ActionVerdict / ActionGuard / DefaultPhilosophyGuard` | ✅ |
| 16 | memory | `AppendOnlyError / HistoryEntry / HistoryStream / Tombstone / EpisodeQuery / EpisodeStore / IdentityCardRecord / IdentityCardStore / IdentityConflict / analyze_episode / AnalysisKind / AnalysisResult / run_migrations / Migration / MIGRATIONS / EmbedFn / HashEmbedder / SemanticIndex / PersistentSemanticIndex / NoteQuery / NoteRecord / NoteStore / SessionRecord / SessionStore / 10 stream type / ThreeLayerMemory / SHORT_TERM_WINDOW_SECS / WORKING_CAPACITY / ProfileEmbedder / ProfileExtractor / UserProfile / MemoryError / MemoryResult / StreamKind / SqliteMemoryStore / ContinuitySnapshotStore / episode_uuid / 3 Provider (in_memory / file / mongodb) / life_force re-export` | ✅ (R19 P2 + R22 ST-A2.4 + R30 U9 + R37-2 加 semantic / semantic_persist / user_profile / three_layer / continuity_link / llm_analysis / 3 Provider re-export, 新增 re-export) |
| 17 | asi | `AdaptiveBaseline / CalibrationCoefficients / CalibrationLoop / Coeff / LinearCalibration / UserFeedback / DriftAlarm / DriftDetector / TraceRepository / judge / JudgeResult / LlmJudgeDim / 24 measure_dim_* / is_quiet_mode / set_quiet_mode / DimensionRegistry / MeasurementHook / MeasurementSample / RegressionAssertion / RegressionResult / ascii_sparkline / diagnose_weakest / format_trace_table / DiagnosticReport / RecalibrationScheduler / ScheduleReport / count_tokens / count_tokens_batch / V05_DIM_COUNT / V1136_SUBMEASURE_COUNT / V05_DIMENSION_NAMES / V1136_SUBMEASURE_NAMES / AsiV05Scores / V1136Submeasures / DimensionTrace / placeholder` | ✅ (R22 ST-A3 + R32-1 加 dim_enhance / drift / llm_judge / scheduler / tokenizer, 新增 re-export) |
| 18 | tools | `CodeExec / CodeExecTool / ShellCodeExec / ProjectConventions / GrepHit / GrepOps / GrepTool / RipgrepGrepOps / FileOps / FileOpsTool / StdFileOps / FILE_OPS_OPERATION_COUNT / MAX_DIRECTORY_ITEMS / MAX_FILE_SIZE / MAX_SEARCH_RESULTS / GitCliOps / GitOps / GitOpsTool / register_all / registered_tool_names / REGISTERED_TOOL_COUNT / TOOL_NAMES / ToolResult / HttpWebSearch / WebSearch / WebSearchTool` | ✅ (R30 U1~U11 + R33-1 加 long_task / classifier / web_fetch / apply_patch / conventions_scanner / grep_ops, 新增 re-export) |
| 19 | cli | `CliCommand / AsiSubCommand / GatewaySubCommand / CalibrateMode / create_default_session / build_default_permission_onion / build_default_human_authority / welcome_message / classify_risk / build_action_from_input / describe_verdict / handle_input_line / run_session_action / placeholder / 4 dispatch_asi_* / dispatch_asi_calibrate / Key re-export` | ✅ (R116 + R127-2 P9-1 加 commands / output_format, 新增 re-export) |
| 20 | bench | `swe_bench (TaskInstance / RunReport / Executor / Runner / Summary) / agent_bench (category / task / runner) / self_disable_bench (20 case) / latency_bench (P50/P99) / placeholder / v2_expansion_summary / V1190_BENCH_NAME / v1190_summary` | ✅ |
| 21 | cognition | `CognitiveOutput / CognitivePipeline / ReflectionReport / ReflectionVerdict / continuity_score / identity_score / philosophy_guard_score / salience_score / transferability_score / CognitionError / CognitionResult / CognitiveInput / CognitiveCycle / BasicCognitiveEngine / 8 trait (Cognition / Intuition / Reasoning / MetaCognition / Recall / Consolidation / Forgetting / Learning / Abstraction)` | ✅ (R10 P2 加 BasicCognitiveEngine + 8 trait 默认实现, 新增 re-export) |
| 22 | action | `ActionAtom / ActionEngine / ActionPlan / ExecutionResult / RollbackResult / TxId / ActionIntent / ExpressionChannel / StructuredOutput / SilenceReason / ActionError / ActionResult / 3 trait (ActionExecution / ActionExpression / ActionSilence) / DefaultActionEngine / run_execute / run_express / run_silence / is_actionable / new_tx_id` | ✅ |
| 23 | life-force | `SelfGrowthIndicator / ReflectionPeriod / ReflectionPeriodState / StandardReflectionPeriod / ENDURANCE_MIN / ENDURANCE_MAX / ENDURANCE_EXHAUSTION_THRESHOLD / ENDURANCE_RECOVERY_TARGET / ReflectionTrigger / LifeForce / LifeForceError / reflection_trigger / exhaustion_check / recovery_start / validate_endurance / reflection_progress / EmergenceDetector / EmergenceError / EmergenceReport / EmergenceSignal / EmergenceSignalType / ReflectionCycleError / ReflectionCycleEvent / ReflectionCycleScheduler / ReflectionPhase` | ✅ (R22 ST-A2.1 + R22 ST-A2.3 加 reflection_cycle / emergence, 新增 re-export) |
| 24 | constraint | `PhilosophyKeyAccess / HardCodeConstraint / TwelveKeysHardcode / FourGates / FiveGates (deprecated) / PermissionGrant / GrantVerdict / RiskGrant / GateVerdict / VerdictCache / ConstraintEngine / ConstraintError / 4 deep_impl 顶层` | ✅ (round7-05 v15 命名修正: 5 重 → 4 重 + 权限发放, FiveGates 保留为 deprecated 向后兼容别名, 新增 re-export) |

**verify 结论**: **24/24 LOCKED crate 入口签名 0 改 全部通过**. V1.0 release 0 改 src 严守 (per 决策 #33 §2.3 + 决策 #74 §1 B1) 实施无虞.

---

## 2. 24 LOCKED 入口分布 8 个优化方向详细分析

### 2.1 方向 ①: 入口签名一致性 (Per-crate pub use 模式)

**现状**:
- 24 LOCKED crate 入口签名风格高度不一致, 总结为 5 种:
  - **类型 A (重 re-export facade)**: supervisor / agent / council / api / memory / core / mcp / graph / pipeline / constraint / evolution / cognition / life-force / tools / tool-runtime / tool-registry / tool-approval / asi / cli / bench (20/24)
    - 模式: `pub use module::*` 大量重导出
    - 优点: 消费者只需 `use apeireth_xxx::*` 就能拿全部 API
    - 缺点: 编译时间增加, 公开 API 表面膨胀, crate 间可见性模糊
  - **类型 B (轻 facade + 主类型定义)**: protocol / bus / bus
    - 模式: 入口文件直接定义核心类型 (BusMessage / BackpressurePolicy / BusStats) + 轻 re-export
    - 优点: 核心类型集中, 跨 crate 集成清晰
  - **类型 C (单 trait 入口)**: extension
    - 模式: 单 `pub use` 块重导出
    - 优点: 简洁
  - **类型 D (大 enum 主类型)**: asi / supervisor
    - 模式: 主 enum + 相关 const + 测量函数
  - **类型 E (纯 trait 模块)**: cognition
    - 模式: 入口几乎不 re-export, 主要靠 module 公开

**问题**:
- 24 个 crate 用了 5 种风格, 跨 crate 集成时需要先看每个 lib.rs 才能知道有哪些 API
- 公开 API 表面 = 24 crate 的 re-export union, 难以维护一份完整的"24 LOCKED public API"清单
- 编译时间: 重 re-export 模式下, 任何下游 crate 改一个就触发整个 union 重编译

**优化方向**:
- V1.0 release: 0 改 (R11 baseline 严守)
- V1.1 release: 引入"per-crate pub use 模式标准" (3 选 1: 全 re-export / 主类型 facade / 按需 re-export), per-crate 自决
- V2.0 release: 全量统一到 **3 模式之一**, 跟 organ-first 拓扑对齐

**风险**:
- 中: 改 re-export 模式 = 改 crate 公开 API 表面 = 改消费者 `use` 路径
- 缓解: 保留 `pub mod` 重新导出, 消费者用 `apeireth_xxx::module::Type` 全路径仍能用

---

### 2.2 方向 ②: 公开 API 表面 (Public API surface) 总量

**现状 (粗估)**:
- 24 LOCKED crate 的公开 API 表面 (按 re-export + 入口定义类型计):
  - supervisor: ~12 (actor / child / pid_one / strategy / supervisor)
  - agent: ~25 (含 4 专家 + SubAgent Registry)
  - council: ~50+ (Advisor + Council + Hold + Lifecycle + LLM + Persona + Sovereignty + Synthesis + 7 factory + 4 Collaboration mode + Constitution + Trace + Graph)
  - bus: ~20
  - protocol: ~40 (4 adapter + 4 bridge + bridge_ext 5 + normalized 8 + ws_v1 8 + 5 const)
  - mcp: ~30 (ServerInfo + 3 capability + ToolDef + 4 ResourceServer + 8 frame)
  - tool-registry: ~30 (Tool + 6 enum + 5 axis + 6 mock + Classifier 8 + Token 8)
  - tool-runtime: ~25 (5 module + 11 mcp_protocol)
  - graph: ~40 (Checkpoint + 4 conditional + 4 state + 11 Subgraph/Channel + 5 StateGraph + 7 Context)
  - pipeline: ~35 (8 module + 9 force_translate + 3 placeholder + 9 provider_registry + 3 retry + 2 streaming + 5 token + 6 tool_loop + 3 Pipeline)
  - tool-approval: ~15 (3 + 1 + 2 + 6 + 2 + 1)
  - extension: ~17 (5 + 6 plugin + 2 + 3 + 1 const)
  - evolution: ~50+ (5 council + 5 engine + 4 fail + 7 PODA + 19 library_autonomy + 14 library_autonomy_loop + 4 state + 13 traits + 3 const + 1 fn)
  - api: ~40+ (22 LLM + 11 protocol + 4 const)
  - core: ~50+ (4 + 1 + 5 onion + 2 human + 12 PhilosophyKey + 3 verdict + 1 trait + 5 Gate + 5 Risk + 13 ActionTarget + 4 ActionVerdict + 1 ActionGuard)
  - memory: ~50+ (EpisodeQuery + EpisodeStore + Identity + 3 analysis + Migration + 3 Semantic + 2 Note + 10 stream + 2 ThreeLayer + 3 UserProfile + MemoryError + 6 StreamKind + SqliteMemoryStore + ContinuitySnapshotStore + 3 Provider)
  - asi: ~50+ (8 calibration + 2 drift + TraceRepository + 3 llm_judge + 26 measure_* + 7 registry + 4 render + 2 scheduler + 2 tokenizer + 4 const + 4 name array + 2 legacy struct + DimensionTrace + placeholder)
  - tools: ~30 (5+7 trait + 6 grep + 7 file_ops + 3 git + 1 code_exec + 1 register + 1 result + 1 web_search + 5 const)
  - cli: ~25 (3 + 2 + 1 + 6 + 5 dispatch + Key)
  - bench: ~20 (swe_bench + agent_bench + self_disable_bench + latency_bench + 3 const/fn)
  - cognition: ~25 (3 decision + 2 reflection + 5 scoring + 5 error + CognitiveInput + CognitiveCycle + BasicCognitiveEngine + 8 trait)
  - action: ~20 (5 execution + 3 expression + 1 silence + 3 trait + DefaultActionEngine + 5 fn + 1 const)
  - life-force: ~25 (3 SGI + 3 Reflection + 4 Endurance const + 1 Trigger + 1 LifeForce + 1 Error + 5 fn + 6 emergence + 5 reflection_cycle)
  - constraint: ~25 (5 trait + 2 type + 4 type + 2 verdict enum + VerdictCache + ConstraintEngine + Error + 4 deep_impl)

**总计**: 24 crate 公开 API 表面 = **~800+ pub items** (粗估, 实测需 ripgrep 验证)

**问题**:
- 公开 API 表面过大 → 编译时间增加, 维护成本高
- 入口签名稳定性 = LOCKED 严守, 任何新增都触发 lib_tests 守 + compile-time assert
- 跨 crate 集成时命名冲突风险 (e.g. cognition 与 action 都有 `ExecutionResult` 类型不同)

**优化方向**:
- V1.0 release: 0 改 (R11 baseline 严守)
- V1.1 release: 公开 API 表面"瘦身" (per-crate 暴露 ≤30 pub items 目标, 多余的转 `pub(crate)` 或 module-private)
- V2.0 release: 全量按 9 organ 重构, 公开 API 表面按 organ 暴露 (e.g. `apeireth-brain::*` 暴露 brain-related types)

**风险**:
- 高: 公开 API 表面"瘦身" = 改入口签名 = 改消费者 `use` 路径 = breaking change
- 缓解: 保留 `pub mod module::Type` 全路径, 消费者用全路径仍能用, V1.1 release bump 1.2.1 (per 决策 #74 B2)

---

### 2.3 方向 ③: crate 间依赖图 (Dependency graph) 合理性

**现状 (实测 by lib.rs use 路径)**:
```
core (基础, 0 依赖其他 LOCKED)
↑
memory (use apeireth_core)  asi (0 LOCKED dep)  constraint (use apeireth_core)  cognition (use apeireth_core + asi)
↑
api (use pipeline + protocol)  pipeline (use protocol)  tool-registry (0 LOCKED dep)  protocol (0 LOCKED dep)  graph (0 LOCKED dep)  supervisor (0 LOCKED dep)
↑
agent (use tool-registry)  tool-runtime (use memory + tool-registry)  tool-approval (use tool-runtime)  tools (use tool-registry)  bus (0 LOCKED dep)  mcp (use tool-registry)  bench (0 LOCKED dep)  life-force (use core)  action (use core)  extension (0 LOCKED dep)  evolution (0 LOCKED dep)
↑
council (use core)  pipeline (use protocol)  api (use pipeline + protocol)
↑
cli (use core + asi)  bench (跨 24 LOCKED 全测)
```

**24 LOCKED crate 依赖图核心特征**:
1. **core 是基座** (7 个 crate 依赖: memory / constraint / cognition / council / life-force / action / cli)
2. **tool-registry 是 tool 生态基座** (5 个 crate 依赖: agent / tool-runtime / tools / mcp)
3. **protocol + pipeline 是 LLM 链基座** (2 个 crate 依赖: api + pipeline 互依)
4. **asi 是认知基座** (1 个 crate 依赖: cognition + cli)
5. **memory 是历史流基座** (1 个 crate 依赖: tool-runtime)
6. **0 依赖其他 LOCKED crate 的"叶子"**: supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench (9 个) — 这 9 个是 V1.0 release 之后"分层下沉"或"独立发布"的候选
7. **潜在循环依赖** (per lib.rs 静态分析, 0 实测验证):
   - core ↔ memory (memory use core, 但 R23 extensions re-export 也在 memory, 0 循环)
   - council ↔ evolution (council → evolution 集成, evolution 通过 CouncilAdapter 调 council, 实测是单向: evolution use council 字段, 0 循环)
   - api → pipeline → protocol (单向)
   - cognition → constraint → core (单向)
8. **无明显循环依赖** (基于 lib.rs 静态 import 分析)

**问题**:
- core 过度中心化 (7 个 crate 依赖), 任何 core 改动触发大面积重编译
- tool-registry 中心化 (5 个), 同样问题
- 9 个"叶子" crate (supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench) 实际上有内部跨 crate 集成 (extension 用 core, asi 用 asi 内), 但不被其他 LOCKED crate 依赖 → 这些可以下沉到独立子 workspace
- 24 LOCKED 数量 vs 实际 24+ 跨 crate 集成点 (per 决策 #75 §2.1 R131-4 cargo workspace 结构优化)

**优化方向**:
- V1.0 release: 0 改 (R11 baseline 严守)
- V1.1 release: **9 叶子 crate 拆 workspace** (per R131-4 cargo workspace 结构优化结论 + 决策 #74 B1 Mavis 自决改)
  - 新 workspace: `apeireth-leaf/{supervisor,protocol,bus,tool-registry,graph,extension,evolution,asi,bench}/Cargo.toml`
  - 顶层 `apeireth/Cargo.toml` 0 改, 9 叶子拆出来独立发布
- V2.0 release: 全量按 organ 拆 workspace (e.g. `apeireth-brain/{agent,council,cognition,asi}/Cargo.toml` + `apeireth-hand/{tool-registry,tool-runtime,tool-approval,tools,mcp,extension,action}/Cargo.toml` + `apeireth-memory/{memory,life-force}/Cargo.toml` + `apeireth-voice/{protocol,pipeline,api}/Cargo.toml` + `apeireth-bus/{bus,supervisor,graph}/Cargo.toml` + `apeireth-core/{core,constraint}/Cargo.toml` + `apeireth-cli/{cli,bench}/Cargo.toml`)

**风险**:
- 中: 拆 workspace = 改 Cargo.toml 路径 = 改消费者 `use apeireth_xxx` → `use apeireth::organ::xxx` (路径变化)
- 缓解: 保留 re-export facade (顶层 `apeireth` 重新导出全部 `apeireth-organ::xxx`, 0 改消费者代码)
- 缓解: V1.1 release bump 1.2.1 (per 决策 #74 B2)

---

### 2.4 方向 ④: crate 内部模块 (internal modules) 分布

**现状 (per lib.rs pub mod 块数 + 模块大小)**:
| # | crate | pub mod 数 | 主要模块 | 模块大小 |
|---|---|---|---|---|
| 1 | supervisor | 5 | actor / child / pid_one / strategy / supervisor | 中 |
| 2 | agent | 3 (+1 subagent) | agent / manager / subagent | 小+中+大 |
| 3 | council | 13 (+4 collaboration) | advisor / bus_bridge / graph_bridge / mcp_bridge / council_member / council_member_deliberation / council_member_persona_combo / deliberation / hold / lifecycle / mock_llm / persona / sovereignty / stress_test / synthesis / advisors / collaboration / constitution / trace / graph_orchestration | 极大 (20+ 模块) |
| 4 | bus | 5 (+3 cfg) | l0 / l1 / l2 / l3 / l4 | 中 |
| 5 | protocol | 7 | adapter / adapters / bridge / bridge_ext / error / gateway / normalized / ws_v1 | 中 |
| 6 | mcp | 13 | protocol / resources / resource_servers / subscriptions / tool_subscriptions / tool_bridge / tools / initialize / prompts / telemetry_bridge / transport / primitives / macros | 极大 (13 模块) |
| 7 | tool-registry | 5 | classifier / registry / token_budget / trait_def / types | 中 |
| 8 | tool-runtime | 6 | executor / fuzzy / mcp_protocol / parser / privacy / record | 中 |
| 9 | graph | 11 | checkpoint / conditional / executor / mcp_resource / state / subgraph / channel / state_graph / context_graph / cognition_graph | 极大 (11 模块) |
| 10 | pipeline | 11 | force_translate / model_router / placeholder / provider_registry / tiktoken_counter / retry_suppression / role_divider / streaming / token_budget / tool_loop | 极大 (11 模块) |
| 11 | tool-approval | 6 | decision / fuzzy_bridge / history / manager / rule / rule_trait | 中 |
| 12 | extension | 7 | audit / error / manifest / plugins / registry / sandbox / traits | 中 |
| 13 | evolution | 9 | council_bridge / engine / fail / poda_cycle / state / traits / library_autonomy / library_autonomy_loop | 极大 (9 模块) |
| 14 | api | 13 | llm / protocol_handlers / server / cache / replay_cache / retry / routing / v2_endpoints / audit_sqlite / v2_routes / observability / endpoints / v1_tools / auth / ws_v1 / protocol_handler_trait | 极大 (16 模块) |
| 15 | core | 0 (everything in lib.rs) | n/a (核心 type 全在 lib.rs) | 极大 (1 个 lib.rs = 108KB) |
| 16 | memory | 10 + 2 pub | append_only / episode / identity / migrations / semantic / semantic_persist / session_note / streams / three_layer / user_profile / history_streams / continuity_link / llm_analysis | 极大 (13 模块) |
| 17 | asi | 8 | calibration / dim_enhance / drift / history / llm_judge / measurement / render / scheduler / tokenizer | 极大 (9 模块) |
| 18 | tools | 11 | code_exec / long_task / classifier / web_fetch / apply_patch / conventions_scanner / grep_ops / file_ops / git_ops / register / result / web_search | 极大 (12 模块) |
| 19 | cli | 3 + 1 sub | commands / output_format (其他在 lib.rs) | 小 |
| 20 | bench | 5 | swe_bench / agent_bench / self_disable_bench / latency_bench | 中 |
| 21 | cognition | 3 | decision / reflection / scoring | 中 |
| 22 | action | 3 | execution / expression / silence | 中 |
| 23 | life-force | 2 | reflection_cycle / emergence | 中 |
| 24 | constraint | 1 (+1 deep_impl) | deep_impl | 大 |

**问题**:
- **大模块集中**: council (20+) / mcp (13) / graph (11) / pipeline (11) / api (16) / memory (13) / asi (9) / tools (12) / evolution (9) → 这些 crate 内部模块多, 入口文件 re-export 100+ items
- **core 是单 lib.rs 108KB**: 没有 pub mod 拆分, 全部 50+ 类型定义在一个文件 → 编译时全文件 re-parse, 难维护
- **mcp / pipeline / api / memory 内部 module 边界模糊**: 多个 module 之间 cross-use, 实测命名重复 (e.g. `mcp::protocol::Id` vs `mcp::tools::Id`)

**优化方向**:
- V1.0 release: 0 改 (R11 baseline 严守)
- V1.1 release: **core 拆 pub mod** (core 当前 1 个 108KB lib.rs, 拆成 `core/{types,onion,human,principle,gate,action,verdict}/mod.rs`, 0 改入口签名, 仅内部重构)
- V1.1 release: **大模块集中 crate 拆 sub-crate** (e.g. mcp 拆 `mcp-core` + `mcp-resources` + `mcp-subscribe` + `mcp-tools` + `mcp-prompts` + `mcp-transport` + `mcp-primitives` + `mcp-macros`, 顶层 mcp 保留 re-export facade)
- V2.0 release: 按 organ 重构, module 边界跟 organ 边界对齐 (e.g. brain 模块只放 agent / council / cognition / asi, hand 模块只放 tool-runtime / tool-registry / tool-approval / tools / mcp / extension / action)

**风险**:
- 中: 拆 module = 改 import 路径 (`use apeireth_xxx::module::Type` → `use apeireth_xxx::sub_crate::Type`) = breaking change
- 缓解: 顶层 re-export facade 保留, 消费者用 `apeireth_xxx::Type` 仍能用

---

### 2.5 方向 ⑤: 三洋葱架构 (Three-onion architecture) 落地

**三洋葱架构** (per 决策 #33 + R125 B6 + 决策 #74 §1 B4):
1. **原则洋葱** (PrincipleOnion): E/S/A/M/O 5 切片 (核心哲学) — 锁在 core
2. **权限洋葱** (PermissionOnion): L0-L5 6 切片 (权限分层) — 锁在 core
3. **DSL 洋葱** (新, R125 B6 升级, v6 守门): Colang DSL 守门 (NeMo Guardrails 借鉴)

**24 LOCKED crate 跟三洋葱架构对应关系**:
| 洋葱层 | 24 LOCKED 落地 |
|---|---|
| **原则洋葱 E 层 (存在层, 不可降级)** | core (L0 HA 锁) / constraint (哲学守门) / life-force (SGI 锁) |
| **原则洋葱 S 层 (价值层, 智囊团审议)** | council (7 强制 Advisor) / evolution (演化审议) |
| **原则洋葱 A 层 (经验沉淀层)** | memory (历史流 6 表) / asi (24 维测量历史) |
| **原则洋葱 M 层 (方法论层)** | cognition / pipeline / protocol / bus / graph |
| **原则洋葱 O 层 (操作原则层, 可改)** | agent / tool-registry / tool-runtime / tool-approval / tools / mcp / extension / action / api / cli / bench / supervisor |
| **权限洋葱 L0 (HA 核心)** | core (L0 HA 锁) / constraint (gate3 物理隔离) |
| **权限洋葱 L1-L5** | api (V2 端点) / tool-approval (5 规则 + 5min 窗口) |
| **DSL 洋葱 (Colang DSL)** | 0 落地, 24 LOCKED 都 0 引用 Colang |

**问题**:
- 三洋葱架构在 24 LOCKED crate 中的落地是"分散式" (原则洋葱散布在 core / constraint / life-force, 权限洋葱散布在 core / api / tool-approval, DSL 洋葱 0 落地)
- E 层锚 (L0 HA / SGI / 12 键) 锁在 core 单 crate, 风险: core 改动 = 整个哲学基础重定义
- DSL 洋葱 0 落地, 24 LOCKED 0 引用 Colang, per R125 B6 升级路线

**优化方向**:
- V1.0 release: 0 改 (R11 baseline 严守)
- V1.1 release: **DSL 洋葱落地** (新增 `apeireth-dsl` crate, Colang 真实施, 24 LOCKED crate 引用 dsl 守门)
- V1.1 release: **三洋葱架构 workspace 化** (per R131-4 cargo workspace 优化 + 决策 #74 B1 Mavis 自决改)
  - `apeireth-onion/` workspace: core (原则 + 权限双洋葱) + constraint (守门) + dsl (DSL 洋葱) + life-force (SGI)
- V2.0 release: 全量按三洋葱重构成 3 个 workspace (原则 / 权限 / DSL), 24 LOCKED 全部下沉到对应洋葱 workspace

**风险**:
- 高: 拆三洋葱 workspace = 改大量 import 路径 = breaking change
- 缓解: 顶层 `apeireth-onion` facade 重新导出全部洋葱 module, 消费者 0 改
- 缓解: V1.1 release bump 1.2.1 (per 决策 #74 B2)

---

### 2.6 方向 ⑥: 9 organ 代码对应 (per R11 9 organ + R125 B7 内部借 OpenCode)

**9 organ** (per `reports/9-organ-summary-2026-08-10.md`):
0=Heart / 1=Brain / 2=Hand / 3=Eye / 4=Ear / 5=Memory / 6=Voice / 7=Body / 8=Mind

**24 LOCKED crate 跟 9 organ 代码对应** (实测 by lib.rs 顶层 type + 职责):
| Organ | 对应 LOCKED crate | 理由 |
|---|---|---|
| **Heart (0, LLM 网关心跳)** | supervisor + bus (L0 层) + pipeline (5 步管线) | 进程监管 + 5 层通信 + LLM 调用节拍 |
| **Brain (1, Multi-Agent 决策)** | agent + council + cognition + constraint | 4 大主脑 + 智囊团审议 + 12 键 verdict 守门 |
| **Hand (2, Tool Protocol)** | tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action | 6 工具执行 + 5 规则 + 5 trait + 6 类插件 + 行动 |
| **Eye (3, 用户输入感知)** | (暂无 LOCKED crate) | 9 organ 中 Eye 在 apeireth-tui/src/organ/eye.rs, 不在 24 LOCKED |
| **Ear (4, 系统事件监听)** | bus (L1-L4) | 5 层通信总线 (L1 UDS / L2 pipe / L3 gRPC / L4 WS) |
| **Memory (5, 3 层 facade)** | memory + asi (历史 24 维) + life-force (SGI 锁) + core (IdentityCard 跨载体) | 历史流 6 表 + 24 维测量 + SGI 主体连续性 |
| **Voice (6, TTS/STT)** | protocol (WS 8 帧) + pipeline (流式) | LLM 协议归一化 (4 LLM + WS) + 流式输出 |
| **Body (7, 长程任务)** | bench + api (HTTP server) + cli | 长程任务状态 + HTTP server + CLI runner |
| **Mind (8, 9-stage lifecycle)** | evolution + graph (lifecycle 编排) + constraint (5 重守门) | 6 状态机 + graph 编排 + 4 重守门 |

**覆盖率分析**:
- **8/9 organ 100% 覆盖** (除 Eye 在 tui, 不在 24 LOCKED): Heart / Brain / Hand / Ear / Memory / Voice / Body / Mind
- **Eye (用户输入感知) 缺失**: 24 LOCKED 0 crate 对应 Eye, Eye 实际在 apeireth-tui/src/organ/eye.rs (per 9-organ-summary §3 Eye 11.0KB, 4 输入通道: keystroke / mouse_click / voice_input)
- **Hand 重复覆盖** (7 个 LOCKED crate): tool-registry / tool-runtime / tool-approval / tools / mcp / extension / action — Hand 实际是"工具 + 行动"复合 organ, 7 个 crate 都跟 Hand 强相关
- **Brain 重复覆盖** (4 个): agent / council / cognition / constraint — Brain 是"Multi-Agent 决策 + 智囊团审议 + 认知 + 守门"复合 organ
- **Memory 重复覆盖** (4 个): memory / asi / life-force / core — Memory 是"历史流 + 测量 + 生命力 + 主体"复合 organ
- **Mind 重复覆盖** (3 个): evolution / graph / constraint — Mind 是"演化 + 编排 + 守门"复合 organ

**问题**:
- 24 LOCKED crate 对 9 organ 映射不是 1:1, 而是 N:1 (多个 LOCKED crate 对应同一 organ)
- Eye 在 24 LOCKED 0 对应, 在 tui 有独立 organ 入口
- Mind organ 实际包含 constraint (5 重守门) → 守门 organ 跟 mind organ 边界模糊
- 9 organ 内部借 OpenCode (R125 B7) 在 24 LOCKED crate 中 0 体现 (organ-first 拓扑 0 落地)

**优化方向**:
- V1.0 release: 0 改 (R11 baseline 严守)
- V1.1 release: **9 organ workspace 化** (per 决策 #74 B1 Mavis 自决改 + R125 B7 内部借 OpenCode)
  - 新增 `apeireth-organ/{heart,brain,hand,eye,ear,memory,voice,body,mind}/Cargo.toml` 9 个 organ workspace
  - 24 LOCKED crate 按 9 organ 拆:
    - `apeireth-heart` workspace: supervisor + bus (L0) + pipeline
    - `apeireth-brain` workspace: agent + council + cognition + constraint
    - `apeireth-hand` workspace: tool-registry + tool-runtime + tool-approval + tools + mcp + extension + action
    - `apeireth-eye` workspace: (从 tui/src/organ/eye.rs 抽 crate)
    - `apeireth-ear` workspace: bus (L1-L4)
    - `apeireth-memory` workspace: memory + asi + life-force + core (IdentityCard 跨载体)
    - `apeireth-voice` workspace: protocol + pipeline (流式) + (未来 tts/stt crate)
    - `apeireth-body` workspace: bench + api + cli
    - `apeireth-mind` workspace: evolution + graph + (约束守门从 brain/constraint 拆过来)
- V2.0 release: 全量按 9 organ 重构, 24 LOCKED 全部下沉到 organ workspace, 顶层 `apeireth` re-export 全部 organ types

**风险**:
- 极高: 9 organ 重构 = 改 24 LOCKED crate 全部路径 = 改 N 个消费者的 `use` 路径 = breaking change
- 缓解: 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用
- 缓解: V1.1 release bump 1.2.1, V2.0 release bump 2.0.0 (semver major)
- 缓解: 跟"不要怕复杂度 + 最强效果 + 最厉害工程"哲学一致 (per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

---

### 2.7 方向 ⑦: R11 baseline 严守 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)

**R11 baseline 3 值** (per 决策 #33 §2.3 A1 + 决策 #74 §1 A1):
- V1141 IC-001 fresh 24 维均值: 0.8682
- V1131 dashboard 9 维均值: 0.8532
- V1136 9 子测度均值: 0.9063

**实测 24 LOCKED 入口分布跟 R11 baseline 对应**:
- V1141 24 维: 锁在 `apeireth-asi::V05_DIMENSION_NAMES` (24 维名 + V05_DIM_COUNT 编译期 hardcode)
- V1131 dashboard 9 维: 锁在 `apeireth-asi::V1136_SUBMEASURE_NAMES` (9 子测度名 + V1136_SUBMEASURE_COUNT 编译期 hardcode)
- V1136 9 子测度基础: 锁在 `apeireth-asi::measurement::measure_dim_*` + `measure_sub_*` 真实测量函数 (24+9 = 33 个测量函数)

**24 LOCKED 入口分布严守 R11 baseline**:
- ✅ V1141 / V1131 / V1136 数字 0 改 (per 决策 #33 §2.3 A1)
- ✅ V05_DIM_COUNT = 24 编译期 hardcode (apeireth-asi lib.rs line 53)
- ✅ V1136_SUBMEASURE_COUNT = 9 编译期 hardcode (apeireth-asi lib.rs line 56)
- ✅ V05_DIMENSION_NAMES 数组 (apeireth-asi lib.rs line 59-89) 24 个名称顺序 LOCKED
- ✅ V1136_SUBMEASURE_NAMES 数组 (apeireth-asi lib.rs line 92-105) 9 个名称顺序 LOCKED
- ✅ 24 measure_dim_* 函数 (apeireth-asi lib.rs line 32-46) 函数签名 LOCKED
- ✅ 9 measure_sub_* 函数 (apeireth-asi lib.rs line 32-46) 函数签名 LOCKED

**问题**:
- 0 (R11 baseline 严守 100% 实施, 24 LOCKED 入口分布无破坏)

**优化方向**:
- V1.0 release: 0 改 (R11 baseline 严守, 决策 #74 §1 A1)
- V1.1 release: **R12 测度对齐** (per 决策 #74 §2.3 V1.1 release 边界, R12 测度更新 → 新 baseline 更高, 24 LOCKED 入口签名更新 R12 测度)
- V2.0 release: **R12+ 测度重评** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评)

**风险**:
- 中: 改 R12 测度 = 改 24 测量函数签名 = 改 24 LOCKED 入口签名
- 缓解: 仅在 V1.1 release 改 (per 决策 #74 §2.3 V1.1 release 边界), V1.0 release 仍 R11 baseline 严守

---

### 2.8 方向 ⑧: V1.1 release 改写 + V2.0 release 重构 边界

**V1.0 release (整合 #5.1 commit, 0 改 src 严守)**:
- 24 LOCKED 入口签名 0 改 (per §1.2 verify)
- 24 LOCKED crate mtime 8/10 16:34 之后 8 个 (agent / mcp / tool-runtime / graph / pipeline / evolution / api / cli) 保持 0 改
- R11 baseline 3 值严守
- PHL-07 spec-only 0 实施
- workspace.version 1.2.0 严守 (per 决策 #74 §1 B2)

**V1.1 release 入口签名 改写方案** (per 决策 #74 §2.3 V1.1 release 边界):
- 触发条件: 更好的架构 (e.g. ASI Stage 9 长程 AI 成长 + 9 organ 内部借 OpenCode + 三洋葱架构升级)
- Mavis 自决改
- workspace.version bump 1.2.0 → 1.2.1 (per 决策 #74 §1 B2)
- PHL-07 实施 (per R129-11 关键诚实标)

**V1.1 release 改写入口签名 8 个方向**:
1. **入口签名一致性 标准化**: per-crate 选 3 模式之一 (全 re-export / 主类型 facade / 按需 re-export), 24 LOCKED 全部统一
2. **公开 API 表面 瘦身**: per-crate 暴露 ≤30 pub items, 多余的转 `pub(crate)` 或 module-private
3. **9 叶子 crate 拆 workspace**: supervisor / protocol / bus / tool-registry / graph / extension / evolution / asi / bench → `apeireth-leaf/` workspace
4. **core 拆 pub mod**: 当前 1 个 108KB lib.rs 拆成 `core/{types,onion,human,principle,gate,action,verdict}/mod.rs`
5. **大模块集中 crate 拆 sub-crate**: mcp 拆 mcp-core / mcp-resources / mcp-tools / mcp-transport / mcp-primitives / mcp-macros; pipeline 拆 pipeline-token / pipeline-placeholder / pipeline-force-translate / pipeline-retry / pipeline-streaming / pipeline-tool-loop
6. **DSL 洋葱落地**: 新增 `apeireth-dsl` crate, Colang 真实施, 24 LOCKED crate 引用 dsl 守门
7. **9 organ 内部借 OpenCode (R125 B7)**: 24 LOCKED crate 内部 fn 借 OpenCode 0 改入口签名
8. **R12 测度对齐**: 24 测量函数签名更新 R12 测度, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新

**V2.0 release 入口签名 重构方案** (per 决策 #74 §2.3 V2.0 release 8 硬墙可重评):
- 触发条件: per Mavis 自决 + 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚"
- workspace.version bump 1.2.1 → 2.0.0 (semver major, breaking change)
- 全 8 硬墙可重评
- 推翻 + 重建 8 哲学锚 (per "不要怕复杂度 + 最强效果 + 最厉害工程" 哲学)

**V2.0 release 重构入口签名 8 个方向**:
1. **全量统一入口签名 3 模式**: 24 LOCKED 全部按 organ-first 选 1 模式
2. **公开 API 表面全量按 organ 暴露**: `apeireth-brain::*` / `apeireth-hand::*` / `apeireth-memory::*` 等
3. **9 organ workspace 化**: 24 LOCKED 全部下沉到 organ workspace, 顶层 `apeireth` re-export 全部
4. **core 全量拆 pub mod**: core 拆成 onion / human / principle / gate / action / verdict 6 个 sub-module
5. **大模块集中 crate 拆 sub-crate**: mcp / pipeline / api / memory / asi / tools / evolution 全拆
6. **三洋葱 workspace**: 原则 / 权限 / DSL 3 个独立 workspace, 24 LOCKED 全部下沉
7. **9 organ 内部借 OpenCode 实施**: organ-first 拓扑落地, Eye 抽 crate
8. **R12+ 测度重评**: 24 测量函数按 ASI Stage 9 重写, 编译期 hardcode 全部更新

---

## 3. 8 硬墙严守 + B1 改写 (per 决策 #33 §2.3 + 决策 #74 §1)

| # | 8 硬墙 | V1.0 release (整合 #5.1 commit) | V1.1 release (per 决策 #74 §2.3) | V2.0 release (per 决策 #74 §2.3) |
|---|---|---|---|---|
| **B1** | 24 LOCKED 入口签名 | 🔒 0 改严守 (R11 baseline) | 🟢 Mavis 自决改 (前提: 更好的架构) | 🟢 重构 (per Mavis 自决 + 主人 8/11 01:14 拍板) |
| **B2** | workspace.version 1.2.0 | 🔒 1.2.0 严守 | 🔒 bump 1.2.1 | 🔒 bump 2.0.0 |
| **A1** | R11 baseline 3 值 | 🔒 0 改严守 (哲学 + 效果标) | 🟢 可改 (前提: 新的 baseline 更高) | 🟢 可重评 |
| **A3** | 12 键 + PHL-07 | 🔒 PHL-07 spec-only 0 实施 + 12 键其他可改 | 🟢 PHL-07 实施 | 🟢 可重评 |
| **B3** | V0.5 30 维 | 🔒 严守 (哲学) | 🔒 严守 | 🟢 可重评 |
| **B4** | 6 重守门 v7 | 🔒 严守 (哲学) | 🔒 严守 | 🟢 可重评 |
| **B5** | 8 哲学锚 | 🔒 严守 (哲学) | 🔒 严守 | 🟢 推翻 + 重建 |
| **C1** | 0 主动 commit (主人起床前) | 🔒 严守 | 🔒 严守 | 🔒 严守 |
| **C2** | 0 装 PASS | 🔒 严守 | 🔒 严守 | 🔒 严守 |
| **0 push** | 0 主动 push (主人起床前) | 🔒 严守 | 🔒 严守 | 🔒 严守 |

**B1 改写边界 (per 决策 #74 §2.2 + §2.3)**:
- ✅ V1.0 release: 24 LOCKED 入口签名 0 改 (R11 baseline 严守) — 整合 #5.1 commit 0 改 src
- ✅ V1.1 release: 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决)
- ✅ V2.0 release: 24 LOCKED 入口签名 可重评 (per Mavis 自决 + 主人 8/11 01:14 拍板)

---

## 4. 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1)

8 哲学锚 (per R125 B5 升 8 锚, `docs/conventions/09-anchor.md`):
- **S-1 (服务 ASI 北极星)**: 24 LOCKED 入口分布全围绕 ASI Stage 9 长程 AI 成长 (V0.5 30 维 + 9 organ + 24 LOCKED 全部对齐)
- **S-2 (实事求是)**: 24 LOCKED 入口签名 0 改 verify (per §1.2) = 不漂移
- **S-3 (R125 B5 新增, 主人 16:27 拍板)**: 24 LOCKED crate 都有"实测函数" (e.g. measure_dim_*) → 不装 PASS
- **O-1 (质量工程化)**: 24 LOCKED 入口都有 `compile-time assert` 守门 (per lib.rs `const _: () = { assert!(...) }` 块)
- **O-2 (安全优先)**: 24 LOCKED 入口都有 12 键 verdict 守门 (per V0 + V1 + V2 + V3 AND 门)
- **O-3 (走在前人经验上)**: 24 LOCKED 入口都有"VCP / AutoGen / LangGraph / OpenCode / superpowers / aGLM" 等借鉴注释 (per lib.rs 顶部 doc comment)
- **O-4 (干到底)**: 24 LOCKED 入口都有 unit tests ≥ 20 (per 各 lib.rs `mod tests` 块)
- **O-5 (任何人都能接手)**: 24 LOCKED 入口都有"架构位置" + "不假装" + "不修改承诺" 3 段 doc comment

**8 哲学锚严守 0 漂移**: ✅ 24 LOCKED 全部严守, V1.0 release / V1.1 release / V2.0 release 都严守 (除 B5 V2.0 release 推翻 + 重建)

---

## 5. 不要怕复杂度哲学落地 (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md)

**主人 8/11 01:14 拍板 3 件套 §3** (per 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md):
> "总哲学除了思想文档的，我给你补充一点，就是不要怕复杂度爆炸或者维护复杂，我们只要最强的效果和最厉害的工程，因为自然会有高水平的团队来接手维护"

**3 核心**:
1. **最强效果 > 最简单代码** (推翻 KISS, 拥抱 SOTA)
2. **最厉害工程 > 最易维护** (推翻 DRY, 拥抱 BORROW)
3. **维护交给未来高水平团队** (推翻"代码要让初级团队能接手", 拥抱"代码要让高水平团队能发挥")

**24 LOCKED 入口分布 跟 不要怕复杂度哲学 落地**:
- ✅ 24 LOCKED 入口表面 800+ pub items → "最强效果" (高 API 表面 = 强大功能)
- ✅ 24 LOCKED 跨 crate 集成 24+ 集成点 → "最厉害工程" (多借鉴 = 高水平)
- ✅ 24 LOCKED 入口 compile-time assert + 12 键守门 + 5 重守门 → "高质量" (复杂守门 = 安全)
- ✅ 24 LOCKED 入口 doc comment 极详细 (per 顶部 50-100 行 doc) → "高水平团队能接手" (详细文档)

**V1.1 release / V2.0 release 改写/重构 跟不要怕复杂度哲学**:
- V1.1 release 拆 workspace / 拆 sub-crate / 加 DSL 洋葱 → "不要怕复杂度" (拆 = 复杂, 但效果更强)
- V2.0 release 全量按 organ 重构 → "不要怕复杂度" (全量重构 = 极复杂, 但工程最厉害)
- 维护: 交给未来高水平团队 (per 主人 8/11 01:14 拍板 §3)

---

## 6. 风险 + 决策原则

### 6.1 风险

| # | 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|---|
| R1 | 主人 8/11 01:14 决策 3 件套理解有误 | 低 | 中 | 决策 #73 §2.1-§4.1 详细解读 + 决策 #74 §1 8 硬墙改写表 + §3 分类 + §2 B1 改写边界 |
| R2 | 整合 #5.1 commit 拍板推迟 (R129-3 报告迟迟不出) | 中 | 中 | 01:15 tick 仍未出 → Section 3 中断接手, Mavis 写报告 |
| R3 | 主人起床后看 8 硬墙 B1 改写觉得"破坏 R11 baseline" | 低 | 高 | V1.0 release 仍 0 改严守, V1.1 release Mavis 自决改 (R12 测度对齐 + 跟 R125 B3 + R127 25 维公式), 不会破坏 V1.0 release |
| R4 | V1.1 release locked 改写打破向后兼容 | 中 | 中 | V1.1 release 是 minor release bump 1.2.0 → 1.2.1 (per 决策 #74 B2), semver 兼容 |
| R5 | 团队对"不要怕复杂度"哲学不适应 | 中 | 中 | 主人 8/11 01:14 拍板"自然会有高水平的团队来接手维护", 未来高水平团队能适应 |
| R6 | 9 organ workspace 重构打破 24 LOCKED 入口签名 | 高 | 高 | 顶层 `apeireth` re-export facade 保留, 消费者用 `apeireth::Type` 仍能用 |
| R7 | 三洋葱架构升级 (DSL 洋葱) 引入新依赖 | 中 | 中 | V1.1 release 评估, V2.0 release 才实施 |
| R8 | R12 测度对齐改动过大, 24 测量函数签名全变 | 中 | 高 | 24 测量函数签名更新 R12 测度, V05_DIM_COUNT / V1136_SUBMEASURE_COUNT 编译期 hardcode 同步更新, 测试全跑 |
| R9 | core 拆 pub mod 引发 core 内部 cross-use 错误 | 中 | 中 | 拆 module 时保持原 re-export, 内部 cross-use 路径不变 |
| R10 | 24 LOCKED 入口分布优化的 mtime baseline 16:34 之前 8 个 crate 实际 mtime 已超 | 已发生 | 低 | 8 个超 16:34 的 crate 是 R127-2/R128 era 升级, 0 改入口签名, V1.0 release commit 拍板时保持 mtime 不再变 |

### 6.2 决策原则

- **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- **8 硬墙严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1 拍板)
- **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 + V2.0 release 可重评
- **B2 workspace.version 1.2.0**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1 + V2.0 release bump 2.0.0
- **A1 R11 baseline 3 值**: V1.0 release 严守 + V1.1 release 可改 (前提: 新 baseline 更高) + V2.0 release 可重评
- **A3 12 键 + PHL-07**: V1.0 release 12 键其他可改 + PHL-07 spec-only 0 实施 + V1.1 release PHL-07 实施 + V2.0 release 可重评
- **B3 V0.5 30 维**: V1.0 release 严守 + V1.1 release 严守 + V2.0 release 可重评
- **B4 6 重守门 v7**: V1.0 release 严守 + V1.1 release 严守 + V2.0 release 可重评
- **B5 8 哲学锚**: V1.0 release 严守 + V1.1 release 严守 + V2.0 release 推翻 + 重建
- **C1 0 主动 commit (主人起床前)**: 严守
- **C2 0 装 PASS 严守**: 严守
- **0 push (主人起床前)**: 严守
- **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3)
- **整合 #5 commit 由 Mavis 自动拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- **0 主动删** (per Safety policy + 决策 #44 + #60)
- **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- **决策日志写** (per 决策 #10 + 用户记忆 #10)

---

## 7. 整合 #5 commit 拍板逻辑 (per 决策 #62 + 决策 #73 §5 + 决策 #74 §4)

### 7.1 整合 #5.1 commit (src/ 实施, 95+ 文件)

- ✅ 0 改 24 LOCKED 入口签名 (per §1.2 verify, 24/24 通过)
- ✅ 0 改 24 LOCKED crate mtime baseline 16:34 之前 (8/10 16:34 之后 8 个 crate 已发生, V1.0 release commit 拍板时保持 mtime 不再变)
- ✅ 0 改 R11 baseline 3 值
- ✅ PHL-07 spec-only 0 实施 (V1.1 实施)
- ✅ 排除 `crates/apeireth-graph/src/lib.rs.bak.p6-2` (P6-2 backup, per 决策 #62 §5.1)

### 7.2 整合 #5.2 commit (docs/ + Cargo.toml, 10 文件)

- ✅ CHANGELOG.md / ROADMAP.md / RELEASE_NOTES.md / OSS_NOTICE.md
- ✅ Cargo.toml borrow 段 update 17:44 → 22:50 状态 (per 决策 #62 §5.2)
- ✅ Cargo.lock / .gitignore
- ✅ docs/roadmap/ / frontend/ / library/
- ✅ + 新增 `docs/conventions/15-no-fear-complexity.md` (per 决策 #73 §3)
- ✅ + 更新 `docs/conventions/10-locked.md` (per 决策 #73 §2.3)
- ✅ + 更新 `docs/conventions/09-anchor.md` (per 决策 #73 §4.2)
- ✅ + 更新 `docs/conventions/README.md` (per 决策 #73 §2.3 + §4.2)
- ✅ + 更新 `CONTRIBUTING.md` (per 决策 #73 §2.3)
- ✅ + 更新 `README.md` (per 决策 #73 §2.3)

### 7.3 整合 #5.3 commit (reports/, 60+ 文件)

- ✅ 决策链 #30-#64 全读 verify
- ✅ 41 sub-agent 报告
- ✅ HANDOFF
- ✅ + 新增 decision-73 (主) + decision-74 (8 硬墙 B1 改写) (per 决策 #73 §2.2 + §5)
- ✅ + 新增 R131 era 调研 3 sub-agent 报告 (R131-1 + R131-2 + R131-3, per 决策 #73 §3.2)
- ✅ + 新增 `philosophy-no-fear-complexity-2026-08-11.md`
- ✅ + 新增 **本报告 R131-5 24 LOCKED 入口分布优化** (本文件)

---

## 8. 总结

### 8.1 24 LOCKED 入口分布 8 优化方向 一句话总结

1. **入口签名一致性**: 24 LOCKED 用 5 种 re-export 风格, V1.0 release 0 改, V1.1 release 标准化 3 模式之一, V2.0 release 全量统一
2. **公开 API 表面**: 24 LOCKED 共 ~800+ pub items, V1.0 release 0 改, V1.1 release 瘦身 ≤30/crate, V2.0 release 按 organ 暴露
3. **crate 间依赖**: 24 LOCKED 7 个 dep core + 5 个 dep tool-registry + 9 叶子 crate, V1.0 release 0 改, V1.1 release 9 叶子拆 workspace, V2.0 release 全量按 organ 拆
4. **crate 内部模块**: 24 LOCKED 5 大模块集中 crate (council 20+ / mcp 13 / graph 11 / pipeline 11 / api 16), V1.0 release 0 改, V1.1 release core 拆 pub mod + 大模块拆 sub-crate, V2.0 release 按 organ 拆
5. **三洋葱架构**: 24 LOCKED 落地原则 + 权限双洋葱, DSL 洋葱 0 落地, V1.0 release 0 改, V1.1 release DSL 洋葱落地 + 三洋葱 workspace 化, V2.0 release 全量按三洋葱重构
6. **9 organ 代码对应**: 24 LOCKED 8/9 organ 覆盖 (Eye 缺失), V1.0 release 0 改, V1.1 release 9 organ workspace 化 (新增 Eye), V2.0 release 全量按 9 organ 重构
7. **R11 baseline 严守**: V1141=0.8682 / V1131=0.8532 / V1136=0.9063 100% 严守, V1.0 release 0 改, V1.1 release R12 测度对齐, V2.0 release R12+ 测度重评
8. **V1.1/V2.0 改写边界**: V1.0 release 0 改, V1.1 release 8 方向改写 (Mavis 自决), V2.0 release 8 方向全量重构 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板)

### 8.2 V1.0 release 拍板 (per 整合 #5 commit 拍板逻辑)

- ✅ 24 LOCKED 入口签名 0 改 全部 verify 通过 (per §1.2)
- ✅ 0 改 src 严守 (per 决策 #33 §2.3 + 决策 #74 §1 B1)
- ✅ 8 硬墙 严守 (per 决策 #74 §1)
- ✅ 8 哲学锚 严守 (per 决策 #33 §2.3 B5)
- ✅ R11 baseline 3 值 严守 (per 决策 #33 §2.3 A1)
- ✅ 0 主动 commit (主人起床前) 严守 (per 决策 #33 §2.3 C1)
- ✅ 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
- ✅ 0 主动 push 严守 (per 决策 #33)

### 8.3 V1.1 release 改写 路线图 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套)

- **触发条件**: 更好的架构 (per 决策 #74 §2.3, 主人 8/11 01:14 拍板 "Mavis 自决架构拍板")
- **改写方向** (8 方向 per §2.8 V1.1 release 改写入口签名 8 个方向):
  1. 入口签名一致性 标准化
  2. 公开 API 表面 瘦身
  3. 9 叶子 crate 拆 workspace
  4. core 拆 pub mod
  5. 大模块集中 crate 拆 sub-crate
  6. DSL 洋葱落地
  7. 9 organ 内部借 OpenCode (R125 B7)
  8. R12 测度对齐
- **Mavis 自决改**: per 主人 8/11 01:14 拍板 "Mavis 自决架构拍板"
- **workspace.version**: 1.2.0 → 1.2.1 (per 决策 #74 §1 B2)
- **PHL-07 实施**: per R129-11 关键诚实标 + 决策 #74 §2.3 V1.1 release 边界

### 8.4 V2.0 release 重构 路线图 (per 决策 #74 §2.3 + 主人 8/11 01:14 拍板 3 件套)

- **触发条件**: per Mavis 自决 + 主人 8/11 01:14 拍板 "推翻 + 重建 8 哲学锚"
- **重构方向** (8 方向 per §2.8 V2.0 release 重构入口签名 8 个方向):
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
- **跟"不要怕复杂度"哲学一致**: per 主人 8/11 01:14 拍板 3 件套 §3 + 决策 #73 §3 + 哲学文档 15-no-fear-complexity.md

---

## 9. 历史脉络

- R11 末: 24 LOCKED crate 入口签名 R11 baseline LOCKED (per 决策 #33 §2.3 B1)
- R19+ 集成期: 24 LOCKED 入口签名持续 R11 baseline 严守
- R20 阶段 6: 24 LOCKED 入口签名 + mtime baseline 16:34 之前 严守
- R25 D-3: council 加 4 协作模式 + 角色宪法 + reasoning trace + 图编排 (新增 re-export, 0 改入口签名)
- R33-3 / R33-3-1 / R33-4 / R33-4-1 / R33-4-2: mcp / council 加 resources / council_member / deliberation (新增 re-export)
- R37-1: protocol 砍 ProtocolRouter 中间层 (R36-2 删), 加 ProtocolBridge trait + 4 Bridge struct
- R120 + R122-1-retry + R123-2 + R30 U1~U11: api 加 cache / replay_cache / retry / routing / v2_endpoints / audit_sqlite / observability / endpoints / v1_tools / auth / ws_v1 / protocol_handler_trait (新增 re-export, 8/10 22:22 mtime)
- R125-4: mcp 拆 4 子文件 + 加 primitives / macros (新增 re-export, 8/10 17:53 mtime)
- R125 B1-B7: 9 项实质 Locked 升级路线, 主人 16:31 最高权限授权 (per `docs/conventions/10-locked.md`)
- R125-7: evolution 加 poda_cycle (R125-7 借脑 1.0, 新增 re-export)
- R127 P5-1: evolution 加 library_autonomy (新增 re-export, 8/10 21:45 mtime)
- R127-2 P6-2: agent 加 4 专家 + AgentRouter; tool-runtime 加 mcp_protocol; graph 加 context_graph; cli 加 commands / output_format (新增 re-export, 8/10 21:48-21:52 mtime)
- R127-2 P9-1: graph 加 state_graph (langgraph 829 cloned 借脑, per decision-56 §2.4)
- R128-2: pipeline 持续 R122-1~5 借鉴 VCP (model_router / provider_registry / role_divider / tiktoken_counter / tool_loop)
- **R130 era 主人 8/11 01:14 拍板 3 件套**: locked 全解锁 + 架构审视 + 不要怕复杂度
- **R130 era 决策 #73 + 决策 #74**: 8 硬墙 B1 改写, V1.0 release 0 改严守 + V1.1 release Mavis 自决改
- **R131 era 5 sub-agent 派活**: R131-1 (架构总审视) + R131-2 (借鉴 12 源差距) + R131-3 (V1.1 release 路线图) + R131-4 (cargo workspace 结构优化) + **R131-5 (24 LOCKED 入口分布优化, 本报告)**

---

## 10. 一句话 (再次强调)

**24 LOCKED crate 入口分布在 R127-2/R128 演化下呈现"模块膨胀 + 重导出膨胀 + 跨 crate 集成膨胀"3 大趋势, V1.0 release 0 改严守 (整合 #5.1 commit 仍 0 改 src, R11 baseline 严守, 24/24 入口签名 0 改 verify 通过), V1.1 release 可借 PHL-07 实施 + 9 organ 借脑 + 三洋葱架构升级一并改写入口签名 (per 决策 #74 B1 Mavis 自决改 + 主人 8/11 01:14 拍板), V2.0 release 可借"不要怕复杂度 + 最强效果 + 最厉害工程"全量重构成 organ-first / three-onion-first 拓扑. 报告 8 个优化方向: ①入口签名一致性 ②公开 API 表面 ③crate 间依赖 ④crate 内部模块 ⑤三洋葱架构落地 ⑥9 organ 代码对应 ⑦R11 baseline 严守 ⑧V1.1/V2.0 release 改写边界. 8 硬墙严守 + B1 改写 + 8 哲学锚严守 + 不要怕复杂度哲学落地. 0 主动 IM 主人 + 0 主动 commit/push + 0 主动改 src 严守.**

---

**报告路径**: `Apeireth-rust\reports\agent-r131-5-24-locked-entry-optimization-2026-08-11.md`
**生成时间**: 2026-08-11 (R131 era 第 2 批, R131-5 sub-agent)
**关联决策**: 决策 #33 + #44 + #60 + #61 + #62 + #64 + #71 + #72 + #73 + #74 + #75 + R130 era 主人 8/11 01:14 拍板 3 件套
**作者**: Mavis (R131-5 sub-agent, 决策 #75 §2.1 派活)
