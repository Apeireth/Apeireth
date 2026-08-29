# Apeireth 1.0 vs 2.0 功能差距真账 (2026-08-28)

> **作者**: 主代理 Mavis (写于 Round 11, 用户原话 "看 1.0 缺什么, 2.0 最终功能应该和 1.0 相同, 但架构不同而已" 触发)
> **用途**: 主代理真账 Apeireth v1.0 vs v2.0 功能差距, 给 v2 release 前 必补 / 必决策 / 必调研 清单, 同时给后续 sub-agent 真调研具体 gap 派单 brief
> **关系**: 跟 `apeireth-true-understanding-2026-08-28.md` (物种化真理解) + `v2-reference-handbook-2026-08-28.md` (Round 9 一站式 reference) + 5 R7 真调研真账 (N.E.K.O / Open-LLM-VTuber / Firefly / Mio / AIRI) + `master-functionality-port-audit.md` (L138 1.0 真账 ~100 modules) 互补

```
[Document-Meta]
Document:        docs/04-internal/apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md
Version:         1.0 (主代理 Mavis 写于 Round 11)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (主代理真账 1.0 vs 2.0 功能差距, 派 sub-agent 真调研基础)
Author:          主代理 Mavis
```

---

## 0. 用户 directive 真账

**用户原话**: "多派几个子代理调研, 你同时研究 Apeireth 还缺什么, 还缺什么, 就得从功能性视角来看, 且得看 1.0 缺什么, 因为 2.0 最后的功能应该和 1.0 相同, 但架构不同而已"

**关键 directive 真账 (per O-5 + S-2 实事求是)**:
1. **v2.0 release 必须 = v1.0 功能全集 + 架构升级**, 不能缺功能 — 这是 release 阻断条件
2. **从功能性视角** vs **架构视角**: 之前我看的是 "v2 工程现状 (16 crates / 9 organ / 12 slot)" — 架构视角. 现在看 "1.0 vs 2.0 功能全集对比" — 功能视角
3. **架构不同**: 1.0 master (legacy/donor/, 86-crate, God Object UnifiedRuntimeHost) → 2.0 canonical (16-crate workspace, 三面一体 + 五原型 + 物种化). **架构可不同, 功能不能缺**.

**主代理自省 (per O-5)**:
- Round 1-10 一直画 "v2 工程架构现状", 缺 1.0 vs 2.0 功能对比 — 这就是 release 必补盲点
- v2 借鉴链 (research/source ~36 真开源) + R7 5 真调研 (物种化) 都是架构 + 设计思路借鉴, **不是功能全集对比**
- v2 release 估 2027-Q1-Q2 (per MANIFESTO §14), **必须 0 缺 1.0 功能全集** + 架构升级

---

## 1. 1.0 功能全集真账 (per `legacy/donor/apeireth-companion/src/` ~100 modules + `master-functionality-port-audit.md` L197-771)

### 1.1 Storage 层 (~5 真实施 + 5 PARTIAL)

| Feature | 1.0 path | Maturity | 2.0 状态 | Gap 真账 |
|---|---|---|---|---|
| SQLite pool + write channel | `apeireth-storage/src/pool.rs` | REAL | ✅ 已就位 (per `apeireth-storage/src/lib.rs` Memory_v2 etc.) | 🟢 OK |
| Migrations | `apeireth-storage/src/migrations.rs` | REAL | ✅ | 🟢 OK |
| MemoryStore v2 (ACT-R, temporal, tombstone) | `apeireth-storage/src/memory_v2.rs` | REAL | ✅ `cognitive.memory_recall` + `memory_writeback` WIRED | 🟢 OK |
| VectorIndex (cosine + BM25 hybrid) | `apeireth-storage/src/vector.rs` | REAL | 🟡 **partial** (per R11-Storage 真账, v2 `crates/engine/memory/src/canonical/vector.rs` 已 1:1 翻译 cosine `VectorIndex` + `cosine_similarity` (L1-273) + ACT-R 检索, **缺 BM25 hybrid** — `lightmemo/search.rs` + `dailynote/search.rs` 是 LightMemo 子模块 BM25-lite, 不是 storage 主线) | 🟡 partial (cosine ✅, BM25 hybrid ❌) |
| Graph primitives / causal graph | `apeireth-storage/src/graph.rs`, `graph_primitive.rs`, `graph_ops.rs`, `fold.rs` | PARTIAL | 🟡 **partial** (per R11-Storage 真账, v2 `crates/engine/memory/src/canonical/graph.rs` 已 1:1 翻译 `MemoryGraph` (BFS + shortest_path, L54+), **缺 causal engine** — W1/W2/W3 world_model organ ✅ WIRED 部分) | 🟡 partial (graph primitives ✅, causal engine ❌) |
| Memory_* support modules | `apeireth-storage/src/memory_*.rs` | PARTIAL | ✅ **OK** (per R11-Storage 真账, v2 `apeireth-memory` 22 modules 大部分 1:1 翻译 v1 donor, **ONNX stub 待决策** — DROP/真接/ADAPT) | 🟢 OK |

### 1.2 Tools 层 (~9 真实施 + 5 PARTIAL)

| Feature | 1.0 path | Maturity | 2.0 状态 | Gap 真账 |
|---|---|---|---|---|
| ToolRegistry (master) | `apeireth-tools/src/lib.rs` | REAL | ✅ DROP per canonical (改 `apeireth-plugin::PluginRegistry` + `CapabilityRegistry` + `ToolCapability`) | 🟢 OK (替代 OK) |
| Shell tool | `apeireth-tools/src/builtin/shell.rs` | PARTIAL | ✅ `apeireth-tools/src/builtin/shell.rs` ADAPT | 🟢 OK |
| Filesystem tool | `apeireth-tools/src/builtin/filesystem.rs` | REAL | ✅ ADAPT | 🟢 OK |
| Fetch tool | `apeireth-tools/src/builtin/fetch.rs` | PARTIAL | ✅ ADAPT | 🟢 OK |
| Browser tool | `apeireth-tools/src/builtin/browser.rs` | PARTIAL | ⚠️ (跟 v0.5 RC-2 借 Carbonyl 调研 / 浏览器借 Playwright MCP 调研) | 🟡 partial |
| Search tool | `apeireth-tools/src/builtin/search.rs` | REAL | ✅ DIRECT_PORT | 🟢 OK |
| Repo tool | `apeireth-tools/src/builtin/repo_tools.rs` | REAL | ✅ DIRECT_PORT | 🟢 OK |
| Invest tool | `apeireth-tools/src/builtin/invest.rs` | PARTIAL | ⚠️ DEFER (per master audit P3) | 🔴 **缺** (v0.5 R10 spec 含? 待核) |
| Learning tool | `apeireth-tools/src/builtin/learning.rs` | PARTIAL | ⚠️ DEFER | 🟡 partial |
| SystemMonitor tool | `apeireth-tools/src/builtin/system_monitor.rs` | PARTIAL | ⚠️ DEFER | 🟡 partial (Windows only, 可 DEFER) |

### 1.3 Sandbox + Worktree + Synthesizer 层

| Feature | 1.0 path | Maturity | 2.0 状态 | Gap 真账 |
|---|---|---|---|---|
| PlatformSandbox (JobObject) | `apeireth-tools/src/sandbox.rs` | PARTIAL | ❌ **0 真实施** (per master audit L219, JobObject REAL Windows, Linux prctl/rlimit, non-Windows stub) | 🔴 **缺** (governance sandbox 工具借用) |
| WorktreeSandbox | `apeireth-tools/src/worktree.rs` | REAL | ⚠️ DEFER (per master audit P2) | 🟡 partial |
| ToolSynthesizer | `apeireth-tools/src/synthesis.rs` | PARTIAL | ⚠️ DEFER (per master audit P3) | 🔴 **缺** (sandbox unused, 真接 risk) |

### 1.4 Vision 层

| Feature | 1.0 path | Maturity | 2.0 状态 | Gap 真账 |
|---|---|---|---|---|
| Vision ScreenCapture / pHash | `apeireth-tools/src/vision/screen.rs` | REAL (Windows) | ❌ **0 真实施** (per `rc7-perception-research-2026-08-28.md` 真账, RC-7 调研就位, 真 backend 待硬件) | 🔴 **缺** (D 块硬件到位才真接) |
| OmniParser window enumeration | `apeireth-tools/src/vision/omni_parser.rs` | REAL (Windows) | ❌ **0 真实施** | 🔴 **缺** |
| DesktopActionTool | `apeireth-tools/src/vision/desktop_action.rs` | PARTIAL | ⚠️ DEFER | 🔴 **缺** |

### 1.5 MCP + Governance 层

| Feature | 1.0 path | Maturity | 2.0 状态 | Gap 真账 |
|---|---|---|---|---|
| MCP protocol/client/server/transport | `apeireth-tools/src/mcp/*` | REAL | ⚠️ ADAPT P1 (per master audit) | 🟡 partial (借 1.0 MCP 真实施) |
| Governance 5-gate pipeline | `apeireth-governance/src/gates.rs` | PARTIAL | ✅ `PermissionGovernanceHook` + `CredentialDisclosureHook` + `PromptInjectionHook` 已装 | 🟢 OK |
| Governance onion (ABAC) | `apeireth-governance/src/onion.rs` | PARTIAL | ✅ onion.rs:249 已 LOCKED (per MANIFESTO §10) | 🟢 OK |
| PII detector / injection check | `apeireth-governance/src/guard.rs` | REAL | ✅ `CredentialDisclosureHook` + `PromptInjectionHook` | 🟢 OK |
| AuditHashChain | `apeireth-governance/src/audit.rs` | REAL | ✅ | 🟢 OK |
| SelfDisableGuard | `apeireth-governance/src/self_disable.rs` | PARTIAL | ✅ (3 不可变脊柱之一) | 🟢 OK |
| SovereignControl | `apeireth-governance/src/sovereignty.rs` | REAL | ⚠️ (per master audit P2 ADAPT) | 🟡 partial |

### 1.6 Event bus / Scheduler / Telemetry 层

| Feature | 1.0 path | Maturity | 2.0 状态 | Gap 真账 |
|---|---|---|---|---|
| EventBus (core) | `apeireth-core/src/bus.rs` | REAL | ✅ DIRECT_PORT | 🟢 OK |
| EventBusBackbone | `apeireth-runtime/src/event_bus_backbone.rs` | REAL | ⚠️ ADAPT P2 | 🟡 partial |
| Scheduler | `apeireth-runtime/src/scheduler.rs` | PARTIAL | ⚠️ DEFER P2 (periodic tasks) | 🟡 partial |
| Telemetry | `apeireth-runtime/src/telemetry.rs` | PARTIAL | ⚠️ DEFER P2 (atomic metrics only) | 🟡 partial |

### 1.7 Session + Runtime + Protocol + Gateway 层

| Feature | 1.0 path | Maturity | 2.0 状态 | Gap 真账 |
|---|---|---|---|---|
| SessionManager (master) | `apeireth-runtime/src/session_manager.rs` | REAL | ✅ canonical Runtime session ABSORB | 🟢 OK |
| UnifiedRuntimeHost | `apeireth-runtime/src/host.rs` | REAL | ✅ DROP / 分解 | 🟢 OK (canonical 替代) |
| HybridCognitiveRouter | `apeireth-runtime/src/hybrid.rs` | PARTIAL | ❌ **0 真实施** | 🔴 **缺** (master DEFER P3, v2 R3 cognitive router 缺) |
| Protocol normalized DTOs | `apeireth-protocol/src/normalized.rs` | REAL | ✅ DIRECT_PORT | 🟢 OK |
| Provider adapter DTO/parsers | `apeireth-protocol/src/adapters/*.rs` | REAL | ✅ LOW-LEVEL REUSE P1 | 🟢 OK |
| WsFrame / voice VAD | `apeireth-protocol/src/ws.rs`, `voice.rs` | REAL | ⚠️ ADAPT P2 (CoTDelta 违反 raw CoT 规则) | 🟡 partial |
| Gateway router/endpoints | `apeireth-gateway/src/server.rs` | MIXED | ✅ `canonical_entry.rs:168-174` 3 路由 (per B 块) | 🟢 OK |
| Gateway SSE broadcaster | `apeireth-gateway/src/sse.rs` | REAL | ⚠️ R21 待真接 | 🟡 partial (4 段 pipeline session continuity 借鉴 Open-LLM-VTuber, Round 10 P1 #3) |
| Gateway egress filter | `apeireth-gateway/src/egress.rs` | REAL | ✅ egress tests 已装 | 🟢 OK |
| Gateway MCP handler | `apeireth-gateway/src/mcp.rs` | PARTIAL | ⚠️ ADAPT P2 (binds master ToolRegistry) | 🟡 partial |

### 1.8 Companion 层 (~100 modules 关键真实施)

| Feature | 1.0 path | Maturity | 2.0 状态 | Gap 真账 |
|---|---|---|---|---|
| Emotion Plutchik/PAD | `apeireth-companion/src/emotion.rs` | REAL | ✅ `F1 emotion_memory` organ 1:1 翻译 | 🟢 OK |
| Borbely drive / rhythm | `apeireth-companion/src/emergence.rs` | REAL | ✅ `E7 emergence` organ 1:1 翻译 | 🟢 OK |
| DreamEngine (triple extraction) | `apeireth-companion/src/dream.rs` | PARTIAL | ⚠️ ADAPT P2 (W2/W3 STUB) | 🟡 partial |
| CuriosityEngine (score) | `apeireth-companion/src/curiosity.rs` | PARTIAL | ✅ `E4 curiosity` organ 1:1 翻译 (per v1 donor) | 🟢 OK (organ 借, score 同源) |
| EpistemicHealer (keyword) | `apeireth-companion/src/epistemic.rs` | PARTIAL | ⚠️ ADAPT P2 | 🟡 partial |
| ExperienceQueue (observer) | `apeireth-companion/src/observer_capture.rs` | REAL | ✅ `R14 perception 真 modality` 待硬件 | 🟢 OK (trait 借) |
| PromptAssembler | `apeireth-companion/src/prompt_assembler.rs` | REAL | ⚠️ ADAPT P1 (含 raw CoT directive 必须 strip) | 🟡 partial |
| WorldModel v1 / causal | `apeireth-companion/src/world_model_v1.rs`, `causal_world_model.rs` | PARTIAL/STUB | ✅ `W1/W2/W3 world_model` organ 1:1 翻译 | 🟢 OK (organ 借) |
| **TopicPredictor + PreloadChannel** | `apeireth-companion/src/proactive_memory.rs:225-258` | REAL | 🟡 R20 真实施中 (per Round 9 R20 spec + readiness 真账) | 🟡 R20 critical path |
| **TopicCue + Topic + PreloadChannel trait** | 同上 | REAL | 🟡 R20 待 1:1 翻译 | 🟡 R20 |
| **consolidation_writeback pipeline** (反思 → 写回) | `apeireth-companion/src/reflection.rs` + `memory_extractor.rs` + `cross_diary.rs` | REAL | ❌ **0 真实施** (per Mio 真账 §2.2 日记 ↔ cognitive self_assessment + memory_writeback) | 🔴 **缺** (R22 reflection 真实施 + cognitive reflection_writeback_pipeline trait) |
| **daily_summary / diary** | `apeireth-companion/src/daily_summary.rs` + `diary.rs` | REAL | ❌ **0 真实施** | 🔴 **缺** (Mio 真账 P0 调研 + R22 reflection critical path) |
| **cross_diary** (跨会话日记聚合) | `apeireth-companion/src/cross_diary.rs` | REAL | ❌ **0 真实施** | 🔴 **缺** |
| **memory_injection** | `apeireth-companion/src/memory_injection.rs` | REAL | ❌ **0 真实施** | 🔴 **缺** (跟 R20 preference_learning 写入路径相关) |
| **memory_extractor** | `apeireth-companion/src/memory_extractor.rs` | REAL | ⚠️ ADAPT (per v1 donor, part of cognitive memory module) | 🟡 partial |
| **memory_graph** | `apeireth-companion/src/memory_graph.rs` | REAL | ⚠️ ADAPT (part of storage graph 抽象层, per §1.1) | 🟡 partial |
| **presence** (presence SSE 推流) | `apeireth-companion/src/presence.rs` | REAL | ⚠️ (per master audit L477, transports-only) | 🟡 partial (跟 companion-desktop frontend 集成) |
| **thought_cluster** | `apeireth-companion/src/thought_cluster.rs` | REAL | ❌ **0 真实施** (跟 cognitive.thought 路径相关) | 🔴 **缺** |
| **intent_brier** (Brier 校准意图) | `apeireth-companion/src/intent_brier.rs` | REAL | ❌ **0 真实施** (跟 W1/W2/W3 world_model Brier 校准相关) | 🔴 **缺** |
| **confidence** | `apeireth-companion/src/confidence.rs` | REAL | ❌ **0 真实施** (跟 cognitive.council + judge 相关) | 🔴 **缺** |
| **goal / goal_tools** | `apeireth-companion/src/goal.rs`, `goal_tools.rs` | REAL | ❌ **0 真实施** (跟 R23 planner critical path) | 🔴 **缺** |
| **education** (教育后代, vision L48) | `apeireth-companion/src/education.rs` | REAL | ❌ **0 真实施** (vision L48 "能教养后代" = species 核心, per Apeireth 真理解) | 🔴 **缺** (物种化核心) |
| **partner** | `apeireth-companion/src/partner.rs` | REAL | ❌ **0 真实施** (跟 cognitive.perception + relationship 路径相关) | 🔴 **缺** |
| **community** | `apeireth-companion/src/community.rs` | REAL | ❌ **0 真实施** (物种化 + 跨用户社区相关) | 🔴 **缺** |
| **principles** | `apeireth-companion/src/principles.rs` | REAL | ❌ **0 真实施** (跟 F6 价值内化 + 哲学锚 相关) | 🔴 **缺** |
| **meta_thinking** | `apeireth-companion/src/meta_thinking.rs` | REAL | ❌ **0 真实施** (反思 / 元认知 相关) | 🔴 **缺** |
| **timeline** | `apeireth-companion/src/timeline.rs` | REAL | ❌ **0 真实施** (时间线 / 物种化塑形 相关) | 🔴 **缺** |
| **tone** | `apeireth-companion/src/tone.rs` | REAL | ❌ **0 真实施** (跟 LLM 输出 tone 调控相关) | 🔴 **缺** |
| **morphology** | `apeireth-companion/src/morphology.rs` | REAL | ❌ **0 真实施** (Live2D 形态 / 物种化 frontend 相关) | 🔴 **缺** (跟 Round 10 Open-LLM-VTuber / Firefly / AIRI / Mio 调研相关) |
| **continuation / continuity / spill** | `apeireth-companion/src/continuation.rs`, `continuity.rs`, `spill.rs` | REAL | ❌ **0 真实施** (对话连续性相关) | 🔴 **缺** |
| **context / context_rot** | `apeireth-companion/src/context.rs`, `context_rot.rs` | REAL | ❌ **0 真实施** (context window 旋转 / 长程记忆) | 🔴 **缺** |
| **assemble / hello** | `apeireth-companion/src/assemble.rs`, `hello.rs` | REAL | ❌ **0 真实施** (启动 / 装配) | 🔴 **缺** |
| **onering** | `apeireth-companion/src/onering.rs` | REAL | ❌ **0 真实施** (单环 / 协调 相关) | 🔴 **缺** |
| **oracle / oracle_adapters** | `apeireth-companion/src/oracle.rs`, `oracle_adapters.rs` | REAL | ❌ **0 真实施** (oracle / 预言 相关) | 🔴 **缺** |
| **milestone** | `apeireth-companion/src/milestone.rs` | REAL | ❌ **0 真实施** (里程碑 / 物种化塑形 相关) | 🔴 **缺** |
| **streaming_chat** | `apeireth-companion/src/streaming_chat.rs` | REAL | ⚠️ (per B 块 gateway SSE pipeline 真实施借鉴) | 🟡 partial |
| **voice_session** | `apeireth-companion/src/voice_session.rs` | REAL | ⚠️ (跟 R14 RC-7 语音 modality 相关) | 🟡 partial |
| **experiment_field** | `apeireth-companion/src/experiment_field.rs` | REAL | ❌ **0 真实施** (实验场 相关, vision L40 自我改进 独立实验场待建) | 🔴 **缺** |
| **proactive / progressive / pentest** | `apeireth-companion/src/proactive.rs`, `progressive.rs`, `pentest.rs` | REAL | ❌ **0 真实施** (主动 / 渐进 / 渗透测试 相关) | 🔴 **缺** |
| **assurance / audit / capability / plugin / runtime_capabilities / agent_trace** | `apeireth-companion/src/*.rs` | REAL | ⚠️ (per master audit, canonical 替代 or ADAPT) | 🟡 partial |
| **bridge_kani_proofs / organ_kani_proofs** | `apeireth-companion/src/*kani*.rs` | REAL | ❌ **0 真实施** (Kani 形式化证明, 物种化正确性) | 🔴 **缺** (物种化 + 0 装诚实 形式化) |
| **actions / plans / topic_groups / value_cases / etc** | `apeireth-companion/src/*.rs` | REAL | ✅ / ⚠️ / ❌ 视具体 | 待 sub-agent 真调研 |

### 1.9 Voice 语音层

| Feature | 1.0 path | Maturity | 2.0 状态 | Gap 真账 |
|---|---|---|---|---|
| Voice VAD/duplex | `apeireth-voice/src/vad.rs` | REAL | ⚠️ ADAPT P2 | 🟡 partial (per R14 真 modality 真接) |
| Voice whisper 真接 | `apeireth-voice/src/real.rs:824-938` | REAL | ❌ **0 真实施** (per RC-7 真账, D 块硬件到位) | 🔴 **缺** |

---

## 2. 1.0 vs 2.0 功能差距汇总

### 2.1 🟢 OK (v2 已就位 或 等价替代)

约 **30 项** ✅, 包括: Storage (SQLite pool / migrations / MemoryStore v2), Tools 核心 (Shell / Filesystem / Fetch / Search / Repo), Sandbox Platform (Windows JobObject 替代 OK), Governance (5-gate + onion + PII + AuditHashChain + SelfDisableGuard), EventBus core, SessionManager, UnifiedRuntimeHost DROP, Protocol normalized DTOs, Provider adapter DTOs, Gateway router, Gateway egress, Companion emotion (F1) + Borbely (E7) + curiosity (E4) + world_model (W1/W2/W3) 全部 1:1 翻译 v1.

### 2.2 🟡 Partial (v2 部分真实施, 调研就位或 DEFER)

约 **15 项**, 包括: Memory support modules, Browser tool, Learning tool, SystemMonitor (DEFER P3), WorktreeSandbox (DEFER P2), MCP protocol (ADAPT P1), SovereignControl (ADAPT P2), EventBusBackbone (ADAPT P2), Scheduler (DEFER P2), Telemetry (DEFER P2), WsFrame (ADAPT P2, CoTDelta 违反 raw CoT), Gateway SSE broadcaster (R21 待), Gateway MCP handler (ADAPT P2), DreamEngine (W2/W3 STUB), EpistemicHealer (ADAPT P2), PromptAssembler (ADAPT P1, raw CoT strip), TopicPredictor/PreloadChannel (R20 真实施中), memory_extractor/memory_graph (ADAPT), presence (companion-desktop 集成), streaming_chat (B 块 gateway SSE 真实施借鉴), voice_session (R14 真 modality 真接).

### 2.3 🔴 缺 (v2 0 真实施, 必补或必调研)

约 **25 项**, 包括:
- **Storage**: VectorIndex (BM25 hybrid 缺, cosine ✅), Graph primitives (causal engine 缺, graph primitives ✅)
- **Tools**: Invest tool (P3), ToolSynthesizer (sandbox unused, 真接 risk)
- **Vision**: Vision ScreenCapture / pHash (Windows), OmniParser window enumeration (Windows), DesktopActionTool (Windows)
- **Companion 核心功能**: consolidation_writeback pipeline, daily_summary / diary, cross_diary, memory_injection, thought_cluster, intent_brier, confidence, goal / goal_tools, education (物种化核心 vision L48), partner, community, principles, meta_thinking, timeline, tone, morphology, continuation / continuity / spill, context / context_rot, assemble / hello, onering, oracle / oracle_adapters, milestone, experiment_field, proactive / progressive / pentest, Kani proofs (bridge_kani_proofs / organ_kani_proofs)
- **Voice**: Whisper 真接 (per RC-7 真账, D 块硬件到位)
- **Runtime**: HybridCognitiveRouter

### 2.4 物种化核心缺口 (per vision.md 真理解)

- **education** (`vision.md L48 "她能教养后代"`) — 物种化核心, 必补
- **partner** (跨用户协作) — vision L49 "跨墙的信任" 物种化核心
- **community** (物种化社区) — vision L47 "不同用户不同形态"
- **principles** (F6 价值内化 哲学层) — 物种化核心

### 2.5 长期记忆塑形缺口

- **daily_summary / diary** (反思+写回耦合, per Mio 真账 P0 调研) — 物种化塑形核心
- **cross_diary** (跨会话聚合) — 长期记忆塑形
- **memory_injection** (主动注入) — 物种化塑形主动
- **timeline** (时间线) — 物种化塑形时间维度
- **tone** (口吻塑形) — 物种化塑形语言维度
- **morphology** (形态) — 物种化塑形 frontend
- **milestone** (里程碑) — 物种化塑形节点

### 2.6 反思 + 元认知缺口

- **meta_thinking** (元思考)
- **reflexion** (反思循环)
- **reflection** (反思, v2 R22 真实施) — 1:1 翻译 v1 donor
- **self_assessment** (v2 cognitive slot WIRED, Judge-backed)

### 2.7 协调 + 上下文缺口

- **onering** (单环协调)
- **oracle / oracle_adapters** (预言 / 适配器)
- **context / context_rot** (context window / 旋转)
- **continuation / continuity / spill** (连续性)
- **assemble / hello** (启动 / 装配)
- **thought_cluster** (思考聚类)
- **intent_brier** (Brier 校准意图)
- **confidence** (置信度)

### 2.8 工具 + 安全缺口

- **HybridCognitiveRouter** (master, hybrid routing)
- **ToolSynthesizer** (sandbox unused)
- **Scheduler** (periodic tasks)
- **Telemetry** (observability)
- **SelfDisableGuard** (binary hash check, scanner keyword-based, P2 ADAPT)
- **SovereignControl** (P2 ADAPT)
- **OmniParser** (Windows window enumeration)
- **DesktopActionTool** (Windows desktop action)
- **Voice whisper 真接** (R14 真 modality backend)

---

## 3. 主代理真账 — 1.0 vs 2.0 release 阻断清单

### 3.1 🔴 必补 (release 必补, 不可 DEFER)

按 vision.md "v2.0 = 1.0 功能全集 + 架构升级" + 用户原话 "2.0 最终功能应该和 1.0 相同":

| # | Feature | 估时 | 阻塞 | 派单建议 |
|---|---|---|---|---|
| 1 | **VectorIndex (BM25 hybrid 补, cosine ✅ 真账已就位)** | 1 周 | 0 | 派 sub-agent 真实施 BM25 hybrid 补 (per R11-Storage 真账 §1.2 hybrid 修订) |
| 2 | **Graph primitives (causal engine 补, graph primitives ✅ 真账已就位)** | 1-2 周 | 0 | 派 sub-agent 真实施 causal engine (per R11-Storage 真账 §1.3 修订) |
| 3 | **ToolSynthesizer** (sandbox unused fix) | 1 周 | 0 | 派 sub-agent 真调研 + 真实施 (security critical) |
| 4 | **daily_summary / diary + cross_diary + memory_injection** (长期记忆塑形) | 2-3 周 | 0 | 派 sub-agent 真调研 (Mio 真账 §5 #5 已推荐) + 真实施 (跟 R20 + R22 critical path) |
| 5 | **Vision ScreenCapture / pHash + OmniParser + DesktopActionTool** (Windows 真接) | 2-3 周 | 硬件 | D 块 RC-7 真 modality 真接 (Windows 真接已调研, 真实施需硬件) |
| 6 | **Voice whisper 真接** (`apeireth-voice/src/real.rs:824-938` 1:1 翻译) | 1-2 周 | 硬件 | R14 真 modality 真接 (Round 9 RC-7 真账已调研, 真 backend 接入) |
| 7 | **HybridCognitiveRouter** (master hybrid routing 真接) | 1-2 周 | 0 | 派 sub-agent 真调研 + 真实施 |
| 8 | **education** (物种化核心, vision L48 "能教养后代") | 2-3 周 | 0 | 派 sub-agent 真调研 + 主代理亲做 spec (物种化核心决策) |
| 9 | **partner + community + principles** (物种化跨墙信任 + 物种社区 + 哲学价值内化) | 3-4 周 | 0 | 派 sub-agent 真调研 + 主代理亲做 spec |
| 10 | **timeline + tone + morphology** (物种化塑形时间 + 语言 + 形态) | 2-3 周 | 0 | 派 sub-agent 真调研 + 真实施 (跟 Open-LLM-VTuber / Firefly / AIRI / Mio 借鉴) |
| 11 | **thought_cluster + intent_brier + confidence** (认知聚类 + Brier 校准 + 置信度) | 2-3 周 | 0 | 派 sub-agent 真调研 + 真实施 |
| 12 | **onering + oracle + oracle_adapters + meta_thinking + reflexion** (协调 + 预言 + 元思考 + 反思循环) | 3-4 周 | 0 | 派 sub-agent 真调研 + 真实施 |
| 13 | **context + context_rot + continuation + continuity + spill + assemble + hello + milestone** (context window + 连续性 + 启动 + 里程碑) | 2-3 周 | 0 | 派 sub-agent 真调研 + 真实施 |
| 14 | **experiment_field + proactive + progressive + pentest** (实验场 + 主动 + 渐进 + 渗透测试) | 2-3 周 | 0 | 派 sub-agent 真调研 + 真实施 |
| 15 | **Kani proofs (bridge_kani_proofs + organ_kani_proofs)** (形式化证明, 物种化 + 0 装诚实) | 2-3 周 | 0 | 派 sub-agent 真调研 + Kani 工具链实施 |
| 16 | **Invest tool + Learning tool + SystemMonitor tool** (P3 真接) | 1-2 周 | 0 | 派 sub-agent 真调研 + 真实施 |

### 3.2 🟡 Partial (release 可缓, post-release 排上)

约 **15 项**, 详 §2.2.

### 3.3 🟢 OK (v2 已就位)

约 **30 项**, 详 §2.1.

---

## 4. 派单 brief (sub-agent 真调研每项缺口)

### 4.1 派单原则 (per O-6 总体最优)

- **总估时 P0 必补 (~23 项, per Round 12 终极审计 + Round 13 1.0 maturity 补查)**: 估 **12-14 周 critical path** (修订主代理真账 §3.1 估 3-4 周 ❌ 偏乐观, 实际 12-14 周 = 1:1 翻译 + trait 口主代理亲做 spec + PARTIAL 真实施 critical path 累加). 主代理必亲做 spec ~2 周 (v1 rot_score 融合 + cognitive module trait + education 真 CAS + confidence BetaBinomial + reflexion 3 trait 口 + hello 主题 + git clone v2 master branch)
- **并行**: 派 5-6 sub-agent 真调研 (每个 ~2-3 周调研 + ~2-4 周真实施)
- **不重叠**: 借鉴链 per Round 10 5 真调研 + research/source 已借鉴
- **0 装诚实标**: 必含, 不假装 OK
- **5 重守门 baseline**: cargo test + clippy + LOCKED 0 触碰

### 4.2 派单顺序 (per O-6 + 用户 directive "多派几个")

**Round 11 P0 派单 (6 sub-agent 真调研具体 gap)**:
1. **Storage 抽象层真调研** (VectorIndex BM25 hybrid + Graph causal engine 补, 派 1 sub-agent, 1-2 周真实施) — **R11-Storage 真账就位** (主代理真账 §1.1 标 ❌ 错修订为 🟡 partial)
2. **长期记忆塑形真调研** (daily_summary / diary + cross_diary + memory_injection, 派 1 sub-agent, 2-3 周, 跟 Mio 真账 P0 调研同步)
3. **物种化核心真调研** (education + partner + community + principles, 派 1 sub-agent, 3-4 周, 跟 vision.md 真理解 species 核心)
4. **物种化塑形维度真调研** (timeline + tone + morphology, 派 1 sub-agent, 2-3 周, 跟 Round 10 Open-LLM-VTuber / Firefly / AIRI / Mio 调研对接)
5. **反思+元认知真调研** (meta_thinking + reflexion + thought_cluster + intent_brier + confidence, 派 1 sub-agent, 2-3 周)
6. **协调+上下文真调研** (onering + oracle + context + continuation + assemble + milestone + experiment_field + Kani proofs, 派 1 sub-agent, 3-4 周)

**Round 12 P1 派单 (工具 + 安全)**:
- ToolSynthesizer sandbox 修复
- Browser tool 真接 (Playwright MCP)
- HybridCognitiveRouter 真接
- Vision Windows 真接 (ScreenCapture + OmniParser + DesktopAction)

### 4.3 派单 brief 模板 (per sub-agent)

每个 sub-agent brief 必含:
- 任务: 具体 1.0 vs 2.0 功能 gap 真调研 + 写真账
- 必读: `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` (本文件) + 1.0 真账 `legacy/donor/apeireth-companion/src/<module>.rs` + 2.0 真账 `v2-reference-handbook-2026-08-28.md` + Round 9 6 真账 + Round 10 5 真调研
- 输出: `docs/04-internal/r11-<name>-gap-research-2026-08-28.md` (≤ 300 行)
- 写真账必含:
  - 1.0 真账 (maturity + path)
  - 2.0 现状 (maturity + path)
  - 0 装诚实标 (sub-agent 没 git clone v2 master branch, 仅读 2.0 真账推论)
  - 真实施建议路径 (P0/P1/P2, 估时, 借鉴链)
  - 主代理决策建议
- 约束: 不写真账以外的 file / 不 git add / commit / push / 0 触碰 LOCKED / ≤ 4h

---

## 5. 0 装诚实标 (per O-5)

| 失守 | 详情 | 修法 |
|---|---|---|
| **Round 1-10 局限视角** | 一直画 "v2 工程架构现状", 没看 1.0 vs 2.0 功能差距 — 之前没意识到 v2 release 必补 1.0 全集 | 本文件真账 + 派 sub-agent 真调研具体 gap |
| **约 23 项 1.0 功能 v2 缺** (修订: VectorIndex/Graph 从 🔴 缺 → 🟡 partial) | BM25 hybrid / causal engine / consolidation_writeback / daily_summary / cross_diary / memory_injection / 教育 / partner / community / principles / Kani / 等 | 派 sub-agent 真调研, 估 2-4 月 critical path 真补 |
| **本真账 0 实测** | 未 git clone v2 master branch (per master audit L138), 仅读 1.0 真账 (legacy/donor/) + 2.0 handbook + 5 R7 真调研 + master audit 真账推论 | 真实施前主代理必亲验 (per §4.2 派单顺序) |
| **数 1.0 真账基于 master audit L197-771 + glob 100+ modules 路径** | 实际 v1 master branch (~86-crate) 完整代码未逐行读, 仅按功能分类列 | 主代理派 sub-agent 逐 gap 真 clone v2 main + grep 跟 1.0 真对照 |

---

## 6. 留 backlog (Round 11 派单 + 真实施)

### 6.1 Round 11 P0 派单 (6 sub-agent 真调研)

| # | 派单 | 估时 | 阻塞 |
|---|---|---|---|
| 1 | 派 sub-agent 真调研 **Storage 抽象层 gap** (VectorIndex + Graph primitives) | 2-3 周 | 0 |
| 2 | 派 sub-agent 真调研 **长期记忆塑形 gap** (daily_summary / diary + cross_diary + memory_injection) | 2-3 周 | 0 (跟 R20 + R22 + Mio 调研并行) |
| 3 | 派 sub-agent 真调研 **物种化核心 gap** (education + partner + community + principles) | 3-4 周 | 0 (物种化核心, 主代理拍板 spec) |
| 4 | 派 sub-agent 真调研 **物种化塑形维度 gap** (timeline + tone + morphology) | 2-3 周 | 0 (跟 Round 10 Open-LLM-VTuber / Firefly / AIRI / Mio 借鉴) |
| 5 | 派 sub-agent 真调研 **反思+元认知 gap** (meta_thinking + reflexion + thought_cluster + intent_brier + confidence) | 2-3 周 | 0 (跟 R22 reflection 真实施并行) |
| 6 | 派 sub-agent 真调研 **协调+上下文 gap** (onering + oracle + context + continuation + assemble + milestone + experiment_field + Kani proofs) | 3-4 周 | 0 |

### 6.2 Round 12 P1 派单 (工具 + 安全)

| # | 派单 | 估时 | 阻塞 |
|---|---|---|---|
| 7 | ToolSynthesizer sandbox 修复 (sandbox unused 真接) | 1 周 | 0 (security critical) |
| 8 | Browser tool 真接 (Playwright MCP) | 1 周 | 0 (借 Open-LLM-VTuber Carbonyl 调研) |
| 9 | HybridCognitiveRouter 真接 | 1-2 周 | 0 |
| 10 | Vision Windows 真接 (ScreenCapture + OmniParser + DesktopAction) | 2-3 周 | D 块硬件 |
| 11 | Voice whisper 真接 (1:1 翻译 `apeireth-voice/src/real.rs:824-938`) | 1-2 周 | D 块硬件 |

### 6.3 修订 release 路径

- **v2.0 release 估时**: 原 2027-Q1 (per MANIFESTO §14 4-6 月), 现在 release 路径需重估:
  - 1.0 功能全集必补 (~23 项 P0 必补, 估 **12-14 周 critical path**, per Round 13 1.0 maturity 补查)
  - release 估时上调: 4-6 月 → **6-9 月** (P0 真补完, 真实施 critical path 12-14 周)
  - 物种化核心 (education / partner / community / principles) 估 3-4 周真调研 + 6-8 周真实施 = 2-3 月 critical path
- 修订 ROADMAP §7 总进度 (70% → 80% → **70-75% 重新估** 因 1.0 功能全集差距发现 + Round 13 1.0 maturity 补查, **降回 70-75%** 真实)
- 修订 MANIFESTO §14 release timeline (2027-Q1-Q2 → **2027-Q3**)

---

_Mavis 写于 2026-08-28 Round 11, 用户原话 "看 1.0 缺什么, 2.0 最终功能应该和 1.0 相同, 但架构不同而已" 触发, 主代理真账 1.0 vs 2.0 功能全集差距, ~25 项 P0 必补, 派 6 sub-agent 真调研 + Round 12 P1 真实施. 0 装诚实标: 0 git clone v2 master branch, 仅读 1.0 真账 (legacy/donor/) + 2.0 handbook + 5 R7 真调研 + master audit 真账推论, 真实施前主代理必亲验._
