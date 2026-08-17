# Apeireth Architecture (1.0)

> 对齐实际代码（2026-08-18 master）。84 active crates / ~34 万行 Rust。

## Layer View

```
┌─────────────────────────────────────────────────────┐
│  Frontends (consumers via HTTP/ACP)                 │
│  TUI · Web panel · companion_serve (:8090)          │
└──────────────────────┬──────────────────────────────┘
                       │ OpenAI-compatible / HTTP
┌──────────────────────▼──────────────────────────────┐
│  Companion (apeireth-companion, the partner organ)  │
│  injection pipeline · memory v2 · world model       │
│  curiosity · hypothesis · emotions · emergence      │
│  value cases · daemon (dream/reflect/utter)         │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│  Organs & Services                                  │
│  council (7 advisors) · cognition · consciousness   │
│  evolution · sovereignty · arbitration · gateway    │
│  bus (5-layer) · workflow · experience · skills     │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│  Tool Layer (9 sub-crates + runtime)                │
│  registry → approval → executor → record            │
│  schema validation · guardrails · egress policy     │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│  Base (memory store · http-client · protocol · api) │
└─────────────────────────────────────────────────────┘
```

## Crate Groups (85 crates, aligned with code)

| Group | Crates | Responsibility |
|---|---|---|
| **Companion core** | companion (25K 行) | The partner organ: injection, memory, world model, curiosity, hypothesis, emotions, value, emergence, daemon |
| **Cognition & self** | cognition, consciousness, evolution, council, asi, critic, value, motivation, life-force, perception | Thinking, self-improvement, 7-advisor deliberation, ASI metrics |
| **Memory** | memory (11K), experience, graph, graph-primitive, vector, context-fold | SQLite memory v2, knowledge, deterministic graphs |
| **Tools** | tool-registry, tool-runtime, tool-approval, tools, tool-shell, tool-fetch, tool-browser, tool-codesearch, tool-search, tool-filesystem, tool-image-gen, tool-image-process | The tool pipeline with schema + guardrails |
| **Security** | sovereignty, guard, arbitration, credentials, restricted-token, directory-acl, app-container, sandbox | Double onion, PII redaction, HASH-SQL audit, sandboxes |
| **Infrastructure** | bus, api, protocol, gateway, http-client, acp, mcp, telemetry, pipeline, pipeline-g5, runtime, host, central, cron, config, i18n | Transport, gateway, protocols, lifecycle |
| **Integration** | integration-e2e, pybridge, tui, tui-e2e, web, cli, sdk, bench, eval, repo-tools, naming-v05, blueprint-impl, upgrade, plugin, extension | Consumers, tooling, e2e |
| **External adapters** | stock (N3 financial data), wiki (Markdown KB), lark, voice, livekit, rate-limiter, llm-iface, constraint, onion, supervi-sor, team-lead, agent, workflow, action, central, experience, credentials, cron, environment, eval, gateway, naming-v05, telemetry, upgrade, value, vector, verify, archive | Adapters & ecosystem |

> Full list with descriptions: [docs/03-reference/crates.md](../03-reference/crates.md)

## Key Data Flows

- **Injection**: memory ranking + graph + preferences + today + growth → ContextAssembler (L0/L1 core, budgeted) → LLM context
- **Memory**: dialog → extraction (importance) → reconcile (ADD/UPDATE/DELETE + tomb) → ranking → injection
- **World model**: counterfactual timeline → Brier calibration → reject if uncalibrated; causal graph MCTS with LLM at branch points
- **Tools**: LLM emits `<<<[TOOL_REQUEST]>>>` → parser → approval (5 rules + bridge) → executor (schema/guardrail) → record → observer capture (W5)
- **Emergence**: rhythm + relationship pressure + mood gate → Initiative → LLM utterance (min-interval + backoff)

## Runtime

- `companion_serve` — the full partner endpoint (:8090, OpenAI-compatible): L0/L1 injection, daemon resident (dream/reflect/utter), tool bridge, approval queue
- `apeireth-tui` — terminal companion dashboard
- `apeireth-cli` — CLI runner (assembly matrix: base/suite/plugin)
