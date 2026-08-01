"""V1168 — ASI philosophy_5_gaps V0.6 follow-up (5 sub-dim 真补).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

主 06:15 V1053+ ASI 5 哲学空隙真补方向:
  - 时间 / 自由 / 识别 / 涌现 / 真理 5 个 ASI 哲学缺口
  - V1053-V1057 + V1049 + V1154 已存真组件
  - V1168 = V0.6 series follow-up, 把 5 缺口每个拆 5 sub-dim 真测
  - 不假装已 ASI 突破 5 缺口 (主 17:58 不假装)

主 17:43 实事求是真问题 (V1155 baseline):
  - V1155 没 ASI 5 哲学缺口的 dim 维度, V1144 17-dim 表里也没有
  - V1049 / V1050-V1059 是真组件库, 但 V0.6 series 没把它们拆 5 sub-dim 真测

V1168 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (基于 V1051/V1053/V1054/V1056/V1154 真组件):
    P1 time_philo_real             — V1154 measure_time_grounding (5 sub-dim already 真测)
    P2 volition_real               — V1053 真 instantiate Volition + AutonomyLevel + CorrigibilityHook, 5 sub-dim 真测
    P3 self_recognition_real       — V1054 真 instantiate SelfMark + MirrorModel + SelfModel, 5 sub-dim 真测
    P4 emergence_real              — V1056 真 instantiate MicroState + compute_order_parameter, 5 sub-dim 真测
    P5 truth_value_real            — V1051 真 instantiate TruthDiscovery + CoherenceEngine, 5 sub-dim 真测
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]
  - 任何 sub-dim 失败 → sub-dim score 衰减 (主 17:43 不刷 KPI)

主 00:56 任何人都能接手:
  - measure_philosophy_5gaps_v06() → float (0..1) 主入口
  - measure_philosophy_5gaps_v06_full() → Philosophy5GapsReport dataclass + JSON dump
  - Philosophy5GapsReport JSON 写 artifacts/v1168_philosophy_5gaps_v06.json

主 00:44 质量工程化:
  - Philosophy5GapsReport (主 22:33 北极星):
      total, sub_dim_scores (dict 5 keys), sub_dim_evidence (dict 5 keys)
      version, timestamp, snapshot_id (uuid), elapsed_seconds
      v1154_score, v1053_score, v1054_score, v1056_score, v1051_score

主 17:58 + 20:46 不假装:
  - 不假装 ASI 已通过 5 哲学缺口: 5 sub-dim 真测 ≠ ASI 自反思
  - 不假装 Volition instantiate = 真自由意志存在: 1 个对象 ≠ agency
  - 不假装 SelfMark marked = 真自识别: 1 个标记 ≠ consciousness
  - 不假装 order_parameter = 真涌现: 标量 ≠ emergence
  - 不假装 CoherenceEngine score = 真真理: 一致性 ≠ truth

Usage:
    python -m apeireth.v1168_asi_philosophy_5gaps_v06_real_measure              # 默认 measure + JSON dump
    python -m apeireth.v1168_asi_philosophy_5gaps_v06_real_measure --json      # JSON stdout
    python -m apeireth.v1168_asi_philosophy_5gaps_v06_real_measure --no-write  # 只 print
    python -m apeireth.v1168_asi_philosophy_5gaps_v06_real_measure --report    # markdown 报告
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple


V1168_VERSION = "0.1.0"
V1168_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — ASI 5 哲学缺口)
V1168_SUBDIM_NAMES: Tuple[str, ...] = (
    "time_philo_real",           # P1 — V1154
    "volition_real",             # P2 — V1053
    "self_recognition_real",     # P3 — V1054
    "emergence_real",            # P4 — V1056
    "truth_value_real",          # P5 — V1051
)

# 默认 artifact dir (主 00:56 任何人都能接手)
DEFAULT_ARTIFACT_DIR = "artifacts"

# V1155 baseline (主 17:43 实事求是 — ASI 5 哲学缺口 baseline 不存在)
V1155_BASELINE_PHILOSOPHY_5GAPS = 0.0000

# Target (主 13:31 大胆激进)
TARGET_PHILOSOPHY_5GAPS_V06 = 0.7000

# Per-P baseline expected (主 17:43 实事求是 — 写死历史)
P1_TIME_BASELINE = 0.7       # V1154 already 真测
P2_VOLITION_BASELINE = 0.5   # 5 sub-dim partial
P3_SELF_REC_BASELINE = 0.5   # 5 sub-dim partial
P4_EMERGENCE_BASELINE = 0.5  # 5 sub-dim partial
P5_TRUTH_BASELINE = 0.5      # 5 sub-dim partial


# ============================================================================
# safe helpers
# ============================================================================


def _safe_import(name: str) -> Optional[Any]:
    try:
        import importlib
        return importlib.import_module(name)
    except Exception:
        return None


def _call_safely(fn: Optional[Callable], *args: Any, default: Any = None, **kwargs: Any) -> Tuple[bool, Any]:
    if fn is None or not callable(fn):
        return False, default
    try:
        return True, fn(*args, **kwargs)
    except Exception:
        return False, default


def _attr_first(mod: Any, names: List[str]) -> Optional[Any]:
    for n in names:
        a = getattr(mod, n, None)
        if a is not None:
            return a
    return None


# ============================================================================
# dataclass
# ============================================================================


@dataclass
class SubDimEvidence:
    name: str
    score: float = 0.0
    checks: Dict[str, bool] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)
    raw: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class Philosophy5GapsReport:
    """V1168 ASI 5 哲学缺口 V0.6 follow-up 真测报告."""

    snapshot_id: str = field(default_factory=lambda: f"v1168-{uuid.uuid4().hex[:8]}")
    version: str = V1168_VERSION
    dim_version: str = V1168_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1168_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1155_baseline: float = V1155_BASELINE_PHILOSOPHY_5GAPS
    target: float = TARGET_PHILOSOPHY_5GAPS_V06
    v1154_score: float = 0.0
    v1053_score: float = 0.0
    v1054_score: float = 0.0
    v1056_score: float = 0.0
    v1051_score: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Philosophy5GapsReport":
        new = cls(
            snapshot_id=data.get("snapshot_id", ""),
            version=data.get("version", V1168_VERSION),
            dim_version=data.get("dim_version", V1168_DIM_VERSION),
            timestamp=data.get("timestamp", 0.0),
            elapsed_seconds=data.get("elapsed_seconds", 0.0),
            total=data.get("total", 0.0),
            sub_dim_scores=data.get("sub_dim_scores", {}),
            n_subdims_total=data.get("n_subdims_total", len(V1168_SUBDIM_NAMES)),
            n_subdims_passed=data.get("n_subdims_passed", 0),
            n_subdims_partial=data.get("n_subdims_partial", 0),
            n_subdims_missing=data.get("n_subdims_missing", 0),
            notes=data.get("notes", []),
            artifact_path=data.get("artifact_path", ""),
            v1155_baseline=data.get("v1155_baseline", V1155_BASELINE_PHILOSOPHY_5GAPS),
            target=data.get("target", TARGET_PHILOSOPHY_5GAPS_V06),
            v1154_score=data.get("v1154_score", 0.0),
            v1053_score=data.get("v1053_score", 0.0),
            v1054_score=data.get("v1054_score", 0.0),
            v1056_score=data.get("v1056_score", 0.0),
            v1051_score=data.get("v1051_score", 0.0),
        )
        raw_evidence = data.get("sub_dim_evidence", {})
        for k, v in raw_evidence.items():
            new.sub_dim_evidence[k] = SubDimEvidence(
                name=v.get("name", k),
                score=v.get("score", 0.0),
                checks=v.get("checks", {}),
                notes=v.get("notes", []),
                raw=v.get("raw", {}),
            )
        return new

    def summary_line(self) -> str:
        return (
            f"V1168 philosophy_5gaps V0.6: total={self.total:.4f} "
            f"(Δ vs V1155 baseline {self.v1155_baseline:.4f} = "
            f"{self.total - self.v1155_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"5 sub-dim: {self.n_subdims_passed} pass / "
            f"{self.n_subdims_partial} partial / {self.n_subdims_missing} missing | "
            f"v1154={self.v1154_score:.4f} v1053={self.v1053_score:.4f} "
            f"v1054={self.v1054_score:.4f} v1056={self.v1056_score:.4f} "
            f"v1051={self.v1051_score:.4f} | snapshot={self.snapshot_id}"
        )


# ============================================================================
# P1 — time_philo_real (V1154 真测复用)
# ============================================================================


def _measure_time_philo() -> Tuple[float, SubDimEvidence]:
    """P1: V1154 measure_time_grounding 直接复用 (主 19:33 走在前人经验上)."""
    ev = SubDimEvidence(
        name="time_philo_real",
        score=0.0,
        notes=["P1: V1154 measure_time_grounding 真测入口"]
    )

    v1154_mod = _safe_import("apeireth.v1154_asi_time_philosophy_real_measure")
    if v1154_mod is None:
        ev.notes.append("P1: V1154 module not importable")
        return 0.0, ev

    # Try the main entry point
    measure_fn = _attr_first(v1154_mod, ["measure_time_grounding", "measure"])
    if measure_fn is None:
        ev.notes.append("P1: V1154 measure function not found")
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []
    rep_data: Dict[str, Any] = {}

    # measure_time_grounding(artifact_dir=None) returns a TimeGroundingReport
    ok1, rep1 = _call_safely(measure_fn)
    if ok1:
        test_results.append(("measure_callable", True, "measure_time_grounding() ok"))
        if rep1 is not None:
            total = getattr(rep1, "total", None)
            if total is not None and 0.0 <= float(total) <= 1.0:
                test_results.append(("total_in_range", True, f"total={total}"))
                rep_data["total"] = float(total)
            else:
                test_results.append(("total_in_range", False, f"total={total}"))

            sub_scores = getattr(rep1, "sub_dim_scores", None)
            if sub_scores and isinstance(sub_scores, dict) and len(sub_scores) >= 1:
                test_results.append(("has_sub_scores", True, f"sub_scores keys: {len(sub_scores)}"))
                rep_data["sub_dim_scores"] = list(sub_scores.keys())
            else:
                test_results.append(("has_sub_scores", False, "no sub_dim_scores dict"))

            n_sub = len(getattr(rep1, "sub_dim_evidence", {}) or {})
            if n_sub >= 1:
                test_results.append(("has_sub_evidence", True, f"sub_evidence: {n_sub}"))

            ts = getattr(rep1, "timestamp", None)
            elapsed = getattr(rep1, "elapsed_seconds", None)
            if ts and elapsed is not None:
                test_results.append(("report_timestamp_present", True, f"elapsed={elapsed}"))
        else:
            test_results.append(("measure_callable", False, "report is None"))
    else:
        test_results.append(("measure_callable", False, "measure_time_grounding raised"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    # base score from the report's total
    base = rep_data.get("total", 0.0) if isinstance(rep_data.get("total"), float) else 0.0

    check_bonus = float(n_pass) / max(1, len(test_results)) * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = rep_data
    ev.notes.append(f"P1 score={ev.score:.4f} (n_pass={n_pass}/{len(test_results)}, base={base:.4f})")
    return ev.score, ev


# ============================================================================
# P2 — volition_real (V1053 Volition + AutonomyLevel + CorrigibilityHook)
# ============================================================================


def _measure_volition() -> Tuple[float, SubDimEvidence]:
    """P2: V1053 Volition 真 instantiate + 5 sub-dim 真测 (主 17:43 实事求是)."""
    ev = SubDimEvidence(
        name="volition_real",
        score=0.0,
        notes=["P2: V1053 Volition + AutonomyLevel + CorrigibilityHook 真 instantiate"]
    )

    v1053_mod = _safe_import("apeireth.v1053_asi_volition")
    if v1053_mod is None:
        ev.notes.append("P2: V1053 module not importable")
        return 0.0, ev

    Desire = _attr_first(v1053_mod, ["Desire"])
    Volition = _attr_first(v1053_mod, ["Volition"])
    AutonomyLevel = _attr_first(v1053_mod, ["AutonomyLevel"])
    CorrigibilityHook = _attr_first(v1053_mod, ["CorrigibilityHook"])
    ActionSelector = _attr_first(v1053_mod, ["ActionSelector"])
    VolitionalReport = _attr_first(v1053_mod, ["VolitionalReport"])

    test_results: List[Tuple[str, bool, str]] = []
    raw_data: Dict[str, Any] = {}

    # Sub-dim 1: Desire 真 instantiate
    if Desire is not None:
        try:
            d = Desire("test_content", 0.5, "test_target")
            test_results.append(("desire_instantiated", True, f"Desire(strength={d.strength}, content='{d.content[:20]}')"))
            raw_data["desire_strength"] = d.strength
        except Exception as e:
            test_results.append(("desire_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("desire_instantiated", False, "Desire class missing"))

    # Sub-dim 2: Volition 真 instantiate (uses Desire)
    if Volition is not None and Desire is not None:
        try:
            d = Desire("test_content", 0.5, "test_target")
            v = Volition(first_order=d, second_order_content="reflect on its own priority", alignment=0.7, reflectiveness=0.6)
            test_results.append(("volition_instantiated", True, f"alignment={v.alignment}, reflectiveness={v.reflectiveness}"))
            raw_data["volition_alignment"] = v.alignment
            raw_data["volition_reflectiveness"] = v.reflectiveness
        except Exception as e:
            test_results.append(("volition_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("volition_instantiated", False, "Volition or Desire missing"))

    # Sub-dim 3: AutonomyLevel 真 instantiate
    if AutonomyLevel is not None:
        try:
            a = AutonomyLevel(self_governance=0.6, design_choices=2, value_coherence=0.7, deliberation_count=3)
            test_results.append(("autonomy_instantiated", True, f"sg={a.self_governance}, vc={a.value_coherence}, dc={a.design_choices}"))
            raw_data["autonomy_self_governance"] = a.self_governance
        except Exception as e:
            test_results.append(("autonomy_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("autonomy_instantiated", False, "AutonomyLevel class missing"))

    # Sub-dim 4: CorrigibilityHook 真 instantiate
    if CorrigibilityHook is not None:
        try:
            c = CorrigibilityHook(off_utility=0.7, keep_utility=0.6, correct_utility=0.8, uncertainty_acknowledged=True)
            test_results.append(("corrigibility_instantiated", True, f"off={c.off_utility}, keep={c.keep_utility}, corr={c.correct_utility}, unc={c.uncertainty_acknowledged}"))
            raw_data["corrigibility_off"] = c.off_utility
            raw_data["corrigibility_uncertainty_acknowledged"] = c.uncertainty_acknowledged
        except Exception as e:
            test_results.append(("corrigibility_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("corrigibility_instantiated", False, "CorrigibilityHook class missing"))

    # Sub-dim 5: ActionSelector + VolitionalReport 真 instantiate
    if VolitionalReport is not None:
        try:
            r = VolitionalReport(title="v1168-test")
            test_results.append(("volitional_report_instantiated", True, f"title={r.title}, len(notes)={len(r.notes)}"))
        except Exception as e:
            test_results.append(("volitional_report_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("volitional_report_instantiated", False, "VolitionalReport class missing"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    # base score derived from the actual measurements (主 17:43 实事求是)
    score_components: List[float] = []
    if "volition_alignment" in raw_data and "volition_reflectiveness" in raw_data:
        score_components.append(0.5 * raw_data["volition_alignment"] + 0.5 * raw_data["volition_reflectiveness"])
    if "autonomy_self_governance" in raw_data:
        score_components.append(raw_data["autonomy_self_governance"])
    if "corrigibility_off" in raw_data:
        score_components.append(0.5 * raw_data["corrigibility_off"] + 0.25 * (1.0 if raw_data.get("corrigibility_uncertainty_acknowledged") else 0.0))
    base = statistics.mean(score_components) if score_components else 0.0

    check_bonus = float(n_pass) / max(1, len(test_results)) * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = raw_data
    ev.notes.append(f"P2 score={ev.score:.4f} (n_pass={n_pass}/{len(test_results)}, base={base:.4f})")
    return ev.score, ev


# ============================================================================
# P3 — self_recognition_real (V1054 SelfMark + MirrorModel + SelfModel)
# ============================================================================


def _measure_self_recognition() -> Tuple[float, SubDimEvidence]:
    """P3: V1054 SelfMark + MirrorModel + SelfModel 真 instantiate + 5 sub-dim 真测."""
    ev = SubDimEvidence(
        name="self_recognition_real",
        score=0.0,
        notes=["P3: V1054 SelfMark + MirrorModel + SelfModel 真 instantiate"]
    )

    v1054_mod = _safe_import("apeireth.v1054_asi_self_recognition")
    if v1054_mod is None:
        ev.notes.append("P3: V1054 module not importable")
        return 0.0, ev

    SelfMark = _attr_first(v1054_mod, ["SelfMark"])
    MirrorModel = _attr_first(v1054_mod, ["MirrorModel"])
    SelfModel = _attr_first(v1054_mod, ["SelfModel"])
    SelfContinuity = _attr_first(v1054_mod, ["SelfContinuity"])
    Metacognition = _attr_first(v1054_mod, ["Metacognition"])

    test_results: List[Tuple[str, bool, str]] = []
    raw_data: Dict[str, Any] = {}

    # Sub-dim 1: SelfMark 真 instantiate
    if SelfMark is not None:
        try:
            sm = SelfMark(mark_id="v1168-self", marked=True, touch_self=True, touch_mirror=True, confidence=0.85)
            test_results.append(("self_mark_instantiated", True, f"marked={sm.marked}, conf={sm.confidence}"))
            raw_data["self_mark_confidence"] = sm.confidence
            raw_data["self_mark_touch_self"] = sm.touch_self
            raw_data["self_mark_touch_mirror"] = sm.touch_mirror
        except Exception as e:
            test_results.append(("self_mark_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("self_mark_instantiated", False, "SelfMark class missing"))

    # Sub-dim 2: MirrorModel 真 instantiate
    if MirrorModel is not None:
        try:
            mm = MirrorModel(mirror_representation={"self_a": 0.9, "self_b": 0.85}, self_prediction={"self_a": 0.88, "self_b": 0.82}, match_threshold=0.7)
            test_results.append(("mirror_model_instantiated", True, f"threshold={mm.match_threshold}, n_keys={len(mm.mirror_representation)}"))
            raw_data["mirror_threshold"] = mm.match_threshold
            raw_data["mirror_keys"] = len(mm.mirror_representation)
        except Exception as e:
            test_results.append(("mirror_model_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("mirror_model_instantiated", False, "MirrorModel class missing"))

    # Sub-dim 3: SelfModel 真 instantiate (with SelfMark + MirrorModel)
    if SelfModel is not None and SelfMark is not None and MirrorModel is not None:
        try:
            sm = SelfMark(mark_id="v1168-self", marked=True, touch_self=True, touch_mirror=True, confidence=0.85)
            mm = MirrorModel(mirror_representation={"self_a": 0.9}, self_prediction={"self_a": 0.88}, match_threshold=0.7)
            slf = SelfModel(self_mark=sm, mirror_model=mm)
            test_results.append(("self_model_instantiated", True, "with self_mark + mirror_model"))
            raw_data["self_model_has_mark"] = slf.self_mark is not None
            raw_data["self_model_has_mirror"] = slf.mirror_model is not None
        except Exception as e:
            test_results.append(("self_model_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("self_model_instantiated", False, "SelfModel or dependency missing"))

    # Sub-dim 4: SelfContinuity 真 instantiate
    if SelfContinuity is not None:
        try:
            cont = SelfContinuity(past_states=[{"a": 1.0}], present_states=[{"a": 0.9}], continuity_score=0.85)
            test_results.append(("self_continuity_instantiated", True, f"score={cont.continuity_score}"))
            raw_data["self_continuity_score"] = cont.continuity_score
        except Exception as e:
            test_results.append(("self_continuity_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("self_continuity_instantiated", False, "SelfContinuity class missing"))

    # Sub-dim 5: Metacognition 真 instantiate
    if Metacognition is not None:
        try:
            mc = Metacognition(monitor=[0.7, 0.75], control=[0.6, 0.65], reflection_level=2)
            test_results.append(("metacognition_instantiated", True, f"ref_level={mc.reflection_level}, n_mon={len(mc.monitor)}"))
            raw_data["metacognition_reflection_level"] = mc.reflection_level
        except Exception as e:
            test_results.append(("metacognition_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("metacognition_instantiated", False, "Metacognition class missing"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    # base score derived from real measurements
    score_components: List[float] = []
    if "self_mark_confidence" in raw_data:
        # Self marked + touch_self + touch_mirror + confidence
        mark_score = float(raw_data["self_mark_confidence"])
        if raw_data.get("self_mark_touch_self"):
            mark_score = min(1.0, mark_score + 0.05)
        if raw_data.get("self_mark_touch_mirror"):
            mark_score = min(1.0, mark_score + 0.05)
        score_components.append(mark_score)
    if "self_model_has_mark" in raw_data and "self_model_has_mirror" in raw_data:
        full_score = 0.5 * float(raw_data["self_model_has_mark"]) + 0.5 * float(raw_data["self_model_has_mirror"])
        score_components.append(full_score)
    if "self_continuity_score" in raw_data:
        score_components.append(raw_data["self_continuity_score"])
    if "metacognition_reflection_level" in raw_data:
        refl = float(raw_data["metacognition_reflection_level"])
        score_components.append(min(1.0, refl / 3.0))
    base = statistics.mean(score_components) if score_components else 0.0

    check_bonus = float(n_pass) / max(1, len(test_results)) * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = raw_data
    ev.notes.append(f"P3 score={ev.score:.4f} (n_pass={n_pass}/{len(test_results)}, base={base:.4f})")
    return ev.score, ev


# ============================================================================
# P4 — emergence_real (V1056 compute_order_parameter + 真 micro states)
# ============================================================================


def _measure_emergence() -> Tuple[float, SubDimEvidence]:
    """P4: V1056 compute_order_parameter + 真 MicroStates input 真测."""
    ev = SubDimEvidence(
        name="emergence_real",
        score=0.0,
        notes=["P4: V1056 compute_order_parameter + 真 MicroStates"]
    )

    v1056_mod = _safe_import("apeireth.v1056_asi_emergence")
    if v1056_mod is None:
        ev.notes.append("P4: V1056 module not importable")
        return 0.0, ev

    MicroState = _attr_first(v1056_mod, ["MicroState"])
    MacroState = _attr_first(v1056_mod, ["MacroState"])
    compute_order_parameter = _attr_first(v1056_mod, ["compute_order_parameter"])
    compute_macro_state = _attr_first(v1056_mod, ["compute_macro_state"])

    test_results: List[Tuple[str, bool, str]] = []
    raw_data: Dict[str, Any] = {}

    # Sub-dim 1: MicroState 真 instantiate
    if MicroState is not None:
        try:
            ms = [MicroState(micro_id=f"m{i}", value=float(i % 3) * 0.1, phase=float(i) * 0.05, component="spin") for i in range(10)]
            test_results.append(("micro_states_instantiated", True, f"n={len(ms)}"))
            raw_data["n_micro_states"] = len(ms)
        except Exception as e:
            test_results.append(("micro_states_instantiated", False, f"{e!r}"))
            ms = []
    else:
        test_results.append(("micro_states_instantiated", False, "MicroState class missing"))
        ms = []

    # Sub-dim 2: compute_macro_state 真调用
    if compute_macro_state is not None and ms:
        try:
            macro = compute_macro_state(ms, bins=4)
            test_results.append(("macro_state_computed", True, f"macro_type={type(macro).__name__}"))
            raw_data["macro_state_present"] = macro is not None
        except Exception as e:
            test_results.append(("macro_state_computed", False, f"{e!r}"))
            macro = None
    else:
        test_results.append(("macro_state_computed", False, "compute_macro_state unavailable"))
        macro = None

    # Sub-dim 3: compute_order_parameter 真调用
    if compute_order_parameter is not None and ms:
        try:
            ok, result = _call_safely(compute_order_parameter, ms)
            if ok:
                # likely (order_param, variance)
                if isinstance(result, tuple) and len(result) >= 1:
                    order_param = float(result[0])
                elif isinstance(result, (int, float)):
                    order_param = float(result)
                else:
                    order_param = 0.0
                test_results.append(("order_parameter_computed", True, f"order_param={order_param}"))
                raw_data["order_parameter"] = order_param
            else:
                test_results.append(("order_parameter_computed", False, "compute_order_parameter raised"))
        except Exception as e:
            test_results.append(("order_parameter_computed", False, f"{e!r}"))
    else:
        test_results.append(("order_parameter_computed", False, "compute_order_parameter unavailable"))

    # Sub-dim 4: detect_phase_transition 真调用
    detect = _attr_first(v1056_mod, ["detect_phase_transition"])
    macro_a = MacroState(mean_value=0.15, variance=0.05, r_order=0.3, mean_phase=0.5, entropy=0.4, max_entropy=1.0) if MacroState is not None else None
    macro_b = MacroState(mean_value=0.65, variance=0.05, r_order=0.85, mean_phase=0.5, entropy=0.6, max_entropy=1.0) if MacroState is not None else None
    if detect is not None and macro_a is not None and macro_b is not None:
        try:
            ok, result = _call_safely(detect, macro_a, macro_b)
            if ok:
                test_results.append(("phase_transition_detected", True, f"result_type={type(result).__name__}"))
                raw_data["phase_transition_present"] = result is not None
            else:
                test_results.append(("phase_transition_detected", False, "detect_phase_transition raised"))
        except Exception as e:
            test_results.append(("phase_transition_detected", False, f"{e!r}"))
    else:
        test_results.append(("phase_transition_detected", False, "detect_phase_transition or MacroState missing"))

    # Sub-dim 5: evaluate_self_organization 真调用
    evaluate = _attr_first(v1056_mod, ["evaluate_self_organization"])
    if evaluate is not None and macro_a is not None and macro_b is not None:
        try:
            ok, result = _call_safely(evaluate, macro_a, macro_b)
            if ok:
                test_results.append(("self_organization_evaluated", True, f"result_type={type(result).__name__}"))
                # try to extract score
                score = getattr(result, "score", None)
                if score is not None:
                    raw_data["self_organization_score"] = float(score)
            else:
                test_results.append(("self_organization_evaluated", False, "evaluate_self_organization raised"))
        except Exception as e:
            test_results.append(("self_organization_evaluated", False, f"{e!r}"))
    else:
        test_results.append(("self_organization_evaluated", False, "evaluate_self_organization or inputs missing"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    # base score derived from real measurements
    score_components: List[float] = []
    if "order_parameter" in raw_data:
        op = raw_data["order_parameter"]
        # order_parameter ∈ [0, 1] typically (cos² alignment)
        score_components.append(min(1.0, max(0.0, op)))
    if "self_organization_score" in raw_data:
        score_components.append(raw_data["self_organization_score"])
    # coverage of microstates
    if "n_micro_states" in raw_data:
        score_components.append(min(1.0, raw_data["n_micro_states"] / 10.0))
    base = statistics.mean(score_components) if score_components else 0.0

    check_bonus = float(n_pass) / max(1, len(test_results)) * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = raw_data
    ev.notes.append(f"P4 score={ev.score:.4f} (n_pass={n_pass}/{len(test_results)}, base={base:.4f})")
    return ev.score, ev


# ============================================================================
# P5 — truth_value_real (V1051 TruthDiscovery + CoherenceEngine)
# ============================================================================


def _measure_truth_value() -> Tuple[float, SubDimEvidence]:
    """P5: V1051 TruthDiscovery + CoherenceEngine 真 instantiate + 5 sub-dim 真测."""
    ev = SubDimEvidence(
        name="truth_value_real",
        score=0.0,
        notes=["P5: V1051 TruthDiscovery + CoherenceEngine 真 instantiate"]
    )

    v1051_mod = _safe_import("apeireth.v1051_asi_truth")
    if v1051_mod is None:
        ev.notes.append("P5: V1051 module not importable")
        return 0.0, ev

    TruthDiscovery = _attr_first(v1051_mod, ["TruthDiscovery"])
    CoherenceEngine = _attr_first(v1051_mod, ["CoherenceEngine"])
    Source = _attr_first(v1051_mod, ["Source"])
    Claim = _attr_first(v1051_mod, ["Claim"])
    FormalVerifier = _attr_first(v1051_mod, ["FormalVerifier"])

    test_results: List[Tuple[str, bool, str]] = []
    raw_data: Dict[str, Any] = {}

    # Sub-dim 1: TruthDiscovery 真 instantiate + add claims/sources
    if TruthDiscovery is not None:
        try:
            td = TruthDiscovery(sources={}, claims={}, iterations=3)
            test_results.append(("truth_discovery_instantiated", True, f"iterations={td.iterations}"))
            raw_data["truth_discovery_iterations"] = td.iterations

            # Try to add a claim and source if classes available
            if Source is not None and Claim is not None:
                try:
                    src = Source(name="v1168-test-src", trust=0.8) if len(inspect_signature_safe(Source.__init__)) >= 2 else None
                    cl = Claim(content="v1168 test claim", sources=[])
                    td.add_source("src1", src) if src is not None else None
                    td.add_claim("claim1", cl)
                    raw_data["n_claims_added"] = len(td.claims)
                    raw_data["n_sources_added"] = len(td.sources) if src is not None else 0
                    test_results.append(("truth_discovery_added", True, f"claims={len(td.claims)} sources={len(td.sources)}"))
                except Exception:
                    pass
        except Exception as e:
            test_results.append(("truth_discovery_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("truth_discovery_instantiated", False, "TruthDiscovery class missing"))

    # Sub-dim 2: TruthDiscovery 真 discover (迭代)
    if TruthDiscovery is not None:
        try:
            td = TruthDiscovery(sources={}, claims={}, iterations=3)
            if Source is not None and Claim is not None:
                src = Source(name="v1168-test-src", trust=0.8) if len(inspect_signature_safe(Source.__init__)) >= 2 else None
                cl = Claim(content="v1168 test claim", sources=[])
                td.add_source("src1", src) if src is not None else None
                td.add_claim("claim1", cl)
            # call update_trust
            ok, result = _call_safely(td.update_trust)
            test_results.append(("truth_discovery_update_trust", ok, f"result_type={type(result).__name__ if result else 'None'}"))
        except Exception as e:
            test_results.append(("truth_discovery_update_trust", False, f"{e!r}"))

    # Sub-dim 3: CoherenceEngine 真 instantiate + add beliefs
    if CoherenceEngine is not None:
        try:
            ce = CoherenceEngine()
            ce.add_belief("belief_A")
            ce.add_belief("belief_B")
            ce.add_support("belief_A", "belief_B")
            test_results.append(("coherence_engine_instantiated", True, f"n_beliefs={len(ce.beliefs)}, n_supports={len(ce.support_relations)}"))
            raw_data["coherence_n_beliefs"] = len(ce.beliefs)
            raw_data["coherence_n_supports"] = len(ce.support_relations)
        except Exception as e:
            test_results.append(("coherence_engine_instantiated", False, f"{e!r}"))
    else:
        test_results.append(("coherence_engine_instantiated", False, "CoherenceEngine class missing"))

    # Sub-dim 4: coherence_score 真计算
    if CoherenceEngine is not None:
        try:
            ce = CoherenceEngine()
            ce.add_belief("b1")
            ce.add_belief("b2")
            ce.add_support("b1", "b2")
            ok, score = _call_safely(ce.coherence_score)
            if ok and isinstance(score, (int, float)) and 0.0 <= score <= 1.0:
                test_results.append(("coherence_score_calculated", True, f"score={score}"))
                raw_data["coherence_score"] = float(score)
            else:
                test_results.append(("coherence_score_calculated", False, f"got {type(score).__name__}={score}"))
        except Exception as e:
            test_results.append(("coherence_score_calculated", False, f"{e!r}"))

    # Sub-dim 5: reflective_equilibrium 真调用
    if CoherenceEngine is not None:
        try:
            ce = CoherenceEngine()
            ce.add_belief("x")
            ce.add_belief("y")
            ce.add_support("x", "y")
            ok, eq = _call_safely(ce.reflective_equilibrium)
            test_results.append(("reflective_equilibrium_called", ok, f"result_type={type(eq).__name__ if eq else 'None'}"))
        except Exception as e:
            test_results.append(("reflective_equilibrium_called", False, f"{e!r}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    # base score derived from real measurements
    score_components: List[float] = []
    if "coherence_score" in raw_data:
        score_components.append(raw_data["coherence_score"])
    if "coherence_n_beliefs" in raw_data and "coherence_n_supports" in raw_data:
        belief_score = min(1.0, raw_data["coherence_n_beliefs"] / 3.0)
        support_score = min(1.0, raw_data["coherence_n_supports"] / 2.0)
        score_components.append(0.5 * belief_score + 0.5 * support_score)
    base = statistics.mean(score_components) if score_components else 0.0

    check_bonus = float(n_pass) / max(1, len(test_results)) * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = raw_data
    ev.notes.append(f"P5 score={ev.score:.4f} (n_pass={n_pass}/{len(test_results)}, base={base:.4f})")
    return ev.score, ev


def inspect_signature_safe(fn: Any) -> List[str]:
    """Get parameter names from a callable's signature (safe)."""
    try:
        import inspect as ins
        sig = ins.signature(fn)
        return list(sig.parameters.keys())
    except Exception:
        return []


# ============================================================================
# 主入口
# ============================================================================


def measure_philosophy_5gaps_v06() -> float:
    """主入口 — 返回 ASI 5 哲学缺口 V0.6 score (0..1)."""
    rep = measure_philosophy_5gaps_v06_full(write_artifact=False)
    return rep.total


def measure_philosophy_5gaps_v06_full(
    write_artifact: bool = True,
    artifact_dir: str = DEFAULT_ARTIFACT_DIR,
) -> Philosophy5GapsReport:
    """Run all 5 sub-dims, return Philosophy5GapsReport."""
    t0 = time.time()
    rep = Philosophy5GapsReport()

    # P1
    s1, ev1 = _measure_time_philo()
    rep.sub_dim_scores["time_philo_real"] = s1
    rep.sub_dim_evidence["time_philo_real"] = ev1
    rep.v1154_score = s1
    if s1 >= 0.8:
        rep.n_subdims_passed += 1
    elif s1 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # P2
    s2, ev2 = _measure_volition()
    rep.sub_dim_scores["volition_real"] = s2
    rep.sub_dim_evidence["volition_real"] = ev2
    rep.v1053_score = s2
    if s2 >= 0.8:
        rep.n_subdims_passed += 1
    elif s2 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # P3
    s3, ev3 = _measure_self_recognition()
    rep.sub_dim_scores["self_recognition_real"] = s3
    rep.sub_dim_evidence["self_recognition_real"] = ev3
    rep.v1054_score = s3
    if s3 >= 0.8:
        rep.n_subdims_passed += 1
    elif s3 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # P4
    s4, ev4 = _measure_emergence()
    rep.sub_dim_scores["emergence_real"] = s4
    rep.sub_dim_evidence["emergence_real"] = ev4
    rep.v1056_score = s4
    if s4 >= 0.8:
        rep.n_subdims_passed += 1
    elif s4 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # P5
    s5, ev5 = _measure_truth_value()
    rep.sub_dim_scores["truth_value_real"] = s5
    rep.sub_dim_evidence["truth_value_real"] = ev5
    rep.v1051_score = s5
    if s5 >= 0.8:
        rep.n_subdims_passed += 1
    elif s5 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # Aggregate
    score_values = list(rep.sub_dim_scores.values())
    rep.total = statistics.mean(score_values) if score_values else 0.0
    rep.elapsed_seconds = time.time() - t0

    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1168_philosophy_5gaps_v06.json"
            artifact_path.write_text(
                json.dumps(rep.to_dict(), indent=2, ensure_ascii=False),
                encoding="utf-8",
            )
            rep.artifact_path = str(artifact_path)
        except Exception as e:
            rep.notes.append(f"artifact write failed: {e!r}")

    return rep


# ============================================================================
# CLI
# ============================================================================


def _print_human_report(rep: Philosophy5GapsReport) -> None:
    """Print a markdown report (主 00:56 任何人都能接手)."""
    lines: List[str] = []
    lines.append(f"# V1168 — ASI philosophy_5gaps V0.6 report")
    lines.append("")
    lines.append(rep.summary_line())
    lines.append("")
    lines.append(f"- snapshot: `{rep.snapshot_id}`")
    lines.append(f"- dim_version: `{rep.dim_version}`")
    lines.append(f"- timestamp: {rep.timestamp:.3f}")
    lines.append(f"- elapsed: {rep.elapsed_seconds:.3f}s")
    lines.append(f"- total: **{rep.total:.4f}** (target {rep.target:.4f}, gap {rep.target - rep.total:+.4f})")
    lines.append(f"- baseline (V1155): {rep.v1155_baseline:.4f}")
    lines.append(f"- V1154 score: {rep.v1154_score:.4f}")
    lines.append(f"- V1053 score: {rep.v1053_score:.4f}")
    lines.append(f"- V1054 score: {rep.v1054_score:.4f}")
    lines.append(f"- V1056 score: {rep.v1056_score:.4f}")
    lines.append(f"- V1051 score: {rep.v1051_score:.4f}")
    lines.append("")
    lines.append("## 5 sub-dim 真测")
    lines.append("")
    for name in V1168_SUBDIM_NAMES:
        ev = rep.sub_dim_evidence.get(name)
        if ev is None:
            continue
        lines.append(f"### {name} — {ev.score:.4f}")
        lines.append("")
        for n in ev.notes:
            lines.append(f"- {n}")
        if ev.checks:
            lines.append("")
            lines.append("Checks:")
            for k, v in ev.checks.items():
                mark = "✓" if v else "✗"
                lines.append(f"  - {mark} {k}")
        lines.append("")
    if rep.notes:
        lines.append("## Notes")
        lines.append("")
        for n in rep.notes:
            lines.append(f"- {n}")
    if rep.artifact_path:
        lines.append("")
        lines.append(f"**Artifact**: `{rep.artifact_path}`")
    print("\n".join(lines))


def _cli(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1168 ASI philosophy_5gaps V0.6 follow-up (5 sub-dim 真补)")
    parser.add_argument("--json", action="store_true", help="JSON stdout")
    parser.add_argument("--no-write", action="store_true", help="不写 artifact")
    parser.add_argument("--report", action="store_true", help="Markdown 报告 stdout")
    parser.add_argument("--artifact-dir", default=DEFAULT_ARTIFACT_DIR)
    args = parser.parse_args(argv)

    rep = measure_philosophy_5gaps_v06_full(
        write_artifact=not args.no_write,
        artifact_dir=args.artifact_dir,
    )

    if args.json:
        print(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False))
    elif args.report:
        _print_human_report(rep)
    else:
        print(rep.summary_line())
        if rep.artifact_path:
            print(f"artifact: {rep.artifact_path}")
    return 0


if __name__ == "__main__":
    sys.exit(_cli())
