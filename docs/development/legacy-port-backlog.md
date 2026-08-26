# Legacy Port Backlog

The current workspace is the canonical product boundary. The entries below
record functionality intentionally left behind when legacy dependencies were
removed. A donor location is a source of reference, not a production dependency.

| Feature | Donor location | Missing canonical behavior | Target owner | Disposition |
| --- | --- | --- | --- | --- |
| Living-day orchestration and background heartbeat | `legacy/donor/apeireth-supervisor`, `legacy/donor/apeireth-workflow`, `legacy/donor/apeireth-bus` | No autonomous scheduler, workflow worker, or event bus in the canonical runtime | `apeireth-runtime` | REASSESS — canonical turns and session lifecycle are the current contract |
| Provider facade and multi-vendor HTTP dispatcher | `legacy/donor/apeireth-acp`, `legacy/donor/apeireth-llm-iface`, `legacy/donor/apeireth-http-client` | No facade-level status API or shared dispatcher; canonical providers own wire translation | `apeireth-provider` | DROP — superseded by `ProviderCapability` plugins and normalized protocol |
| Memory-side LLM episode analysis | `legacy/donor/apeireth-llm-iface` and removed memory bridge | No canonical analysis capability or prompt/extraction policy | `apeireth-memory` | PORT — only after a canonical memory-analysis contract is specified |
| Text embedding and persistent semantic-vector integration | `legacy/donor/apeireth-vector` and removed semantic modules | Canonical memory has a standalone vector index, but no text embedder or persistent vec0 bridge | `apeireth-memory` | REASSESS — preserve canonical vector/graph primitives; do not restore the donor bridge |
| External memory provider implementations | `legacy/donor/apeireth-memory-extensions` | No file/Mongo provider adapters in the canonical repository API | `apeireth-memory` | REASSESS — add adapters only behind an approved canonical repository contract |
| Life-force and reflection semantics | `legacy/donor/apeireth-life-force` | No canonical endurance/reflection model | `apeireth-memory` or a future module | PORT — require an explicit ownership and governance design |
| Gateway privacy guard and rate limiting | `legacy/donor/apeireth-guard`, `legacy/donor/apeireth-rate-limiter` | Canonical gateway currently validates transport and delegates execution; no PII redaction or quota policy | `apeireth-gateway` / `apeireth-governance` | PORT — define policy at the adapter/governance boundary before implementation |
| CLI skills, eval, council, MCP, and ASI commands | `legacy/donor/apeireth-skills`, `legacy/donor/apeireth-eval`, `legacy/donor/apeireth-council`, `legacy/donor/apeireth-mcp`, `legacy/donor/apeireth-asi` | CLI now exposes only session, chat, and gateway serve | Future dedicated adapters | REASSESS — no historical command is part of the canonical CLI contract |
| Companion module | `legacy/donor/apeireth-companion` | No canonical companion feature module in the current workspace | `crates/modules/companion` | PORT — reintroduce only as a canonical plugin/module with no donor dependency |

This backlog is deliberately explicit about gaps. It is not permission for
current crates to import donor code, and it does not claim that donor behavior
is preserved by the canonical APIs.
