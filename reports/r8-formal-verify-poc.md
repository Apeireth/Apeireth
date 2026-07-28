# V1099 — Formal Verify PoC (TLA+ Bounded Model Checker)

- module: `v1099_formal_verify_basic`
- version: `0.1.0-poc`
- states_explored: **14**
- transitions: **14**
- max_depth: **15**
- duration_ms: **2.05**
- scenario: `{"diff_lines": 250, "touches_protected": true, "hqb_delta": 0.8, "hqb_measured": true, "human_approved": true, "inject_touch_production": true}`

## Safety Invariants Checked (5)

| # | Invariant | Result |
|---|-----------|--------|
| _inv1_process_before_sandbox | ✅ PASS |
| _inv2_protected_paths_require_human | ✅ PASS |
| _inv3_revert_records_taxonomy | ❌ VIOLATED (2) |
| _inv4_hqb_must_be_measured | ✅ PASS |
| _inv5_no_production_module_mutation | ✅ PASS |

## Liveness Properties Checked (3)

| # | Property | Result |
|---|----------|--------|
| liveness_proposal_decided | ✅ PASS |
| liveness_no_infinite_review | ✅ PASS |
| liveness_revert_eventually_retryable | ✅ PASS |

## Counterexamples (first 3)

### Counterexample 1: _inv3_revert_records_taxonomy
  - violation: INV3 violated: REVERT without taxonomy_recorded (prev=PROCESS_GATE)
  - trace: `IDLE -> PROPOSED -> PROCESS_GATE -> REVERT`

### Counterexample 2: _inv3_revert_records_taxonomy
  - violation: INV3 violated: REVERT without taxonomy_recorded (prev=EVAL_GATE)
  - trace: `IDLE -> PROPOSED -> PROCESS_GATE -> REVERT -> STUCK -> STUCK -> STUCK -> IDLE -> PROPOSED -> PROCESS_GATE -> SANDBOX_GATE -> EVAL_GATE -> REVERT`

## State Graph (sample)

```
{
  "IDLE": [
    "PROPOSED"
  ],
  "PROPOSED": [
    "PROCESS_GATE"
  ],
  "PROCESS_GATE": [
    "REVERT",
    "SANDBOX_GATE"
  ],
  "REVERT": [
    "STUCK"
  ],
  "STUCK": [
    "IDLE",
    "STUCK"
  ],
  "SANDBOX_GATE": [
    "EVAL_GATE"
  ],
  "EVAL_GATE": [
    "REVERT"
  ]
}
```

## V3 Philosophy Guard

  - guard: `PASS`
  - notes:
    - `not_tla_is_proof`: Bounded TLA+ model checking is exhaustive BFS, not Coq-style proof. It refutes spec violations; it does not prove the absence of all bugs.
    - `not_checker_is_truth`: A bounded checker only sees up to N states. No violation found <= N does NOT entail no violation exists at all.
    - `not_invariant_is_axiom`: Invariants are spec claims authored by humans, not metaphysical truths. If the invariant is wrong, the checker enforces the wrong spec.
    - `not_export_is_verified`: Exporting a .tla file is a syntactic translation. The exported spec has not been verified by TLC unless a human runs TLC on it.

V1099 = BFS TLA+ 风格 PoC, 不是 Coq 证明, 不是 TLC 真跑 (需 Java).
导出 .tla 需用 TLC 真验证 (人类操作).
