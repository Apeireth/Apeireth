# M2C-A — Canonical Durable Approval / Resume Lifecycle

Status: complete
Branch: `reconstruct_v2`

## 1. Problem

Before M2C-A, `Runtime::execute` returned `RuntimeError::ApprovalRequired` and
stopped the turn. No pending operation was persisted, no stable approval id
existed, and there was no way to approve or reject later. A tool that required
approval could block execution but could not be resumed.

## 2. Ownership

- **Governance** still owns only: should this operation require approval?
- **Runtime** owns: what is waiting, what exact operation is frozen, how it is
  resumed, whether it was already resolved, and where the turn continues.
- **Session / persistence** owns durability through the existing
  `SessionStore` seam.
- **ToolCapability** owns tool execution semantics.
- **ProcessExecutor** owns OS process execution.
- **Gateway / CLI** own presentation/adaptation only.

No `ApprovalManager`, `ApprovalRuntime`, `ApprovalExecutor`,
`ApprovalEngine`, `ApprovalRegistry`, or `ApprovalPipeline` exists.

## 3. New public contract

### 3.1 `ApprovalId`

A UUID-backed generated id in `apeireth_core::kernel::ids`, following the same
pattern as `SessionId`, `TraceId`, and `RequestId`.

### 3.2 Outcome model

```rust
pub enum TurnOutcome {
    Completed(TurnResponse),
    PendingApproval(PendingApprovalView),
}
```

`Runtime::execute_outcome` returns `TurnOutcome`. `Runtime::execute` remains as
a compatibility wrapper and maps `PendingApproval` back to
`RuntimeError::ApprovalRequired` for callers that have not adopted the new
model.

### 3.3 Resolution model

```rust
pub enum ApprovalDecision {
    Approve,
    Reject { reason: Option<String> },
}

pub enum ApprovalResolution {
    Resumed(TurnOutcome),
    AlreadyResolved { status: ApprovalStatus },
    Expired,
    NotFound,
}
```

`Runtime::resolve_approval(session, approval_id, decision)` resolves the
stored operation. Resolvers supply only a decision and optional human reason;
they never supply replacement tool arguments, cwd, script text, or process
configuration.

### 3.4 Approval status

```text
Pending -> Approved -> Consumed
Pending -> Rejected
Pending -> Expired
```

- `Pending` is the only resumable state.
- `Rejected`, `Expired`, and `Consumed` are terminal.
- `Approved` is the atomic claim state: after claim, no other resolver may
  execute the operation. `Consumed` is recorded after tool invocation and
  transcript append.

## 4. Frozen continuation

`FrozenTurnContinuation` stores:

- `request_id`
- `trace_id`
- `model`
- `round`
- the original assistant `tool_calls` batch
- `next_tool_index`
- optional `approved_tool_index` / `approved_approval_id`

The provider is never re-queried to regenerate the original tool call. The
original `ToolCall` batch is authoritative.

## 5. Operation fingerprint

`operation_fingerprint_with_invocation` hashes, with SHA-256 and canonical
JSON:

- action kind
- capability id
- tool name
- tool call id
- tool arguments
- capability-frozen `effective_invocation` when supplied
- session id
- request id
- round

JSON object keys are sorted recursively; arrays keep order. Object insertion
order does not change the fingerprint.

## 6. Persistence and reopen

Pending approvals are stored inside `Session` as orchestration metadata:

- `Session::approvals: BTreeMap<ApprovalId, PendingApproval>`
- `Session::active_approval_id: Option<ApprovalId>`

They are persisted through the existing `SessionStore` seam and are therefore
as durable as the configured store. The in-memory store supports runtime
rebuild/reopen in the same process. A future durable `SessionStore` backend
makes the same lifecycle durable across process restarts.

## 7. Concurrency

`Runtime` owns per-session locks (`SessionLocks`). Different sessions may
proceed concurrently; the same session cannot start a new turn or resolve an
approval while another operation on that session is in progress. Atomic claim
is performed under the per-session lock, so two concurrent approvers cannot
double-execute.

A new turn on a session with a pending approval returns
`RuntimeError::SessionApprovalPending`.

## 8. Expiry

Every pending approval has `expires_at` computed from the runtime clock and
`RuntimeConfig::approval_ttl_ms` (default 5 minutes). Expiry is lazy: checked
on `resolve_approval`. Expired approvals cannot be approved.

## 9. At-most-once semantics

The implemented guarantee is **at-most-once execution after approval**:

- one resolution wins the `Pending -> Approved` claim;
- repeated approval/resolution observes terminal state and never re-executes;
- if the process crashes after the claim but before tool invocation, the
  action is lost (not executed); the approval remains in the store in
  `Approved` state. There is no false transactional exactly-once claim.

## 10. Tests

`crates/apeireth-runtime/tests/canonical_approval_lifecycle.rs` proves:

- pending outcome and zero tool invocation before approval
- approve executes once and continues remaining tool calls
- reject skips the tool and the model recovers
- double approve executes once
- concurrent approve executes once
- reject then approve does not execute
- expired approval does not execute
- pending approval survives runtime rebuild over the same store
- new turn while pending is blocked

`crates/apeireth-runtime/tests/canonical_architecture_invariants.rs` proves no
parallel approval engine exists in canonical runtime source.
