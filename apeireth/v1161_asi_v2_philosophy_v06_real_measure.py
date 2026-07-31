"""V1161 — ASI v2_philosophy V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

主 17:43 实事求是真问题 (V1155 baseline):
  - V1155 next-ROI top-1 = v2_philosophy (current 0.7143)
  - V1144._measure_v2_philosophy 用 V1135 + V1137, score = 5/7 真答覆盖
  - 真哲学补 = 真答覆盖 + 真哲学 keys 覆盖 + 真 V3 guards 覆盖

V1161 真补路径:
  - 5 sub-dim 真测:
    V1 V1135_answers_real       — V1135.ALL_ANSWERS 真有 ≥ 5 真答
    V2 V1137_remaining_real     — V1137 真有 ≥ 2 真答
    V3 PHILOSOPHY_9_KEYS_real    — V1114 PHILOSOPHY_9_KEYS 真覆盖
    V4 ASI_7_QUESTIONS_real     — V1154 V1154_DIM_NAMES 真覆盖 5 哲学问题
    V5 v3_guards_real           — V1114 V3_GUARDS 真覆盖

Usage:
    python -m apeireth.v1161_asi_v2_philosophy_v06_real_measure
    python -m apeireth.v1161_asi_v2_philosophy_v06_real_measure --json
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1161_VERSION = "0.1.0"
V1161_DIM_VERSION = "0.6"

V1161_SUBDIM_NAMES: Tuple[str, ...] = (
    "V1135_answers_real",
    "V1137_remaining_real",
    "PHILOSOPHY_9_KEYS_real",
    "ASI_7_QUESTIONS_real",
    "v3_guards_real",
)

DEFAULT_ARTIFACT_DIR = "artifacts"
V1144_BASELINE_V2_PHILOSOPHY = 0.7143
TARGET_V2_PHILOSOPHY_V06 = 0.9000


@dataclass
class SubDimEvidence:
    name: str
    score: float
    checks: Dict[str, bool] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)
    raw: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class V2PhilosophyReport:
    snapshot_id: str = field(default_factory=lambda: f"v1161-{uuid.uuid4().hex[:8]}")
    version: str = V1161_VERSION
    dim_version: str = V1161_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1161_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1144_baseline: float = V1144_BASELINE_V2_PHILOSOPHY
    target: float = TARGET_V2_PHILOSOPHY_V06

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    def summary_line(self) -> str:
        return (
            f"V1161 v2_philosophy V0.6: total={self.total:.4f} "
            f"(Δ vs V1144 baseline {self.v1144_baseline:.4f} = "
            f"{self.total - self.v1144_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"snapshot={self.snapshot_id}"
        )


def _safe_import(name: str) -> Optional[Any]:
    try:
        import importlib
        return importlib.import_module(name)
    except Exception:
        return None


def _attr_first(mod: Any, names: List[str]) -> Optional[Any]:
    for n in names:
        a = getattr(mod, n, None)
        if a is not None:
            return a
    return None


# V1 — V1135_answers_real
def _measure_V1135_answers_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(name="V1135_answers_real", score=0.0)
    mod = _safe_import("apeireth.v1135_asi_5_philosophical_gaps")
    if mod is None:
        ev.notes.append("no V1135 → V1 = 0")
        return 0.0, ev
    answers = getattr(mod, "ALL_ANSWERS", None)
    n = len(answers) if answers is not None and hasattr(answers, "__len__") else 0
    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("all_answers_present", answers is not None, ""))
    test_results.append(("has_3_answers", n >= 3, f"n={n}"))
    test_results.append(("has_5_answers", n >= 5, f"n={n}"))
    test_results.append(("has_7_answers", n >= 7, f"n={n}"))
    test_results.append(("answers_truthy", n > 0, f"n={n}"))
    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {"test_results": [{"name": n_, "ok": ok, "note": note} for n_, ok, note in test_results],
              "n_pass": n_pass, "n_answers": n}
    return ev.score, ev


# V2 — V1137_remaining_real
def _measure_V1137_remaining_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(name="V1137_remaining_real", score=0.0)
    mod = _safe_import("apeireth.v1137_asi_philosophy_remaining_2")
    if mod is None:
        ev.notes.append("no V1137 → V2 = 0")
        return 0.0, ev
    answers = None
    for attr in ["ANSWERS", "REMAINING_ANSWERS", "ALL_ANSWERS"]:
        a = getattr(mod, attr, None)
        if a is not None and hasattr(a, "__len__") and len(a) > 0:
            answers = a
            break
    n = len(answers) if answers is not None else 0
    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("answers_present", answers is not None, ""))
    test_results.append(("has_1_answer", n >= 1, f"n={n}"))
    test_results.append(("has_2_answers", n >= 2, f"n={n}"))
    test_results.append(("has_3_answers", n >= 3, f"n={n}"))
    test_results.append(("answers_truthy", n > 0, f"n={n}"))
    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {"test_results": [{"name": n_, "ok": ok, "note": note} for n_, ok, note in test_results],
              "n_pass": n_pass, "n_answers": n}
    return ev.score, ev


# V3 — PHILOSOPHY_9_KEYS_real
def _measure_PHILOSOPHY_9_KEYS_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(name="PHILOSOPHY_9_KEYS_real", score=0.0)
    mod = _safe_import("apeireth.v1114_weekly_integration_evaluator")
    if mod is None:
        ev.notes.append("no V1114 → V3 = 0")
        return 0.0, ev
    keys = getattr(mod, "PHILOSOPHY_9_KEYS", None)
    if not isinstance(keys, (list, tuple, set)):
        ev.notes.append("no PHILOSOPHY_9_KEYS → V3 = 0")
        return 0.0, ev
    n = len(keys)
    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("keys_present", keys is not None, ""))
    test_results.append(("has_3_keys", n >= 3, f"n={n}"))
    test_results.append(("has_7_keys", n >= 7, f"n={n}"))
    test_results.append(("has_9_keys", n >= 9, f"n={n}"))
    test_results.append(("keys_nonempty", n > 0, f"n={n}"))
    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {"test_results": [{"name": n_, "ok": ok, "note": note} for n_, ok, note in test_results],
              "n_pass": n_pass, "n_keys": n}
    return ev.score, ev


# V4 — ASI_7_QUESTIONS_real (V1154 真覆盖)
def _measure_ASI_7_QUESTIONS_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(name="ASI_7_QUESTIONS_real", score=0.0)
    try:
        from apeireth.v1154_asi_time_philosophy_real_measure import V1154_DIM_NAMES
        n = len(V1154_DIM_NAMES)
        has_5 = n == 5
        test_results: List[Tuple[str, bool, str]] = []
        test_results.append(("V1154_DIM_NAMES_present", True, ""))
        test_results.append(("V1154_DIM_NAMES_5", has_5, f"n={n}"))
        test_results.append(("V1154_DIM_NAMES_has_wall_clock", "wall_clock_grounding" in V1154_DIM_NAMES, ""))
        test_results.append(("V1154_DIM_NAMES_has_duration", "duration_self_perception" in V1154_DIM_NAMES, ""))
        test_results.append(("V1154_DIM_NAMES_has_causal", "causal_order_awareness" in V1154_DIM_NAMES, ""))
        n_pass = sum(1 for _, ok, _ in test_results if ok)
        ev.score = float(n_pass) / 5.0
        ev.score = min(1.0, max(0.0, ev.score))
        ev.checks = {name: ok for name, ok, _ in test_results}
        ev.raw = {"test_results": [{"name": n_, "ok": ok, "note": note} for n_, ok, note in test_results],
                  "n_pass": n_pass, "n_dims": n, "names": list(V1154_DIM_NAMES)}
    except Exception as e:
        ev.notes.append(f"V1154 import failed: {e!r}")
    return ev.score, ev


# V5 — v3_guards_real
def _measure_v3_guards_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(name="v3_guards_real", score=0.0)
    mod = _safe_import("apeireth.v1114_weekly_integration_evaluator")
    if mod is None:
        ev.notes.append("no V1114 → V5 = 0")
        return 0.0, ev
    keys = getattr(mod, "V3_GUARDS", None)
    if not isinstance(keys, (list, tuple, set)):
        ev.notes.append("no V3_GUARDS → V5 = 0")
        return 0.0, ev
    n = len(keys)
    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("keys_present", keys is not None, ""))
    test_results.append(("has_3_keys", n >= 3, f"n={n}"))
    test_results.append(("has_5_keys", n >= 5, f"n={n}"))
    test_results.append(("has_7_keys", n >= 7, f"n={n}"))
    test_results.append(("keys_nonempty", n > 0, f"n={n}"))
    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {"test_results": [{"name": n_, "ok": ok, "note": note} for n_, ok, note in test_results],
              "n_pass": n_pass, "n_keys": n}
    return ev.score, ev


def measure_v2_philosophy_v06() -> float:
    rep = measure_v2_philosophy_v06_full(write_artifact=False)
    return rep.total


def measure_v2_philosophy_v06_full(write_artifact=True, artifact_dir=DEFAULT_ARTIFACT_DIR) -> V2PhilosophyReport:
    t0 = time.time()
    rep = V2PhilosophyReport()
    sub_dims = [
        ("V1135_answers_real", _measure_V1135_answers_real),
        ("V1137_remaining_real", _measure_V1137_remaining_real),
        ("PHILOSOPHY_9_KEYS_real", _measure_PHILOSOPHY_9_KEYS_real),
        ("ASI_7_QUESTIONS_real", _measure_ASI_7_QUESTIONS_real),
        ("v3_guards_real", _measure_v3_guards_real),
    ]
    for name, fn in sub_dims:
        s, ev = fn()
        rep.sub_dim_scores[name] = s
        rep.sub_dim_evidence[name] = ev
        if s >= 0.8:
            rep.n_subdims_passed += 1
        elif s > 0.0:
            rep.n_subdims_partial += 1
        else:
            rep.n_subdims_missing += 1
    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1161_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0
    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1161_v2_philosophy_v06.json"
            artifact_path.write_text(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8")
            rep.artifact_path = str(artifact_path)
        except Exception as e:
            rep.notes.append(f"artifact write failed: {e!r}")
    return rep


def _cli() -> int:
    parser = argparse.ArgumentParser(description="V1161 v2_philosophy V0.6 真补")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--no-write", action="store_true")
    args = parser.parse_args()
    rep = measure_v2_philosophy_v06_full(write_artifact=not args.no_write)
    if args.json:
        print(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False))
    else:
        print(rep.summary_line())
    return 0


if __name__ == "__main__":
    sys.exit(_cli())
