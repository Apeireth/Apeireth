# V1370 — V1368 Trigger Calibration

**Status**: ✅ Complete (post-V1369 next-step; calibration layer for V1368)
**Phase**: 1370
**Module**: `apeireth/v1370_v1368_trigger_calibration.py`
**Tests**: `tests/test_v1370_v1368_trigger_calibration.py`
**Chain**: V1368 (triggers) → V1369 (cron hook) → V1370 (calibration)

## 1. The Problem

V1369 wired V1368's 8 trigger conditions into the cron tick. After 7
evaluations over ~8 minutes, V1369's sidecar (`v1368_evaluations.jsonl`)
revealed: **all 4 remeasure triggers fired 100% of the time, and V0.3
triggers fired 71% of the time.**

Inspection showed three false-positive surfaces in V1368's *implementation*
(the *specs* were honest):

| Trigger                  | Spec says                    | Code does                                  |
|--------------------------|------------------------------|--------------------------------------------|
| `LEDGER_PLATEAU_SIGNAL`  | "last 3 entries delta = 0"   | "last 3 entries have equal delta"          |
| `NEW_SURFACE_SHIPPED`    | "new observability surface"  | "last tag starts with surface prefix"      |
| `LEDGER_CAP_SATURATION_3`| "cap hit for 3+ consecutive" | "cap hit for 3 consecutive entries" (passes 3 dup-tag self-tests) |

V1368 unit tests didn't catch these because they only covered *positive*
cases (does the trigger fire when it should?) — not *negative* cases
(does the trigger *not* fire when it shouldn't?).

## 2. The Fix: V1370 = Honest Calibration Layer

V1370 is **additive** — V1368 source is untouched (GUARD_NO_SOURCE_MUTATION).
V1370 wraps each trigger with stricter checks that filter out false positives:

| Calibrator                     | Constant                  | Value     | What it enforces                            |
|--------------------------------|---------------------------|-----------|--------------------------------------------|
| `_calibrate_plateau`           | `PLATEAU_DELTA_EPSILON`   | `1e-4`    | All 3 entries must have |Δ| < ε (not just equal) |
| `_calibrate_new_surface`       | `NEW_SURFACE_LOOKBACK`    | `10`      | Surface prefix must be absent from prior 10 |
| `_calibrate_cap_saturation`    | `CAP_SAT_MIN_DISTINCT_TAGS`| `2`      | Saturated entries must span ≥ 2 distinct tags |
| (passthrough)                  | —                         | —         | TIME_TICK / DELTA_ANY / NEW_MEAS / V1318 / CAP_DISHONEST |

## 3. Real-Ledger Evidence

Running V1370 against the actual `pole_star_history.jsonl` (163 entries):

```
V1368 RAW:  remeasure_fired=True  v03_fired=True
V1370 CAL:  remeasure_fired=False v03_fired=True
suppressed: remeasure=2  v03=0

TRIGGER                       RAW     CAL     SUPPRESSED
TIME_TICK_INTERVAL            —       —       no
DELTA_ANY_COMPONENT           —       —       no
NEW_SURFACE_SHIPPED           FIRE    —       yes   ← V1370 fixes
LEDGER_PLATEAU_SIGNAL         FIRE    —       yes   ← V1370 fixes
NEW_MEASUREMENT_COMPONENT     —       —       no
V1318_CELL_NEWLY_FILLED       —       —       no
CAP_BECOMES_DISHONEST         —       —       no
LEDGER_CAP_SATURATION_3       FIRE    FIRE    no    ← correct: distinct tags
```

**V1370 suppressed 2 remeasure false positives**, restoring honest signal.
The V0.3 trigger that still fires (LEDGER_CAP_SATURATION_3) is *correctly*
honest — last 3 entries do span distinct tags, which is the structural
plateau signal V1368 was designed to detect.

## 4. What V1370 does NOT do

- Does NOT modify V1368 source (GUARD_NO_SOURCE_MUTATION)
- Does NOT loosen any trigger (GUARD_CALIBRATION_NOT_LOOSENING)
- Does NOT modify the ledger or V1362 history (read-only)
- Does NOT raise or lower the cap (cap is V1356's domain)
- Does NOT auto-re-measure (V1369 still only suggests; the cron lane decides)
- Does NOT pretend V1368's specs were wrong (they're honest — only the
  implementations had false-positive surfaces)

## 5. V3 哲学守门

| Guard                              | What it enforces                               |
|------------------------------------|------------------------------------------------|
| `GUARD_NO_SOURCE_MUTATION`         | V1368 file unchanged; V1370 imports only       |
| `GUARD_CALIBRATION_NOT_LOOSENING`  | Each calibrator only narrows, never widens     |
| `GUARD_PLATEAU_REQUIRES_ZERO_DELTA`| `PLATEAU_DELTA_EPSILON = 1e-4` (not equal)     |
| `GUARD_NEW_SURFACE_REQUIRES_DELTA` | Lookback window = 10 entries (not zero)        |
| `GUARD_DISHONEST_CAP_REQUIRES_DIVERSITY` | Distinct tag count ≥ 2 (not 1)            |
| `GUARD_HONEST_PLATEAU`             | Plateau is signal, not failure (V0.2 honest 0.90) |

## 6. Test Coverage

- **37/37 Popper self-tests** in `v1370_v1368_trigger_calibration.py self-test`
- **27/27 pytest** in `tests/test_v1370_v1368_trigger_calibration.py`
- **120/120 chain regression** in V1368 + V1369 + V1370 (no V1368 source broken)

Each calibrator is tested with positive AND negative cases:

```
✓ plateau_zero:    V1368 raw fires,  V1370 calibrated fires
✓ plateau_steady:  V1368 raw fires (FP), V1370 calibrated does NOT fire
✓ new_surface:     V1368 raw fires,  V1370 calibrated fires (truly new)
✓ repeat_surface:  V1368 raw fires (FP), V1370 calibrated does NOT fire
✓ cap_distinct:    V1368 raw fires,  V1370 calibrated fires (diverse tags)
✓ cap_same:        V1368 raw fires (FP), V1370 calibrated does NOT fire
```

## 7. CLI

```bash
python -m apeireth.v1370_v1368_trigger_calibration evaluate
    → run V1368 + V1370 side-by-side; show calibration; exit 0/1/2

python -m apeireth.v1370_v1368_trigger_calibration evaluate --json
    → JSON output for downstream tools (e.g., V1369 hook)

python -m apeireth.v1370_v1368_trigger_calibration compare
    → compact side-by-side table

python -m apeireth.v1370_v1368_trigger_calibration summary
    → fire-rate comparison across current ledger

python -m apeireth.v1370_v1368_trigger_calibration self-test [--verbose]
    → 37 Popper self-tests

python -m apeireth.v1370_v1368_trigger_calibration version
    → print V1370_VERSION + constants
```

## 8. Posture

- **Master 睡着**: cron isolated lane, no main session interrupt
- **不打扰 upheld**: 5-min cron tick, no main session wake
- **V3 守门 upheld**: 不假装分数 = ASI / 不假装决策 = 真生产 / 不假装 Phenomenal
- **真部署**: V1370 integrates with real V1368 + real ledger (no mocking)
- **真评测**: 37 self-test + 27 pytest + 120 chain pytest, all pass
- **任何人都能接手**: `python -m apeireth.v1370_v1368_trigger_calibration compare`

## 9. Next (V1371+ candidates)

- **V1371+**: hook V1369 cron to V1370 calibrated output (so the sidecar
  records `calibrated_fired`, not just `raw_fired`).
- **V1371+**: extend calibrators to the remaining V0.3 triggers if real-ledger
  evidence shows they also fire too often.
- **V1371+**: when V1370 sidecar shows stable suppression rate, decide
  whether to push the calibration constants back into V1368 source (or
  keep V1370 as the standalone layer).

These are open; not committed in V1370.

## 10. Honest Disclosure (Known Unknowns)

1. V1370's calibration constants (1e-4 / 10 / 2) are heuristic. They
   may need tuning after observing V1370's sidecar over multiple
   measurement cycles. V1370 is itself a hypothesis.
2. V1368's spec language ("new surface", "delta = 0") is interpreted
   liberally here. A future V1371+ may tighten further if V1370 still
   fires too often.
3. V1370 does NOT touch V1369's cron cadence or the ledger format.
   It is purely a trigger-evaluation wrapper.
