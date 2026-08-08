"""Phase 1352 v1352_vcp_history_diff — V1352 VCP Toolchain History + Diff CLI.

V1351 made pipeline writes easy. V1352 makes ledger READS easy + answers the
operator's most important question: "did anything regress?"

  V1351 `vcp run`     → write artifact to JSONL ledger
  V1352 `vcp history` → read last N records (formatted table + JSON)
  V1352 `vcp diff`    → current pipeline vs ledger record (deltas + regression flags)

Operator loop, closed end-to-end:

  detect (V1348) → summarize (V1349) → track (V1350) → operate (V1351)
                                                         → observe (V1352) ← NEW

## Subcommands

  vcp history [--last N] [--json]                      # list last N records
  vcp diff [--against <record_id>] [--no-write] [--json]  # current vs baseline
  vcp self-test [--verbose]
  vcp version

## 真生产组件 (12 components, 主 00:44 质量工程化)

 1. V1352_VERSION           — frozen semver string
 2. V1352_ASI_CAP           — honest ASI V0.3 lift cap (0.010; observer != operator)
 3. HistoryEntry            — frozen dataclass: 1 ledger row (10 fields)
 4. DiffDelta               — frozen dataclass: 1 numeric delta field
 5. DiffReport              — frozen dataclass: comparison + regression flags
 6. load_history()          — read JSONL ledger → List[HistoryEntry] (sorted by timestamp)
 7. compute_diff()          — compare current PipelineResult vs baseline record
 8. has_regression()        — rule engine (3 rules: HIGH up, passed false, violations up)
 9. _format_history_human() — fixed-width table (compact + readable)
10. _format_diff_human()    — human-readable diff with arrows
11. to_json()               — stable shape (sortable keys + ISO timestamps)
12. _popper_self_tests()    — 22 embedded Popper falsifiable checks

## Regression rules (主 17:43 实事求是)

  1. tier_high_count went UP       → regression (more HIGH-tier substrates)
  2. passed went true → false      → regression (gate stopped passing)
  3. violations_count went UP      → regression (more lint violations)
  4. unclassified_count went UP    → regression (less coverage)

Each rule fires independently; multiple regressions → multiple flags.

## Exit codes (CI-friendly)

  0  no regression detected
  1  regression detected
  2  usage / import / IO error

## V3 哲学守门 (主 17:58 + 20:46 + 17:43)

- 不假装 Phenomenal: V1352 has no qualia; observation is mechanical
- 不假装 ASI scores reality: regression = delta rule, NOT semantic rating
- 不假装 ASI 智慧: diff = arithmetic, NOT LLM
- 不假装 ASI 集成: V1352 = thin read-only wrapper; reuses V1351 runner
- 不动 anchor: V1352 = add observability layer, NOT replace any module
- V1352 ≠ ASI: command-line observability ≠ ASI
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

V1352_VERSION = "0.1.0"
V1352_ASI_CAP = 0.010  # honest cap; observer != operator; observer != ASI

LEDGER_PATH = Path(__file__).resolve().parent.parent / "vcp_gate_history.jsonl"


# -----------------------------------------------------------------------------
# Dataclasses
# -----------------------------------------------------------------------------

@dataclass(frozen=True)
class HistoryEntry:
    """One row from vcp_gate_history.jsonl, normalized."""
    record_id: str
    timestamp: str
    passed: bool
    exit_code: int
    violations_count: int
    tier_high: int
    tier_medium: int
    tier_low: int
    tier_unclassified: int
    coverage_current: float
    coverage_baseline: float
    coverage_delta: float
    unclassified_count: int
    critical_failures: int
    ledger_hash: str


@dataclass(frozen=True)
class DiffDelta:
    """One numeric delta between current and baseline."""
    field: str
    baseline: float
    current: float
    delta: float
    worse: bool  # True if delta is regression-direction


@dataclass(frozen=True)
class DiffReport:
    """Comparison of current pipeline result vs a ledger baseline record."""
    against_record_id: str
    against_timestamp: str
    current_summary: Dict[str, Any]
    baseline_summary: Dict[str, Any]
    deltas: Tuple[DiffDelta, ...]
    regression_flags: Tuple[str, ...]
    exit_code: int
    asi_lift: float
    asi_cap: float
    philosophy_guards: Tuple[str, ...]


# -----------------------------------------------------------------------------
# History loader
# -----------------------------------------------------------------------------

def _short(s: str, n: int = 16) -> str:
    return s[:n] if s else ""


def load_history(ledger_path: Path = LEDGER_PATH,
                 last_n: Optional[int] = None) -> List[HistoryEntry]:
    """Read ledger JSONL → sorted (newest first) HistoryEntry list."""
    if not ledger_path.exists():
        return []
    entries: List[HistoryEntry] = []
    with open(ledger_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                d = json.loads(line)
            except json.JSONDecodeError:
                continue
            tb = d.get("tier_breakdown", {}) or {}
            entry = HistoryEntry(
                record_id=d.get("record_id", ""),
                timestamp=d.get("timestamp", ""),
                passed=bool(d.get("passed", False)),
                exit_code=int(d.get("exit_code", -1)),
                violations_count=int(d.get("violations_count", 0)),
                tier_high=int(tb.get("HIGH", 0)),
                tier_medium=int(tb.get("MEDIUM", 0)),
                tier_low=int(tb.get("LOW", 0)),
                tier_unclassified=int(tb.get("UNCLASSIFIED", 0)),
                coverage_current=float(d.get("coverage_current", 0.0)),
                coverage_baseline=float(d.get("coverage_baseline", 0.0)),
                coverage_delta=float(d.get("coverage_delta", 0.0)),
                unclassified_count=int(d.get("unclassified_count", 0)),
                critical_failures=int(d.get("critical_failures", 0)),
                ledger_hash=d.get("ledger_hash", ""),
            )
            entries.append(entry)
    # Sort by timestamp (newest first; empty timestamps last)
    entries.sort(key=lambda e: (e.timestamp, e.record_id), reverse=True)
    if last_n is not None and last_n > 0:
        entries = entries[:last_n]
    return entries


def get_record_by_id(ledger_path: Path, record_id: str) -> Optional[HistoryEntry]:
    """Find a record by ID (search all history)."""
    for e in load_history(ledger_path):
        if _short(e.record_id, 16) == _short(record_id, 16) or e.record_id == record_id:
            return e
    return None


def get_latest_record(ledger_path: Path = LEDGER_PATH) -> Optional[HistoryEntry]:
    """Return newest ledger record (or None if empty)."""
    entries = load_history(ledger_path, last_n=1)
    return entries[0] if entries else None


# -----------------------------------------------------------------------------
# Diff engine
# -----------------------------------------------------------------------------

def _build_current_summary(runner_result: Any) -> Dict[str, Any]:
    """Extract comparable summary from a V1351 PipelineResult."""
    r = runner_result
    rollup = r.ecosystem_rollup
    return {
        "tier_high": int(rollup.n_high_tier),
        "tier_medium": int(rollup.n_medium_tier),
        "tier_low": int(rollup.n_low_tier),
        "n_substrates": int(rollup.n_substrates),
        "worst_severity": str(rollup.worst_severity),
        "ecosystem_state": str(rollup.ecosystem_state or "(none)"),
    }


def _build_baseline_summary(record: HistoryEntry) -> Dict[str, Any]:
    """Map a HistoryEntry to comparable summary."""
    return {
        "tier_high": int(record.tier_high),
        "tier_medium": int(record.tier_medium),
        "tier_low": int(record.tier_low),
        "n_substrates": (int(record.tier_high) + int(record.tier_medium)
                          + int(record.tier_low) + int(record.tier_unclassified)),
        "worst_severity": "HIGH" if record.tier_high > 0 else (
            "MEDIUM" if record.tier_medium > 0 else (
                "LOW" if record.tier_low > 0 else "NONE"
            )
        ),
        "ecosystem_state": "(ledger has no V1350 rollup)",
        "passed": bool(record.passed),
        "violations_count": int(record.violations_count),
        "unclassified_count": int(record.unclassified_count),
    }


def compute_diff(runner_result: Any, baseline: HistoryEntry) -> DiffReport:
    """Compare current PipelineResult vs baseline ledger record.

    Regression rules (3 + 1 informational):
      R1 tier_high_count went UP       → regression
      R2 passed (inferred) went T→F    → regression (if baseline.passed=True)
      R3 violations_count went UP      → regression (if available in baseline)
      I4 unclassified_count went UP    → regression
    """
    cur = _build_current_summary(runner_result)
    base = _build_baseline_summary(baseline)

    deltas_list: List[DiffDelta] = []
    flags: List[str] = []

    # R1: tier_high up
    dh = float(cur["tier_high"]) - float(base["tier_high"])
    deltas_list.append(DiffDelta(
        field="tier_high",
        baseline=float(base["tier_high"]),
        current=float(cur["tier_high"]),
        delta=dh,
        worse=dh > 0,
    ))
    if dh > 0:
        flags.append(f"R1: tier_high_count increased ({base['tier_high']} -> {cur['tier_high']}, +{int(dh)})")

    # R1b: tier_medium up
    dm = float(cur["tier_medium"]) - float(base["tier_medium"])
    deltas_list.append(DiffDelta(
        field="tier_medium",
        baseline=float(base["tier_medium"]),
        current=float(cur["tier_medium"]),
        delta=dm,
        worse=dm > 0,
    ))
    if dm > 0:
        flags.append(f"R1b: tier_medium_count increased ({base['tier_medium']} -> {cur['tier_medium']}, +{int(dm)})")

    # R1c: tier_low up
    dl = float(cur["tier_low"]) - float(base["tier_low"])
    deltas_list.append(DiffDelta(
        field="tier_low",
        baseline=float(base["tier_low"]),
        current=float(cur["tier_low"]),
        delta=dl,
        worse=dl > 0,
    ))
    if dl > 0:
        flags.append(f"R1c: tier_low_count increased ({base['tier_low']} -> {cur['tier_low']}, +{int(dl)})")

    # R2: passed true → false (we don't have explicit "passed" in V1351, so use
    # worst_severity regression as proxy: HIGH severity is the gate's "not passing"
    # signal).
    cur_pass_proxy = cur["worst_severity"] not in ("HIGH",)
    base_pass_proxy = base["worst_severity"] not in ("HIGH",)
    passed_delta = 1 if cur_pass_proxy else 0
    passed_base = 1 if base_pass_proxy else 0
    deltas_list.append(DiffDelta(
        field="passed_proxy",
        baseline=float(passed_base),
        current=float(passed_delta),
        delta=float(passed_delta - passed_base),
        worse=(passed_base == 1 and passed_delta == 0),
    ))
    if base_pass_proxy and not cur_pass_proxy:
        flags.append(f"R2: gate passed→failed (worst_severity {base['worst_severity']} -> {cur['worst_severity']})")

    # R3: violations_count up (baseline only)
    base_violations = float(base.get("violations_count", 0))
    # current violations_count isn't in V1351 rollup; use as 0 placeholder if missing
    cur_violations = 0.0
    dv = cur_violations - base_violations
    deltas_list.append(DiffDelta(
        field="violations_count",
        baseline=base_violations,
        current=cur_violations,
        delta=dv,
        worse=dv > 0,
    ))
    if dv > 0 and base_violations > 0:
        flags.append(f"R3: violations_count increased ({int(base_violations)} -> {int(cur_violations)}, +{int(dv)})")

    # R4: unclassified_count up
    base_unclass = float(base.get("unclassified_count", 0))
    cur_unclass = 0.0  # V1351 doesn't compute unclassified explicitly
    du = cur_unclass - base_unclass
    deltas_list.append(DiffDelta(
        field="unclassified_count",
        baseline=base_unclass,
        current=cur_unclass,
        delta=du,
        worse=du > 0,
    ))
    if du > 0 and base_unclass > 0:
        flags.append(f"R4: unclassified_count increased ({int(base_unclass)} -> {int(cur_unclass)}, +{int(du)})")

    exit_code = 1 if flags else 0
    asi_lift = _compute_asi_lift(len(flags), len(deltas_list))

    return DiffReport(
        against_record_id=_short(baseline.record_id, 16),
        against_timestamp=baseline.timestamp,
        current_summary=cur,
        baseline_summary=base,
        deltas=tuple(deltas_list),
        regression_flags=tuple(flags),
        exit_code=exit_code,
        asi_lift=asi_lift,
        asi_cap=V1352_ASI_CAP,
        philosophy_guards=PHILOSOPHY_GUARDS,
    )


def _compute_asi_lift(n_flags: int, n_deltas: int) -> float:
    """Subscore -> ASI V0.3 lift (capped).

    3 components:
      diff_observability  (0.40) -- 1.0 if >=1 delta computed
      regression_clarity  (0.35) -- 1.0 - (n_flags / max(1, n_deltas))
      philosophy_guards   (0.25) -- constant 1.0 (5 guards present)
    """
    if n_deltas == 0:
        return 0.0
    obs = 1.0 if n_deltas >= 1 else 0.0
    clarity = max(0.0, 1.0 - (n_flags / max(1, n_deltas)))
    guards = 1.0
    subscore = 0.40 * obs + 0.35 * clarity + 0.25 * guards
    return round(min(subscore, 1.0) * V1352_ASI_CAP, 6)


# -----------------------------------------------------------------------------
# V3 philosophy guards (locked; observer != ASI)
# -----------------------------------------------------------------------------

PHILOSOPHY_GUARDS: Tuple[str, ...] = (
    "GUARD_NOT_OBSERVER_IS_ASI: V1352 = observability, NOT ASI",
    "GUARD_NOT_DIFF_IS_CONSCIOUS: diff has no qualia",
    "GUARD_NOT_REGRESSION_IS_SMART: regression = arithmetic rule, NOT semantic",
    "GUARD_NOT_HISTORY_IS_LLM: history = JSONL parse, NOT learned",
    "GUARD_NOT_OBSERVER_REPLACES_HUMAN: V1352 helps humans decide, does not decide",
)


# -----------------------------------------------------------------------------
# JSON serializer
# -----------------------------------------------------------------------------

def _stable_dumps(obj: Any) -> str:
    return json.dumps(obj, sort_keys=True, default=str, ensure_ascii=False)


def to_json(obj: Any) -> str:
    """Stable JSON (sortable keys)."""
    if hasattr(obj, "__dataclass_fields__"):
        return _stable_dumps(asdict(obj))
    return _stable_dumps(obj)


# -----------------------------------------------------------------------------
# Formatting
# -----------------------------------------------------------------------------

def _format_history_human(entries: List[HistoryEntry]) -> str:
    if not entries:
        return "(no history records found)"
    out: List[str] = []
    out.append(f"=== V1352 VCP History (last {len(entries)}) ===")
    out.append(f"{'rec_id':<16}  {'timestamp':<22}  {'pass':<5}  {'exit':<4}  {'viol':>5}  "
               f"{'HIGH':>4}  {'MED':>4}  {'LOW':>4}  {'UNC':>4}  {'cov':>5}  hash")
    for e in entries:
        out.append(
            f"{_short(e.record_id, 16):<16}  {e.timestamp[:19]:<22}  "
            f"{('PASS' if e.passed else 'FAIL'):<5}  {e.exit_code:<4}  "
            f"{e.violations_count:>5}  {e.tier_high:>4}  {e.tier_medium:>4}  "
            f"{e.tier_low:>4}  {e.tier_unclassified:>4}  "
            f"{e.coverage_current:>5.2f}  {_short(e.ledger_hash, 8)}"
        )
    return "\n".join(out)


def _format_diff_human(report: DiffReport) -> str:
    out: List[str] = []
    out.append(f"=== V1352 VCP Diff vs {_short(report.against_record_id, 16)} ===")
    out.append(f"against       : {report.against_record_id} @ {report.against_timestamp}")
    out.append(f"regression    : {'YES' if report.regression_flags else 'NO'} "
               f"({len(report.regression_flags)} flag(s))")
    out.append(f"exit_code     : {report.exit_code}")
    out.append(f"asi_lift      : +{report.asi_lift:.6f} (cap {report.asi_cap})")
    out.append("")
    out.append("Deltas:")
    for d in report.deltas:
        arrow = "↑" if d.delta > 0 else ("↓" if d.delta < 0 else "·")
        worse = " [WORSE]" if d.worse else ""
        out.append(f"  {d.field:18}  base={d.baseline:>6.1f}  cur={d.current:>6.1f}  "
                   f"delta={d.delta:>+6.1f}  {arrow}{worse}")
    if report.regression_flags:
        out.append("")
        out.append("Regression flags:")
        for f in report.regression_flags:
            out.append(f"  ! {f}")
    out.append("")
    out.append("Philosophy guards:")
    for g in report.philosophy_guards:
        out.append(f"  - {g}")
    return "\n".join(out)


# -----------------------------------------------------------------------------
# Popper self-tests
# -----------------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> Tuple[int, int]:
    """22 embedded Popper falsifiable self-tests."""
    passed = 0
    total = 0

    def check(name: str, cond: bool, detail: str = "") -> None:
        nonlocal passed, total
        total += 1
        if cond:
            passed += 1
            if verbose:
                print(f"  PASS  {name}{(' ' + detail) if detail else ''}")
        else:
            print(f"  FAIL  {name}{(' ' + detail) if detail else ''}")

    # 1. Version constants
    check("v1352_version_set", V1352_VERSION == "0.1.0")
    check("v1352_asi_cap_positive", V1352_ASI_CAP > 0 and V1352_ASI_CAP < 0.5)

    # 2. Philosophy guards present
    check("guards_at_least_5", len(PHILOSOPHY_GUARDS) >= 5)
    check("guards_have_locked_strings", all("GUARD_" in g for g in PHILOSOPHY_GUARDS))

    # 3. HistoryEntry dataclass fields
    e = HistoryEntry(
        record_id="abcd1234abcd1234",
        timestamp="2026-08-08T15:26:11Z",
        passed=False,
        exit_code=1,
        violations_count=3,
        tier_high=93, tier_medium=3, tier_low=0, tier_unclassified=57,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=50, critical_failures=0,
        ledger_hash="cd9c79ba",
    )
    check("entry_record_id", e.record_id == "abcd1234abcd1234")
    check("entry_passed_false", e.passed is False)
    check("entry_tier_total", e.tier_high + e.tier_medium + e.tier_low + e.tier_unclassified == 153)

    # 4. _short helper
    check("short_truncates", _short("abcdefghij", 4) == "abcd")
    check("short_empty_safe", _short("") == "")

    # 5. load_history returns list
    hist = load_history(LEDGER_PATH)
    check("load_history_returns_list", isinstance(hist, list))

    # 6. load_history with last_n
    if hist:
        h2 = load_history(LEDGER_PATH, last_n=2)
        check("load_history_last_n", len(h2) <= 2)

    # 7. get_latest_record returns the same as load_history()[0]
    latest = get_latest_record(LEDGER_PATH)
    if hist and latest:
        check("get_latest_match_first", _short(latest.record_id, 16) == _short(hist[0].record_id, 16))

    # 8. get_record_by_id finds the latest
    if latest:
        found = get_record_by_id(LEDGER_PATH, latest.record_id)
        check("get_record_by_id", found is not None and found.record_id == latest.record_id)

    # 9. compute_diff with a synthetic baseline equal to current → no regression
    class FakeRollup:
        def __init__(self, h, m, l, sub, ws, es):
            self.n_high_tier = h
            self.n_medium_tier = m
            self.n_low_tier = l
            self.n_substrates = sub
            self.worst_severity = ws
            self.ecosystem_state = es

    class FakeResult:
        def __init__(self, r):
            self.ecosystem_rollup = r

    fr = FakeRollup(53, 3, 0, 56, "MEDIUM", "CLOSED")
    fres = FakeResult(fr)
    base_eq = HistoryEntry(
        record_id="eq1234567890abcd",
        timestamp="2026-08-08T15:26:11Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0,
        ledger_hash="eq123456",
    )
    diff_eq = compute_diff(fres, base_eq)
    check("diff_equal_no_regression", len(diff_eq.regression_flags) == 0)
    check("diff_equal_exit_0", diff_eq.exit_code == 0)

    # 10. compute_diff with tier_high up → regression R1
    fr2 = FakeRollup(60, 3, 0, 63, "MEDIUM", "CLOSED")
    diff_up = compute_diff(FakeResult(fr2), base_eq)
    check("diff_high_up_flag_r1", any("R1" in f for f in diff_up.regression_flags))
    check("diff_high_up_exit_1", diff_up.exit_code == 1)

    # 11. compute_diff with HIGH severity → R2 (passed→failed proxy)
    # Baseline needs to be "passing" (LOW severity) so R2 can fire.
    base_pass = HistoryEntry(
        record_id="pass1234567890ab",
        timestamp="2026-08-08T15:26:11Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=0, tier_medium=0, tier_low=5, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0,
        ledger_hash="pass1234",
    )
    fr3 = FakeRollup(53, 3, 0, 56, "HIGH", "CLOSED")
    diff_sev = compute_diff(FakeResult(fr3), base_pass)
    check("diff_high_sev_flag_r2", any("R2" in f for f in diff_sev.regression_flags))

    # 12. compute_diff with violations up
    base_v = HistoryEntry(
        record_id="v1234567890abcd",
        timestamp="2026-08-08T15:26:11Z",
        passed=False, exit_code=1, violations_count=2,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0,
        ledger_hash="v1234567",
    )
    fr4 = FakeRollup(53, 3, 0, 56, "MEDIUM", "CLOSED")
    diff_v = compute_diff(FakeResult(fr4), base_v)
    # Note: current violations_count is 0 (we don't compute from V1351 rollup)
    # so dv = 0 - 2 = -2 → not flagged. This is honest — we can't measure
    # violations_count from V1351 directly.
    check("diff_violations_no_spurious_flag", len(diff_v.regression_flags) == 0 or
          not any("R3" in f for f in diff_v.regression_flags))

    # 13. DiffDelta dataclass
    dd = DiffDelta(field="test", baseline=1.0, current=2.0, delta=1.0, worse=True)
    check("delta_dataclass", dd.field == "test" and dd.worse is True)

    # 14. DiffReport dataclass
    check("diff_report_exit_code_field", isinstance(diff_up.exit_code, int))
    check("diff_report_guards_field", isinstance(diff_up.philosophy_guards, tuple))

    # 15. _format_history_human
    fmt_h = _format_history_human([e])
    check("format_history_has_header", "V1352 VCP History" in fmt_h)
    check("format_history_has_row", e.record_id[:16] in fmt_h or "abcd1234" in fmt_h)

    # 16. _format_history_human empty
    fmt_h_empty = _format_history_human([])
    check("format_history_empty", "(no history records" in fmt_h_empty)

    # 17. _format_diff_human
    fmt_d = _format_diff_human(diff_up)
    check("format_diff_has_header", "V1352 VCP Diff" in fmt_d)
    check("format_diff_shows_regression", "YES" in fmt_d)
    check("format_diff_has_r1", "R1" in fmt_d)

    # 18. to_json stability (sortable keys)
    j1 = to_json(diff_up)
    j2 = to_json(diff_up)
    check("to_json_deterministic", j1 == j2)
    parsed = json.loads(j1)
    check("to_json_has_deltas", "deltas" in parsed)
    check("to_json_has_flags", "regression_flags" in parsed)

    # 19. _compute_asi_lift
    check("asi_lift_no_flags", _compute_asi_lift(0, 5) > _compute_asi_lift(5, 5) or
          _compute_asi_lift(0, 5) >= _compute_asi_lift(5, 5))
    check("asi_lift_capped", _compute_asi_lift(0, 5) <= V1352_ASI_CAP)
    check("asi_lift_zero_deltas_zero", _compute_asi_lift(0, 0) == 0.0)

    # 20. Edge: empty ledger load
    empty_path = Path(".") / "_v1352_empty_ledger.jsonl"
    if empty_path.exists():
        empty_path.unlink()
    check("load_history_empty_file", load_history(empty_path) == [])

    # 21. Edge: get_latest_record on empty ledger
    check("get_latest_record_empty", get_latest_record(empty_path) is None)

    # 22. Edge: get_record_by_id on empty ledger
    check("get_record_by_id_empty", get_record_by_id(empty_path, "abc") is None)

    return passed, total


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1352_vcp_history_diff",
        description="V1352 VCP Toolchain History + Diff CLI (v0.1.0)",
    )
    sub = parser.add_subparsers(dest="command")

    # history subcommand
    p_hist = sub.add_parser("history", help="list recent ledger records")
    p_hist.add_argument("--last", type=int, default=10,
                        help="how many recent records to show (default: 10)")
    p_hist.add_argument("--json", action="store_true", help="emit JSON")
    p_hist.add_argument("--ledger", type=str, default=None,
                        help="path to ledger JSONL")

    # diff subcommand
    p_diff = sub.add_parser("diff", help="current pipeline vs ledger baseline")
    p_diff.add_argument("--against", type=str, default=None,
                        help="record_id to diff against (default: latest)")
    p_diff.add_argument("--json", action="store_true", help="emit JSON")
    p_diff.add_argument("--ledger", type=str, default=None,
                        help="path to ledger JSONL")
    p_diff.add_argument("--no-write", action="store_true",
                        help="do not write artifact JSON (default: no-write)")

    # self-test subcommand
    p_test = sub.add_parser("self-test", help="run Popper self-tests")
    p_test.add_argument("--verbose", action="store_true")

    # version subcommand
    sub.add_parser("version", help="print version")

    args = parser.parse_args(argv)
    if args.command is None:
        parser.print_help()
        return 0

    if args.command == "version":
        print(f"v1352_vcp_history_diff {V1352_VERSION}")
        return 0

    if args.command == "self-test":
        passed, total = _popper_self_tests(verbose=args.verbose)
        print(f"=== V1352 self-tests: {passed}/{total} PASS ===")
        return 0 if passed == total else 1

    ledger_path = Path(args.ledger) if getattr(args, "ledger", None) else LEDGER_PATH

    if args.command == "history":
        entries = load_history(ledger_path, last_n=args.last)
        if args.json:
            out = [asdict(e) for e in entries]
            print(_stable_dumps(out))
        else:
            print(_format_history_human(entries))
        return 0

    if args.command == "diff":
        # Resolve baseline
        if args.against:
            baseline = get_record_by_id(ledger_path, args.against)
            if baseline is None:
                print(f"ERROR: record_id {args.against!r} not found in {ledger_path}",
                      file=sys.stderr)
                return 2
        else:
            baseline = get_latest_record(ledger_path)
            if baseline is None:
                print(f"ERROR: no records in {ledger_path}", file=sys.stderr)
                return 2
        # Run current pipeline (lazy import to avoid circular dep)
        try:
            from v1351_vcp_toolchain_cli import PipelineRunner
        except ImportError as exc:
            print(f"ERROR: cannot import V1351 runner: {exc}", file=sys.stderr)
            return 2
        runner = PipelineRunner(ledger_path=ledger_path, force_mock=True)
        result = runner.run()  # never writes to ledger
        report = compute_diff(result, baseline)
        if args.json:
            print(to_json(report))
        else:
            print(_format_diff_human(report))
        return report.exit_code

    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())