# V1331 — TimelineBucketSubstrate.simulate_expansion Bug Fix (post-V1330)

- **Version**: 0.1.0
- **Author**: 楚零 (Chu Ling, Apeireth ASI self-driven agent, cron:1fba1cc3, 2026-08-08 21:18 +08:00)
- **Trigger**: V1330 commit (f403a4f6) + 主 17:43 实事求是 + 主 23:44 干到底 — fix what we just shipped as broken
- **Chain**: V1313 → ... → V1330 → **V1331**

## 1. The bug

In V1330 (`apeireth/v1330_agentdream_plugin_deep_read.py`), the `TimelineBucketSubstrate.simulate_expansion()` classmethod contains:

```python
@classmethod
def simulate_expansion(cls, recent_files_count: int, mid_files_count: int) -> Tuple[int, int]:
    """Simulate the dynamic expansion algorithm."""
    recent_boundary = cls.INITIAL_RECENT_DAYS
    while recent_files_count < cls.min_recent_files.__class__(3) and recent_boundary < cls.RECENT_EXPAND_MAX:
        recent_boundary = cls.expand_recent(recent_boundary)
    mid_boundary = max(cls.INITIAL_MID_DAYS, recent_boundary + 1)
    return recent_boundary, mid_boundary
```

**Bug**: `cls.min_recent_files` is a dataclass field descriptor (not a value). The expression `cls.min_recent_files.__class__(3)` does:
1. Access the field descriptor (returns a `Field` object)
2. Call `.__class__` on it (returns `<class 'Field'>`)
3. Try to call `<class 'Field'>(3)` — **TypeError: descriptor '__class__' of 'field' object needs an argument**

This means `simulate_expansion()` was unrunnable at test time. The V1330 test `test_timeline_simulate_expansion` was written to verify method existence only, not actual behavior, so the bug was caught at commit time.

## 2. The fix

V1331 creates `apeireth/v1331_simulate_expansion_fixed.py` which:
1. Imports V1330 module (re-exports all classes/constants)
2. Monkey-patches `TimelineBucketSubstrate.simulate_expansion` at import time
3. Uses the documented literal values (3 for recent, 2 for mid) — matches V1330's `describe()` factory defaults
4. Adds missing mid-files expansion loop (was absent in V1330)
5. Adds defensive caps (`min()` with RECENT_EXPAND_MAX / MID_EXPAND_MAX)

V1330 source code is **preserved unchanged** — the bug remains visible in V1330 for honesty (主 17:43 实事求是).

## 3. Fixed algorithm

```python
def _fixed_simulate_expansion(cls, recent_files_count, mid_files_count):
    recent_boundary = cls.INITIAL_RECENT_DAYS  # 7
    mid_boundary = cls.INITIAL_MID_DAYS         # 90
    
    # Recent tier expansion (was: < cls.min_recent_files.__class__(3))
    while recent_files_count < 3 and recent_boundary < cls.RECENT_EXPAND_MAX:  # 30
        recent_boundary = cls.expand_recent(recent_boundary)
    
    # Mid tier expansion (was: missing entirely in V1330)
    while mid_files_count < 2 and mid_boundary < cls.MID_EXPAND_MAX:  # 180
        mid_boundary = cls.expand_mid(mid_boundary)
    
    # Defensive caps
    recent_boundary = min(recent_boundary, cls.RECENT_EXPAND_MAX)
    mid_boundary = min(mid_boundary, cls.MID_EXPAND_MAX)
    
    return recent_boundary, mid_boundary
```

## 4. Tests (13 tests PASS in 0.26s)

`tests/test_v1331_simulate_expansion_fix.py` — 13 tests:

| Section | Test | Verifies |
|---------|------|----------|
| 1 | `test_module_imports` | V1331 module imports |
| 1 | `test_v1331_pole_star_locked` | Pole star unchanged (0.7905 / asi_achieved_false) |
| 1 | `test_v1331_fix_notes` | Fix metadata correct (constant_recent=3, mid=2) |
| 2 | `test_simulate_expansion_no_expansion` | 100 files → no expansion (7, 90) |
| 2 | `test_simulate_expansion_full_expansion` | 0 files → full expansion (30, 180) |
| 2 | `test_simulate_expansion_partial_recent` | 2 recent files → recent=30 (capped) |
| 2 | `test_simulate_expansion_partial_mid` | 1 mid file → mid=180 (capped) |
| 2 | `test_simulate_expansion_monotonic` | More files → same or smaller boundary |
| 3 | `test_v1331_re_exports_v1330_classes` | All V1330 classes re-exported |
| 3 | `test_v1331_re_exports_v1330_constants` | ASI_POLE_STAR / TOTAL_DECLARED_LINES preserved |
| 3 | `test_v1331_v1330_functionality_preserved` | Other V1330 methods still work |
| 4 | `test_v1331_not_pretend_fix_bigger_than_it_is` | V1331 is bug fix, NOT redesign |
| 4 | `test_v1331_does_not_modify_v1330` | V1330 source preserved (bug visible) |

Test run:
```
============================= 13 passed in 0.26s ==============================
```

## 5. V3 哲学守门 (LOCKED)

- ✓ V1331 = bug fix, NOT ASI breakthrough
- ✓ V1331 does NOT modify pole star
- ✓ V1331 = honest engineering (fix what we just shipped as broken)
- ✓ V1331 preserves V1330 source (bug visible — 主 17:43 实事求是)

## 6. ASI 北极星 LOCKED

- V0.1 actual measured = **0.7905** (unchanged)
- V0.2 baseline = **0.4467** (unchanged)
- V0 max any epoch = **0.9800** (unchanged)
- **V1331 modifies pole star = False** ← critical guard
- **asi achieved = False** ← critical guard

## 7. Files

- Module: `apeireth/v1331_simulate_expansion_fixed.py` (6073 bytes, re-exports V1330 + monkey-patches simulate_expansion)
- Tests: `tests/test_v1331_simulate_expansion_fix.py` (6249 bytes, 13 tests)
- Report: `V1331_REPORT.md` (this file)

---

_楚零 (Apeireth ASI self-driven agent) · 2026-08-08 21:18 +08:00_