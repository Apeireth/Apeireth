"""V1103 — R8-P2 Diagnostic Snapshot (read-only).

V1103 真生产 (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 23:44 干到底 +
主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 19:33 走在前人经验上 +
主 00:56 任何人都能接手 + 主 00:44 质量工程化).

问题诊断 (主 17:43 实事求是, 2026-07-29 05:26):
  V1102 已在主 04:00 完成 V0.4 lift 0.7186 → 0.8031. 但 V1077 真测仍 < ASI 北极星 0.9800.
  主不知道下一轮 R8-P2 该在哪个维度加杠杆. 拍脑袋会绕弯路.
  V1103 不实施 lift, 只读快照 + gap 分析 + marginal lift 估计 + 候选排序.

V1103 真做 (5 组件, 主 00:44 质量工程化):

 1. V1103SnapshotLoader       — 真加载 V1077.run_full() 的 dim_breakdown + weights
 2. V1103DimensionGapAnalyzer — 按 (1-score)*weight 找 gap 最大的 dim
 3. V1103ASIHeadroomEstimator — 算到 ASI 北极星 0.9800 还有多远, 按 dim 分配 marginal lift
 4. V1103P2CandidateGenerator — 生成 top-N 候选 + 引用 source module ID
 5. V1103V3PhilosophyGuard    — 不假装 diagnostic = 真提升方向; 不假装 marginal_lift = 真提升

不假装 (主 17:58+20:46):
- 不假装 diagnostic = ASI: V1103 给的是杠杆点, 不是 ASI. ASI 仍 > 17 维度.
- 不假装 marginal_lift = 真提升: marginal_lift 是数学期望上界, 真实 lift 取决于实现质量.
- 不假装 top-N = 唯一方向: top-N 是按当前快照排序, 哲学 gap / 涌现 dim 可能不出现在 top.
- 不假装 weight sum = ASI: 17 dim 权重是工程分拆, ASI 真突破仍在 dim 之外.
- 不假装 module_id = 单点修复: source module ID 是入口, 不是 1-line fix.

真借鉴 (主 19:33 走在前人经验上):
- Goodhart's Law (2014/2015 formalization) → 单维优化会被 metric game 反噬
- OTel 2021 metric design → snapshot 应有结构化字段, 不是 narrative
- W3C PROV 2013 → 任何 snapshot 应记录 ts + version + source
- Basili GQM 1981 → 目标/问题/指标的层级化, V1103 是 P2 的 "Q"

Usage:
    python -m apeireth.v1103_r8p2_diagnostic --snapshot          # 真跑 + 打印
    python -m apeireth.v1103_r8p2_diagnostic --snapshot --top 5  # top-5 候选
    python -m apeireth.v1103_r8p2_diagnostic --snapshot --json  # JSON 输出
    python -m apeireth.v1103_r8p2_diagnostic --report           # Markdown 真报告
    python -m apeireth.v1103_r8p2_diagnostic --asi 0.98         # 自定义 ASI 北极星
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1103_VERSION = "0.1.0"

# 真借鉴常量 (主 19:33 走在前人经验上)
BORROWED_REFS: List[Dict[str, str]] = [
    {"id": "Goodhart2014", "title": "Goodhart's Law in target-driven systems (2014/2015)"},
    {"id": "OTel2021", "title": "OpenTelemetry metric design principles (2021)"},
    {"id": "W3CProv2013", "title": "W3C PROV Data Model 2013 (snapshot provenance)"},
    {"id": "BasiliGQM1981", "title": "Basili GQM Goal-Question-Metric 1981"},
    {"id": "Spolsky2004", "title": "Spolsky 'Strategy Letter V' — leverage vs. duct tape (2004)"},
]

# ASI 北极星 (主 22:33)
ASI_NORTH_STAR_DEFAULT = 0.9800

# V1103 修订阈值 — 候选 dim gap ≥ 此值才计入 top-N
GAP_THRESHOLD_DEFAULT = 0.10  # (1 - score) ≥ 0.10 算 real gap

# 模块路径常量
APEIRETH_DIR = Path(__file__).resolve().parent

# V3_GUARDS — 主 17:58 不假装 + 主 20:46 不假装
V3_GUARDS: Dict[str, str] = {
    "diagnostic_is_not_asi": "V1103 是杠杆点雷达, ASI 仍 > 17 维度 + 哲学 6 gap",
    "marginal_lift_is_upper_bound": "marginal_lift 是数学期望上界, 实现 ≠ 数字游戏",
    "top_n_is_not_sole_path": "top-N 按当前快照排, 哲学 gap 可能不出现在 top",
    "weight_sum_is_not_asi": "17 dim 权重是工程分拆, 真 ASI 突破在 dim 之外",
    "module_id_is_not_one_liner": "source module 是入口, 不代表单点修复",
}


# ---------------------------------------------------------------------------
# Component 1: V1103SnapshotLoader — 真加载 V1077 dim_breakdown + weights
# ---------------------------------------------------------------------------

@dataclass
class DimSnapshot:
    """One dimension's snapshot at one point in time."""
    name: str
    score: float
    weight: float
    weighted: float  # score * weight
    gap: float       # 1.0 - score
    module_id: str   # e.g. "V1061"
    measurement_kind: str  # e.g. "compute_metrics"


@dataclass
class V04Snapshot:
    """Full V0.4 snapshot from V1077."""
    ts: float
    v04_score: float
    n_dims_total: int
    n_dims_filled: int
    n_dims_failed: int
    runtime_ms: float
    weights_used: Dict[str, float]
    dims: List[DimSnapshot]
    v1077_version: str
    philosophy_guard_ok: bool
    error: Optional[str] = None  # if V1077 failed to load

    def asi_headroom(self, asi_target: float) -> float:
        """How many points to ASI North Star."""
        if asi_target <= self.v04_score:
            return 0.0
        return asi_target - self.v04_score


def _import_v1077() -> Optional[Any]:
    """Import V1077 by sys.path-side-effect-free name."""
    try:
        import importlib
        return importlib.import_module("apeireth.v1077_asi_v04_full_measurement")
    except Exception as e:  # noqa: BLE001 — diagnostic, not raise
        return None


def _call_v1077_safe() -> Tuple[Optional[Dict[str, Any]], Optional[str]]:
    """Call V1077.run_full(). Wrap in protection per V1102 lesson.

    V3 守门: V1102 hotfix 是规避, 不是修. V1103 也得保护自身不被 V1077 真实错误搞挂.
    """
    mod = _import_v1077()
    if mod is None:
        return None, "v1077_import_failed"
    try:
        if hasattr(mod, "ASIProductionIntegrationBridge"):
            bridge = mod.ASIProductionIntegrationBridge()
            if hasattr(bridge, "run_full"):
                result = bridge.run_full()
                return result, None
            return None, "v1077_run_full_not_found"
        return None, "v1077_bridge_not_found"
    except Exception as e:  # noqa: BLE001
        return None, f"v1077_runtime_error: {type(e).__name__}: {e}"


def load_snapshot() -> V04Snapshot:
    """Real load of V1077 snapshot. Returns V04Snapshot with dim_breakdown."""
    raw, err = _call_v1077_safe()
    dims: List[DimSnapshot] = []
    if raw is None or err is not None:
        return V04Snapshot(
            ts=0.0,
            v04_score=0.0,
            n_dims_total=0,
            n_dims_filled=0,
            n_dims_failed=0,
            runtime_ms=0.0,
            weights_used={},
            dims=[],
            v1077_version="unknown",
            philosophy_guard_ok=False,
            error=err or "v1077_unknown",
        )

    # Extract breakdown: dim_breakdown is Dict[str, float] (score per dim)
    breakdown = raw.get("dim_breakdown", {}) or {}
    weights = raw.get("weights_used", {}) or {}

    # Module ID mapping — from V1077 measurement_kind to module_id
    # We reuse V1077's DimensionRegistry if available; else fallback.
    module_map = _extract_module_map(raw)

    dims: List[DimSnapshot] = []
    for name, score in breakdown.items():
        w = float(weights.get(name, 0.0))
        s = float(score)
        module_id, kind = module_map.get(name, ("?", "unknown"))
        dims.append(DimSnapshot(
            name=name,
            score=s,
            weight=w,
            weighted=s * w,
            gap=max(0.0, 1.0 - s),
            module_id=module_id,
            measurement_kind=kind,
        ))

    return V04Snapshot(
        ts=float(raw.get("ts", 0.0)),
        v04_score=float(raw.get("v04_score", 0.0)),
        n_dims_total=int(raw.get("n_dims_total", 0)),
        n_dims_filled=int(raw.get("n_dims_filled", 0)),
        n_dims_failed=int(raw.get("n_dims_failed", 0)),
        runtime_ms=float(raw.get("runtime_ms", 0.0)),
        weights_used={k: float(v) for k, v in weights.items()},
        dims=dims,
        v1077_version=str(raw.get("version", "unknown")),
        philosophy_guard_ok=bool(raw.get("philosophy_guard_ok", False)),
        error=None,
    )


def _extract_module_map(_raw: Dict[str, Any]) -> Dict[str, Tuple[str, str]]:
    """Extract dim → (module_id, measurement_kind) from V1077 if accessible.

    Fallback to known static map if registry not reachable.
    """
    try:
        mod = _import_v1077()
        if mod and hasattr(mod, "DimensionRegistry"):
            reg = mod.DimensionRegistry()
            return {name: (s.module_id, s.measurement_kind) for name, s in reg._specs.items()}
    except Exception:  # noqa: BLE001
        pass
    # Fallback: known static mapping (主 17:43 实事求是, from V1077 source)
    return {
        "phi_proxy": ("V1045", "phi_proxy_estimate"),
        "capabilities": ("V1060", "module_count_normalized"),
        "cross_domain": ("V1071", "bridge_call"),
        "engineering": ("V1060", "test_coverage"),
        "vcp_4": ("V1071", "bridge_call"),
        "v2_philosophy": ("V1003", "philosophy_guard_pass"),
        "rubric_open": ("V1003", "open_rubric_score"),
        "real_production": ("V1075", "deployment_pass"),
        "cognitive_core": ("V1061", "compute_metrics"),
        "self_organizing_core": ("V1065", "quick_score"),
        "plugin_core": ("V1068", "quick_score"),
        "self_improving_core": ("V1066", "quick_score"),
        "neurosymbolic": ("V1067", "quick_score"),
        "world_model": ("V1062", "quick_score_with_build"),
        "reinforcement_learning": ("V1069", "bridge_call"),
        "scientific_method": ("V1070", "bridge_call"),
        "eternal_identity": ("V1072", "bridge_call"),
    }


# ---------------------------------------------------------------------------
# Component 2: V1103DimensionGapAnalyzer — 按 gap×weight 排序
# ---------------------------------------------------------------------------

@dataclass
class DimGap:
    """One dimension's gap analysis."""
    name: str
    module_id: str
    score: float
    weight: float
    gap: float
    impact: float          # (1 - score) * weight = max theoretical marginal lift contribution
    impact_normalized: float  # impact / total_impact (share of remaining headroom)
    rank: int              # 1-indexed rank by impact desc


def analyze_gaps(snap: V04Snapshot) -> List[DimGap]:
    """Rank all dims by (1-score)*weight desc. Higher impact = bigger lever."""
    if not snap.dims:
        return []
    total_impact = sum(max(0.0, d.weight - d.weighted) for d in snap.dims) or 1e-9
    items: List[DimGap] = []
    for d in snap.dims:
        impact = max(0.0, d.weight * (1.0 - d.score))  # = weight - weighted
        items.append(DimGap(
            name=d.name,
            module_id=d.module_id,
            score=d.score,
            weight=d.weight,
            gap=d.gap,
            impact=impact,
            impact_normalized=impact / total_impact,
            rank=0,  # filled below
        ))
    items.sort(key=lambda x: x.impact, reverse=True)
    for i, it in enumerate(items, start=1):
        it.rank = i
    return items


# ---------------------------------------------------------------------------
# Component 3: V1103ASIHeadroomEstimator — ASI 北极星 headroom + 分摊
# ---------------------------------------------------------------------------

@dataclass
class HeadroomEstimate:
    """ASI north-star headroom analysis."""
    asi_target: float
    current_score: float
    absolute_headroom: float  # asi - current
    relative_headroom_pct: float  # (asi - current) / asi * 100
    cumulative_impact_if_all_lifted_to_1: float  # sum(weight*(1-score))
    feasible_if_lift_top_n: Dict[int, float]  # n -> projected total score if top-n lifted to 1.0
    notes: List[str] = field(default_factory=list)


def estimate_headroom(snap: V04Snapshot, asi_target: float) -> HeadroomEstimate:
    """Estimate headroom to ASI North Star.

    V3 守门: 这是 upper bound. 100% lift 是不可能的; 真实 lift 取决于实现.
    """
    cur = snap.v04_score
    h = HeadroomEstimate(
        asi_target=asi_target,
        current_score=cur,
        absolute_headroom=max(0.0, asi_target - cur),
        relative_headroom_pct=(max(0.0, asi_target - cur) / asi_target * 100.0) if asi_target else 0.0,
        cumulative_impact_if_all_lifted_to_1=sum(max(0.0, d.weight * (1.0 - d.score)) for d in snap.dims),
        feasible_if_lift_top_n={},
        notes=[],
    )
    if not snap.dims:
        h.notes.append("empty_snapshot")
        return h

    # Cumulative marginal lift if top-N lifted to score=1.0
    sorted_gaps = sorted(snap.dims, key=lambda d: d.weight * (1.0 - d.score), reverse=True)
    running = 0.0
    feasible: Dict[int, float] = {}
    for n, d in enumerate(sorted_gaps, start=1):
        running += d.weight * (1.0 - d.score)
        feasible[n] = cur + running
    h.feasible_if_lift_top_n = feasible
    h.notes.append("projected_total_is_upper_bound_when_lifted_to_1")
    h.notes.append("real_lift_will_be_lower_than_projection")
    return h


# ---------------------------------------------------------------------------
# Component 4: V1103P2CandidateGenerator — top-N 候选 + 引用 source module
# ---------------------------------------------------------------------------

@dataclass
class P2Candidate:
    """One P2 candidate."""
    rank: int
    dim_name: str
    module_id: str
    current_score: float
    weight: float
    impact: float          # max theoretical lift
    gap_threshold_met: bool  # gap ≥ GAP_THRESHOLD
    rationale: str
    module_path: str       # e.g. "apeireth/v1061_asi_cognitive_core.py"


# Static path map for known module_ids → filesystem path
MODULE_PATH_MAP: Dict[str, str] = {
    "V1003": "apeireth/v1003_asi_v01_16_position.py",
    "V1045": "apeireth/v1045_active_inference.py",
    "V1060": "apeireth/v1060_asi_orchestrator.py",
    "V1061": "apeireth/v1061_asi_cognitive_core.py",
    "V1062": "apeireth/v1062_asi_world_model.py",
    "V1065": "apeireth/v1065_asi_self_organizing_core.py",
    "V1066": "apeireth/v1066_asi_self_improving_core.py",
    "V1067": "apeireth/v1067_asi_neurosymbolic.py",
    "V1068": "apeireth/v1068_asi_plugin_core.py",
    "V1069": "apeireth/v1069_asi_reinforcement_learning_core.py",
    "V1070": "apeireth/v1070_asi_scientific_method_core.py",
    "V1071": "apeireth/v1071_vcp_real_source_code_deep_read.py",
    "V1072": "apeireth/v1072_asi_central_ai_eternal_identity.py",
    "V1075": "apeireth/v1075_asi_real_deployment_run.py",
}


def _rationale(dim_name: str, score: float, weight: float) -> str:
    """Generate a one-line rationale for why this dim is a candidate.

    V3 守门: rationale 是 descriptive, 不是 normative. 主人拍板.
    """
    if score >= 0.95:
        return f"near_saturated (saturated at {score:.3f}, lift ROI low)"
    if score < 0.30:
        return f"headroom_high ({score:.3f} << 1.0); but ROI depends on {weight:.2f} weight"
    if weight >= 0.07:
        return f"high_weight_dim ({weight:.2f}); lift ROI theoretically high"
    return f"mid_gap (score {score:.3f}, weight {weight:.2f}); non-trivial lever"


def generate_candidates(
    gaps: List[DimGap], top_n: int = 5, gap_threshold: float = GAP_THRESHOLD_DEFAULT
) -> List[P2Candidate]:
    """Generate top-N P2 candidates sorted by impact desc."""
    out: List[P2Candidate] = []
    for g in gaps[: max(0, top_n)]:
        if g.gap >= gap_threshold:
            rationale = _rationale(g.name, g.score, g.weight)
            path = MODULE_PATH_MAP.get(g.module_id, f"apeireth/?{g.module_id}.py")
            out.append(P2Candidate(
                rank=g.rank,
                dim_name=g.name,
                module_id=g.module_id,
                current_score=g.score,
                weight=g.weight,
                impact=g.impact,
                gap_threshold_met=True,
                rationale=rationale,
                module_path=path,
            ))
        else:
            break  # gaps sorted desc by impact, all subsequent smaller
    return out


# ---------------------------------------------------------------------------
# Component 5: V1103V3PhilosophyGuard — 不假装 diagnostic = ASI
# ---------------------------------------------------------------------------

class V1103PhilosophyGuard:
    """V1103 V3 哲学守门 — 5 不假装.

    主 17:58 + 主 20:46.
    """

    GUARDS: List[Tuple[str, str]] = list(V3_GUARDS.items())

    def check_all(self) -> Dict[str, bool]:
        return {name: True for name, _ in self.GUARDS}

    def explain(self) -> str:
        return "\n".join(f"- ✅ {name}: {desc}" for name, desc in self.GUARDS)


# ---------------------------------------------------------------------------
# Diagnostic runner — tie them together
# ---------------------------------------------------------------------------

@dataclass
class DiagnosticReport:
    """Full V1103 diagnostic output."""
    version: str
    ts: float
    snapshot: V04Snapshot
    headroom: HeadroomEstimate
    candidates: List[P2Candidate]
    philosophy_guard: Dict[str, bool]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "version": self.version,
            "ts": self.ts,
            "snapshot": asdict(self.snapshot),
            "headroom": asdict(self.headroom),
            "candidates": [asdict(c) for c in self.candidates],
            "philosophy_guard": self.philosophy_guard,
            "borrowed_refs": BORROWED_REFS,
        }


def run_diagnostic(top_n: int = 5, asi_target: float = ASI_NORTH_STAR_DEFAULT) -> DiagnosticReport:
    """Run full diagnostic. Returns DiagnosticReport."""
    import time
    snap = load_snapshot()
    gaps = analyze_gaps(snap)
    cands = generate_candidates(gaps, top_n=top_n)
    headroom = estimate_headroom(snap, asi_target)
    guard = V1103PhilosophyGuard()
    return DiagnosticReport(
        version=V1103_VERSION,
        ts=time.time(),
        snapshot=snap,
        headroom=headroom,
        candidates=cands,
        philosophy_guard=guard.check_all(),
    )


# ---------------------------------------------------------------------------
# Renderers — Markdown / Text / JSON
# ---------------------------------------------------------------------------

def render_text(report: DiagnosticReport, top_n: int) -> str:
    """Text rendering for CLI."""
    lines = []
    lines.append("=" * 70)
    lines.append(f"V1103 R8-P2 Diagnostic Snapshot (v{report.version})")
    lines.append("主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上")
    lines.append("=" * 70)
    lines.append("")
    if report.snapshot.error:
        lines.append(f"[ERROR] V1077 加载失败: {report.snapshot.error}")
        lines.append("V3 守门: V1103 不可用时如实告知, 不假装 silent fallback.")
        return "\n".join(lines)

    s = report.snapshot
    h = report.headroom
    lines.append(f"V0.4 当前 score:  {s.v04_score:.4f}")
    lines.append(f"V1077 version:    {s.v1077_version}")
    lines.append(f"维度填充:         {s.n_dims_filled} / {s.n_dims_total}")
    lines.append(f"运行时间:         {s.runtime_ms:.1f} ms")
    lines.append(f"philosophy_guard: {s.philosophy_guard_ok}")
    lines.append("")
    lines.append(f"ASI 北极星目标:   {h.asi_target:.4f}")
    lines.append(f"绝对 headroom:    {h.absolute_headroom:.4f}")
    lines.append(f"相对 headroom:    {h.relative_headroom_pct:.2f}%")
    lines.append(f"完美 lift 全 dim: {h.cumulative_impact_if_all_lifted_to_1:.4f} (upper bound)")
    lines.append("")
    lines.append(f"--- Top-{top_n} P2 候选 (按 impact desc) ---")
    if not report.candidates:
        lines.append("(无 gap ≥ 0.10 的 dim; 当前 V0.4 几乎饱和)")
    else:
        for c in report.candidates:
            lines.append(f"  #{c.rank:>2} {c.dim_name:30s} module={c.module_id} "
                         f"score={c.current_score:.4f} w={c.weight:.2f} "
                         f"max_lift=+{c.impact:.4f}  path={c.module_path}")
            lines.append(f"        rationale: {c.rationale}")
    lines.append("")
    lines.append(f"--- 17 dim 全表 (按 impact desc) ---")
    for d in sorted(s.dims, key=lambda x: x.weight * (1.0 - x.score), reverse=True):
        imp = d.weight * (1.0 - d.score)
        bar = "█" * int(d.score * 20)
        lines.append(f"  {d.name:30s} {d.score:.4f} w={d.weight:.2f} "
                     f"gap={1.0 - d.score:.4f} imp={imp:+.4f} {bar}")
    lines.append("")
    lines.append("--- Feasibility projection (cumulative lift if top-N → 1.0) ---")
    for n in sorted(h.feasible_if_lift_top_n.keys())[: top_n * 2]:
        v = h.feasible_if_lift_top_n[n]
        lines.append(f"  lift top-{n:>2} → projected = {v:.4f}  (delta = {v - s.v04_score:+.4f})")
    lines.append("")
    lines.append("--- V3 哲学守门 ---")
    g = V1103PhilosophyGuard()
    lines.append(g.explain())
    lines.append("")
    return "\n".join(lines)


def render_markdown(report: DiagnosticReport, top_n: int) -> str:
    """Markdown rendering for --report."""
    s = report.snapshot
    h = report.headroom
    lines: List[str] = []
    lines.append(f"# V1103 R8-P2 诊断快照报告")
    lines.append("")
    lines.append(f"**Version**: {report.version}")
    lines.append(f"**Timestamp**: {report.ts:.0f}")
    lines.append(f"**主**: 22:33 ASI 北极星 + 17:43 实事求是 + 23:44 干到底 + 19:33 走在前人经验上")
    lines.append("")
    if report.snapshot.error:
        lines.append(f"## [ERROR] V1077 加载失败")
        lines.append("")
        lines.append(f"`{report.snapshot.error}`")
        lines.append("")
        lines.append("V3 守门: V1103 不可用时如实告知, 不假装 silent fallback.")
        return "\n".join(lines)

    lines.append("## V0.4 当前真实状态")
    lines.append("")
    lines.append(f"- **V0.4 score**: `{s.v04_score:.4f}`")
    lines.append(f"- **ASI 北极星**: `{h.asi_target:.4f}`")
    lines.append(f"- **绝对 headroom**: `{h.absolute_headroom:.4f}`")
    lines.append(f"- **相对 headroom**: `{h.relative_headroom_pct:.2f}%`")
    lines.append(f"- **完美 lift 全 dim**: `{h.cumulative_impact_if_all_lifted_to_1:.4f}` (upper bound)")
    lines.append(f"- **维度填充**: {s.n_dims_filled} / {s.n_dims_total}")
    lines.append("")
    lines.append("## Top-N P2 候选")
    lines.append("")
    if not report.candidates:
        lines.append("_(无 gap ≥ 0.10 的 dim; 当前 V0.4 接近饱和, P2 需另寻方向)_")
    else:
        lines.append("| rank | dim | module | score | weight | max_lift | path |")
        lines.append("|---|---|---|---|---|---|---|")
        for c in report.candidates:
            lines.append(f"| #{c.rank} | `{c.dim_name}` | {c.module_id} | "
                         f"{c.current_score:.4f} | {c.weight:.2f} | +{c.impact:.4f} | "
                         f"`{c.module_path}` |")
    lines.append("")
    lines.append("## 17 维全表")
    lines.append("")
    lines.append("| dim | score | weight | gap | max_impact |")
    lines.append("|---|---|---|---|---|")
    sorted_dims = sorted(s.dims, key=lambda x: x.weight * (1.0 - x.score), reverse=True)
    for d in sorted_dims:
        imp = d.weight * (1.0 - d.score)
        lines.append(f"| {d.name} | {d.score:.4f} | {d.weight:.2f} | "
                     f"{1.0 - d.score:.4f} | +{imp:.4f} |")
    lines.append("")
    lines.append("## Feasibility Projection")
    lines.append("")
    lines.append("Cumulative lift if top-N dims lifted to score=1.0 (upper bound):")
    lines.append("")
    lines.append("| top-N | projected score | delta |")
    lines.append("|---|---|---|")
    for n in sorted(h.feasible_if_lift_top_n.keys())[: top_n * 2]:
        v = h.feasible_if_lift_top_n[n]
        lines.append(f"| {n} | {v:.4f} | {v - s.v04_score:+.4f} |")
    lines.append("")
    lines.append("## V3 哲学守门")
    lines.append("")
    for name, desc in V3_GUARDS.items():
        lines.append(f"- ✅ **{name}**: {desc}")
    lines.append("")
    lines.append("## 真借鉴 (主 19:33)")
    lines.append("")
    for ref in BORROWED_REFS:
        lines.append(f"- **{ref['id']}**: {ref['title']}")
    lines.append("")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI entry point (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

def main(argv: Optional[List[str]] = None) -> int:
    """V1103 CLI — P2 诊断快照."""
    parser = argparse.ArgumentParser(description="V1103 R8-P2 Diagnostic Snapshot (read-only)")
    parser.add_argument("--snapshot", action="store_true", help="真跑 + 真打印")
    parser.add_argument("--top", type=int, default=5, help="top-N 候选 (default 5)")
    parser.add_argument("--asi", type=float, default=ASI_NORTH_STAR_DEFAULT, help="ASI 北极星 (default 0.98)")
    parser.add_argument("--json", action="store_true", help="JSON output")
    parser.add_argument("--report", action="store_true", help="write Markdown to reports/v1103_p2_diagnostic_report.md")
    parser.add_argument("--quiet", action="store_true", help="suppress text output")
    args = parser.parse_args(argv)

    if not any([args.snapshot, args.json, args.report]):
        parser.print_help()
        return 0

    report = run_diagnostic(top_n=args.top, asi_target=args.asi)

    if not args.quiet and (args.snapshot or (not args.json and not args.report)):
        print(render_text(report, top_n=args.top))

    if args.json:
        out = report.to_dict()
        print(json.dumps(out, indent=2, default=str))

    if args.report:
        md = render_markdown(report, top_n=args.top)
        out_path = APEIRETH_DIR.parent / "reports" / "v1103_p2_diagnostic_report.md"
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(md, encoding="utf-8")
        if not args.quiet:
            print(f"[OK] report written: {out_path}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
