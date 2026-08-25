# M1C — Canonical Governance Donor Primitives

Status: complete
Branch: `reconstruct_v2`
Starting HEAD: `af49988fd36be5a2e132a634acdabcaf4d29e7f8`
Donor: `origin/master:reconstruction_v2/crates/apeireth-governance/src/`

This migration took algorithms and primitives from the master donor
implementation and adapted them to the frozen canonical governance contract in
`apeireth-governance`. It did **not** port the donor architecture.

## Donor source

| Donor module | Maturity | Ported? | Strategy |
| --- | --- | --- | --- |
| `guard.rs` (`PiiDetector`) | REAL | yes | ADAPT |
| `audit.rs` (`AuditHashChain`) | REAL | yes | ADAPT |
| `onion.rs` (`Permission`, `PermissionPack`, `PermissionOnion`) | PARTIAL | partial | ADAPT permission core only |
| `onion.rs` (`PrincipleOnion`, `DslOnion`) | PARTIAL | no | DROP |
| `self_disable.rs` (`SelfDisableGuard`, `Scanner`) | PARTIAL | no | DROP / DEFER |
| `gates.rs` (five-gate `GovernancePipeline`) | PARTIAL | no | DROP |
| `sovereignty.rs` | REAL | no | DEFER |

## Ported

### `input_security` (M1C-A)

Donor `guard.rs` used a single `PiiDetector` struct for three concerns: PII
scrubbing, prompt-injection blocking, and implicit security classification.
M1C-A separated them:

- `PiiDetector::findings` -> structured `PiiFinding { kind, start, end }`.
  Kinds: `Email`, `Phone` (mainland-China mobile shape from donor), and
  `CredentialKey` (`sk-...`, `AKIA...`, and a `Bearer` token shape).
  Offsets are **byte offsets**.
- `PiiDetector::redact` -> stable placeholders (`[REDACTED_EMAIL]`,
  `[REDACTED_PHONE]`, `[REDACTED_CREDENTIAL]`).
- `PromptInjectionHeuristic::signals` -> structured
  `PromptInjectionSignal { kind, start, end }` from the donor's five patterns,
  matched case-insensitively with compiled static regexes.
- `PromptInjectionHook` and `CredentialDisclosureHook` -> canonical
  `GovernanceHook` implementations. They inspect capability-dispatch arguments
  and map findings to `RequireApproval`, never to an implicit global `Deny`.
  Email/phone PII alone does not change the decision.

### `audit` (M1C-B)

Donor `audit.rs` was a real append-only hash chain, but appended with
`chrono::Utc::now()` and hashed a `:`-separated `format!` string. M1C-B:

- `append` takes an explicit `apeireth_core::kernel::Timestamp` (no wall clock
  inside the primitive).
- Canonical serialization is fixed field order with length prefixes:
  `sequence:u64`, `timestamp_epoch_millis:i64`, length-prefixed `event_kind`,
  length-prefixed `subject`, length-prefixed `previous_hash`.
- Genesis previous hash is the 64-char zero hash.
- `verify()` detects sequence mismatch, genesis mismatch, previous-hash pointer
  breaks, and record hash corruption; removal and reorder therefore fail.
- Stored in memory only. `ExecutionTrace` was not replaced.

### `permission` (M1C-C)

Donor `onion.rs` had a deterministic `Permission` / `PermissionPack` core plus
philosophy-coded `PrincipleOnion` and an ad-hoc `DslOnion`. M1C-C ported only
the deterministic core:

- `Permission` enum (`ReadMemory`, `WriteMemory`, `ExecuteTool(String)`,
  `NetworkEgress(String)`, `ModifyIdentity`, `AdminOverride`).
- `PermissionSet` over `BTreeSet` for deterministic iteration.
- `PermissionPolicy::decision_for_capability` maps capability dispatch to the
  existing canonical `Decision`:
  - missing grant and no `AdminOverride` -> `Deny`
  - granted but marked for approval -> `RequireApproval`
  - otherwise -> `Allow`
- `PermissionGovernanceHook` wraps the policy for `GovernancePipeline`.

## Rejected

- **Donor five-gate `GovernancePipeline`** (`gates.rs`): a second pipeline with
  philosophy keys and council voting. The canonical `GovernancePipeline` already
  composes hooks. Not ported.
- **`PrincipleOnion` and `DslOnion`**: facade layering and hardcoded product
  philosophy; no new canonical semantics. Not ported.
- **Donor `standard_agent` permission pack**: hardcoded vendor egress
  (`api.minimax.chat`) and product-specific tool grants. Not ported.
- **`SelfDisableGuard`**: duplicates existing `apeireth-core` self-disable
  baseline. Not ported in M1C.
- **`LifecycleHandle`** (donor runtime facade): not ported.
- **`SovereignControl`**: real but role/pause-resume policy with hashed token;
  outside the M1C scope. Deferred.

## Deferred

- Persistent audit-chain backend.
- Runtime wiring of audit-chain append at every governance event.
- Runtime/gateway integration for completion-content input security (the
  canonical `GovernanceRequest` currently carries capability arguments but not
  provider transcript text; adding that is a canonical context extension, not a
  detector change).
- `SovereignControl` and any role/pause-resume policy.
- Gateway auth, tool sandbox policy, network/egress policy, MCP policy, and
  credential backend integration remain later phases.

## Security limitations

- `PiiDetector` is heuristic and pattern-based. It detects the configured
  patterns only; it does **not** detect all secrets or all PII.
- `PromptInjectionHeuristic` is a signal for human review. It is **not**
  complete protection against prompt injection, and its matches are mapped to
  `RequireApproval`, not treated as a security boundary.
- `AuditHashChain` is tamper-evident, **not** tamper-proof. It proves that
  in-memory records changed, but it does not prevent mutation or persist the
  chain.
- Redaction is best-effort string replacement for the configured categories.
- Tests use fake fixtures only; no real secrets are stored or printed.
