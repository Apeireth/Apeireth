# Apeireth Security Model (1.0)

> 对齐实际代码（2026-08-18）。安全底线永不外包。

> **现状 (2026-08-27)**：本文是 v1.0（86-crate 时代）的安全模型快照，安全设计本身（三洋葱 / L0 永不改变 / 默认拒绝哲学）不变。工程现状：表中旧 crate 现位于 `legacy/`；S4 出站"实装待补"已被 M2D egress + M3A 受控 fetch 取代（`crates/capabilities/tools/src/{egress,fetch}.rs`）；当前工作区的安全边界清单见根 `SECURITY.md`，恢复排期见根 `ROADMAP.md` §4。

## Layers (from code)

| Layer | Mechanism | Crate |
|---|---|---|
| **Approval** | 5-rule ApprovalManager (blacklist/trust/risk/frequency/whitelist) + ApprovalBridge (companion ↔ orchestrator, silent 透传) | tool-approval, team-lead |
| **Permission** | 双洋葱 (principle onion embedded in L0-L5), permission packs, Self-Disable protection | sovereignty, packs |
| **Constitution** | structured constitution hard-gate (compile-time rules) + LLM constitution review (E-layer judging) | council, companion::constitution_gate |
| **Isolation** | Windows Job Object (time/memory/CPU limits + violation trace) + restricted token + AppContainer trait 口 + exec_worker per-call subprocess | companion::job_object, restricted_token, app_container |
| **Privacy** | PII detection + redaction + audit (outbound LLM requests scrubbed) | apeireth-guard |
| **Outbound** | **S4 default-deny** (trait 口已备): domain/protocol allowlist + SHA-256 audit chain + budget hook — **实装待补** (per backlog S4 P1 未实施, 2026-08-18 复核) | http-client::egress (trait) |
| **Audit** | HASH-SQL arbitration (immutable timeline), session log hash chain, egress audit chain | arbitration, session_log |
| **Credentials** | CredentialsStore trait + file backend; high-risk credentials go through approval gate trait | credentials |

## The Triple Onion (三洋葱, R125-5 升双→三, 加 DSL 洋葱)

- **L0**: real human approval — **never mutable** (Self-Disable "百年章节" prevents AI self-bypass)
- V1 (principles) + V2 (permissions) + V3 (HA) AND-gate: any independent rejection blocks
- DSL onion (Colang DSL 守门, R125-5 NVIDIA Guardrails 借鉴): 表达"什么操作允许/禁止"
- Risk grading → council seat count (critical 7 / high 5 / medium 3 / low 1)

## Verified Behaviors

- Outbound: `egress.rs` — **trait 口已备, 实装待补** (backlog S4 P1 未实施; 真出站策略在 gateway 暂未挂载, 2026-08-18 复核)
- Tool: schema validation rejects missing/wrong-typed fields; guardrail blocks path traversal + shell injection; tripwire flags credential leaks
- Sandbox: CPU-time limit kills child (trace kept); memory limit **denies allocation** (Windows semantics: OOM not kill — 0 装 PASS)
- Approval: rejected → cannot approve (only pending); silent reject transmitted end-to-end (N20)

## Honest Status

- Docker build untested (no docker locally — labeled 待实测)
- VM-level isolation (smol-vm / Hyperlight): **research only** — not implemented (labeled)
- AppContainer: trait 口已备未接 (labeled)
