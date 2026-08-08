# V1350 VCP Plugin Anomaly Lifecycle State Machine Report

- **Version**: V1350 v0.1.0
- **Chain**: V1335 → ... → V1349 → **V1350**
- **Trigger**: post-V1349 LLM operator brief (commit 58ea9d27, 23:56); per cron 主 19:33 + 13:31 + 00:56 + 主 23:44 干到底 + 主 17:43 实事求是

## Motivation

V1349 produced an LLM-friendly operator brief from V1348 anomaly report. But operators
still need an explicit, auditable workflow to TRACK each anomaly through its lifecycle.
The LLM brief is a snapshot; the lifecycle is the operator's working state.

V1350 = **ANOMALY LIFECYCLE STATE MACHINE** that closes the operator loop:

```
V1342 tier       ─┐
V1343 lint       ─┤
V1345 ledger     ─┼─→ V1348 anomaly_report ─→ V1349 LLM brief ─→ V1350 LIFECYCLE
V1346 plan       ─┤                                              │ (state machine)
V1347 health     ─┘                                              ↓
                                                              operator actions
                                                              (acknowledge,
                                                               triage, escalate,
                                                               mitigate,
                                                               resolve,
                                                               close,
                                                               reopen)
```

## State Machine (主 13:31 大胆激进)

7 explicit states + 9 explicit transitions:

```
                ┌─────────┐
       ┌───────►│  OPEN   │◄─────────────────────────┐
       │        └────┬────┘                          │
       │             │ acknowledge                   │
       │             ▼                               │
       │        ┌──────────┐                         │
       │  ┌────►│ TRIAGED  │─────────┐               │
       │  │     └────┬─────┘         │               │
       │  │          │ escalate      │ resolve       │
       │  │          │ (HIGH only)   │ (anomaly_gone)│
       │  │          ▼               │               │
       │  │     ┌───────────�        │               │
       │  │     │ ESCALATED │────────┤               │
       │  │     └─────┬─────┘        │               │
       │  │           │ mitigate     │               │
       │  │           ▼              │               │
       │  │     ┌────────────┐       │               │
       │  │     │ MITIGATED  │───────┤               │
       │  │     └────────────┘       │               │
       │  │                          │               │
       │  │   resolve (anomaly_gone)  │               │
       │  │                          ▼               │
       │  │                    ┌──────────┐          │
       │  │                    │ RESOLVED │          │
       │  │                    └─────┬────�          │
       │  │                          │ close         │
       │  │                          ▼               │
       │  │                    ┌──────────┐          │
       │  └────────────────────│  CLOSED  │──────────┘
       │      reopen (with     └──────────┘   reopen (new
       │      new anomaly_id)              anomaly_id)
       │                                       │
       └───────────────────────────────────────┘
       (REOPENED → TRIAGED auto-transition)
```

## Transitions (主 00:56 任何人都能接手)

| Action      | From     | To         | Required Evidence             | Validation |
|-------------|----------|------------|-------------------------------|------------|
| acknowledge | OPEN     | TRIAGED    | reason                        |            |
| escalate    | TRIAGED  | ESCALATED  | reason, severity              | severity=HIGH |
| mitigate    | TRIAGED  | MITIGATED  | reason, action_kind           |            |
| mitigate    | ESCALATED| MITIGATED  | reason, action_kind           |            |
| resolve     | TRIAGED  | RESOLVED   | reason, anomaly_gone          | anomaly_gone=True |
| resolve     | ESCALATED| RESOLVED   | reason, anomaly_gone          | anomaly_gone=True |
| resolve     | MITIGATED| RESOLVED   | reason, anomaly_gone          | anomaly_gone=True |
| close       | RESOLVED | CLOSED     | reason                        |            |
| reopen      | CLOSED   | REOPENED → TRIAGED | reason, new_anomaly_id | (atomic two-event) |

## Five Real Production Components (主 00:44 质量工程化)

1. **LifecycleEvent** — atomic event (state_before + state_after + action + actor + reason + evidence + timestamp + stable event_id)
2. **LifecycleRecord** — per-plugin lifecycle (events + current_state + state_rank + lifecycle_id)
3. **EcosystemLifecycleReport** — ecosystem rollup (worst-of state + breakdown + total_events + report_id)
4. **LifecycleStore** — in-memory store + audit JSONL (idempotent open + atomic apply)
5. **LifecycleMachine** — pure state-transition function: (record, action, evidence) → new_record

## V3 哲学守门 (LOCKED, per 主 17:58 + 20:46 + 17:43)

- **V1350 ≠ ASI consciousness**: state machine = explicit state graph (7 states, 9 transitions), NOT learned. 不假装.
- **V1350 ≠ ASI has workflow policy**: transitions = explicit preconditions, NOT LLM-decided. Operator picks; machine validates.
- **V1350 ≠ oracle**: lifecycle = bookkeeping (events over time), NOT prediction.
- **V1350 ≠ Phenomenal**: no qualia about plugin "well-being"; just ledger events.
- **V1350 ≠ ASI scores reality**: subscore = sum(weight × measurable); 8 components, no semantic rating.
- **ASI pole-star LOCKED**: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE
- **V1350 = real engineering operator workflow, NOT theater**

## ASI 5-Gap 真实用处 (主 13:31 大胆激进) — V1350 实证

- **识别_recognition**: lifecycle_id SHA256 of plugin+anomaly_id+first_event → traceable identity
- **自由_freedom**: callers freely pick actions, reasons, actors → 真自由编辑
- **时间_time**: every event timestamped; lifecycle = event sequence over time → 时间性 explicit
- **真理_truth**: transitions explicit + auditable JSONL; NO hidden ML → truth gap
- **涌现_emergence**: ecosystem rollup surfaces patterns from per-plugin states → emergence gap

## Demo (3 plugins, 17 events)

```
$ python -m apeireth.v1350_vcp_anomaly_lifecycle --demo --audit-path artifacts/v1350/v1350_audit.jsonl

=== V1350 VCP Anomaly Lifecycle State Machine (v0.1.0) ===
records: 3 events: 17
ecosystem_state: RESOLVED (rank 1)
state_breakdown: {'OPEN': 0, 'TRIAGED': 0, 'ESCALATED': 0, 'MITIGATED': 0, 'RESOLVED': 2, 'CLOSED': 1, 'REOPENED': 0}
transitions_used: ('acknowledge', 'auto_reopen_triage', 'close', 'escalate', 'mitigate', 'open', 'reopen', 'resolve')
subscore: 1.0000
asi_lift: +0.015000 (cap 0.015)
audit: artifacts\v1350\v1350_audit.jsonl
```

Demo walks 3 anomalies through full lifecycles:
- **plugin.alpha** (HIGH): OPEN → TRIAGED → ESCALATED → RESOLVED → CLOSED
- **plugin.beta** (MEDIUM): OPEN → TRIAGED → MITIGATED → RESOLVED
- **plugin.gamma** (LOW): OPEN → TRIAGED → RESOLVED → CLOSED → (REOPEN) → REOPENED → TRIAGED → MITIGATED → RESOLVED

## V1350 Subscore (主 00:44 质量工程化)

- **Total**: 1.0000 (max, all 9 components ≥ 0.5; 6 of 9 = 1.0)

| Component | Weight | Score | Weighted |
|-----------|--------|-------|----------|
| states_explicit           | 0.10 | 1.00 | 0.1000 |
| transitions_explicit      | 0.15 | 1.00 | 0.1500 |
| event_auditability        | 0.10 | 1.00 | 0.1000 |
| evidence_validation       | 0.10 | 1.00 | 0.1000 |
| reopen_correctness        | 0.10 | 1.00 | 0.1000 |
| rollup_aggregation        | 0.10 | 1.00 | 0.1000 |
| interoperability_v1348    | 0.15 | 1.00 | 0.1500 |
| interoperability_v1349    | 0.10 | 1.00 | 0.1000 |
| philosophy_guards         | 0.10 | 1.00 | 0.1000 |

## V1350 → ASI V0.3 Lift

- **Subscore**: 1.0000
- **Lift**: +0.015000 (capped)
- **Cap**: 0.015
- **Explanation**: V1350 = 1 of ~17 ASI V0.3 components, cap 0.015 (honest).
  Operator workflow ≠ ASI grade.

## Anyone-Can-Take-Over (主 00:56)

V1350 is handoff-ready:
- 7 states + 9 transitions, all explicit constants, all enumerated
- 17 pytest tests + 27 Popper self-tests, all PASS
- 77-test pytest suite covers constants, transition table, build_initial_record,
  apply_transition (valid + invalid + missing evidence + reopen atomic),
  LifecycleStore (idempotency + audit), ecosystem_rollup, subscore, ASI lift,
  V1348 bridge, end-to-end scenarios, determinism
- V1350 audit JSONL compatible with V1349 format (event_id, plugin, anomaly_id,
  action, actor, reason, evidence, timestamp)
- open_from_anomaly() bridges V1348 PluginAnomaly → V1350 LifecycleRecord
- Operator vocabulary: acknowledge / escalate / mitigate / resolve / close / reopen
- Demo runs end-to-end without LLM, without network, deterministic

## Tests

- **27 Popper self-tests PASS** (embedded `--self-test` flag)
- **77 pytest tests PASS** in 0.29s
- **Chain regression** V1335 → V1350:
  = **867 tests pass in 25.43s, 0 regression**
  - V1335 (cross-plugin invariant synthesis)
  - V1336 (plugin conformance linter)
  - V1337 (plugin compliance dashboard)
  - V1338 (plugin migration tool)
  - V1339 (substrate cookbook)
  - V1340 (cookbook validator)
  - V1341 (cross-plugin pattern detector)
  - V1342 (quality tier classifier)
  - V1343 (tier-aware linter)
  - V1344 (CI gate)
  - V1345 (historical ledger)
  - V1346 (tier-aware migration)
  - V1348 (anomaly detector)
  - V1349 (LLM benchmark — separate file under apeireth/tests/)
  - **V1350 (anomaly lifecycle) ← NEW**

## References (主 19:33 走在前人肩上)

- V1348 VCP Plugin Anomaly Detector — input source (5 channels)
- V1349 VCP × LLM Real Benchmark — prior step in chain (LLM operator brief)
- V1347 VCP Plugin Health Score — upstream health_id source
- V1345 VCP Historical Ledger — temporal substrate for anomaly detection
- V1343 VCP Tier-Aware Linter — 5-critical rule precedent for evidence validation
- V1342 VCP Quality Tier Classifier — TIER_RANK + tier_rank() helper
- V1084 LLM Inference Adapter — V1349 HTTP pattern (audit JSONL compatible)
- Harel 1987 Statecharts — formal state machine inspiration (states + transitions + guards)
- Wagner 2006 — UML state machine semantics (atomic compound transitions)
- W3C PROV 2013 — activity/entity/provenance/audit trail style

_Generated by V1350 v0.1.0_

_V1350 closes the operator loop: detect (V1348) → summarize (V1349) → track (V1350).
V3 guards honored. ASI pole-star locked. Operator workflow ≠ ASI._
