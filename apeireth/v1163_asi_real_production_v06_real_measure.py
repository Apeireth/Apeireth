"""V1163 — ASI real_production V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

主 17:43 实事求是真问题 (V1155 baseline):
  - V1155 next-ROI top-2 不含 real_production (V1155 没列); V1144._measure_real_production
    当前 0.96 — 但 0.96 来自 naive "ok_total / (ok_total + failed_total)" 算法, 把
    compose_files_parsed=2 + services_seen=14 + k8s_manifests_ok=3 + dockerfile_valid=2 +
    subprocess_runs_ok=2 + canonical_bundle_valid (1) 加起来 / 分母 = 0.96, 实际不是
    真"production 能力"分解.
  - V1163 真补: 把 V1132 DeploymentValidator 的 9 真 check 拆成 5 sub-dim 真测.

V1163 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (基于 V1132 V1132DeploymentReport 真 9 字段):
    R1 compose_orchestration_real  — compose_files_parsed ≥ 2 + services_seen ≥ 10
    R2 k8s_dockerfile_real         — k8s_manifests_ok ≥ 3 + dockerfile_valid ≥ 2
    R3 subprocess_runtime_real     — subprocess_runs_ok ≥ 1 + subprocess_runs_failed == 0
    R4 health_probe_real           — health_probes_ok ≥ 1 OR (probes_ok == 0 AND failed == 0)
    R5 canonical_bundle_real       — canonical_bundle_valid AND offline_valid
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]
  - 任何 sub-dim 失败 → sub-dim score 衰减 (主 17:43 不刷 KPI)

主 00:56 任何人都能接手:
  - measure_real_production_v06() → float (0..1) 主入口
  - measure_real_production_full() → RealProductionReport dataclass + JSON dump
  - RealProductionReport JSON 写 artifacts/v1163_real_production_v06.json

主 00:44 质量工程化:
  - RealProductionReport (主 22:33 北极星):
      total, sub_dim_scores (dict 5 keys), sub_dim_evidence (dict 5 keys)
      version, timestamp, snapshot_id (uuid), elapsed_seconds

主 17:58 + 20:46 不假装:
  - 不假装 0.96 = 真 production: naive ratio 不等于 deployment readiness
  - 不假装 subprocess_runs == production runtime: 一过式 subprocess ≠ 长期 runtime
  - 不假装 canonical_bundle_valid = 真 k8s apply: bundle 文件齐 ≠ 集群可跑
  - 不假装 ASI has production: 9 真 check 测量 ≠ ASI 自部署自运维

Usage:
    python -m apeireth.v1163_asi_real_production_v06_real_measure                  # 默认 measure + JSON dump
    python -m apeireth.v1163_asi_real_production_v06_real_measure --json          # JSON stdout
    python -m apeireth.v1163_asi_real_production_v06_real_measure --no-write      # 只 print
    python -m apeireth.v1163_asi_real_production_v06_real_measure --report        # markdown 报告

作为 V1144 real_production dim 真测入口:
    from apeireth.v1163_asi_real_production_v06_real_measure import measure_real_production_v06
    score = measure_real_production_v06()  # 0..1
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


V1163_VERSION = "0.1.0"
V1163_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — 借鉴 production readiness 5 axis)
V1163_SUBDIM_NAMES: Tuple[str, ...] = (
    "compose_orchestration_real",   # R1 — compose 编排真
    "k8s_dockerfile_real",          # R2 — k8s manifest + Dockerfile 真
    "subprocess_runtime_real",      # R3 — subprocess 真跑
    "health_probe_real",            # R4 — healthcheck probe 真
    "canonical_bundle_real",        # R5 — deploy/ 整包真
)

# 默认 artifact dir (主 00:56 任何人都能接手)
DEFAULT_ARTIFACT_DIR = "artifacts"

# V1144 baseline (主 17:43 实事求是 — 写死历史 naive ratio)
V1144_BASELINE_REAL_PRODUCTION = 0.9600

# Target (主 13:31 大胆激进)
TARGET_REAL_PRODUCTION_V06 = 0.8500

# V1132 真 9 check 字段 (主 17:43 实事求是)
V1132_REPORT_FIELDS: Tuple[str, ...] = (
    "docker_daemon_available",
    "compose_files_parsed",
    "services_seen",
    "k8s_manifests_ok",
    "dockerfile_valid",
    "subprocess_runs_ok",
    "subprocess_runs_failed",
    "health_probes_ok",
    "health_probes_failed",
    "canonical_bundle_valid",
    "offline_valid",
    "runtime_valid",
)

# 阈值 (主 17:43 实事求是 — 写在常量里, 不在 measurement 里魔改)
_COMPOSE_FILES_MIN = 2
_SERVICES_MIN = 10
_K8S_MIN = 3
_DOCKERFILE_MIN = 2
_SUBPROCESS_OK_MIN = 1
_HEALTH_OK_MIN = 1


# ============================================================================
# SubDimEvidence + RealProductionReport — 真测结果 dataclass (主 00:44 质量工程化)
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
class RealProductionReport:
    """V1163 real_production V0.6 真测报告."""

    snapshot_id: str = field(default_factory=lambda: f"v1163-{uuid.uuid4().hex[:8]}")
    version: str = V1163_VERSION
    dim_version: str = V1163_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1163_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1144_baseline: float = V1144_BASELINE_REAL_PRODUCTION
    target: float = TARGET_REAL_PRODUCTION_V06
    v1132_report_id: str = ""
    v1132_docker_daemon: bool = False

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "RealProductionReport":
        new = cls(
            snapshot_id=data.get("snapshot_id", ""),
            version=data.get("version", V1163_VERSION),
            dim_version=data.get("dim_version", V1163_DIM_VERSION),
            timestamp=data.get("timestamp", 0.0),
            elapsed_seconds=data.get("elapsed_seconds", 0.0),
            total=data.get("total", 0.0),
            sub_dim_scores=data.get("sub_dim_scores", {}),
            n_subdims_total=data.get("n_subdims_total", len(V1163_SUBDIM_NAMES)),
            n_subdims_passed=data.get("n_subdims_passed", 0),
            n_subdims_partial=data.get("n_subdims_partial", 0),
            n_subdims_missing=data.get("n_subdims_missing", 0),
            notes=data.get("notes", []),
            artifact_path=data.get("artifact_path", ""),
            v1144_baseline=data.get("v1144_baseline", V1144_BASELINE_REAL_PRODUCTION),
            target=data.get("target", TARGET_REAL_PRODUCTION_V06),
            v1132_report_id=data.get("v1132_report_id", ""),
            v1132_docker_daemon=data.get("v1132_docker_daemon", False),
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
            f"V1163 real_production V0.6: total={self.total:.4f} "
            f"(Δ vs V1144 naive {self.v1144_baseline:.4f} = "
            f"{self.total - self.v1144_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"5 sub-dim: {self.n_subdims_passed} pass / "
            f"{self.n_subdims_partial} partial / {self.n_subdims_missing} missing | "
            f"v1132_daemon={self.v1132_docker_daemon} | "
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


def _get_v1132_report() -> Tuple[bool, Any, str]:
    """Try to get a real V1132 report. Returns (ok, report_or_None, reason)."""
    v1132_mod = _safe_import("apeireth.v1132_real_deployment_validator")
    if v1132_mod is None:
        return False, None, "v1132_module_not_found"
    cls = _attr_first(v1132_mod, ["V1132DeploymentValidator", "RealDeploymentValidator", "DeploymentValidator"])
    if cls is None:
        return False, None, "v1132_class_not_found"
    try:
        inst = cls()
    except Exception as e:
        return False, None, f"v1132_instantiation_failed: {e!r}"
    fn = _attr_first(inst, ["run_full_validation", "validate", "run", "validate_real_deployment"])
    if fn is None:
        return False, None, "v1132_run_method_not_found"
    try:
        rep = fn()
        if rep is None:
            return False, None, "v1132_run_returned_none"
        return True, rep, "ok"
    except Exception as e:
        return False, None, f"v1132_run_failed: {e!r}"


def _safe_field(report: Any, name: str, default: Any = 0) -> Any:
    """Safely read a field from V1132DeploymentReport (主 17:43 实事求是)."""
    try:
        v = getattr(report, name, None)
        if v is None:
            return default
        return v
    except Exception:
        return default


# ============================================================================
# R1 — compose_orchestration_real
# ============================================================================


def _measure_compose_orchestration(report: Any) -> Tuple[float, SubDimEvidence]:
    """R1: compose_files_parsed + services_seen 真测."""
    ev = SubDimEvidence(
        name="compose_orchestration_real",
        score=0.0,
        notes=["R1: V1132 compose_files_parsed + services_seen 真测"]
    )

    compose_files = _safe_field(report, "compose_files_parsed", 0)
    services_seen = _safe_field(report, "services_seen", 0)

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("compose_files_at_least_2", compose_files >= _COMPOSE_FILES_MIN,
                        f"compose_files={compose_files}>={_COMPOSE_FILES_MIN}"))
    test_results.append(("compose_files_at_least_4", compose_files >= _COMPOSE_FILES_MIN * 2,
                        f"compose_files={compose_files}>={_COMPOSE_FILES_MIN * 2}"))
    test_results.append(("services_seen_at_least_10", services_seen >= _SERVICES_MIN,
                        f"services_seen={services_seen}>={_SERVICES_MIN}"))
    test_results.append(("services_seen_at_least_20", services_seen >= _SERVICES_MIN * 2,
                        f"services_seen={services_seen}>={_SERVICES_MIN * 2}"))
    test_results.append(("compose_to_services_reasonable",
                        compose_files > 0 and services_seen >= compose_files * 2,
                        f"{compose_files} compose → {services_seen} services"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    # score = mean of two normalized component scores
    compose_norm = min(1.0, compose_files / (_COMPOSE_FILES_MIN * 4.0))  # saturate at 8
    services_norm = min(1.0, services_seen / (_SERVICES_MIN * 4.0))       # saturate at 40
    base = 0.5 * compose_norm + 0.5 * services_norm
    check_bonus = float(n_pass) / 5.0 * 0.2  # up to 0.2 bonus
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "compose_files_parsed": compose_files,
        "services_seen": services_seen,
        "compose_norm": compose_norm,
        "services_norm": services_norm,
    }
    ev.notes.append(
        f"R1 score={ev.score:.4f} (n_pass={n_pass}/5, compose={compose_files}, services={services_seen})"
    )
    return ev.score, ev


# ============================================================================
# R2 — k8s_dockerfile_real
# ============================================================================


def _measure_k8s_dockerfile(report: Any) -> Tuple[float, SubDimEvidence]:
    """R2: k8s_manifests_ok + dockerfile_valid 真测."""
    ev = SubDimEvidence(
        name="k8s_dockerfile_real",
        score=0.0,
        notes=["R2: V1132 k8s_manifests_ok + dockerfile_valid 真测"]
    )

    k8s_ok = _safe_field(report, "k8s_manifests_ok", 0)
    dockerfile_ok = _safe_field(report, "dockerfile_valid", 0)

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("k8s_at_least_3", k8s_ok >= _K8S_MIN, f"k8s={k8s_ok}>={_K8S_MIN}"))
    test_results.append(("k8s_at_least_6", k8s_ok >= _K8S_MIN * 2, f"k8s={k8s_ok}>={_K8S_MIN * 2}"))
    test_results.append(("dockerfile_at_least_2", dockerfile_ok >= _DOCKERFILE_MIN,
                        f"dockerfile={dockerfile_ok}>={_DOCKERFILE_MIN}"))
    test_results.append(("dockerfile_at_least_4", dockerfile_ok >= _DOCKERFILE_MIN * 2,
                        f"dockerfile={dockerfile_ok}>={_DOCKERFILE_MIN * 2}"))
    test_results.append(("both_present", k8s_ok >= 1 and dockerfile_ok >= 1,
                        f"k8s={k8s_ok}, dockerfile={dockerfile_ok}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    k8s_norm = min(1.0, k8s_ok / (_K8S_MIN * 4.0))         # saturate at 12
    df_norm = min(1.0, dockerfile_ok / (_DOCKERFILE_MIN * 4.0))  # saturate at 8
    base = 0.5 * k8s_norm + 0.5 * df_norm
    check_bonus = float(n_pass) / 5.0 * 0.2
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "k8s_manifests_ok": k8s_ok,
        "dockerfile_valid": dockerfile_ok,
        "k8s_norm": k8s_norm,
        "df_norm": df_norm,
    }
    ev.notes.append(f"R2 score={ev.score:.4f} (n_pass={n_pass}/5, k8s={k8s_ok}, df={dockerfile_ok})")
    return ev.score, ev


# ============================================================================
# R3 — subprocess_runtime_real
# ============================================================================


def _measure_subprocess_runtime(report: Any) -> Tuple[float, SubDimEvidence]:
    """R3: subprocess_runs_ok vs subprocess_runs_failed 真测."""
    ev = SubDimEvidence(
        name="subprocess_runtime_real",
        score=0.0,
        notes=["R3: V1132 subprocess_runs_ok + subprocess_runs_failed 真测"]
    )

    sub_ok = _safe_field(report, "subprocess_runs_ok", 0)
    sub_failed = _safe_field(report, "subprocess_runs_failed", 0)

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("subprocess_ok_at_least_1", sub_ok >= _SUBPROCESS_OK_MIN,
                        f"sub_ok={sub_ok}>={_SUBPROCESS_OK_MIN}"))
    test_results.append(("subprocess_ok_at_least_2", sub_ok >= _SUBPROCESS_OK_MIN * 2,
                        f"sub_ok={sub_ok}>={_SUBPROCESS_OK_MIN * 2}"))
    test_results.append(("no_failed_runs", sub_failed == 0, f"sub_failed={sub_failed}==0"))
    test_results.append(("no_failed_runs_strict", sub_failed <= 0,
                        f"sub_failed={sub_failed}<=0"))
    test_results.append(("ok_far_exceeds_failed", sub_ok >= max(1, sub_failed * 2),
                        f"ok={sub_ok}, failed={sub_failed}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    # score: 失败惩罚 + 通过奖励
    if sub_ok == 0 and sub_failed == 0:
        base = 0.0
        ev.notes.append("R3: no subprocess runs at all → 0")
    else:
        # normalize: ok/(ok + failed + 1) — 加 1 防止 0/0
        denom = sub_ok + sub_failed
        ratio = sub_ok / denom if denom > 0 else 0.0
        # scale by raw count (饱和 at ok=10)
        scale = min(1.0, sub_ok / 10.0)
        base = ratio * (0.5 + 0.5 * scale)
    check_bonus = float(n_pass) / 5.0 * 0.2
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "subprocess_runs_ok": sub_ok,
        "subprocess_runs_failed": sub_failed,
        "base_score": base,
    }
    ev.notes.append(f"R3 score={ev.score:.4f} (n_pass={n_pass}/5, ok={sub_ok}, failed={sub_failed})")
    return ev.score, ev


# ============================================================================
# R4 — health_probe_real
# ============================================================================


def _measure_health_probe(report: Any) -> Tuple[float, SubDimEvidence]:
    """R4: health_probes_ok vs health_probes_failed 真测.

    主 17:43 实事求是: 没跑过 probe ≠ 0 分, 但全 failed = 0 分.
    """
    ev = SubDimEvidence(
        name="health_probe_real",
        score=0.0,
        notes=["R4: V1132 health_probes_ok + health_probes_failed 真测"]
    )

    h_ok = _safe_field(report, "health_probes_ok", 0)
    h_failed = _safe_field(report, "health_probes_failed", 0)

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("probe_ok_at_least_1", h_ok >= _HEALTH_OK_MIN,
                        f"h_ok={h_ok}>={_HEALTH_OK_MIN}"))
    test_results.append(("probe_no_failed", h_failed == 0, f"h_failed={h_failed}==0"))
    test_results.append(("probe_pass_rate_high",
                        h_ok + h_failed == 0 or (h_ok / max(1, h_ok + h_failed)) >= 0.5,
                        f"ok={h_ok}, failed={h_failed}"))
    test_results.append(("probe_pass_rate_perfect",
                        h_failed == 0 and h_ok >= 1,
                        f"perfect=ok>0&&failed=0"))
    test_results.append(("probe_attempted", h_ok + h_failed >= 1,
                        f"h_ok={h_ok}+h_failed={h_failed}>=1"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    if h_ok == 0 and h_failed == 0:
        # 主 17:43: 没跑过 probe = 没法证明 health, 给 0.3 partial (没跑 ≠ 不行)
        base = 0.3
        ev.notes.append("R4: no probe attempts → 0.3 partial (untested ≠ unsafe)")
    elif h_failed == 0:
        # 全 ok
        base = 0.8 + min(0.2, h_ok * 0.05)
    else:
        ratio = h_ok / max(1, h_ok + h_failed)
        base = ratio * 0.7

    check_bonus = float(n_pass) / 5.0 * 0.1  # 较低权重, 避免 check 主导
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "health_probes_ok": h_ok,
        "health_probes_failed": h_failed,
        "base_score": base,
    }
    ev.notes.append(f"R4 score={ev.score:.4f} (n_pass={n_pass}/5, ok={h_ok}, failed={h_failed})")
    return ev.score, ev


# ============================================================================
# R5 — canonical_bundle_real
# ============================================================================


def _measure_canonical_bundle(report: Any) -> Tuple[float, SubDimEvidence]:
    """R5: canonical_bundle_valid + offline_valid + runtime_valid 真测."""
    ev = SubDimEvidence(
        name="canonical_bundle_real",
        score=0.0,
        notes=["R5: V1132 canonical_bundle_valid + offline_valid + runtime_valid 真测"]
    )

    canonical = bool(_safe_field(report, "canonical_bundle_valid", False))
    offline = bool(_safe_field(report, "offline_valid", False))
    runtime = bool(_safe_field(report, "runtime_valid", False))

    test_results: List[Tuple[str, bool, str]] = []
    test_results.append(("canonical_bundle_valid", canonical, f"canonical={canonical}"))
    test_results.append(("offline_valid", offline, f"offline={offline}"))
    test_results.append(("runtime_valid", runtime, f"runtime={runtime}"))
    test_results.append(("canonical_and_offline", canonical and offline,
                        f"canonical={canonical}&&offline={offline}"))
    test_results.append(("all_three", canonical and offline and runtime,
                        f"all={canonical and offline and runtime}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)

    # 主 17:43 不刷 KPI: 至少需要 canonical + offline, runtime 是 bonus
    if canonical and offline:
        base = 0.75 + (0.25 if runtime else 0.0)
    elif canonical or offline:
        base = 0.4
    else:
        base = 0.0

    check_bonus = float(n_pass) / 5.0 * 0.15
    ev.score = min(1.0, max(0.0, base + check_bonus))
    ev.checks = {n: ok for n, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "canonical_bundle_valid": canonical,
        "offline_valid": offline,
        "runtime_valid": runtime,
        "base_score": base,
    }
    ev.notes.append(f"R5 score={ev.score:.4f} (n_pass={n_pass}/5, canonical={canonical}, offline={offline}, runtime={runtime})")
    return ev.score, ev


# ============================================================================
# 主入口
# ============================================================================


def measure_real_production_v06() -> float:
    """主入口 — 返回 real_production V0.6 score (0..1)."""
    rep = measure_real_production_full(write_artifact=False)
    return rep.total


def measure_real_production_full(
    write_artifact: bool = True,
    artifact_dir: str = DEFAULT_ARTIFACT_DIR,
) -> RealProductionReport:
    """Run all 5 sub-dims, return RealProductionReport."""
    t0 = time.time()
    rep = RealProductionReport()

    ok, v1132_rep, reason = _get_v1132_report()
    if not ok or v1132_rep is None:
        rep.notes.append(f"V1132 unavailable: {reason} → all sub-dim = 0")
        rep.elapsed_seconds = time.time() - t0
        if write_artifact:
            try:
                ad = Path(artifact_dir)
                ad.mkdir(parents=True, exist_ok=True)
                artifact_path = ad / "v1163_real_production_v06.json"
                artifact_path.write_text(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8")
                rep.artifact_path = str(artifact_path)
            except Exception as e:
                rep.notes.append(f"artifact write failed: {e!r}")
        return rep

    rep.v1132_report_id = str(_safe_field(v1132_rep, "report_id", "unknown"))
    rep.v1132_docker_daemon = bool(_safe_field(v1132_rep, "docker_daemon_available", False))

    # R1
    s1, ev1 = _measure_compose_orchestration(v1132_rep)
    rep.sub_dim_scores["compose_orchestration_real"] = s1
    rep.sub_dim_evidence["compose_orchestration_real"] = ev1
    if s1 >= 0.8:
        rep.n_subdims_passed += 1
    elif s1 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # R2
    s2, ev2 = _measure_k8s_dockerfile(v1132_rep)
    rep.sub_dim_scores["k8s_dockerfile_real"] = s2
    rep.sub_dim_evidence["k8s_dockerfile_real"] = ev2
    if s2 >= 0.8:
        rep.n_subdims_passed += 1
    elif s2 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # R3
    s3, ev3 = _measure_subprocess_runtime(v1132_rep)
    rep.sub_dim_scores["subprocess_runtime_real"] = s3
    rep.sub_dim_evidence["subprocess_runtime_real"] = ev3
    if s3 >= 0.8:
        rep.n_subdims_passed += 1
    elif s3 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # R4
    s4, ev4 = _measure_health_probe(v1132_rep)
    rep.sub_dim_scores["health_probe_real"] = s4
    rep.sub_dim_evidence["health_probe_real"] = ev4
    if s4 >= 0.8:
        rep.n_subdims_passed += 1
    elif s4 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # R5
    s5, ev5 = _measure_canonical_bundle(v1132_rep)
    rep.sub_dim_scores["canonical_bundle_real"] = s5
    rep.sub_dim_evidence["canonical_bundle_real"] = ev5
    if s5 >= 0.8:
        rep.n_subdims_passed += 1
    elif s5 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1163_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0

    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1163_real_production_v06.json"
            artifact_path.write_text(json.dumps(rep.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8")
            rep.artifact_path = str(artifact_path)
            rep.notes.append(f"artifact written: {rep.artifact_path}")
        except Exception as e:
            rep.notes.append(f"artifact write failed: {e!r}")

    return rep


# ============================================================================
# 报告渲染 (主 00:44 质量工程化)
# ============================================================================


def render_report_md(rep: RealProductionReport) -> str:
    lines: List[str] = []
    lines.append(f"# V1163 real_production V0.6 真补报告 — {rep.snapshot_id}\n")
    lines.append(f"- **version**: {rep.version}")
    lines.append(f"- **dim_version**: {rep.dim_version}")
    lines.append(f"- **timestamp**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(rep.timestamp))}")
    lines.append(f"- **elapsed**: {rep.elapsed_seconds:.3f}s")
    lines.append(f"- **artifact**: `{rep.artifact_path or 'N/A'}`")
    lines.append(f"- **v1132 report_id**: `{rep.v1132_report_id}`")
    lines.append(f"- **v1132 docker_daemon**: {rep.v1132_docker_daemon}\n")

    lines.append("## Total")
    lines.append(f"- **real_production V0.6**: {rep.total:.4f}")
    lines.append(f"- **vs V1144 naive baseline**: {rep.v1144_baseline:.4f} (Δ = {rep.total - rep.v1144_baseline:+.4f})")
    lines.append(f"- **target**: {rep.target:.4f} (gap = {rep.target - rep.total:+.4f})\n")

    lines.append("## 5 sub-dim 真测\n")
    lines.append("| sub-dim | score | status |")
    lines.append("|---|---:|:---:|")
    for name in V1163_SUBDIM_NAMES:
        s = rep.sub_dim_scores.get(name, 0.0)
        status = "✅ pass" if s >= 0.8 else ("⚠️ partial" if s > 0.0 else "❌ missing")
        lines.append(f"| {name} | {s:.4f} | {status} |")

    lines.append("\n## Sub-dim Evidence\n")
    for name in V1163_SUBDIM_NAMES:
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
    lines.append(f"_Generated by V1163 {rep.version}_")
    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================


def _cli() -> int:
    parser = argparse.ArgumentParser(description="V1163 real_production V0.6 真补")
    parser.add_argument("--json", action="store_true", help="输出 JSON stdout")
    parser.add_argument("--no-write", action="store_true", help="不写 artifact")
    parser.add_argument("--report", action="store_true", help="输出 Markdown 报告")
    parser.add_argument("--artifact-dir", default=DEFAULT_ARTIFACT_DIR)
    parser.add_argument("--md-out", default=None)
    args = parser.parse_args()

    rep = measure_real_production_full(
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
    sys.exit(_cli())