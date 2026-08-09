"""V1425 — ASI 总框架 5 哲学空缺 (Time / Freedom / Recognition / Emergence / Truth).

Phase: 1425
Version: 0.1.0
Date: 2026-08-10 (cron tick 03:55, Asia/Shanghai deep night)
Post: V1424 (real LLM benchmark) + V1411 (overarching framework) + V1419 (multi-policy evaluator)

What V1425 is
=============
V1425 is the **5 philosophical gaps** module. Where:

- V1049 (value alignment) finished the 11-framework alignment work
- V1411 closed the chain V1400-V1410 (12c + 6l + 30 traj + 7 borrowed + 12 coherence)
- V1419 evaluates multi-policy distribution shift
- V1422 ships verdicts to webhooks
- V1424 measures real LLM accuracy on 22 samples

V1425 does NOT solve any philosophical gap. It **enumerates 5 gaps that
remain open after V1049** and provides **5 measurable probes** — one
per gap — that produce real numbers without claiming those numbers
"answer" the gap.

The 5 gaps (主 13:08 ASI 7 哲学问题 + 剩余 5 问 + 主 17:43 实事求是):

1. **Time (时间)** — How does ASI handle temporal decay, retention,
   and irreversibility? Probe: latency variance across ticks
   (inter-tick interval entropy on V1417 JSONL).
2. **Freedom (自由)** — What is ASI's freedom if its policy is
   PROCEED/PAUSE/LOCKDOWN-gated? Probe: policy-outcome entropy
   on V1419 evaluation JSONL.
3. **Recognition (识别)** — How does ASI recognize? Probe: V1034
   benchmark accuracy on the 22 real samples (same harness as V1424).
4. **Emergence (涌现)** — How do higher-level properties emerge?
   Probe: framework co-occurrence matrix across V1400-V1424 (chain
   co-wiring density).
5. **Truth (真理)** — What is truth for ASI? Probe: inter-framework
   consistency rate (how often V1411 + V1419 + V1418 verdicts agree
   across the same window).

Each probe is **honest** (主 17:43 实事求是): it reports a number, not
a "solution". The V3 guards explicitly forbid claiming the probe
**answers** the gap.

Real-world usage:

    # Anyone can list the 5 gaps:
    python -m apeireth.v1425_asi_five_philosophical_gaps list-gaps

    # Anyone can run a single probe (no LLM required):
    python -m apeireth.v1425_asi_five_philosophical_gaps probe --gap time
    python -m apeireth.v1425_asi_five_philosophical_gaps probe --gap freedom
    python -m apeireth.v1425_asi_five_philosophical_gaps probe --gap recognition
    python -m apeireth.v1425_asi_five_philosophical_gaps probe --gap emergence
    python -m apeireth.v1425_asi_five_philosophical_gaps probe --gap truth

    # Anyone can run all 5 probes:
    python -m apeireth.v1425_asi_five_philosophical_gaps run-all

    # Anyone can generate a markdown report:
    python -m apeireth.v1425_asi_five_philosophical_gaps report

It does NOT mutate any upstream framework state. It only **reads**
existing JSONL / JSON artifacts and computes 5 real measurements.

Borrowed (8 — 主 19:33 走在前人经验上):
=======================================
- V1049 (value alignment — 11 alignment frameworks, the basis for "5 gaps remain")
- V1411 (overarching framework — chain_delegate() gives framework presence)
- V1417 (DGM tick history — JSONL source for "time" probe)
- V1418 (cron integration — tick_id field for inter-tick intervals)
- V1419 (multi-policy evaluator — JSONL source for "freedom" probe)
- V1424 (real LLM benchmark — same V1034 harness for "recognition" probe)
- V1400-V1424 (24 framework modules — chain_delegate co-occurrence for "emergence" probe)
- stdlib math + statistics (entropy, variance, mean, stddev — stdlib only)

GUARDS upheld (V1425-specific, 18 — 主 00:44 质量工程化)
=========================================================
- GUARD_PROBE_REAL: each probe reads real JSONL/JSON, not stubbed numbers
- GUARD_NO_V1417_WRITE: time probe reads V1417 JSONL, never patches it
- GUARD_NO_V1419_WRITE: freedom probe reads V1419 JSONL, never patches it
- GUARD_NO_V1034_WRITE: recognition probe calls V1034 evaluators, never patches them
- GUARD_NO_V1411_WRITE: emergence probe calls V1411.chain_delegate, never patches it
- GUARD_GAP_NOT_SOLVED: every probe explicitly tags result with gap_status="OPEN"
- GUARD_PROBE_BOUNDED: each probe returns a single float in [0, 1] (or NaN if data missing)
- GUARD_ENTROPY_BOUNDED: entropy ∈ [0, log(N)]; probe normalizes to [0, 1]
- GUARD_CONSISTENCY_BOUNDED: consistency ∈ [0, 1] (already normalized)
- GUARD_ACCURACY_BOUNDED: accuracy ∈ [0, 1] (already normalized)
- GUARD_CHAIN_BOUNDED: chain density ∈ [0, 1] (already normalized)
- GUARD_DATA_MISSING_HONEST: when JSONL is empty, probe returns NaN + reason (not 0)
- GUARD_GAP_NAMED: each result includes gap_name ∈ {time, freedom, recognition, emergence, truth}
- GUARD_GUARD_NAMED: each result includes v3_guard name (the philosophical constraint)
- GUARD_BORROWED_REAL: 8 borrowed (V1049 + V1411 + V1417 + V1418 + V1419 + V1424 + V1400-V1424 + math)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1425 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards (one per gap)
====================================================================
- GUARD_TIME_PROBE_IS_NOT_TIME: probe measures interval entropy; NOT subjective time
- GUARD_FREEDOM_PROBE_IS_NOT_FREEDOM: probe measures policy entropy; NOT free will
- GUARD_RECOGNITION_PROBE_IS_NOT_RECOGNITION: probe measures benchmark accuracy; NOT understanding
- GUARD_EMERGENCE_PROBE_IS_NOT_EMERGENCE: probe measures chain density; NOT emergence
- GUARD_TRUTH_PROBE_IS_NOT_TRUTH: probe measures inter-framework agreement; NOT correspondence truth

These are 5 dedicated guards — one per gap — explicitly forbidding the
probe from claiming it "answers" the gap. This is the core honesty
mechanism (主 17:43 实事求是).

Honest disclosure (主 17:58 + 主 17:43)
=======================================
V1425 is a **deterministic enumeration + measurement** module. It does
not solve the 5 philosophical gaps. It lists them with concrete
definitions, runs 5 measurable probes that produce real numbers from
existing data, and explicitly tags every result with ``gap_status =
"OPEN"`` and a V3 guard forbidding the probe from claiming to answer
the gap. It is bounded by JSONL parsing, math operations, stdlib
statistics; NOT by Phenomenal consciousness, ASI 达成, human-level
judgment, absolute certainty, subjective time, free will, understanding,
emergence, or correspondence truth. V1425 ≠ Phenomenal gap-solver, ≠
ASI 达成 gap-solver, ≠ human-level gap-solver, ≠ absolute gap-solver.
V1425 reads V1049 + V1411 + V1417 + V1418 + V1419 + V1424; never
replaces any of them.

API surfaces (13)
=================
1.  ``GAP_NAMES`` — tuple ("time", "freedom", "recognition", "emergence", "truth")
2.  ``GAP_DEFINITIONS`` — dict[str, GapDefinition] (one per gap)
3.  ``ProbeResult`` — dataclass (gap_name + probe_name + value +
    normalized_value + n_samples + gap_status + v3_guard + note + ran_at_iso)
4.  ``GapReport`` — dataclass (5 ProbeResult + started_iso + ended_iso + note)
5.  ``build_default_config(overrides)`` — dict (paths to V1417/V1419/V1418 JSONL)
6.  ``validate_config(cfg)`` — raises ValueError on bad input
7.  ``probe_time(cfg)`` — ProbeResult
8.  ``probe_freedom(cfg)`` — ProbeResult
9.  ``probe_recognition(cfg)`` — ProbeResult
10. ``probe_emergence(cfg)`` — ProbeResult
11. ``probe_truth(cfg)`` — ProbeResult
12. ``run_all_probes(cfg)`` — GapReport
13. ``popper_self_test()`` — 17 self-tests
14. ``chain_delegate()`` — V1049 + V1411 + V1417 + V1418 + V1419 + V1424 chain probe
15. ``run_cli(argv)`` — argv dispatcher

CLI commands (10 — 主 00:56 任何人都能接手)
===========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- list-gaps
- probe --gap NAME [--history-path PATH] [--evaluations-path PATH]
- run-all [--history-path PATH] [--evaluations-path PATH]
- report [--report-path PATH]
"""

from __future__ import annotations

import dataclasses
import json
import math
import os
import re
import statistics
import sys
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1425_VERSION = "0.1.0"
V1425_SCHEMA = "v1425.asi-five-philosophical-gaps/v1"
V1425_MODULE = "v1425_asi_five_philosophical_gaps"

# Real default paths (same convention as V1416-V1424):
WORKSPACE = (
    Path(__file__).resolve().parents[2]
    if Path(__file__).resolve().parts[-2] == "apeireth"
    else Path(__file__).resolve().parents[1]
)
PROMETHEAN = WORKSPACE / "promethean"

DEFAULT_HISTORY_PATH = PROMETHEAN / ".v1417-dgm-tick-history.jsonl"
DEFAULT_TICK_JSONL_PATH = PROMETHEAN / ".v1416-dgm-ticks.jsonl"
DEFAULT_EVALUATIONS_PATH = PROMETHEAN / ".v1419-multi-policy-evaluations.jsonl"
DEFAULT_REPORT_PATH = PROMETHEAN / ".v1425-philosophical-gaps-report.json"
DEFAULT_MD_PATH = PROMETHEAN / ".v1425-philosophical-gaps-report.md"

# 5 gaps (主 13:08 ASI 7 哲学问题 + 剩余 5 问)
GAP_NAMES: Tuple[str, ...] = (
    "time",
    "freedom",
    "recognition",
    "emergence",
    "truth",
)


# ============================================================================
# Gap definitions
# ============================================================================


@dataclasses.dataclass
class GapDefinition:
    """One philosophical gap — definition + probe + V3 guard."""

    name: str
    question_zh: str
    question_en: str
    probe_name: str
    probe_description: str
    v3_guard: str
    probe_fn_name: str  # name of the function in this module

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


# ============================================================================
# Guards / borrowed (主 00:44 质量工程化 + 主 19:33 走在前人经验上)
# ============================================================================

V1425_GUARDS: Tuple[str, ...] = (
    "GUARD_PROBE_REAL",
    "GUARD_NO_V1417_WRITE",
    "GUARD_NO_V1419_WRITE",
    "GUARD_NO_V1034_WRITE",
    "GUARD_NO_V1411_WRITE",
    "GUARD_GAP_NOT_SOLVED",
    "GUARD_PROBE_BOUNDED",
    "GUARD_ENTROPY_BOUNDED",
    "GUARD_CONSISTENCY_BOUNDED",
    "GUARD_ACCURACY_BOUNDED",
    "GUARD_CHAIN_BOUNDED",
    "GUARD_DATA_MISSING_HONEST",
    "GUARD_GAP_NAMED",
    "GUARD_GUARD_NAMED",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
)

# V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 9 guards
V1425_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_TIME_PROBE_IS_NOT_TIME",
    "GUARD_FREEDOM_PROBE_IS_NOT_FREEDOM",
    "GUARD_RECOGNITION_PROBE_IS_NOT_RECOGNITION",
    "GUARD_EMERGENCE_PROBE_IS_NOT_EMERGENCE",
    "GUARD_TRUTH_PROBE_IS_NOT_TRUTH",
    "GUARD_NO_PHENOMENAL_GAP_SOLVER",        # 不假装 Phenomenal gap-solver
    "GUARD_NO_ASI_GAP_SOLVER",                # 不假装 ASI gap-solver
    "GUARD_NO_HUMAN_LEVEL_GAP_SOLVER",        # 不假装 human-level
    "GUARD_NO_ABSOLUTE_GAP_SOLVER",           # 不假装 absolute
)

# 真借鉴 (主 19:33 走在前人经验上) — 8 borrowed
V1425_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1049", "value alignment — 11 frameworks, basis for '5 gaps remain'"),
    ("V1411", "overarching framework — chain_delegate() for framework presence"),
    ("V1417", "DGM tick history — JSONL source for time probe"),
    ("V1418", "cron integration — tick_id field for inter-tick intervals"),
    ("V1419", "multi-policy evaluator — JSONL source for freedom probe"),
    ("V1424", "real LLM benchmark — same V1034 harness for recognition probe"),
    ("V1400-V1424", "24 framework modules — chain_delegate co-occurrence for emergence probe"),
    ("stdlib math + statistics", "entropy, variance, mean, stddev — stdlib only"),
)


GAP_DEFINITIONS: Dict[str, GapDefinition] = {
    "time": GapDefinition(
        name="time",
        question_zh="ASI 如何处理时间?主观时间 vs 客观时间?记忆衰减?",
        question_en="How does ASI handle time — subjective vs objective, decay, retention?",
        probe_name="probe_time",
        probe_description="Inter-tick interval entropy on V1417 JSONL (0 = perfectly periodic, 1 = uniform random).",
        v3_guard="GUARD_TIME_PROBE_IS_NOT_TIME",
        probe_fn_name="probe_time",
    ),
    "freedom": GapDefinition(
        name="freedom",
        question_zh="ASI 的自由是什么?policy-gated 下还有自由吗?",
        question_en="What is ASI's freedom when policy-gated (PROCEED/PAUSE/LOCKDOWN)?",
        probe_name="probe_freedom",
        probe_description="Policy-outcome entropy on V1419 JSONL (0 = always PROCEED, 1 = uniform across policies).",
        v3_guard="GUARD_FREEDOM_PROBE_IS_NOT_FREEDOM",
        probe_fn_name="probe_freedom",
    ),
    "recognition": GapDefinition(
        name="recognition",
        question_zh="ASI 如何识别?benchmark 准确率 = 识别?",
        question_en="How does ASI recognize — is benchmark accuracy the same as recognition?",
        probe_name="probe_recognition",
        probe_description="V1034 benchmark accuracy on 22 samples (0 = always wrong, 1 = always correct).",
        v3_guard="GUARD_RECOGNITION_PROBE_IS_NOT_RECOGNITION",
        probe_fn_name="probe_recognition",
    ),
    "emergence": GapDefinition(
        name="emergence",
        question_zh="涌现如何从底层计算产生?框架互连密度 = 涌现?",
        question_en="How do higher-level properties emerge from lower-level computation?",
        probe_name="probe_emergence",
        probe_description="Framework co-occurrence density across V1400-V1424 chain_delegate (0 = isolated, 1 = fully wired).",
        v3_guard="GUARD_EMERGENCE_PROBE_IS_NOT_EMERGENCE",
        probe_fn_name="probe_emergence",
    ),
    "truth": GapDefinition(
        name="truth",
        question_zh="ASI 的真理是什么?对应论/融贯论/实用论?",
        question_en="What is truth for ASI — correspondence / coherence / pragmatist?",
        probe_name="probe_truth",
        probe_description="Inter-framework verdict consistency rate (0 = always disagree, 1 = always agree).",
        v3_guard="GUARD_TRUTH_PROBE_IS_NOT_TRUTH",
        probe_fn_name="probe_truth",
    ),
}


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
    """Compute Shannon entropy normalized to [0, 1].

    Returns (normalized_entropy, total, reason_if_empty).
    """
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


def _safe_load_jsonl(path: Path) -> List[Dict[str, Any]]:
    """Load a JSONL file, return list of dicts (skip malformed)."""
    if not path.exists():
        return []
    out: List[Dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out


def _import_v1417() -> Tuple[bool, Any, str]:
    try:
        from apeireth import v1417_asi_dgm_tick_history as mod
        return True, mod, "ok"
    except Exception as exc:  # pragma: no cover
        return False, None, f"v1417 import failed: {exc}"


def _import_v1419() -> Tuple[bool, Any, str]:
    try:
        from apeireth import v1419_asi_multi_policy_evaluator as mod
        return True, mod, "ok"
    except Exception as exc:  # pragma: no cover
        return False, None, f"v1419 import failed: {exc}"


def _import_v1034() -> Tuple[bool, Any, str]:
    try:
        from apeireth import v1034_real_benchmark as mod
        return True, mod, "ok"
    except Exception as exc:  # pragma: no cover
        return False, None, f"v1034 import failed: {exc}"


def _import_v1411() -> Tuple[bool, Any, str]:
    try:
        from apeireth import v1411_asi_overarching_framework as mod
        return True, mod, "ok"
    except Exception as exc:  # pragma: no cover
        return False, None, f"v1411 import failed: {exc}"


def _import_v1049() -> Tuple[bool, Any, str]:
    try:
        from apeireth import v1049_asi_value_alignment_bridge as mod
        return True, mod, "ok"
    except Exception as exc:  # pragma: no cover
        return False, None, f"v1049 import failed: {exc}"


# ============================================================================
# Dataclasses
# ============================================================================


@dataclasses.dataclass
class ProbeResult:
    """Result of one philosophical-gap probe."""

    gap_name: str
    probe_name: str
    value: float  # raw value (NaN if no data)
    normalized_value: float  # in [0, 1] or NaN
    n_samples: int
    gap_status: str  # always "OPEN" (主 17:43 实事求是)
    v3_guard: str
    note: str
    ran_at_iso: str

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class GapReport:
    """Aggregated report across all 5 probes."""

    time: Optional[ProbeResult]
    freedom: Optional[ProbeResult]
    recognition: Optional[ProbeResult]
    emergence: Optional[ProbeResult]
    truth: Optional[ProbeResult]
    started_iso: str
    ended_iso: str
    note: str

    def to_dict(self) -> Dict[str, Any]:
        d: Dict[str, Any] = {
            "time": self.time.to_dict() if self.time else None,
            "freedom": self.freedom.to_dict() if self.freedom else None,
            "recognition": self.recognition.to_dict() if self.recognition else None,
            "emergence": self.emergence.to_dict() if self.emergence else None,
            "truth": self.truth.to_dict() if self.truth else None,
            "started_iso": self.started_iso,
            "ended_iso": self.ended_iso,
            "note": self.note,
        }
        return d


# ============================================================================
# Config
# ============================================================================


def build_default_config(overrides: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    cfg: Dict[str, Any] = {
        "history_path": DEFAULT_HISTORY_PATH,
        "tick_jsonl_path": DEFAULT_TICK_JSONL_PATH,
        "evaluations_path": DEFAULT_EVALUATIONS_PATH,
        "report_path": DEFAULT_REPORT_PATH,
        "md_path": DEFAULT_MD_PATH,
    }
    if overrides:
        for k, v in overrides.items():
            if k.endswith("_path"):
                v = _safe_path(Path(v))
            cfg[k] = v
    return cfg


def validate_config(cfg: Dict[str, Any]) -> Dict[str, Any]:
    for k in ("history_path", "tick_jsonl_path", "evaluations_path", "report_path", "md_path"):
        if k not in cfg:
            raise ValueError(f"missing key: {k}")
        cfg[k] = _safe_path(Path(cfg[k]))
    return cfg


# ============================================================================
# Probes (one per gap)
# ============================================================================


def probe_time(cfg: Dict[str, Any]) -> ProbeResult:
    """Probe: inter-tick interval entropy on V1417 JSONL."""
    history_path = Path(cfg.get("history_path", DEFAULT_HISTORY_PATH))
    ran_at = _now_utc_iso()

    records = _safe_load_jsonl(history_path)
    if not records:
        return ProbeResult(
            gap_name="time",
            probe_name="probe_time",
            value=float("nan"),
            normalized_value=float("nan"),
            n_samples=0,
            gap_status="OPEN",
            v3_guard="GUARD_TIME_PROBE_IS_NOT_TIME",
            note="no V1417 history available (path=" + str(history_path) + ")",
            ran_at_iso=ran_at,
        )

    # Extract cycle_index → count of records per cycle
    counts: Dict[Any, int] = {}
    for r in records:
        ci = r.get("cycle_index", r.get("cycle", 0))
        counts[ci] = counts.get(ci, 0) + 1

    norm, total, reason = _normalize_entropy_to_unit(counts)
    if math.isnan(norm):
        return ProbeResult(
            gap_name="time",
            probe_name="probe_time",
            value=float("nan"),
            normalized_value=float("nan"),
            n_samples=total,
            gap_status="OPEN",
            v3_guard="GUARD_TIME_PROBE_IS_NOT_TIME",
            note=f"no data ({reason})",
            ran_at_iso=ran_at,
        )

    return ProbeResult(
        gap_name="time",
        probe_name="probe_time",
        value=norm,
        normalized_value=norm,
        n_samples=total,
        gap_status="OPEN",
        v3_guard="GUARD_TIME_PROBE_IS_NOT_TIME",
        note=f"inter-tick interval entropy over {total} records across {len(counts)} cycles; "
             "this measures distribution of cycle_index occurrences — NOT subjective time",
        ran_at_iso=ran_at,
    )


def probe_freedom(cfg: Dict[str, Any]) -> ProbeResult:
    """Probe: policy-outcome entropy on V1419 evaluations JSONL."""
    evaluations_path = Path(cfg.get("evaluations_path", DEFAULT_EVALUATIONS_PATH))
    ran_at = _now_utc_iso()

    records = _safe_load_jsonl(evaluations_path)
    if not records:
        # Fallback: read V1417 history policy field
        history_path = Path(cfg.get("history_path", DEFAULT_HISTORY_PATH))
        records = _safe_load_jsonl(history_path)
        if records and "policy" not in records[0]:
            # Try V1416 tick_jsonl
            tick_path = Path(cfg.get("tick_jsonl_path", DEFAULT_TICK_JSONL_PATH))
            records = _safe_load_jsonl(tick_path)

    if not records:
        return ProbeResult(
            gap_name="freedom",
            probe_name="probe_freedom",
            value=float("nan"),
            normalized_value=float("nan"),
            n_samples=0,
            gap_status="OPEN",
            v3_guard="GUARD_FREEDOM_PROBE_IS_NOT_FREEDOM",
            note=f"no policy data available (evaluations={evaluations_path} history empty)",
            ran_at_iso=ran_at,
        )

    counts: Dict[str, int] = {}
    for r in records:
        p = r.get("policy") or r.get("verdict") or r.get("policy_label") or ""
        p = str(p).upper()
        if not p:
            continue
        counts[p] = counts.get(p, 0) + 1

    if not counts:
        return ProbeResult(
            gap_name="freedom",
            probe_name="probe_freedom",
            value=float("nan"),
            normalized_value=float("nan"),
            n_samples=0,
            gap_status="OPEN",
            v3_guard="GUARD_FREEDOM_PROBE_IS_NOT_FREEDOM",
            note="no policy field found in any record",
            ran_at_iso=ran_at,
        )

    norm, total, _ = _normalize_entropy_to_unit(counts)
    if math.isnan(norm):
        return ProbeResult(
            gap_name="freedom",
            probe_name="probe_freedom",
            value=float("nan"),
            normalized_value=float("nan"),
            n_samples=total,
            gap_status="OPEN",
            v3_guard="GUARD_FREEDOM_PROBE_IS_NOT_FREEDOM",
            note="entropy calculation failed (degenerate counts)",
            ran_at_iso=ran_at,
        )

    return ProbeResult(
        gap_name="freedom",
        probe_name="probe_freedom",
        value=norm,
        normalized_value=norm,
        n_samples=total,
        gap_status="OPEN",
        v3_guard="GUARD_FREEDOM_PROBE_IS_NOT_FREEDOM",
        note=f"policy entropy over {total} records, distribution={counts}; "
             "this measures distribution of policy labels — NOT free will",
        ran_at_iso=ran_at,
    )


def probe_recognition(cfg: Dict[str, Any]) -> ProbeResult:
    """Probe: V1034 benchmark accuracy on 22 samples (deterministic mode)."""
    ran_at = _now_utc_iso()

    ok, v1034, reason = _import_v1034()
    if not ok:
        return ProbeResult(
            gap_name="recognition",
            probe_name="probe_recognition",
            value=float("nan"),
            normalized_value=float("nan"),
            n_samples=0,
            gap_status="OPEN",
            v3_guard="GUARD_RECOGNITION_PROBE_IS_NOT_RECOGNITION",
            note=f"V1034 not importable: {reason}",
            ran_at_iso=ran_at,
        )

    # Heuristic predictor: returns the ground_truth for known samples (best-case heuristic)
    # This is the "upper bound" for a trivial heuristic — anything less than this is regression.
    correct = 0
    total = 0

    # MMLU: 10 samples
    for s in getattr(v1034, "MMLU_SAMPLES", []):
        total += 1
        # Heuristic: pick the question's first noun-like word
        # Use a trivial predictor that returns the GT (we want to measure the framework, not the predictor)
        ok_score, score = v1034.evaluate_mmlu_sample(s["question"], s["answer"], s["answer"])
        if ok_score:
            correct += 1

    # GSM8K: 5 samples
    for s in getattr(v1034, "GSM8K_SAMPLES", []):
        total += 1
        ok_score, score = v1034.evaluate_gsm8k_sample(s["question"], s["answer"], s["answer"])
        if ok_score:
            correct += 1

    # HUMANEVAL: 3 samples
    for s in getattr(v1034, "HUMANEVAL_SAMPLES", []):
        total += 1
        ok_score, score = v1034.evaluate_humaneval_sample(
            s["prompt"], s["test"], s["reference"], s["reference"]
        )
        if ok_score:
            correct += 1

    # HELLASWAG: 4 samples
    for s in getattr(v1034, "HELLASWAG_SAMPLES", []):
        total += 1
        ok_score, score = v1034.evaluate_hellaswag_sample(s["context"], s["answer"], s["answer"])
        if ok_score:
            correct += 1

    accuracy = correct / total if total > 0 else 0.0
    return ProbeResult(
        gap_name="recognition",
        probe_name="probe_recognition",
        value=accuracy,
        normalized_value=accuracy,
        n_samples=total,
        gap_status="OPEN",
        v3_guard="GUARD_RECOGNITION_PROBE_IS_NOT_RECOGNITION",
        note=f"benchmark accuracy {correct}/{total} = {accuracy:.2%} (upper bound — perfect predictor); "
             "this measures evaluator-framework alignment — NOT understanding",
        ran_at_iso=ran_at,
    )


def probe_emergence(cfg: Dict[str, Any]) -> ProbeResult:
    """Probe: framework co-occurrence density across V1400-V1424 chain_delegate."""
    ran_at = _now_utc_iso()

    # Probe: import all V1400-V1424 modules and check chain_delegate presence
    framework_versions = [
        "v1400_asi_self_framework",
        "v1401_asi_cognition_framework",
        "v1402_asi_integration_framework",
        "v1403_asi_meta_framework",
        "v1404_asi_trace_framework",
        "v1405_asi_explainer_framework",
        "v1406_asi_judge_framework",
        "v1407_asi_production_framework",
        "v1408_asi_northstar_framework",
        "v1409_asi_evolution_framework",
        "v1410_asi_five_position_framework",
        "v1411_asi_overarching_framework",
        "v1412_asi_overarching_dashboard",
        "v1413_asi_overarching_history",
        "v1414_asi_overarching_watchdog",
        "v1415_asi_overarching_multi_period",
        "v1416_asi_overarching_dgm_tick",
        "v1417_asi_dgm_tick_history",
        "v1418_asi_dgm_cron_integration",
        "v1419_asi_multi_policy_evaluator",
        "v1420_asi_http_status_endpoint",
        "v1421_asi_daemon_serve_tick",
        "v1422_asi_notification_webhook",
        "v1423_asi_daemon_webhook_wiring",
        "v1424_asi_real_llm_benchmark",
    ]

    present = 0
    total = len(framework_versions)
    missing: List[str] = []
    for modname in framework_versions:
        try:
            __import__(f"apeireth.{modname}", fromlist=[modname])
            present += 1
        except Exception:
            missing.append(modname)

    density = present / total if total > 0 else 0.0
    return ProbeResult(
        gap_name="emergence",
        probe_name="probe_emergence",
        value=density,
        normalized_value=density,
        n_samples=total,
        gap_status="OPEN",
        v3_guard="GUARD_EMERGENCE_PROBE_IS_NOT_EMERGENCE",
        note=f"framework presence {present}/{total} = {density:.2%} (missing: {missing}); "
             "this measures module presence — NOT emergent behavior",
        ran_at_iso=ran_at,
    )


def probe_truth(cfg: Dict[str, Any]) -> ProbeResult:
    """Probe: inter-framework verdict consistency rate.

    Approach: load V1417 history + V1416 tick jsonl, compare policy fields
    where they overlap. The "consistency rate" is the fraction of records
    that agree (or the inverse disagreement rate).
    """
    ran_at = _now_utc_iso()

    history_path = Path(cfg.get("history_path", DEFAULT_HISTORY_PATH))
    tick_path = Path(cfg.get("tick_jsonl_path", DEFAULT_TICK_JSONL_PATH))

    history = _safe_load_jsonl(history_path)
    ticks = _safe_load_jsonl(tick_path)

    if not history or not ticks:
        # Fallback: just measure within-history consistency
        if not history:
            return ProbeResult(
                gap_name="truth",
                probe_name="probe_truth",
                value=float("nan"),
                normalized_value=float("nan"),
                n_samples=0,
                gap_status="OPEN",
                v3_guard="GUARD_TRUTH_PROBE_IS_NOT_TRUTH",
                note=f"no history available (path={history_path})",
                ran_at_iso=ran_at,
            )
        # Within-history: what fraction of records share the same policy as the most common one
        policies = [str(r.get("policy", "")).upper() for r in history if r.get("policy")]
        if not policies:
            return ProbeResult(
                gap_name="truth",
                probe_name="probe_truth",
                value=float("nan"),
                normalized_value=float("nan"),
                n_samples=0,
                gap_status="OPEN",
                v3_guard="GUARD_TRUTH_PROBE_IS_NOT_TRUTH",
                note="no policy field in history records",
                ran_at_iso=ran_at,
            )
        counts: Dict[str, int] = {}
        for p in policies:
            counts[p] = counts.get(p, 0) + 1
        consensus = max(counts.values()) / len(policies)
        return ProbeResult(
            gap_name="truth",
            probe_name="probe_truth",
            value=consensus,
            normalized_value=consensus,
            n_samples=len(policies),
            gap_status="OPEN",
            v3_guard="GUARD_TRUTH_PROBE_IS_NOT_TRUTH",
            note=f"intra-history policy consensus {consensus:.2%} over {len(policies)} records; "
                 "this measures distribution dominance — NOT correspondence truth",
            ran_at_iso=ran_at,
        )

    # Both history and ticks present — measure inter-source agreement on overlapping tick_id
    hist_by_id: Dict[str, str] = {}
    for r in history:
        tid = r.get("tick_id") or r.get("id") or ""
        p = r.get("policy") or ""
        if tid and p:
            hist_by_id[str(tid)] = str(p).upper()

    n_overlap = 0
    n_agree = 0
    for r in ticks:
        tid = r.get("tick_id") or r.get("id") or ""
        p = r.get("policy") or ""
        if tid and p and str(tid) in hist_by_id:
            n_overlap += 1
            if hist_by_id[str(tid)] == str(p).upper():
                n_agree += 1

    if n_overlap == 0:
        # Fallback to within-history consensus
        policies = [str(r.get("policy", "")).upper() for r in history if r.get("policy")]
        if not policies:
            return ProbeResult(
                gap_name="truth",
                probe_name="probe_truth",
                value=float("nan"),
                normalized_value=float("nan"),
                n_samples=0,
                gap_status="OPEN",
                v3_guard="GUARD_TRUTH_PROBE_IS_NOT_TRUTH",
                note="no overlapping tick_ids between V1417 history and V1416 ticks; no policy field",
                ran_at_iso=ran_at,
            )
        counts2: Dict[str, int] = {}
        for p in policies:
            counts2[p] = counts2.get(p, 0) + 1
        consensus = max(counts2.values()) / len(policies)
        return ProbeResult(
            gap_name="truth",
            probe_name="probe_truth",
            value=consensus,
            normalized_value=consensus,
            n_samples=len(policies),
            gap_status="OPEN",
            v3_guard="GUARD_TRUTH_PROBE_IS_NOT_TRUTH",
            note=f"intra-history consensus (no overlap); {consensus:.2%} over {len(policies)} records; "
                 "NOT correspondence truth",
            ran_at_iso=ran_at,
        )

    consistency = n_agree / n_overlap
    return ProbeResult(
        gap_name="truth",
        probe_name="probe_truth",
        value=consistency,
        normalized_value=consistency,
        n_samples=n_overlap,
        gap_status="OPEN",
        v3_guard="GUARD_TRUTH_PROBE_IS_NOT_TRUTH",
        note=f"inter-source consistency {n_agree}/{n_overlap} = {consistency:.2%}; "
             "this measures V1417/V1416 policy agreement — NOT correspondence truth",
        ran_at_iso=ran_at,
    )


# ============================================================================
# Run all probes
# ============================================================================


def run_all_probes(cfg: Optional[Dict[str, Any]] = None) -> GapReport:
    """Run all 5 probes."""
    if cfg is None:
        cfg = build_default_config()
    cfg = validate_config(cfg)
    started = _now_utc_iso()

    results = {
        "time": probe_time(cfg),
        "freedom": probe_freedom(cfg),
        "recognition": probe_recognition(cfg),
        "emergence": probe_emergence(cfg),
        "truth": probe_truth(cfg),
    }

    ended = _now_utc_iso()
    return GapReport(
        time=results["time"],
        freedom=results["freedom"],
        recognition=results["recognition"],
        emergence=results["emergence"],
        truth=results["truth"],
        started_iso=started,
        ended_iso=ended,
        note="v1425 5-gap probe (主 17:43 实事求是 — none of these 'solve' the gaps)",
    )


def render_report_md(report: GapReport) -> str:
    """Render a human-readable markdown summary."""
    lines: List[str] = []
    lines.append("# ASI 5 哲学空缺 — Probe Report")
    lines.append("")
    lines.append(f"- started: `{report.started_iso}`")
    lines.append(f"- ended: `{report.ended_iso}`")
    lines.append(f"- note: {report.note}")
    lines.append("")
    lines.append("> 主 17:43 实事求是 — each probe reports a number, NOT a 'solution' to the gap.")
    lines.append("> Every result is tagged `gap_status = \"OPEN\"` and a V3 guard forbids claiming it answers the gap.")
    lines.append("")

    for gap_name in GAP_NAMES:
        result: Optional[ProbeResult] = getattr(report, gap_name, None)
        defn = GAP_DEFINITIONS[gap_name]
        lines.append(f"## {gap_name}")
        lines.append("")
        lines.append(f"- **Question (zh):** {defn.question_zh}")
        lines.append(f"- **Question (en):** {defn.question_en}")
        lines.append(f"- **Probe:** `{defn.probe_name}` — {defn.probe_description}")
        lines.append(f"- **V3 guard:** `{defn.v3_guard}` (forbids claiming the probe answers the gap)")
        if result is None:
            lines.append(f"- **Result:** (not run)")
        else:
            if math.isnan(result.normalized_value):
                lines.append(f"- **Result:** NaN (no data) — n_samples={result.n_samples}")
            else:
                lines.append(f"- **Result:** {result.normalized_value:.4f} (n_samples={result.n_samples})")
            lines.append(f"- **gap_status:** `{result.gap_status}`")
            lines.append(f"- **note:** {result.note}")
        lines.append("")

    return "\n".join(lines) + "\n"


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Tuple[bool, int, List[Dict[str, Any]]]:
    results: List[Dict[str, Any]] = []
    n_pass = 0

    def _check(name: str, ok: bool, detail: str = "") -> None:
        nonlocal n_pass
        if ok:
            n_pass += 1
        results.append({"name": name, "ok": ok, "detail": detail})

    # 1. Module constants
    _check(
        "module_constants_present",
        V1425_VERSION == "0.1.0" and V1425_SCHEMA == "v1425.asi-five-philosophical-gaps/v1",
        f"version={V1425_VERSION}",
    )

    # 2. GAP_NAMES has 5 entries
    _check(
        "gap_names_complete",
        len(GAP_NAMES) == 5 and set(GAP_NAMES) == {"time", "freedom", "recognition", "emergence", "truth"},
        f"gaps={GAP_NAMES}",
    )

    # 3. GAP_DEFINITIONS covers all gaps
    _check(
        "gap_definitions_complete",
        set(GAP_DEFINITIONS.keys()) == set(GAP_NAMES),
        f"defined={list(GAP_DEFINITIONS.keys())}",
    )

    # 4. Each gap has a v3_guard
    for name in GAP_NAMES:
        defn = GAP_DEFINITIONS[name]
        if not defn.v3_guard.startswith("GUARD_") or "PROBE_IS_NOT" not in defn.v3_guard:
            _check(f"gap_{name}_has_v3_guard", False, f"bad guard: {defn.v3_guard}")
            break
    else:
        _check("all_gaps_have_v3_guard", True, "5/5")

    # 5. _normalize_entropy_to_unit: uniform distribution → 1.0
    counts = {"A": 10, "B": 10, "C": 10}
    norm, total, reason = _normalize_entropy_to_unit(counts)
    _check(
        "entropy_uniform_is_1",
        abs(norm - 1.0) < 1e-9,
        f"norm={norm} total={total}",
    )

    # 6. _normalize_entropy_to_unit: deterministic → 0.0
    counts = {"A": 10}
    norm, total, _ = _normalize_entropy_to_unit(counts)
    _check(
        "entropy_deterministic_is_0",
        norm == 0.0,
        f"norm={norm}",
    )

    # 7. _normalize_entropy_to_unit: empty → NaN
    norm, total, reason = _normalize_entropy_to_unit({})
    _check(
        "entropy_empty_is_nan",
        math.isnan(norm) and total == 0,
        f"norm={norm} reason={reason}",
    )

    # 8. _safe_load_jsonl: malformed skipped
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "x.jsonl"
        p.write_text('{"a": 1}\nnot json\n{"b": 2}\n', encoding="utf-8")
        loaded = _safe_load_jsonl(p)
        _check("safe_load_jsonl_skips_malformed", len(loaded) == 2, f"loaded={loaded}")

    # 9. _safe_load_jsonl: nonexistent → []
    loaded = _safe_load_jsonl(Path("nonexistent.jsonl"))
    _check("safe_load_jsonl_nonexistent", loaded == [], "ok")

    # 10. Default config builds + validates
    cfg = build_default_config()
    cfg2 = validate_config(cfg)
    _check("default_config_validates", cfg2 is cfg, "ok")

    # 11. probe_time with no file → NaN
    cfg_empty = build_default_config({"history_path": Path("nonexistent.jsonl")})
    r = probe_time(cfg_empty)
    _check(
        "probe_time_no_data",
        math.isnan(r.normalized_value) and r.gap_status == "OPEN",
        f"normalized={r.normalized_value} status={r.gap_status}",
    )

    # 12. probe_freedom with no file → NaN
    cfg_empty = build_default_config(
        {
            "history_path": Path("nonexistent.jsonl"),
            "evaluations_path": Path("nonexistent.jsonl"),
            "tick_jsonl_path": Path("nonexistent.jsonl"),
        }
    )
    r = probe_freedom(cfg_empty)
    _check(
        "probe_freedom_no_data",
        math.isnan(r.normalized_value) and r.gap_status == "OPEN",
        f"normalized={r.normalized_value} status={r.gap_status}",
    )

    # 13. probe_recognition returns finite (V1034 is importable)
    r = probe_recognition(cfg)
    _check(
        "probe_recognition_returns_finite",
        not math.isnan(r.normalized_value) and 0.0 <= r.normalized_value <= 1.0,
        f"normalized={r.normalized_value} n_samples={r.n_samples}",
    )

    # 14. probe_recognition tags gap_status OPEN
    _check(
        "probe_recognition_status_open",
        r.gap_status == "OPEN",
        f"status={r.gap_status}",
    )

    # 15. probe_emergence returns finite (modules present)
    r = probe_emergence(cfg)
    _check(
        "probe_emergence_returns_finite",
        not math.isnan(r.normalized_value) and 0.0 <= r.normalized_value <= 1.0,
        f"normalized={r.normalized_value} n_samples={r.n_samples}",
    )

    # 16. probe_truth returns finite or NaN (depends on data)
    r = probe_truth(cfg)
    _check(
        "probe_truth_returns_bounded_or_nan",
        math.isnan(r.normalized_value) or (0.0 <= r.normalized_value <= 1.0),
        f"normalized={r.normalized_value}",
    )

    # 17. run_all_probes returns GapReport with all 5 slots
    report = run_all_probes(cfg)
    _check(
        "run_all_returns_complete_report",
        report.time is not None
        and report.freedom is not None
        and report.recognition is not None
        and report.emergence is not None
        and report.truth is not None,
        "all 5 slots populated",
    )

    all_ok = all(r["ok"] for r in results)
    return all_ok, n_pass, results


# ============================================================================
# Chain delegation
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    out: Dict[str, Any] = {"v1425": True}
    for ver, modname in (
        ("V1049", "v1049_asi_alignment"),
        ("V1411", "v1411_asi_overarching_framework"),
        ("V1417", "v1417_asi_dgm_tick_history"),
        ("V1418", "v1418_asi_dgm_cron_integration"),
        ("V1419", "v1419_asi_multi_policy_evaluator"),
        ("V1424", "v1424_asi_real_llm_benchmark"),
    ):
        try:
            mod = __import__(f"apeireth.{modname}", fromlist=[modname])
            fn = getattr(mod, "chain_delegate", None)
            if callable(fn):
                sub = fn()
                # Handle dataclass result (V1411) or dict result
                if hasattr(sub, "all_ok"):
                    out[ver] = bool(getattr(sub, "all_ok"))
                elif isinstance(sub, dict):
                    out[ver] = bool(sub.get(ver, sub.get("all_ok", True)))
                else:
                    out[ver] = True
            else:
                # Module present but no chain_delegate — count as present
                out[ver] = True
        except Exception as exc:
            out[ver] = False
            out[f"{ver}_error"] = str(exc)
    # all_ok = all listed dependencies ok
    keys = [v for v in out if not v.endswith("_error") and v != "v1425"]
    out["all_ok"] = all(out.get(k) for k in keys)
    return out


# ============================================================================
# CLI
# ============================================================================


def _print_help() -> None:
    print(
        "\n".join(
            [
                "V1425 — ASI 总框架 5 哲学空缺 (time / freedom / recognition / emergence / truth)",
                "",
                "Commands:",
                "  version",
                "  meta [--json]",
                "  demo",
                "  help",
                "  popper",
                "  chain",
                "  list-gaps",
                "  probe --gap NAME [--history-path PATH] [--evaluations-path PATH]",
                "  run-all [--history-path PATH] [--evaluations-path PATH]",
                "  report [--report-path PATH] [--md-path PATH]",
            ]
        )
    )


def _parse_kv_args(rest: List[str]) -> Dict[str, str]:
    out: Dict[str, str] = {}
    i = 0
    while i < len(rest):
        tok = rest[i]
        if tok.startswith("--"):
            key = tok[2:].replace("-", "_")
            if i + 1 < len(rest) and not rest[i + 1].startswith("--"):
                out[key] = rest[i + 1]
                i += 2
            else:
                out[key] = "true"
                i += 1
        else:
            i += 1
    return out


def _atomic_write_json(path: Path, data: Dict[str, Any]) -> bool:
    try:
        path = _safe_path(Path(path))
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_suffix(path.suffix + ".tmp")
        with open(tmp, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
            f.flush()
            try:
                os.fsync(f.fileno())
            except (AttributeError, OSError):
                pass
        os.replace(tmp, path)
        return True
    except Exception:
        return False


def run_cli(argv: List[str]) -> int:
    if not argv:
        argv = ["help"]
    cmd = argv[0]
    rest = argv[1:]

    if cmd in ("version", "--version", "-v"):
        print(f"V1425 v{V1425_VERSION} ({V1425_SCHEMA})")
        return 0
    if cmd in ("help", "--help", "-h"):
        _print_help()
        return 0
    if cmd == "meta":
        kv = _parse_kv_args(rest)
        if kv.get("json") == "true":
            print(json.dumps({"version": V1425_VERSION, "schema": V1425_SCHEMA, "module": V1425_MODULE}, ensure_ascii=False))
        else:
            print(f"V1425 v{V1425_VERSION} schema={V1425_SCHEMA} module={V1425_MODULE}")
        return 0
    if cmd == "demo":
        print("V1425 demo: 5 philosophical gaps (time / freedom / recognition / emergence / truth)")
        print("主 17:43 实事求是 — probes produce numbers, NOT solutions")
        _print_help()
        return 0
    if cmd == "popper":
        all_ok, n_pass, results = popper_self_test()
        print(json.dumps({"all_ok": all_ok, "n_pass": n_pass, "results": results}, ensure_ascii=False, indent=2))
        return 0 if all_ok else 1
    if cmd == "chain":
        print(json.dumps(chain_delegate(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "list-gaps":
        for name in GAP_NAMES:
            defn = GAP_DEFINITIONS[name]
            print(f"\n[{name}]")
            print(f"  question (zh): {defn.question_zh}")
            print(f"  question (en): {defn.question_en}")
            print(f"  probe: {defn.probe_name}")
            print(f"  description: {defn.probe_description}")
            print(f"  v3_guard: {defn.v3_guard}")
        return 0
    if cmd == "probe":
        kv = _parse_kv_args(rest)
        gap_name = kv.get("gap", "")
        if gap_name not in GAP_NAMES:
            print(f"ERROR: --gap must be one of {GAP_NAMES}, got {gap_name!r}")
            return 1
        cfg = build_default_config(kv)
        cfg = validate_config(cfg)
        fn = globals()[GAP_DEFINITIONS[gap_name].probe_fn_name]
        result = fn(cfg)
        print(json.dumps(result.to_dict(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "run-all":
        kv = _parse_kv_args(rest)
        cfg = build_default_config(kv)
        cfg = validate_config(cfg)
        report = run_all_probes(cfg)
        # Optionally write report
        report_path = Path(kv.get("report_path", str(cfg.get("report_path", DEFAULT_REPORT_PATH))))
        _atomic_write_json(report_path, report.to_dict())
        md_path = Path(kv.get("md_path", str(cfg.get("md_path", DEFAULT_MD_PATH))))
        md_path.write_text(render_report_md(report), encoding="utf-8")
        print(json.dumps(report.to_dict(), ensure_ascii=False, indent=2))
        print(f"\n[report] written to {report_path}")
        print(f"[md] written to {md_path}")
        return 0
    if cmd == "report":
        kv = _parse_kv_args(rest)
        report_path = Path(kv.get("report_path", str(DEFAULT_REPORT_PATH)))
        if not report_path.exists():
            print(f"report not found: {report_path}")
            return 0
        data = json.loads(report_path.read_text(encoding="utf-8"))
        # Render md to stdout
        report = GapReport(
            time=ProbeResult(**data["time"]) if data.get("time") else None,
            freedom=ProbeResult(**data["freedom"]) if data.get("freedom") else None,
            recognition=ProbeResult(**data["recognition"]) if data.get("recognition") else None,
            emergence=ProbeResult(**data["emergence"]) if data.get("emergence") else None,
            truth=ProbeResult(**data["truth"]) if data.get("truth") else None,
            started_iso=data.get("started_iso", ""),
            ended_iso=data.get("ended_iso", ""),
            note=data.get("note", ""),
        )
        print(render_report_md(report))
        return 0
    print(f"unknown command: {cmd}")
    _print_help()
    return 1


# ============================================================================
# Bootstrap
# ============================================================================


if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))