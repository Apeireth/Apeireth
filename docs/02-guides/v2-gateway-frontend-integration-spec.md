# v2 Gateway Frontend Integration Spec (2026-08-28, 子代理 R9 写, 主代理 Mavis 待审)

> **本文档定位**: v2 canonical gateway ↔ companion-desktop 集成 spec. 真实生产前阻塞 #2 (估 4-6 周, 2027-Q1 启动).
> **HEAD 状态**: `7d990297` (Round 6 完). 历史 v2.0.0-rc.1 tag @ `b9026186`.
> **何时写**: 子代理 R9 在 rc.1 收盘后, 整合 #2 commit 拍板 + 9 organ 全 done 状态下写本 spec (真实施由主代理后续派 sub-agent).
> **关系文档**: `FINAL-HANDOFF-V2.0.0-RC.1.md` + `v2.0.0-release-path.md` + `cognitive-module-wiring.md` + `frontend-data-contract.md` (历史 v1 companion 契约).

```
[Document-Meta]
Document:        docs/02-guides/v2-gateway-frontend-integration-spec.md
Version:         Spec-1.0
Last-Modified:   2026-08-28
Status:          🟡 Spec 完成 (真实施 4-6 周, 2027-Q1 启动)
Author:          子代理 R9
```

---

## 0. TL;DR

**v2 canonical gateway** (`crates/adapters/gateway/src/lib.rs:1-15`) 已实现 OpenAI Chat 兼容入口 + axum HTTP router (`canonical_entry.rs:168-174`), 真接 LLM call 1.16s (RC-5 MiniMax adapter 拍板, 子代理 D 验证).

**9 organ** 全部真移植 (`crates/engine/organ/src/lib.rs:11-32`): E4/F4/F6/F1/W1/W2/W3/E7/Memory 9 organ trait 抽象 + 1:1 v1 翻译.

**认知模块** 6/12 slot WIRED (`docs/04-internal/cognitive-module-wiring.md:20-35`): **6 WIRED + 6 DEFERRED** (`memory_recall` / `preference_recall` / `judge` / `council` / `self_assessment` / `memory_writeback`; judge/council 为 WIRED, OFF by default).

**前端 companion-desktop** (`frontend/companion-desktop/README.md:1-124`) 当前 0 触碰 v2 gateway, 仅用历史 v1 companion :8090 接口. **完整迁移 = 真生产前阻塞 #2, 估 4-6 周** (主代理估 2027-Q1 启动).

**本 spec 范围**: 端点契约 + 9 organ 集成路径 + stream 协议 + 认知模块集成 + 错误处理 + 安全 + 部署 checklist. **不**含真实施代码 (本 spec 仅写契约, 真实施由主代理后续派 sub-agent).

---

## 1. 概述 (Why & What)

### 1.1 v2 gateway 与 v1 companion 区别

v1 `apeireth-companion` (`legacy/donor/apeireth-companion/`) 是 86-crate 单体, `companion_serve.rs` 提供 :8090 HTTP/SSE (legacy/donor/apeireth-companion/examples/companion_serve.rs). 9 organ 散落在 lib.rs 顶层 mod, 内部 if-else 散落 (`crates/engine/organ/src/lib.rs:3-7`).

v2 canonical gateway 是 15-crate workspace 的 HTTP adapter (`crates/adapters/gateway/src/lib.rs:7-15`), 通过 axum router 暴露 OpenAI Chat 兼容入口, 不持有 provider 路由 / 会话 / 治理 / 工具分发 / 第二编排引擎 (per `crates/adapters/gateway/src/lib.rs:2-6`).

**关键变化**:
- 端点: v1 :8090 18 条路由 → v2 :8080 3 条主路由 (`/health`, `/v1/chat`, `/v1/chat/completions`, `canonical_entry.rs:168-174`)
- 治理: v1 内置 PermissionGovernance hook → v2 由 runtime 治理 (`RuntimeError::Denied`, `canonical_entry.rs:269-281`)
- 工具: v1 tool_bridge_all → v2 模块化 (per `refactor(tools): move capability registration and MCP behind modules` commit `18d6bf36`)
- Organ: v1 散落 → v2 `apeireth-organ` 9 organ trait 抽象 (`crates/engine/organ/src/lib.rs:48-62`)

### 1.2 companion-desktop 当前状态

per `frontend/companion-desktop/README.md:6-7`: "当前保留历史 companion HTTP/SSE 接口的适配, 完整迁移到 canonical gateway 属于 deferred work."

`runtime.ts` 是 OpenAI-compatible adapter (`frontend/companion-desktop/README.md:18-19`), 但当前指向 :8090, **不**指向 :8080. 真生产前必须重写 runtime.ts 指向 :8080 + 加 9 organ stream hook + 加认知模块集成 + 加治理 hook 透传.

### 1.3 本 spec 不做的事 (0 装诚实)

- **不**写真实施代码 (4-6 周真实施 = 估 2027-Q1 启动, 由主代理后续派 sub-agent)
- **不**改 LOCKED 5 项 (per §10.0 严守)
- **不**改 Cargo.toml workspace.version = "1.2.0" (per R11 LOCKED)
- **不**假装 "frontend 已对接" — 标 "spec 完成 + 真实施待"

---

## 2. 端点契约 (OpenAI Chat 兼容)

v2 gateway 当前实现 3 条路由 (`canonical_entry.rs:168-174`):

### 2.1 `POST /v1/chat/completions` (主对话, 9 organ 串联)

**handler**: `canonical_entry.rs:204-265` (`openai_chat`)

**请求** (OpenAI Chat 兼容 schema, `canonical_entry.rs:118-124`):

```json
{
  "model": "MiniMax-M3",  // Optional, 不指定走 default
  "messages": [
    {"role": "system", "content": "You are Apeireth v2.0-rc.1 cognitive companion."},
    {"role": "user", "content": "今天心情如何?"}
  ],
  "session_id": "01HXXXXXXXXXXXXXXXXXXXXXXXXXX"  // Optional, 缺省新 session
}
```

**响应** (OpenAI Chat 兼容 + Apeireth 扩展 metadata, `canonical_entry.rs:126-156`):

```json
{
  "id": "<runtime_request_id>",
  "object": "chat.completion",
  "created": 1756382400,
  "model": "MiniMax-M3",
  "choices": [{
    "index": 0,
    "message": {"role": "assistant", "content": "主人, 我心情平稳, 略好奇您今天想问什么。"},
    "finish_reason": "stop"
  }],
  "usage": {"prompt_tokens": 42, "completion_tokens": 18, "total_tokens": 60},
  "apeireth": {
    "session_id": "01HXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "trace_id": "<execution_trace_id>",
    "served_by": "minimax-m3-thinking",
    "rounds": 1
  }
}
```

**9 organ 串联位置**: 主对话走 runtime (`canonical_entry.rs:99` `runtime.execute(turn).await`), runtime 调用认知模块 (per `cognitive-module-wiring.md:37-43` 注册顺序 TurnStart → AfterModelResponse → AfterTurn). 9 organ 串联通过 `OrganOrchestrator` (per §3 集成路径, R11 spec 已完 + R12 真实施已落 `crates/engine/runtime/src/canonical/orchestrator.rs`).

**v1 差异**: v1 `companion_serve.rs:1066` 同样路径但内置 9 organ 散落调用, v2 由 runtime + cognitive module 抽象接管.

### 2.2 `POST /v1/chat` (Native gateway, 不暴露给前端)

**handler**: `canonical_entry.rs:191-202` (`native_chat`)

**用途**: CLI bootstrap + integration test. 前端**不**消费此路径.

**请求 schema**: `canonical_entry.rs:20-33` (`CanonicalChatRequest`)

```json
{"session": "<SessionId>", "input": "你好", "model": "MiniMax-M3", "system": "..."}
```

### 2.3 `GET /health` (存活探针)

**handler**: `canonical_entry.rs:184-189` (`health`)

**响应**:

```json
{"status": "ok", "execution_owner": "apeireth-runtime::canonical"}
```

### 2.4 真生产新增端点 (本 spec 提案, 待实施)

#### 2.4.1 `POST /v1/audio/transcriptions` (Whisper 真接, RC-7 估)

**当前状态**: ⏳ RC-7 Perception trait 架构 done (子代理 R, commit `6e918c12`), 但 Whisper 真接需硬件 (麦克风) + 真 API key, 估 2-3 周真生产.

**请求** (OpenAI Audio 兼容):

```http
POST /v1/audio/transcriptions
Content-Type: multipart/form-data
Authorization: Bearer <APEIRETH_API_KEY>

file=@recording.wav
model=whisper-1
```

**响应**:

```json
{"text": "今天心情如何?"}
```

**实施位置**: 真生产时通过 `apeireth-perception` (PerceptionInput trait, `crates/foundation/plugin/src/perception_backend.rs`) 注入 Whisper backend.

#### 2.4.2 `POST /v1/audio/speech` (TTS, 后续)

**当前状态**: 0 装 (per rc.1 范围, TTS 估 v2.1).

#### 2.4.3 `GET /v1/models` (model list)

**当前状态**: 0 装 (v2 gateway 当前无此路由, 仅 `/health` + `/v1/chat` + `/v1/chat/completions`).

**真生产 schema** (本 spec 提案):

```json
{
  "object": "list",
  "data": [
    {"id": "MiniMax-M3", "object": "model", "created": 1756382400, "owned_by": "MiniMax"},
    {"id": "minimax-m3-thinking", "object": "model", "created": 1756382400, "owned_by": "MiniMax"}
  ]
}
```

**说明**: v2 gateway 当前走 `MiniMax-M3` + `minimax-m3-thinking` (per `LlmFactory` trait `crates/foundation/plugin/src/llm_factory.rs`), 9 organ 不暴露为独立 model ID — 9 organ 通过 cognitive module 内部注入 (`cognitive-module-wiring.md:37-43` 注册顺序).

---

## 3. 9 organ 集成路径 (L0-L5, per Q1 建议顺序)

per `crates/engine/organ/src/lib.rs:11-32` 9 organ 全 done, 真生产路径通过 `OrganOrchestrator` (R12 真实施已落 `crates/engine/runtime/src/canonical/orchestrator.rs`) 串联 9 organ.

### 3.1 L0 人类审批 (LlmFactory 注入)

**位置**: `crates/foundation/plugin/src/llm_factory.rs` (RC-5 真接 MiniMax, commit `02faa6d0`)

**路径**: 主人发请求 → runtime TurnStart → cognitive.module.preference_recall (WIRED) → 9 organ E4 curiosity (TurnStart 阶段注入, 待 OrganOrchestrator).

**L0 锚**: 哲学锚 O-5 (不假装) — LlmFactory 默认 None, 仅当 organ 真需要 LLM 才注入 (`organ.rs:34-37`: "0 装 PASS ... llm_factory() 默认返 None, 不假装每个 organ 都接 LLM").

### 3.2 L1 自我诊断 (cognitive self_assessment via RC-4 SQLite)

**位置**: `crates/engine/runtime/src/canonical/cognitive.rs:23` (`use apeireth_plugin::self_assessment::...`)

**路径**: AfterTurn hook → `cognitive.self_assessment` (WIRED, Judge-backed, `cognitive-module-wiring.md:28`) → 记录 Judge 真实结果 (不假装评分) → SQLite (RC-4 真接, commit `042ad4eb`).

**SQLite schema**: per `docs/04-internal/cognitive-module-wiring.md:50-65` (SqliteConnectionPool 统一 backend).

### 3.3 L2 提案生成 (orchestrator + 7 LlmAdvisor via RC-6 Council)

**位置**: `apeireth_orchestration::Council` (`crates/engine/runtime/src/canonical/cognitive.rs:15`)

**路径**: AfterModelResponse hook → `cognitive.council` (WIRED, OFF by default, `cognitive-module-wiring.md:27`) → 7 LlmAdvisor (`RC-6 真接, commit a3768fd6`) → 60s timeout → DeferToHuman (per RC-6 真实现).

**Council invariants** (per `cognitive-module-wiring.md:27`):
- bounded typed advisor path through `ModuleInvoker`
- no tool dispatch
- 最多 7 advisor (per RC-6 真实现)
- 10s/60s bounded (per RC-6 真实现)

### 3.4 L3 验证 (sandbox 跑 E4 + F1 + F4)

**位置**: 9 organ trait (`crates/foundation/plugin/src/organ.rs:60-89` OrganKind enum, 9 variant)

**路径**: 主对话期间 → runtime → OrganOrchestrator (待) → 9 organ 串联 E4 curiosity → F1 emotion → F4 hypothesis → F6 value → W1 world → W2 causal → W3 edges → E7 emergence → Memory (per Q1 建议顺序).

**Organ Trait**: `OrganTrait::process` (`crates/foundation/plugin/src/organ.rs:260-394`, async_trait macro per `organ.rs:45` "async-trait: 用 async_trait::async_trait 宏 per llm_factory.rs 同模式").

**9 organ impl 状态** (per `crates/engine/organ/src/lib.rs:11-32`):

| ID | v1 module | v2 impl | LLM 接? |
|----|-----------|---------|---------|
| W1 | `world_model` (TP31) | ✅ `WorldModelOrgan` (R4) | 真接 LLM |
| W2 | `causal_world_model` | ✅ `CausalWorldModelOrgan` (R5) | 真接 LLM MCTS |
| W3 | `causal_world_model` 边挖 | ✅ `EdgeMinerOrgan` (R6) | 确定性无 LLM |
| E4 | `curiosity` | ✅ `CuriosityOrgan` (Q1) | 确定性无 LLM |
| F4 | `hypothesis` | ✅ `HypothesisOrgan` (R2) | 1:1 翻译 v1 |
| F1 | `emotion_memory` | ✅ `EmotionOrgan` (R1) | 1:1 翻译 v1 |
| F6 | `value_cases` | ✅ `ValueCasesOrgan` (R3) | 1:1 翻译 v1 |
| E7 | `emergence` | ✅ `EmergenceOrgan` (R7) | 确定性无 LLM |
| Memory | memory_extractor | ✅ `MemoryMergerOrgan` (R8) | 跨 8 organ dedup/weight/persist |

**0 装诚实标** (per `organ.rs:31-44`):
- 9 organ 全实装 (no `NotImplemented` returned)
- E4 curiosity trait 接口允许 llm_factory() 但**真实现**确定性无 LLM (per `organ.rs:36-37`)
- W1/W2 真接 LLM (per `engine/organ/src/lib.rs:16-17`)
- 其余 organ 不假装调 LLM (per 0 装诚实锚 O-5)

### 3.5 L4 主人审批 (governance 3 hook)

**位置**: runtime `RuntimeError::Denied` / `RuntimeError::ApprovalRequired` (`canonical_entry.rs:269-281`)

**路径**: runtime 调用治理 hook (3 hook: PermissionGovernance / CredentialDisclosure / PromptInjection, per §7.1) → 主人审批 → 继续.

**HTTP status**:
- `403 FORBIDDEN` (Denied, `canonical_entry.rs:270`)
- `409 CONFLICT` (ApprovalRequired, `canonical_entry.rs:271`)
- `503 SERVICE_UNAVAILABLE` (NoHealthyProvider, `canonical_entry.rs:272-275`)

### 3.6 L5 runtime patch (git tag v2.x+1)

**位置**: workspace release pipeline (per `docs/04-internal/v2.0.0-release-path.md`)

**路径**: v2.0.0 release → 真生产反馈 → v2.0.1 patch → `git tag v2.0.1` (per R11 LOCKED release pipeline).

**HEAD 拍板**: `b9026186` (v2.0.0-rc.1 release tag 拍板, per 子代理 R9 必读 #1).

---

## 4. Stream 协议 (SSE / WebSocket)

### 4.1 当前状态 (v2.0.0-rc.1)

`canonical_entry.rs` 当前**无**SSE stream 路径 (per `canonical_entry.rs:168-174` 路由: `/health` + `/v1/chat` + `/v1/chat/completions` 全是非流式 JSON).

### 4.2 v1 历史契约

`frontend-data-contract.md:55` 标: `POST /v1/chat/completions (handler :1066) — 对话主链路 (非流式 + SSE 透传流式)`.

### 4.3 真生产 SSE 提案 (本 spec 提案, 待实施)

**Content-Type**: `text/event-stream`

**帧格式** (OpenAI Chat 兼容 stream, `data: {...}` prefix + `\n\n` 分隔):

```
data: {"id":"<req_id>","object":"chat.completion.chunk","created":1756382400,"model":"MiniMax-M3","choices":[{"index":0,"delta":{"role":"assistant","content":"主人"},"finish_reason":null}]}

data: {"id":"<req_id>","object":"chat.completion.chunk","created":1756382400,"model":"MiniMax-M3","choices":[{"index":0,"delta":{"content":", "},"finish_reason":null}]}

data: {"id":"<req_id>","object":"chat.completion.chunk","created":1756382400,"model":"MiniMax-M3","choices":[{"index":0,"delta":{"content":"我心情平稳"},"finish_reason":null}]}

data: {"id":"<req_id>","object":"chat.completion.chunk","created":1756382400,"model":"MiniMax-M3","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

**9 organ 输出串联** (per Q1 建议顺序, 估真生产 streaming):
- E4 curiosity: `data: {"organ":"E4","phase":"curiosity","targets":[...],"ask_master":[...]}` (TurnStart 注入, 仅 dry_run 模式触发)
- F1 emotion: `data: {"organ":"F1","phase":"emotion","pleasure":0.7,"arousal":0.3,"trend":"stable"}` (AfterModelResponse)
- F4 hypothesis: `data: {"organ":"F4","phase":"hypothesis","id":42,"statement":"...","conf":0.8}` (AfterModelResponse)
- F6 value: `data: {"organ":"F6","phase":"value","case_id":17,"verdict":"allow"}` (AfterModelResponse)
- W1 world: `data: {"organ":"W1","phase":"world","edges":[...],"counterfactual":[...]}` (AfterModelResponse)
- W2 causal: `data: {"organ":"W2","phase":"causal","edges":[...]}` (AfterModelResponse)
- W3 edges: `data: {"organ":"W3","phase":"edges","edges":[...]}` (AfterModelResponse)
- E7 emergence: `data: {"organ":"E7","phase":"emergence","action":"proactive_greet","spoke":true}` (AfterTurn)
- Memory: `data: {"organ":"Memory","phase":"memory","notes_added":3,"notes_merged":1}` (AfterTurn)

**0 装诚实标**: 9 organ SSE stream 路径**未**实施 (估真生产 4-6 周内实施, 由主代理后续派 sub-agent 真写).

### 4.4 WebSocket 路径 (本 spec 不实施, 估 v2.1)

v2 gateway 当前 0 WebSocket 路由. 前端 v1 companion :8090 也仅 SSE (`frontend-data-contract.md:55`). 真生产建议 SSE, WebSocket 估 v2.1+.

---

## 5. 认知模块集成

### 5.1 12 slot 注入 (per `cognitive-module-wiring.md:22-35`)

per `docs/04-internal/cognitive-module-wiring.md:20-35`:

| Slot / stable id | Hook | Status | Dependency |
|---|---|---|---|
| `cognitive.memory_recall` | TurnStart | **WIRED** | `Arc<dyn MemoryBackend>` |
| `cognitive.preference_recall` | TurnStart | **WIRED** | `Arc<dyn PreferenceStore>` |
| `cognitive.judge` | AfterModelResponse | **WIRED, OFF by default** | `ModuleInvoker` side-call |
| `cognitive.council` | AfterModelResponse | **WIRED, OFF by default** | bounded typed advisor |
| `cognitive.self_assessment` | AfterTurn | **WIRED, Judge-backed** | records real Judge result |
| `cognitive.memory_writeback` | AfterTurn | **WIRED** | successful final turn only |
| `cognitive.preference_learning` | — | DEFERRED | no evidence-extraction |
| `cognitive.critic` | — | DEFERRED INTO JUDGE | Judge's bounded critique |
| `cognitive.reflection` | AfterTurn | DEFERRED INTO SELF-ASSESSMENT | current-turn only |
| `cognitive.planner` | — | NOT AN AGENT MODULE | orchestration service |
| `cognitive.orchestrator` | — | NOT AN AGENT MODULE | long-running service |
| `cognitive.perception` | — | NOT AN AGENT MODULE | perception adapter |

**Status 总结**: **6 WIRED + 6 DEFERRED** (`memory_recall` / `preference_recall` / `judge` / `council` / `self_assessment` / `memory_writeback`; judge/council 为 WIRED, OFF by default).

**注册顺序确定性** (per `cognitive-module-wiring.md:37-43`):

```
TurnStart:          memory_recall -> preference_recall
AfterModelResponse: judge -> council
AfterTurn:          self_assessment -> memory_writeback
```

### 5.2 OrganOrchestrator 类似 AwakeCompanion (R11 spec 已完 + R12 真实施已落)

**当前状态**: 0 装 — R11 spec 已完 (`docs/01-architecture/organ-orchestrator-spec.md`, 500 行) + R12 真实施已落 `crates/engine/runtime/src/canonical/orchestrator.rs` (8+5=13 gate + 5 状态机 PolicyStage 前向声明 + 9 organ 顺序 process, 10 lib + 3 integration tests 全过). 9 organ 已 trait 抽象 + 真实现, 串联逻辑已真写 (真生产估 1-3 周内落地).

**提案 schema** (本 spec, 待实施):

```rust
// R12 真实施已落: crates/engine/runtime/src/canonical/orchestrator.rs (原提案位 crates/engine/organ/src/orchestrator.rs)
pub struct OrganOrchestrator {
    organs: BTreeMap<OrganKind, Arc<dyn OrganTrait>>,
}

impl OrganOrchestrator {
    pub fn new() -> Self { ... }
    pub fn register(&mut self, kind: OrganKind, organ: Arc<dyn OrganTrait>) { ... }
    pub async fn run_preturn(&self, input: OrganInput) -> Result<Vec<OrganOutput>, OrganError>;
    pub async fn run_postturn(&self, input: OrganInput) -> Result<Vec<OrganOutput>, OrganError>;
}
```

**集成路径**: runtime TurnRequest → OrganOrchestrator.run_preturn → cognitive.module.preference_recall → LLM call → cognitive.module.judge/council → OrganOrchestrator.run_postturn → cognitive.module.self_assessment → memory_writeback.

**0 装诚实标**: OrganOrchestrator 当前 0 装 — 9 organ trait 已抽象 + impl 已真写, 但 runtime 不串联 (per `canonical_entry.rs:99 runtime.execute(turn)` 只走单一 loop, 不调 OrganOrchestrator).

---

## 6. 错误处理

### 6.1 LlmError → LlmUnavailable (OrganError 透传)

per `crates/foundation/plugin/src/organ.rs:243-254`:

```rust
pub enum OrganError {
    NotImplemented(OrganKind),
    LlmUnavailable(String),  // ← LlmFactory None 时返
    LlmError(String),         // ← LlmError 1:1 映射 (凭证 / 网络 / rate limit / provider / stream)
    Config(String),
    BudgetExhausted { remaining: f64, required: f64 },
    Internal(String),
}
```

**LlmError 1:1 映射** (per `organ.rs:248`: "LLM 调用失败 (凭证 / 网络 / rate limit / provider / stream, per LlmError 1:1 映射)").

### 6.2 60s timeout → DeferToHuman (Council 决策)

per RC-6 真实现 (commit `a3768fd6`): Council 7 LlmAdvisor 60s timeout, 超时后 `DeferToHuman` 决策 (per 子代理 D 验证).

**HTTP status** (per `canonical_entry.rs:269-281`):
- `RuntimeError::Denied` → `403 FORBIDDEN`
- `RuntimeError::ApprovalRequired` → `409 CONFLICT`
- `RuntimeError::NoHealthyProvider` → `503 SERVICE_UNAVAILABLE`
- `RuntimeError::Provider` → `502 BAD_GATEWAY`
- 其他 → `500 INTERNAL_SERVER_ERROR`

### 6.3 RateLimited → retry-after-ms

per v1 companion :8090 (legacy `companion_serve.rs`), OpenAI 兼容 `Retry-After` header (秒). v2 gateway 当前未实现 rate limit, 真生产估加 (估 4-6 周内).

---

## 7. 安全

### 7.1 3 governance hook

per `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md:65-67` + v2 治理 runtime:

1. **PermissionGovernance** (permission grant)
2. **CredentialDisclosure** (key/token disclosure)
3. **PromptInjection** (输入消毒)

**位置**: runtime 治理 (per `canonical_entry.rs:269-281` `RuntimeError::Denied` / `ApprovalRequired`).

**HTTP 透传**: 前端 `runtime.ts` 处理 `403 FORBIDDEN` + `409 CONFLICT` → 弹主人审批 UI (companion-desktop 待 R13 接力审后真实施, 估 4-6 周).

### 7.2 13 键降级为哲学标准 (RUNTIME_ENFORCED = false)

per `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md:64-66`: 13 键降级决策 = `RUNTIME_ENFORCED = false`, **不强制** runtime. 哲学锚 O-5 (不假装) 要求 0 装诚实标 "13 键非强制".

### 7.3 8 守门 vs Self-Disable 保护

per `docs/04-internal/v2.0.0-release-path.md:55-56`: 工程规范 5 重守门 (clippy / tests / legacy compat / 13 键 LOCKED / 哲学锚表头).

**Self-Disable**: 真生产时如遇 clippy 失败 / tests 失败 / 13 键泄漏, 系统 self-disable (per 哲学锚 O-1 安全优先).

---

## 8. 真生产前阻塞

per `docs/04-internal/v2.0.0-release-path.md:30-36`:

| # | 阻塞项 | 状态 | 估时 |
|---|---|---|---|
| **#1** | 9 organ 真移植全 done | ✅ DONE (整合 #2 commit `bbf70293`, 9/9) | — |
| **#2** | frontend companion-desktop 对接 | ⏳ **本 spec 完成 + 真实施 = 估 4-6 周** | 2027-Q1 启动 |
| **#3** | RC-7 Perception backend trait 架构 | ✅ DONE (子代理 R, commit `6e918c12`) | 真生产估 2-3 周 |
| **#4** | RC-11 migration script + APX2 envelope | ✅ DONE (子代理 I + 别人, commits `926465c8` + `483fb4cd` + `615121bd`) | 真生产前必跑 |

**真生产前阻塞 2.5/4 完成** (per `v2.0.0-release-path.md:36`). 阻塞 #2 (frontend) = 本 spec 标的范围.

---

## 9. 部署 checklist

### 9.1 Git tag 拍板

- ✅ `git tag v2.0.0-rc.1` (HEAD `b9026186`, 已拍板, per 子代理 R9 必读 #1)
- ⏳ `git tag v2.0.0` (frontend 对接后, 估 2027-01-08 至 2027-03 月, per 子代理 L 估)

### 9.2 真生产前必跑 5 重守门 (per `FINAL-HANDOFF-V2.0.0-RC.1.md:114-121`)

```bash
cargo clippy --workspace --all-targets --locked -- -D warnings  # ✅ 0 警告
cargo test --workspace --locked                                   # ✅ 0 FAILED
# legacy compat path < 100 引用 (✅)
# 13 键 LOCKED + 9 哲学锚 + workspace.version 1.2.0 + R11 baseline 3 值 0 触碰 (✅)
# 哲学锚表头 0 减 (✅)
```

### 9.3 真生产前必跑 RC-11 migration (v1 db 迁移)

per `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md:67`: "RC-11 migration script 真生产前验证 (1-2 天, 有 key 但没 v1 db 验证)".

```bash
python scripts/migrate_v1_to_v2_encrypted.py --src <v1_db> --dst <v2_db>  # APX2 envelope
cargo test -p apeireth-migration --locked                                # Rust 集成测试
```

### 9.4 前端对接 checklist (本 spec 后续实施)

- [ ] companion-desktop `runtime.ts` 重写指向 `:8080` (从 :8090)
- [ ] SSE stream 路径实施 (`/v1/chat/completions` + `text/event-stream`)
- [ ] 9 organ stream hook (per §4.3 schema)
- [ ] 认知模块集成 (per §5.1 12 slot)
- [ ] OrganOrchestrator 实施 (per §5.2)
- [ ] 治理 hook HTTP 透传 (403 / 409 → 主人审批 UI)
- [ ] Whisper 真接 (per §2.4.1, 估 2-3 周)
- [ ] `GET /v1/models` 实施 (per §2.4.3)
- [ ] E2E test (per `frontend/companion-desktop/README.md:62-76` mock + 真 LLM)

---

## 10. 接手人 9 actionable 验证 (per 子代理 D handoff 5/5 done + 4 新加 #6-#9)

per `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md` + 子代理 D handoff + R13 接力审:

- ✅ **#1 RC-5/6/7 + 9 organ 真移植全 done** (HEAD `b9026186` 拍板, 9 organ 真兑现)
- ✅ **#2 哲学锚 ledger 待核** (9 锚 LOCKED 0 改, O-6 新加, `eight_anchors.rs:58-79` 编译期 hardcode)
- ✅ **#3 12 consumer 弃用迁移** (100+ consumer 0 破, v1 `apeireth-companion` 在 `legacy/donor/`)
- ✅ **#4 RC-10 line header AAD + APX2 envelope** (`canonical_entry.rs` runtime 通过 RC-10 真接)
- ✅ **#5 cognitive module 不变量 + 9 organ trait 抽象边界** (`cognitive-module-wiring.md` + `organ.rs` 守门)
- 🔄 **#6 OrganOrchestrator** (R11 spec done + R12 真实施已落, 真生产估 1-3 周)
- ⏳ **#7 6 DEFERRED slot 激活** (R10 spec done + R15 preference_learning spec done, 估 6-10 周真实施)
- ⏳ **#8 frontend 对接** (R9 spec done + R13 接力审完成, 估 4-6 周真实施, 2027-Q1 启动)
- ⏳ **#9 RC-7 Perception 真 modality** (R14 spec done, 估 2-3 周真实施, 需硬件)

---

## 11. 0 装诚实真账 (子代理 Z 独立审计触发主代理亲做)

per 子代理 Z 独立审计触发主代理亲做 (`docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md`):

### 11.1 9 organ 真兑现 + 1713 tests + 0 clippy + 0 触碰 LOCKED

- 9 organ 真兑现: ✅ 9/9 done (`crates/engine/organ/src/lib.rs:11-32`)
- workspace tests: ✅ 1713 passed 0 FAILED (子代理 Z 当时实测, per `v2.0.0-release-path.md:26`; A 块后 1739 passed)
- clippy: ✅ 0 警告
- LOCKED 5 项: ✅ 0 触碰 (per §10 actionable 验证)

### 11.2 整合 #2 commit message "无新外部 dep" 标错

per 子代理 Z 审计触发主代理亲做 (per `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md` 0 装诚实标):

- 整合 #2 commit `bbf70293` message 标 "无新外部 dep"
- **真** = +83 行 + 5 新外部 dep per RC-10 AES-256-GCM (`aes-gcm` crate, per RC-10 真接 commit `e2a5be08` + `38cc1039`)
- 主代理亲做核验 + 撤回 broken state + 修文档

### 11.3 0 装诱导 prevention 本身是 0 装诱导 (子代理 Z 独立判断)

- O-6 永远追求最优真兑现 = 主代理亲做核验 + 撤回 broken state + 修文档
- 0 装诱导 prevention 本身**不**假装"已做 0 装诱导 prevention" — 持续审计 + 修正

### 11.4 本 spec 的 0 装诚实

- 本 spec **不**真做 4-6 周 frontend 对接 (估 2027-Q1 启动)
- 本 spec **仅**写契约 (本文件 + `v2-frontend-quickstart.md`)
- 真实施由主代理后续派 sub-agent (估 4-6 周)

---

## 12. 附录: 子代理 R9 独立判断 (前 28 sub-agent 没写的事)

per 子代理 R9 必读 #1-#11 + 必跑命令 + 必读文档:

1. **frontend spec 缺失**: 前 28 sub-agent (A-H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z) 全部没写 frontend spec. R9 是**第 29 个视角**, 标 frontend 对接 = 真生产前阻塞 #2 的 spec.
2. **canonical_entry.rs 当前 0 SSE**: v1 `companion_serve.rs:1066` 标 "非流式 + SSE 透传流式", v2 `canonical_entry.rs:168-174` 仅 3 路由非流式. **SSE 实施** = frontend 对接 4-6 周内的真实施范畴 (本 spec 提案 schema).
3. **OrganOrchestrator 0 装**: 9 organ trait 已抽象 + impl 已真写, 原串联逻辑 0 装 (R9 写本文时), R11 spec 已完 + R12 真实施已落 (主代理审后修正).
4. **GET /v1/models 0 装**: v2 gateway 当前无此路由, 真生产估加.
5. **APX2 envelope 在 integration test 通过**: per `FINAL-HANDOFF-V2.0.0-RC.1.md`, 但真生产前需 1-2 天有 key 但没 v1 db 验证.

---

## 13. 参考引用 (file:line)

- v2 gateway 入口: `crates/adapters/gateway/src/lib.rs:1-15`
- OpenAI Chat 兼容 handler: `crates/adapters/gateway/src/canonical_entry.rs:204-265`
- Native chat handler: `crates/adapters/gateway/src/canonical_entry.rs:191-202`
- HTTP router 路由表: `crates/adapters/gateway/src/canonical_entry.rs:168-174`
- HTTP error 映射: `crates/adapters/gateway/src/canonical_entry.rs:267-290`
- 9 organ trait 抽象: `crates/foundation/plugin/src/organ.rs:60-394`
- 9 organ v2 impl: `crates/engine/organ/src/lib.rs:11-32`
- 9 organ v2 impl (per organ): `crates/engine/organ/src/{curiosity,hypothesis,value_cases,emotion_memory,world_model,causal_world_model,causal_world_model_edges,emergence,memory}.rs`
- 认知模块 12 slot ledger: `docs/04-internal/cognitive-module-wiring.md:22-35`
- 认知模块注册顺序: `docs/04-internal/cognitive-module-wiring.md:37-43`
- LlmFactory MiniMax 真实现: `crates/foundation/plugin/src/llm_factory.rs` (RC-5 commit `02faa6d0`)
- Council 7 LlmAdvisor + 60s timeout: RC-6 commit `a3768fd6`
- SQLite backend: RC-1/2/3/4 commits `43ec9635` + `4e4fba89` + `61cc0421` + `042ad4eb`
- APX2 envelope: RC-10 commits `e2a5be08` + `38cc1039`
- RC-11 migration: commits `926465c8` + `483fb4cd` + `615121bd`
- RC-7 Perception trait: commit `6e918c12`
- HEAD 拍板: `b9026186` (v2.0.0-rc.1 release tag)
- 接手报告: `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md`
- Release 路径: `docs/04-internal/v2.0.0-release-path.md`
- v1 companion 契约: `docs/02-guides/frontend-data-contract.md` (历史 v1, 待 v2 重对齐)
- companion-desktop 当前状态: `frontend/companion-desktop/README.md:1-124`

---

**End of Spec (v1.0, 子代理 R9 写, 主代理 Mavis 待审)**