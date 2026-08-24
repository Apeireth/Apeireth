# Master Functionality Port Audit

## 1. Git State

```text
Canonical branch: reconstruct_v2
Canonical HEAD: 21646e79b82e7cffc95b0942604111c4e7b3a9b3
origin/reconstruct_v2: b34418834fce69fd2332c09580a1e214d18e3a01
origin/master: 46c25b4bfb258351fb2185baad22b0b3eb7b302d
Merge base (HEAD, origin/master): b0a017f060841119b46d79e28870732ae80e1aed
Working tree: clean at audit start; only this report added
```

Verification performed:

- `git fetch origin` succeeded.
- `git log HEAD..origin/reconstruct_v2` is empty: remote has **not** advanced.
- `origin/reconstruct_v2` is the direct ancestor of local `HEAD`; the local branch is
  ahead by the four freeze commits (736962db, 0b17f66c, 21bd45e6, 21646e79).
- Fast-forward push is permitted by the precondition rules.

---

## 2. Audit Scope

Crates inspected under `origin/master:reconstruction_v2/crates`:

```text
apeireth-core
apeireth-governance
apeireth-storage
apeireth-protocol
apeireth-tools
apeireth-runtime
apeireth-companion
apeireth-gateway
apeireth-cli
apeireth-sdk
apeireth-avatar
apeireth-voice
apeireth-bridge
apeireth-pybridge
```

Feature groups inspected:

```text
Storage / SQLite / migrations / vector / graph / memory
Tools / builtin tools / sandbox / worktree / vision / desktop action
Runtime services / session / scheduler / telemetry / event bus / lifecycle
Companion / emotion / dream / curiosity / world model / prompt assembly
Provider / protocol adapters / DTO / parser / streaming frames
Gateway / REST endpoints / SSE / WS / MCP / egress
CLI / TUI / SDK / voice / avatar / bridge / pybridge
Security: credentials, SSRF, shell, filesystem, desktop, MCP, CoT exposure
```

Canonical references read:

```text
ARCHITECTURE.md
docs/01-architecture/canonical-skeleton-freeze-audit.md
crates/apeireth-runtime/src/canonical/*
crates/apeireth-plugin/src/*
crates/apeireth-governance/src/*
crates/apeireth-provider/src/canonical_*
crates/apeireth-gateway/src/canonical_entry.rs
crates/apeireth-cli/src/lib.rs (canonical bootstrap)
```

---

## 3. Canonical Freeze Summary

Authoritative rules used for this audit (from `ARCHITECTURE.md`):

1. Runtime is the only orchestration root: `Runtime::execute` owns session lifecycle,
   governance evaluation, provider selection/invocation, tool dispatch/continuation,
   failure persistence, and trace.
2. Gateway and CLI are transport/bootstrap adapters only.
3. Runtime never branches on vendor identity.
4. `ProviderCapability` owns vendor invocation; providers get secrets via
   `CredentialResolver` and receive redacted `Secret` values only.
5. Protocol owns canonical DTOs and vendor wire translation, not HTTP clients,
   credentials, retry, routing, or fallback.
6. Providers never execute tools.
7. `PluginRegistry` / `CapabilityRegistry` are the only capability ownership system.
   No second registries (`ToolRegistry`, `McpRegistry`, `SkillRegistry`, ...).
8. Governance returns `Allow` / `Deny` / `RequireApproval`.
9. One canonical session ownership path belongs to Runtime.
10. Public canonical contracts never expose raw chain-of-thought
    (`reasoning_cot`, `raw_chain_of_thought`, `reasoning_content`).
11. MCP is a transport/capability boundary, not a parallel plugin/tool ecosystem.
12. Raw private CoT must not enter public canonical contracts.

---

## 4. Master Architecture Summary

Workspace layout: `reconstruction_v2` is a self-contained workspace with 14 crates
(including `apeireth-pybridge`, which canonical `reconstruction_v2` does not have).

Actual architecture found:

- **Runtime composition**: `apeireth-runtime::host::UnifiedRuntimeHost` is a single
  God Object that owns `api_key: String`, `default_model`, `SessionManager`,
  `EventBusBackbone`, `LifecycleHandle`, `SqliteConnectionPool`, `MemoryStore`,
  `Plutchik`, `BorbelyModel`, `ContextAssembler`, `ToolRegistry`, `PlatformSandbox`,
  `ModelRouter`, `CapabilityRegistry`, `PresenceHub`, `DreamEngine`,
  `EpistemicHealer`, `HybridCognitiveRouter`, `ToolSynthesizer`, `WorktreeSandbox`,
  `Telemetry`, `Scheduler`, `ExperienceQueue`, and `CuriosityEngine`.
  `handle_chat_turn` implements the agent/tool loop directly on the host object.

- **Provider model**: `apeireth-protocol::ProtocolAdapter::execute(api_key, request)`
  owns HTTP transport and credentials per call. `ModelRouter` does
  `exact:` / `prefix:` / fallback routing. Adapters exist for OpenAI, Anthropic,
  Gemini, MiniMax. The runtime default is MiniMax with hardcoded default model
  `MiniMax-Text-01`.

- **Gateway model**: `apeireth-gateway::server::build_router` mounts ~30 routes and
  handlers reach **directly** into `host.tool_registry`, `host.session_manager`,
  `host.memory_store`, `host.model_router`, `host.presence_hub`, and
  `host.lifecycle_handle.audit_chain`. Several endpoints return hardcoded/mock JSON.

- **Storage**: `apeireth-storage` has a real SQLite pool with a dedicated write
  channel, WAL pragmas, migrations (simple `CREATE TABLE IF NOT EXISTS`), a
  `MemoryStore` over a single `facts` JSON table, in-memory `VectorIndex` (cosine +
  BM25 hybrid), and graph primitives (`CausalGraph`, `MctsCausalSimulator`). Many
  `memory_*` modules are small in-memory simplified/stub modules copied from v1.

- **Tools**: `apeireth-tools` has a master `ToolRegistry` (canonical violation),
  built-in tools (shell, filesystem, fetch, browser, search, repo, invest, learning,
  system_monitor), vision tools (screen capture, OmniParser window enumeration,
  desktop_action), worktree/synthesis sandboxing, and an MCP client/server module.

- **Companion**: `apeireth-companion` has ~100 modules. Key real implementations:
  Plutchik/PAD emotion, Borbely drive, DreamEngine (rule-based triple extraction,
  but W2/W3 world-model simulators are explicitly stubs), CuriosityEngine scoring,
  EpistemicHealer keyword-based distillation, observer capture queue, and a prompt
  assembler that injects identity, memory, PAD, and tool descriptions.

---

## 5. Architecture Comparison Matrix

```text
Concept
Canonical reconstruct_v2
Master reconstruction_v2
Winner / authoritative
Reuse opportunity
```

| Concept | Canonical | Master | Winner | Reuse opportunity |
| --- | --- | --- | --- | --- |
| Runtime root | `apeireth-runtime::canonical::Runtime` + `Runtime::execute` | `UnifiedRuntimeHost::handle_chat_turn` | Canonical | Decompose host functions; do not import object |
| Provider abstraction | `ProviderCapability` + `CredentialResolver` + `Secret` | `ProtocolAdapter::execute(api_key, request)` | Canonical | Reuse adapter DTO/serializer/parser only |
| Model routing | `ProviderRouter` + `ModelDescriptor` in runtime canonical | `ModelRouter` with `exact:` / `prefix:` / fallback | Canonical | Drop master architecture; small route-matching utility at most |
| Credentials | `CredentialResolver` -> `Secret`, logical names | raw `api_key: String`, fixed path `C:\Users\31683\apikey-ultra.txt` | Canonical | None |
| Protocol | `apeireth-protocol::canonical` DTOs; stateless adapters | `normalized` DTOs + adapters that own reqwest | Canonical | Reuse DTO/parser/serializer codecs |
| Tool registry | `apeireth-plugin` `PluginRegistry` + `CapabilityRegistry` | `apeireth-tools::ToolRegistry` + runtime `CapabilityRegistry` | Canonical | Reuse concrete tool implementations behind `ToolCapability` |
| Capability registry | `apeireth-plugin::CapabilityRegistry` (index over manifests) | master `apeireth-runtime::capability_registry` wraps `ToolRegistry` | Canonical | None |
| Session | `apeireth-runtime::canonical::Session` + `SessionStore` seam | `SessionManager` in-memory `HashMap` with cloned `SessionState` | Canonical | Absorb selective metadata/timestamps; do not import second owner |
| Trace | `ExecutionTrace` / `SessionEvent` structured events | `AuditHashChain` + `Telemetry` atomics | Canonical | Reuse audit-chain hash primitive as governance hook, not as trace |
| Governance | `GovernanceHook` / `Decision` Allow/Deny/RequireApproval | 5-gate pipeline, onion, PII, audit chain | Canonical semantics; reuse policy implementations | ADAPT PII/audit/onion into hooks |
| Storage | `apeireth-storage` (durability) not yet created; `InMemorySessionStore` seam | real SQLite pool + MemoryStore + VectorIndex + graph | Canonical owner + master implementation | Strong donor: DIRECT_PORT pool/memory/vector/graph primitives |
| Memory | companion cognition consumes session; durable store in storage | `MemoryStore` inside storage; runtime retrieves ACT-R items | Split: storage vs companion vs runtime | ADAPT MemoryStore into storage; retrieval into runtime/companion |
| Gateway | `canonical_entry` calls `Runtime::execute` | Axum router with direct host subsystem access | Canonical | ADAPT selected endpoints; drop direct access and mock routes |
| Streaming | canonical `StreamEvent` in protocol; gateway transport SSE | `WsFrame`, `SseBroadcaster`, presence SSE | Canonical contract + master transport pieces | Reuse SSE broadcaster/WS frame parser with canonical payloads |
| MCP | transport/capability boundary only | `McpClient`, `McpServer`, `StdioTransport` + direct `ToolRegistry` injection | Canonical rule + master protocol/transport code | ADAPT MCP protocol/transport; remove `McpRegistry`/direct injection |
| Companion | `apeireth-companion` owns emotion/dream/world model | same crate with many small modules and stubs | Canonical | ADAPT real emotion, dream scoring, Borbely; drop stubs |

---

## 6. Architecture Conflict Matrix

| Master component | Canonical replacement | Conflict? | Action |
| --- | --- | --- | --- |
| `UnifiedRuntimeHost` | `Runtime` + `RuntimeBuilder` + `Runtime::execute` | YES | Decompose; never port object |
| `ModelRouter` | `ProviderRouter` | YES | DROP architecture; maybe reuse tiny matcher utility |
| `ProtocolAdapter::execute(api_key, request)` | `ProviderCapability` with `CredentialResolver` | YES | Reuse codec/serializer/parser only; DROP trait shape |
| `ToolRegistry` (master tools crate) | `PluginRegistry` + `CapabilityRegistry` + `ToolCapability` | YES | DROP registry; port tool implementations as capabilities |
| `CapabilityRegistry` (runtime) | `apeireth-plugin::CapabilityRegistry` | YES | DROP duplicate registry |
| `SessionManager` (runtime, in-memory only) | `apeireth-runtime::canonical::SessionManager` + `SessionStore` | YES | ABSORB selected semantics; do not import second owner |
| `EventBusBackbone` | `apeireth-core::kernel::Event` + runtime trace events | PARTIAL | ADAPT generic event primitives; do not duplicate trace |
| `LifecycleHandle` facade | Runtime builder wiring | PARTIAL | ADAPT wiring pattern only, not as ownership facade |
| Gateway direct `host.tool_registry` / `host.memory_store` / `host.session_manager` | `Runtime::execute` + canonical service boundary | YES | ADAPT endpoints through Runtime; drop direct access |
| Master `CapabilityRegistry` skills/agents metadata | canonical plugin manifest capability declarations | YES | Map into plugin manifests later |
| `reasoning_cot` / `reasoning_content` public fields | canonical `ExecutionTrace` / no raw CoT | YES | DROP raw CoT exposure; adapt callers to structured trace |
| `api_key: String` fields and fixed key path | `CredentialResolver` -> `Secret` | YES | DROP; canonical bootstrap already does this |
| `McpServer` bound to `Arc<ToolRegistry>` | MCP transport -> `ToolCapability` declarations | YES | ADAPT; remove duplicated registry binding |

---

## 7. Full Feature Inventory

Maturity labels: `REAL`, `PARTIAL`, `STUB`, `MOCK`, `PLACEHOLDER`, `DOC-ONLY`.

| Feature | Source path | Maturity | Canonical target | Strategy | Priority | Dependencies | Tests | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| SQLite pool + write channel | `crates/apeireth-storage/src/pool.rs` | REAL | new canonical `apeireth-storage` | DIRECT_PORT | P0 | rusqlite, r2d2, tokio | unit + concurrent read/write in `lib.rs` | WAL, single writer, reader pool |
| Migrations | `crates/apeireth-storage/src/migrations.rs` | REAL | new canonical `apeireth-storage` | DIRECT_PORT | P0 | rusqlite | exercised by storage tests | simple but honest `CREATE TABLE IF NOT EXISTS` |
| MemoryStore v2 (ACT-R, temporal, tombstone) | `crates/apeireth-storage/src/memory_v2.rs` | REAL | storage layer; runtime/companion retrieval clients | ADAPT | P1 | SQLite pool, clock | `lib.rs` memory tests, CJK, clustering | stores JSON in one facts table; no normalized schema |
| VectorIndex (cosine + BM25 hybrid) | `crates/apeireth-storage/src/vector.rs` | REAL | storage/retrieval layer | DIRECT_PORT | P1 | in-memory only | `vector.rs` tests | no persistence; useful as first retrieval primitive |
| Graph primitives / causal graph | `crates/apeireth-storage/src/graph.rs`, `graph_primitive.rs`, `graph_ops.rs`, `fold.rs` | PARTIAL | storage/graph capability | ADAPT | P2 | std only | graph traversal tests in `lib.rs` | BFS/MCTS-like but simplified; not a full causal engine |
| Memory_* support modules | `crates/apeireth-storage/src/memory_*.rs` | PARTIAL | storage or companion modules | ADAPT/DEFER | P2 | std | many small unit tests | most are simplified in-memory stores; ONNX is stub |
| ToolRegistry (master) | `crates/apeireth-tools/src/lib.rs` | REAL | DROP; canonical plugin registry | DROP | - | - | `lib.rs` tests | canonical violation |
| Shell tool | `crates/apeireth-tools/src/builtin/shell.rs` | PARTIAL | `ToolCapability` implementation plugin | ADAPT | P1 | tokio process | shell tests | sanitizer is blacklist; sandbox not actually attached |
| Filesystem tool | `crates/apeireth-tools/src/builtin/filesystem.rs` | REAL | `ToolCapability` implementation plugin | ADAPT | P1 | tokio fs | fs tests incl. traversal | basic path traversal check; no canonicalize |
| Fetch tool | `crates/apeireth-tools/src/builtin/fetch.rs` | PARTIAL | `ToolCapability` implementation plugin | ADAPT | P1 | reqwest | SSRF test | DNS-based SSRF check; no redirect re-check, no allowlist |
| Browser tool | `crates/apeireth-tools/src/builtin/browser.rs` | PARTIAL | `ToolCapability` implementation plugin | ADAPT | P2 | reqwest | none dedicated | HTML stripping only; no SSRF check; proxy fallback |
| Search tool | `crates/apeireth-tools/src/builtin/search.rs` | REAL | `ToolCapability` implementation plugin | DIRECT_PORT | P1 | std | no dedicated test file | recursive local search; simple and useful |
| Repo tool | `crates/apeireth-tools/src/builtin/repo_tools.rs` | REAL | `ToolCapability` implementation plugin | DIRECT_PORT | P1 | git subprocess | no dedicated test file | safe git read-only commands |
| Invest tool | `crates/apeireth-tools/src/builtin/invest.rs` | PARTIAL | `ToolCapability` implementation plugin | DEFER/ADAPT | P3 | reqwest | risk-planning test | fallback hardcoded quote when API unavailable |
| Learning tool | `crates/apeireth-tools/src/builtin/learning.rs` | PARTIAL | `ToolCapability` implementation plugin | DEFER | P3 | std | digest test | deterministic learning digest; no external dependency |
| SystemMonitor tool | `crates/apeireth-tools/src/builtin/system_monitor.rs` | PARTIAL | `ToolCapability` implementation plugin | DEFER | P3 | winapi (Windows) | system monitor test | Windows-specific; non-Windows path returns fallback |
| PlatformSandbox | `crates/apeireth-tools/src/sandbox.rs` | PARTIAL | tool sandbox capability | ADAPT | P1 | winapi/libc | sandbox lifecycle test | JobObject real on Windows, but ShellTool/SyntheticTool do not assign processes; Linux uses prctl/rlimit; non-Windows stub honest |
| WorktreeSandbox | `crates/apeireth-tools/src/worktree.rs` | REAL | autonomous factory capability | DEFER/ADAPT | P2 | git subprocess | patch-set test | real git worktree pipeline; broad application-layer feature |
| ToolSynthesizer | `crates/apeireth-tools/src/synthesis.rs` | PARTIAL | tool capability factory | DEFER/ADAPT | P3 | std/tokio | synthesis test | writes temp script and runs unsandboxed; sandbox field unused |
| Vision ScreenCapture / pHash | `crates/apeireth-tools/src/vision/screen.rs` | REAL (Windows) | screen perception capability | DEFER/ADAPT | P2 | winapi (Windows) | no dedicated test file | real GDI capture on Windows; non-Windows returns None |
| OmniParser window enumeration | `crates/apeireth-tools/src/vision/omni_parser.rs` | REAL (Windows) | screen perception capability | DEFER/ADAPT | P2 | winapi | no dedicated test file | real window enumeration; non-Windows empty |
| DesktopActionTool | `crates/apeireth-tools/src/vision/desktop_action.rs` | PARTIAL | desktop action capability | DEFER/ADAPT | P2 | winapi | taskbar/shell tests | real SendInput but no governance, no per-action policy; coordinate bounds only |
| MCP protocol/client/server/transport | `crates/apeireth-tools/src/mcp/*` | REAL | MCP transport boundary | ADAPT | P1 | tokio/serde | `crates/apeireth-tools/tests/mcp_test.rs` | protocol solid; server binds master `ToolRegistry` (canonical violation) |
| Governance 5-gate pipeline | `crates/apeireth-governance/src/gates.rs` | PARTIAL | `GovernanceHook` implementations | ADAPT | P1 | core philosophy | gates tests | compile/runtime/council/physical/reflection gates; budget hardcoded in host |
| Governance onion (ABAC) | `crates/apeireth-governance/src/onion.rs` | PARTIAL | `GovernanceHook` implementations | ADAPT | P2 | std | onion tests | simple permission set, not full ABAC |
| PII detector / injection check | `crates/apeireth-governance/src/guard.rs` | REAL | `GovernanceHook` implementation | DIRECT_PORT | P1 | regex | guard tests | scrub email/phone/sk-like keys; simple injection patterns |
| AuditHashChain | `crates/apeireth-governance/src/audit.rs` | REAL | governance/audit primitive | DIRECT_PORT | P1 | sha2 | audit tests incl. tamper | useful primitive; not a replacement for canonical trace |
| SelfDisableGuard | `crates/apeireth-governance/src/self_disable.rs` | PARTIAL | security hook | ADAPT | P2 | sha2 | self-disable tests | binary hash check; scanner is keyword-based |
| SovereignControl | `crates/apeireth-governance/src/sovereignty.rs` | REAL | governance/security hook | ADAPT | P2 | sha2 | sovereignty tests | role-based pause/resume with hashed token |
| EventBus (core) | `crates/apeireth-core/src/bus.rs` | REAL | core event primitive | DIRECT_PORT | P1 | tokio broadcast | bus tests | generic; canonical core already owns event concept |
| EventBusBackbone | `crates/apeireth-runtime/src/event_bus_backbone.rs` | REAL | runtime service / core event channels | ADAPT | P2 | core bus | backbone tests | multi-channel; do not duplicate canonical trace |
| Scheduler | `crates/apeireth-runtime/src/scheduler.rs` | PARTIAL | runtime service | ADAPT | P2 | tokio | no dedicated test file | periodic tasks; must not become second Runtime |
| Telemetry | `crates/apeireth-runtime/src/telemetry.rs` | PARTIAL | observability adapter | ADAPT | P2 | std/tracing | no dedicated test file | atomic metrics only; no export/OTel |
| SessionManager (master) | `crates/apeireth-runtime/src/session_manager.rs` | REAL | canonical Runtime session | ABSORB | P0 | in-memory | session tests | in-memory only; no persistence; do not import as second owner |
| UnifiedRuntimeHost | `crates/apeireth-runtime/src/host.rs` | REAL | DROP; decompose | REIMPLEMENT per feature | P0 | many | host dream test | God Object; raw api_key; raw CoT; direct subsystem wiring |
| HybridCognitiveRouter | `crates/apeireth-runtime/src/hybrid.rs` | PARTIAL | companion/service router | DEFER | P3 | none | no dedicated test file | rule-based fast path with hardcoded templates |
| Protocol normalized DTOs | `crates/apeireth-protocol/src/normalized.rs` | REAL | protocol canonical DTOs | DIRECT_PORT (codec) | P0 | serde | no dedicated test file | already mirrored in canonical; use master for tool-call text fallback parser |
| Provider adapter DTO/parsers | `crates/apeireth-protocol/src/adapters/*.rs` | REAL | provider codec layer | LOW-LEVEL REUSE ONLY | P1 | reqwest/serde | adapter parse tests | serializers/parsers useful; `execute(api_key, ...)` architecture DROP |
| WsFrame / voice VAD | `crates/apeireth-protocol/src/ws.rs`, `voice.rs` | REAL | protocol/transport/voice | ADAPT | P2 | serde | frame/VAD tests | `CoTDelta` frame violates raw CoT rule |
| Gateway router/endpoints | `crates/apeireth-gateway/src/server.rs` | MIXED | gateway transport layer | ADAPT | P1 | axum | gateway health test | many real; several mock/hardcoded; direct host access |
| Gateway SSE broadcaster | `crates/apeireth-gateway/src/sse.rs` | REAL | gateway SSE transport | DIRECT_PORT | P2 | tokio | no dedicated test file | generic typed SSE broadcaster |
| Gateway egress filter | `crates/apeireth-gateway/src/egress.rs` | REAL | gateway/provider egress policy | ADAPT | P1 | sha2 | egress tests | default-deny allowlist; not wired into fetch/browser |
| Gateway MCP handler | `crates/apeireth-gateway/src/mcp.rs` | PARTIAL | MCP transport boundary | ADAPT | P2 | axum | no dedicated test file | binds master `ToolRegistry` |
| Emotion Plutchik/PAD | `crates/apeireth-companion/src/emotion.rs` | REAL | companion emotion | DIRECT_PORT | P1 | serde | emotion tests | deterministic mapping/decay |
| Borbely drive / rhythm | `crates/apeireth-companion/src/emergence.rs` | REAL | companion | DIRECT_PORT | P1 | std | emergence tests | simple and deterministic |
| DreamEngine | `crates/apeireth-companion/src/dream.rs` | PARTIAL | companion dream | ADAPT | P2 | world_model_v1, brier | dream tests | triple extraction real; W2/W3 are explicit stubs; hardcoded sleep pressure output |
| CuriosityEngine | `crates/apeireth-companion/src/curiosity.rs` | PARTIAL | companion | ADAPT | P2 | std | no dedicated test file | score function only; no memory interaction or state |
| EpistemicHealer | `crates/apeireth-companion/src/epistemic.rs` | PARTIAL | companion | ADAPT | P2 | std | epistemic tests | keyword-based root cause rules |
| ExperienceQueue | `crates/apeireth-companion/src/observer_capture.rs` | REAL | companion | DIRECT_PORT | P2 | std | no dedicated test file | bounded observation queue |
| PromptAssembler | `crates/apeireth-companion/src/prompt_assembler.rs` | REAL | companion prompt assembly | ADAPT | P1 | emotion types | prompt assembler test | includes "thinking chain" directives; must strip raw CoT public handling |
| WorldModel v1 / causal | `crates/apeireth-companion/src/world_model_v1.rs`, `causal_world_model.rs` | PARTIAL/STUB | companion world model | DEFER | P3 | std | world model tests | W2/W3 explicitly stubbed |
| Voice VAD/duplex | `crates/apeireth-voice/src/vad.rs`, `crates/apeireth-protocol/src/voice.rs` | REAL | voice capability | ADAPT | P2 | std | voice/VAD tests | energy VAD + barge-in deterministic |
| Voice TTS | `crates/apeireth-voice/src/tts.rs` | STUB | voice capability | DEFER | P3 | reqwest | no dedicated test file | synthetic sine-wave PCM; SSML only |
| Voice lipsync | `crates/apeireth-voice/src/lipsync.rs` | PARTIAL | voice/avatar | DEFER | P3 | std | no dedicated test file | RMS/ZCR viseme mapping real but synthetic timing |
| Avatar expression/state | `crates/apeireth-avatar/src/*` | PARTIAL | avatar/desktop | DEFER | P3 | serde | avatar state test | no renderer; parameter mapping only |
| Bridge clients | `crates/apeireth-bridge/src/*` | PARTIAL | bridge/application | DEFER | P3 | reqwest/jsonwebtoken | bridge tests | Discord/Telegram/OneBot/Lark/LiveKit/Stock have real HTTP/JWT pieces; web/game are stubs |
| SDK client/session/memory/tool | `crates/apeireth-sdk/src/*` | PARTIAL | SDK API surface | ADAPT | P2 | reqwest | SDK init test | real HTTP client; some response types mismatch gateway JSON |
| PyBridge | `crates/apeireth-pybridge/src/lib.rs` | PARTIAL | SDK/bindings | DEFER | P3 | pyo3/ureq | pybridge tests | real HTTP to gateway; application-layer |
| CLI/TUI | `crates/apeireth-cli/src/*` | PARTIAL | CLI adapter | ADAPT | P2 | clap/ratatui | CLI parse tests | CLI is an adapter; TUI state lives in CLI |
| Public raw CoT | `crates/apeireth-runtime/src/host.rs` `ChatTurnOutput.reasoning_cot`; `crates/apeireth-gateway/src/server.rs` `reasoning_content` | REAL | DROP | DROP | P0 | - | host/gateway tests reference it | canonical violation |
| Hardcoded key path | `crates/apeireth-gateway/src/main.rs`, `crates/apeireth-cli/src/lib.rs` | REAL | DROP | DROP | P0 | - | - | `C:\Users\31683\apikey-ultra.txt` fixed path |

---

## 8. Storage & Memory

### Findings

- `SqliteConnectionPool` (`pool.rs`) is the strongest storage primitive. It uses
  `r2d2_sqlite`, WAL, `synchronous=NORMAL`, `busy_timeout=5000`, a reader pool, and a
  dedicated writer thread fed by an mpsc channel. The write path returns a `oneshot`
  result. This is architecture-neutral and well-tested by `test_concurrent_read_write`.
- `run_migrations` is simple but real. Tables are generic JSON stores; there is no
  schema migration framework beyond idempotent `CREATE TABLE IF NOT EXISTS`.
- `MemoryStore` (`memory_v2.rs`) is real and stores serialized `MemoryItem` JSON in
  one `facts` table. It supports Add/Update/Delete with tombstones, `CurrentOnly` /
  `Historical` / `All` query modes, ACT-R activation calculation, CJK bigram
  tokenization, Jaccard similarity, and greedy clustering. Sorting is by ACT-R +
  importance.
- `VectorIndex` (`vector.rs`) is a real in-memory hybrid retriever with cosine +
  BM25 and user-profile keyword extraction. It is not persisted and uses whitespace
  tokenization (weak for CJK despite MemoryStore CJK bigrams).
- Graph modules (`graph.rs`, `graph_primitive.rs`, `graph_ops.rs`) implement a
  simple causal graph with BFS crawl and an MCTS-named simulator. This is useful as
  a primitive but is not a full causal/knowledge graph.
- Many `memory_*` modules (`memory_onnx`, `memory_hallways`, `memory_gen_cache`,
  `memory_continuity_link`, `memory_agent_trace`, etc.) are honest but small
  simplified modules. `memory_onnx` is explicitly a stub (no onnxruntime), and
  several are in-memory HashMaps with no SQLite persistence.

### Classification

```text
DIRECT_PORT:
  pool.rs, migrations.rs, memory_v2.rs (as storage implementation),
  vector.rs, graph primitives

ADAPT:
  memory_v2 retrieval call sites (must go through runtime/companion, not gateway)
  graph -> storage or memory capability

DEFER:
  ONNX stub, hallways, continuity store, three-layer memory, etc.
```

### Target

Current canonical architecture has **no root `apeireth-storage` crate**.
`ARCHITECTURE.md` explicitly says durable storage is *not yet created* and
`InMemorySessionStore` occupies the seam. Therefore the target is:

```text
NEW CANONICAL MODULE REQUIRED: apeireth-storage
```

It must depend only on `apeireth-core` (and generic libs), and must not own Runtime,
Gateway, or Companion. The master implementation is strong enough to become the
first major port.

---

## 9. Tools & Sandbox

### Tool implementations

Real and useful:

- `filesystem` (`builtin/filesystem.rs`): read/write/list/delete with a root and a
  `..` path traversal check. Tests cover round-trip and traversal rejection.
- `search` (`builtin/search.rs`): recursive filename + text search, skips hidden /
  target / node_modules, line snippets.
- `repo` (`builtin/repo_tools.rs`): safe git status/log/diff/branch/summary.
- `fetch` (`builtin/fetch.rs`): HTTP GET/POST with DNS-resolved private/loopback
  IP blocking. Missing redirect re-validation and allowlist.
- `browser` (`builtin/browser.rs`): fetches URL and strips HTML. No SSRF check,
  no egress allowlist; falls back to local proxy.
- `shell` (`builtin/shell.rs`): real subprocess execution but the sanitizer is a
  blacklist of a few destructive strings. The `PlatformSandbox` is not attached to
  the spawned process.
- `invest` and `learning`: deterministic helpers, with `invest` returning a
  hardcoded quote when the Yahoo endpoint is unavailable.
- `system_monitor`: Windows-only real-ish metrics, non-Windows fallback.

Vision/desktop:

- `ScreenCapture::capture_native_screen` is real Windows GDI capture.
- `OmniParser::detect_live_elements` is real Windows window/child enumeration.
- `DesktopActionTool` uses `SendInput` with coordinate bounds and a 50 ms rate
  limit. It has **no governance integration** and no action-specific allowlist.
  The gateway endpoint `/v1/vision/act` executes it directly.

### Sandbox

- `PlatformSandbox` creates a Windows JobObject with 256 MB memory limits and a
  restricted token; Linux uses `prctl(PR_SET_NO_NEW_PRIVS)` + `setrlimit`;
  macOS uses `setrlimit`.
- **Critical gap**: the sandbox object is created and `apply_restrictions()` is
  called on the host process, but `ShellTool`, `ToolSynthesizer`, and
  `DesktopActionTool` never assign child processes to the JobObject. The sandbox
  is not actually enforcing the spawned tool processes.
- `ToolSynthesizer` writes a temp script and runs it with `powershell/python/cmd`
  without assigning the JobObject or using a restricted token; `sandbox` field is
  `#[allow(dead_code)]`.
- Tests cover lifecycle only, not real restriction enforcement.
- `WorktreeSandbox::run_live_worktree_pipeline` is a real git worktree add/write/
  test/diff/remove pipeline. It is powerful but application-layer.

### Classification

```text
DIRECT_PORT:
  filesystem, search, repo (after wrapping in canonical ToolCapability)

ADAPT:
  shell (attach real sandbox, stronger command policy)
  fetch (redirect re-validation + egress allowlist)
  browser (SSRF + egress allowlist)
  PlatformSandbox (complete process assignment, fail-closed non-Windows)
  WorktreeSandbox (behind a governed factory capability)

DEFER:
  vision/desktop stack, invest, learning, system_monitor
```

Canonical rule: master `ToolRegistry` is **not** imported. Tools become
`ToolCapability` implementations in plugins.

---

## 10. Runtime Services

| Service | Master implementation | Finding |
| --- | --- | --- |
| Session | `SessionManager` (`runtime/src/session_manager.rs`) | In-memory `HashMap<String, SessionState>` under `tokio::sync::Mutex`; `get_or_create`, `get`, `with_mut`, `snapshot`. No durability, no revision counter, no structured events. Canonical `Session` already supersedes this. |
| Event bus | `EventBus` (`core/src/bus.rs`) + `EventBusBackbone` (`runtime/src/event_bus_backbone.rs`) | Generic broadcast bus + multi-channel wrapper. Core bus is useful; Backbone duplicates channel concept canonical Core can already express. Do not duplicate trace. |
| Scheduler | `Scheduler` (`runtime/src/scheduler.rs`) | Spawns periodic tasks with interval and stop. Minimal but real. Must be a runtime service, never a second runtime. |
| Telemetry | `Telemetry` (`runtime/src/telemetry.rs`) | Atomic counters for chat turns, tool executions, total latency, token usage. No OTel export. Useful as an observability adapter, separate from canonical trace semantics. |
| Lifecycle | `LifecycleHandle` (`runtime/src/lifecycle.rs`) | Facade holding governance/audit/lifecycle/telemetry/scheduler. God-object adjacent; do not port as architecture. |
| Presence | `PresenceHub` (`runtime/src/presence_hub.rs`) | Snapshot of PAD, response style, drive warmth. Silence pressure is hardcoded 0.0. |
| Hybrid router | `hybrid.rs` | Rule-based local fast path with hardcoded templates. Not a real local SLM. |

Classification: `EventBus` core = DIRECT_PORT; scheduler/telemetry = ADAPT;
SessionManager = ABSORBED by canonical session; LifecycleHandle/HybridRouter = DROP.

---

## 11. Companion Functionality

| Feature | File | Maturity | Notes |
| --- | --- | --- | --- |
| Emotion (Plutchik -> PAD -> ResponseStyle) | `companion/src/emotion.rs` | REAL | deterministic, tested; decay and mapping |
| Borbely drive/rhythm | `companion/src/emergence.rs` | REAL | tested; simple two-factor drive |
| Dream engine | `companion/src/dream.rs` | PARTIAL | real triple extraction and Brier calibration; W2/W3 simulators are explicit stubs; `sleep_pressure_after` is hardcoded 0.15 |
| Curiosity | `companion/src/curiosity.rs` | PARTIAL | score formula and signal generation only; no memory interaction |
| Epistemic healer | `companion/src/epistemic.rs` | PARTIAL | keyword-based root cause rules; real tests |
| Observer capture | `companion/src/observer_capture.rs` | REAL | bounded queue with drain/recent |
| Prompt assembler | `companion/src/prompt_assembler.rs` | REAL | multi-layer prompt with identity, tools, philosophy, PAD; contains "thinking chain" directive; must be adapted to canonical no-raw-CoT rule |
| World model | `world_model_v1.rs`, `causal_world_model.rs` | PARTIAL/STUB | simple entity/relation BFS; W2/W3 explicitly stubbed |
| Intent Brier | `intent_brier.rs` | REAL | sliding-window Brier scores |
| Many remaining modules | ~80 files | STUB/PARTIAL | small simplified modules with honest "0 装 PASS" notes |

Companion is valuable as **implementation donor** for emotion, Borbely, Brier, and
observer capture. Dream and curiosity need adaptation and better memory input.

---

## 12. Provider / Protocol Donor Code

Reusable low-level code:

| Component | File | Reuse |
| --- | --- | --- |
| OpenAI Chat serialization/parsing | `protocol/src/adapters/openai.rs` | DTO, tool-call parser, usage mapping |
| Anthropic Messages serialization/parsing | `protocol/src/adapters/anthropic.rs` | DTO, content-block/tool-use parser |
| Gemini generateContent serialization/parsing | `protocol/src/adapters/gemini.rs` | DTO, candidate/usage parser |
| MiniMax Chat serialization/parsing | `protocol/src/adapters/minimax.rs` | DTO, tool-call text fallback |
| Normalized DTO + text tool-call fallback | `protocol/src/normalized.rs` | useful parser logic |
| WsFrame codec | `protocol/src/ws.rs` | frame codec only; remove `CoTDelta` in canonical surface |
| EnergyVad / duplex | `protocol/src/voice.rs` | VAD and barge-in logic |

Do **not** port:

- `ProtocolAdapter::execute(api_key, request)` trait shape.
- `ModelRouter` prefix/exact/fallback architecture.
- Any hardcoded default model/provider in runtime.
- `GeminiAdapter` as first-class migration now: record as
  `future ProviderCapability candidate`.

---

## 13. Gateway / Streaming / MCP

### Gateway endpoint groups

| Group | Endpoints | Real? |
| --- | --- | --- |
| Health/models/chat | `/health`, `/v1/models`, `/v1/chat/completions` | PARTIAL: health/models real (models hardcoded), chat real when host present but exposes `reasoning_content` |
| Panel/sessions/memory/graph/audit/tools | `/v1/panel/*` | PARTIAL: real direct host access; graph derives naive fact/link JSON from memory text |
| Approval | `/v1/apeireth/approval-requests`, `/grant`, `/grants` | MOCK: empty JSON |
| Memory | `/v1/memory/append`, `/v1/apeireth/memory`, `/v1/memory/list`, `/v1/memory/search` | PARTIAL: real memory_store query, no canonical boundary |
| Agents/skills | `/v1/agents`, `/v1/agent/turn`, `/v1/skills`, `/v1/skill/invoke` | MIXED: `agent_turn` calls `host.model_router` directly; skills hardcoded calculator |
| Presence/events | `/v1/apeireth/presence/ws`, `/v1/apeireth/events` | REAL for presence; SSE repeats presence every 3s |
| Vision | `/v1/vision/observe`, `/v1/vision/act` | PARTIAL: observe real Windows; act executes `host.tool_registry` directly |
| Factory | `/v1/factory/tasks`, `/v1/factory/merge` | PARTIAL: create task runs real worktree pipeline; list tasks is hardcoded |
| MCP hub | `/v1/mcp/registry`, `/v1/mcp/install`, `/v1/mcp/uninstall` | PARTIAL: install spawns stdio and injects into master `ToolRegistry` |
| Admin/training/evolution | `/v1/admin/policy`, `/v1/training/feedback`, `/v1/evolution/proposals`, `/v1/blueprint/run` | MOCK/STUB: empty policies, echo feedback, empty proposals, stub blueprint |

Canonical rule: any endpoint doing `Gateway -> MemoryStore/ToolRegistry/SessionManager`
directly requires `ADAPT` through Runtime/service boundary. The chat endpoint should
call canonical `Runtime::execute`, not `host.handle_chat_turn`.

### Streaming

- `SseBroadcaster` (`gateway/src/sse.rs`) is a clean generic SSE broadcaster.
- `companion_events_sse` emits presence snapshots every 3s; it is transport-only.
- `WsFrame` includes `CoTDelta` — raw CoT in public contract; canonical must not use.
- Master has no full chat-token SSE stream: `/v1/chat/completions` ignores
  `stream` and always returns a complete JSON object.

### MCP

- `McpClient`, `McpServer`, protocol types, and transports are real and tested.
- `McpServer` is bound to master `Arc<ToolRegistry>`; canonical target is a
  transport/capability boundary: MCP tools become `ToolCapability` instances from a
  plugin declaring `transport.mcp`. The install endpoint injecting directly into
  `host.tool_registry` is a duplicated registry and must not be ported as-is.

---

## 14. SDK / Voice / Avatar / Bridge

- **SDK**: real HTTP client for chat/session/memory/tools, but it is application
  layer and should track canonical gateway contracts. `MemoryClient::search`
  deserializes the wrong shape for the panel endpoint; this must be fixed when the
  gateway contract is finalized.
- **Voice**: `EnergyVad` and `VoiceDuplexEngine` are deterministic and useful.
  `apeireth-voice` VAD is a state-machine over RMS. TTS is a synthetic sine-wave
  placeholder with SSML construction; it is not a real provider integration.
- **Avatar**: expression mapping (PAD -> Live2D/VRM) is real parameter math;
  controllers are config/state only, no renderer or real Live2D/VRM runtime.
- **Bridge**: social bridges (Discord webhook, Telegram bot API, OneBot) are small
  but real HTTP clients. LiveKit has real JWT HS256 signing. Lark has a real
  tenant-token flow. `web` and `game` are stubs. All are application-layer and
  should be `DEFER`red.
- **PyBridge**: real HTTP client to gateway endpoints, but application-layer.
  `DEFER`.

---

## 15. Security Findings

| # | Finding | Severity | Evidence |
| --- | --- | --- | --- |
| 1 | Raw API key as `pub api_key: String` on `UnifiedRuntimeHost`; passed into `ModelRouter.execute` and gateway `agent_turn` | HIGH | `runtime/src/host.rs:44`, `gateway/src/server.rs:867` |
| 2 | Fixed machine-specific key path `C:\Users\31683\apikey-ultra.txt` | HIGH | `gateway/src/main.rs:5`, `cli/src/lib.rs:59` |
| 3 | Raw CoT exposed as `ChatTurnOutput.reasoning_cot` and gateway `reasoning_content` | HIGH | `runtime/src/host.rs:52`, `gateway/src/server.rs:215` |
| 4 | `ShellTool` executes arbitrary commands with blacklist sanitizer; sandbox not attached | HIGH | `tools/src/builtin/shell.rs` |
| 5 | `PlatformSandbox` created/applied to host process, but child processes not assigned on Windows | HIGH | `tools/src/sandbox.rs`, `host.rs` tool loop |
| 6 | `ToolSynthesizer` runs temp scripts unsandboxed (`sandbox` field dead) | HIGH | `tools/src/synthesis.rs` |
| 7 | `BrowserTool` has no SSRF check and falls back to a local proxy | MEDIUM | `tools/src/builtin/browser.rs` |
| 8 | `FetchTool` SSRF check does not re-validate redirect targets | MEDIUM | `tools/src/builtin/fetch.rs` |
| 9 | `EgressFilter` exists but is not wired into fetch/browser/provider adapters | MEDIUM | `gateway/src/egress.rs` |
| 10 | Gateway endpoints directly execute tools and mutate memory without Runtime/governance boundary | MEDIUM | `gateway/src/server.rs:591`, `:513` |
| 11 | `DesktopActionTool` has no governance action policy beyond coordinate bounds | MEDIUM | `tools/src/vision/desktop_action.rs` |
| 12 | `list_models` and `ModelRouter` default provider/model hardcoded | LOW | `gateway/src/server.rs:131`, `runtime/src/host.rs` |
| 13 | MCP install spawns external `npx` process and injects tools without governance | MEDIUM | `gateway/src/server.rs:737` |

---

## 16. Placeholder / Mock Findings

These are **not** counted as implemented features:

- `/v1/apeireth/approval-requests`, `/grant`, `/grants`: empty JSON.
- `/v1/admin/policy`: empty policy list.
- `/v1/training/feedback`: echo only.
- `/v1/evolution/proposals`: empty list with note.
- `/v1/blueprint/run`: returns `"result": "stub"`.
- `/v1/factory/tasks` list: one hardcoded task.
- `knowledge_graph`: returns empty nodes/edges with a note.
- `/v1/organs`: empty list.
- `/v1/panel/traces`: empty traces.
- `list_skills` / `list_skills_full`: hardcoded calculator skill.
- `UnifiedRuntimeHost::trigger_nightly_dream_evolution`: hardcoded unresolved
  episode and prediction inputs.
- `ToolSynthesizer` sandbox field unused.
- `PlatformSandbox` Linux `platform_type` says "Linux-Seccomp-Rlimit" but no seccomp
  is applied.
- `voice/src/tts.rs`: synthetic sine-wave PCM, not real TTS.
- `avatar`: no actual Live2D/VRM runtime.
- `bridge/src/web.rs`: `start()` returns `Ok(())` with no server.
- `bridge/src/game/vision_loop.rs`: stub decisions.

---

## 17. Lost Capabilities

Features present in master that canonical branch currently lacks (at production
path level):

| Feature | Value | Canonical target | Port difficulty | Dependencies | Priority |
| --- | --- | --- | --- | --- | --- |
| SQLite read/write pool | HIGH | `apeireth-storage` (new) | LOW | rusqlite/r2d2 | P0 |
| ACT-R memory store + temporal/tombstone | HIGH | `apeireth-storage` | LOW | pool, clock | P1 |
| Hybrid vector index (cosine+BM25) | MEDIUM | `apeireth-storage`/retrieval | LOW | none | P1 |
| Graph primitives | MEDIUM | `apeireth-storage`/graph capability | MEDIUM | none | P2 |
| Filesystem/search/repo tools | HIGH | `ToolCapability` plugins | LOW | std/tokio | P1 |
| Fetch/browser tools | MEDIUM | `ToolCapability` plugins | MEDIUM | reqwest + egress | P1/P2 |
| Platform sandbox (Windows JobObject) | HIGH | tool sandbox capability | MEDIUM | winapi | P1 |
| Worktree sandbox / factory | MEDIUM | factory capability | HIGH | git subprocess | P2 |
| MCP protocol/transport | HIGH | MCP transport boundary | MEDIUM | tokio/serde | P1 |
| PII scrub + prompt injection hook | HIGH | governance hooks | LOW | regex | P1 |
| Audit hash chain | HIGH | governance/audit primitive | LOW | sha2 | P1 |
| Onion/ABAC policy | MEDIUM | governance hooks | MEDIUM | std | P2 |
| Event bus (generic) | HIGH | core event primitive | LOW | tokio | P1 |
| Scheduler | MEDIUM | runtime service | LOW | tokio | P2 |
| Telemetry metrics | MEDIUM | observability adapter | LOW | std/tracing | P2 |
| Plutchik/PAD emotion | HIGH | companion | LOW | serde | P1 |
| Borbely rhythm/drive | HIGH | companion | LOW | std | P1 |
| Dream engine | MEDIUM | companion | MEDIUM | memory, world model | P2 |
| Curiosity | LOW | companion | MEDIUM | memory | P3 |
| Epistemic healer | MEDIUM | companion | LOW | std | P2 |
| Prompt assembler | HIGH | companion | LOW | emotion | P1 |
| Presence snapshot | MEDIUM | companion/gateway | LOW | emotion | P2 |
| Voice VAD/duplex | MEDIUM | voice capability | LOW | std | P2 |
| Voice TTS | LOW | voice capability | HIGH | external provider | P3 |
| Avatar expression mapping | LOW | avatar/desktop | LOW | emotion | P3 |
| Gemini adapter DTO/parser | MEDIUM | future `ProviderCapability` | LOW | serde | P2 |
| SDK client | MEDIUM | SDK API surface | LOW | reqwest | P2 |

---

## 18. Dependency DAG

Derived from actual `reconstruction_v2` dependency edges and canonical ownership.

```text
Storage (SQLite pool -> migrations -> MemoryStore -> VectorIndex -> Graph)
├── Memory retrieval (Runtime)          [needs storage + clock + session context]
│   ├── Context / prompt assembly       [needs memory + emotion + tool declarations]
│   ├── Dream                           [needs memory + Brier + world model]
│   └── Curiosity                       [needs memory novelty + Brier]
│
Tool Sandbox (PlatformSandbox -> process assignment)
├── Shell                               [needs sandbox + governance]
├── Filesystem                          [needs root jail]
├── Fetch / Browser                     [needs egress allowlist + SSRF]
├── Repo / Search                       [needs std/git]
└── Factory / Worktree                  [needs git + worktree sandbox]
│
Runtime Services
├── EventBus (core primitive)           [needs core only]
├── Scheduler                           [needs tokio]
└── Telemetry                           [needs tracing]
│
Governance Hooks
├── PII detector                        [needs regex]
├── Audit hash chain                    [needs sha2]
└── Onion / ABAC                        [needs core]
│
Companion
├── Emotion (Plutchik/PAD/Borbely)      [needs core]
├── Prompt assembly                     [needs memory + emotion + tools]
└── Presence                            [needs emotion + Borbely]
│
Gateway Streaming
└── Runtime StreamEvent                 [needs protocol canonical]
│
MCP
├── Protocol codec                      [needs serde]
├── Transport                           [needs tokio]
└── Capability mapping                  [needs plugin ToolCapability]
```

Migration order follows the DAG bottom-up.

---

## 19. Recommended Migration Phases

```text
M1: Storage primitives + governance primitives
  - Port SqliteConnectionPool, migrations, MemoryStore, VectorIndex into a new
    canonical apeireth-storage (or the designated storage owner).
  - Port PiiDetector + AuditHashChain behind GovernanceHook.
  - Validate with existing storage/guard/audit tests.

M2: Tool capabilities + sandbox
  - Wrap filesystem, search, repo as ToolCapability plugin implementations.
  - Port PlatformSandbox and enforce real process assignment for shell.
  - Adapt shell/fetch/browser with egress allowlist and redirect re-validation.

M3: Memory retrieval + companion emotion + prompt assembly
  - Wire storage MemoryStore through Runtime session/context service.
  - Port Plutchik/PAD, Borbely, PromptAssembler (CoT-safe) into companion.
  - Add runtime retrieval of memory into prompt assembly.

M4: Streaming + MCP boundary
  - Adapt SSE broadcaster and WS frame transport to canonical StreamEvent.
  - Port MCP protocol/transport and map MCP tools to ToolCapability declarations.

M5: Scheduler + telemetry + presence
  - Port Scheduler and Telemetry as runtime services.
  - Port PresenceHub over companion emotion.

Deferred after M5:
  - Dream/curiosity/world model (need stable memory + Brier).
  - Worktree factory, desktop/vision, voice, avatar, bridge, SDK.
```

Each phase is small, dependency-safe, and testable.

---

## 20. First Recommended Port

Best first migration candidates (1–3 functionality groups):

1. **Storage primitives** — `pool.rs`, `migrations.rs`, `memory_v2.rs`, `vector.rs`.
   Why: architecture-neutral, well-tested, few dependencies, clear owner
   (`apeireth-storage`, new canonical module required), P0 because many future
   features depend on durability.
2. **Governance primitives** — `PiiDetector` and `AuditHashChain`.
   Why: deterministic, tested, map cleanly onto canonical `GovernanceHook`
   semantics, no runtime/gateway coupling.
3. **Simple tool implementations** — `filesystem`, `search`, `repo`.
   Why: real, tested or easily testable, low dependency pollution, wrap behind
   canonical `ToolCapability` without importing master `ToolRegistry`.

Do **not** start with `UnifiedRuntimeHost`, Companion-everything, MCP-everything,
Voice stack, Desktop stack, or whole Gateway.

---

## 21. Deferred Functionality

```text
Dream engine and world model
Curiosity engine
Voice TTS and full voice stack
Avatar Live2D/VRM runtime
Bridge (social/lark/livekit/stock)
Desktop/vision full stack
Worktree factory and ToolSynthesizer
Invest and Learning tools
PyBridge
SDK polish (after canonical gateway contract stabilizes)
```

---

## 22. Dropped Architecture

Confirmed architecture pieces that should **never** be ported:

```text
UnifiedRuntimeHost                 (God Object; decompose, never port)
ModelRouter                        (superseded by ProviderRouter)
ProtocolAdapter::execute(api_key)  (superseded by ProviderCapability + CredentialResolver)
master ToolRegistry                (duplicate capability ownership)
master runtime CapabilityRegistry  (duplicate registry)
SessionManager as second session owner (canonical Runtime Session owns this)
LifecycleHandle facade             (God-object adjacent)
HybridCognitiveRouter fast path    (hardcoded templates, second loop)
raw api_key: String in Runtime     (credential boundary violation)
raw reasoning_cot / reasoning_content public fields
Gateway direct host subsystem access (direct ToolRegistry/MemoryStore/SessionManager)
McpRegistry / MCP install into ToolRegistry
```

---

## 23. Open Questions

1. Should the new canonical `apeireth-storage` be a root crate or a module inside
   the existing `reconstruction_v2` storage crate? The architecture says a root
   storage crate is pending; exact crate path is not yet fixed.
2. Does the canonical runtime need a memory retrieval service in `Runtime::execute`
   or should companion prompt assembly be a plugin capability that receives the
   session transcript? The master implementation fuses these; canonical split is
   not yet specified at the code level.
3. Are `PiiDetector` and `AuditHashChain` governance hooks or separate
   observability/security crates? Canonical `ARCHITECTURE.md` says they "live in
   their own crates and become GovernanceHook implementations"; the crate names
   are not fixed.
4. Should the first storage port preserve master's JSON-in-one-table schema or
   introduce a normalized schema? This affects migration SQL and future graph
   traversal.
5. MCP install/uninstall lifecycle: canonical plugins are static/in-process; how
   should a spawned MCP server be represented as a plugin manifest?

---

## 24. Final Verdict

Master `reconstruction_v2` is primarily useful as:

```text
B) implementation donor
```

Explanation: the master architecture is a God-Object runtime with raw credentials,
raw CoT in public contracts, duplicate registries, and Gateway direct subsystem
access. It cannot serve as the architecture donor. However, a substantial subset of
its implementation is real, tested, and deterministic: SQLite storage, memory
retrieval, several tools, sandbox primitives, governance primitives, emotion, VAD,
MCP protocol, and protocol DTO parsers. The correct process is to port those pieces
behind the frozen canonical skeleton, one phase at a time, exactly as described
above.
