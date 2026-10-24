# Apeireth 代码实现盘点（实测版）

> 范围：`crates/` 全部 18 个包（foundation 6 / engine 8 / capabilities 1 / adapters 3）+ `frontend/companion-desktop`（含 `src-tauri`）。
> 排除：`crates/_archived/`、`legacy/`、`research/`、`target/`、`node_modules`。
> 方法：只读读码 + 脚本静态计数。一切结论以仓库当前代码为准；不引用任何宣称性文档作为证据。数字为静态计数（未运行 `cargo test`），测试数按 `#[test]` / `#[tokio::test]` 属性计数。

## 0. 口径与评级标尺

- **源文件数**：`src/**/*.rs`（含子目录；`tests/`、`examples/`、`benches/` 不计）。
- **代码行（不含测试）**：源文件总行数 − `#[cfg(test)]` 块内行数。
- **测试行**：`#[cfg(test)]` 块内行数 + `tests/**/*.rs` 全部行数。
- **pub API 数**：`pub fn/struct/enum/trait/type/const/static/union` 粗计。
- **测试数**：单测 = `src` 内 `#[test]` 数；集成 = `tests/` 内 `#[test]` 数。
- **评级**（宁严勿宽）：
  - 🟢 生产可用：接生产路径 + 有测试 + 默认生效；
  - 🟡 库级完备：实现 + 测试齐，但未接生产或默认关；
  - 🟠 实验：实现薄、测试少或契约对不上；
  - 🔴 占位：stub / `unimplemented!` / `todo!` 主体。

全仓合计（18 包）：源文件 466 个、源码 202,317 行（其中 `#[cfg(test)]` 56,185 行）、非测试代码 146,132 行；`tests/` 93 文件 36,443 行；测试总数 **4,101**（单测 3,219 + 集成 882）。

---

## 1. `crates/foundation/core`（apeireth-core）

1. **定位**：基础原语层：kernel ID/Clock/Episode/事件 + 双洋葱、13 哲学键门禁与 verdict cache 的库实现。
2. **规模**：源文件 19 / 非测试代码 5,383 行 / 测试 5,451 行 / pub API ≈247（单测 124、集成 224）。
3. **核心实现清单**
   - kernel 原语：`kernel/{ids,clock,event,memory,metadata,lifecycle,stream_kind,time}.rs`（SessionId/TraceId/ApprovalId/Clock/Episode/StreamKind）。
   - 13 键哲学清单 + 编译期断言：`src/philosophy.rs:99`（`ALL_THIRTEEN_KEYS: [PhilosophyKey; 13]` + `THIRTEEN_KEYS_HARDCODE`），`ALL_TWELVE_KEYS` 只是历史别名（`philosophy.rs:120-125`，实为 13 元素）。
   - verdict 路由：`src/lifecycle.rs:93` `verdict_for_target` const fn + `VerdictCache`（`philosophy.rs:223`）。
   - 自disable/元禁令：`src/lib.rs:1573` `SelfDisableAudit` + 反射白名单 + 编译期 const 断言（lib.rs:116 起）。
   - 双洋葱判定：`src/onion_gate.rs`（737 行 `DoubleOnionGate`）+ `src/onion.rs`（HumanAuthority/HA 模式 + 13 键 verdict 映射表）。
   - 会话/认知状态机：`src/statechart.rs`（564 行）、9 阶段生命周期（`src/lifecycle.rs`）。
4. **实现完整度评级**：🟡 库级完备——kernel 原语是 🟢 级（生产默认生效、测试足），但本包主打的哲学门禁/verdict 链是库级：`philosophy.rs:142` `RUNTIME_ENFORCED: bool = false`，`verdict_for_target`/`VerdictCache`/`SelfDisableAudit` 在生产执行链无调用点。
5. **接线状态**：kernel/clock 默认生效（`crates/engine/runtime-assembly/src/canonical/production.rs:10,263`、`crates/adapters/cli/src/lib.rs:382`）；`onion_gate` 经 `OnionLayerHook` 进治理管线但**默认关**（`APEIRETH_ENABLE_ONION_LAYER`，`cli/src/lib.rs:259-263`）；gate/verdict/self-disable 链未进执行主链（仅测试与示例引用）。
6. **测试**：单测 124 / 集成 224。关键：`pentest_gptfuzz_*`/`pentest_projectzero_*` 等 150+ 负向渗透用例（`tests/self_disable_q20_pentest.rs`）、`and_gate_semantics_v1v2v3_all_must_pass`（`tests/integration_v1v2v3.rs`）、`test_all_twelve_keys_complete`（`tests/verdict_keys.rs`）。
7. **问题清单**：TODO/FIXME 0；`unimplemented!/todo!` 0；非测试 `unwrap/expect` 1 处（`onion_gate.rs:557`）；死代码：`verdict_for_target`/`VerdictCache`/`SelfDisableAudit` 生产 0 引用（仅 core 自身测试）；薄实现：`RUNTIME_ENFORCED=false`（`philosophy.rs:142`）明示运行期不强制。

## 2. `crates/foundation/protocol`（apeireth-protocol）

1. **定位**：LLM 协议归一层：4 协议（OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini）↔ `Normalized*` 统一类型 + WS 8 帧契约 + retry/usage/acp 库件。
2. **规模**：源文件 23 / 非测试代码 5,151 行 / 测试 3,281 行 / pub API ≈232（单测 188、集成 27）。
3. **核心实现清单**
   - 归一化类型：`src/normalized.rs`（761 行，NormalizedRequest/Response/Message/ContentPart/ToolChoice）。
   - 交互契约：`src/canonical/{model,stream,tool_result}.rs`（ModelDescriptor/StreamEvent/ToolResult，runtime 直接消费）。
   - 4 协议适配：`src/adapters/{openai_chat,openai_responses,anthropic_messages,gemini}.rs` + `src/bridge.rs`（encode/decode dispatch）。
   - WS 8 帧：`src/ws_v1.rs`（596 行 + 编译期常量）+ `src/ws_session.rs`（auth-first 门/版本协商/帧方向）。
   - 重试语义：`src/retry.rs`（状态分级/backoff/jitter）。
   - 工具结果错误判定：`src/error.rs` `is_tool_result_error`。
4. **实现完整度评级**：🟢 生产可用（核心归一层）——`Normalized*/canonical` 类型在 runtime/provider/memory/gateway 全链默认生效、测试覆盖 4 协议往返；但 bridge/adapters/acp/usage 等约半数公有面为库级（见 7）。
5. **接线状态**：`NormalizedRequest/NormalizedMessage` 等全仓 257 处引用（provider 三家 canonical、runtime execute、memory context_window 等），**默认生效**；`ws_v1` 仅被 sdk 引用（19 处）；`bridge.rs`/`bridge_ext.rs`/`adapters/`/`acp.rs`/`usage.rs`/`ws_session.rs`/`gateway.rs(ProtocolKind)` crate 外 **0 引用**。
6. **测试**：单测 188 / 集成 27。关键：`bridge_dispatch_all_4_protocols`、`anthropic_request_requires_max_tokens`、`ws_frame_tool_invoke_json_roundtrip`。
7. **问题清单**：TODO 1（`src/p2p_mesh.rs:19`：TODO(v2.1) 接真加密，现为明文 `MeshPacket`）；`unimplemented!/todo!` 0；非测试 unwrap/expect 0；死面大：bridge/adapters/acp/usage/ws_session/ProtocolKind 全仓 0 外部引用（约 2,500+ 行）；`KEEP_ALIVE_*` 5 常量仅编译期 hardcode 无消费方（lib.rs:157-169 自述"战役 1-2 真用"）。

## 3. `crates/foundation/plugin`（apeireth-plugin）

1. **定位**：插件=能力提供者的统一模型：双注册表（PluginRegistry + CapabilityRegistry）+ 生命周期 + 各能力 trait 口。
2. **规模**：源文件 35 / 非测试代码 8,094 行 / 测试 4,045 行 / pub API ≈499（单测 272、集成 0）。
3. **核心实现清单**
   - 双注册表与索引：`src/registry.rs`（CapabilityRecord/PluginRegistry/CapabilityRegistry，id→owner 单一事实源）。
   - 生命周期与视图：`src/manager.rs`（922 行 PluginManager）。
   - 清单与边界校验：`src/manifest.rs` + `src/bounds.rs`（kebab-id/semver/权限/资源上限校验）+ `src/semver.rs`。
   - 工具能力契约：`src/tool.rs`（`ToolCapability`/`FrozenInvocation`）+ `src/capability.rs`。
   - 能力 trait 口 6 组：`memory_backend.rs`/`perception_backend.rs`/`organ.rs`/`experience.rs`/`preference.rs`/`self_assessment.rs`。
   - LLM 工厂桥：`src/llm_factory.rs` + `src/llm_orchestration_mirror.rs`（`MirrorLlmFactory`，plugin→orchestration 镜像）。
   - MCP 客户端协议族：`src/mcp/*` 11 文件（jsonrpc/sse/lifecycle/reconnect/subscribe/uri/schema/prompt/resource/discovery，约 3,600 行）。
   - 别名解析：`src/alias.rs`（别名索引 + LRU 缓存，22 个测试）。
4. **实现完整度评级**：🟢 生产可用（注册表/ToolCapability/trait 口部分）；`mcp/` 与 `watch.rs` 为库级（见 7）。
5. **接线状态**：`ToolCapability` 全仓 100 处引用（tools 各工具 + runtime capability 注册）**默认生效**；`PluginRegistry/CapabilityRegistry` 经 runtime builder/capability.rs 17 处引用默认生效；`MirrorLlmFactory` 经 cli（council/subagent）opt-in 路径使用；`src/mcp/*` 与 `watch::MetadataWatcher` crate 外 **0 引用**（tools 有自己的 `mcp.rs::McpClient`）。
6. **测试**：单测 272 / 集成 0（全部在 `src` 的 `#[cfg(test)]`）。关键：`register_and_get_by_id`（alias.rs）、`every_kind_has_a_distinct_prefix`（capability.rs）、`plugin_manifest_text_rejects_empty_description_and_loose_version`（bounds.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 2（`alias.rs:132`、`mcp/jsonrpc.rs:266`）；死面：`mcp/` 约 3,600 行 0 消费方、`watch.rs` 0 引用；0 集成测试（272 个测试全在 src 内）。

## 4. `crates/foundation/governance`（apeireth-governance）

1. **定位**：治理管线与策略：`GovernancePipeline`/`GovernanceHook` 框架 + 权限策略 + 审批策略 + 审计哈希链 + 输入安全。
2. **规模**：源文件 15 / 非测试代码 4,589 行 / 测试 1,980 行 / pub API ≈273（单测 119、集成 0）。
3. **核心实现清单**
   - Hook 框架：`src/lib.rs:319-505`（`GovernanceHook` trait、`Decision`/`GovernanceVerdict`、`GovernancePipeline` 聚合、`DenyUnconfigured` fail-closed 基元）。
   - 权限：`src/permission.rs`（PermissionPolicy grant/deny/require_approval）+ `PermissionGovernanceHook`（lib 根导出）。
   - 审批策略：`src/approval_policy.rs`（命令级匹配/频率限次/黑白名单/风险前缀）。
   - 审计账本：`src/audit.rs`（哈希链追加 + 篡改/重排/删中段检测）。
   - 输入安全：`src/input_security.rs`（PromptInjectionHook）+ `src/untrusted_mark.rs`。
   - 风险/证据/评测：`src/risk.rs`、`src/evidence.rs`（CredentialDisclosureHook 所在）、`src/{eval,rubric,tool_desc_audit}.rs`。
   - CoLang DSL：`src/colang.rs`（1,044 行，规则解析 + guard）。
4. **实现完整度评级**：🟢 生产可用（管线 + 权限 + 审批策略 + 审计链默认进生产）；colang/rubric/eval/research_autonomy 为库级（nightwatch 显式命令消费 colang/eval/rubric）。
5. **接线状态**：生产治理管线 = `Permission → CredentialDisclosure → PromptInjection → BehaviorChainGuard(→ 可选 OnionLayer)`（`crates/adapters/cli/src/lib.rs:250-264`），**每个回合默认生效**；`APEIRETH_ENABLE_ONION_LAYER=1` 才追加洋葱层；`research_autonomy.rs`/`tool_desc_audit.rs`/`untrusted_mark.rs` crate 外 0 引用。
6. **测试**：单测 119 / 集成 0。关键：`blacklist_beats_trust`/`frequency_denies_on_third_call`（approval_policy.rs）、`payload_tamper_fails`/`reorder_fails`（audit.rs）、`guard_blocks_harm_user`（colang.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 16（`audit.rs:142`、`colang.rs:257,333,335,355,376` 等）；死代码：`research_autonomy.rs`（543 行）全仓 0 引用；0 集成测试（无 `tests/`，生产管线组合正确性由 cli/gateway 侧测试间接覆盖）。

## 5. `crates/foundation/credentials`（apeireth-credentials）

1. **定位**：凭据存储与解析：OS keyring / 加密文件 / 内存三级后端 + 自动降级 + 脱敏审计。
2. **规模**：源文件 8 / 非测试代码 2,404 行 / 测试 1,528 行 / pub API ≈96（单测 67、集成 29）。
3. **核心实现清单**
   - Keyring 抽象与 3 实现：`src/keyring.rs`（1,835 行：`KeyringBackend` trait、`PlatformKeyring`（keyring crate 3.6）、`EncryptedFileBackend`（chacha20poly1305 + master.key + 0600 原子写）、`InMemoryKeyring`）。
   - 后端选择与降级：`KeyringSelector::select_or_fallback`（`keyring.rs:1254-1370`，auto→Platform→EncryptedFile→InMemory）。
   - 脱敏审计：`FileAuditSink`（JSONL 落档）+ `name_hash`（SHA-256(service)[:16]，可加盐，`keyring.rs:37-42`）。
   - 明文文件库：`src/store.rs`（FileCredentialsStore，靠 OS 权限边界）。
   - 敏感值包装：`src/secret.rs`（Secret，Drop 零化）。
   - 治理钩子：`src/hook.rs`（凭证披露拦截）+ `src/gate.rs`。
4. **实现完整度评级**：🟢 生产可用。
5. **接线状态**：`crates/adapters/cli/src/keyring_bootstrap.rs` `build_keyring_resolver()` 注入 `RuntimeBuilder::with_credentials`（`cli/src/lib.rs:413-415`），**默认生效**（`APEIRETH_KEYRING_BACKEND` 选后端，缺省 auto）；gateway `/v1/admin/config` 热改 keyring 写入同后端（`cli/src/lib.rs:1499`）。
6. **测试**：单测 67 / 集成 29。关键：`file_store_persists_across_instances`（store_integration.rs）、`migrate_then_decrypt_with_same_key`（memory 侧同模式）、keyring.rs §9.2 EncryptedFileBackend round-trip 系列。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 0；薄/风险：master.key 与密文同目录（`keyring.rs:32-36` 自述边界）；平台 keyring 无 list API（`keyring.rs:674`）。

## 6. `crates/foundation/orchestration`（apeireth-orchestration）

1. **定位**：编排层库件：七席 Council、subagent 链、worktree 沙箱，以及一批未接线的认知调度/上下文算法。
2. **规模**：源文件 30 / 非测试代码 8,058 行 / 测试 5,067 行 / pub API ≈364（单测 240、集成 13）。
3. **核心实现清单**
   - 七席 Council：`src/council/mod.rs` + `src/council/advisors_llm.rs`（7 个 LlmAdvisor 并行真 LLM 审议、一票否决、超时 defer-to-human）。
   - subagent 编排：`src/subagent_llm.rs`（LlmSubagentOrchestrator，plan→impl→review）+ `src/worktree_sandbox.rs`（731 行 git worktree 物理隔离装饰器）。
   - 持久化/重放：`src/durable/{history,replay,retry}.rs`（history 474 + replay 922 + retry 433）。
   - 上下文算法：`src/context_fold/`（5 文件 1,261 行折叠）、`src/context_budget.rs`、`src/context_rot.rs`。
   - 调度族：`src/cron.rs`（922 行）、`src/cognitive_quota_scheduler.rs`（499 行）、`src/ambient_context.rs`（416 行）、`src/speech_arbiter.rs`、`src/care_potential_field.rs`、`src/lineage_spawning.rs`。
   - 研究侧：`src/research_{context_policy,vault_ftrl,cost_ledger}.rs`（约 1,500 行）。
4. **实现完整度评级**：🟡 库级完备——Council 与 subagent/worktree 有真实现且可经显式命令/开关到达；其余大半（cron/durable/context_fold/ambient/speech/quota/care/lineage/research_*）全仓 0 生产引用。
5. **接线状态**：Council 仅 `APEIRETH_COGNITIVE_COUNCIL=1`（cli 构造，`cli/src/lib.rs:715-721`）且默认关；subagent 仅 `apeireth subagent` 显式命令（`cli/src/lib.rs:810`，含 y/N 人工审批门）；worktree 仅 `APEIRETH_ENABLE_WORKTREE_SANDBOX=1` 默认关；`CronScheduler`/`durable`/`context_fold`/`AmbientContext`/`SpeechArbiter`/`CognitiveQuotaScheduler`/`CarePotentialField`/`LineageSpawning` crate 外 0 引用。
6. **测试**：单测 240 / 集成 13。关键：`council_all_seven_advisors_call_llm`、`council_one_advisor_veto_overrides_others`、`council_short_timeout_triggers_defer_to_human`（tests/council_llm.rs）；`council_7_advisor_live_decide`（tests/council_live.rs，真 LLM）。
7. **问题清单**：TODO 5（`ambient_context.rs:183,407`、`cognitive_quota_scheduler.rs:182`（FIFO 淘汰缺 submit_seq）、`lineage_spawning.rs:30,105`（ed25519 未接真签名））；非测试 unwrap/expect 7（`lib.rs:743,784`、`semantic.rs:206`、`advisors_llm.rs:86,110,147` 等）；死代码：上列 0 引用模块合计约 4,000+ 行；`lineage_spawning` 签名字段可被 mock 串伪造（TODO 自述）。

## 7. `crates/engine/runtime`（apeireth-runtime）

1. **定位**：canonical 运行时内核：回合执行器（agent loop）、审批生命周期、provider 路由、模块注册表、会话管理。
2. **规模**：源文件 16 / 非测试代码 7,126 行 / 测试 8,146 行 / pub API ≈251（单测 63、集成 89）。
3. **核心实现清单**
   - 回合执行器：`src/canonical/execute.rs`（2,359 行：多轮 loop、`max_rounds` 上限、单轮 tool_calls 上限（M20）、逐动作治理授权、trace 全链、streaming sink）。
   - 审批生命周期：`src/canonical/approval.rs`（736 行：冻结调用指纹、claim-before-invoke 持久化、崩溃后不重放、过期/双批/并发批语义）。
   - Provider 路由：`src/canonical/provider.rs`（746 行：ProviderRouter、健康态、fallback order、按 model 路由）。
   - 运行时装配与快照：`src/canonical/runtime.rs`（RuntimeBuilder/RuntimeSnapshot/RuntimeHealthSnapshot）。
   - 模块系统：`src/canonical/module.rs`（716 行：ModuleRegistry/ModuleInvoker/invocation 预算/调用深度上限）。
   - 会话：`src/canonical/session.rs`（SessionManager/SessionSettings/PermissionPreset）。
   - 子循环：`src/canonical/subloop.rs`（私有转录 + 能力白名单）；心跳/流锁 `heartbeat.rs`；审批状态机 `research_approval_sm.rs`（830 行，研究侧，含 3 个 Kani proof）。
4. **实现完整度评级**：🟢 生产可用（全产品唯一执行主链）。
5. **接线状态**：`Runtime::builder → execute_outcome/execute_outcome_streaming` 被 cli（chat/gateway serve/approval）与 gateway（`/v1/chat*`）**默认调用**；无开关（始终生效）。
6. **测试**：单测 63 / 集成 89。关键：`minimal_tool_call_agent_loop_closes_end_to_end`、`crash_after_claim_before_invoke_never_re_executes`、`the_loop_falls_back_to_a_second_provider_and_records_why`（tests/canonical_agent_loop.rs、canonical_approval_lifecycle.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 9（`approval.rs:388`、`provider.rs:343`、`research_approval_sm.rs:190,581,593,594` 等）；`research_approval_sm` 自述不挂生产审批路径（库级+测试）；集成测试 6,656 行 > 非测试代码 7,126 行（测试密度高，属正面项）。

## 8. `crates/engine/runtime-assembly`（apeireth-runtime-assembly）

1. **定位**：唯一生产组合根：认知模块/工具模块/organ 的装配与注册序（production.rs），外加各具体生产模块实现。
2. **规模**：源文件 22 / 非测试代码 8,384 行 / 测试 11,588 行 / pub API ≈295（单测 87、集成 91）。
3. **核心实现清单**
   - 组合根：`src/canonical/production.rs`（593 行：`ProductionModulesConfig` 22 个开关字段 + `ProductionBackends` 22 个注入槽 + `build()` 固定注册序/重复 id 拒绝/缺后端 boot 失败）。
   - 认知模块族：`src/canonical/cognitive.rs`（3,357 行：MemoryRecall/MemoryWriteback/PreferenceRecall/Judge/SelfAssessment/Council/Reflexion/AbsorptionInsight/PartnerBond + CognitiveTelemetry）。
   - organ 生产属主：`src/canonical/organ_module.rs`（唯一 organ 注册点，W1/W2 每回合瞬态构造 + `organ_llm_bridge.rs` invoker→LlmFactory 桥）+ `src/canonical/orchestrator.rs`（1,871 行：9 organ 串行 + 8 gate + 5 状态机）。
   - typed 记忆写读对称：`src/canonical/memory_typed_sink.rs`（写侧）+ `src/canonical/typed_recall.rs`（读侧 SqliteTypedMemoryRecallSource）。
   - 工具注册：`src/canonical/tool_modules.rs`（Filesystem/Search/Repo/Shell/Fetch/Mcp/Education 模块）+ `src/canonical/preference_learning.rs`。
   - 治理与观测辅助：`src/canonical/permission_preset.rs`（read_only/standard/full 会话档）、`guard_observer.rs`（dataset 闭环）、`onion_layer.rs`（L3-L5 末层 hook）、`nightwatch.rs`（闲时审计）。
   - 周期/实验：`upgrade_cycle.rs`（L0-L5 自升级驱动）、`dream_llm.rs`（LLM 思考器+确定性降级）、`experiment_field.rs`、`sqlite_session.rs`。
4. **实现完整度评级**：🟢 生产可用。
5. **接线状态**：`ProductionCognitiveModules::build` 是**唯一**生产装配点，被 `crates/adapters/cli/src/lib.rs:758` 调用（cli chat / gateway serve 同源）；各模块默认：memory_recall/writeback/preference_recall/self_assessment/filesystem/search/repo = 开，judge/council/organs/preference_learning/memory_injection/consolidation/reflexion/partner_bond/morphology/community_triage/education/absorption_insight = 关（详见 §21.2）。`experiment_field.rs`/`harness_patch.rs` crate 外 0 引用。
6. **测试**：单测 87 / 集成 91。关键：`organ_chain_runs_only_at_afterturn`/`denied_organ_side_calls_fail_open_with_zero_provider_calls`（tests/canonical_organ_module.rs）、`turn1_learning_reaches_turn2_provider_context`（tests/canonical_preference_learning.rs）、`production_memory_module_recalls_after_restart_and_honors_forget`（tests/cognitive_vnext_production.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 11（`cognitive.rs:1828,1835,1843,1997,2243` 等、`causal_world_model.rs:110`）；死代码：`experiment_field.rs`（450 行）/`harness_patch.rs`（160 行）0 外部引用；`partner_store` 注入的是 `InMemoryPartnerStore`（`cli/src/lib.rs:751-755`），重启即散。

## 9. `crates/engine/guard`（apeireth-guard）

1. **定位**：两段式行为链安全分类：Fast Guard（确定性 Stage A）+ Behavior Chain Guard（Stage B 复合风险/外泄追踪）+ ML 分类器与数据集。
2. **规模**：源文件 21 / 非测试代码 10,243 行 / 测试 1,975 行 / pub API ≈215（单测 31、集成 44）。
3. **核心实现清单**
   - 生产钩子：`src/hook.rs`（689 行 `BehaviorChainGuardHook`：治理 hook + 跨轮摘要 + 事件观测）。
   - Stage A：`src/fast_guard.rs`（破坏性命令/只读 scope 拒绝）+ `src/intent.rs`（872 行 RuleIntentInterpreter + 否定感知抽取）。
   - Stage B：`src/chain.rs`（行为链图）+ `src/chain_guard.rs`（敏感源→外部 sink / 重试升级检测）+ `src/session.rs`（跨轮历史）。
   - ML 分类：`src/classifier.rs`（JointRiskClassifier/JSON 工件 sha256 校验/校准/shadow-advisory-enforce 三模式）+ `features.rs`/`features_v2.rs`/`fusion.rs`/`snapshot.rs`（特征快照与模型共享）。
   - 数据集：`src/dataset.rs`（脱敏 JSONL v3 + 录制开关）。
   - 语义与场景：`src/semantics.rs`（能力安全描述符注册表）、`src/scenario.rs`（3,853 行场景 DSL + 大目录 + oracle 标注）、`src/command_effect.rs`（shell 效果语义）。
4. **实现完整度评级**：🟢 生产可用（hook 默认进生产治理管线）。
5. **接线状态**：`BehaviorChainGuardHook` 在 `cli/src/lib.rs:243-256` 挂入治理管线，**默认生效**；ML 分类器默认 `NoClassifier`（`APEIRETH_GUARD_ML_MODE`+`APEIRETH_GUARD_ML_MODEL` 才启用，`cli/src/lib.rs:286-304`）；数据集默认关（`APEIRETH_GUARD_DATASET_ENABLED`）。
6. **测试**：单测 31 / 集成 44。关键：`test_chain_guard_detects_sensitive_source_to_external_sink`、`classifier_and_dataset_share_exact_feature_snapshot`、`scenario_dsl_runs_real_extractor_and_catalog_is_large`。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 4（`scenario.rs:373,406,409,413`）；死代码（crate 外 0 引用）：`EnforcementDirective`（enforcement.rs）、`CommandEffectAnalyzer`、`ScenarioOracle` 等仅 crate 内/测试使用；`scenario.rs` 3,853 行大半是场景目录资产而非运行逻辑。

## 10. `crates/engine/provider`（apeireth-provider）

1. **定位**：三家 canonical LLM provider（Anthropic / MiniMax / OpenAI-compatible）+ embeddings + LLM 工厂。
2. **规模**：源文件 12 / 非测试代码 3,458 行 / 测试 3,493 行 / pub API ≈101（单测 107、集成 48）。
3. **核心实现清单**
   - `src/canonical_anthropic.rs`（851 行：Anthropic Messages 真 HTTP、401/429/500/超时→永久/可重试错误分类、max_tokens/stop_reason 映射）。
   - `src/canonical_minimax.rs`（703 行）与 `src/canonical_openai_compatible.rs`（1,012 行）：OpenAI-chat 兼容 + Bearer。
   - `src/openai_chat.rs`（625 行：请求构造/解析/retry-after 归一，三 provider 共用）。
   - `src/embeddings.rs`（OpenAI-compatible embeddings；URL/MODEL 双缺→None、半配→报错）。
   - `src/{minimax,openai_compatible}_llm_factory.rs`（LlmFactory → Council/subagent/dream 用）。
   - `src/provider_model.rs`（wire 模型名映射）、`src/credentials.rs`。
4. **实现完整度评级**：🟢 生产可用。
5. **接线状态**：`cli/src/lib.rs:437-457` **默认注册** minimax + anthropic；openai-compatible 由 `APEIRETH_OPENAI_MODELS` 非空触发；`fallback_order` 默认生效；embedding provider 由 `APEIRETH_EMBEDDING_URL/MODEL` 双全才接（fail-closed）。
6. **测试**：单测 107 / 集成 48。关键：`request_conversion_sends_anthropic_envelope_not_openai`、`missing_credential_fails_permanently_without_network`、`http_429_maps_to_rate_limited_retryable`（tests/canonical_anthropic.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 0；死代码：`src/reasoning_adapter.rs`（548 行）全仓除自身 0 引用，未接进任何 canonical provider；tests 里有 `openai_compatible_live.rs`/`organ_live_llm` 类真网络 smoke（需密钥，CI 外）。

## 11. `crates/engine/storage`（apeireth-storage）

1. **定位**：存储地基：SQLite 连接池/迁移 + 缓存、限速、配额、机器码等库原语。
2. **规模**：源文件 15 / 非测试代码 4,110 行 / 测试 1,850 行 / pub API ≈251（单测 115、集成 15）。
3. **核心实现清单**
   - `src/pool.rs`（415 行 SqliteConnectionPool：WAL、写串行、panic 存活写线程）。
   - `src/migrations.rs`（261 行版本化幂等迁移 + LATEST_SCHEMA_VERSION）。
   - `src/cache/`（6 文件 1,895 行：LRU+TTL+shard+evictor+stats）。
   - `src/rate_limit/`（3 文件 1,985 行：TokenBucket/LeakyBucket/retry）。
   - `src/quota.rs`（324 行快照配额）、`src/machine_id.rs`（555 行机器码探测）。
4. **实现完整度评级**：🟢 生产可用（pool + migrations）；cache/rate_limit/quota/machine_id 为库级（见 7）。
5. **接线状态**：`SqliteConnectionPool` + `run_migrations` 被 cli（cognitive.sqlite3 / sessions.sqlite3）与 memory 全部 sqlite store 使用，**默认生效**；`cache`/`rate_limit`/`quota`/`machine_id` crate 外 0 引用（tools/fetch 自带 `fetch/rate_limit.rs`）。
6. **测试**：单测 115 / 集成 15。关键：`writer_thread_survives_a_panicking_task`、`migrations_are_versioned_and_idempotent_on_file_db`、`cache_lru_ttl_and_shard_roundtrip`（tests/storage_foundation.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 1（`rate_limit/mod.rs:492` expect）；死代码：`machine_id.rs`（555 行）等 0 外部引用。

## 12. `crates/engine/memory`（apeireth-memory）

1. **定位**：记忆系统全栈：SQLite 记忆库 + 六流历史 + 混合检索 + 治理遗忘 + 记忆闭环 + 做梦/日记 + 一批实验算法。
2. **规模**：源文件 129 / 非测试代码 35,995 行 / 测试 17,006 行 / pub API ≈1,418（单测 787、集成 67）——全仓最大包。
3. **核心实现清单**
   - 存储与迁移：`src/backend/{sqlite,file,file_encrypted,in_memory}.rs`、`src/migrations.rs`（1,230 行）、typed stores（`commitments.rs`/`persona_store_sqlite.rs`/`temporal_graph_store.rs`/`experience_store_sqlite.rs`/`preference_store_sqlite.rs`/`self_assessment_store_sqlite.rs`）。
   - 统一协调器：`src/coordinator.rs`（1,009 行 MemoryCoordinator：混合召回/写回/注入格式/typed 召回想定）。
   - 检索链：`src/hybrid_search.rs`（BM25+向量）、`src/retrieval_pipeline.rs`、`src/context_window.rs`（上下文投影）、`src/access_history.rs`、`src/extraction.rs`+`src/memory_materializer.rs`。
   - 治理遗忘：`src/memory_governance.rs`（862 行 forget/protect/override）、`src/universal_forget.rs`、`src/forget_coordinator.rs`、`src/derived_repair.rs`。
   - 记忆闭环：`src/reflexion.rs`（964 行失败沉淀）、`src/consolidation.rs`、`src/memory_injection.rs`、`src/proactive_recall.rs`。
   - 做梦/日记：`src/dreaming.rs`（6 阶段循环）+ `src/dream_wiring.rs` + `src/meta_thinking.rs` + `src/diary.rs`。
   - 账本/身份：`src/context_ledger.rs`（onering 账本）、`src/continuity_link.rs`、`src/session_lifecycle.rs`、`src/identity.rs`。
   - 实验算法族：`betti_hole_detector.rs`/`kuramoto_resonance.rs`/`residual_pyramid.rs`/`river_topology.rs`/`semantic_axis.rs`/`five_dimensional.rs`/`three_tier_vault.rs`/`wiki_fs.rs`/`amem_graph.rs`/`bitemporal_graph.rs`/`community.rs`；研究族 `research_derived_memory.rs`（1,211 行）/`research_roaming_memory.rs`/`research_non_interference.rs`/`admission_gate.rs`。
4. **实现完整度评级**：🟢 生产可用（存储/召回/写回/治理部分）；实验与 research 族为库级（见 7）。
5. **接线状态**：`MemoryCoordinator`+MemoryRecall/Writeback+typed stores+context_window+access_history 经 `production.rs:328-366,369-519` **默认生效**；reflexion/consolidation/injection/proactive/community 各默认关（开关见 §21.2）；做梦仅 `apeireth dream` 显式命令（`cli/src/lib.rs:1231`）；onering 账本仅 `APEIRETH_ENABLE_ONERING_LEDGER`；`hallways.rs`/`research_roaming_memory.rs` 全仓 0 引用；`dailynote/`/`layered_memo/`（含自带 MCP server 形态）无生产调用点。
6. **测试**：单测 787 / 集成 67。关键：`test_governance_forget_strictly_excluded_from_recall`、`test_writeback_and_multi_layer_recall`（tests/memory_2_tests.rs）、`end_to_end_six_streams_independent`（tests/integration_six_streams.rs）。
7. **问题清单**：TODO 0（`tests/universal_forget.rs:15` 出现的小写 `todo` 是 SQL 测试夹具值，非标记）；`unimplemented!/todo!` 0；非测试 unwrap/expect 24（`arbitration.rs:182,217,292`、`coordinator.rs:996,1000`、`context_window.rs:60` 等）；死面：`hallways.rs`（740 行）、`research_roaming_memory.rs`（295 行）0 引用；`dailynote/`+`layered_memo/`（约 4,000 行）无产品路径；向量检索为暴力全扫描余弦（`canonical/vector.rs`），无 ANN 索引。

## 13. `crates/engine/perception`（apeireth-perception）

1. **定位**：感知算法层：语音（STT/VAD/会话）、截屏视觉、事件归一化——全部未接生产。
2. **规模**：源文件 17 / 非测试代码 3,918 行 / 测试 2,401 行 / pub API ≈202（单测 130、集成 10）。
3. **核心实现清单**
   - 语音：`src/voice/whisper_http.rs`（1,147 行真 HTTP STT）、`energy_vad.rs`（能量 VAD）、`audio_frame.rs`/`stream_frame.rs`（PCM16 分帧/缓冲）、`audio_session.rs`（录音会话状态机）、`emotion_voice.rs`、`minimax_tts.rs`（178 行，仅请求构造，无 HTTP 实调）。
   - 视觉：`src/vision/xcap_backend.rs`（757 行 xcap 截屏）+ `vision/noop.rs`。
   - 归一化管线：`src/normalize.rs`（471 行 5 模态观测归一）、`src/capture.rs`、`src/observe.rs`（ObservationQueue）、`src/screen.rs`。
   - 属主编排：`src/owner.rs`（PerceptionOwner，多模态管线端到端）。
4. **实现完整度评级**：🟡 库级完备（实现 + 测试齐，但全包未接生产）。
5. **接线状态**：**0 生产接线**——`apeireth_perception` 在 crates/ 内仅被自身与 `apeireth_plugin::perception` trait 引用；runtime-assembly/cli/gateway 无一处引用；`owner.rs` 自述 default-off and unwired（`src/lib.rs:12-13`）。
6. **测试**：单测 130 / 集成 10。关键：`perception_voice_and_vision_backends_wire_cleanly`、`enabled_owner_runs_end_to_end_multimodal_pipeline`、`disabled_owner_is_the_default_production_path`（tests/perception_integration.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 0；主要缺口：整包未进任何生产路径（约 3,900 行实现 + 2,400 行测试无产品消费方）；`minimax_tts` 无实际合成调用（仅 `build_request`）。

## 14. `crates/engine/organ`（apeireth-organ）

1. **定位**：9 器官认知实现（好奇心/假设/价值案例/情绪记忆/世界模型/因果世界模型/因果边挖掘/涌现/记忆合并）+ 检索形态学等配套。
2. **规模**：源文件 17 / 非测试代码 8,662 行 / 测试 6,557 行 / pub API ≈434（单测 179、集成 44）。
3. **核心实现清单**
   - 9 organ 实现：`curiosity.rs`（645 行）、`hypothesis.rs`（831）、`value_cases.rs`（765）、`emotion_memory.rs`（714）、`world_model.rs`（1,336）、`causal_world_model.rs`（1,864，含 LLM 反事实/MCTS）、`causal_world_model_edges.rs`（945，确定性边挖掘）、`emergence.rs`（1,218，8 门控）、`memory.rs`（782，记忆合并）。
   - 检索形态学：`src/morphology.rs`（查询形态→检索深度/温度）。
   - 配套子系统：`goal.rs`（887 行 GoalService）、`prompt_assembly.rs`（941 行 PromptAssembler）、`context_assembly.rs`、`tone.rs`、`motivation.rs`、`experience_growth.rs`。
   - 占位兜底：`NoopOrgan`（lib.rs:105，显式返 `OrganError::NotImplemented`）。
4. **实现完整度评级**：🟡 库级完备——9 organ 有真实现 + 单/集成/真 LLM smoke 测试，但生产仅经默认关的开关可达，且 6 个配套子系统未接线。
5. **接线状态**：`OrganModule` 仅 `APEIRETH_ENABLE_ORGANS=1` 注册（`production.rs:458-460`，默认关，生产实测 `organ_module.rs:124-135` 串 9 organ、W1/W2 每回合瞬态）；`morphology` 仅 `APEIRETH_ENABLE_MORPHOLOGY_RECALL`（经 `cognitive.rs:973`）；`goal/tone/motivation/prompt_assembly/context_assembly/experience_growth` crate 外 0 引用。
6. **测试**：单测 179 / 集成 44。关键：`w2_w3_pipeline_mining_then_simulate`、`emergence_organ_should_speak_respects_rate_limit_and_idle`、`w1_world_model_simulate_live`（tests/organ_live_llm.rs，真 LLM）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 1（`emergence.rs:336`）；死代码：6 个配套子系统约 3,000 行 crate 外 0 引用；`NoopOrgan` 为占位（诚实返回 NotImplemented）。

## 15. `crates/capabilities/tools`（apeireth-tools）

1. **定位**：内置工具能力：本地只读三件套、shell、fetch、进程遏制、补丁、egress、MCP 客户端等。
2. **规模**：源文件 29 / 非测试代码 10,921 行 / 测试 5,444 行 / pub API ≈273（单测 208、集成 67）。
3. **核心实现清单**
   - shell：`src/shell.rs`（1,098 行 TrustedShellConfig + 沙箱执行）+ `src/bin/sandbox_test_child.rs`。
   - 进程遏制：`src/process/`（6 文件 3,078 行：Windows Job Object + AppContainer（`windows.rs` 1,091 / `appcontainer.rs` 414）、Linux no_new_privs/组树、macOS 组树、平台能力报告 fail-closed）。
   - fetch：`src/fetch.rs` + `src/fetch/`（4 文件 1,589 行：受控 egress、redirect 前置判定、响应上限/超时/限速/缓存、HTML 可访问性摘要）。
   - 本地只读：`src/filesystem.rs`、`src/search.rs`、`src/repo.rs`、`src/repo_map.rs`。
   - `src/apply_patch.rs`（940 行事务补丁 + 回滚报告）、`src/egress.rs`（1,018 行允许清单）、`src/guardrail.rs`（凭据泄漏 tripwire）、`src/spill.rs`（长输出落盘）。
   - `src/mcp.rs`（McpClient）、`src/education.rs`（DxCheck 工具）、`src/std_sub_supervisor.rs`（真启进程）/`src/supervisor.rs`（Noop）。
4. **实现完整度评级**：🟢 生产可用。
5. **接线状态**：filesystem/search/repo 经 `production.rs:286-305` **默认注册**，`tool.repo` 恒 grant、`tool.filesystem/search` 默认 grant（`cli/src/lib.rs:236-241`，`APEIRETH_DISABLE_LOCAL_READ_TOOLS=1` 可关）；shell/fetch 仅 `APEIRETH_ENABLE_SHELL/FETCH=1`（注册 + grant + require_approval，`cli/src/lib.rs:336-354`）；education 仅 `APEIRETH_ENABLE_EDUCATION`；mcp 需 `config.mcp`（无 env 旋钮，默认 false）。
6. **测试**：单测 208 / 集成 67。关键：`process_executor_attaches_child_to_a_real_job_object`/`kill_on_job_close_terminates_a_running_child`（tests/process_executor.rs）、`sandbox_denies_workspace_external_reads`（tests/shell_execution.rs）、`private_loopback_is_denied_under_public_internet_only_before_contact`（tests/fetch_execution.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 22（`education.rs:302`、`fetch.rs:187`、`filesystem.rs:70`、`plugin.rs:54,60,64` 等）；死代码：`StealthCrawlerEngine`（`stealth_crawler.rs`）全仓 0 引用；`StdSubSupervisor` 仅以字符串能力名出现在 `permission_preset.rs:49-50`/`onion_layer.rs:41` 分类表，无构造点（与 gap-matrix"零件齐而树不存在"一致）；`ToolGuardrail`/`SpillStore` crate 外 0 引用。

## 16. `crates/adapters/gateway`（apeireth-gateway）

1. **定位**：HTTP 网关适配器：canonical chat/审批/面板/SSE 事件/热配置路由面。
2. **规模**：源文件 13 / 非测试代码 4,883 行 / 测试 3,905 行 / pub API ≈170（单测 35、集成 32）。
3. **核心实现清单**
   - 路由与执行面：`src/canonical_entry.rs`（1,145 行：`/health`、`/v1/models`、`/v1/providers`、`/v1/runtime/snapshot`、`/v1/chat`、`/v1/chat/completions`（OpenAI 兼容 + SSE 流）、`/v1/approvals`、`/v1/approvals/resolve`（token 门）、`/v1/sessions/:id/settings`、`/v1/apeireth/events`（SSE）、`/v1/admin/config`（热改 + token 门）；`execute_chat` 委托 `runtime.execute_outcome`，不绕过运行时）。
   - 面板数据面：`src/panels.rs`（1,241 行 `/v1/panel/*` + `/v1/tools/list` + `/v1/apeireth/capabilities`，无后端时 501 降级）。
   - 事件总线：`src/events.rs`（EventBus + SSE 订阅 + `RuntimeObservationSink` trace/audit 归档）。
   - 热配置：`src/admin.rs`（provider base_url/凭据热改，下一请求生效）。
   - 存在感：`src/presence.rs`（829 行 PresenceSynthesizer/PresenceService + 心跳，接 EventBus）+ `ember_hud_driver.rs`（呼吸参数）。
   - 会话设置：`src/session_settings.rs`；错误契约：`error_frame.rs`/`error_codes.rs`。
4. **实现完整度评级**：🟢 生产可用。
5. **接线状态**：`apeireth gateway serve` → `serve_canonical_with_services`（`cli/src/lib.rs:1516`），面板数据由 `CliPanelData` 注入；presence 接入 gateway state（`canonical_entry.rs:449`、`session_settings.rs:192`）；默认绑定 loopback（非 loopback 打警告，`cli/src/lib.rs:1509-1515`）。
6. **测试**：单测 35 / 集成 32。关键：`real_http_entry_closes_the_canonical_tool_loop`、`http_pending_approval_can_be_approved_without_double_execution`（tests/canonical_entry_e2e.rs）、`panel_routes_degrade_to_501_without_backends`（tests/panel_routes.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 16（`barge_in.rs:65-142` 一批 Mutex unwrap 等）；死代码：`barge_in.rs`/`duplex_gateway.rs`/`file_fetcher.rs` 除自身测试与 lib re-export 外无调用点、无路由暴露（全仓也无 WebSocket 服务端）；**无 `/v1/tools/{tool}/invoke` 路由**（见 §21.5 新发现）。

## 17. `crates/adapters/cli`（apeireth-cli）

1. **定位**：产品组合根 + 命令行入口：唯一调用 `ProductionModules::build` 的装配方。
2. **规模**：源文件 5 / 非测试代码 3,511 行 / 测试 2,199 行 / pub API ≈48（单测 28、集成 40）。
3. **核心实现清单**
   - 命令面：`src/main.rs`（840 行：`session`/`chat`/`approve`/`reject`/`cancel`/`gateway serve`/`dream`/`council`/`subagent`/`nightwatch` 10 命令 + 参数解析测试）。
   - 组合根：`src/lib.rs`（1,612 行：`build_canonical_runtime_with_sessions_from_env` → 认知库 schema 校验/migrations/typed stores → `ProductionCognitiveModules::build` → 3 provider 注册 + fallback order + guard dataset 观测）。
   - 治理组装：`build_production_governance_parts_from_env`（lib.rs:322-356：权限 grant + 4 hook 管线 + shell/fetch 旋钮）。
   - 面板数据面：`src/gateway_panels.rs`（1,119 行 CliPanelData：会话/记忆/授权/审计/trace 归档）。
   - keyring 引导：`src/keyring_bootstrap.rs`；便携包：`src/portable_bundle.rs`。
4. **实现完整度评级**：🟢 生产可用。
5. **接线状态**：**全产品唯一生产装配入口**（`cli/src/lib.rs:758` 是 `ProductionCognitiveModules::build` 的唯一非测试调用方）；chat/approve/gateway serve 三命令共用同一 runtime 构造；开关解析全部在本包（见 §21.2）。
6. **测试**：单测 28 / 集成 40。关键：`the_cli_bootstrap_registers_both_canonical_providers_and_serves_minimax`、`typed_recall_defaults_on_and_identity_is_stable`、`shell_knob_registers_tool_and_requires_approval`（tests/production_knobs.rs）。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 4（`lib.rs:345` policy 锁 `expect`、`lib.rs:440,445,455` `CapabilityId::new(..).unwrap()`）；`main.rs:12` 的 help 文本只列 session/chat/approve/reject/cancel/gateway，未列 dream/council/subagent/nightwatch（命令存在但无帮助项）。

## 18. `crates/adapters/sdk`（apeireth-sdk）

1. **定位**：多语言 SDK 客户端面：HTTP 工具调用客户端 + Wire/版本/错误码 + FFI 桥 + 4 个 feature-gated 子 SDK。
2. **规模**：源文件 40 / 非测试代码 11,242 行 / 测试 6,712 行 / pub API ≈662（单测 439、集成 42）。**注意**：其中 30 文件/约 13,300 行属 `lark/livekit/sandbox/voice` 4 子模块，`default features = []` 下**不编译**；默认编译面约 10 文件/3,500 行。
3. **核心实现清单**
   - 客户端：`src/client.rs`（1,553 行：`ApeirethClient` 6 工具方法 + `invoke_tool` 真 HTTP（reqwest + Bearer + 有界超时 + audit）、`AuthPipeline` 5 组件、TokenBucket/AuditLogger、`STUB_MODE` 守门）。
   - 线协议：`src/wire.rs`（Envelope）+ `src/version.rs`（semver 协商）+ `src/error.rs`（8 错误码）。
   - FFI：`src/abi.rs`/`src/c.rs`（cbindgen C-ABI）/`src/node.rs`/`src/python.rs`（局部 `#![allow(unsafe_code)]`）。
   - 子 SDK：`src/lark/`（9 文件）、`src/livekit/`（7）、`src/sandbox/`（6）、`src/voice/`（8，含 wake.rs）——feature 门控。
4. **实现完整度评级**：🟠 实验——客户端传输是真的，但目标端点与本仓服务端契约对不上（见 7），WS 通用调用恒 stub。
5. **接线状态**：不在服务端生产路径（定位为外部客户端）；6 工具方法打 `/v1/tools/{web_search,file_ops,git_ops,code_exec,calendar,message}/invoke`（`client.rs:225-232`）——**gateway 无这些路由**（只有 `/v1/tools/list`，`panels.rs:646`）；`invoke_stream` WS 8 帧恒 `STUB_MODE`（`lib.rs:28-30`）；`QuotaStub` 恒 501（`client.rs:544` 注释）。
6. **测试**：单测 439 / 集成 42（大量 wiremock HTTP 桩）。关键：`k1_http_transport_is_real_and_ws_remains_stub`、`client_invoke_tool_transports_over_http`、`sdk_c_ffi_hash_request_returns_same_value_as_rust`。
7. **问题清单**：TODO 0；`unimplemented!/todo!` 0；非测试 unwrap/expect 2（`c.rs:177,214`）；死代码：4 子 SDK（约 13,300 行）feature-off + 全仓 0 引用；契约断头：6 个 invoke 端点与 WS 面在本仓无服务端实现；`STUB_MODE`/`QuotaStub` 为显式桩。

## 19. `frontend/companion-desktop`（含 src-tauri）

1. **定位**：Svelte + Tauri 桌面伴侣壳：单页抽屉式多面板 UI + 侧车后端监督。
2. **规模**：src/ 47 个 .svelte（21,010 行）+ 26 个 TS/JS（8,877 行）＝ 73 文件 29,887 行（无 routes/pages，`App.svelte` 3,103 行）；tests/ 19 文件 3,110 行；src-tauri/ 6 个 .rs 3,233 行（`backend_supervisor.rs` 1,827）+ tests/511 行。
3. **核心实现清单**
   - 网关客户端：`src/lib/runtime.ts`（2,296 行，25 个网关 HTTP 端点唯一接线点）。
   - 面板：chat 三栏壳（App.svelte）、会话列表/历史、记忆卷宗（episodes+图谱+protect/forget）、日记、工具、治理 4 tab（审批/授权/守卫/审计）、状态、活动日志、设置（SettingsView 2,857 行）、快速窗、Workbench。
   - 实时通道：SSE（`EventSource` `/v1/apeireth/events`，命名帧 `presence_state`/`approval_required`/`approval_resolved`/`review_rejected`，`src/lib/chat-shell/gateway-events.ts:60-88`）。
   - Tauri 命令面：19 个 `#[tauri::command]`（`src-tauri/src/lib.rs:27-177`：后端启停/重启、provider key 存取（keychain）、工作区、日志目录、快速窗等）。
   - 侧车监督：`src-tauri/src/backend_supervisor.rs`（进程监督 + 崩溃重启计数 + env 注入 19 个 `APEIRETH_ENABLE_*/DISABLE_*` 旋钮，`backend_supervisor.rs:267-350`）。
4. **实现完整度评级**：🟡 库级完备（接近生产）——面板真实接线 25 端点 + SSE、0 TODO/0 mock；扣分：门内无 UI 自动化、`tauri.conf.json` `"csp": null`、rc 版本。
5. **接线状态**：与网关契约见 3（`/v1/chat/completions`、`/v1/messages`、`/v1/chat`、`/v1/panel/*`、`/v1/approvals*`、`/v1/admin/config`、`/v1/sessions/{id}/settings`、`/v1/safety/guard/*` 等）；**不使用** WebSocket/`WsFrame`/`apeireth-protocol`（全目录 0 命中）；设置面板 19 个开关与 CLI 旋钮一一对应（`SettingsView.svelte:181-220`）。
6. **测试**：前端自研 runner（`tests/run-all.mjs`）15 个 suite / 547 个 assert（非 vitest；`it(/test(` 计数 0）；src-tauri Rust 41 个 `#[test]`（33 单测 + 8 集成，`tests/supervisor_lifecycle.rs` 真打 `/health`、`/v1/models`、`/v1/tools/list`）；Playwright 冒烟（`tests/frontend-smoke.cjs`）被 runner 刻意排除（`run-all.mjs:9-11`）。
7. **问题清单**：TODO/FIXME 0、mock 0；唯一空态是 DiaryView 契约式诚实空态（后端无日记端点）；`csp: null`（`tauri.conf.json`）；UI 自动化不在测试门内；`tests/e2e-streamChat-test.mts` 需活网关，归打包 E2E（`scripts/packaged-sidecar-e2e.ps1`）。

---

## 20. 全局汇总表（18 + 1）

| 包 | 评级 | 接线 | 默认 | 测试（单/集） | 一行备注 |
|---|---|---|---|---|---|
| foundation/core | 🟡 | kernel 是；哲学门禁否 | kernel 开 / onion 层关 | 124 / 224 | `RUNTIME_ENFORCED=false`，verdict 链无生产调用点 |
| foundation/protocol | 🟢 | Normalized/canonical 是 | 开 | 188 / 27 | bridge/adapters/acp/usage 约半数面 0 引用 |
| foundation/plugin | 🟢 | 注册表/ToolCapability 是 | 开 | 272 / 0 | `mcp/` 3,600 行 0 消费方；无集成测试 |
| foundation/governance | 🟢 | 治理管线每回合 | 开（onion 层关） | 119 / 0 | `research_autonomy` 543 行 0 引用；无 tests/ |
| foundation/credentials | 🟢 | keyring 注入 credential resolver | 开 | 67 / 29 | master.key 与密文同目录（自述边界） |
| foundation/orchestration | 🟡 | Council/subagent/worktree 是 | 全部关/命令级 | 240 / 13 | cron/durable/context_fold/quota/care/lineage 0 引用 |
| engine/runtime | 🟢 | execute_outcome 主链 | 开（无开关） | 63 / 89 | 审批崩溃不重放、回退路由有测试实证 |
| engine/runtime-assembly | 🟢 | 唯一组合根 | 8 开 12 关 | 87 / 91 | `production.rs` 22 开关 22 注入槽 |
| engine/guard | 🟢 | 治理 hook 每回合 | ML 分类器关 | 31 / 44 | `scenario.rs` 3,853 行大半是场景资产 |
| engine/provider | 🟢 | 3 provider 默认注册 | 开 | 107 / 48 | `reasoning_adapter.rs` 548 行 0 引用 |
| engine/storage | 🟢 | pool+migrations | 开 | 115 / 15 | cache/rate_limit/quota/machine_id 0 外部引用 |
| engine/memory | 🟢 | coordinator+召回+写回 | 核心开、闭环 5 件关 | 787 / 67 | 最大包；hallways/research_* 无消费方 |
| engine/perception | 🟡 | **0 接线** | —（owner 关且未接） | 130 / 10 | 语音/视觉实现全包无产品路径 |
| engine/organ | 🟡 | 仅 organs/morphology 旋钮 | 关 | 179 / 44 | 6 个配套子系统 0 引用 |
| capabilities/tools | 🟢 | tool_modules 注册 | 只读 3 件开；shell/fetch/education 关 | 208 / 67 | `stealth_crawler` 0 引用；supervisor 树不存在 |
| adapters/gateway | 🟢 | serve_canonical 路由面 | serve 命令 | 35 / 32 | barge_in/duplex/file_fetcher 无路由无调用 |
| adapters/cli | 🟢 | 全产品组合根 | 开 | 28 / 40 | help 未列 dream/council/subagent/nightwatch |
| adapters/sdk | 🟠 | 客户端（对不上服务端） | 4 子 SDK feature-off | 439 / 42 | 6 个 invoke 端点 gateway 无路由；WS 恒 stub |
| frontend/companion-desktop | 🟡 | 25 端点 + SSE | — | 547 assert + 41 Rust | 无 UI 自动化门；`csp:null` |

---

## 21. 专项检查

### 21.1 `unimplemented!()` / `todo!()` 真实调用

**逐处结论：0 处。** 对 `crates/**` 与 `frontend/**`（排除 node_modules）全部 `.rs/.ts/.svelte` 扫描 `unimplemented!(`/`todo!(`（剔除注释行），无任何真实调用。占位以显式错误/桩形态存在并自述：`OrganError::NotImplemented`（`engine/organ/src/lib.rs:136-139`）、`NoopVisionBackend`/`NoopSpeech*`（perception）、`STUB_MODE`+`QuotaStub`（sdk/client.rs）、`NoopSubSupervisor`（tools/supervisor.rs:189-246）。

注释态 TODO 全仓 6 处（4 文件）：`foundation/protocol/src/p2p_mesh.rs:19`（v2.1 才接真加密）、`foundation/orchestration/src/ambient_context.rs:183,407`、`foundation/orchestration/src/cognitive_quota_scheduler.rs:182`、`foundation/orchestration/src/lineage_spawning.rs:30,105`（ed25519 未接）。无 FIXME。

### 21.2 默认关开关全景（APEIRETH_ENABLE_* / APEIRETH_DISABLE_*）

接线点均为 `crates/adapters/cli/src/lib.rs`（组合根），桌面端在 `frontend/companion-desktop/src-tauri/src/backend_supervisor.rs:267-350` 注入、`SettingsView.svelte:181-220` 呈现。

| 开关 | 功能 | 默认 | 接线点 |
|---|---|---|---|
| `APEIRETH_ENABLE_LOCAL_READ_TOOLS` | 本地只读三件套（filesystem/search）legacy 显式开 | **默认开**（此开关只是兼容写法） | lib.rs:86-100, 238-241 |
| `APEIRETH_DISABLE_LOCAL_READ_TOOLS` | 逃生门：关掉本地只读 | 不设（=开）；与 ENABLE 同设时 DISABLE 胜 | lib.rs:82-99 |
| `APEIRETH_ENABLE_SHELL` | 注册 tool.shell + grant + 每次 require_approval | 关 | lib.rs:336-349, 699-702 |
| `APEIRETH_ENABLE_FETCH` | 注册 tool.fetch + grant + require_approval | 关 | lib.rs:339-353, 703 |
| `APEIRETH_ENABLE_ORGANS` | OrganModule（9 organ AfterTurn 认知链） | 关 | lib.rs:665-667；production.rs:458 |
| `APEIRETH_ENABLE_PREFERENCE_LEARNING` | 偏好学习（AfterTurn 显式证据） | 关 | lib.rs:668-670；production.rs:415 |
| `APEIRETH_ENABLE_PROACTIVE_RECALL` | 主动召回策略（budget 2 / 阈值 0.10） | 关（`proactive_recall: None`） | lib.rs:107-112；production.rs:388 |
| `APEIRETH_DISABLE_TYPED_RECALL` | typed 召回读侧逃生门 | 不设（typed 召回**默认开**，写读对称修复） | lib.rs:119-123, 745-746 |
| `APEIRETH_ENABLE_MEMORY_INJECTION` | donor 反幻觉注入格式（编号证据清单） | 关 | lib.rs:171-175；production.rs:345 |
| `APEIRETH_ENABLE_CONSOLIDATION` | 每轮记忆整理 + 洞察落库 | 关 | lib.rs:179-183；production.rs:506 |
| `APEIRETH_ENABLE_REFLEXION` | reflexion 失败闭环模块 | 关 | lib.rs:187-191；production.rs:462 |
| `APEIRETH_ENABLE_ONION_LAYER` | 治理管线末层双洋葱判定（L3-L5） | 关 | lib.rs:259-263, 280-284 |
| `APEIRETH_ENABLE_WORKTREE_SANDBOX` | subagent 独立 git worktree 隔离 | 关 | lib.rs:938-942 |
| `APEIRETH_ENABLE_ONERING_LEDGER` | 回合 user/assistant 入 context_ledger | 关 | lib.rs:1166-1170 |
| `APEIRETH_ENABLE_PARTNER_BOND` | 伙伴羁绊（TurnStart 注入 + AfterTurn 演化） | 关（注入 InMemory store） | lib.rs:1418-1422；production.rs:477 |
| `APEIRETH_ENABLE_MORPHOLOGY_RECALL` | 查询形态学→检索深度自适应 | 关 | lib.rs:1426-1430；cognitive.rs:973 |
| `APEIRETH_ENABLE_COMMUNITY_TRIAGE` | 图社区分诊（检索前置双路） | 关 | lib.rs:1434-1438；production.rs:394 |
| `APEIRETH_ENABLE_EDUCATION` | Dx-Check 教育工具注册 | 关 | lib.rs:1442-1446；production.rs:308 |
| `APEIRETH_ENABLE_ABSORPTION_INSIGHT` | 四算法实验洞察（betti/残差/river/kuramoto） | 关 | lib.rs:1451-1455；production.rs:473 |

同族非 ENABLE/DISABLE 旋钮（一并列出）：`APEIRETH_SHELL_SANDBOX`（默认沙箱**开**，`=0` 裸跑，lib.rs:1410-1414）、`APEIRETH_COGNITIVE_JUDGE`/`APEIRETH_COGNITIVE_COUNCIL`（关）、`APEIRETH_GUARD_DATASET_ENABLED`（关）+`APEIRETH_GUARD_ML_MODE`+`APEIRETH_GUARD_ML_MODEL`（ML 分类器关，缺省 NoClassifier）、`APEIRETH_COUNCIL_ADVISORS`（默认 3）+`APEIRETH_COUNCIL_TIMEOUT_MS`、`APEIRETH_COGNITIVE_DB`/`APEIRETH_SESSION_DB`/`APEIRETH_DATA_DIR`/`APEIRETH_PERSONA_ID`/`APEIRETH_SUBJECT_ID`/`APEIRETH_REFLEXION_DIR`/`APEIRETH_MODEL`/`APEIRETH_KEYRING_BACKEND`/`APEIRETH_OPENAI_MODELS`/`APEIRETH_EMBEDDING_URL|MODEL|KEY`。

**一句话结论**：默认产品形态 = 记忆（召回+写回+typed）+ 本地只读三件套 + 三 provider + 治理/审批/审计 + 桌面面板；其余 17 个 ENABLE 开关全部默认关、2 个 DISABLE 为逃生门（其中 typed 召回默认开）。

### 21.3 生产入口链完整性（逐跳）

1. `apeireth-cli` `main.rs` 命令分发（chat/session/approve·reject·cancel/gateway serve/dream/council/subagent/nightwatch）→ 各 `run_*` → `apeireth_cli::dispatch_*`。✅
2. `dispatch_canonical_chat`（lib.rs:996）/`dispatch_gateway_serve_on`（lib.rs:1475）→ `build_canonical_runtime_with_sessions_from_env`（lib.rs:371）→ ① `production_session_store()`（SqliteSessionStore，默认 `.apeireth/sessions.sqlite3`）② `build_cognitive_modules_from_env`（lib.rs:536）③ `build_canonical_runtime_with_parts`。✅
3. `build_cognitive_modules_from_env` → 开关解析（19 旋钮）→ 认知库 schema 校验 + `run_migrations_on_pool` → 6 个 sqlite typed store + access_history → `CognitiveModuleConfig{..}` + `CognitiveBackends{..}` → **`ProductionCognitiveModules::build`（lib.rs:758，全仓唯一生产调用）**。✅
4. `ProductionModules::build`（`production.rs:261`）→ 工具能力注册（filesystem/search/repo/education/shell/fetch/mcp）→ MemoryCoordinator 组装（scoped/embedding/typed/injection/preferences/experience/access_history）→ 认知模块按固定序注册（MemoryRecall→PreferenceRecall→PreferenceLearning→Judge→SelfAssessment→Council→Organ→Reflexion→AbsorptionInsight→PartnerBond→MemoryWriteback）→ 重复 id 校验。✅
5. `build_canonical_runtime_with_parts` → `Runtime::builder` → `with_credentials(keyring_bootstrap)` → `build_production_governance_parts_from_env`（Permission→CredentialDisclosure→PromptInjection→BehaviorChainGuard→可选 OnionLayer）→ `PermissionPresetGovernanceHook` 包装 → `cognitive.register_context_projection/register_into` → provider 插件（minimax→anthropic→openai-compatible?）+ fallback order + 默认模型 → guard dataset 观测者。✅
6. 各引擎模块承接：runtime `execute_outcome`（agent loop）逐动作走 governance；MemoryRecallModule TurnStart 注入 overlay、MemoryWritebackModule AfterTurn 落库；tool_modules 经 `ToolCapability` 被 dispatch_one_tool 执行（冻结→审批→执行→结果入 transcript）。✅
7. `gateway serve` 在同一 runtime 上加 `CliPanelData` + `GatewayServices` + `RuntimeObservationSink` → `serve_canonical_with_services`（cli lib.rs:1516）。`session` 命令 = 同一 bootstrap 后打印 providers。`dream`/`nightwatch`/`council`/`subagent` 各自直开库/工厂（不经回合执行器，属显式命令面）。✅

**结论**：入口链完整、单一组合根、无第二 runtime；各跳均有对应测试（cli 40 集成、runtime-assembly 91 集成、gateway 32 集成）。

### 21.4 frontend/companion-desktop 专项

- 页面/组件：47 个 .svelte（10+ 面板：chat/会话/记忆/日记/工具/治理 4tab/状态/日志/设置/快速窗/Workbench），无路由框架（单页抽屉壳 `App.svelte:138` DrawerId）。
- 网关接线端点：25 个 HTTP 端点全部集中在 `src/lib/runtime.ts`（chat 3 形态 `/v1/chat/completions`、`/v1/messages`、`/v1/chat`；`/v1/models`；`/v1/sessions/{id}/settings` GET/PATCH；`/v1/admin/config` GET/POST；`/v1/panel/{sessions,memory/episodes,graph,audit,tools,grants,traces}`；`/v1/approvals`+`/v1/approvals/resolve`；`/v1/memory/append`；`/v1/apeireth/capabilities`；`/v1/apeireth/memory/episodes/{id}/forget|protect|unprotect`；`/v1/safety/guard/{status,events,evaluate}`；`/v1/workbench/turn`；`/v1/runtime/snapshot`）+ SSE `/v1/apeireth/events`（EventSource，命名帧 presence_state/approval_*）。
- src-tauri：19 个 `#[tauri::command]`；41 个 `#[test]`（33 单测 + 8 集成，集成真打 `/health`、`/v1/models`、`/v1/tools/list`）。
- 测试：前端 15 suite / 547 assert（自研 runner）；`tests/` 19 文件 3,110 行；UI 自动化（Playwright 冒烟）被排除在门外。

### 21.5 与既有两文档交叉对账（`docs/04-internal/stage123-gap-matrix.md` / `claims-evidence-matrix.md`）

两文档整体判断与本次实测高度一致（"机制在、默认不跑"是主流形态、感知/编排大半未接线、SDK/协议面有桩）。以下列**不一致或需修正处**（其余判定经抽样复核成立）：

| # | 文档主张 | 实测结果 | 裁决 |
|---|---|---|---|
| 1 | claims："代码 `ALL_TWELVE_KEYS`（12 键）与 13 键宣称口径漂移"（claims L104） | `philosophy.rs:99` `ALL_THIRTEEN_KEYS: [PhilosophyKey; 13]`，`ALL_TWELVE_KEYS` 是 13 元素历史别名（philosophy.rs:120-125），测试 `all_twelve_keys_compat_alias_contains_thirteen_keys` 断言 len==13 | **gap 正确、claims 需修正**：仅命名遗留，非数量漂移 |
| 2 | 测试数三套口径：claims 引 INSTALL.md "3662 passed / 130 suites"（静态手填）、gap 引 "TOTAL_TESTS=2265 / core 344" | 静态实测 crates `#[test]` 合计 **4,101**（单 3,219 + 集 882）；core 348（124+224） | **三口径互不一致，均与实测有差**；建议统一用静态计数口径 |
| 3 | claims："8 个 `#[kani::proof]` 无 `kani::any()`"（L37-38） | 实测 12 个 proof 6 文件：4 个 organ_kani_proofs 共 8 个（确无 kani::any）+ `research_approval_sm.rs:568,588,601` 3 个 + `gateway/file_fetcher.rs:267` 1 个，且该 proof 用 `kani::any()`（file_fetcher.rs:270） | **部分成立**：8 个的口径只覆盖 organ_kani_proofs，遗漏 file_fetcher 1 个（及 research 3 个的记法差异） |
| 4 | claims："MiniMax TTS 178 行零 HTTP 调用"（L127） | `minimax_tts.rs` 178 行仅 `build_request`/`derive_tone_from_pad`，无任何 HTTP | **文档正确**（复核确认） |
| 5 | claims/gap："PerceptionOwner 默认关且未接线"、perception 全面未接 | 全仓 `apeireth_perception` 仅自身 + plugin trait 引用；owner 0 引用 | **文档正确** |
| 6 | claims："CognitiveQuotaScheduler / CarePotentialField / LineageSpawningOrchestrator 无接线"；gap："supervisor 零件齐而树不存在" | 实测全部 crate 外 0 引用；`StdSubSupervisor` 仅字符串名在 `permission_preset.rs:49-50`/`onion_layer.rs:41` 分类表 | **文档正确** |
| 7 | claims："SDK STUB_MODE + 4 子 SDK NotImplemented"；gap："SDK 阶段 6 stub" | 属实，且 4 子 SDK 是 feature-gated（`lib.rs:277-284`），默认 build 不编译 | **文档正确且偏轻**：约 13,300 行默认不编译、全仓 0 引用 |
| 8 | claims L146："canonical_entry 路由表无 `/v1/ws`" | 属实（`canonical_entry.rs:589-617`），且全仓无 WebSocket 服务端 | **文档正确**；补充：前端实测也只用 SSE，WS 面全链缺服务端 |
| 9 | （两文档均未记）SDK 6 个 `/v1/tools/{tool}/invoke` 端点 | gateway 只有 `/v1/tools/list`（panels.rs:646），无任何 invoke 路由；SDK 工具名（web_search/file_ops/git_ops/code_exec/calendar/message）与 runtime 工具（tool.filesystem/search/repo/shell/fetch/education）也不同名 | **新增缺口**：SDK↔服务端契约断头，两文档未覆盖 |
| 10 | （两文档均未记）provider/protocol/storage/plugin 的死面 | `reasoning_adapter.rs` 548 行、protocol bridge/adapters/acp/usage/ws_session、storage cache/rate_limit/quota/machine_id、plugin `mcp/`+`watch.rs` 均 0 外部引用 | **新增**：多为库级资产，未进产品 |
| 11 | gap L86："NSIS E2E 17/17 含真聊天" vs claims L145："15 个 Node 套件 CI 不跑/点击流从未点过" | 前端实测：`run-all.mjs` 15 suite 但刻意排除 Playwright 冒烟；`scripts/packaged-sidecar-e2e.ps1` 为打包门（需活网关） | 两说法可并存；**"测试门内无 UI 自动化"为实** |
| 12 | claims L93："审批 500ms 与代码 5 分钟不符"、"README 两个 API 不存在"等 | 本次未复核（超出读码范围） | 标注**未核** |

### 21.6 其他横向事实

- 危险 `unwrap/expect`（非测试）合计 120 处：memory 24、tools 22、gateway 16、governance 16、runtime-assembly 11、runtime 9、orchestration 7、cli 4、guard 4、plugin 2、sdk 2、storage 1、core 1；perception/provider/credentials/protocol 为 0。
- 死代码（pub 但全仓 0 生产引用）按体量排序：sdk 4 子 SDK（~13,300 行，feature-off）→ memory hallways/research_roaming/dailynote/layered_memo（~5,000 行）→ plugin mcp/（~3,600 行）→ orchestration cron/durable/context_fold/quota/care/lineage/research_*（~4,000 行）→ protocol bridge/adapters/acp/usage/ws_session（~2,500 行）→ organ 6 子系统（~3,000 行）→ provider reasoning_adapter（548）→ storage machine_id/cache/rate_limit/quota（~3,100 行）→ gateway barge_in/duplex/file_fetcher（~780 行）→ tools stealth_crawler（160）。

---

## 22. 全局诚实结论

**① 真实现的硬核在哪。** 一条完整、单组合根、带崩溃安全语义的生产主链是实打实的：`engine/runtime` 的回合执行器（多轮 + 轮数/单轮工具数双上限 + 逐动作治理授权 + 审批冻结/claim-before-invoke/崩溃不重放）+ `runtime-assembly/production.rs` 唯一装配根（22 开关 22 注入槽、缺后端 boot 失败、重复 id 拒绝）+ 三家 canonical provider（错误分级/回退/缺凭据 fail-closed 不出网）+ `engine/memory` 的 SQLite 记忆栈（1,230 行迁移、六流历史、BM25+向量混合召回、治理 forget 严格排除、typed 写读对称）+ `engine/guard` 两段式行为链安全（快挡 + 敏感源→外部 sink + 脱敏数据集）+ `capabilities/tools` 的真遏制（Windows Job Object/AppContainer、shell 沙箱、egress 允许清单、fetch 前置 deny）+ `foundation/credentials` 三级 keyring。这些路径默认生效、有合计 4,101 个测试压着，其中审批生命周期、fetch/shell 沙箱、记忆治理遗忘等负向测试是真刀真枪的。桌面端 25 个端点 + SSE 与网关契约对齐，19 个开关与 CLI 旋钮一一对应。

**② 薄在哪。** 三类薄：(a) **未接线的大块资产**——perception 整包（~6,300 行）0 生产引用；organ 6 子系统、orchestration 的 cron/durable/context_fold/quota/care/lineage、plugin 的 mcp/、protocol 的 bridge/adapters/acp/usage、storage 的 cache/rate_limit/quota/machine_id、gateway 的 barge_in/duplex/file_fetcher，合计约 3.5 万行库级代码无产品消费方；(b) **契约断头与桩**——sdk 的 6 个 `/v1/tools/{tool}/invoke` 端点服务端不存在、WS 8 帧 `invoke_stream` 恒 stub、`QuotaStub` 恒 501、4 个子 SDK feature-off、`p2p_mesh` 明文、`lineage_spawning` 签名 TODO；(c) **默认关的差异化功能**——17 个 `APEIRETH_ENABLE_*` 全部默认关（organs/偏好学习/记忆闭环四件套/reflexion/羁绊/形态学/社区分诊/教育/吸收洞察/洋葱层/worktree/onering），且 `partner_store` 是 InMemory（重启即散）；`core` 的哲学门禁 `RUNTIME_ENFORCED=false`，verdict 链不进执行主链。测试面上：governance/plugin 无 `tests/`、桌面端测试门内无 UI 自动化、`csp:null`、非测试 `unwrap/expect` 127 处。

**③ 如果明天要发 T0 正式版，挡住发布的三个最大实现缺口。** 其一，**默认产品形态与卖点错位**：所有差异化认知能力（9 organs、偏好学习、主动召回、记忆注入/整理/反思、羁绊、社区分诊）默认全关，伙伴羁绊用 InMemory 存储一重启即失——T0 默认路径只是"记忆 chat + 本地只读工具"，要么把这些切默认开并补持久化（partner sqlite 后端），要么把发布口径降到与默认形态一致。其二，**SDK/实时通道是断头路**：`apeireth-sdk` 的 6 个工具 invoke 端点在 gateway 无路由、工具名与 runtime 能力名对不上、WS `invoke_stream` 恒 stub、配额恒 501——对外 SDK 面目前打不通任何一个真实端点，T0 若承诺 SDK/第三方集成，必须补 `/v1/tools/*/invoke`（或改 SDK 对齐 `/v1/chat`+面板契约）并裁决 WS 面去留。其三，**多模态与 UI 验证双缺**：perception 语音/视觉（含 whisper_http 1,147 行真 STT、xcap 757 行截屏）一条都没接进 runtime/桌面端，桌面伴侣的"多模态在场"无任何生产跳；同时桌面端测试门内无 UI 自动化（Playwright 冒烟被刻意排除）且 `tauri.conf.json` `csp:null`——T0 前至少要接通一条感知链（语音或截屏任一）并把 UI 冒烟纳入测试门、补 CSP。
