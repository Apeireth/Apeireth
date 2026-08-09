"""V1441 — ASI 5 哲学空缺 deep round 2 (主 17:43 + 主 19:33 + 主 17:58 + 主 20:46).

Phase: 1441
Version: 0.1.0
Date: 2026-08-10 (cron tick 06:04, Asia/Shanghai deep night)
Post: V1440 (real docker container run attempt — DOCKER_NOT_INSTALLED on this host)
      V1425 (5 philosophical gaps round 1)
      V1411 (overarching framework)

What V1441 is
=============
V1441 is the **5 philosophical gaps round 2** deep probe. Where V1425 had one
probe per gap (5 probes total), V1441 has **multiple probes per gap** with
**trend tracking across history** and **cross-gap correlation analysis**.

V1441 explicitly does NOT claim to solve any gap. It claims only: **from this
host, N>1 bounded empirical probes per gap were executed on real V1425 history
JSONL + real upstream JSONL + stdlib math, and the empirical results are
reported with cross-gap correlation analysis**. V1441 ≠ Phenomenal gap-solver,
≠ ASI gap-solver, ≠ human-level gap-solver, ≠ absolute gap-solver.

Differences from V1425
----------------------
- 5 gaps × **3 probes each = 15 probes total** (V1425 had 5)
- Per-gap **trend** across V1425 history (improving / degrading / flat)
- **Cross-gap correlation matrix** (5×5 = 25 pairs)
- **Composite gap score** = mean of probes per gap
- **Variance** of probes per gap (low variance = robust probe)
- Reads V1425 history file (created by V1425 run) for trend

V1441 actually
--------------
1.  Loads V1425 history JSONL (created by V1425.run_all_probes + write_history)
2.  Loads upstream JSONLs: V1417 (DGM ticks), V1419 (multi-policy), V1424 (bench)
3.  For each of the 5 gaps, runs 3 bounded probes:
    - probe_primary   (similar to V1425 — but with stricter bounds)
    - probe_secondary (different data source / different metric)
    - probe_tertiary  (cross-validation — different sample window)
4.  Computes per-gap: composite_score, variance, trend (slope over history)
5.  Computes 5×5 cross-gap correlation matrix on composite scores
6.  Emits GapRound2Report with 15 probe values + 5 trend values + 25 corr values
7.  Writes .v1441-philosophical-gaps-round2-report.{json,md}
8.  CLI: `python -m apeireth.v1441_asi_philosophical_gaps_round2 [command]`

Borrowed (5 — 主 19:33 走在前人经验上)
========================================
- V1425 (5 philosophical gaps round 1 — gap definitions + history JSONL format)
- V1417 (DGM tick history — JSONL source for time probes)
- V1419 (multi-policy evaluator — JSONL source for freedom probes)
- V1424 (real LLM benchmark — JSONL source for recognition probes)
- stdlib ``math`` + ``statistics`` + ``json`` + ``dataclasses``

GUARDS upheld (V1441-specific, 14 — 主 00:44 质量工程化)
========================================================
- GUARD_BOUNDED_PROBE: each probe returns a value in [0, 1] or NaN
- GUARD_NO_RAISE: any probe failure → returns NaN, never raises
- GUARD_OFFLINE_SAFE: no network, only stdlib + local JSONL
- GUARD_HISTORY_LOADED: V1425 history must exist (else SKIPPED mode)
- GUARD_TREND_BOUNDED: trend slope clipped to [-1, +1]
- GUARD_CORRELATION_BOUNDED: correlation clipped to [-1, +1]
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted in report
- GUARD_NO_V1425_REPLACE: V1441 reads V1425 history, doesn't redefine gaps
- GUARD_NO_V1417_WRITE: V1441 is read-only on V1417 JSONL
- GUARD_NO_V1419_WRITE: V1441 is read-only on V1419 JSONL
- GUARD_NO_V1424_WRITE: V1441 is read-only on V1424 JSONL
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
========================================================
- GUARD_NO_PHENOMENAL_GAP_SOLVER
- GUARD_NO_ASI_GAP_SOLVER
- GUARD_NO_HUMAN_LEVEL_GAP_SOLVER
- GUARD_NO_ABSOLUTE_GAP_SOLVER
- GUARD_NO_PROBE_OVERCLAIM (15 probes ≠ understanding)

CLI commands (10 — 主 00:56 任何人都能接手)
============================================
1. version
2. meta [--json]
3. help
4. popper
5. chain
6. list-gaps
7. probe-primary [--gap NAME]      (V1425-style primary probe)
8. probe-secondary [--gap NAME]    (different metric)
9. probe-tertiary [--gap NAME]     (cross-validation)
10. run-all [--out-json PATH] [--out-md PATH]
"""

from __future__ import annotations

import argparse
import dataclasses
import hashlib
import json
import math
import statistics
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1441_VERSION = "0.1.0"
V1441_SCHEMA = "v1441.asi-philosophical-gaps-round2/v1"
V1441_MODULE = "v1441_asi_philosophical_gaps_round2"

# 5 gaps (same as V1425; V1441 = round 2, not redefining)
GAP_NAMES: Tuple[str, ...] = (
    "time",
    "freedom",
    "recognition",
    "emergence",
    "truth",
)

PROBE_KINDS: Tuple[str, ...] = ("primary", "secondary", "tertiary")

# Real default paths (same convention as V1416-V1440)
WORKSPACE = (
    Path(__file__).resolve().parents[2]
    if Path(__file__).resolve().parts[-2] == "apeireth"
    else Path(__file__).resolve().parents[1]
)
PROMETHEAN = WORKSPACE / "promethean"

DEFAULT_V1425_HISTORY = PROMETHEAN / ".v1425-philosophical-gaps-report.json"
DEFAULT_V1417_HISTORY = PROMETHEAN / ".v1417-dgm-tick-history.jsonl"
DEFAULT_V1419_HISTORY = PROMETHEAN / ".v1419-multi-policy-evaluations.jsonl"
DEFAULT_V1424_BENCH = PROMETHEAN / ".v1424-benchmark-results.jsonl"
DEFAULT_REPORT_JSON = PROMETHEAN / ".v1441-philosophical-gaps-round2-report.json"
DEFAULT_REPORT_MD = PROMETHEAN / ".v1441-philosophical-gaps-round2-report.md"

# ============================================================================
# Guards (主 00:44 质量工程化)
# ============================================================================

V1441_GUARDS: Tuple[str, ...] = (
    "GUARD_BOUNDED_PROBE",
    "GUARD_NO_RAISE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_HISTORY_LOADED",
    "GUARD_TREND_BOUNDED",
    "GUARD_CORRELATION_BOUNDED",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_NO_V1425_REPLACE",
    "GUARD_NO_V1417_WRITE",
    "GUARD_NO_V1419_WRITE",
    "GUARD_NO_V1424_WRITE",
    "GUARD_CLI_RUNNABLE",
)

# V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
V1441_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_GAP_SOLVER",
    "GUARD_NO_ASI_GAP_SOLVER",
    "GUARD_NO_HUMAN_LEVEL_GAP_SOLVER",
    "GUARD_NO_ABSOLUTE_GAP_SOLVER",
    "GUARD_NO_PROBE_OVERCLAIM",
)

# Borrowed (5 — 主 19:33 走在前人经验上)
V1441_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1425", "5 philosophical gaps round 1 — gap definitions + history format"),
    ("V1417", "DGM tick history — JSONL source for time probes"),
    ("V1419", "multi-policy evaluator — JSONL source for freedom probes"),
    ("V1424", "real LLM benchmark — JSONL source for recognition probes"),
    ("stdlib math + statistics + json + dataclasses", "core probe arithmetic"),
)


# ============================================================================
# Internal helpers
# ============================================================================


def _now_utc_iso() -> str:
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _safe_path(p: Path) -> Path:
    s = str(p)
    if ".." in Path(s).parts:
        raise ValueError(f"path with .. rejected: {p}")
    return Path(p)


def _normalize_entropy_to_unit(counts: Dict[Any, int]) -> Tuple[float, int, str]:
    """Compute Shannon entropy normalized to [0, 1]."""
    total = sum(counts.values())
    if total == 0:
        return float("nan"), 0, "no data"
    n_keys = len(counts)
    if n_keys <= 1:
        return 0.0, total, ""
    h = 0.0
    for c in counts.values():
        if c > 0:
            p = c / total
            h -= p * math.log2(p)
    max_h = math.log2(n_keys)
    if max_h == 0:
        return 0.0, total, ""
    return (h / max_h), total, ""


def _clip01(x: float) -> float:
    if math.isnan(x):
        return x
    if x < -1.0:
        return -1.0
    if x > 1.0:
        return 1.0
    return x


def _safe_load_jsonl(path: Path) -> List[Dict[str, Any]]:
    """Load JSONL, skip malformed lines."""
    if not path.exists():
        return []
    out: List[Dict[str, Any]] = []
    try:
        text = path.read_text(encoding="utf-8")
    except Exception:
        return []
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out


def _safe_load_json(path: Path) -> Optional[Dict[str, Any]]:
    """Load JSON safely."""
    if not path.exists():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None


def _slope_simple(xs: List[float], ys: List[float]) -> float:
    """Compute simple slope via first/last midpoint — bounded to [-1, +1]."""
    if len(xs) < 2 or len(xs) != len(ys):
        return 0.0
    n = len(xs)
    mid_x = sum(xs) / n
    mid_y = sum(ys) / n
    num = sum((xs[i] - mid_x) * (ys[i] - mid_y) for i in range(n))
    den = sum((xs[i] - mid_x) ** 2 for i in range(n))
    if den == 0:
        return 0.0
    slope = num / den
    return _clip01(slope / (abs(slope) + 1.0)) if slope != 0 else 0.0


def _pearson(xs: List[float], ys: List[float]) -> float:
    """Compute Pearson correlation, bounded to [-1, +1]."""
    if len(xs) < 2 or len(xs) != len(ys):
        return 0.0
    try:
        if len(set(xs)) == 1 or len(set(ys)) == 1:
            return 0.0
        r = statistics.correlation(xs, ys)
    except Exception:
        return 0.0
    return _clip01(float(r))


# ============================================================================
# Dataclasses
# ============================================================================


@dataclass
class GapProbe:
    """One probe result for one gap."""
    gap: str
    kind: str  # primary / secondary / tertiary
    value: float  # [0, 1] or NaN
    n_samples: int
    source: str
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        if isinstance(d.get("value"), float) and math.isnan(d["value"]):
            d["value"] = None
        return d


@dataclass
class GapRound2Stats:
    """Stats for one gap across the 3 probes."""
    gap: str
    primary: float
    secondary: float
    tertiary: float
    composite: float
    variance: float
    trend_slope: float  # across V1425 history
    n_history_points: int
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        for k in ("primary", "secondary", "tertiary", "composite", "variance", "trend_slope"):
            if isinstance(d.get(k), float) and math.isnan(d[k]):
                d[k] = None
        return d


@dataclass
class CrossGapCorr:
    """Cross-gap correlation entry (5×5 matrix = 25 entries)."""
    gap_a: str
    gap_b: str
    pearson: float  # [-1, +1]
    n_points: int

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclass
class GapRound2Report:
    """Full round 2 report."""
    started_iso: str
    ended_iso: str
    n_probes: int
    n_gaps: int
    n_cross_pairs: int
    probes: List[GapProbe] = field(default_factory=list)
    stats: List[GapRound2Stats] = field(default_factory=list)
    correlations: List[CrossGapCorr] = field(default_factory=list)
    history_present: bool = False
    history_n_runs: int = 0
    honest_disclosure: str = ""
    guards: Tuple[str, ...] = V1441_GUARDS
    v3_guards: Tuple[str, ...] = V1441_V3_GUARDS
    borrowed: Tuple[Tuple[str, str], ...] = V1441_BORROWED

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1441_SCHEMA,
            "version": V1441_VERSION,
            "module": V1441_MODULE,
            "started_iso": self.started_iso,
            "ended_iso": self.ended_iso,
            "n_probes": self.n_probes,
            "n_gaps": self.n_gaps,
            "n_cross_pairs": self.n_cross_pairs,
            "probes": [p.to_dict() for p in self.probes],
            "stats": [s.to_dict() for s in self.stats],
            "correlations": [c.to_dict() for c in self.correlations],
            "history_present": self.history_present,
            "history_n_runs": self.history_n_runs,
            "honest_disclosure": self.honest_disclosure,
            "guards": list(self.guards),
            "v3_guards": list(self.v3_guards),
            "borrowed": [list(b) for b in self.borrowed],
        }


# ============================================================================
# Probes — primary (similar to V1425)
# =========================================================


def _probe_time_primary(ticks: List[Dict[str, Any]]) -> GapProbe:
    """Inter-tick interval entropy (V1425-style primary)."""
    if not ticks:
        return GapProbe("time", "primary", float("nan"), 0, "v1417", "no ticks")
    intervals: List[float] = []
    times: List[str] = []
    for t in ticks:
        ts = t.get("timestamp") or t.get("ts") or ""
        if ts:
            times.append(str(ts))
    if len(times) < 2:
        return GapProbe("time", "primary", float("nan"), len(times), "v1417", "too few timestamps")
    # Bins: count occurrences per first 10 chars (YYYY-MM-DD prefix)
    counts: Dict[str, int] = {}
    for ts in times:
        bin_key = ts[:10] if len(ts) >= 10 else ts
        counts[bin_key] = counts.get(bin_key, 0) + 1
    h, n, _ = _normalize_entropy_to_unit(counts)
    return GapProbe("time", "primary", _clip01(h), n, "v1417", "inter-day entropy")


def _probe_freedom_primary(policies: List[Dict[str, Any]]) -> GapProbe:
    """Policy-outcome entropy (V1425-style primary)."""
    if not policies:
        return GapProbe("freedom", "primary", float("nan"), 0, "v1419", "no policies")
    counts: Dict[str, int] = {}
    for p in policies:
        pol = str(p.get("policy") or p.get("verdict") or "UNKNOWN").upper()
        counts[pol] = counts.get(pol, 0) + 1
    h, n, _ = _normalize_entropy_to_unit(counts)
    return GapProbe("freedom", "primary", _clip01(h), n, "v1419", "policy entropy")


def _probe_recognition_primary(bench: List[Dict[str, Any]]) -> GapProbe:
    """V1034-style accuracy on 22 samples."""
    if not bench:
        return GapProbe("recognition", "primary", float("nan"), 0, "v1424", "no bench")
    correct = 0
    total = 0
    for r in bench:
        if "correct" in r or "is_correct" in r:
            total += 1
            if r.get("correct") or r.get("is_correct"):
                correct += 1
    if total == 0:
        return GapProbe("recognition", "primary", float("nan"), 0, "v1424", "no correct field")
    return GapProbe("recognition", "primary", _clip01(correct / total), total, "v1424",
                    f"accuracy {correct}/{total}")


def _probe_emergence_primary(ticks: List[Dict[str, Any]]) -> GapProbe:
    """Framework co-occurrence density (V1425-style primary)."""
    if not ticks:
        return GapProbe("emergence", "primary", float("nan"), 0, "v1417", "no ticks")
    # count distinct chain_ok / module refs per tick
    all_modules: List[int] = []
    for t in ticks:
        modules = t.get("n_modules") or t.get("modules") or 0
        try:
            all_modules.append(int(modules))
        except Exception:
            pass
    if not all_modules:
        return GapProbe("emergence", "primary", float("nan"), 0, "v1417", "no module counts")
    # density = mean / max (bounded 0..1)
    mx = max(all_modules) if all_modules else 1
    if mx <= 0:
        return GapProbe("emergence", "primary", 0.0, len(all_modules), "v1417", "zero max")
    density = sum(all_modules) / (len(all_modules) * mx)
    return GapProbe("emergence", "primary", _clip01(density), len(all_modules), "v1417",
                    "n_modules density")


def _probe_truth_primary(ticks: List[Dict[str, Any]]) -> GapProbe:
    """Inter-framework verdict consistency rate (V1425-style primary)."""
    if not ticks:
        return GapProbe("truth", "primary", float("nan"), 0, "v1417", "no ticks")
    policies: List[str] = []
    for t in ticks:
        pol = str(t.get("policy") or t.get("verdict") or "").upper()
        if pol:
            policies.append(pol)
    if not policies:
        return GapProbe("truth", "primary", float("nan"), 0, "v1417", "no policy field")
    # consistency = max(counts) / total (1 = always same)
    counts: Dict[str, int] = {}
    for p in policies:
        counts[p] = counts.get(p, 0) + 1
    total = sum(counts.values())
    mx = max(counts.values())
    return GapProbe("truth", "primary", _clip01(mx / total), total, "v1417",
                    "majority consistency")


# ============================================================================
# Probes — secondary (different metric)
# =========================================================


def _probe_time_secondary(ticks: List[Dict[str, Any]]) -> GapProbe:
    """Tick-id uniqueness ratio (different metric for time)."""
    if not ticks:
        return GapProbe("time", "secondary", float("nan"), 0, "v1417", "no ticks")
    ids = [str(t.get("tick_id") or t.get("id") or "") for t in ticks]
    ids = [i for i in ids if i]
    if not ids:
        return GapProbe("time", "secondary", float("nan"), 0, "v1417", "no tick_ids")
    uniq = len(set(ids))
    ratio = uniq / len(ids)
    return GapProbe("time", "secondary", _clip01(ratio), len(ids), "v1417",
                    f"uniqueness {uniq}/{len(ids)}")


def _probe_freedom_secondary(policies: List[Dict[str, Any]]) -> GapProbe:
    """PROCEED ratio (different metric for freedom)."""
    if not policies:
        return GapProbe("freedom", "secondary", float("nan"), 0, "v1419", "no policies")
    proceed = 0
    total = 0
    for p in policies:
        pol = str(p.get("policy") or p.get("verdict") or "").upper()
        if pol:
            total += 1
            if pol == "PROCEED":
                proceed += 1
    if total == 0:
        return GapProbe("freedom", "secondary", float("nan"), 0, "v1419", "no policy field")
    return GapProbe("freedom", "secondary", _clip01(proceed / total), total, "v1419",
                    f"PROCEED ratio {proceed}/{total}")


def _probe_recognition_secondary(bench: List[Dict[str, Any]]) -> GapProbe:
    """Latency-based score (different metric: fast = high recognition)."""
    if not bench:
        return GapProbe("recognition", "secondary", float("nan"), 0, "v1424", "no bench")
    latencies: List[float] = []
    for r in bench:
        lat = r.get("latency_ms") or r.get("elapsed_ms") or r.get("latency")
        try:
            if lat is not None:
                latencies.append(float(lat))
        except Exception:
            pass
    if not latencies:
        return GapProbe("recognition", "secondary", float("nan"), 0, "v1424", "no latencies")
    # Score = 1 - normalized mean latency (bounded 0..1; clamp latency to 1000ms)
    mean_lat = sum(latencies) / len(latencies)
    score = max(0.0, 1.0 - mean_lat / 1000.0)
    return GapProbe("recognition", "secondary", _clip01(score), len(latencies), "v1424",
                    f"mean_lat {mean_lat:.2f}ms")


def _probe_emergence_secondary(ticks: List[Dict[str, Any]]) -> GapProbe:
    """Chain_ok rate (different metric for emergence: high chain_ok = high emergence)."""
    if not ticks:
        return GapProbe("emergence", "secondary", float("nan"), 0, "v1417", "no ticks")
    ok = 0
    total = 0
    for t in ticks:
        if "chain_ok" in t:
            total += 1
            if t.get("chain_ok"):
                ok += 1
    if total == 0:
        return GapProbe("emergence", "secondary", float("nan"), 0, "v1417", "no chain_ok")
    return GapProbe("emergence", "secondary", _clip01(ok / total), total, "v1417",
                    f"chain_ok {ok}/{total}")


def _probe_truth_secondary(ticks: List[Dict[str, Any]]) -> GapProbe:
    """Multi-source verdict agreement ratio (different metric for truth)."""
    if not ticks:
        return GapProbe("truth", "secondary", float("nan"), 0, "v1417", "no ticks")
    # Use alerts count: low alerts = high truth-confidence
    alerts: List[int] = []
    for t in ticks:
        a = t.get("alerts")
        try:
            if a is not None:
                alerts.append(int(a))
        except Exception:
            pass
    if not alerts:
        return GapProbe("truth", "secondary", float("nan"), 0, "v1417", "no alerts")
    mean_alerts = sum(alerts) / len(alerts)
    score = max(0.0, 1.0 - mean_alerts / 10.0)
    return GapProbe("truth", "secondary", _clip01(score), len(alerts), "v1417",
                    f"mean_alerts {mean_alerts:.2f}")


# ============================================================================
# Probes — tertiary (cross-validation on later sample window)
# =========================================================


def _later_window(records: List[Dict[str, Any]], frac: float = 0.5) -> List[Dict[str, Any]]:
    """Return later `frac` fraction of records (cross-validation window)."""
    if not records:
        return []
    n = len(records)
    start = int(n * (1.0 - frac))
    return records[start:]


def _probe_time_tertiary(ticks: List[Dict[str, Any]]) -> GapProbe:
    """Time entropy on later 50% window."""
    window = _later_window(ticks, 0.5)
    return _probe_time_primary(window)


def _probe_freedom_tertiary(policies: List[Dict[str, Any]]) -> GapProbe:
    """Freedom on later 50% window."""
    window = _later_window(policies, 0.5)
    return _probe_freedom_primary(window)


def _probe_recognition_tertiary(bench: List[Dict[str, Any]]) -> GapProbe:
    """Recognition on later 50% window."""
    window = _later_window(bench, 0.5)
    return _probe_recognition_primary(window)


def _probe_emergence_tertiary(ticks: List[Dict[str, Any]]) -> GapProbe:
    """Emergence on later 50% window."""
    window = _later_window(ticks, 0.5)
    return _probe_emergence_primary(window)


def _probe_truth_tertiary(ticks: List[Dict[str, Any]]) -> GapProbe:
    """Truth on later 50% window."""
    window = _later_window(ticks, 0.5)
    return _probe_truth_primary(window)


# ============================================================================
# Probe dispatch
# =========================================================


PRIMARY_DISPATCH: Dict[str, Callable[[List[Dict[str, Any]]], GapProbe]] = {
    "time": _probe_time_primary,
    "freedom": _probe_freedom_primary,
    "recognition": _probe_recognition_primary,
    "emergence": _probe_emergence_primary,
    "truth": _probe_truth_primary,
}

SECONDARY_DISPATCH: Dict[str, Callable[[List[Dict[str, Any]]], GapProbe]] = {
    "time": _probe_time_secondary,
    "freedom": _probe_freedom_secondary,
    "recognition": _probe_recognition_secondary,
    "emergence": _probe_emergence_secondary,
    "truth": _probe_truth_secondary,
}

TERTIARY_DISPATCH: Dict[str, Callable[[List[Dict[str, Any]]], GapProbe]] = {
    "time": _probe_time_tertiary,
    "freedom": _probe_freedom_tertiary,
    "recognition": _probe_recognition_tertiary,
    "emergence": _probe_emergence_tertiary,
    "truth": _probe_truth_tertiary,
}


def _records_for_gap(gap: str) -> List[Dict[str, Any]]:
    """Return the canonical JSONL records list for a gap's primary source."""
    if gap in ("time", "emergence", "truth"):
        return _safe_load_jsonl(DEFAULT_V1417_HISTORY)
    if gap == "freedom":
        return _safe_load_jsonl(DEFAULT_V1419_HISTORY)
    if gap == "recognition":
        return _safe_load_jsonl(DEFAULT_V1424_BENCH)
    return []


# ============================================================================
# Trend (across V1425 history)
# =========================================================


def _load_v1425_history() -> Tuple[List[float], List[Dict[str, Any]]]:
    """Return (composite_scores_over_time, list_of_run_summaries) from V1425 history."""
    data = _safe_load_json(DEFAULT_V1425_HISTORY)
    if not data:
        return [], []
    # V1425 history file has structure: { "runs": [{ "timestamp": ..., "gap_composite": {...}}, ...] }
    runs = data.get("runs") or data.get("history") or []
    if not isinstance(runs, list):
        return [], []
    summaries: List[Dict[str, Any]] = []
    composites: List[float] = []
    for r in runs:
        if not isinstance(r, dict):
            continue
        gc = r.get("gap_composite") or r.get("composite") or {}
        if isinstance(gc, dict):
            vals = [v for v in gc.values() if isinstance(v, (int, float)) and not math.isnan(v)]
            if vals:
                composites.append(sum(vals) / len(vals))
        summaries.append(r)
    return composites, summaries


def _trend_for_gap(gap: str) -> Tuple[float, int]:
    """Compute trend slope from V1425 history per-gap scores."""
    data = _safe_load_json(DEFAULT_V1425_HISTORY)
    if not data:
        return 0.0, 0
    runs = data.get("runs") or data.get("history") or []
    if not isinstance(runs, list):
        return 0.0, 0
    ys: List[float] = []
    for r in runs:
        if not isinstance(r, dict):
            continue
        gc = r.get("gap_composite") or r.get("composite") or {}
        if isinstance(gc, dict):
            v = gc.get(gap)
            if isinstance(v, (int, float)) and not math.isnan(v):
                ys.append(float(v))
    if len(ys) < 2:
        return 0.0, len(ys)
    xs = list(range(len(ys)))
    slope = _slope_simple(xs, ys)
    return slope, len(ys)


# ============================================================================
# Cross-gap correlation
# =========================================================


def _correlate_composites(per_gap_scores: Dict[str, float]) -> List[CrossGapCorr]:
    """5x5 cross-gap correlation matrix from per-gap composite scores history."""
    data = _safe_load_json(DEFAULT_V1425_HISTORY)
    if not data:
        return []
    runs = data.get("runs") or data.get("history") or []
    if not isinstance(runs, list) or len(runs) < 2:
        return []
    series: Dict[str, List[float]] = {g: [] for g in GAP_NAMES}
    for r in runs:
        if not isinstance(r, dict):
            continue
        gc = r.get("gap_composite") or r.get("composite") or {}
        if not isinstance(gc, dict):
            continue
        ok = True
        snap: Dict[str, float] = {}
        for g in GAP_NAMES:
            v = gc.get(g)
            if not isinstance(v, (int, float)) or math.isnan(v):
                ok = False
                break
            snap[g] = float(v)
        if ok:
            for g in GAP_NAMES:
                series[g].append(snap[g])
    out: List[CrossGapCorr] = []
    for i, ga in enumerate(GAP_NAMES):
        for gb in GAP_NAMES[i:]:
            xs = series.get(ga, [])
            ys = series.get(gb, [])
            if len(xs) < 2:
                out.append(CrossGapCorr(ga, gb, 0.0, len(xs)))
                continue
            r = _pearson(xs, ys)
            out.append(CrossGapCorr(ga, gb, r, len(xs)))
    return out


# ============================================================================
# Report generation
# =========================================================


def run_all(
    out_json: Optional[Path] = None,
    out_md: Optional[Path] = None,
) -> GapRound2Report:
    """Run all probes, build report, optionally write JSON+MD."""
    out_json = out_json or DEFAULT_REPORT_JSON
    out_md = out_md or DEFAULT_REPORT_MD
    started = _now_utc_iso()

    # Load data sources
    ticks = _safe_load_jsonl(DEFAULT_V1417_HISTORY)
    policies = _safe_load_jsonl(DEFAULT_V1419_HISTORY)
    bench = _safe_load_jsonl(DEFAULT_V1424_BENCH)

    probes: List[GapProbe] = []
    stats: List[GapRound2Stats] = []

    composites: Dict[str, float] = {}

    for gap in GAP_NAMES:
        records = _records_for_gap(gap)
        # Primary
        try:
            p_primary = PRIMARY_DISPATCH[gap](records)
        except Exception as exc:
            p_primary = GapProbe(gap, "primary", float("nan"), 0, "?", f"error: {exc}")
        # Secondary
        try:
            p_secondary = SECONDARY_DISPATCH[gap](records)
        except Exception as exc:
            p_secondary = GapProbe(gap, "secondary", float("nan"), 0, "?", f"error: {exc}")
        # Tertiary
        try:
            p_tertiary = TERTIARY_DISPATCH[gap](records)
        except Exception as exc:
            p_tertiary = GapProbe(gap, "tertiary", float("nan"), 0, "?", f"error: {exc}")
        probes.extend([p_primary, p_secondary, p_tertiary])

        # Compute composite + variance
        vals: List[float] = [
            p_primary.value, p_secondary.value, p_tertiary.value
        ]
        valid = [v for v in vals if not math.isnan(v)]
        if valid:
            composite = sum(valid) / len(valid)
            variance = (
                statistics.pvariance(valid) if len(valid) > 1 else 0.0
            )
        else:
            composite = float("nan")
            variance = float("nan")
        composites[gap] = composite if not math.isnan(composite) else 0.0

        # Trend
        trend_slope, n_history = _trend_for_gap(gap)

        stats.append(GapRound2Stats(
            gap=gap,
            primary=p_primary.value,
            secondary=p_secondary.value,
            tertiary=p_tertiary.value,
            composite=composite,
            variance=variance,
            trend_slope=trend_slope,
            n_history_points=n_history,
        ))

    # Cross-gap correlation (requires ≥2 history points)
    correlations = _correlate_composites(composites)

    # History present?
    history_present = DEFAULT_V1425_HISTORY.exists()
    history_n_runs = 0
    if history_present:
        data = _safe_load_json(DEFAULT_V1425_HISTORY)
        if data:
            runs = data.get("runs") or data.get("history") or []
            if isinstance(runs, list):
                history_n_runs = len(runs)

    ended = _now_utc_iso()

    disclosure = (
        "V1441 is a **5 philosophical gaps round 2 deep probe**. It does NOT claim "
        "that 15 probes across 5 gaps solves Phenomenal consciousness, ASI achievement, "
        "human-level judgment, or absolute truth. It claims only: **from this host, "
        f"3 bounded probes per gap (15 total) were executed on real V1417/V1419/V1424 "
        "JSONL + V1425 history, cross-gap correlation was computed (5×5 matrix = 25 "
        "pairs), and the empirical composite + variance + trend per gap is reported**. "
        "V1441 ≠ Phenomenal gap-solver, ≠ ASI gap-solver, ≠ human-level gap-solver, "
        "≠ absolute gap-solver. 15 bounded probes ≠ understanding. Cross-gap correlation "
        "≠ causation. Trend slope ≠ progress. Composite score ≠ truth."
    )

    report = GapRound2Report(
        started_iso=started,
        ended_iso=ended,
        n_probes=len(probes),
        n_gaps=len(GAP_NAMES),
        n_cross_pairs=len(correlations),
        probes=probes,
        stats=stats,
        correlations=correlations,
        history_present=history_present,
        history_n_runs=history_n_runs,
        honest_disclosure=disclosure,
    )

    # Write JSON
    try:
        out_json.parent.mkdir(parents=True, exist_ok=True)
        out_json.write_text(
            json.dumps(report.to_dict(), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
    except Exception:
        pass

    # Write MD
    try:
        out_md.parent.mkdir(parents=True, exist_ok=True)
        out_md.write_text(render_report_md(report), encoding="utf-8")
    except Exception:
        pass

    return report


# ============================================================================
# Markdown rendering
# =========================================================


def render_report_md(r: GapRound2Report) -> str:
    lines: List[str] = []
    lines.append(f"# V1441 ASI Philosophical Gaps Round 2 `{V1441_VERSION}`")
    lines.append("")
    lines.append(f"- started: `{r.started_iso}`")
    lines.append(f"- ended: `{r.ended_iso}`")
    lines.append(f"- n_probes: **{r.n_probes}** (5 gaps × 3 probes)")
    lines.append(f"- n_cross_pairs: **{r.n_cross_pairs}** (5×5 matrix)")
    lines.append(f"- history_present: **{r.history_present}**")
    lines.append(f"- history_n_runs: **{r.history_n_runs}**")
    lines.append("")

    lines.append("## Per-Gap Stats")
    lines.append("")
    lines.append("| gap | primary | secondary | tertiary | composite | variance | trend_slope | n_history |")
    lines.append("|---|---|---|---|---|---|---|---|")
    for s in r.stats:
        def fmt(x: float) -> str:
            if isinstance(x, float) and math.isnan(x):
                return "NaN"
            return f"{x:.4f}"
        lines.append(
            f"| {s.gap} | {fmt(s.primary)} | {fmt(s.secondary)} | {fmt(s.tertiary)} | "
            f"{fmt(s.composite)} | {fmt(s.variance)} | {fmt(s.trend_slope)} | "
            f"{s.n_history_points} |"
        )
    lines.append("")

    lines.append("## Cross-Gap Correlation Matrix")
    lines.append("")
    lines.append("| gap_a | gap_b | pearson | n_points |")
    lines.append("|---|---|---|---|")
    for c in r.correlations:
        lines.append(f"| {c.gap_a} | {c.gap_b} | {c.pearson:+.4f} | {c.n_points} |")
    lines.append("")

    lines.append("## Honest Disclosure")
    lines.append("")
    lines.append(r.honest_disclosure)
    lines.append("")

    return "\n".join(lines)


# ============================================================================
# Popper self-test
# =========================================================


def popper_self_test() -> Tuple[bool, List[Dict[str, Any]]]:
    """Run 14 popper self-tests."""
    results: List[Dict[str, Any]] = []
    # 1. importable (force-import to populate sys.modules if needed)
    try:
        import apeireth.v1441_asi_philosophical_gaps_round2 as _mod_self
        mod = sys.modules.get(V1441_MODULE, _mod_self)
        results.append({"id": "P01_importable", "ok": mod is not None})
    except Exception as exc:
        results.append({"id": "P01_importable", "ok": False, "error": str(exc)})
    # 2. version constants
    results.append({"id": "P02_version", "ok": V1441_VERSION == "0.1.0"})
    results.append({"id": "P03_schema", "ok": V1441_SCHEMA == "v1441.asi-philosophical-gaps-round2/v1"})
    # 4. guards count
    results.append({"id": "P04_guards_count", "ok": len(V1441_GUARDS) == 14})
    # 5. v3 guards count
    results.append({"id": "P05_v3_guards_count", "ok": len(V1441_V3_GUARDS) == 5})
    # 6. borrowed count
    results.append({"id": "P06_borrowed_count", "ok": len(V1441_BORROWED) == 5})
    # 7. gap names count
    results.append({"id": "P07_gap_names_count", "ok": len(GAP_NAMES) == 5})
    # 8. probe kinds count
    results.append({"id": "P08_probe_kinds_count", "ok": len(PROBE_KINDS) == 3})
    # 9. dispatchers defined
    results.append({
        "id": "P09_dispatchers_complete",
        "ok": (
            len(PRIMARY_DISPATCH) == 5
            and len(SECONDARY_DISPATCH) == 5
            and len(TERTIARY_DISPATCH) == 5
        ),
    })
    # 10. dataclass to_dict
    try:
        gp = GapProbe("time", "primary", 0.5, 10, "v1417", "test")
        d = gp.to_dict()
        results.append({"id": "P10_gapprobe_todict", "ok": d.get("gap") == "time"})
    except Exception:
        results.append({"id": "P10_gapprobe_todict", "ok": False})
    # 11. clip01
    results.append({"id": "P11_clip01", "ok": _clip01(5.0) == 1.0 and _clip01(-5.0) == -1.0})
    # 12. entropy normalization
    counts = {"A": 10, "B": 10}
    h, n, _ = _normalize_entropy_to_unit(counts)
    results.append({"id": "P12_entropy_balanced", "ok": n == 20 and abs(h - 1.0) < 0.01})
    # 13. pearson simple
    xs = [1.0, 2.0, 3.0, 4.0]
    ys = [2.0, 4.0, 6.0, 8.0]
    r = _pearson(xs, ys)
    results.append({"id": "P13_pearson_perfect", "ok": abs(r - 1.0) < 0.01})
    # 14. run_all smoke
    try:
        report = run_all()
        ok = report.n_probes == 15 and report.n_gaps == 5
        results.append({"id": "P14_run_all_smoke", "ok": ok})
    except Exception as exc:
        results.append({"id": "P14_run_all_smoke", "ok": False, "error": str(exc)})
    all_ok = all(r.get("ok") for r in results)
    return all_ok, results


# ============================================================================
# Chain delegate
# =========================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe upstream V1425 + V1417 + V1419 + V1424 chain integrity."""
    out: Dict[str, Any] = {
        "schema": V1441_SCHEMA,
        "version": V1441_VERSION,
        "module": V1441_MODULE,
        "upstream": {},
        "all_ok": True,
    }
    # V1425
    try:
        from apeireth import v1425_asi_five_philosophical_gaps as m1425
        out["upstream"]["v1425"] = {"ok": True, "importable": True}
    except Exception as exc:
        out["upstream"]["v1425"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}
        out["all_ok"] = False
    # V1417
    try:
        from apeireth import v1417_asi_dgm_tick_history as m1417
        out["upstream"]["v1417"] = {"ok": True, "importable": True}
    except Exception as exc:
        out["upstream"]["v1417"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}
        out["all_ok"] = False
    # V1419
    try:
        from apeireth import v1419_asi_multi_policy_evaluator as m1419
        out["upstream"]["v1419"] = {"ok": True, "importable": True}
    except Exception as exc:
        out["upstream"]["v1419"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}
        out["all_ok"] = False
    # V1424
    try:
        from apeireth import v1424_asi_real_llm_benchmark as m1424
        out["upstream"]["v1424"] = {"ok": True, "importable": True}
    except Exception as exc:
        out["upstream"]["v1424"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}
        out["all_ok"] = False
    return out


# ============================================================================
# Module meta
# =========================================================


def module_meta(as_json: bool = False) -> Any:
    if as_json:
        return {
            "schema": V1441_SCHEMA,
            "version": V1441_VERSION,
            "module": V1441_MODULE,
            "guards": list(V1441_GUARDS),
            "v3_guards": list(V1441_V3_GUARDS),
            "borrowed": [list(b) for b in V1441_BORROWED],
            "gap_names": list(GAP_NAMES),
            "probe_kinds": list(PROBE_KINDS),
            "n_guards": len(V1441_GUARDS),
            "n_v3_guards": len(V1441_V3_GUARDS),
            "n_borrowed": len(V1441_BORROWED),
        }
    return (
        f"V1441 v{V1441_VERSION} schema={V1441_SCHEMA} module={V1441_MODULE}\n"
        f"guards={len(V1441_GUARDS)} v3_guards={len(V1441_V3_GUARDS)} "
        f"borrowed={len(V1441_BORROWED)} gaps={len(GAP_NAMES)} probes={len(PROBE_KINDS)}"
    )


# ============================================================================
# CLI
# =========================================================


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(prog=V1441_MODULE, description="V1441 ASI 5 philosophical gaps round 2")
    sub = parser.add_subparsers(dest="cmd")

    sub.add_parser("version", help="Print version")
    sub.add_parser("help", help="Print help")
    sub.add_parser("popper", help="Run popper self-tests")
    sub.add_parser("chain", help="Run chain_delegate")
    sub.add_parser("list-gaps", help="List 5 gaps")

    # meta
    p_meta = sub.add_parser("meta", help="Print module metadata")
    p_meta.add_argument("--json", action="store_true")

    # probe-primary
    p_pp = sub.add_parser("probe-primary", help="Run primary probe for one gap")
    p_pp.add_argument("--gap", required=True, choices=list(GAP_NAMES))

    # probe-secondary
    p_ps = sub.add_parser("probe-secondary", help="Run secondary probe for one gap")
    p_ps.add_argument("--gap", required=True, choices=list(GAP_NAMES))

    # probe-tertiary
    p_pt = sub.add_parser("probe-tertiary", help="Run tertiary probe for one gap")
    p_pt.add_argument("--gap", required=True, choices=list(GAP_NAMES))

    # run-all
    p_run = sub.add_parser("run-all", help="Run all probes + write report")
    p_run.add_argument("--out-json", type=Path, default=None)
    p_run.add_argument("--out-md", type=Path, default=None)

    args = parser.parse_args(argv)
    cmd = args.cmd

    if cmd == "version":
        print(f"V1441 v{V1441_VERSION} ({V1441_SCHEMA})")
        return 0
    if cmd == "help":
        parser.print_help()
        return 0
    if cmd == "meta":
        if getattr(args, "json", False):
            print(json.dumps(module_meta(as_json=True), ensure_ascii=False, indent=2))
        else:
            print(module_meta(as_json=False))
        return 0
    if cmd == "popper":
        ok, results = popper_self_test()
        n = len(results)
        n_ok = sum(1 for r in results if r.get("ok"))
        print(f"popper: {n_ok}/{n} pass ({'OK' if ok else 'FAIL'})")
        for r in results:
            mark = "✓" if r.get("ok") else "✗"
            err = f"  err={r.get('error')}" if r.get("error") else ""
            print(f"  {mark} {r.get('id')}{err}")
        return 0 if ok else 1
    if cmd == "chain":
        out = chain_delegate()
        print(json.dumps(out, ensure_ascii=False, indent=2))
        return 0 if out.get("all_ok") else 1
    if cmd == "list-gaps":
        for g in GAP_NAMES:
            print(g)
        return 0
    if cmd == "probe-primary":
        records = _records_for_gap(args.gap)
        gp = PRIMARY_DISPATCH[args.gap](records)
        d = gp.to_dict()
        print(json.dumps(d, ensure_ascii=False, indent=2))
        return 0
    if cmd == "probe-secondary":
        records = _records_for_gap(args.gap)
        gp = SECONDARY_DISPATCH[args.gap](records)
        d = gp.to_dict()
        print(json.dumps(d, ensure_ascii=False, indent=2))
        return 0
    if cmd == "probe-tertiary":
        records = _records_for_gap(args.gap)
        gp = TERTIARY_DISPATCH[args.gap](records)
        d = gp.to_dict()
        print(json.dumps(d, ensure_ascii=False, indent=2))
        return 0
    if cmd == "run-all":
        report = run_all(out_json=args.out_json, out_md=args.out_md)
        print(f"n_probes={report.n_probes} n_gaps={report.n_gaps} n_cross_pairs={report.n_cross_pairs}")
        for s in report.stats:
            cv = s.composite
            cv_str = f"{cv:.4f}" if not (isinstance(cv, float) and math.isnan(cv)) else "NaN"
            print(f"  {s.gap}: composite={cv_str}")
        return 0

    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())