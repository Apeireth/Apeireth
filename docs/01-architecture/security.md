# Apeireth Security Model (1.0)

> 对齐实际代码（2026-08-18）。安全底线永不外包。

## Layers (from code)

| Layer | Mechanism | Crate |
|---|---|---|
| **Approval** | 5-rule ApprovalManager (blacklist/trust/risk/frequency/whitelist) + ApprovalBridge (companion ↔ orchestrator, silent 透传) | tool-approval, team-lead |
| **Permission** | 双洋葱 (principle onion embedded in L0-L5), permission packs, Self-Disable protection | sovereignty, packs |
| **Constitution** | structured constitution hard-gate (compile-time rules) + LLM constitution review (E-layer judging) | council, companion::constitution_gate |
| **Isolation** | Windows Job Object (time/memory/CPU limits + violation trace) + restricted token + AppContainer trait 口 + exec_worker per-call subprocess | companion::job_object, restricted_token, app_container |
| **Privacy** | PII detection + redaction + audit (outbound LLM requests scrubbed) | apeireth-guard |
| **Outbound** | **S4 default-deny**: domain/protocol allowlist + SHA-256 audit chain + budget hook — every HttpClient request checked | http-client::egress |
| **Audit** | HASH-SQL arbitration (immutable timeline), session log hash chain, egress audit chain | arbitration, session_log |
| **Credentials** | CredentialsStore trait + file backend; high-risk credentials go through approval gate trait | credentials |

## The Double Onion (核心)

- **L0**: real human approval — **never mutable** (Self-Disable "百年章节" prevents AI self-bypass)
- V1 (principles) + V2 (permissions) + V3 (HA) AND-gate: any independent rejection blocks
- Risk grading → council seat count (critical 7 / high 5 / medium 3 / low 1)

## Verified Behaviors

- Outbound: `egress.rs` — default deny outside allowlist; https-only unless explicitly allowed; audit chain tamper-detected (tests)
- Tool: schema validation rejects missing/wrong-typed fields; guardrail blocks path traversal + shell injection; tripwire flags credential leaks
- Sandbox: CPU-time limit kills child (trace kept); memory limit **denies allocation** (Windows semantics: OOM not kill — 0 装 PASS)
- Approval: rejected → cannot approve (only pending); silent reject transmitted end-to-end (N20)

## Honest Status

- Docker build untested (no docker locally — labeled 待实测)
- VM-level isolation (smol-vm / Hyperlight): **research only** — not implemented (labeled)
- AppContainer: trait 口已备未接 (labeled)
