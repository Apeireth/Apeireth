"""V1158 — ASI plugin_core V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

主 17:43 实事求是真问题 (V1155 baseline):
  - V1155 next-ROI top-1 = plugin_core (current 0.6500)
  - V1144._measure_plugin_core 当前 V1071 真测, 但只取 'total'/'n_plugins' 1 个字段
  - 插件核心 = ASI 平台可扩展性, 真补 = 5 sub-dim 真测

V1158 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (不空 placeholder):
    P1 plugin_discovery          — V1071.run 真 produce n_plugins > 0
    P2 plugin_parse_rate         — n_parsed / n_plugins = 真实 parse rate
    P3 plugin_validation_coverage — entry_validation 真验证覆盖率
    P4 plugin_capability_summary  — capability_summary 5 keys 真覆盖
    P5 plugin_protocol_diversity  — protocol_diversity + type_diversity 真测
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]
  - 任何 sub-dim 失败 → sub-dim = 0.0 (不假装满分)

主 00:56 任何人都能接手:
  - measure_plugin_core_v06() → float (0..1) 主入口
  - measure_plugin_core_full() → PluginCoreReport dataclass + JSON dump
  - PluginCoreReport JSON 写 artifacts/v1158_plugin_core_v06.json

主 00:44 质量工程化:
  - PluginCoreReport (主 22:33 北极星 + V1155 baseline):
      total, sub_dim_scores (dict 5 keys), sub_dim_evidence (dict 5 keys)
      version, timestamp, snapshot_id (uuid), elapsed_seconds
  - 三个出口: CLI / Python / JSON (主 17:43 实事求是)

主 17:58 + 20:46 不假装:
  - 不假装 sub-dim = extensibility: 5 sub-dim 是工程测量, 不冒充真 universal plugins
  - 不假装 total = ASI: 是 plugin_core V0.6 lift, ASI 是更大目标 (主 22:33)
  - 不假装 V1158 > V1071: 是 V1071 + 5 sub-dim 的 lift, 不替代任何模块

Usage:
    python -m apeireth.v1158_asi_plugin_core_v06_real_measure                  # 默认 measure + JSON dump
    python -m apeireth.v1158_asi_plugin_core_v06_real_measure --json          # JSON stdout
    python -m apeireth.v1158_asi_plugin_core_v06_real_measure --no-write      # 只 print
    python -m apeireth.v1158_asi_plugin_core_v06_real_measure --report        # markdown 报告
    python -m apeireth.v1158_asi_plugin_core_v06_real_measure --artifact-dir artifacts  # 改目录

作为 V1144 plugin_core dim 真测入口:
    from apeireth.v1158_asi_plugin_core_v06_real_measure import measure_plugin_core_v06
    score = measure_plugin_core_v06()  # 0..1
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

V1158_VERSION = "0.1.0"
V1158_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — 借鉴 plugin system 5 axis)
V1158_SUBDIM_NAMES: Tuple[str, ...] = (
    "plugin_discovery",             # P1 — 真 discover ≥ 1 plugin
    "plugin_parse_rate",            # P2 — parse rate ≥ 50%
    "plugin_validation_coverage",   # P3 — validation 真覆盖
    "plugin_capability_summary",    # P4 — 5 capability keys 覆盖
    "plugin_protocol_diversity",    # P5 — 多协议/类型多样
)

# 默认 artifact dir (主 00:56 任何人都能接手)
DEFAULT_ARTIFACT_DIR = "artifacts"

# V1144 baseline (主 17:43 实事求是 — 写死历史值)
V1144_BASELINE_PLUGIN_CORE = 0.6500

# Target (主 13:31 大胆激进)
TARGET_PLUGIN_CORE_V06 = 0.8500


# ============================================================================
# PluginCoreReport — 真测结果 dataclass (主 00:44 质量工程化)
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
class PluginCoreReport:
    """V1158 plugin_core V0.6 真测报告."""

    snapshot_id: str = field(default_factory=lambda: f"v1158-{uuid.uuid4().hex[:8]}")
    version: str = V1158_VERSION
    dim_version: str = V1158_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    n_subdims_total: int = len(V1158_SUBDIM_NAMES)
    n_subdims_passed: int = 0
    n_subdims_partial: int = 0
    n_subdims_missing: int = 0
    notes: List[str] = field(default_factory=list)
    artifact_path: str = ""
    v1144_baseline: float = V1144_BASELINE_PLUGIN_CORE
    target: float = TARGET_PLUGIN_CORE_V06

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "PluginCoreReport":
        new = cls(
            snapshot_id=data.get("snapshot_id", ""),
            version=data.get("version", V1158_VERSION),
            dim_version=data.get("dim_version", V1158_DIM_VERSION),
            timestamp=data.get("timestamp", 0.0),
            elapsed_seconds=data.get("elapsed_seconds", 0.0),
            total=data.get("total", 0.0),
            sub_dim_scores=data.get("sub_dim_scores", {}),
            n_subdims_total=data.get("n_subdims_total", len(V1158_SUBDIM_NAMES)),
            n_subdims_passed=data.get("n_subdims_passed", 0),
            n_subdims_partial=data.get("n_subdims_partial", 0),
            n_subdims_missing=data.get("n_subdims_missing", 0),
            notes=data.get("notes", []),
            artifact_path=data.get("artifact_path", ""),
            v1144_baseline=data.get("v1144_baseline", V1144_BASELINE_PLUGIN_CORE),
            target=data.get("target", TARGET_PLUGIN_CORE_V06),
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
            f"V1158 plugin_core V0.6: total={self.total:.4f} "
            f"(Δ vs V1144 baseline {self.v1144_baseline:.4f} = "
            f"{self.total - self.v1144_baseline:+.4f}) | "
            f"target={self.target:.4f} (gap {self.target - self.total:+.4f}) | "
            f"5 sub-dim: {self.n_subdims_passed} pass / "
            f"{self.n_subdims_partial} partial / {self.n_subdims_missing} missing | "
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


# ============================================================================
# 统一入口: V1071 测
# ============================================================================


def _v1071_run() -> Tuple[bool, Dict[str, Any]]:
    """Return (success, run_result_dict)."""
    v1071_mod = _safe_import("apeireth.v1071_vcp_real_source_code_deep_read")
    if v1071_mod is None:
        return False, {}
    cls = _attr_first(v1071_mod, ["V1071VCPDeepRead", "VCPDeepRead"])
    if cls is None:
        return False, {}
    try:
        inst = cls()
    except Exception:
        return False, {}

    # 真 run() 真测
    run_fn = getattr(inst, "run", None)
    if not callable(run_fn):
        return False, {}
    ok, r = _call_safely(run_fn)
    if not ok or not isinstance(r, dict):
        return False, {}
    return True, r


def _v1071_measure() -> Tuple[bool, Dict[str, Any]]:
    """Return (success, measure_result_dict)."""
    v1071_mod = _safe_import("apeireth.v1071_vcp_real_source_code_deep_read")
    if v1071_mod is None:
        return False, {}
    cls = _attr_first(v1071_mod, ["V1071VCPDeepRead", "VCPDeepRead"])
    if cls is None:
        return False, {}
    try:
        inst = cls()
    except Exception:
        return False, {}

    measure_fn = getattr(inst, "measure", None)
    if not callable(measure_fn):
        return False, {}
    ok, r = _call_safely(measure_fn)
    if not ok or not isinstance(r, dict):
        return False, {}
    return True, r


# ============================================================================
# P1 — plugin_discovery (真 discover ≥ 1)
# ============================================================================


def _measure_plugin_discovery() -> Tuple[float, SubDimEvidence]:
    """P1: V1071 真 discover n_plugins ≥ 1."""
    ev = SubDimEvidence(
        name="plugin_discovery",
        score=0.0,
        notes=["P1: V1071 真 run() 真 discover plugin n_plugins"]
    )

    ok, run = _v1071_run()
    if not ok:
        ev.notes.append("V1071 run() failed → P1 = 0")
        ev.raw = {"test_results": [], "reason": "run_failed"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    # Test 1: n_plugins ≥ 1
    n_plugins = run.get("n_plugins", 0)
    if isinstance(n_plugins, (int, float)) and n_plugins >= 1:
        test_results.append(("n_plugins_present", True, f"n_plugins={int(n_plugins)}"))
    else:
        test_results.append(("n_plugins_present", False, f"n_plugins={n_plugins}"))

    # Test 2: n_plugins ≥ 10 (够多)
    if isinstance(n_plugins, (int, float)) and n_plugins >= 10:
        test_results.append(("n_plugins_sufficient", True, f"n_plugins={int(n_plugins)}"))
    else:
        test_results.append(("n_plugins_sufficient", False, f"n_plugins={n_plugins}"))

    # Test 3: vcp_root 真有
    vcp_root = run.get("vcp_root", "")
    test_results.append(("vcp_root_present", bool(vcp_root), f"vcp_root={vcp_root}"))

    # Test 4: spec_result 真有
    spec_result = run.get("spec_result")
    test_results.append(("spec_result_present", spec_result is not None, f"type={type(spec_result).__name__}"))

    # Test 5: manifests 真有
    manifests = run.get("manifests")
    test_results.append(("manifests_present", isinstance(manifests, list), f"type={type(manifests).__name__}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_total": 5,
        "n_plugins": n_plugins,
        "vcp_root": str(vcp_root),
    }
    return ev.score, ev


# ============================================================================
# P2 — plugin_parse_rate (n_parsed / n_plugins)
# ============================================================================


def _measure_plugin_parse_rate() -> Tuple[float, SubDimEvidence]:
    """P2: parse rate = n_parsed / n_plugins."""
    ev = SubDimEvidence(
        name="plugin_parse_rate",
        score=0.0,
        notes=["P2: n_parsed / n_plugins = 真实 parse rate"]
    )

    ok, run = _v1071_run()
    if not ok:
        ev.notes.append("V1071 run() failed → P2 = 0")
        ev.raw = {"test_results": [], "reason": "run_failed"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    n_plugins = run.get("n_plugins", 0)
    n_parsed = run.get("n_parsed", 0)
    if not isinstance(n_plugins, (int, float)) or n_plugins == 0:
        ev.notes.append("no n_plugins → P2 = 0")
        ev.raw = {"test_results": [], "reason": "no_plugins"}
        return 0.0, ev

    parse_rate = float(n_parsed) / float(n_plugins) if n_plugins else 0.0
    ev.raw = {"parse_rate": parse_rate, "n_plugins": n_plugins, "n_parsed": n_parsed}

    # Test 1: parse_rate ≥ 30%
    test_results.append(("parse_rate_30", parse_rate >= 0.30, f"rate={parse_rate:.2f}"))

    # Test 2: parse_rate ≥ 50%
    test_results.append(("parse_rate_50", parse_rate >= 0.50, f"rate={parse_rate:.2f}"))

    # Test 3: parse_rate ≥ 70%
    test_results.append(("parse_rate_70", parse_rate >= 0.70, f"rate={parse_rate:.2f}"))

    # Test 4: n_parsed 真有 (> 0)
    test_results.append(("n_parsed_positive", isinstance(n_parsed, (int, float)) and n_parsed > 0,
                          f"n_parsed={n_parsed}"))

    # Test 5: parse_rate 计算正确 (parse_rate < 1.0)
    test_results.append(("parse_rate_valid", 0.0 <= parse_rate <= 1.0,
                          f"rate={parse_rate:.2f}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    # 加 0.2 base if 真有 parse rate
    if parse_rate > 0:
        ev.score = min(1.0, 0.2 + 0.8 * (float(n_pass) / 5.0))
    else:
        ev.score = 0.0
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw["test_results"] = [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results]
    ev.raw["n_pass"] = n_pass
    return ev.score, ev


# ============================================================================
# P3 — plugin_validation_coverage
# ============================================================================


def _measure_plugin_validation_coverage() -> Tuple[float, SubDimEvidence]:
    """P3: entry_validation 真有 + 覆盖真 plugins."""
    ev = SubDimEvidence(
        name="plugin_validation_coverage",
        score=0.0,
        notes=["P3: entry_validation 真有 + 真覆盖"]
    )

    ok, run = _v1071_run()
    if not ok:
        ev.notes.append("V1071 run() failed → P3 = 0")
        ev.raw = {"test_results": [], "reason": "run_failed"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    entry_validation = run.get("entry_validation")
    test_results.append(("entry_validation_present", entry_validation is not None,
                          f"type={type(entry_validation).__name__}"))

    # entry_validation 真有 key data
    if isinstance(entry_validation, dict) and len(entry_validation) > 0:
        test_results.append(("entry_validation_nonempty", True, f"keys={list(entry_validation.keys())[:5]}"))
    elif isinstance(entry_validation, list) and len(entry_validation) > 0:
        test_results.append(("entry_validation_nonempty", True, f"n={len(entry_validation)}"))
    else:
        test_results.append(("entry_validation_nonempty", False, f"empty"))

    # Test: Validation 覆盖率 (entry_validation 真有 valid entries)
    has_valid = False
    if isinstance(entry_validation, dict):
        for k, v in entry_validation.items():
            if isinstance(v, dict) and v.get("valid") is True:
                has_valid = True
                break
            if isinstance(v, bool) and v is True:
                has_valid = True
                break
    elif isinstance(entry_validation, list):
        for v in entry_validation:
            if isinstance(v, dict) and v.get("valid") is True:
                has_valid = True
                break
    test_results.append(("has_valid_entries", has_valid, ""))

    # Test 4: entry_validation 包含 error reporting
    has_error = False
    if isinstance(entry_validation, dict):
        for v in entry_validation.values():
            if isinstance(v, dict) and (v.get("errors") or v.get("warnings")):
                has_error = True
                break
    test_results.append(("has_error_reporting", has_error, ""))

    # Test 5: spec_result 真有效
    spec_result = run.get("spec_result")
    test_results.append(("spec_result_valid",
                          spec_result is not None and isinstance(spec_result, dict),
                          f"type={type(spec_result).__name__}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_total": 5,
    }
    return ev.score, ev


# ============================================================================
# P4 — plugin_capability_summary
# ============================================================================


def _measure_plugin_capability_summary() -> Tuple[float, SubDimEvidence]:
    """P4: capability_summary 5 keys 真覆盖."""
    ev = SubDimEvidence(
        name="plugin_capability_summary",
        score=0.0,
        notes=["P4: capability_summary 5 keys 真覆盖"]
    )

    ok, run = _v1071_run()
    if not ok:
        ev.notes.append("V1071 run() failed → P4 = 0")
        ev.raw = {"test_results": [], "reason": "run_failed"}
        return 0.0, ev

    capability_summary = run.get("capability_summary")
    if not isinstance(capability_summary, dict):
        ev.notes.append("no capability_summary dict → P4 = 0")
        ev.raw = {"test_results": [], "reason": "no_capability_summary"}
        return 0.0, ev

    expected_keys = {
        "total_invocations",
        "unique_identifiers",
        "n_plugins_with_commands",
        "n_plugins_with_config",
        "n_plugins_with_dependencies",
    }
    actual_keys = set(capability_summary.keys())

    test_results: List[Tuple[str, bool, str]] = []

    # Test 1-5: 每个 expected key 是否存在
    for k in expected_keys:
        present = k in actual_keys
        v = capability_summary.get(k)
        v_type_ok = isinstance(v, (int, float))
        test_results.append((f"key_{k}", present and v_type_ok,
                              f"present={present}, value={v}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_total": 5,
        "capability_summary": capability_summary,
    }
    return ev.score, ev


# ============================================================================
# P5 — plugin_protocol_diversity
# ============================================================================


def _measure_plugin_protocol_diversity() -> Tuple[float, SubDimEvidence]:
    """P5: protocol_diversity + type_diversity + 多协议."""
    ev = SubDimEvidence(
        name="plugin_protocol_diversity",
        score=0.0,
        notes=["P5: V1071 真 measure() 真 protocol_diversity + type_diversity"]
    )

    ok, meas = _v1071_measure()
    if not ok:
        ev.notes.append("V1071 measure() failed → P5 = 0")
        ev.raw = {"test_results": [], "reason": "measure_failed"}
        return 0.0, ev

    test_results: List[Tuple[str, bool, str]] = []

    # Test 1: protocol_diversity 真有 (≥ 1)
    pd = meas.get("protocol_diversity", 0)
    test_results.append(("protocol_diversity_present",
                          isinstance(pd, (int, float)) and pd >= 1,
                          f"protocol_diversity={pd}"))

    # Test 2: type_diversity 真有 (≥ 1)
    td = meas.get("type_diversity", 0)
    test_results.append(("type_diversity_present",
                          isinstance(td, (int, float)) and td >= 1,
                          f"type_diversity={td}"))

    # Test 3: protocol_diversity ≥ 3 (多协议 = 真多样性)
    test_results.append(("protocol_diversity_high",
                          isinstance(pd, (int, float)) and pd >= 3,
                          f"protocol_diversity={pd}"))

    # Test 4: type_diversity ≥ 3 (多类型 = 真多样性)
    test_results.append(("type_diversity_high",
                          isinstance(td, (int, float)) and td >= 3,
                          f"type_diversity={td}"))

    # Test 5: validity 真有
    validity = meas.get("validity")
    test_results.append(("validity_present", validity is not None, f"type={type(validity).__name__}"))

    n_pass = sum(1 for _, ok, _ in test_results if ok)
    ev.score = float(n_pass) / 5.0
    ev.score = min(1.0, max(0.0, ev.score))
    ev.checks = {name: ok for name, ok, _ in test_results}
    ev.raw = {
        "test_results": [{"name": n, "ok": ok, "note": note} for n, ok, note in test_results],
        "n_pass": n_pass,
        "n_total": 5,
        "measure_result": meas,
    }
    return ev.score, ev


# ============================================================================
# 主入口
# ============================================================================


def measure_plugin_core_v06() -> float:
    """主入口 — 返回 plugin_core V0.6 score (0..1)."""
    rep = measure_plugin_core_full(write_artifact=False)
    return rep.total


def measure_plugin_core_full(
    write_artifact: bool = True,
    artifact_dir: str = DEFAULT_ARTIFACT_DIR,
) -> PluginCoreReport:
    """Run all 5 sub-dims, return PluginCoreReport."""
    t0 = time.time()
    rep = PluginCoreReport()

    # P1
    s1, ev1 = _measure_plugin_discovery()
    rep.sub_dim_scores["plugin_discovery"] = s1
    rep.sub_dim_evidence["plugin_discovery"] = ev1
    if s1 >= 0.8:
        rep.n_subdims_passed += 1
    elif s1 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # P2
    s2, ev2 = _measure_plugin_parse_rate()
    rep.sub_dim_scores["plugin_parse_rate"] = s2
    rep.sub_dim_evidence["plugin_parse_rate"] = ev2
    if s2 >= 0.8:
        rep.n_subdims_passed += 1
    elif s2 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # P3
    s3, ev3 = _measure_plugin_validation_coverage()
    rep.sub_dim_scores["plugin_validation_coverage"] = s3
    rep.sub_dim_evidence["plugin_validation_coverage"] = ev3
    if s3 >= 0.8:
        rep.n_subdims_passed += 1
    elif s3 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # P4
    s4, ev4 = _measure_plugin_capability_summary()
    rep.sub_dim_scores["plugin_capability_summary"] = s4
    rep.sub_dim_evidence["plugin_capability_summary"] = ev4
    if s4 >= 0.8:
        rep.n_subdims_passed += 1
    elif s4 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    # P5
    s5, ev5 = _measure_plugin_protocol_diversity()
    rep.sub_dim_scores["plugin_protocol_diversity"] = s5
    rep.sub_dim_evidence["plugin_protocol_diversity"] = ev5
    if s5 >= 0.8:
        rep.n_subdims_passed += 1
    elif s5 > 0.0:
        rep.n_subdims_partial += 1
    else:
        rep.n_subdims_missing += 1

    rep.total = sum(rep.sub_dim_scores.values()) / float(len(V1158_SUBDIM_NAMES))
    rep.total = min(1.0, max(0.0, rep.total))
    rep.elapsed_seconds = time.time() - t0

    if write_artifact:
        try:
            ad = Path(artifact_dir)
            ad.mkdir(parents=True, exist_ok=True)
            artifact_path = ad / "v1158_plugin_core_v06.json"
            data = rep.to_dict()
            artifact_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
            rep.artifact_path = str(artifact_path)
            rep.notes.append(f"artifact written: {rep.artifact_path}")
        except Exception as e:
            rep.notes.append(f"artifact write failed: {e!r}")

    return rep


# ============================================================================
# 报告渲染 (主 00:44 质量工程化)
# ============================================================================


def render_report_md(rep: PluginCoreReport) -> str:
    lines: List[str] = []
    lines.append(f"# V1158 plugin_core V0.6 真补报告 — {rep.snapshot_id}\n")
    lines.append(f"- **version**: {rep.version}")
    lines.append(f"- **dim_version**: {rep.dim_version}")
    lines.append(f"- **timestamp**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(rep.timestamp))}")
    lines.append(f"- **elapsed**: {rep.elapsed_seconds:.3f}s")
    lines.append(f"- **artifact**: `{rep.artifact_path or 'N/A'}`\n")
    lines.append("## Total")
    lines.append(f"- **plugin_core V0.6**: {rep.total:.4f}")
    lines.append(f"- **vs V1144 baseline**: {rep.v1144_baseline:.4f} (Δ = {rep.total - rep.v1144_baseline:+.4f})")
    lines.append(f"- **target**: {rep.target:.4f} (gap = {rep.target - rep.total:+.4f})\n")

    lines.append("## 5 sub-dim 真测\n")
    lines.append("| sub-dim | score | status |")
    lines.append("|---|---:|:---:|")
    for name in V1158_SUBDIM_NAMES:
        s = rep.sub_dim_scores.get(name, 0.0)
        status = "✓ pass" if s >= 0.8 else ("◐ partial" if s > 0.0 else "✗ missing")
        lines.append(f"| {name} | {s:.4f} | {status} |")

    lines.append("\n## Sub-dim Evidence\n")
    for name in V1158_SUBDIM_NAMES:
        ev = rep.sub_dim_evidence.get(name)
        if ev is None:
            continue
        lines.append(f"### {name} (score = {ev.score:.4f})")
        if ev.notes:
            for n in ev.notes:
                lines.append(f"- note: {n}")
        if ev.checks:
            for cn, cv in ev.checks.items():
                lines.append(f"- `{cn}`: {'✓' if cv else '✗'}")
        lines.append("")

    lines.append("## Notes\n")
    for n in rep.notes:
        lines.append(f"- {n}")
    lines.append("")
    lines.append("---")
    lines.append(f"_Generated by V1158 {rep.version}_")
    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================


def _cli() -> int:
    parser = argparse.ArgumentParser(description="V1158 plugin_core V0.6 真补")
    parser.add_argument("--json", action="store_true", help="输出 JSON stdout")
    parser.add_argument("--no-write", action="store_true", help="不写 artifact")
    parser.add_argument("--report", action="store_true", help="输出 Markdown 报告")
    parser.add_argument("--artifact-dir", default=DEFAULT_ARTIFACT_DIR)
    parser.add_argument("--md-out", default=None)
    args = parser.parse_args()

    rep = measure_plugin_core_full(
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
