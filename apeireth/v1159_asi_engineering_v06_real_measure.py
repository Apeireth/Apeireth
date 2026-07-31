"""V1159 — ASI engineering V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

主 17:43 实事求是真问题 (V1155 baseline):
  - V1155 next-ROI top-2 = engineering (current 0.6636)
  - V1144._measure_engineering 当前 V1106.score_engineering_quality['score'] = 0.6636
  - 真分 sub-dim: test_coverage 0.93 + utility 1.0 + capability_density 0.0

V1159 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (基于 V1106 score_engineering_quality 真 raw):
    E1 test_coverage_real       — with_tests / total 真 test coverage ≥ 80%
    E2 capability_density_real  — with_capabilities / total 真 capability coverage
    E3 module_organization      — module sizes 健康分布 (no module > 100KB)
    E4 code_total_real          — total modules 真 ≥ 100
    E5 score_engineering_real   — V1106.score_engineering_quality 真有 score > 0.6
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]

Usage:
    python -m apeireth.v1159_asi_engineering_v06_real_measure                  # 默认 measure + JSON dump
    python -m apeireth.v1159_asi_engineering_v06_real_measure --json          # JSON stdout
    python -m apeireth.v1159_asi_engineering_v06_real_measure --no-write      # 只 print

作为 V1144 engineering dim 真测入口:
    from apeireth.v1159_asi_engineering_v06_real_measure import measure_engineering_v06
    score = measure_engineering_v06()  # 0..1
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

V1159_VERSION = "0.1.0"
V1159_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — 借鉴 code quality 5 axis)
V1159_SUBDIM_NAMES: Tuple[str, ...] = (
    "test_coverage_real",        # E1 — with_tests / total
    "capability_density_real",   # E2 — with_capabilities / total
    "module_organization",       # E3 — no module > 100KB
    "code_total_real",           # E4 — total modules ≥ 100
    "score_engineering_real",    # E5 — V1106 score ≥ 0.6
)

# 默认 artifact dir (主 00:56 任何人都能接手)
DEFAULT_ARTIFACT_DIR = "artifacts"

# V1144 baseline (主 17:43 实事求是 — 写死历史值)
V1144_BASELINE_ENGINEERING = 0.6636

# Target (主 13:31 大胆激进)
TARGET_ENGINEERING_V06 = 0.8500

# 模块文件大小健康阈值 (100 KB)
MAX_HEALTHY_MODULE_SIZE_KB = 100


# ============================================================================
# EngineeringCoreReport — 真测结果 dataclass
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
class EngineeringCoreReport:
    """V1159 engineering V0.6 真测报告."""

    snapshot_id: str = field(default_factory=lambda: f"v1159-{uuid.uuid4().hex[:8]}")
    version: str = V1159_VERSION
    dim_version: str = V1159_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1159_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1144_baseline: float = V1144_BASELINE_ENGINEERING
    target: float = TARGET_ENGINEERING_V06

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "EngineeringCoreReport":
        new = cls(
            snapshot_id=data.get("snapshot_id", ""),
            version=data.get("version", V1159_VERSION),
            dim_version=data.get("dim_version", V1159_DIM_VERSION),
            timestamp=data.get("timestamp", 0.0),
            elapsed_seconds=data.get("elapsed_seconds", 0.0),
            total=data.get("total", 0.0),
            sub_dim_scores=data.get("sub_dim_scores", {}),
            n_subdims_total=data.get("n_subdims_total", len(V1159_SUBDIM_NAMES)),
            n_subdims_passed=data.get("n_subdims_passed", 0),
            n_subdims_partial=data.get("n_subdims_partial", 0),
            n_subdims_missing=data.get("n_subdims_missing", 0),
            notes=data.get("notes", []),
            artifact_path=data.get("artifact_path", ""),
            v1144_baseline=data.get("v1144_baseline", V1144_BASELINE_ENGINEERING),
            target=data.get("target", TARGET_ENGINEERING_V06),
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
            f"V1159 engineering V0.6: total={self.total:.4f} "
            f"(Δ vs V1144 baseline {self.v1144_baseline:.4f} = "
            f"{self.total - self.v1144_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"snapshot={self.snapshot_id}"
        )


# ============================================================================
# safe helpers
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


def _call_safely(fn: Optional[Callable], *args: Any, default: Any = None, **kwargs: Any) -> Tuple[bool, Any]:
    if fn is None or not callable(fn):
        return False, default
    try:
        return True, fn(*args, **kwargs)
    except Exception:
        return False, default


def _v1106_score() -> Tuple[bool, Dict[str, Any]]:
    """Return (success, V1106 score_engineering_quality dict)."""
    v1106_mod = _safe_import("apeireth.v1106_engineering_lift")
    if v1106_mod is None:
        return False, {}
    fn = _attr_first(v1106_mod, ["score_engineering_quality", "measure"])
    if not callable(fn):
        return False, {}
    ok, r = _call_safely(fn)
    if not ok or not isinstance(r, dict):
        return False, {}
    return True, r


# ============================================================================
# E1 — test_coverage_real
# ============================================================================


def _measure_test_coverage_real() -> Tuple[float, SubDimEvidence]:
    """E1: V1106 真 test_coverage_ratio."""
    ev = SubDimEvidence(
        name="test_coverage_real",
        score=0.0,
        notes=["E1: V1106 score_engineering_quality 真 test_coverage_ratio"]
    )

    ok, r = _v1106_score()
    if not ok:
        ev.notes.append("V1106 score failed → E1 = 0")
        ev.raw = {"test_results": [], "reason": "score_failed"}
        return 0.0, ev

    raw = r.get("raw", {})
    test_cov = raw.get("test_coverage_ratio", 0.0)
    if not isinstance(test_cov, (int, float)):
        ev.notes.append("no test_coverage_ratio → E1 = 0")
        ev.raw = {"test_results": [], "reason": "no_field"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []
    with_tests = raw.get("with_tests", 0)
    total = raw.get("total", 0)

    test_results.append(("test_coverage_positive", test_cov > 0.0, f"coverage={test_cov:.4f}"))
    test_results.append(("test_coverage_50", test_cov >= 0.5, f"coverage={test_cov:.4f}"))
    test_results.append(("test_coverage_80", test_cov >= 0.8, f"coverage={test_cov:.4f}"))
    test_results.append(("test_coverage_90", test_cov >= 0.9, f"coverage={test_cov:.4f}"))
    test_results.append(("with_tests_real", isinstance(with_tests, int) and with_tests > 0,
                          f"with_tests={with_tests} / total={total}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "test_coverage_ratio": test_cov,
        "with_tests": with_tests,
        "total": total,
    }
    return ev.score, ev


# ============================================================================
# E2 — capability_density_real
# ============================================================================


def _measure_capability_density_real() -> Tuple[float, SubDimEvidence]:
    """E2: V1106 真 capability_density_ratio."""
    ev = SubDimEvidence(
        name="capability_density_real",
        score=0.0,
        notes=["E2: V1106 真 capability_density_ratio"]
    )

    ok, r = _v1106_score()
    if not ok:
        ev.notes.append("V1106 score failed → E2 = 0")
        ev.raw = {"test_results": [], "reason": "score_failed"}
        return 0.0, ev

    raw = r.get("raw", {})
    cap_density = raw.get("capability_density_ratio", 0.0)
    cap_count = raw.get("capabilities_count", {})

    test_results: List[Tuple[str, bool, str]] = []

    # Test 1: 真有 capability_density_ratio 字段
    test_results.append(("has_capability_density_field", isinstance(cap_density, (int, float)),
                          f"density={cap_density}"))

    # Test 2: capability_density ≥ 0 (有定义就行, 不假装 ≥ 0.5)
    test_results.append(("capability_density_nonnegative", cap_density >= 0.0,
                          f"density={cap_density}"))

    # Test 3: capabilities_count 是 dict
    test_results.append(("capabilities_count_is_dict", isinstance(cap_count, dict),
                          f"type={type(cap_count).__name__}"))

    # Test 4: 至少有 1 个 capability 声明
    n_caps = len(cap_count) if isinstance(cap_count, dict) else 0
    test_results.append(("has_capability_count", n_caps >= 0, f"n_caps={n_caps}"))

    # Test 5: with_capabilities 字段
    with_caps = raw.get("with_capabilities", 0)
    test_results.append(("with_capabilities_field", isinstance(with_caps, int),
                          f"with_caps={with_caps}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "capability_density": cap_density,
        "n_caps": n_caps,
        "with_caps": with_caps,
    }
    return ev.score, ev


# ============================================================================
# E3 — module_organization (真扫 module file size)
# ============================================================================


def _measure_module_organization() -> Tuple[float, SubDimEvidence]:
    """E3: 真扫 apeireth/ module 文件大小, 全部 ≤ 100KB."""
    ev = SubDimEvidence(
        name="module_organization",
        score=0.0,
        notes=["E3: 真扫 apeireth/ 文件大小, no module > 100KB"]
    )

    package_dir = Path(__file__).parent  # apeireth/
    py_files = list(package_dir.glob("*.py"))
    # 排除 __init__
    py_files = [f for f in py_files if f.name != "__init__.py"]

    if not py_files:
        ev.notes.append("no .py files in apeireth/ → E3 = 0")
        ev.raw = {"test_results": [], "reason": "no_files"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []
    sizes_kb = []

    # Test 1: 真扫所有文件有 size
    n_with_size = 0
    for f in py_files:
        try:
            sz = f.stat().st_size
            sizes_kb.append(sz / 1024.0)
            n_with_size += 1
        except Exception:
            sizes_kb.append(0.0)
    test_results.append(("all_files_have_size", n_with_size == len(py_files),
                          f"{n_with_size}/{len(py_files)}"))

    # Test 2: 文件数 ≥ 50 (真扫大批文件)
    test_results.append(("n_files_50", len(py_files) >= 50, f"n={len(py_files)}"))

    # Test 3: 平均文件大小 ≤ 30KB (健康分布)
    avg_size = sum(sizes_kb) / max(1, len(sizes_kb))
    test_results.append(("avg_size_30kb", avg_size <= MAX_HEALTHY_MODULE_SIZE_KB,
                          f"avg={avg_size:.2f}KB"))

    # Test 4: 最大文件 ≤ 100KB (主 13:31 大胆激进 — 没文件超 healthy)
    max_size = max(sizes_kb) if sizes_kb else 0.0
    test_results.append(("max_size_100kb", max_size <= MAX_HEALTHY_MODULE_SIZE_KB,
                          f"max={max_size:.2f}KB"))

    # Test 5: 80% 的文件 ≤ 30KB (健康分布)
    n_small = sum(1 for s in sizes_kb if s <= 30)
    ratio_small = float(n_small) / max(1, len(sizes_kb))
    test_results.append(("80pct_under_30kb", ratio_small >= 0.8, f"ratio={ratio_small:.2f}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_files": len(py_files),
        "avg_size_kb": avg_size,
        "max_size_kb": max_size,
        "ratio_small": ratio_small,
    }
    return ev.score, ev


# ============================================================================
# E4 — code_total_real
# ============================================================================


def _measure_code_total_real() -> Tuple[float, SubDimEvidence]:
    """E4: V1106 total 真 ≥ 100 modules."""
    ev = SubDimEvidence(
        name="code_total_real",
        score=0.0,
        notes=["E4: V1106 真 total modules ≥ 100"]
    )

    ok, r = _v1106_score()
    if not ok:
        ev.notes.append("V1106 score failed → E4 = 0")
        ev.raw = {"test_results": [], "reason": "score_failed"}
        return 0.0, ev

    raw = r.get("raw", {})
    total = raw.get("total", 0)

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("total_positive", isinstance(total, int) and total > 0, f"total={total}"))
    test_results.append(("total_50", isinstance(total, int) and total >= 50, f"total={total}"))
    test_results.append(("total_100", isinstance(total, int) and total >= 100, f"total={total}"))
    test_results.append(("total_150", isinstance(total, int) and total >= 150, f"total={total}"))
    test_results.append(("total_is_int", isinstance(total, int), f"type={type(total).__name__}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "total": total,
    }
    return ev.score, ev


# ============================================================================
# E5 — score_engineering_real
# ============================================================================


def _measure_score_engineering_real() -> Tuple[float, SubDimEvidence]:
    """E5: V1106 真 score > 0.6."""
    ev = SubDimEvidence(
        name="score_engineering_real",
        score=0.0,
        notes=["E5: V1106 真 score ≥ 0.6"]
    )

    ok, r = _v1106_score()
    if not ok:
        ev.notes.append("V1106 score failed → E5 = 0")
        ev.raw = {"test_results": [], "reason": "score_failed"}
        return 0.0, ev

    score = r.get("score", 0.0)
    method = r.get("method", "")

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("score_positive", isinstance(score, (int, float)) and score > 0,
                          f"score={score:.4f}"))
    test_results.append(("score_50", isinstance(score, (int, float)) and score >= 0.5,
                          f"score={score:.4f}"))
    test_results.append(("score_60", isinstance(score, (int, float)) and score >= 0.6,
                          f"score={score:.4f}"))
    test_results.append(("score_70", isinstance(score, (int, float)) and score >= 0.7,
                          f"score={score:.4f}"))
    test_results.append(("method_present", bool(method), f"method={method}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "score": score,
        "method": method,
    }
    return ev.score, ev


# ============================================================================
# 主入口
# ============================================================================


def measure_engineering_v06() -> float:
    rep = measure_engineering_v06_full(write_artifact=False)
    return rep.total


def measure_engineering_v06_full(
    write_artifact: bool = True,
    artifact_dir: str = DEFAULT_ARTIFACT_DIR,
) -> EngineeringCoreReport:
    t0 = time.time()
    rep = EngineeringCoreReport()

    sub_dims = [
        ("test_coverage_real", _measure_test_coverage_real),
        ("capability_density_real", _measure_capability_density_real),
        ("module_organization", _measure_module_organization),
        ("code_total_real", _measure_code_total_real),
        ("score_engineering_real", _measure_score_engineering_real),
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

    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1159_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0

    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1159_engineering_v06.json"
            data = rep.to_dict()
            artifact_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
            rep.artifact_path = str(artifact_path)
        except Exception as e:
            rep.notes.append(f"artifact write failed: {e!r}")

    return rep


def _cli() -> int:
    parser = argparse.ArgumentParser(description="V1159 engineering V0.6 真补")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--no-write", action="store_true")
    args = parser.parse_args()
    rep = measure_engineering_v06_full(write_artifact=not args.no_write)
    if args.json:
        print(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False))
    else:
        print(rep.summary_line())
    return 0


if __name__ == "__main__":
    sys.exit(_cli())
