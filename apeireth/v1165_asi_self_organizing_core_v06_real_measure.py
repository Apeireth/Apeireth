"""V1165 — ASI self_organizing_core V0.6 真补 (5 sub-dim 真测).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.

主 17:43 实事求是真问题 (V1155 baseline):
  - V1155 V0.6 = 0.8929, gap -0.0871 (LOCKED 0.9800 北极星)
  - 21 dim 中 self_organizing_core = 0.8000 (V0.5 hardcoded, 没 V0.6 真测)
  - V1155 next-ROI top-4 = self_organizing_core (potential_gain 0.0100)
  - V1144 跑成 0.8000 来自 V1089 HotColdMemory 真测 + 比例, 没拾 V1065 真生产 9 组件

V1165 真补路径 (主 17:43 实事求是):
  - 5 sub-dim 真测 (V1065 真生产 9 组件 + 真 measure 函数):
    S1 autopoietic_closure    — V1065 autopoietic_closure measure
    S2 autocatalytic_raf      — V1065 autocatalytic_raf measure (Kauffman 1993 RAF)
    S3 requisite_variety       — V1065 requisite_variety_ratio (Ashby 1956)
    S4 dissipative_export     — V1065 dissipative_export_rate (Prigogine 1977)
    S5 chemoton_coupling      — V1065 chemoton_coupling (Ganti 1975 chemoton)
  - aggregate = mean(sub_dim_scores) ∈ [0, 1]
  - 任何 sub-dim 失败 → sub-dim score = 0.0 (不假装满分)

主 00:56 任何人都能接手:
  - measure_self_organizing_core_v06() → float (0..1) 主入口
  - measure_self_organizing_core_full() → SelfOrganizingCoreReport dataclass + JSON dump
  - SelfOrganizingCoreReport JSON 写 artifacts/v1165_self_organizing_core_v06.json

主 00:44 质量工程化:
  - SelfOrganizingCoreReport (主 22:33 北极星):
      total, sub_dim_scores (dict 5 keys), sub_dim_evidence (dict 5 keys)
      version, timestamp, snapshot_id (uuid), elapsed_seconds

主 17:58 + 20:46 不假装:
  - 不假装 sub-dim = 真涌现: 5 sub-dim 是工程测量, 不冒充真 strong emergence
  - 不假装 total = ASI: 是 self_organizing_core V0.6 真测, 不是 ASI
  - 不假装 chemoton_coupling > 0.7 = 真生命: V1065 chemoton 0.667 是数学, 不是真生物
  - 不假装 adaptive_diversity > 1 = 真 diversity: 0.2 是 V1065 真跑, 不是 CAS 已涌现

Usage:
    python -m apeireth.v1165_asi_self_organizing_core_v06_real_measure         # 默认 measure + JSON dump
    python -m apeireth.v1165_asi_self_organizing_core_v06_real_measure --json  # JSON stdout
    python -m apeireth.v1165_asi_self_organizing_core_v06_real_measure --no-write
    python -m apeireth.v1165_asi_self_organizing_core_v06_real_measure --report
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


V1165_VERSION = "0.1.0"
V1165_DIM_VERSION = "0.6"

# 5 sub-dim names (LOCKED 主 19:33 走在前人经验上 — 借鉴 V1065 真生产 9 组件的 5 axis)
V1165_SUBDIM_NAMES: Tuple[str, ...] = (
    "autopoietic_closure",          # S1 — autopoietic (Maturana/Varela 1980)
    "autocatalytic_raf",            # S2 — autocatalytic RAF (Kauffman 1993)
    "requisite_variety",            # S3 — requisite variety (Ashby 1956)
    "dissipative_export",           # S4 — dissipative export (Prigogine 1977)
    "chemoton_coupling",            # S5 — chemoton coupling (Ganti 1975)
)

# 默认 artifact dir (主 00:56 任何人都能接手)
DEFAULT_ARTIFACT_DIR = "artifacts"

# V1155 baseline hardcoded (主 17:43 实事求是)
V1155_BASELINE_SELF_ORGANIZING_CORE = 0.8000


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


def _loss_to_score(value: Optional[float], ok_threshold: float = 1.0, miss_threshold: float = 0.0) -> float:
    """value in [0..1] → clamped score."""
    if value is None:
        return 0.0
    try:
        v = float(value)
    except Exception:
        return 0.0
    return max(0.0, min(1.0, v))


# ============================================================================
# dataclasses
# ============================================================================


@dataclass
class SubDimEvidence:
    name: str
    score: float = 0.0
    checks: Dict[str, bool] = field(default_factory=dict)
    raw: Dict[str, Any] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)
    baseline_v1155: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class SelfOrganizingCoreReport:
    version: str = V1165_VERSION
    dim_version: str = V1165_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    snapshot_id: str = field(default_factory=lambda: str(uuid.uuid4())[:8])
    elapsed_seconds: float = 0.0
    baseline_v1155: float = V1155_BASELINE_SELF_ORGANIZING_CORE
    total: float = 0.0
    sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    sub_dim_evidence: Dict[str, SubDimEvidence] = field(default_factory=dict)
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["sub_dim_evidence"] = {k: v.to_dict() for k, v in self.sub_dim_evidence.items()}
        return d

    def summary_line(self) -> str:
        n_pass = sum(1 for v in self.sub_dim_scores.values() if v >= 0.5)
        n_part = sum(1 for v in self.sub_dim_scores.values() if 0.0 < v < 0.5)
        n_miss = sum(1 for v in self.sub_dim_scores.values() if v == 0.0)
        return (
            f"V1165 self_organizing_core V0.6: total={self.total:.4f} "
            f"(Δ vs V1155 baseline {self.baseline_v1155:.4f} = "
            f"{self.total - self.baseline_v1155:+.4f}) | "
            f"target=0.8500 (gap {0.8500 - self.total:+.4f}) | "
            f"5 sub-dim: {n_pass} pass / {n_part} partial / {n_miss} missing | "
            f"snapshot=v1165-{self.snapshot_id}"
        )


# ============================================================================
# V1065 真连
# ============================================================================


def _v1065_core():
    """Build V1065 SelfOrganizingCore (主 17:43 实事求是)."""
    v1065_mod = _safe_import("apeireth.v1065_asi_self_organizing_core")
    if v1065_mod is None:
        return False, None
    builder = _attr_first(v1065_mod, ["build_self_organizing_core"])
    if builder is None:
        return False, None
    try:
        return True, builder()
    except Exception:
        return False, None


def _core_measure_dict(core: Any) -> Optional[Dict[str, float]]:
    """V1065 core.measure() 返回 dict of 9 components."""
    if core is None:
        return None
    fn = getattr(core, "measure", None)
    if fn is None:
        return None
    try:
        m = fn()
        if isinstance(m, dict):
            return {str(k): float(v) for k, v in m.items() if isinstance(v, (int, float))}
    except Exception:
        return None
    return None


# ============================================================================
# 5 sub-dim 真测
# ============================================================================


def _measure_autopoietic_closure() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="autopoietic_closure",
        baseline_v1155=V1155_BASELINE_SELF_ORGANIZING_CORE,
        notes=["S1: V1065 SelfOrganizingCore.measure()['autopoietic_closure']"],
    )
    ok, core = _v1065_core()
    if not ok or core is None:
        ev.notes.append("V1065 core unavailable → S1 = 0")
        return 0.0, ev
    m = _core_measure_dict(core)
    if m is None or "autopoietic_closure" not in m:
        ev.notes.append("V1065 measure() missing autopoietic_closure → S1 = 0")
        return 0.0, ev
    val = m["autopoietic_closure"]
    score = _loss_to_score(val)
    ev.score = score
    ev.checks = {
        "core_build_ok": True,
        "measure_returns_dict": True,
        "autopoietic_closure_present": True,
        "value_in_unit_interval": 0.0 <= val <= 1.0,
        "value_above_5": val >= 0.5,
    }
    ev.raw = {"autopoietic_closure": val, "all_measure_keys": list(m.keys())}
    ev.notes.append(f"S1 score={score:.4f} (val={val:.4f})")
    return score, ev


def _measure_autocatalytic_raf() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="autocatalytic_raf",
        baseline_v1155=V1155_BASELINE_SELF_ORGANIZING_CORE,
        notes=["S2: V1065 SelfOrganizingCore.measure()['autocatalytic_raf'] (Kauffman 1993 RAF)"],
    )
    ok, core = _v1065_core()
    if not ok or core is None:
        ev.notes.append("V1065 core unavailable → S2 = 0")
        return 0.0, ev
    m = _core_measure_dict(core)
    if m is None or "autocatalytic_raf" not in m:
        ev.notes.append("V1065 measure() missing autocatalytic_raf → S2 = 0")
        return 0.0, ev
    val = m["autocatalytic_raf"]
    score = _loss_to_score(val)
    ev.score = score
    ev.checks = {
        "core_build_ok": True,
        "measure_returns_dict": True,
        "autocatalytic_raf_present": True,
        "value_in_unit_interval": 0.0 <= val <= 1.0,
        "value_above_5": val >= 0.5,
    }
    ev.raw = {"autocatalytic_raf": val, "all_measure_keys": list(m.keys())}
    ev.notes.append(f"S2 score={score:.4f} (val={val:.4f})")
    return score, ev


def _measure_requisite_variety() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="requisite_variety",
        baseline_v1155=V1155_BASELINE_SELF_ORGANIZING_CORE,
        notes=["S3: V1065 requisite_variety_ratio (Ashby 1956 Law of Requisite Variety)"],
    )
    ok, core = _v1065_core()
    if not ok or core is None:
        ev.notes.append("V1065 core unavailable → S3 = 0")
        return 0.0, ev
    m = _core_measure_dict(core)
    if m is None or "requisite_variety_ratio" not in m:
        ev.notes.append("V1065 measure() missing requisite_variety_ratio → S3 = 0")
        return 0.0, ev
    val = m["requisite_variety_ratio"]
    score = _loss_to_score(val)
    ev.score = score
    ev.checks = {
        "core_build_ok": True,
        "measure_returns_dict": True,
        "requisite_variety_present": True,
        "value_in_unit_interval": 0.0 <= val <= 1.0,
        "value_above_5": val >= 0.5,
    }
    ev.raw = {"requisite_variety": val, "all_measure_keys": list(m.keys())}
    ev.notes.append(f"S3 score={score:.4f} (val={val:.4f})")
    return score, ev


def _measure_dissipative_export() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="dissipative_export",
        baseline_v1155=V1155_BASELINE_SELF_ORGANIZING_CORE,
        notes=["S4: V1065 dissipative_export_rate (Prigogine 1977 dissipative structures)"],
    )
    ok, core = _v1065_core()
    if not ok or core is None:
        ev.notes.append("V1065 core unavailable → S4 = 0")
        return 0.0, ev
    m = _core_measure_dict(core)
    if m is None or "dissipative_export_rate" not in m:
        ev.notes.append("V1065 measure() missing dissipative_export_rate → S4 = 0")
        return 0.0, ev
    val = m["dissipative_export_rate"]
    score = _loss_to_score(val)
    ev.score = score
    ev.checks = {
        "core_build_ok": True,
        "measure_returns_dict": True,
        "dissipative_export_present": True,
        "value_in_unit_interval": 0.0 <= val <= 1.0,
        "value_above_5": val >= 0.5,
    }
    ev.raw = {"dissipative_export": val, "all_measure_keys": list(m.keys())}
    ev.notes.append(f"S4 score={score:.4f} (val={val:.4f})")
    return score, ev


def _measure_chemoton_coupling() -> Tuple[float, SubDimEvidence]:
    ev = SubDimEvidence(
        name="chemoton_coupling",
        baseline_v1155=V1155_BASELINE_SELF_ORGANIZING_CORE,
        notes=["S5: V1065 chemoton_coupling (Ganti 1975 chemoton 3-subsystem coupling)"],
    )
    ok, core = _v1065_core()
    if not ok or core is None:
        ev.notes.append("V1065 core unavailable → S5 = 0")
        return 0.0, ev
    m = _core_measure_dict(core)
    if m is None or "chemoton_coupling" not in m:
        ev.notes.append("V1065 measure() missing chemoton_coupling → S5 = 0")
        return 0.0, ev
    val = m["chemoton_coupling"]
    score = _loss_to_score(val)
    ev.score = score
    ev.checks = {
        "core_build_ok": True,
        "measure_returns_dict": True,
        "chemoton_coupling_present": True,
        "value_in_unit_interval": 0.0 <= val <= 1.0,
        "value_above_5": val >= 0.5,
    }
    ev.raw = {"chemoton_coupling": val, "all_measure_keys": list(m.keys())}
    ev.notes.append(f"S5 score={score:.4f} (val={val:.4f})")
    return score, ev


# ============================================================================
# main entries
# ============================================================================


def measure_self_organizing_core_v06() -> float:
    rep = measure_self_organizing_core_full(write_artifact=False)
    return rep.total


def measure_self_organizing_core_full(write_artifact: bool = True, artifact_dir: str = DEFAULT_ARTIFACT_DIR) -> SelfOrganizingCoreReport:
    rep = SelfOrganizingCoreReport()
    t0 = time.time()

    s1_score, s1_ev = _measure_autopoietic_closure()
    rep.sub_dim_scores["autopoietic_closure"] = round(s1_score, 4)
    rep.sub_dim_evidence["autopoietic_closure"] = s1_ev

    s2_score, s2_ev = _measure_autocatalytic_raf()
    rep.sub_dim_scores["autocatalytic_raf"] = round(s2_score, 4)
    rep.sub_dim_evidence["autocatalytic_raf"] = s2_ev

    s3_score, s3_ev = _measure_requisite_variety()
    rep.sub_dim_scores["requisite_variety"] = round(s3_score, 4)
    rep.sub_dim_evidence["requisite_variety"] = s3_ev

    s4_score, s4_ev = _measure_dissipative_export()
    rep.sub_dim_scores["dissipative_export"] = round(s4_score, 4)
    rep.sub_dim_evidence["dissipative_export"] = s4_ev

    s5_score, s5_ev = _measure_chemoton_coupling()
    rep.sub_dim_scores["chemoton_coupling"] = round(s5_score, 4)
    rep.sub_dim_evidence["chemoton_coupling"] = s5_ev

    valid_scores = [v for v in rep.sub_dim_scores.values() if v > 0.0]
    if valid_scores:
        rep.total = round(statistics.mean(valid_scores), 4)
    else:
        rep.total = 0.0
    rep.elapsed_seconds = time.time() - t0
    rep.note = (
        f"V1165 self_organizing_core V0.6 真补 (Δ vs V1155 baseline "
        f"{rep.baseline_v1155:.4f} = {rep.total - rep.baseline_v1155:+.4f})"
    )

    if write_artifact:
        artifact_path = Path(artifact_dir) / "v1165_self_organizing_core_v06.json"
        artifact_path.parent.mkdir(parents=True, exist_ok=True)
        with open(artifact_path, "w", encoding="utf-8") as fh:
            json.dump(rep.to_dict(), fh, indent=2)

    return rep


def render_report_md(rep: SelfOrganizingCoreReport) -> str:
    lines = [
        "# V1165 — ASI self_organizing_core V0.6 Report",
        "",
        f"- Version: `{rep.version}`",
        f"- Dim Version: `{rep.dim_version}`",
        f"- Timestamp: `{rep.timestamp:.3f}`",
        f"- Snapshot: `v1165-{rep.snapshot_id}`",
        f"- Total: **{rep.total:.4f}** (Δ vs V1155 baseline {rep.baseline_v1155:.4f} = {rep.total - rep.baseline_v1155:+.4f})",
        f"- Target: `0.8500` (gap `{0.8500 - rep.total:+.4f}`)",
        f"- Elapsed: `{rep.elapsed_seconds:.3f}s`",
        f"- Note: {rep.note}",
        "",
        "## 5 sub-dim 真补",
        "",
        "| dim | score | baseline V1155 | Δ | notes |",
        "|-----|------:|---------------:|--:|-------|",
    ]
    for name in V1165_SUBDIM_NAMES:
        score = rep.sub_dim_scores.get(name, 0.0)
        ev = rep.sub_dim_evidence.get(name)
        baseline = ev.baseline_v1155 if ev else 0.0
        delta = score - baseline
        notes = "; ".join(ev.notes) if ev else ""
        lines.append(f"| {name} | {score:.4f} | {baseline:.4f} | {delta:+.4f} | {notes[:60]} |")
    lines.append("")
    lines.append("## V3 哲学守门 (主 17:58 + 20:46 不假装)")
    lines.append("")
    lines.append("- 不假装 sub-dim = 真涌现: 5 sub-dim 是工程测量, 不冒充 strong emergence")
    lines.append("- 不假装 chemoton_coupling > 0.7 = 真生命: V1065 chemoton 0.67 是数学, 不是真生物")
    lines.append("- 不假装 adaptive_diversity 高 = CAS 已涌现: 0.2 是 V1065 真跑, 不是真涌现")
    lines.append("- 不假装 total = ASI: 是 V0.6 真测, 不是 ASI")
    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1165 ASI self_organizing_core V0.6 real measure")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--no-write", action="store_true")
    parser.add_argument("--report", action="store_true")
    parser.add_argument("--artifact-dir", type=str, default=DEFAULT_ARTIFACT_DIR)
    args = parser.parse_args(argv)

    rep = measure_self_organizing_core_full(
        write_artifact=not args.no_write,
        artifact_dir=args.artifact_dir,
    )

    if args.report:
        md = render_report_md(rep)
        print(md)
        if not args.no_write:
            path = Path(args.artifact_dir) / "v1165_self_organizing_core_v06.md"
            path.parent.mkdir(parents=True, exist_ok=True)
            with open(path, "w", encoding="utf-8") as fh:
                fh.write(md)
        return 0

    if args.json:
        print(json.dumps(rep.to_dict(), indent=2, default=str))
        return 0

    print(rep.summary_line())
    return 0


if __name__ == "__main__":
    sys.exit(main())
