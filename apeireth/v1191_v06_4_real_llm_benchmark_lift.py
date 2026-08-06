"""V1191 — V1166 real_llm_benchmark V0.6.4 lift (使用 V1190 真 LLM 数据).

主 06:15 V1051 真 benchmark 接入 LLM API + 主 22:33 ASI 北极星 + 主 17:43 实事求是 +
主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 +
主 00:56 任何人都能接手 + 主 00:44 质量工程化.

为什么 V1191:
  V1166 = ASI real_llm_benchmark V0.6 真补 (5 sub-dim 真测, V1133 驱动)
  V1133 默认 endpoint = api.MiniMax.chat → SSL hostname mismatch → 全 error → V1166 = 0.416
  V1190 = V1133 修复版 (endpoint + model 修复 → HTTP 200 + content 正常) → total 0.7281
  V1191 = V1166 V0.6.4 lift: 用 V1190 数据重算 V1166 5 sub-dim:
    - api_key_resolution_real    (V1190.api_key_present + source) → 1.0
    - endpoint_reachability_real (V1190.n_samples vs n_error)       → 1.0
    - sample_coverage_real       (V1190.n_samples / 22)             → 1.0
    - pass_rate_real             (V1190.pass_rate ≥ 0.5)            → ~0.65+
    - latency_distribution_real  (V1190.p50/p95 latency)            → ~0.99
  V1191 = mean(5 sub-dim) → 真实 V1166 lift score

V1191 vs V1166:
  V1166: 5 sub-dim measurement of V1133 (V1133 fail → 0.416)
  V1191: 5 sub-dim measurement of V1190 (V1190 succeed → ~0.95)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
  - 不假装 V1191 = ASI 北极星 (V1191 = real_llm_benchmark dim lift, 不是 ASI 总)
  - 不假装 V1190 = V1133 全替代 (V1190 = V1133 endpoint fix; V1133 仍是 reference)
  - 不假装 HTTP 200 = ASI 推理 (HTTP 200 + content match = 1 sample pass; ASI 推理 ≠ 一过式)
  - 不假装 V1191 pass_rate = ASI 总 (pass_rate 是 5 sub-dim 之一, 不是 ASI 总)
  - 不假装 V1191 = ASI V1.0 (V1191 是 V0.6.4 中间版本)

主 00:56 任何人都能接手:
  - measure_v1191() → float (0..1) 主入口 (V1166 V0.6.4 lift score)
  - run_v1191_full() → V1191Report dataclass + JSON dump
  - V1191Report JSON 写 artifacts/v1191_v06_4_real_llm_benchmark_lift.json
  - 默认从 artifacts/v1190_real_llm_working.json 读 (V1190 跑出来的 artifact)
  - 也支持直接调 V1190 inline (--realtime 模式)

主 00:44 质量工程化:
  V1191Report:
    snapshot_id, version, timestamp, elapsed_seconds
    total (V1166 V0.6.4 lift score, mean of 5 sub-dim)
    sub_dim_scores (dict 5 keys): api_key/endpoint/coverage/pass_rate/latency
    sub_dim_evidence (dict 5 keys): raw data + checks
    v1190_snapshot_id, v1190_total, v1190_pass_rate
    vs_v1166_baseline (V1166 = 0.416 → delta +X)
    vs_v1182 (V1182 had V1166 = 0.416 → ASI baseline lift)

Usage:
    python -m apeireth.v1191_v06_4_real_llm_benchmark_lift              # 默认从 artifact 读
    python -m apeireth.v1191_v06_4_real_llm_benchmark_lift --realtime  # 实时跑 V1190
    python -m apeireth.v1191_v06_4_real_llm_benchmark_lift --json      # JSON stdout
    python -m apeireth.v1191_v06_4_real_llm_benchmark_lift --report    # Markdown report
    python -m apeireth.v1191_v06_4_real_llm_benchmark_lift --measure   # 只 print measure
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

V1191_VERSION = "0.1.0"
V1191_DIM_VERSION = "0.6.4"

# 5 sub-dim names (与 V1166 同步, 主 19:33 走在前人经验上)
V1191_SUBDIM_NAMES: Tuple[str, ...] = (
    "api_key_resolution_real",
    "endpoint_reachability_real",
    "sample_coverage_real",
    "pass_rate_real",
    "latency_distribution_real",
)

# 阈值 (主 17:43 实事求是 — 与 V1166 同步)
_N_SAMPLES_FULL = 22
_PASS_RATE_MIN = 0.5
_P50_LATENCY_MAX_MS = 30000.0
_P95_LATENCY_MAX_MS = 60000.0

# 历史 baseline (主 17:43 — 写死历史值)
V1166_BASELINE = 0.4160
V1133_BASELINE = 0.9500
V1182_BASELINE = 0.7425
V1189_BASELINE = 0.8903
NORTH_STAR = 0.9800

# 默认 artifact paths
DEFAULT_V1190_ARTIFACT = "artifacts/v1190_real_llm_working.json"


# ============================================================================
# Dataclasses — 主 00:44 质量工程化
# ============================================================================


@dataclass
class V1191SubDimEvidence:
    name: str
    score: float
    checks: Dict[str, bool] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)
    raw: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class V1191Report:
    """V1191 V1166 V0.6.4 lift 真补报告."""

    snapshot_id: str = field(default_factory=lambda: f"v1191-{uuid.uuid4().hex[:8]}")
    version: str = V1191_VERSION
    dim_version: str = V1191_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0  # V1166 V0.6.4 lift score (mean of 5 sub-dim)
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, V1191SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1191_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    v1190_snapshot_id: str = ""
    v1190_total: float = 0.0
    v1190_pass_rate: float = 0.0
    v1190_n_samples: int = 0
    v1190_n_passed: int = 0
    v1190_n_error: int = 0
    v1190_endpoint: str = ""
    v1190_model: str = ""
    v1190_api_key_present: bool = False
    v1190_api_key_source: str = ""
    v1190_p50_latency_ms: float = 0.0
    v1190_p95_latency_ms: float = 0.0
    v1166_baseline: float = V1166_BASELINE
    v1133_baseline: float = V1133_BASELINE
    v1182_baseline: float = V1182_BASELINE
    v1189_baseline: float = V1189_BASELINE
    north_star: float = NORTH_STAR
    artifact_path: str = ""
    notes: List[str] = field(default_factory=list)
    source: str = ""  # "artifact" or "realtime"

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "V1191Report":
        new = cls()
        for k, v in data.items():
            if k == "sub_dim_evidence":
                continue
            if hasattr(new, k):
                setattr(new, k, v)
        raw_evidence = data.get("sub_dim_evidence", {})
        for k, v in raw_evidence.items():
            new.sub_dim_evidence[k] = V1191SubDimEvidence(
                name=v.get("name", k),
                score=v.get("score", 0.0),
                checks=v.get("checks", {}),
                notes=v.get("notes", []),
                raw=v.get("raw", {}),
            )
        return new

    def summary_line(self) -> str:
        return (
            f"V1191 V1166 V0.6.4 lift: total={self.total:.4f} | "
            f"5 sub-dim: {self.n_subdims_passed} pass / {self.n_subdims_partial} partial / {self.n_subdims_missing} missing | "
            f"v1190 total={self.v1190_total:.4f} pass_rate={self.v1190_pass_rate:.4f} | "
            f"vs V1166 baseline 0.416: delta {self.total - V1166_BASELINE:+.4f} | "
            f"source={self.source} snapshot={self.snapshot_id}"
        )


# ============================================================================
# Helpers
# ============================================================================


def _read_v1190_artifact(artifact_path: str) -> Tuple[bool, Optional[Dict[str, Any]], str]:
    """Read V1190 artifact JSON. Returns (ok, dict_or_None, reason)."""
    p = Path(artifact_path)
    if not p.is_file():
        return False, None, f"artifact_not_found: {artifact_path}"
    try:
        data = json.loads(p.read_text(encoding="utf-8"))
        return True, data, "ok"
    except Exception as e:
        return False, None, f"artifact_parse_failed: {e!r}"


def _safe_import_run_v1190(max_samples: Optional[int], timeout: float) -> Tuple[bool, Optional[Any], str]:
    """Real-time run V1190 inline. Returns (ok, v1190_report_dict, reason)."""
    try:
        from apeireth import v1190_real_llm_working_benchmark as v1190_mod
    except Exception as e:
        return False, None, f"v1190_import_failed: {e!r}"
    try:
        rep = v1190_mod.run_v1190_full(max_samples=max_samples, timeout=timeout)
        return True, rep.to_dict(), "ok"
    except Exception as e:
        return False, None, f"v1190_run_failed: {e!r}"


# ============================================================================
# 5 sub-dim measurement (主 17:43 实事求是 — 与 V1166 同步公式)
# ============================================================================


def _measure_api_key_from_v1190(v1190: Dict[str, Any]) -> Tuple[float, V1191SubDimEvidence]:
    """L1 — api_key_resolution_real (from V1190.api_key_present + api_key_source)."""
    ev = V1191SubDimEvidence(name="api_key_resolution_real", score=0.0)
    api_key_present = bool(v1190.get("api_key_present", False))
    api_key_source = str(v1190.get("api_key_source", "none"))

    checks: Dict[str, bool] = {
        "api_key_present": api_key_present,
        "source_from_env": api_key_source.startswith("env:"),
        "source_from_file": api_key_source.startswith("file:"),
        "source_not_none": api_key_source not in ("", "none"),
        "source_known": api_key_source in ("none",) or ":" in api_key_source,
    }
    n_pass = sum(1 for v in checks.values() if v)

    if api_key_present and (api_key_source.startswith("env:") or api_key_source.startswith("file:")):
        base = 0.9
    elif api_key_present:
        base = 0.7
    else:
        base = 0.0

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = checks
    ev.raw = {
        "api_key_present": api_key_present,
        "api_key_source": api_key_source,
        "n_pass": n_pass,
    }
    ev.notes.append(f"L1 score={ev.score:.4f} (present={api_key_present}, source={api_key_source})")
    return ev.score, ev


def _measure_endpoint_from_v1190(v1190: Dict[str, Any]) -> Tuple[float, V1191SubDimEvidence]:
    """L2 — endpoint_reachability_real (from V1190.n_samples vs n_error)."""
    ev = V1191SubDimEvidence(name="endpoint_reachability_real", score=0.0)
    n_samples = int(v1190.get("n_samples", 0))
    n_error = int(v1190.get("n_error", 0))
    n_forbidden = int(v1190.get("n_http_forbidden", 0))

    checks = {
        "has_attempts": n_samples > 0,
        "no_total_failure": n_error < n_samples,
        "no_forbidden": n_forbidden == 0,
        "reachable_at_least_half": n_samples > 0 and (n_samples - n_error) >= max(1, n_samples // 2),
        "reachable_at_least_one": n_samples > 0 and (n_samples - n_error) >= 1,
    }
    n_pass = sum(1 for v in checks.values() if v)

    if n_samples == 0:
        base = 0.0
    elif n_error == 0:
        base = 1.0
    else:
        reachable_ratio = (n_samples - n_error) / max(1, n_samples)
        base = reachable_ratio * 0.95

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = checks
    ev.raw = {
        "n_samples": n_samples,
        "n_error": n_error,
        "n_http_forbidden": n_forbidden,
        "reachable_ratio": (n_samples - n_error) / max(1, n_samples) if n_samples else 0.0,
        "n_pass": n_pass,
    }
    ev.notes.append(f"L2 score={ev.score:.4f} (samples={n_samples}, error={n_error})")
    return ev.score, ev


def _measure_coverage_from_v1190(v1190: Dict[str, Any]) -> Tuple[float, V1191SubDimEvidence]:
    """L3 — sample_coverage_real (from V1190.n_samples / 22)."""
    ev = V1191SubDimEvidence(name="sample_coverage_real", score=0.0)
    n_samples = int(v1190.get("n_samples", 0))
    n_passed = int(v1190.get("n_passed", 0))
    n_failed = int(v1190.get("n_failed", 0))
    n_error = int(v1190.get("n_error", 0))

    checks = {
        "samples_at_least_8": n_samples >= 8,
        "samples_at_least_22": n_samples >= _N_SAMPLES_FULL,
        "pass_fail_sum_consistent": (n_passed + n_failed + n_error) == n_samples,
        "at_least_one_passed": n_passed >= 1,
        "at_least_one_attempted": n_samples >= 1,
    }
    n_pass = sum(1 for v in checks.values() if v)

    coverage_ratio = min(1.0, n_samples / _N_SAMPLES_FULL)
    base = coverage_ratio * 0.9
    if n_samples >= _N_SAMPLES_FULL and n_passed >= 1:
        base = max(base, 0.85)

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = checks
    ev.raw = {
        "n_samples": n_samples,
        "n_passed": n_passed,
        "n_failed": n_failed,
        "coverage_ratio": coverage_ratio,
        "n_pass": n_pass,
    }
    ev.notes.append(f"L3 score={ev.score:.4f} (samples={n_samples}/{_N_SAMPLES_FULL})")
    return ev.score, ev


def _measure_pass_rate_from_v1190(v1190: Dict[str, Any]) -> Tuple[float, V1191SubDimEvidence]:
    """L4 — pass_rate_real (from V1190.pass_rate ≥ 0.5)."""
    ev = V1191SubDimEvidence(name="pass_rate_real", score=0.0)
    n_samples = int(v1190.get("n_samples", 0))
    n_passed = int(v1190.get("n_passed", 0))
    pass_rate = float(v1190.get("pass_rate", 0.0))
    p50 = float(v1190.get("p50_latency_ms", 0.0))

    checks = {
        "pass_rate_at_least_half": pass_rate >= _PASS_RATE_MIN,
        "pass_rate_at_least_one_third": pass_rate >= 0.333,
        "pass_rate_measurable": 0.0 <= pass_rate <= 1.0,
        "p50_latency_positive": p50 > 0.0,
        "at_least_one_passed": n_passed >= 1,
    }
    n_pass = sum(1 for v in checks.values() if v)

    if n_passed == 0:
        base = 0.0
    elif pass_rate >= 0.9:
        base = 0.95
    elif pass_rate >= _PASS_RATE_MIN:
        base = 0.7 + (pass_rate - _PASS_RATE_MIN) * 0.6
    else:
        base = pass_rate * 0.8

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = checks
    ev.raw = {
        "pass_rate": pass_rate,
        "n_passed": n_passed,
        "n_samples": n_samples,
        "p50_latency_ms": p50,
        "n_pass": n_pass,
    }
    ev.notes.append(f"L4 score={ev.score:.4f} (pass_rate={pass_rate:.4f}, n_passed={n_passed})")
    return ev.score, ev


def _measure_latency_from_v1190(v1190: Dict[str, Any]) -> Tuple[float, V1191SubDimEvidence]:
    """L5 — latency_distribution_real (from V1190.p50/p95 latency)."""
    ev = V1191SubDimEvidence(name="latency_distribution_real", score=0.0)
    p50 = float(v1190.get("p50_latency_ms", 0.0))
    p95 = float(v1190.get("p95_latency_ms", 0.0))

    checks = {
        "p50_positive": p50 > 0.0,
        "p95_positive": p95 > 0.0,
        "p50_below_30s": p50 <= _P50_LATENCY_MAX_MS,
        "p95_below_60s": p95 <= _P95_LATENCY_MAX_MS,
        "p95_ge_p50": p50 == 0.0 or p95 >= p50,
    }
    n_pass = sum(1 for v in checks.values() if v)

    if p50 == 0.0 and p95 == 0.0:
        base = 0.0
    elif p50 > 0 and p50 <= _P50_LATENCY_MAX_MS:
        ratio_p50 = 1.0 - min(1.0, p50 / _P50_LATENCY_MAX_MS)
        ratio_p95 = 1.0 - min(1.0, p95 / _P95_LATENCY_MAX_MS) if p95 > 0 else 0.5
        base = 0.5 * ratio_p50 + 0.5 * ratio_p95
        base = max(base, 0.6)
    else:
        base = 0.3

    check_bonus = float(n_pass) / 5.0 * 0.1
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = checks
    ev.raw = {
        "p50_latency_ms": p50,
        "p95_latency_ms": p95,
        "n_pass": n_pass,
    }
    ev.notes.append(f"L5 score={ev.score:.4f} (p50={p50:.0f}ms, p95={p95:.0f}ms)")
    return ev.score, ev


# ============================================================================
# Main runner
# ============================================================================


def _run_v1191_full(
    source: str = "artifact",
    v1190_artifact_path: str = DEFAULT_V1190_ARTIFACT,
    max_samples: Optional[int] = None,
    timeout: float = 30.0,
    write_artifact: bool = True,
    artifact_dir: str = "artifacts",
    artifact_name: str = "v1191_v06_4_real_llm_benchmark_lift.json",
) -> V1191Report:
    """Run V1191 lift, return V1191Report."""
    t0 = time.time()
    rep = V1191Report()

    if source == "realtime":
        ok, v1190_data, reason = _safe_import_run_v1190(max_samples, timeout)
        rep.notes.append(f"V1190 realtime run: {reason}")
    else:
        ok, v1190_data, reason = _read_v1190_artifact(v1190_artifact_path)
        rep.notes.append(f"V1190 artifact read ({v1190_artifact_path}): {reason}")

    if not ok or v1190_data is None:
        rep.notes.append("V1190 unavailable → all sub-dim = 0")
        rep.elapsed_seconds = time.time() - t0
        if write_artifact:
            _write_artifact(rep, artifact_dir, artifact_name)
        return rep

    rep.source = source
    rep.v1190_snapshot_id = str(v1190_data.get("snapshot_id", ""))
    rep.v1190_total = float(v1190_data.get("total", 0.0))
    rep.v1190_pass_rate = float(v1190_data.get("pass_rate", 0.0))
    rep.v1190_n_samples = int(v1190_data.get("n_samples", 0))
    rep.v1190_n_passed = int(v1190_data.get("n_passed", 0))
    rep.v1190_n_error = int(v1190_data.get("n_error", 0))
    rep.v1190_endpoint = str(v1190_data.get("endpoint", ""))
    rep.v1190_model = str(v1190_data.get("model", ""))
    rep.v1190_api_key_present = bool(v1190_data.get("api_key_present", False))
    rep.v1190_api_key_source = str(v1190_data.get("api_key_source", ""))
    rep.v1190_p50_latency_ms = float(v1190_data.get("p50_latency_ms", 0.0))
    rep.v1190_p95_latency_ms = float(v1190_data.get("p95_latency_ms", 0.0))

    # 5 sub-dim measurement
    s1, ev1 = _measure_api_key_from_v1190(v1190_data)
    s2, ev2 = _measure_endpoint_from_v1190(v1190_data)
    s3, ev3 = _measure_coverage_from_v1190(v1190_data)
    s4, ev4 = _measure_pass_rate_from_v1190(v1190_data)
    s5, ev5 = _measure_latency_from_v1190(v1190_data)

    for score, ev in [(s1, ev1), (s2, ev2), (s3, ev3), (s4, ev4), (s5, ev5)]:
        rep.sub_dim_scores[ev.name] = score
        rep.sub_dim_evidence[ev.name] = ev
        if score >= 0.8:
            rep.n_subdims_passed += 1
        elif score > 0.0:
            rep.n_subdims_partial += 1
        else:
            rep.n_subdims_missing += 1

    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1191_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0
    rep.notes.append(
        f"V1191 total={rep.total:.4f} (mean 5 sub-dim); "
        f"V1190 backing: pass_rate={rep.v1190_pass_rate:.4f} ({rep.v1190_n_passed}/{rep.v1190_n_samples}); "
        f"vs V1166 baseline 0.416 → delta {rep.total - V1166_BASELINE:+.4f}"
    )

    if write_artifact:
        _write_artifact(rep, artifact_dir, artifact_name)
    return rep


def _write_artifact(rep: V1191Report, artifact_dir: str, artifact_name: str) -> None:
    try:
        ad = Path(artifact_dir)
        ad.mkdir(parents=True, exist_ok=True)
        artifact_path = ad / artifact_name
        artifact_path.write_text(
            json.dumps(rep.to_dict(), indent=2, ensure_ascii=False),
            encoding="utf-8",
        )
        rep.artifact_path = str(artifact_path)
        rep.notes.append(f"artifact written: {rep.artifact_path}")
    except Exception as e:
        rep.notes.append(f"artifact write failed: {e!r}")


# ============================================================================
# Main entry
# ============================================================================


def measure_v1191(
    source: str = "artifact",
    v1190_artifact_path: str = DEFAULT_V1190_ARTIFACT,
    max_samples: Optional[int] = None,
    timeout: float = 30.0,
) -> float:
    """V1191 measure → V1166 V0.6.4 lift score (0..1). 主入口 (主 00:56)."""
    rep = _run_v1191_full(
        source=source,
        v1190_artifact_path=v1190_artifact_path,
        max_samples=max_samples,
        timeout=timeout,
        write_artifact=False,
    )
    return rep.total


def run_v1191_full(
    source: str = "artifact",
    v1190_artifact_path: str = DEFAULT_V1190_ARTIFACT,
    max_samples: Optional[int] = None,
    timeout: float = 30.0,
) -> V1191Report:
    """Run full V1191 lift, write artifact, return V1191Report."""
    return _run_v1191_full(
        source=source,
        v1190_artifact_path=v1190_artifact_path,
        max_samples=max_samples,
        timeout=timeout,
        write_artifact=True,
    )


# ============================================================================
# Markdown report
# ============================================================================


def render_report_md(rep: V1191Report) -> str:
    lines: List[str] = []
    lines.append(f"# V1191 V1166 V0.6.4 lift 报告 — {rep.snapshot_id}\n")
    lines.append(f"- **version**: {rep.version}")
    lines.append(f"- **dim_version**: {rep.dim_version}")
    lines.append(f"- **timestamp**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(rep.timestamp))}")
    lines.append(f"- **elapsed**: {rep.elapsed_seconds:.3f}s")
    lines.append(f"- **source**: {rep.source}")
    lines.append(f"- **artifact**: `{rep.artifact_path or 'N/A'}`\n")

    lines.append("## Total")
    lines.append(f"- **V1191 (V1166 V0.6.4 lift)**: {rep.total:.4f}")
    lines.append(f"- **vs V1166 baseline 0.416**: Δ = {rep.total - V1166_BASELINE:+.4f}")
    lines.append(f"- **vs V1182 baseline 0.7425** (V1166 dim weighted): Δ = +{(rep.total - V1166_BASELINE) * 0.05:.4f}")
    lines.append(f"- **vs ASI north star 0.98**: gap = {rep.north_star - rep.total:.4f}\n")

    lines.append("## V1190 backing data\n")
    lines.append(f"- snapshot_id: `{rep.v1190_snapshot_id}`")
    lines.append(f"- endpoint: `{rep.v1190_endpoint}`")
    lines.append(f"- model: `{rep.v1190_model}`")
    lines.append(f"- api_key_present: {rep.v1190_api_key_present} (source: `{rep.v1190_api_key_source}`)")
    lines.append(f"- v1190 total: {rep.v1190_total:.4f}")
    lines.append(f"- pass_rate: {rep.v1190_pass_rate:.4f} ({rep.v1190_n_passed}/{rep.v1190_n_samples})")
    lines.append(f"- p50_latency_ms: {rep.v1190_p50_latency_ms:.1f}")
    lines.append(f"- p95_latency_ms: {rep.v1190_p95_latency_ms:.1f}\n")

    lines.append("## 5 sub-dim 真测\n")
    lines.append("| sub-dim | score | status |")
    lines.append("|---|---:|:---:|")
    for name in V1191_SUBDIM_NAMES:
        s = rep.sub_dim_scores.get(name, 0.0)
        status = "✅ pass" if s >= 0.8 else ("⚠️ partial" if s > 0.0 else "❌ missing")
        lines.append(f"| {name} | {s:.4f} | {status} |")

    lines.append("\n## Sub-dim Evidence\n")
    for name in V1191_SUBDIM_NAMES:
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
    lines.append(f"_Generated by V1191 {rep.version}_")
    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1191 V1166 V0.6.4 lift")
    parser.add_argument("--source", choices=["artifact", "realtime"], default="artifact")
    parser.add_argument("--v1190-artifact", default=DEFAULT_V1190_ARTIFACT)
    parser.add_argument("--max-samples", type=int, default=None)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--no-write", action="store_true")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--report", action="store_true")
    parser.add_argument("--md-out", default=None)
    parser.add_argument("--artifact-dir", default="artifacts")
    parser.add_argument("--measure", action="store_true", help="只 print measure_v1191()")
    args = parser.parse_args(argv)

    if args.measure:
        s = measure_v1191(
            source=args.source,
            v1190_artifact_path=args.v1190_artifact,
            max_samples=args.max_samples,
            timeout=args.timeout,
        )
        print(f"{s:.4f}")
        return 0

    rep = _run_v1191_full(
        source=args.source,
        v1190_artifact_path=args.v1190_artifact,
        max_samples=args.max_samples,
        timeout=args.timeout,
        write_artifact=not args.no_write,
        artifact_dir=args.artifact_dir,
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