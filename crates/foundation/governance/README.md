# apeireth-governance

> Foundation layer: the canonical governance hook — the decision point the runtime consults before it acts.

`apeireth-governance` sits in the **foundation** layer of the Apeireth engine workspace.
It answers exactly one question: *may this action proceed?* The answer is a `Decision`,
and the runtime is required to honour it. That is the entire canonical surface.

## The contract

- `GovernanceHook` — the one trait the runtime holds as `Arc<dyn GovernanceHook>`
- `Decision` — the decision enum (`Allow` / deny family) returned by a hook
- `GovernancePipeline` — runs hooks in order and stops at the first non-allow decision

Keeping the contract this thin is deliberate. The branch this converges had governance
logic scattered across several crates with no shared decision type, so the runtime could
not consult "governance" at all — only specific gates it happened to know about, in an
order fixed by whoever wrote the call site.

## What is inside

Baseline algorithms live here as **default-off library helpers**. They do **not**
implement `GovernanceHook` and are **not** installed in `GovernancePipeline`; mapping a
helper onto `Decision` is the caller's job:

- `colang.rs` — policy DSL parser
- `approval_policy.rs` / `approval_family.rs` / `approval_closure.rs` — approval scoring
- `eval.rs` / `evidence.rs` / `rubric.rs` / `risk.rs` — evaluation, evidence and risk rank
- `sandbox_ladder.rs` / `sandbox_preset.rs` — sandbox escalation ladder
- `chain.rs` — governance chain composition
- `audit.rs` / `authorization.rs` / `permission.rs` / `rate_limit.rs` — audit trail,
  authorization, permission and rate-limit surfaces
- `input_security.rs` / `untrusted_mark.rs` / `tool_guard.rs` / `tool_desc_audit.rs` —
  input and tool-description hardening
- `intent.rs`, `research_autonomy.rs`, `user_access.rs` — intent interpretation,
  research autonomy policy and user access

## Layering

Depends on `apeireth-core` and nothing else. It must not depend on the runtime, the
gateway, storage, or any concrete capability: a policy that knows what it is guarding
cannot be reused to guard anything else.

## Install

```toml
[dependencies]
apeireth-governance = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT.

## Links

- Source: <https://github.com/Apeireth/Apeireth/tree/main/crates/foundation/governance>
- Workspace manifest: <https://github.com/Apeireth/Apeireth/tree/main/crates/foundation/governance/Cargo.toml>
- Architecture docs: <https://github.com/Apeireth/Apeireth/tree/main/docs/01-architecture>
