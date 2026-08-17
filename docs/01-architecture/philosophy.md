# Apeireth Philosophy

## The Six Anchors

| Anchor | Meaning |
|---|---|
| S-1 北极星 | Everything serves the ASI north star (五原型) |
| S-2 实事求是 | Verify before writing; truth over narrative |
| O-2 前人肩上 | Stand on prior work (borrow, attribute, adapt) |
| O-3 干到底 | Finish what we start; no half-measures |
| O-4 任何人都能接手 | Any newcomer can onboard from docs alone |
| O-5 不假装 (0 装 PASS) | **Never fake it** — the trust bedrock |

## Core Principles

- **基地不是 AI 本身**: the LLM is a tenant; swap models without rebuilding the base (trait strategy everywhere)
- **涌现优先于预定义**: capabilities grow, not pre-built
- **用户是伙伴**: partner = remembers you across sessions, understands you
- **机制而非补丁**: every "add an if" must ask: what is the mechanism?
- **集成而非分立**: new needs hang onto existing mechanisms
- **文档同步自觉**: code changes update docs; research lands in the ledger

## Double Onion (双洋葱)

**Principle onion** (E/S/A/M/O principles) **embedded in** the **permission onion** (L0–L5):

- L0: human approval — **never mutable** (Self-Disable protection, "百年章节")
- L1-L5: escalating permission layers (approval gate, sandbox, etc.)
- Any layer can independently reject (V1+V2+V3 AND gate)

## 0 装 PASS (The Trust Bedrock)

- Unimplemented = labeled `trait 口已备未接`, never silent
- Real network calls in tests (with rate-limit backoff), honest failure
- Docker untested = marked "待实测", not "done"
- Error messages are actionable, not generic

## Key Mechanisms (all implemented, verified by tests)

| Mechanism | Where |
|---|---|
| Memory v2 (importance/reconcile/ranking/versioned chains) | `apeireth-companion::memory_extractor` |
| Memory graph (temporal facts, weighted links, crawl) | `apeireth-companion::memory_graph` |
| World model W1/W2/W3 | `world_model.rs` + `causal_world_model.rs` |
| Curiosity engine (E4) | `curiosity.rs` |
| Hypothesis testing (F4) | `hypothesis.rs` |
| Emotion memory (F1) | `emotion_memory.rs` |
| Value cases (F6) | `value_cases.rs` |
| Emergence loop (E7, when to speak) | `emergence.rs` |
| Tool pipeline (schema/guardrail/approval) | `apeireth-tool-runtime` + `apeireth-tool-approval` |
| Outbound policy (S4, default-deny + audit chain) | `apeireth-http-client::egress` |
| Event bridge + PerceptionGate (A4/TP26) | `apeireth-bus::event_bridge` |
