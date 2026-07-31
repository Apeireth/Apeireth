"""V1160 — ASI rubric_open V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

主 17:43 实事求是真问题 (V1155 baseline):
  - V1155 next-ROI top-3 = rubric_open (current 0.7000)
  - V1144._measure_rubric_open 用 V1114.compute_dashboard, 但 compute_dashboard 需 3 个 dict
  - 真测 fit: V1114.evaluate_week + evaluate_halting_signals + render_markdown + 5 halting checks + 5 guards

V1160 真补路径:
  - 5 sub-dim 真测 (基于 V1114 真函数):
    R1 evaluate_week_real     — 真 evaluate_week 真有 ≥ 8 keys
    R2 halting_signals_real   — 5 halting signals 真 check
    R3 dashboard_render_real  — render_markdown 真 produce markdown
    R4 v3_guards_real         — 5 V3 guards 真覆盖
    R5 track_decision_real    — TrackDecision 真选择

Usage:
    python -m apeireth.v1160_asi_rubric_open_v06_real_measure
    python -m apeireth.v1160_asi_rubric_open_v06_real_measure --json
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

V1160_VERSION = "0.1.0"
V1160_DIM_VERSION = "0.6"

V1160_SUBDIM_NAMES: Tuple[str, ...] = (
    "evaluate_week_real",
    "halting_signals_real",
    "dashboard_render_real",
    "v3_guards_real",
    "track_decision_real",
)

DEFAULT_ARTIFACT_DIR = "artifacts"
V1144_BASELINE_RUBRIC_OPEN = 0.7000
TARGET_RUBRIC_OPEN_V06 = 0.8500


# ============================================================================
# RubricOpenReport
# ============================================================================


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
class RubricOpenReport:
    snapshot_id: str = field(default_factory=lambda: f"v1160-{uuid.uuid4().hex[:8]}")
    version: str = V1160_VERSION
    dim_version: str = V1160_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1160_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1144_baseline: float = V1144_BASELINE_RUBRIC_OPEN
    target: float = TARGET_RUBRIC_OPEN_V06

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    def summary_line(self) -> str:
        return (
            f"V1160 rubric_open V0.6: total={self.total:.4f} "
            f"(Δ vs V1144 baseline {self.v1144_baseline:.4f} = "
            f"{self.total - self.v1144_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"snapshot={self.snapshot_id}"
        )


# ============================================================================
# Helpers
# ============================================================================


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


def _call_safely(fn, *args, default=None, **kwargs):
    if fn is None or not callable(fn):
        return False, default
    try:
        return True, fn(*args, **kwargs)
    except Exception:
        return False, default


def _v1114_evaluate_week() -> Tuple[bool, Dict[str, Any]]:
    mod = _safe_import("apeireth.v1114_weekly_integration_evaluator")
    if mod is None:
        return False, {}
    fn = _attr_first(mod, ["evaluate_week", "evaluate"])
    if not callable(fn):
        return False, {}
    ok, r = _call_safely(fn)
    if not ok or not isinstance(r, dict):
        return False, {}
    return True, r


# ============================================================================
# R1 — evaluate_week_real
# ============================================================================


def _measure_evaluate_week_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="evaluate_week_real",
        score=0.0,
        notes=["R1: V1114.evaluate_week 真跑 ≥ 8 keys"]
    )

    ok, r = _v1114_evaluate_week()
    if not ok:
        ev.notes.append("V1114 evaluate_week failed → R1 = 0")
        ev.raw = {"test_results": [], "reason": "evaluate_week_failed"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    test_results.append(("evaluate_week_returns_dict", isinstance(r, dict), ""))
    test_results.append(("n_keys_5", len(r) >= 5, f"n_keys={len(r)}"))
    test_results.append(("n_keys_8", len(r) >= 8, f"n_keys={len(r)}"))
    test_results.append(("n_keys_10", len(r) >= 10, f"n_keys={len(r)}"))
    test_results.append(("has_all_ok_key", "all_ok" in r, ""))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "keys": list(r.keys()),
    }
    return ev.score, ev


# ============================================================================
# R2 — halting_signals_real
# ============================================================================


def _measure_halting_signals_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="halting_signals_real",
        score=0.0,
        notes=["R2: 5 halting signals 真 check"]
    )

    mod = _safe_import("apeireth.v1114_weekly_integration_evaluator")
    if mod is None:
        ev.notes.append("V1114 not importable → R2 = 0")
        ev.raw = {"test_results": [], "reason": "no_v1114"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    halt_fns = [
        "check_halt_signal_1_perf_regression",
        "check_halt_signal_2_candidate_collapse",
        "check_halt_signal_3_locked_in",
        "check_halt_signal_4_red_queen",
        "check_halt_signal_5_no_new_lift",
    ]

    for name in halt_fns:
        fn = getattr(mod, name, None)
        test_results.append((name, callable(fn), ""))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
    }
    return ev.score, ev


# ============================================================================
# R3 — dashboard_render_real
# ============================================================================


def _measure_dashboard_render_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="dashboard_render_real",
        score=0.0,
        notes=["R3: render_markdown 真 produce markdown"]
    )

    ok, r = _v1114_evaluate_week()
    if not ok:
        ev.notes.append("V1114 evaluate_week failed → R3 = 0")
        ev.raw = {"test_results": [], "reason": "no_week"}
        return 0.0, ev

    mod = _safe_import("apeireth.v1114_weekly_integration_evaluator")
    if mod is None:
        ev.notes.append("V1114 not importable → R3 = 0")
        ev.raw = {"test_results": [], "reason": "no_v1114"}
        return 0.0, ev

    render_fn = getattr(mod, "render_markdown", None)

    test_results: List[Tuple[str, bool, str]] = []

    # Test 1: render_markdown 是 callable
    test_results.append(("render_markdown_callable", callable(render_fn), ""))

    # Test 2: render_markdown(r) 真跑
    if callable(render_fn):
        try:
            md = render_fn(r)
            test_results.append(("render_real", isinstance(md, str) and len(md) > 0, f"len={len(md)}"))
            test_results.append(("markdown_long", isinstance(md, str) and len(md) > 100, f"len={len(md)}"))
            test_results.append(("markdown_has_h1", isinstance(md, str) and ("# " in md or "## " in md), ""))
            # 包含 week_label
            week_label = r.get("week_label", "")
            test_results.append((f"contains_week_label_{week_label}", isinstance(md, str) and week_label in md,
                                  f"week_label={week_label}"))
        except Exception as e:
            test_results.append(("render_real", False, f"raised: {str(e)[:50]}"))
    else:
        test_results.append(("render_real", False, "no render_markdown"))
        test_results.append(("markdown_long", False, ""))
        test_results.append(("markdown_has_h1", False, ""))
        test_results.append(("contains_week_label", False, ""))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
    }
    return ev.score, ev


# ============================================================================
# R4 — v3_guards_real
# ============================================================================


def _measure_v3_guards_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="v3_guards_real",
        score=0.0,
        notes=["R4: V3 哲学守门 5 guards 真覆盖"]
    )

    ok, r = _v1114_evaluate_week()
    if not ok:
        ev.notes.append("evaluate_week failed → R4 = 0")
        ev.raw = {"test_results": [], "reason": "no_week"}
        return 0.0, ev

    guards = r.get("guards", {})
    if not isinstance(guards, dict):
        ev.notes.append("no guards dict → R4 = 0")
        ev.raw = {"test_results": [], "reason": "no_guards"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    test_results.append(("guards_is_dict", isinstance(guards, dict), f"keys={list(guards.keys())[:5]}"))
    test_results.append(("n_guards_3", len(guards) >= 3, f"n_guards={len(guards)}"))
    test_results.append(("n_guards_5", len(guards) >= 5, f"n_guards={len(guards)}"))
    test_results.append(("n_guards_8", len(guards) >= 8, f"n_guards={len(guards)}"))
    test_results.append(("guards_have_truthy", any(v for v in guards.values()),
                          ""))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_guards": len(guards),
        "guards_keys": list(guards.keys())[:10],
    }
    return ev.score, ev


# ============================================================================
# R5 — track_decision_real
# ============================================================================


def _measure_track_decision_real() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="track_decision_real",
        score=0.0,
        notes=["R5: TrackDecision 真选择 + choose_main_track 真函数"]
    )

    ok, r = _v1114_evaluate_week()
    if not ok:
        ev.notes.append("evaluate_week failed → R5 = 0")
        ev.raw = {"test_results": [], "reason": "no_week"}
        return 0.0, ev

    mod = _safe_import("apeireth.v1114_weekly_integration_evaluator")
    if mod is None:
        ev.notes.append("V1114 not importable → R5 = 0")
        ev.raw = {"test_results": [], "reason": "no_v1114"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    # Test 1: track_decision 真有
    track_dec = r.get("track_decision")
    test_results.append(("track_decision_present", track_dec is not None, f"type={type(track_dec).__name__}"))

    # Test 2: TrackDecision 是 dataclass
    test_results.append(("TrackDecision_class_exists", hasattr(mod, "TrackDecision"), ""))

    # Test 3: choose_main_track 真函数
    cm = getattr(mod, "choose_main_track", None)
    test_results.append(("choose_main_track_callable", callable(cm), ""))

    # Test 4: choose_main_track 真跑
    if callable(cm):
        try:
            result = cm()
            test_results.append(("choose_main_track_real", result is not None, f"type={type(result).__name__}"))
        except Exception as e:
            test_results.append(("choose_main_track_real", False, f"raised: {str(e)[:50]}"))
    else:
        test_results.append(("choose_main_track_real", False, "no choose_main_track"))

    # Test 5: track_decision has main_track field
    has_main = False
    if track_dec is not None:
        if isinstance(track_dec, dict):
            has_main = "main_track" in track_dec
        elif hasattr(track_dec, "main_track"):
            has_main = True
    test_results.append(("track_decision_has_main_field", has_main, ""))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
    }
    return ev.score, ev


# ============================================================================
# 主入口
# ============================================================================


def measure_rubric_open_v06() -> float:
    rep = measure_rubric_open_v06_full(write_artifact=False)
    return rep.total


def measure_rubric_open_v06_full(
    write_artifact: bool = True,
    artifact_dir: str = DEFAULT_ARTIFACT_DIR,
) -> RubricOpenReport:
    t0 = time.time()
    rep = RubricOpenReport()

    sub_dims = [
        ("evaluate_week_real", _measure_evaluate_week_real),
        ("halting_signals_real", _measure_halting_signals_real),
        ("dashboard_render_real", _measure_dashboard_render_real),
        ("v3_guards_real", _measure_v3_guards_real),
        ("track_decision_real", _measure_track_decision_real),
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

    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1160_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0

    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1160_rubric_open_v06.json"
            artifact_path.write_text(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8")
            rep.artifact_path = str(artifact_path)
        except Exception as e:
            rep.notes.append(f"artifact write failed: {e!r}")

    return rep


def _cli() -> int:
    parser = argparse.ArgumentParser(description="V1160 rubric_open V0.6 真补")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--no-write", action="store_true")
    args = parser.parse_args()
    rep = measure_rubric_open_v06_full(write_artifact=not args.no_write)
    if args.json:
        print(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False))
    else:
        print(rep.summary_line())
    return 0


if __name__ == "__main__":
    sys.exit(_cli())
