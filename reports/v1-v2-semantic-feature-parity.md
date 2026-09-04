# Apeireth v1 → v2 Semantic Feature Parity Audit

MODE: AUDIT ONLY. No routes added. No frontend modified. No legacy APIs resurrected.

Feature parity ≠ endpoint parity. Unit of comparison: **user / agent capability**.

---

## 0. Source guard

```
branch:           main
working tree:     dirty (untracked packaging bundles only; no audited source mutation)
PARITY_AUDIT_V2_HEAD=bc02b23d36731dd04ac15b86df6883a716423552
origin/main:      464ef9aae735fce0344ec9754a2a32791d967a2f
                  (forced update 86495bb9…464ef9aa during this audit)
left-right:       origin/main...HEAD  behind=2405  ahead=2435
                  (history rewrite on origin; local main is the v2 product under audit)
PARITY_AUDIT_V1_HEAD=76c87048deb9b695d34376d6cba85ebc5a6408fb
v1 ref:           origin/archive/v1.0-master  (read-only worktree donor-wt)
v1.0.0 tag:       993e9107e4122f38272df16b883d31e7cf1cbce2 (ancestor of archive)
```

Trees were not mutated. v1 inventory was taken from `crates/_archived/v1.0-legacy/` + `frontend/companion-desktop` + `apeireth-agent-standalone` on the archive ref. `reconstruction_v2/` on that tree is treated as **DEAD_LEGACY mock gateway**, not as v1 product.

---

## 1. Method

1. Inventory **v1 first** (companion_serve + desktop + stores + tests).
2. Classify each v1 item REAL / PARTIAL / STUB / DEAD / UNREACHABLE / UI-ONLY / TEST-ONLY.
3. Map only **REAL or PARTIAL shipped product** into v2 owners.
4. Split **implementation exists** vs **product can access**.
5. Never recommend restoring `/v1/panel/*`.

Two v1 servers must not be conflated:

| Process | What desktop actually talked to | Memory / tools |
|---|---|---|
| `companion_serve` (product) | `:8090` chat + panel GET + grant + SSE | `SqliteMemoryStore` under `%APPDATA%/apeireth` |
| `apeireth-api` | Separate binary | Different `V2Memory`; often uninstalled |

Desktop MemoryView/ToolsView called **both URL families** against one `baseUrl`. Against companion, many write routes 404.

---

## 2. Hypothesis

> “Most v2 capability loss is exposure loss rather than implementation loss.”

**Result: YES, with a MIXED tail.**

- Implementation still present for session transcript, cognitive memory, tools, governance, providers, organ library, graph/experience stores.
- Desktop/Gateway expose chat + health + models + (CLI) approval resolve.
- True **implementation holes** vs meaningful v1 product: companion aliveness/presence daemon, sleep-time dream loop, streaming tool execution (already broken in v1 desktop), MCP host in the chat loop (never mounted on companion).
- Several “losses” are **honesty**: v1 advertised HTTP that companion_serve never served.

---

## 3. V2 ownership (canonical)

| Concern | Canonical v2 owner | Competing / default-off |
|---|---|---|
| Agent loop | Runtime `execute.rs` | none |
| Session transcript | Runtime `SessionStore` → `.apeireth/sessions.sqlite3` | Memory `session_lifecycle` **unwired** |
| Cognitive memory | `MemoryBackend` + CLI `ProductionCognitiveModules` → `.apeireth/cognitive.sqlite3` | `SqliteMemoryStore` forget/protect; `SqliteMemoryRepository` |
| Tools | `apeireth-tools-canonical` + Module wrappers | `BuiltinToolsPlugin` (tests); empty `McpModule` |
| Governance | `GovernanceHook` pipeline | Colang / ApprovalPolicyEngine libraries |
| Approval resume | Runtime `PendingApproval` + `/v1/approvals/resolve` | v1 PermissionPack / master_token |
| Providers | Provider plugins + `ProviderRouter` | none |
| Credentials | `CredentialResolver` (env/keyring) | frontend must not store provider keys |
| Organs | organ crate + opt-in `OrganModule` (CLI default **false**) | organ `goal` unwired |
| Cron / heartbeat | orchestration `cron.rs` parse-only; runtime `heartbeat.rs` heap | **no production ticker** |
| Telemetry | `ExecutionTrace` on the turn | unused `agent_traces` table; no SSE bus |
| HTTP | Gateway `canonical_router` **five routes** | duplex/barge-in/file_fetcher **unwired** |
| Desktop cache | localStorage conversations/config | not the Runtime session |

Ambiguous ownership (flagged): three session types; three memory stores; two tool spines (`ModuleRegistry` ∪ `CapabilityRegistry`); four graph types (production uses experience store only).

---

## 4. Canonical gateway routes (current)

| Route | Purpose | R/W | Auth | Consumers |
|---|---|---|---|---|
| `GET /health` | Liveness | READ | none | Desktop health |
| `GET /v1/models` | Configured model + **hardcoded `whisper-1`** | READ | none | Settings / health |
| `POST /v1/chat` | Native turn; 202 pending approval | WRITE | none transport; runtime governance | Tests / unused by desktop |
| `POST /v1/chat/completions` | OpenAI-shaped; **buffered fake SSE**; cannot resume approval | WRITE | same | **Desktop chat / voice / quick** |
| `POST /v1/approvals/resolve` | Approve/reject/cancel frozen turn | WRITE | same | **CLI only** |

Not on the router: `/v1/apeireth/*`, `/v1/panel/*`, `/v1/tools/list`, `/v1/memory/*`.

CLI: `session` / `chat` / `approve|reject|cancel` / `gateway serve`.

---

## 5. Desktop pages vs v1

| Page | v1 functionality | v2 now | Classification |
|---|---|---|---|
| Chat | Stream to companion; local transcript; fake tool-success on stream | Stream to canonical OpenAI path; local transcript; `session_id` UUID; no `tool_calls` on wire | PARTIALLY_EXPOSED |
| Conversations | Local CRUD **REAL**; backend ledger often empty | Local CRUD **RECOVERED**; backend tab gated empty | Local RECOVERED; ledger EXISTS_BUT_NOT_EXPOSED |
| Memory | Panel GET real; writes/forget 404 on companion | Always “unsupported” | EXISTS_BUT_NOT_EXPOSED (store yes) |
| Tools | List real; invoke dead on companion; grant live | Always “unsupported” | EXISTS_BUT_NOT_EXPOSED |
| Activity | Audit+SSE live; traces never mounted | Gated empty + **ungated EventSource 404** | EXISTS_BUT_NOT_EXPOSED |
| Settings | Endpoint/model real; persona/memory/onion **copy-only** | Same split; diagnostics health+models; supervisor UI blocked | PARTIALLY_EXPOSED |
| Runtime modal | Health + over-claimed capabilities | Health + conservative 4-cap contract | PARTIALLY_EXPOSED |
| Quick window | Non-persisted chat | Same; `sessionId=quick-…` **fails UUID deserialize** | PARTIALLY_EXPOSED / broken |
| Voice | Browser STT/TTS, not LiveKit | Same | PARTIALLY_EXPOSED |
| Approvals | Grant modal + pending poll | Desktop does **not** call `/v1/approvals/resolve`; App still polls v1 grant list (404) | PARTIALLY_EXPOSED (CLI recovered) |

A page that says “unsupported capability” is **not** parity. It only avoids lying 200s.

---

## 6. Master matrix

Columns: ID · DOMAIN · CAPABILITY · V1_REALITY · V1_VALUE · V2_OWNER · V2_IMPLEMENTATION · V2_CANONICAL_EXPOSURE · V2_DESKTOP_EXPOSURE · CLASSIFICATION · SECURITY_CLASS · RECOMMENDATION · EVIDENCE

### Session / history

| ID | CAPABILITY | V1_REALITY | V1_VALUE | V2_OWNER | V2_IMPL | CANON | DESKTOP | CLASS | SECURITY | REC | EVIDENCE |
|---|---|---|---|---|---|---|---|---|---|---|---|
| S01 | Local conversation create | REAL | CRITICAL | Desktop local state | YES | NONE | FULL | RECOVERED | SAFE | Keep local UX; do not pretend it is Runtime session | `App.svelte` `newConversation`; v2 same |
| S02 | Local persist (localStorage) | REAL | CRITICAL | Desktop local state | YES | NONE | FULL | RECOVERED | SAFE | Same | `runtime.ts` `load/saveConversations` |
| S03 | Local list/search/pin | REAL | HIGH | Desktop | YES | NONE | FULL | RECOVERED | SAFE | Same | `ConversationsView.svelte` |
| S04 | Local rename | REAL | HIGH | Desktop | YES | NONE | FULL | RECOVERED | SAFE | Same | `onRename` → local title |
| S05 | Local archive/restore | REAL | MEDIUM | Desktop | YES | NONE | FULL | RECOVERED | SAFE | Same | local `archived` flag |
| S06 | Local delete | REAL | HIGH | Desktop | YES | NONE | FULL | RECOVERED | SAFE | Same | ConfirmDialog |
| S07 | Continue thread (client history) | REAL | CRITICAL | Desktop + Runtime Session | YES | PARTIAL (`session` on chat) | PARTIAL | PARTIALLY_EXPOSED | SAFE | Desktop still owns transcript; Runtime also persists turns under UUID | v1 `App.send` history map; v2 `streamChat` + `SessionStore` |
| S08 | Backend session list | REAL HTTP, often empty (chat never `upsert_session`) | MEDIUM | Runtime SessionStore | YES | NONE | NONE (gated) | EXISTS_BUT_NOT_EXPOSED | SAFE_READ | Later SessionReadModel; **not** `/v1/panel/sessions` | v1 `panel_sessions`; v2 `SqliteSessionStore` |
| S09 | Backend timeline | REAL on web panel; UNREACHABLE desktop | MEDIUM | Runtime SessionEvent | YES | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | SAFE_READ | SessionReadModel | v1 `panel_session_timeline`; v2 `SessionEventKind` |
| S10 | Backend lifecycle HTTP create/rename/archive/close | TEST-ONLY store; **no route on companion_serve** | — | Memory `session_lifecycle` unwired | PARTIAL | NONE | DEAD fetchers | DEAD_LEGACY (HTTP) / EXISTS_BUT_NOT_EXPOSED (lib) | GOVERNED_WRITE | Do not restore v1 routes; optional later commands | v1 G10; `companion_serve` router 1718–1734 |
| S11 | Durable SessionLog / OneRing / continuation snapshots | TEST-ONLY | LOW | none in loop | NO | NONE | NONE | DEAD_LEGACY | — | Do not restore | `session_log.rs`, `onering.rs` unused by serve |
| S12 | Cross-turn identity (UUID vs `"me"` vs continuity) | PARTIAL (3 ids) | HIGH | Runtime SessionId | YES (one id) | PARTIAL | PARTIAL | PARTIALLY_EXPOSED | SAFE | v2 is cleaner (one SessionId); desktop local id ≈ Runtime id when UUID | v1 MEMORY_SESSION=`me`; v2 `SessionId` |

### Memory / graph / retrieval

| ID | CAPABILITY | V1_REALITY | V1_VALUE | V2_OWNER | V2_IMPL | CANON | DESKTOP | CLASS | SECURITY | REC | EVIDENCE |
|---|---|---|---|---|---|---|---|---|---|---|---|
| M01 | Retrieval into chat | REAL (recency+keyword+A-MEM crawl) | CRITICAL | MemoryRecallModule | YES | NONE (side-effect of chat) | NONE inspect | RECOVERED (agent) / EXISTS_BUT_NOT_EXPOSED (inspect) | SAFE | Keep loop injection; add MemoryInspection later | v1 `assemble.rs`; v2 `MemoryRecallModule` |
| M02 | Episode append (product store) | PARTIAL: extractor/`save_memory`/dream write SQLite; UI `POST /v1/memory/append` **wrong server** | HIGH | MemoryWritebackModule | YES | NONE | NONE (gated write fails) | RECOVERED (agent writeback) / EXISTS_BUT_NOT_EXPOSED (human append) | GOVERNED_WRITE | Human append needs governed command, not raw HTTP | v1 companion vs api split; v2 `put_episode` AfterTurn |
| M03 | Episode browse | REAL ungoverned GET panel | HIGH | MemoryBackend `recent_episodes` | YES | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | SAFE_READ | MemoryInspection read model | v1 `panel_memory_episodes`; MemoryView |
| M04 | Six history streams | REAL backend; DEAD in MemoryView | MEDIUM | Memory `append_stream` | PARTIAL (API exists, writeback unused) | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | SAFE_READ | TIER 2 | v1 `history_streams.rs` |
| M05 | Knowledge graph inspect | PARTIAL (session `"me"` vs `companion-main` bug) | HIGH | Experience `KnowledgeGraphStore` | YES (thin) | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | SAFE_READ | GraphInspection; do not port panel session bug | v1 `panel_graph`; v2 `facts_from`/`put_fact` |
| M06 | A-MEM CRAWL | REAL in companion inject | HIGH | `amem_graph` | YES default-off | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | SAFE | Optional TIER 1 if recall quality regresses | v1 `memory_graph.rs`; v2 “not production-wired” |
| M07 | Forget / protect / unprotect | LIBRARY-REAL; HTTP **absent** on companion | HIGH intended / LOW shipped | `memory_governance` on **wrong store** | PARTIAL | NONE | gated dead | EXISTS_BUT_NOT_EXPOSED | GOVERNED_WRITE | Needs MemoryBackend + governance; no panel routes | v1 `memory_governance.rs` |
| M08 | Dream / sleep consolidation | REAL daemon | MEDIUM | none in Runtime | NO | NONE | copy-only Settings | INTENTIONALLY_REMOVED | — | TIER 2 as module hook, **not** a second daemon | v1 `dream.rs` + daemon_loop |
| M09 | Vector / sqlite-vec | LIBRARY; chat never called it | HIGH if wired | `canonical::vector` / `persistent_vector` | PARTIAL | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | SAFE | TIER 2 | v1 `apeireth-vector`; v2 default-off |
| M10 | Preference extract + inject | REAL silent on companion | HIGH | PreferenceRecall/Writeback | YES | NONE | NONE | PARTIALLY_EXPOSED | SAFE | Recovered internally; no prefs UI (v1 had none) | v1 `memory_extractor.rs`; v2 `preference_recall` |
| M11 | Identity HTTP | WIRED-SPLIT (api only) | MEDIUM | kernel Session DTO | PARTIAL | NONE | NONE | DEAD_LEGACY (HTTP) | — | Do not restore apeireth-api identity routes | v1 `v2_endpoints` identity |

### Tools / MCP / files

| ID | CAPABILITY | V1_REALITY | V1_VALUE | V2_OWNER | V2_IMPL | CANON | DESKTOP | CLASS | SECURITY | REC | EVIDENCE |
|---|---|---|---|---|---|---|---|---|---|---|---|
| T01 | Tool execution in agent loop | REAL **non-stream only**; desktop stream **skipped + fake success** | CRITICAL | Runtime dispatch + ProcessExecutor | YES | NONE as catalog; YES as chat side-effect | NONE (no tool_calls on wire) | PARTIALLY_EXPOSED | GOVERNED_WRITE | Expose tool events on canonical chat; **never** restore fake `'执行成功'` | v1 stream skip comment; v2 `execute.rs` + OpenAI encoder has no tools |
| T02 | Tool catalog / list | REAL GET `/v1/tools/list` | HIGH | ModuleRegistry ∪ CapabilityRegistry | YES | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | SAFE_READ | ToolCatalog read model | v1 `tools_list`; v2 `Runtime::tools()` |
| T03 | Filesystem | PARTIAL (schema/approval holes) | HIGH | `tool.filesystem` read-only | YES | NONE | NONE | PARTIALLY_EXPOSED | GOVERNED_WRITE | Keep read-only; write = new governed design | v2 filesystem.rs “write/delete not implemented” |
| T04 | Search / repo / grep/git analogue | REAL daily pack | HIGH | `tool.search` / `tool.repo` | YES | NONE | NONE | PARTIALLY_EXPOSED | GOVERNED_WRITE | Repo granted by default; search opt-in env | CLI `ProductionModulesConfig` |
| T05 | Fetch / web | REAL WebFetch | HIGH | `tool.fetch` | YES | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | GOVERNED_WRITE | CLI fetch **off** | v2 `fetch: None` default |
| T06 | Shell | PARTIAL gated | HIGH | `tool.shell` | YES | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | GOVERNED_WRITE | CLI shell **off**; e2e exists | `canonical_shell_approval_e2e.rs` |
| T07 | ApplyPatch | LIVE engine, no schema, risk bypass | HIGH | `TransactionalPatchApplier` | YES lib | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | GOVERNED_WRITE | TIER 1 as capability, not HTTP patch | v1 `apply_patch.rs`; v2 not a ToolCapability |
| T08 | MCP host in chat | DEAD on companion (zero imports) | MEDIUM | plugin mcp + empty McpModule | PARTIAL | NONE | NONE | DEAD_LEGACY (product) / EXISTS_BUT_NOT_EXPOSED (lib) | GOVERNED_WRITE | TIER 2; do not port reconstruction `mcp_install` | v1 companion grep empty; v2 `mcp: false` |
| T09 | Duplicate N17 tools / yaml / skills / extensions | Mostly unschematized | LOW | none | NO | — | — | DEAD_LEGACY | DANGEROUS if dumped | Do not port piles of generic tools | Tool-20..28 inventory |
| T10 | Fake stream tool success | LIVE **lie** | — | — | — | — | — | DEAD_LEGACY | DANGEROUS_LEGACY_BYPASS | Must not return | v1 `runtime.ts` 558–566 |

### Governance

| ID | CAPABILITY | V1_REALITY | V1_VALUE | V2_OWNER | V2_IMPL | CANON | DESKTOP | CLASS | SECURITY | REC | EVIDENCE |
|---|---|---|---|---|---|---|---|---|---|---|---|
| G01 | Pause turn for human approval | REAL deny-and-queue | CRITICAL | Runtime PendingApproval | YES | FULL (`/v1/approvals/resolve`) | NONE | PARTIALLY_EXPOSED | GOVERNED_WRITE | Wire desktop to resolve; drop grant modal | v1 `execute_if_allowed`; v2 `canonical_entry` 202 + CLI approve |
| G02 | Timed PermissionPack + master_token HTTP | REAL | HIGH | **removed** | NO | NONE | dead grant UI | INTENTIONALLY_REMOVED | DANGEROUS_LEGACY_BYPASS | Do not restore | v1 `POST /v1/apeireth/grant`; packs RAM-only |
| G03 | List pending approvals | REAL | HIGH | Session pending field | YES | NONE list | NONE | EXISTS_BUT_NOT_EXPOSED | SAFE_READ | ApprovalInbox read model | v1 `GET /v1/apeireth/approval-requests` |
| G04 | Revoke / list grants | Library; HTTP missing on serve | MEDIUM | in-memory PermissionPolicy at boot | PARTIAL | NONE | NONE | INTENTIONALLY_REMOVED (pack model) | GOVERNED_WRITE | Grants are boot policy, not HTTP packs | v1 no `/grants` on serve |
| G05 | Onion L0–L5 as tool ACL | Wired but inert | LOW | unused | NO | — | copy-only Settings | DEAD_LEGACY | — | Do not restore onion inspector | v1 `gate.rs` `\|\| true` |
| G06 | Constitution / prompt injection / judge | REAL string+LLM judge | HIGH | PromptInjectionHook; JudgeModule opt-in | YES | NONE | NONE | PARTIALLY_EXPOSED | GOVERNED_WRITE | Keep hooks; no MiniMaxConstitution UI | CLI pipeline; `APEIRETH_COGNITIVE_JUDGE` |
| G07 | Startup `APEIRETH_GRANT` | REAL bypass | — | — | NO | — | — | DEAD_LEGACY | DANGEROUS_LEGACY_BYPASS | Do not restore | companion_serve 1553–1568 |

### Audit / events / diagnostics

| ID | CAPABILITY | V1_REALITY | V1_VALUE | V2_OWNER | V2_IMPL | CANON | DESKTOP | CLASS | SECURITY | REC | EVIDENCE |
|---|---|---|---|---|---|---|---|---|---|---|---|
| O01 | Tool-call audit log | REAL action_stream | HIGH | ExecutionTrace + SessionEvent | YES | PARTIAL (on chat response) | NONE list | EXISTS_BUT_NOT_EXPOSED | SAFE_READ | AuditQuery from session events; not `/v1/panel/audit` | v1 `RecordStore`; v2 `TraceEvent` |
| O02 | SSE `/v1/apeireth/events` + presence | REAL | HIGH | none | NO | NONE | EventSource still fires | INTENTIONALLY_REMOVED (bus) / MISSING (aliveness) | — | TIER 1 optional event adapter; **no** companion daemon | v1 `BroadcastSink`; v2 no SSE route |
| O03 | Agent trace list/detail | STUB (recorder never constructed) | — | unused `agent_traces` | PARTIAL lib | NONE | gated | DEAD_LEGACY | — | Do not restore panel traces | v1 zero `TraceRecorder` in serve |
| O04 | Health / models diagnostics | PARTIAL/REAL | HIGH | Gateway + supervisor | YES | FULL health/models | PARTIAL (PID UI blocked) | RECOVERED (health) / PARTIALLY_EXPOSED (supervisor) | SAFE | Implement diagnostics **after design approval** | v2 `/health`; `desktop-bridge.ts` |
| O05 | Observability Prometheus HTTP | STUB unmounted | LOW | none | NO | — | — | DEAD_LEGACY | — | Drop | v1 `v2_endpoints` no merge |

### Agent / scheduling / provider / desktop extras

| ID | CAPABILITY | V1_REALITY | V1_VALUE | V2_OWNER | V2_IMPL | CANON | DESKTOP | CLASS | SECURITY | REC | EVIDENCE |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A01 | Main agent loop | REAL | CRITICAL | Runtime execute | YES | FULL via chat | FULL text | RECOVERED | GOVERNED_WRITE | Keep single loop | v2 `execute.rs` |
| A02 | Streaming + tools | PARTIAL (stream skips tools) | CRITICAL gap | buffered fake SSE | PARTIAL | PARTIAL | PARTIAL | PARTIALLY_EXPOSED | — | Honest streaming or buffered is OK; emit tool/approval events | v1 skip; v2 three-chunk dump |
| A03 | Goals | REAL machine, weak UI | HIGH | organ `goal` unwired | YES lib | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | GOVERNED_WRITE | TIER 1 module, not HTTP | v1 `goal.rs`; v2 “Production wiring: none” |
| A04 | Principles as second approval | REAL on tool path | HIGH | `InMemoryPrincipleStore` UNWIRED | YES lib | NONE | NONE | INTENTIONALLY_REMOVED | DANGEROUS if HTTP | Must not become second approval authority | v2 `principles.rs` ban |
| A05 | Organs as desktop product | UNUSED (`fetchOrgans` dead) | LOW | OrganModule opt-in | YES | NONE | NONE | DEAD_LEGACY (product) / EXISTS_BUT_NOT_EXPOSED (lib) | — | TIER 2 | v1 desktop unused |
| A06 | Exec isolation | REAL | HIGH | ProcessExecutor | YES | NONE | NONE | RECOVERED | GOVERNED_WRITE | Keep | v2 tools process boundary |
| A07 | Cron jobs / persisted schedules | STUB | — | cron parse only | PARTIAL | NONE | NONE | DEAD_LEGACY | — | Greenfield 2.1 if needed | v1 CronEngine `#[cfg(test)]` |
| A08 | Dream/reflection/emergence clocks | REAL companion biology | MEDIUM | none | NO | NONE | Settings copy | INTENTIONALLY_REMOVED | — | TIER 2 module hooks; no second loop | v1 daemon_loop |
| A09 | Chat abort | REAL | HIGH | Abort on fetch | YES | NONE | FULL | RECOVERED | SAFE | Keep | App.svelte abort |
| P01 | Model listing | REAL static/configured | HIGH | Gateway list_models | YES | FULL | FULL | RECOVERED | SAFE | Remove fake `whisper-1` | `canonical_entry.rs` 354–368 |
| P02 | Endpoint + model settings | REAL | HIGH | Desktop config | YES | NONE | FULL | RECOVERED | SAFE | Keep | SettingsView |
| P03 | Provider credentials | REAL env; crate unwired | CRITICAL | CredentialResolver | YES | NONE | PARTIAL (env only; no UI) | RECOVERED | GOVERNED_WRITE | Never persist provider keys in UI | CLI keyring_bootstrap |
| P04 | Multi-provider routing | REAL if TOML | MEDIUM | ProviderRouter | YES | NONE | NONE | EXISTS_BUT_NOT_EXPOSED | SAFE | TIER 1 settings later | v2 three-provider tests |
| P05 | Frontend provider API key | DISHONEST v1 | — | stripped | NO | — | field is gateway-auth copy | DEAD_LEGACY | DANGEROUS_LEGACY_BYPASS | Do not restore MiniMax key in Settings | v1 localStorage key |
| D01 | Quick window | PARTIAL | LOW | same chat path | YES | PARTIAL | BROKEN uuid prefix | PARTIALLY_EXPOSED | SAFE | Fix session id (logic-only) | `quick-` + uuid |
| D02 | Voice | PARTIAL browser STT | MEDIUM | none duplex | NO | NONE | PARTIAL | PARTIALLY_EXPOSED | SAFE | TIER 2; not LiveKit parity | `voice.ts` |
| D03 | Scene / PAD / presence visuals | PARTIAL without SSE | LOW | none | NO | NONE | empty | INTENTIONALLY_REMOVED / MISSING | — | Optional event adapter | `presence.ts` |

---

## 7. Counts

Denominator = **meaningful v1 REAL/PARTIAL product capabilities** (companion_serve + desktop), excluding TEST-ONLY/STUB/unmounted HTTP.

| Class | Count |
|---|---|
| TOTAL_REAL_V1_CAPABILITIES | **44** |
| RECOVERED | **13** |
| EXISTS_BUT_NOT_EXPOSED | **15** |
| PARTIALLY_EXPOSED | **10** |
| MISSING | **2** (true streaming+tools as one product; companion presence/aliveness) |
| INTENTIONALLY_REMOVED | **4** (master-token packs; companion daemon loop; `/v1/panel` nest; principles-as-HTTP) |
| DEAD_LEGACY | **28** (separate; not in 44) |

### Weighted (of the 44)

| Band | n | Fully recovered | Recovered % |
|---|---|---|---|
| CRITICAL | 8 | 3 (local session family+chat+loop) | **38%** fully; ~88% have *some* impl |
| HIGH | 18 | 5 | **28%** fully exposed |
| MEDIUM | 14 | 4 | **29%** |
| LOW | 4 | 1 | **25%** |

CRITICAL set used: chat, local session continue, memory retrieval, tool execution, governance pause, health, credentials, process isolation.

---

## 8. Product parity scores

**A. IMPLEMENTATION PARITY** = (RECOVERED + EXISTS_BUT_NOT_EXPOSED + PARTIALLY_EXPOSED) / 44  
= (13+15+10)/44 = **86%**

**B. PRODUCT EXPOSURE PARITY** = (RECOVERED + 0.5×PARTIALLY_EXPOSED) / 44  
= (13+5)/44 = **41%**

CRITICAL capability **product** parity (full recover only): **38%**  
HIGH-VALUE product parity: **28%**

Hypothesis confirmation: **15 EXISTS_BUT_NOT_EXPOSED vs 2 MISSING** → most loss is **exposure**, not deletion. The MIXED tail is daemons, streaming tools, and intentional security removals.

---

## 9. Top 20 gaps (value × exists × fit × effort × security)

Especially HIGH VALUE + EXISTS_BUT_NOT_EXPOSED:

1. **Tool events on canonical chat** — loop already dispatches; OpenAI encoder drops tools; desktop therefore has no tool UX. S. RC-relevant.
2. **Desktop `/v1/approvals/resolve`** — CLI already has it; desktop still uses v1 grant. S. RC blocker for governed tools.
3. **Approval inbox read** — pending lives on Session. S.
4. **MemoryInspection (episode list)** — `recent_episodes` exists. M. Not panel routes.
5. **SessionReadModel (list/timeline)** — SessionStore exists. M.
6. **ToolCatalog** — `Runtime::tools()`. S.
7. **AuditQuery from SessionEvent/trace** — already returned on chat. S–M.
8. **Fetch/shell as opt-in governed tools** — impl exists, CLI off. M.
9. **ApplyPatch as governed capability** — library exists. L.
10. **GraphInspection** — experience store wired. M.
11. **Forget/protect on MemoryBackend** — governance sidecar is the wrong store today. L. Security review.
12. **Goals module** — library complete, unwired. M.
13. **A-MEM production recall** — default-off. M.
14. **Provider/model honesty** — drop `whisper-1`. S.
15. **Quick-window SessionId** — logic-only. S.
16. **Stop App 15s v1 approval poll / Activity EventSource** — logic-only leak. S.
17. **Diagnostics UI** — supervisor exists; **blocked on design approval**. M.
18. **True streaming or honest buffered label** — M.
19. **Optional event adapter for presence** — new, TIER 1/2. L. Architecture: module→adapter, not daemon.
20. **MCP host** — library only; TIER 2. XL. Do not port `npx` installer.

---

## 10. Real implementation losses (not just exposure)

1. Companion **presence/emotion/initiative daemon** (second loop).
2. **Dream/reflection schedulers** as resident biology.
3. **Streaming tool loop** (already skipped in v1 desktop; still absent).
4. MCP **not in the product chat process** (also true in v1 companion).
5. v1 **PermissionPack** model (intentionally replaced, not lost by accident).

Everything else in the HIGH list is present as a crate, module, or store.

---

## 11. Dead legacy that must not return

- `companion_serve`, `:8090`, `0.0.0.0` + “any non-empty key”, `POST /v1/apeireth/test-event`, `/panel` HTML
- Entire `/v1/panel/*` nest (even the honest GET-only `panel_readonly.rs`)
- `/v1/apeireth/grant`, `APEIRETH_GRANT`, master_token-in-body, PermissionPack
- reconstruction_v2 mock router (grant always `{ok:true}`, fake capabilities, `vision_act`, `factory_merge`, `mcp_install`)
- `ToolExecutor` / `UnifiedRuntimeHost` / master `ToolRegistry` / `host.api_key`
- Fake stream `'执行成功'`
- Frontend persistence of provider keys / master tokens
- Second dream/heartbeat loop beside `Runtime::execute`
- Principles HTTP as a second approval authority
- `whisper-1` as a chat model
- Duplicate N17 unsandboxed names that bypassed RiskRule prefixes

---

## 12. Governance / security risks

| Item | Class |
|---|---|
| Canonical tool dispatch + approval freeze | GOVERNED_WRITE — keep |
| Chat/session/memory **inside** the turn | GOVERNED_WRITE — keep |
| Health/models/trace-on-response | SAFE_READ |
| MemoryInspection / SessionReadModel / ToolCatalog | SAFE_READ if no mutation |
| Human memory forget/protect | GOVERNED_WRITE — needs review |
| v1 grant / packs / master_token | DANGEROUS_LEGACY_BYPASS |
| Direct ToolRegistry.execute / desktop_action | DANGEROUS_LEGACY_BYPASS |
| Public `/v1/memory/append` | DANGEROUS_LEGACY_BYPASS |
| Stream path that skips governance | DANGEROUS_LEGACY_BYPASS |

Kernel must not gain memory-browser or audit-UI logic. Preferred: owner module → read model/command → adapter → desktop.

---

## 13. Recovery roadmap (proposal only — not implemented)

For each item: owner, exists, missing, adapter surface, R/W, UI, risks, complexity, RC blocker.

| CAPABILITY | OWNER | EXISTS | MISSING | ADAPTER | R/W | UI | ARCH | SEC | SIZE | RC |
|---|---|---|---|---|---|---|---|---|---|---|
| Tool/approval events on chat | Runtime/Gateway | dispatch + PendingApproval | OpenAI wire + desktop handler | extend completions **or** use native `/v1/chat` | GOVERNED | Chat cards + resolve | low if no second loop | low | M | **YES** |
| Desktop resolve | Gateway already | `/v1/approvals/resolve` | fetcher | none new | GOVERNED | Tools/Chat | low | low | S | **YES** |
| Approval inbox | Session | pending on session | list API | ApprovalInbox | READ | Tools | low | low | S | YES if tools |
| ToolCatalog | Runtime.tools() | yes | HTTP | ToolCatalog | READ | ToolsView | low | low | S | NO (RC can ship without registry UI) |
| MemoryInspection | MemoryBackend | recent_episodes | HTTP | MemoryInspection | READ | MemoryView | low | low | M | NO |
| SessionReadModel | SessionStore | yes | HTTP | SessionReadModel | READ | Conversations backend tab | low | low | M | NO |
| AuditQuery | SessionEvent/trace | yes | list | AuditQuery | READ | Activity | low | low | M | NO |
| Diagnostics | Supervisor | yes | design approval | existing Tauri cmds | READ + restart | Settings | low | low | M | NO (design-gated) |
| Fetch/shell opt-in | tools-canonical | yes | product default policy | none | GOVERNED | Settings later | med | med | M | NO |
| ApplyPatch | library | yes | ToolCapability | capability | GOVERNED | none at first | med | high | L | NO |
| Goals | organ/goal | lib | module wire | command | GOVERNED | later | med | med | M | NO |
| Presence/events | none | no | new module+adapter | EventStream | READ | Activity/scene | **high if daemon** | low | L | NO |
| Dream loop | none | no | AfterTurn/scheduled module | none | WRITE internal | none | high if second loop | med | L | NO |
| MCP host | lib | partial | composition | CapabilityRegistry only | GOVERNED | none | high | high | XL | NO |

---

## 14. Release tiers

### TIER 0 — 2.0 RC blockers (semantic, not endpoint)

- Chat + session_id continuity (already largely present).
- Honest tool/approval path on the **desktop chat** (today CLI can approve; UI cannot).
- Do **not** ship fake tool success or v1 grant.
- Health/models/credentials-from-env (present).
- Stop hitting known-dead v1 URLs from App/Activity (logic-only).

Calling 2.0 “feature-complete vs meaningful v1” **without** desktop approval/tool visibility is false: v1 non-stream companion *could* run tools; v2 runtime *can* too, but the shipped desktop cannot see or resolve them.

### TIER 1 — 2.0 final

- MemoryInspection, SessionReadModel, ToolCatalog, AuditQuery (new semantic adapters, **not** panel).
- Diagnostics UI after `DESIGN APPROVED`.
- Fetch/shell documented opt-in.
- Drop `whisper-1`; fix quick-window SessionId.
- Optional A-MEM recall quality.

### TIER 2 — 2.1

- Goals UI, ApplyPatch capability, MCP host, presence/event adapter, dream/reflection as **modules**, voice duplex, vector search, six-stream browser.

### TIER X — do not restore

See §11.

---

## 15. Answers the report must give

1. **Did v2 lose major capabilities?**  
   Internally: mostly no. As a **desktop product**: yes — memory/tools/activity inspection and desktop tool/approval UX.

2. **Implementation vs exposure?**  
   ~15 exposure gaps vs ~2 true holes (+4 intentional removals).

3. **Critical/high inaccessible today?**  
   Tool execution visibility, approval resolve on desktop, memory/session/tool/audit inspection.

4. **What must not return?**  
   companion_serve, panel nest, grant/packs, mock reconstruction gateway, fake stream tools, provider keys in UI.

5. **Memory, sessions, tools, governance, audit internally?**  
   YES (memory+session stores, tool modules, governance hook, trace/events on session). Audit is per-turn, not a panel table.

6. **Adapter-only?**  
   ToolCatalog, MemoryInspection, SessionReadModel, AuditQuery, ApprovalInbox, desktop resolve.

7. **Need reconstruction?**  
   Presence/dream as product; true token-SSE+tools; MCP host; forget/protect on the production MemoryBackend.

8. **Need governance redesign before exposure?**  
   Memory mutation, patch/shell/fetch defaults, any grant-like API.

9. **Is 2.0 Desktop semantically feature-complete vs meaningful v1?**  
   **NO.**

10. **Should RC packaging continue before parity recovery?**  
    **YES for a narrowly honest RC** (chat + local sessions + health + CLI approval + sidecar), **NO** if RC is marketed as v1 feature-complete. Recommend: continue packaging **and** TIER 0 desktop tool/approval honesty before calling it 2.0-complete.

---

## 16. Required summary block

V1 HEAD: `76c87048deb9b695d34376d6cba85ebc5a6408fb` (`origin/archive/v1.0-master`)  
V2 HEAD: `bc02b23d36731dd04ac15b86df6883a716423552`

TOTAL REAL V1 CAPABILITIES: **44**

RECOVERED: **13**  
EXISTS_BUT_NOT_EXPOSED: **15**  
PARTIALLY_EXPOSED: **10**  
MISSING: **2**  
INTENTIONALLY_REMOVED: **4**  
DEAD_LEGACY: **28** (separate denominator)

IMPLEMENTATION PARITY: **86%**  
PRODUCT EXPOSURE PARITY: **41%**  
CRITICAL CAPABILITY PARITY (full product recover): **38%**  
HIGH-VALUE CAPABILITY PARITY (full product recover): **28%**

TOP CRITICAL GAPS:

1. Desktop cannot see or execute the v2 tool loop (OpenAI wire has no tools; v1 stream also skipped — v2 still has a real loop unused by UI).
2. Desktop cannot call `/v1/approvals/resolve` (CLI can).
3. Memory/session/tool/audit inspection exists in stores, not on the product surface.
4. Companion presence/dream aliveness has no v2 owner (intentional architecture; product hole).
5. App/Activity still probe dead v1 URLs.

TOP HIGH-VALUE EXISTS-BUT-NOT-EXPOSED:

1. ToolCatalog (`Runtime::tools()`)
2. MemoryInspection (`recent_episodes`)
3. SessionReadModel (`SessionStore`)
4. ApprovalInbox (pending on session)
5. AuditQuery (SessionEvent / ExecutionTrace)
6. Fetch/shell modules (off by default)
7. ApplyPatch library
8. Experience graph inspect
9. Goals library
10. Provider router (no UI)

REAL IMPLEMENTATION LOSSES: presence daemon, dream/reflection clocks, streaming tool loop, MCP-in-chat (already dead in v1 companion).

DEAD LEGACY THAT MUST NOT RETURN: companion_serve/:8090, `/v1/panel/*`, grant/packs/master_token, reconstruction mock gateway, ToolExecutor/UnifiedRuntimeHost, fake stream success, frontend provider keys.

GOVERNANCE/SECURITY RISKS: restoring grant/append/direct execute; stream that skips the loop; principles HTTP.

TIER 0 — 2.0 RC BLOCKERS: desktop tool/approval honesty; stop dead-URL polls; keep single Runtime loop.

TIER 1 — 2.0 FINAL: read-model adapters (session/memory/tools/audit); diagnostics after design approval.

TIER 2 — 2.1: goals, patch, MCP, events, dream-as-module, voice duplex, vectors.

TIER X — DO NOT RESTORE: §11.

MOST CAPABILITY LOSS IS EXPOSURE LOSS: **YES** (MIXED tail: daemons + streaming tools + intentional security)

CURRENT 2.0 DESKTOP IS SEMANTICALLY FEATURE-COMPLETE VS V1: **NO**

RC PACKAGING SHOULD CONTINUE BEFORE PARITY RECOVERY: **YES, if scoped honestly; NO, if claiming v1 feature-complete**

---

APEIREITH V1 → V2 SEMANTIC FEATURE PARITY AUDIT: **FAIL** (exposure), **PASS** (implementation spine)

V2 IMPLEMENTATION PARITY: **86%**

V2 PRODUCT EXPOSURE PARITY: **41%**

CRITICAL PARITY GAPS REMAIN: **YES**

ARCHITECTURE-PRESERVING RECOVERY PLAN READY: **YES**

CODE MODIFIED: **NO**
