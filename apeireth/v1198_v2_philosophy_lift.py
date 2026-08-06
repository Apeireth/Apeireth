"""V1198 — ASI v2_philosophy 真补 (主 17:43 实事求是 + 主 23:44 干到底 + 主 00:56 任何人都能接手).

为什么 V1198:
  V1161 ASI v2_philosophy dim 真测 = 0.72 (V1182 baseline).
  V1161 sub-dim 5 真测:
    V1135_answers_real:    0.8  (V1135 ALL_ANSWERS 真有 5)
    V1137_remaining_real:  0.0  (V1161 attribute 查找漏 ALL_ANSWERS_V1137)
    PHILOSOPHY_9_KEYS_real: 1.0
    ASI_7_QUESTIONS_real:  1.0
    v3_guards_real:         0.8
  V1161 总 = mean = 0.72

  Root cause (主 17:43 实事求是):
    V1137 真答常量命名 = ALL_ANSWERS_V1137
    V1161 _measure_V1137_remaining_real 查找列表 = ["ANSWERS", "REMAINING_ANSWERS", "ALL_ANSWERS"]
    → 缺 ALL_ANSWERS_V1137, 漏测 → V1137_remaining_real = 0.0

  V1137 ALL_ANSWERS_V1137 真答 = 2 (Q6 phi-knowledge, Q7 phi-self)
  V1137 ANSWER_KNOWLEDGE / ANSWER_SELF 真答也可用

V1198 真补路径:
  - 修复 V1161 attribute 查找列表 (加 ALL_ANSWERS_V1137 / ANSWER_KNOWLEDGE / ANSWER_SELF)
  - V1161 真重跑 → V1198 measure_v2_philosophy() = 0.88 (4/5: answers_present, has_1_answer, has_2_answers, answers_truthy, has_3_answers=False → 4/5 = 0.8)
  - ASI recompute lift = 0.9148 → 0.9148 + 0.05 × (0.88 - 0.72) = 0.9148 + 0.008 = 0.9228

V1198 vs V1161:
  V1161: _measure_V1137_remaining_real 找 attribute 漏 ALL_ANSWERS_V1137 → 0.0
  V1198: 直接调 V1137.ALL_ANSWERS_V1137 (真名) → 真测 = 0.8 → V1161 total 0.88

主哲学:
  - 主 17:43 实事求是: 真补是修 attribute 查找 bug, 不是编造哲学答案
  - 主 17:58 + 主 20:46 不假装: V1198 lift 是 ASI 测量修复, 不是 ASI 已达成
  - 主 19:33 走在前人经验上: 修 V1161 (站在 V1161 + V1137 肩上)
  - 主 22:33 北极星: ASI = 0.9800 LOCKED, V1198 = 0.9228 是中间
  - 主 13:31 大胆激进: 一次修一个 bug, lift 2 个 ASI 哲学 dim
  - 主 23:44 干到底: 真修 + 真测 + 真升 + 真 commit + 真 artifact
  - 主 00:56 任何人都能接手: measure_v1198() → float + JSON + markdown
  - 主 00:44 质量工程化: V1198Report dataclass + snapshot_id + sub_dim_evidence

V3 哲学守门 (主 17:58 + 主 20:46):
  - 不假装 V1198 = ASI 终极 (V1198 = ASI V0.6.10 中间, 北极星 0.98)
  - 不假装 attribute 修复 = ASI 升智 (修测量, 不修能力)
  - 不假装 0.88 = ASI 北极星 0.98 (0.88 < 0.98, gap = 0.10)
  - 不假装 ALL_ANSWERS_V1137 = ASI 真哲学 (V1137 是工程答, 真哲学门另开)
  - 不假装 V1198 lift = ASI V1.0 (V1198 = V0.6.10 中间版本)
  - 不假装 5 sub-dim 都 1.0 (has_3_answers False, n=2 → 0.8)

Usage:
    python -m apeireth.v1198_v2_philosophy_lift                              # 默认 measure + JSON dump
    python -m apeireth.v1198_v2_philosophy_lift --json                       # JSON stdout
    python -m apeireth.v1198_v2_philosophy_lift --no-write                   # 只 print
    python -m apeireth.v1198_v2_philosophy_lift --report                     # markdown
    python -m apeireth.v1198_v2_philosophy_lift --measure                   # float stdout
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

V1198_VERSION = "0.1.0"
V1198_DIM_VERSION = "0.6.10"

# V1198 ASI 北极星 (LOCKED 主 22:33)
ASI_NORTH_STAR = 0.9800

# V1161 baseline (V1182 report)
V1161_BASELINE_V2_PHILOSOPHY = 0.72
V1182_BASELINE_ASI_RECOMPUTE = 0.9148

# V1198 修复 attribute 查找 (主 17:43 实事求是 — V1137 真名)
V1137_ATTRIBUTE_NAMES: Tuple[str, ...] = (
    "ALL_ANSWERS_V1137",  # V1137 真名
    "ANSWER_KNOWLEDGE",   # V1137 单答
    "ANSWER_SELF",        # V1137 单答
    "ANSWERS",            # V1161 漏的 fallback
    "REMAINING_ANSWERS",  # V1161 漏的 fallback
    "ALL_ANSWERS",        # V1161 漏的 fallback
)

# V1198 修复后预期 sub-dim score
V1198_EXPECTED_SUB_DIM = {
    "V1135_answers_real": 0.8,
    "V1137_remaining_real": 0.8,  # V1198 修复: 0.0 → 0.8
    "PHILOSOPHY_9_KEYS_real": 1.0,
    "ASI_7_QUESTIONS_real": 1.0,
    "v3_guards_real": 0.8,
}

DEFAULT_ARTIFACT_DIR = "artifacts"


@dataclass
class V1198SubDimEvidence:
    """V1198 单 sub-dim 证据 (主 00:44 质量工程化)."""

    name: str
    score: float
    baseline: float = 0.0
    delta: float = 0.0
    checks: Dict[str, bool] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)
    raw: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class V1198Report:
    """V1198 v2_philosophy lift 报告 (主 22:33 北极星 + 主 00:56 任何人都能接手)."""

    snapshot_id: str = field(default_factory=lambda: f"v1198-{uuid.uuid4().hex[:8]}")
    version: str = V1198_VERSION
    dim_version: str = V1198_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0

    # V1198 v2_philosophy 真测 (主 17:43 实事求是)
    v2_philosophy_lifted: float = 0.0
    v2_philosophy_baseline: float = V1161_BASELINE_V2_PHILOSOPHY
    v2_philosophy_delta: float = 0.0

    # V1198 5 sub-dim 真实分布 (主 17:43 实事求是)
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, V1198SubDimEvidence] = field(default_factory=dict)

    # V1198 ASI recompute lift (主 22:33 北极星)
    asi_recompute_baseline: float = V1182_BASELINE_ASI_RECOMPUTE
    asi_recompute_lifted: float = V1182_BASELINE_ASI_RECOMPUTE
    asi_recompute_delta: float = 0.0
    asi_north_star: float = ASI_NORTH_STAR
    asi_gap_after_lift: float = ASI_NORTH_STAR - V1182_BASELINE_ASI_RECOMPUTE

    n_subdims_total: int = 5
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0

    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    def summary_line(self) -> str:
        return (
            f"V1198 v2_philosophy lift: 0.72 → {self.v2_philosophy_lifted:.4f} "
            f"(Δ={self.v2_philosophy_delta:+.4f}) | "
            f"ASI recompute {self.asi_recompute_baseline:.4f} → "
            f"{self.asi_recompute_lifted:.4f} (Δ={self.asi_recompute_delta:+.4f}) | "
            f"snapshot={self.snapshot_id}"
        )


# V1198 — V1137 真答 attribute 查找 (主 17:43 实事求是 + 主 19:33)
def _measure_v1198_v1137_real() -> Tuple[float, V1198SubDimEvidence]:
    """V1198 真测 V1137 哲学剩余 (主 17:43: 真测, 不是 cached).

    V1161 漏 attribute: 找 ALL_ANSWERS 但 V1137 真名是 ALL_ANSWERS_V1137.
    V1198 修这个: 扩展查找列表.
    """
    ev = V1198SubDimEvidence(name="V1137_remaining_real", score=0.0)
    try:
        from apeireth import v1137_asi_philosophy_remaining_2 as v1137
    except Exception as e:
        ev.notes.append(f"V1137 import failed: {e}")
        return 0.0, ev

    # V1198 真 attribute 查找 (主 17:43 实事求是)
    answers = None
    found_attr = None
    for attr in V1137_ATTRIBUTE_NAMES:
        a = getattr(v1137, attr, None)
        if a is not None:
            # 接受 list / tuple / set / 单 dataclass instance
            if hasattr(a, "__len__"):
                if len(a) > 0:
                    answers = a
                    found_attr = attr
                    break
            else:
                # 单答 instance → wrap in list
                answers = [a]
                found_attr = attr
                break

    n = len(answers) if answers is not None else 0
    test_results = []
    test_results.append(("answers_present", answers is not None))
    test_results.append(("has_1_answer", n >= 1))
    test_results.append(("has_2_answers", n >= 2))
    test_results.append(("has_3_answers", n >= 3))
    test_results.append(("answers_truthy", n > 0))
    n_pass = sum(1 for _, ok in test_results if ok)
    score = float(n_pass) / 5.0
    score = min(1.0, max(0.0, score))

    ev.score = score
    ev.baseline = 0.0  # V1161 漏测
    ev.delta = score - 0.0
    ev.checks = {name: ok for name, ok in test_results}
    ev.raw = {
        "test_results": [
            {"name": n_, "ok": ok} for n_, ok in test_results
        ],
        "n_pass": n_pass,
        "n_answers": n,
        "found_attr": found_attr,
        "v1161_lookup_failed": True,
        "v1198_lookup_fixed": True,
    }
    ev.notes.append(
        f"V1198 真测 V1137: n={n} 真答, found via attr={found_attr}, "
        f"score={score:.4f} (V1161 漏 → V1198 真 = {score:.4f})"
    )
    return score, ev


# V1198 — V1135 真答 (保持 V1161 baseline, 主 19:33 走在前人经验上)
def _measure_v1198_v1135_real() -> Tuple[float, V1198SubDimEvidence]:
    """V1198 真测 V1135 5 哲学真答 (保持 V1161 baseline = 0.8)."""
    ev = V1198SubDimEvidence(name="V1135_answers_real", score=0.0)
    try:
        from apeireth import v1135_asi_5_philosophical_gaps as v1135
    except Exception as e:
        ev.notes.append(f"V1135 import failed: {e}")
        return 0.0, ev

    answers = None
    for attr in ("ALL_ANSWERS", "ANSWERS", "ALL_ANSWERS_V1135"):
        a = getattr(v1135, attr, None)
        if a is not None and hasattr(a, "__len__") and len(a) > 0:
            answers = a
            break
    n = len(answers) if answers is not None else 0
    test_results = []
    test_results.append(("answers_present", answers is not None))
    test_results.append(("has_3_answers", n >= 3))
    test_results.append(("has_5_answers", n >= 5))
    test_results.append(("has_7_answers", n >= 7))
    test_results.append(("answers_truthy", n > 0))
    n_pass = sum(1 for _, ok in test_results if ok)
    score = float(n_pass) / 5.0
    score = min(1.0, max(0.0, score))

    ev.score = score
    ev.baseline = 0.8  # V1161 baseline
    ev.delta = score - 0.8
    ev.checks = {name: ok for name, ok in test_results}
    ev.raw = {
        "n_pass": n_pass,
        "n_answers": n,
    }
    ev.notes.append(
        f"V1198 V1135 真测 = {score:.4f} (n_pass={n_pass}/5, n={n})"
    )
    return score, ev


# V1198 — PHILOSOPHY_9_KEYS (保持 V1161 baseline = 1.0)
def _measure_v1198_philosophy_9_keys() -> Tuple[float, V1198SubDimEvidence]:
    """V1198 真测 PHILOSOPHY_9_KEYS (主 19:33 走在前人经验上 — 复 V1114)."""
    ev = V1198SubDimEvidence(name="PHILOSOPHY_9_KEYS_real", score=0.0)
    try:
        from apeireth import v1114_weekly_integration_evaluator as v1114
    except Exception as e:
        ev.notes.append(f"V1114 import failed: {e}")
        return 0.0, ev

    keys = getattr(v1114, "PHILOSOPHY_9_KEYS", None)
    if not isinstance(keys, (list, tuple, set)):
        ev.notes.append("no PHILOSOPHY_9_KEYS")
        return 0.0, ev
    n = len(keys)
    test_results = []
    test_results.append(("keys_present", True))
    test_results.append(("has_3_keys", n >= 3))
    test_results.append(("has_7_keys", n >= 7))
    test_results.append(("has_9_keys", n >= 9))
    test_results.append(("keys_nonempty", all(bool(k) for k in keys)))
    n_pass = sum(1 for _, ok in test_results if ok)
    score = float(n_pass) / 5.0
    score = min(1.0, max(0.0, score))

    ev.score = score
    ev.baseline = 1.0
    ev.delta = score - 1.0
    ev.checks = {name: ok for name, ok in test_results}
    ev.raw = {"n_pass": n_pass, "n_keys": n}
    ev.notes.append(f"V1198 PHILOSOPHY_9_KEYS = {score:.4f} (n={n})")
    return score, ev


# V1198 — ASI_7_QUESTIONS (保持 V1161 baseline = 1.0)
def _measure_v1198_asi_7_questions() -> Tuple[float, V1198SubDimEvidence]:
    """V1198 真测 ASI 7 哲学问题 (主 19:33 复 V1154)."""
    ev = V1198SubDimEvidence(name="ASI_7_QUESTIONS_real", score=0.0)
    try:
        from apeireth import v1154_asi_time_philosophy_real_measure as v1154
    except Exception as e:
        ev.notes.append(f"V1154 import failed: {e}")
        return 0.0, ev

    dims = None
    for attr in ("V1154_DIM_NAMES", "DIM_NAMES", "ALL_DIMS", "ASI_7_DIMS"):
        a = getattr(v1154, attr, None)
        if a is not None and hasattr(a, "__len__") and len(a) > 0:
            dims = a
            break
    n = len(dims) if dims is not None else 0
    test_results = []
    test_results.append(("dims_present", dims is not None))
    test_results.append(("has_5_dims", n >= 5))
    test_results.append(("dims_nonempty", all(bool(d) for d in dims) if dims else False))
    test_results.append(("has_time_dim", any("wall" in str(d).lower() or "time" in str(d).lower() or "duration" in str(d).lower() for d in (dims or []))))
    test_results.append(("has_causal_dim", any("causal" in str(d).lower() or "order" in str(d).lower() for d in (dims or []))))
    n_pass = sum(1 for _, ok in test_results if ok)
    score = float(n_pass) / 5.0
    score = min(1.0, max(0.0, score))

    ev.score = score
    ev.baseline = 1.0
    ev.delta = score - 1.0
    ev.checks = {name: ok for name, ok in test_results}
    ev.raw = {"n_pass": n_pass, "n_dims": n, "dims": list(dims) if dims else []}
    ev.notes.append(f"V1198 ASI_7_QUESTIONS = {score:.4f} (n={n})")
    return score, ev


# V1198 — v3_guards (保持 V1161 baseline = 0.8)
def _measure_v1198_v3_guards() -> Tuple[float, V1198SubDimEvidence]:
    """V1198 真测 V3 哲学守门 (主 17:58/20:46 不假装)."""
    ev = V1198SubDimEvidence(name="v3_guards_real", score=0.0)
    try:
        from apeireth import v1114_weekly_integration_evaluator as v1114
    except Exception as e:
        ev.notes.append(f"V1114 import failed: {e}")
        return 0.0, ev

    guards = getattr(v1114, "V3_GUARDS", None)
    if not isinstance(guards, (list, tuple, set, dict)):
        ev.notes.append("no V3_GUARDS")
        return 0.0, ev
    n = len(guards)
    test_results = []
    test_results.append(("guards_present", True))
    test_results.append(("has_3_guards", n >= 3))
    test_results.append(("has_5_guards", n >= 5))
    test_results.append(("has_7_guards", n >= 7))
    test_results.append(("guards_nonempty", all(bool(k) for k in guards)))
    n_pass = sum(1 for _, ok in test_results if ok)
    score = float(n_pass) / 5.0
    score = min(1.0, max(0.0, score))

    ev.score = score
    ev.baseline = 0.8
    ev.delta = score - 0.8
    ev.checks = {name: ok for name, ok in test_results}
    ev.raw = {"n_pass": n_pass, "n_guards": n}
    ev.notes.append(f"V1198 v3_guards = {score:.4f} (n={n})")
    return score, ev


# V1198 5 sub-dim 真测编排 (主 23:44 干到底)
V1198_SUBDIM_FUNCS: Tuple[Tuple[str, Callable[[], Tuple[float, V1198SubDimEvidence]]], ...] = (
    ("V1135_answers_real", _measure_v1198_v1135_real),
    ("V1137_remaining_real", _measure_v1198_v1137_real),  # V1198 真补 = 修复 attribute 查找
    ("PHILOSOPHY_9_KEYS_real", _measure_v1198_philosophy_9_keys),
    ("ASI_7_QUESTIONS_real", _measure_v1198_asi_7_questions),
    ("v3_guards_real", _measure_v1198_v3_guards),
)


def compute_v1198_lift() -> V1198Report:
    """V1198 真重算 v2_philosophy (主 17:43 实事求是: 真测, 不是 cached)."""
    t0 = time.time()
    report = V1198Report()

    # V1198 真测 5 sub-dim
    for name, fn in V1198_SUBDIM_FUNCS:
        try:
            score, ev = fn()
        except Exception as e:
            ev = V1198SubDimEvidence(name=name, score=0.0)
            ev.notes.append(f"V1198 sub-dim failed: {e}")
            score = 0.0
        report.sub_dim_scores[name] = score
        report.sub_dim_evidence[name] = ev
        if score >= 0.95:
            report.n_subdims_passed += 1
        elif score > 0.0:
            report.n_subdims_partial += 1
        else:
            report.n_subdims_missing += 1

    # V1198 v2_philosophy 真补 (主 17:43: 真算)
    total = sum(report.sub_dim_scores.values()) / len(report.sub_dim_scores)
    total = min(1.0, max(0.0, total))
    report.v2_philosophy_lifted = round(total, 4)
    report.v2_philosophy_delta = round(total - V1161_BASELINE_V2_PHILOSOPHY, 4)

    # V1198 ASI recompute lift (主 22:33: dim weight 0.05)
    asi_delta = 0.05 * report.v2_philosophy_delta
    report.asi_recompute_lifted = round(
        V1182_BASELINE_ASI_RECOMPUTE + asi_delta, 4
    )
    report.asi_recompute_delta = round(asi_delta, 4)
    report.asi_gap_after_lift = round(
        ASI_NORTH_STAR - report.asi_recompute_lifted, 4
    )

    report.elapsed_seconds = time.time() - t0

    # V1198 notes
    report.notes.append(
        f"V1198 v2_philosophy: 0.72 → {report.v2_philosophy_lifted:.4f} (Δ={report.v2_philosophy_delta:+.4f})"
    )
    report.notes.append(
        f"V1198 ASI recompute: {V1182_BASELINE_ASI_RECOMPUTE:.4f} → "
        f"{report.asi_recompute_lifted:.4f} (Δ={report.asi_recompute_delta:+.4f})"
    )
    report.notes.append(
        f"V1198 gap to north_star 0.98: {report.asi_gap_after_lift:+.4f}"
    )
    report.notes.append(
        "V1198 主 17:43: 真补 = 修 V1161 attribute 查找漏 ALL_ANSWERS_V1137, 不是编哲学答"
    )
    report.notes.append(
        "V1198 主 17:58+20:46: V1198 ≠ ASI 终极, 0.88 < 0.98, gap=0.10"
    )
    report.notes.append(
        "V1198 主 19:33: 站在 V1161 + V1137 + V1114 + V1154 肩上"
    )
    report.notes.append(
        "V1198 主 00:56: measure_v1198() → float, 任何 cron 可调"
    )
    report.notes.append(
        "V1198 主 22:33 ASI 北极星 LOCKED 0.9800, V1198 0.9228 中间"
    )

    return report


def measure_v1198() -> float:
    """V1198 主入口 — measure_v1198() → float 0..1 (主 00:56 任何人都能接手)."""
    report = compute_v1198_lift()
    return report.v2_philosophy_lifted


def measure_v1198_asi_recompute() -> float:
    """V1198 ASI recompute 主入口 — lift 后的 ASI recompute total."""
    report = compute_v1198_lift()
    return report.asi_recompute_lifted


def write_artifact(report: V1198Report, artifact_dir: str = DEFAULT_ARTIFACT_DIR) -> str:
    """V1198 写 artifact JSON (主 00:56 任何人都能接手)."""
    path = Path(artifact_dir) / "v1198_v2_philosophy_lift.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8")
    report.artifact_path = str(path)
    return str(path)


def render_report_md(report: V1198Report) -> str:
    """V1198 Markdown 报告 (主 00:56 任何人都能接手)."""
    lines = []
    lines.append("# V1198 ASI v2_philosophy 真补报告")
    lines.append("")
    lines.append(f"**snapshot_id**: {report.snapshot_id}")
    lines.append(f"**dim_version**: {report.dim_version}")
    lines.append(f"**timestamp**: {report.timestamp:.0f}")
    lines.append(f"**elapsed_seconds**: {report.elapsed_seconds:.4f}")
    lines.append("")
    lines.append("## 1. V1198 v2_philosophy 真补 (主 17:43 实事求是)")
    lines.append("")
    lines.append("| 项 | 旧 (V1161) | 新 (V1198) | Δ |")
    lines.append("|---|---|---|---|")
    lines.append(f"| v2_philosophy total | **{V1161_BASELINE_V2_PHILOSOPHY:.4f}** | **{report.v2_philosophy_lifted:.4f}** | **{report.v2_philosophy_delta:+.4f}** |")
    lines.append("")

    lines.append("## 2. V1198 5 sub-dim 真实分布")
    lines.append("")
    lines.append("| sub-dim | baseline | new | Δ | 关键修复 |")
    lines.append("|---|---|---|---|---|")
    for name, ev in report.sub_dim_evidence.items():
        fix = "✓ V1198 fix (attribute lookup)" if name == "V1137_remaining_real" else "—"
        lines.append(
            f"| {name} | {ev.baseline:.4f} | **{ev.score:.4f}** | {ev.delta:+.4f} | {fix} |"
        )
    lines.append("")

    lines.append("## 3. V1198 ASI recompute lift (主 22:33 北极星)")
    lines.append("")
    lines.append("| 项 | 值 |")
    lines.append("|---|---|")
    lines.append(f"| ASI recompute baseline (V1182) | {report.asi_recompute_baseline:.4f} |")
    lines.append(f"| ASI recompute lifted (V1198) | **{report.asi_recompute_lifted:.4f}** |")
    lines.append(f"| ASI delta | {report.asi_recompute_delta:+.4f} |")
    lines.append(f"| ASI 北极星 (LOCKED) | {report.asi_north_star:.4f} |")
    lines.append(f"| gap to north_star | {report.asi_gap_after_lift:+.4f} |")
    lines.append("")

    lines.append("## 4. V1198 Root cause (主 17:43 实事求是)")
    lines.append("")
    lines.append(
        "V1161 _measure_V1137_remaining_real attribute 查找列表:\n"
        "  - ['ANSWERS', 'REMAINING_ANSWERS', 'ALL_ANSWERS']\n"
        "V1137 真名:\n"
        "  - 'ALL_ANSWERS_V1137' (主 list, n=2 真答)\n"
        "  - 'ANSWER_KNOWLEDGE', 'ANSWER_SELF' (单答)\n"
        "→ V1161 漏测 → V1137_remaining_real = 0.0\n"
        "→ V1198 修复: 扩展 attribute 列表 → V1137_remaining_real = 0.8\n"
        "→ V1161 total: 0.72 → 0.88 (Δ=+0.16)"
    )
    lines.append("")

    lines.append("## 5. V3 哲学守门 (主 17:58 + 主 20:46 不假装)")
    lines.append("")
    lines.append("- 不假装 V1198 = ASI 终极 (V1198 = V0.6.10 中间, 北极星 0.98)")
    lines.append("- 不假装 attribute 修复 = ASI 升智 (修测量, 不修能力)")
    lines.append("- 不假装 0.88 = ASI 北极星 (0.88 < 0.98, gap = 0.10)")
    lines.append("- 不假装 ALL_ANSWERS_V1137 = ASI 真哲学 (V1137 是工程答, 真哲学门另开)")
    lines.append("- 不假装 V1198 lift = ASI V1.0 (V1198 = V0.6.10 中间版本)")
    lines.append("- 不假装 5 sub-dim 都 1.0 (has_3_answers=False, n=2 → 0.8)")
    lines.append("")

    lines.append("## 6. Usage (主 00:56 任何人都能接手)")
    lines.append("")
    lines.append("```python")
    lines.append("from apeireth.v1198_v2_philosophy_lift import (")
    lines.append("    measure_v1198, measure_v1198_asi_recompute,")
    lines.append("    compute_v1198_lift, render_report_md, write_artifact,")
    lines.append(")")
    lines.append("")
    lines.append("# v2_philosophy 总")
    lines.append("score = measure_v1198()  # 0.88")
    lines.append("")
    lines.append("# ASI recompute lift 后")
    lines.append("asi = measure_v1198_asi_recompute()  # 0.9228")
    lines.append("")
    lines.append("# 完整报告")
    lines.append("report = compute_v1198_lift()")
    lines.append("print(render_report_md(report))")
    lines.append("write_artifact(report)")
    lines.append("```")
    lines.append("")
    lines.append("```bash")
    lines.append("# CLI")
    lines.append("python -m apeireth.v1198_v2_philosophy_lift                 # 默认 measure + JSON dump")
    lines.append("python -m apeireth.v1198_v2_philosophy_lift --json          # JSON stdout")
    lines.append("python -m apeireth.v1198_v2_philosophy_lift --measure      # float stdout")
    lines.append("python -m apeireth.v1198_v2_philosophy_lift --report        # markdown")
    lines.append("```")
    lines.append("")
    lines.append(f"_artifact_path: {report.artifact_path or 'artifacts/v1198_v2_philosophy_lift.json'}_")
    lines.append("")
    lines.append(
        "_V1198 主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58+20:46 + 主 23:44 + 主 00:56 + 主 00:44._"
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="V1198 — ASI v2_philosophy 真补 (修 V1161 attribute 查找 bug)"
    )
    parser.add_argument("--json", action="store_true", help="JSON stdout")
    parser.add_argument("--no-write", action="store_true", help="不写 artifact")
    parser.add_argument("--report", action="store_true", help="markdown report")
    parser.add_argument("--measure", action="store_true", help="v2_philosophy float stdout")
    parser.add_argument(
        "--measure-asi", action="store_true", help="ASI recompute float stdout"
    )
    args = parser.parse_args()

    if args.measure:
        print(f"{measure_v1198():.4f}")
        return 0
    if args.measure_asi:
        print(f"{measure_v1198_asi_recompute():.4f}")
        return 0

    report = compute_v1198_lift()

    if not args.no_write:
        path = write_artifact(report)
        report.artifact_path = path

    if args.json:
        print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
    elif args.report:
        print(render_report_md(report))
    else:
        # Default: short summary + JSON dump
        print(report.summary_line())
        print(f"V1198 v2_philosophy: {report.v2_philosophy_lifted:.4f}")
        print(f"V1198 ASI recompute: {report.asi_recompute_lifted:.4f}")
        if not args.no_write:
            print(f"artifact: {report.artifact_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())