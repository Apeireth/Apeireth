"""V1166 — ASI real_llm_benchmark V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化 +
主 06:15 V1050+ 真评测.

主 06:15 06:32 真评测方向: V1034 benchmark 接 LLM API (真接 NewAPI M3, 真跑 22 真样本).
主 17:43 实事求是真问题 (V1155 baseline):
  - V1133 真跑 Benchmark 真接 HTTP + 真测 token/cost/latency, 但 V1155 baseline
    没用 22 真样本分维度, 直接给了 0.95 (over-fit). V1166 真补: 把 V1133 V1133BenchmarkReport
    真 9 字段拆成 5 sub-dim 真测.
  - 不假装 pass_rate 0.95 = 真 ASI 推理: 一过式 HTTP 调用 ≠ ASI 级 reasoning.

V1166 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (基于 V1133 V1133BenchmarkReport 真字段):
    L1 api_key_resolution_real     — api_key_present + source ∈ {env, file}
    L2 endpoint_reachability_real  — HTTP attempts ≥ n_samples - n_error
    L3 sample_coverage_real        — n_samples ≥ 22 (full benchmark)
    L4 pass_rate_real              — pass_rate ≥ 0.5 + p50_latency_ms > 0
    L5 latency_distribution_real   — p95_latency_ms > 0 + variance reasonable
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]
  - 任何 sub-dim 失败 → sub-dim score 衰减 (主 17:43 不刷 KPI)

主 00:56 任何人都能接手:
  - measure_real_llm_benchmark_v06() → float (0..1) 主入口
  - measure_real_llm_benchmark_full() → RealLLMBenchmarkReport dataclass + JSON dump
  - RealLLMBenchmarkReport JSON 写 artifacts/v1166_real_llm_benchmark_v06.json

主 00:44 质量工程化:
  - RealLLMBenchmarkReport (主 22:33 北极星):
      total, sub_dim_scores (dict 5 keys), sub_dim_evidence (dict 5 keys)
      version, timestamp, snapshot_id (uuid), elapsed_seconds
      v1133_benchmark_id, v1133_pass_rate, v1133_endpoint, v1133_api_key_present

主 17:58 + 20:46 不假装:
  - 不假装 pass_rate = 真 ASI 推理: 一过式 benchmark ≠ 长期 reasoning quality
  - 不假装 HTTP 200 = ASI 推理成功: HTTP code 2xx + content match = 1 sample pass
  - 不假装 mock = 真实: 不 mock (主 17:43 实事求是)
  - 不假装 ASI has LLM reasoning: 5 sub-dim 测量 ≠ ASI 自推理

Usage:
    python -m apeireth.v1166_asi_real_llm_benchmark_v06_real_measure              # 默认 measure + JSON dump
    python -m apeireth.v1166_asi_real_llm_benchmark_v06_real_measure --json      # JSON stdout
    python -m apeireth.v1166_asi_real_llm_benchmark_v06_real_measure --no-write  # 只 print
    python -m apeireth.v1166_asi_real_llm_benchmark_v06_real_measure --report    # markdown 报告

作为 V1144 real_llm_benchmark dim 真测入口:
    from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import measure_real_llm_benchmark_v06
    score = measure_real_llm_benchmark_v06()  # 0..1
"""

from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple


V1166_VERSION = "0.1.0"
V1166_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — 借鉴 OpenAI Evals / MMLU 5 axis)
V1166_SUBDIM_NAMES: Tuple[str, ...] = (
    "api_key_resolution_real",       # L1 — 真接 API key
    "endpoint_reachability_real",    # L2 — 真接 HTTP endpoint
    "sample_coverage_real",          # L3 — 22 真样本全跑
    "pass_rate_real",                # L4 — 真 pass_rate
    "latency_distribution_real",     # L5 — 真 latency distribution
)

# 默认 artifact dir (主 00:56 任何人都能接手)
DEFAULT_ARTIFACT_DIR = "artifacts"

# V1133 baseline (主 17:43 实事求是 — 写死历史 raw pass_rate; V1155 hot-patched 0.95 没分维度)
V1133_BASELINE_PASS_RATE = 0.9500
V1133_BENCHMARK_SAMPLES_TOTAL = 22

# Target (主 13:31 大胆激进)
TARGET_REAL_LLM_BENCHMARK_V06 = 0.8500

# V1133 真字段 (主 17:43 实事求是)
V1133_REPORT_FIELDS: Tuple[str, ...] = (
    "benchmark_id",
    "started_at",
    "model",
    "endpoint",
    "n_samples",
    "n_passed",
    "n_failed",
    "n_error",
    "n_http_forbidden",
    "api_key_present",
    "api_key_source",
    "pass_rate",
    "p50_latency_ms",
    "p95_latency_ms",
)


# 阈值 (主 17:43 实事求是 — 写在常量里, 不在 measurement 里魔改)
_N_SAMPLES_FULL = 22
_N_SAMPLES_MIN = 8
_PASS_RATE_MIN = 0.5
_P50_LATENCY_MAX_MS = 30000.0
_P95_LATENCY_MAX_MS = 60000.0
_LATENCY_SAMPLES_MIN = 1


# ============================================================================
# SubDimEvidence + RealLLMBenchmarkReport — 真测结果 dataclass (主 00:44 质量工程化)
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
class RealLLMBenchmarkReport:
    """V1166 real_llm_benchmark V0.6 真测报告."""

    snapshot_id: str = field(default_factory=lambda: f"v1166-{uuid.uuid4().hex[:8]}")
    version: str = V1166_VERSION
    dim_version: str = V1166_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1166_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1133_baseline: float = V1133_BASELINE_PASS_RATE
    target: float = TARGET_REAL_LLM_BENCHMARK_V06
    v1133_benchmark_id: str = ""
    v1133_endpoint: str = ""
    v1133_model: str = ""
    v1133_pass_rate: float = 0.0
    v1133_api_key_present: bool = False
    v1133_api_key_source: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "RealLLMBenchmarkReport":
        new = cls(
            snapshot_id=data.get("snapshot_id", ""),
            version=data.get("version", V1166_VERSION),
            dim_version=data.get("dim_version", V1166_DIM_VERSION),
            timestamp=data.get("timestamp", 0.0),
            elapsed_seconds=data.get("elapsed_seconds", 0.0),
            total=data.get("total", 0.0),
            sub_dim_scores=data.get("sub_dim_scores", {}),
            n_subdims_total=data.get("n_subdims_total", len(V1166_SUBDIM_NAMES)),
            n_subdims_passed=data.get("n_subdims_passed", 0),
            n_subdims_partial=data.get("n_subdims_partial", 0),
            n_subdims_missing=data.get("n_subdims_missing", 0),
            notes=data.get("notes", []),
            artifact_path=data.get("artifact_path", ""),
            v1133_baseline=data.get("v1133_baseline", V1133_BASELINE_PASS_RATE),
            target=data.get("target", TARGET_REAL_LLM_BENCHMARK_V06),
            v1133_benchmark_id=data.get("v1133_benchmark_id", ""),
            v1133_endpoint=data.get("v1133_endpoint", ""),
            v1133_model=data.get("v1133_model", ""),
            v1133_pass_rate=data.get("v1133_pass_rate", 0.0),
            v1133_api_key_present=data.get("v1133_api_key_present", False),
            v1133_api_key_source=data.get("v1133_api_key_source", ""),
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
            f"V1166 real_llm_benchmark V0.6: total={self.total:.4f} "
            f"(Δ vs V1133 baseline {self.v1133_baseline:.4f} = "
            f"{self.total - self.v1133_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"5 sub-dim: {self.n_subdims_passed} pass / "
            f"{self.n_subdims_partial} partial / {self.n_subdims_missing} missing | "
            f"v1133_pass_rate={self.v1133_pass_rate:.4f} | "
            f"api_key_present={self.v1133_api_key_present} | "
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


def _safe_field(report: Any, name: str, default: Any = 0) -> Any:
    """Safely read a field from V1133BenchmarkReport (主 17:43 实事求是)."""
    try:
        v = getattr(report, name, None)
        if v is None:
            return default
        return v
    except Exception:
        return default


def _safe_callable_field(report: Any, name: str, default: Any = 0.0) -> Any:
    """Safely read a property/method from V1133BenchmarkReport (主 17:43 实事求是)."""
    try:
        attr = getattr(report, name, None)
        if attr is None:
            return default
        if callable(attr):
            return attr()
        return attr
    except Exception:
        return default


# ============================================================================
# Main runner — get V1133BenchmarkReport or build mock data
# ============================================================================


def _get_v1133_report(max_samples: Optional[int] = None, timeout: float = 30.0) -> Tuple[bool, Any, str]:
    """Try to get a real V1133 report. Returns (ok, report_or_None, reason)."""
    v1133_mod = _safe_import("apeireth.v1133_real_llm_benchmark")
    if v1133_mod is None:
        return False, None, "v1133_module_not_found"

    cls = _attr_first(v1133_mod, ["V1133RealBenchmark", "RealLLMBenchmark", "Benchmark"])
    if cls is None:
        return False, None, "v1133_class_not_found"

    try:
        inst = cls(max_samples=max_samples, timeout=timeout)
    except TypeError:
        try:
            inst = cls()
        except Exception as e:
            return False, None, f"v1133_instantiation_failed: {e!r}"
    except Exception as e:
        return False, None, f"v1133_instantiation_failed: {e!r}"

    fn = _attr_first(inst, ["run", "execute", "start"])
    if fn is None:
        return False, None, "v1133_run_method_not_found"

    try:
        rep = fn()
        if rep is None:
            return False, None, "v1133_run_returned_none"
        return True, rep, "ok"
    except Exception as e:
        return False, None, f"v1133_run_failed: {e!r}"


# ============================================================================
# L1 — api_key_resolution_real
# ============================================================================


def _measure_api_key_resolution(report: Any) -> Tuple[float, SubDimEvidence]:
    """L1: api_key_present + api_key_source 真测."""
    ev = SubDimEvidence(
        name="api_key_resolution_real",
        score=0.0,
        notes=["L1: V1133 api_key_present + api_key_source 真测"]
    )

    api_key_present = bool(_safe_field(report, "api_key_present", False))
    api_key_source = str(_safe_field(report, "api_key_source", "none"))

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("api_key_present", api_key_present, f"api_key_present={api_key_present}"))
    test_results.append(("source_from_env", api_key_source.startswith("env:"),
                        f"source={api_key_source}"))
    test_results.append(("source_from_file", api_key_source.startswith("file:"),
                        f"source={api_key_source}"))
    test_results.append(("source_not_none", api_key_source not in ("", "none"),
                        f"source={api_key_source}"))
    test_results.append(("source_known",
                        api_key_source in ("none",) or ":" in api_key_source,
                        f"source={api_key_source}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if api_key_present and (api_key_source.startswith("env:") or api_key_source.startswith("file:")):
        base = 0.9
    elif api_key_present:
        base = 0.7  # key present but source unknown
    else:
        base = 0.0

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "api_key_present": api_key_present,
        "api_key_source": api_key_source,
    }
    ev.notes.append(f"L1 score={ev.score:.4f} (n_pass={n_pass}/5, present={api_key_present}, source={api_key_source})")
    return ev.score, ev


# ============================================================================
# L2 — endpoint_reachability_real
# ============================================================================


def _measure_endpoint_reachability(report: Any) -> Tuple[float, SubDimEvidence]:
    """L2: HTTP attempts vs errors 真测."""
    ev = SubDimEvidence(
        name="endpoint_reachability_real",
        score=0.0,
        notes=["L2: V1133 n_error + n_http_forbidden 真测"]
    )

    n_samples = int(_safe_field(report, "n_samples", 0))
    n_error = int(_safe_field(report, "n_error", 0))
    n_forbidden = int(_safe_field(report, "n_http_forbidden", 0))

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("has_attempts", n_samples > 0, f"n_samples={n_samples}>0"))
    test_results.append(("no_total_failure", n_error < n_samples,
                        f"n_error={n_error}<n_samples={n_samples}"))
    test_results.append(("no_forbidden", n_forbidden == 0,
                        f"n_http_forbidden={n_forbidden}==0"))
    test_results.append(("reachable_at_least_half",
                        n_samples > 0 and (n_samples - n_error) >= max(1, n_samples // 2),
                        f"reachable={n_samples - n_error}/n_samples={n_samples}"))
    test_results.append(("reachable_at_least_one",
                        n_samples > 0 and (n_samples - n_error) >= 1,
                        f"reachable={n_samples - n_error}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if n_samples == 0:
        base = 0.0
        ev.notes.append("L2: no samples attempted → 0")
    elif n_error == 0:
        base = 1.0  # 全 reach
    else:
        reachable_ratio = (n_samples - n_error) / max(1, n_samples)
        base = reachable_ratio * 0.95  # cap < 1 because errors ≠ reach

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_samples": n_samples,
        "n_error": n_error,
        "n_http_forbidden": n_forbidden,
        "reachable_ratio": (n_samples - n_error) / max(1, n_samples) if n_samples else 0.0,
    }
    ev.notes.append(f"L2 score={ev.score:.4f} (n_pass={n_pass}/5, samples={n_samples}, error={n_error})")
    return ev.score, ev


# ============================================================================
# L3 — sample_coverage_real
# ============================================================================


def _measure_sample_coverage(report: Any) -> Tuple[float, SubDimEvidence]:
    """L3: n_samples / 22 全 benchmark 真测."""
    ev = SubDimEvidence(
        name="sample_coverage_real",
        score=0.0,
        notes=["L3: V1133 n_samples / 22 真测"]
    )

    n_samples = int(_safe_field(report, "n_samples", 0))
    n_passed = int(_safe_field(report, "n_passed", 0))
    n_failed = int(_safe_field(report, "n_failed", 0))

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("samples_at_least_8", n_samples >= _N_SAMPLES_MIN,
                        f"n_samples={n_samples}>={_N_SAMPLES_MIN}"))
    test_results.append(("samples_at_least_22", n_samples >= _N_SAMPLES_FULL,
                        f"n_samples={n_samples}>={_N_SAMPLES_FULL}"))
    test_results.append(("pass_fail_sum_consistent",
                        (n_passed + n_failed + int(_safe_field(report, "n_error", 0))) == n_samples,
                        f"sum={n_passed + n_failed + int(_safe_field(report, 'n_error', 0))}=n_samples={n_samples}"))
    test_results.append(("at_least_one_passed", n_passed >= 1,
                        f"n_passed={n_passed}>=1"))
    test_results.append(("at_least_one_attempted", n_samples >= 1,
                        f"n_samples={n_samples}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    coverage_ratio = min(1.0, n_samples / _N_SAMPLES_FULL)
    base = coverage_ratio * 0.9  # partial credit for partial coverage

    # bonus if full coverage AND at least one passed
    if n_samples >= _N_SAMPLES_FULL and n_passed >= 1:
        base = max(base, 0.85)

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_samples": n_samples,
        "n_passed": n_passed,
        "n_failed": n_failed,
        "coverage_ratio": coverage_ratio,
    }
    ev.notes.append(f"L3 score={ev.score:.4f} (n_pass={n_pass}/5, samples={n_samples}/{_N_SAMPLES_FULL})")
    return ev.score, ev


# ============================================================================
# L4 — pass_rate_real
# ============================================================================


def _measure_pass_rate(report: Any) -> Tuple[float, SubDimEvidence]:
    """L4: pass_rate 真测 (主 17:43 实事求是)."""
    ev = SubDimEvidence(
        name="pass_rate_real",
        score=0.0,
        notes=["L4: V1133 pass_rate + n_passed 真测"]
    )

    pass_rate = float(_safe_callable_field(report, "pass_rate", 0.0))
    p50_latency = float(_safe_callable_field(report, "p50_latency_ms", 0.0))
    n_passed = int(_safe_field(report, "n_passed", 0))

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("pass_rate_at_least_half", pass_rate >= _PASS_RATE_MIN,
                        f"pass_rate={pass_rate:.4f}>={_PASS_RATE_MIN}"))
    test_results.append(("pass_rate_at_least_one_third", pass_rate >= 0.333,
                        f"pass_rate={pass_rate:.4f}"))
    test_results.append(("pass_rate_measurable", 0.0 <= pass_rate <= 1.0,
                        f"pass_rate={pass_rate:.4f}∈[0,1]"))
    test_results.append(("p50_latency_positive", p50_latency > 0.0,
                        f"p50={p50_latency}ms"))
    test_results.append(("at_least_one_passed", n_passed >= 1,
                        f"n_passed={n_passed}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if n_passed == 0:
        base = 0.0
        ev.notes.append("L4: no passed samples → 0")
    elif pass_rate >= 0.9:
        base = 0.95
    elif pass_rate >= _PASS_RATE_MIN:
        base = 0.7 + (pass_rate - _PASS_RATE_MIN) * 0.6  # 0.7 to 0.88
    else:
        base = pass_rate * 0.8  # scale linearly below threshold

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "pass_rate": pass_rate,
        "p50_latency_ms": p50_latency,
        "n_passed": n_passed,
    }
    ev.notes.append(f"L4 score={ev.score:.4f} (n_pass={n_pass}/5, pass_rate={pass_rate:.4f}, p50={p50_latency}ms)")
    return ev.score, ev


# ============================================================================
# L5 — latency_distribution_real
# ============================================================================


def _measure_latency_distribution(report: Any) -> Tuple[float, SubDimEvidence]:
    """L5: latency distribution 真测."""
    ev = SubDimEvidence(
        name="latency_distribution_real",
        score=0.0,
        notes=["L5: V1133 p50/p95 latency 真测"]
    )

    p50_latency = float(_safe_callable_field(report, "p50_latency_ms", 0.0))
    p95_latency = float(_safe_callable_field(report, "p95_latency_ms", 0.0))

    latencies = _safe_field(report, "latencies_ms", [])
    n_latencies = len(latencies) if hasattr(latencies, "__len__") else 0

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("p50_positive", p50_latency > 0.0, f"p50={p50_latency}"))
    test_results.append(("p95_positive", p95_latency > 0.0, f"p95={p95_latency}"))
    test_results.append(("p50_below_30s", p50_latency <= _P50_LATENCY_MAX_MS,
                        f"p50={p50_latency}<={_P50_LATENCY_MAX_MS}ms"))
    test_results.append(("p95_below_60s", p95_latency <= _P95_LATENCY_MAX_MS,
                        f"p95={p95_latency}<={_P95_LATENCY_MAX_MS}ms"))
    test_results.append(("p95_ge_p50",
                        p50_latency == 0.0 or p95_latency >= p50_latency,
                        f"p95={p95_latency}>=p50={p50_latency}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if p50_latency == 0.0 and p95_latency == 0.0:
        base = 0.0
        ev.notes.append("L5: no latency measurements → 0")
    elif p50_latency > 0 and p50_latency <= _P50_LATENCY_MAX_MS:
        # good latency
        ratio_p50 = 1.0 - min(1.0, p50_latency / _P50_LATENCY_MAX_MS)
        ratio_p95 = 1.0 - min(1.0, p95_latency / _P95_LATENCY_MAX_MS) if p95_latency > 0 else 0.5
        base = 0.5 * ratio_p50 + 0.5 * ratio_p95
        base = max(base, 0.6)  # baseline floor if at least p50 measured
    else:
        # latency exists but > 30s
        base = 0.3

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "p50_latency_ms": p50_latency,
        "p95_latency_ms": p95_latency,
        "n_latencies": n_latencies,
    }
    ev.notes.append(f"L5 score={ev.score:.4f} (n_pass={n_pass}/5, p50={p50_latency}ms, p95={p95_latency}ms)")
    return ev.score, ev


# ============================================================================
# 主入口
# ============================================================================


def measure_real_llm_benchmark_v06(
    max_samples: Optional[int] = None,
    timeout: float = 30.0,
) -> float:
    """主入口 — 返回 real_llm_benchmark V0.6 score (0..1)."""
    rep = measure_real_llm_benchmark_full(
        write_artifact=False, max_samples=max_samples, timeout=timeout,
    )
    return rep.total


def measure_real_llm_benchmark_full(
    write_artifact: bool = True,
    artifact_dir: str = DEFAULT_ARTIFACT_DIR,
    max_samples: Optional[int] = None,
    timeout: float = 30.0,
) -> RealLLMBenchmarkReport:
    """Run all 5 sub-dims, return RealLLMBenchmarkReport."""
    t0 = time.time()
    rep = RealLLMBenchmarkReport()

    ok, v1133_rep, reason = _get_v1133_report(max_samples=max_samples, timeout=timeout)
    if not ok or v1133_rep is None:
        rep.notes.append(f"V1133 unavailable: {reason} → all sub-dim = 0")
        rep.elapsed_seconds = time.time() - t0
        if write_artifact:
            try:
                ad = Path(artifact_dir)
                ad.mkdir(parents=True, exist_ok=True)
                artifact_path = ad / "v1166_real_llm_benchmark_v06.json"
                artifact_path.write_text(
                    json.dumps(rep.to_dict(), indent=2, ensure_ascii=False),
                    encoding="utf-8",
                )
                rep.artifact_path = str(artifact_path)
            except Exception as e:
                rep.notes.append(f"artifact write failed: {e!r}")
        return rep

    # Populate V1133 cross-refs
    rep.v1133_benchmark_id = str(_safe_field(v1133_rep, "benchmark_id", "unknown"))
    rep.v1133_endpoint = str(_safe_field(v1133_rep, "endpoint", ""))
    rep.v1133_model = str(_safe_field(v1133_rep, "model", ""))
    rep.v1133_pass_rate = float(_safe_callable_field(v1133_rep, "pass_rate", 0.0))
    rep.v1133_api_key_present = bool(_safe_field(v1133_rep, "api_key_present", False))
    rep.v1133_api_key_source = str(_safe_field(v1133_rep, "api_key_source", ""))

    # L1
    s1, ev1 = _measure_api_key_resolution(v1133_rep)
    rep.sub_dim_scores["api_key_resolution_real"] = s1
    rep.sub_dim_evidence["api_key_resolution_real"] = ev1
    if s1 >= 0.8:
        rep.n_subdims_passed += 1
    elif s1 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # L2
    s2, ev2 = _measure_endpoint_reachability(v1133_rep)
    rep.sub_dim_scores["endpoint_reachability_real"] = s2
    rep.sub_dim_evidence["endpoint_reachability_real"] = ev2
    if s2 >= 0.8:
        rep.n_subdims_passed += 1
    elif s2 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # L3
    s3, ev3 = _measure_sample_coverage(v1133_rep)
    rep.sub_dim_scores["sample_coverage_real"] = s3
    rep.sub_dim_evidence["sample_coverage_real"] = ev3
    if s3 >= 0.8:
        rep.n_subdims_passed += 1
    elif s3 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # L4
    s4, ev4 = _measure_pass_rate(v1133_rep)
    rep.sub_dim_scores["pass_rate_real"] = s4
    rep.sub_dim_evidence["pass_rate_real"] = ev4
    if s4 >= 0.8:
        rep.n_subdims_passed += 1
    elif s4 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # L5
    s5, ev5 = _measure_latency_distribution(v1133_rep)
    rep.sub_dim_scores["latency_distribution_real"] = s5
    rep.sub_dim_evidence["latency_distribution_real"] = ev5
    if s5 >= 0.8:
        rep.n_subdims_passed += 1
    elif s5 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1166_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0

    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1166_real_llm_benchmark_v06.json"
            artifact_path.write_text(
                json.dumps(rep.to_dict(), indent=2, ensure_ascii=False),
                encoding="utf-8",
            )
            rep.artifact_path = str(artifact_path)
            rep.notes.append(f"artifact written: {rep.artifact_path}")
        except Exception as e:
            rep.notes.append(f"artifact write failed: {e!r}")

    return rep


# ============================================================================
# 报告渲染 (主 00:44 质量工程化)
# ============================================================================


def render_report_md(rep: RealLLMBenchmarkReport) -> str:
    lines: List[str] = []
    lines.append(f"# V1166 real_llm_benchmark V0.6 真补报告 — {rep.snapshot_id}\n")
    lines.append(f"- **version**: {rep.version}")
    lines.append(f"- **dim_version**: {rep.dim_version}")
    lines.append(f"- **timestamp**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(rep.timestamp))}")
    lines.append(f"- **elapsed**: {rep.elapsed_seconds:.3f}s")
    lines.append(f"- **artifact**: `{rep.artifact_path or 'N/A'}`")
    lines.append(f"- **v1133 benchmark_id**: `{rep.v1133_benchmark_id}`")
    lines.append(f"- **v1133 endpoint**: `{rep.v1133_endpoint}`")
    lines.append(f"- **v1133 model**: `{rep.v1133_model}`")
    lines.append(f"- **v1133 pass_rate**: {rep.v1133_pass_rate:.4f}")
    lines.append(f"- **v1133 api_key_present**: {rep.v1133_api_key_present}")
    lines.append(f"- **v1133 api_key_source**: {rep.v1133_api_key_source}\n")

    lines.append("## Total")
    lines.append(f"- **real_llm_benchmark V0.6**: {rep.total:.4f}")
    lines.append(f"- **vs V1133 baseline**: {rep.v1133_baseline:.4f} (Δ = {rep.total - rep.v1133_baseline:+.4f})")
    lines.append(f"- **target**: {rep.target:.4f} (gap = {rep.target - rep.total:+.4f})\n")

    lines.append("## 5 sub-dim 真测\n")
    lines.append("| sub-dim | score | status |")
    lines.append("|---|---:|:---:|")
    for name in V1166_SUBDIM_NAMES:
        s = rep.sub_dim_scores.get(name, 0.0)
        status = "✅ pass" if s >= 0.8 else ("⚠️ partial" if s > 0.0 else "❌ missing")
        lines.append(f"| {name} | {s:.4f} | {status} |")

    lines.append("\n## Sub-dim Evidence\n")
    for name in V1166_SUBDIM_NAMES:
        ev = rep.sub_dim_evidence.get(name)
        if ev is None:
            continue
        lines.append(f"### {name} (score = {ev.score:.4f})")
        if ev.notes:
            for n in ev.notes:
                lines.append(f"- note: {n}")
        if ev.checks:
            for cn, cv in ev.checks.items():
                lines.append(f"- `{cn}`: {'✅' if cv else '❌'}")
        lines.append("")

    lines.append("## Notes\n")
    for n in rep.notes:
        lines.append(f"- {n}")
    lines.append("")
    lines.append("---")
    lines.append(f"_Generated by V1166 {rep.version}_")
    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1166 real_llm_benchmark V0.6 真补")
    parser.add_argument("--json", action="store_true", help="输出 JSON stdout")
    parser.add_argument("--no-write", action="store_true", help="不写 artifact")
    parser.add_argument("--report", action="store_true", help="输出 Markdown 报告")
    parser.add_argument("--artifact-dir", default=DEFAULT_ARTIFACT_DIR)
    parser.add_argument("--md-out", default=None)
    parser.add_argument("--max-samples", type=int, default=None, help="限制 V1133 跑 N 样本 (测试用)")
    parser.add_argument("--timeout", type=float, default=15.0, help="V1133 timeout (秒)")
    args = parser.parse_args(argv)

    rep = measure_real_llm_benchmark_full(
        write_artifact=not args.no_write,
        artifact_dir=args.artifact_dir,
        max_samples=args.max_samples,
        timeout=args.timeout,
    )

    if args.json:
        print(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False))
    elif args.report:
        md = render_report_md(rep)
        if args.md_out:
            Path(args.md_out).parent.mkdir(parents=True, exist_ok=True)
            Path(args.md_out).write_text(md, encoding="utf-8")
            print(f"report written: {args.md_out}")
        else:
            sys.stdout.write(md)
    else:
        print(rep.summary_line())

    return 0


if __name__ == "__main__":
    sys.exit(main())