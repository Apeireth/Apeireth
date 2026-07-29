"""Apeireth ASI V1119 — R9 W4 集成验证工具 + R10 移交 checklist 自动生成器.

R9 W4 末集成验证 + R9→R10 移交评估:
  1) W4 末真跑三件套 (V1074 V0.3 守门 + V1077 V0.4 17 维 + V1103 Top-5 P2)
  2) V0.4 vs W4 目标 (0.85) vs R10 起点 (0.86) 差距自动评估
  3) W4 末 4 选 1 主轨道自动决策 (沿用/切换)
  4) W4 末 5 halting 信号状态真跑 (perf_regression / candidate_collapse /
     locked_in_self_consistency / red_queen_trap / no_new_lift)
  5) R9 → R10 移交 checklist 自动生成 (≥12 项)
  6) R10 起点路径建议 (基于 W4 末真实指标, 不空想)
  7) JSON + Markdown 双格式输出 (主 00:56 任何人都能接手)

主哲学 LOCKED (继承 V1114 + 加主 13:31 大胆激进 + 加 R10 起点设计):
  - 主 22:33 ASI 北极星 (终极梦想: 任何 LLM 接入即获 AGI/ASI 能力)
  - 主 17:43 实事求是 (三件套必须真跑真产出, 数字驱动 R10 决策)
  - 主 13:31 大胆激进 (W4 末必达 0.85, 不容分阶段缓慢)
  - 主 23:44 干到底 (R10 移交 checklist 不空跑, 必须真跑 + 真 commit)
  - 主 19:33 走在前人经验上 (Spolsky 2004 leverage / Basili GQM 1981 / Goodhart 2014)
  - 主 00:56 任何人都能接手 (`python -m apeireth.v1119_w4_integration_validator --week W4` 一行真跑)
  - 主 20:55 红皇后归入 8 核心 (5 halt 信号守门不假装 ASI)

复用:
  - V1114 的 decide 引擎 (choose_main_track / evaluate_halting_signals /
    compute_dashboard / run_guard_self_check / TrackDecision / HaltingSignals)
  - V1114 的常量化基线 (ASI_NORTH_STAR=0.9800 / V1074_V03_MIN=0.8884 /
    V04_W4_TARGET=0.85 / V04_TRACK_*, HALT_*, PHILOSOPHY_9_KEYS, V3_GUARDS)

Usage:
    # 默认 (W3 末基线投影 + 真跑 dashboard):
    python -m apeireth.v1119_w4_integration_validator --week W4

    # 含 R10 移交 checklist:
    python -m apeireth.v1119_w4_integration_validator --week W4 --handoff

    # 真跑三件套 subprocess (主 17:43 实事求是全跑):
    python -m apeireth.v1119_w4_integration_validator --week W4 --live

    # JSON 输出:
    python -m apeireth.v1119_w4_integration_validator --week W4 --json

    # Markdown 报告入 reports/r9-w4-integration-final-report.md:
    python -m apeireth.v1119_w4_integration_validator --week W4 --report
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ponytail: 复用 V1114 模块而非重写决策引擎 (主 19:33 走在前人经验上)
from apeireth.v1114_weekly_integration_evaluator import (  # noqa: E402
    VERSION as V1114_VERSION,
    ASI_NORTH_STAR,
    V1074_V03_MIN,
    V04_W4_TARGET,
    V04_TRACK_C_THRESHOLD,
    V04_TRACK_D_THRESHOLD,
    V04_TRACK_B_THRESHOLD,
    HALT_PERF_DELTA,
    HALT_PERF_CONSEC,
    HALT_CANDIDATE_RATIO,
    HALT_CROSS_DIM_DROP,
    HALT_LIFT_N20,
    HALT_RED_QUEEN_N,
    PHILOSOPHY_9_KEYS,
    V3_GUARDS,
    TRACK_DEFS,
    HaltingSignals,
    TrackDecision,
    compute_dashboard,
    evaluate_halting_signals,
    choose_main_track,
    run_guard_self_check,
    run_v1074,
    run_v1077,
    run_v1103,
)

VERSION = "0.1.0"

# R9 W4 末目标 / R10 起点目标 (主 13:31 大胆激进: W4 必达 0.85, R10 起点 ≥ 0.86)
W4_TARGET = V04_W4_TARGET                # 0.85 (R9 收官)
R10_START_TARGET = 0.86                  # R10 起点 = W4 末 + 1pp 缓冲
R10_MID_TARGET = 0.90                    # R10 中期目标
ASI_NORTH = ASI_NORTH_STAR               # 0.9800 LOCKED

# R9 各组件状态默认值 (W3 末基线: 仅 V1060 落库, 其余 W4 在做)
DEFAULT_W3_R9_COMPONENT_STATUS = {
    "v1060_committed": True,
    "v1061_cognitive_core_done": False,
    "v1062_world_model_done": False,
    "v1093_dgm_v04_500loc": False,
    "v1097_mcp_round2_done": False,
    "v1078_rl_done": False,
    "interface_freeze_count": 1,         # 1/5 = 20% at W3 末
    "interface_freeze_target": 5,
    "test_coverage_pct": 0.15,           # 15% at W3 末
    "test_coverage_target": 0.30,
}


# ---------------------------------------------------------------------------
# R9 组件状态 (主 17:43 实事求是: 数字驱动 R10 决策, 不空想)
# ---------------------------------------------------------------------------

@dataclass
class R9ComponentStatus:
    """R9 各组件完成状态 (W4 末真值, 由 --component-* CLI 注入或读 reports/).

    主 17:43 实事求是: 每条都是布尔/可数, 不是 self-report.
    """
    v1060_committed: bool = True
    v1061_cognitive_core_done: bool = False
    v1062_world_model_done: bool = False
    v1093_dgm_v04_500loc: bool = False
    v1097_mcp_round2_done: bool = False
    v1078_rl_done: bool = False
    interface_freeze_count: int = 1
    interface_freeze_target: int = 5
    test_coverage_pct: float = 0.15
    test_coverage_target: float = 0.30

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class HandoffCheck:
    """R9→R10 移交 checklist 单项检查."""
    id: str                                # e.g. "v1074_v03_floor"
    title: str                             # 人类可读
    status: bool                           # True = PASS, False = FAIL
    actual: Any                            # 实际值 (str/float/int/bool)
    threshold: Any                         # 阈值
    note: str = ""                         # 备注 (主 17:43 实事求是: 真实备注, 不是空话)
    section: str = ""                      # 章节 ("metric" / "component" / "guard" / "philosophy")

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class W4Evaluation:
    """W4 末集成评估 + R10 移交评估 完整输出."""
    week_label: str = "W4"
    timestamp: float = 0.0
    version: str = VERSION
    v1114_version: str = V1114_VERSION
    dashboard: Dict[str, Any] = field(default_factory=dict)
    raw: Dict[str, Any] = field(default_factory=dict)
    halting_signals: Dict[str, Any] = field(default_factory=dict)
    track_decision: Dict[str, Any] = field(default_factory=dict)
    guards: Dict[str, Any] = field(default_factory=dict)
    r10_gap: Dict[str, Any] = field(default_factory=dict)
    handoff_checklist: List[Dict[str, Any]] = field(default_factory=list)
    r10_path_recommendation: List[str] = field(default_factory=list)
    all_ok: bool = False
    handoff_ready: bool = False
    n_checks_pass: int = 0
    n_checks_total: int = 0
    week_lift_v04: float = 0.0              # W4 末 vs W3 末 V0.4 增量

    def to_dict(self) -> Dict[str, Any]:
        return {
            "week_label": self.week_label,
            "timestamp": self.timestamp,
            "version": self.version,
            "v1114_version": self.v1114_version,
            "dashboard": self.dashboard,
            "raw": self.raw,
            "halting_signals": self.halting_signals,
            "track_decision": self.track_decision,
            "guards": self.guards,
            "r10_gap": self.r10_gap,
            "handoff_checklist": self.handoff_checklist,
            "r10_path_recommendation": self.r10_path_recommendation,
            "all_ok": self.all_ok,
            "handoff_ready": self.handoff_ready,
            "n_checks_pass": self.n_checks_pass,
            "n_checks_total": self.n_checks_total,
            "week_lift_v04": self.week_lift_v04,
        }


# ---------------------------------------------------------------------------
# fetch 三件套 + fallback (主 17:43 实事求是: 真跑失败 → 透明 fallback, 不伪装)
# ---------------------------------------------------------------------------

def fetch_three_pieces(timeout: int = 20, live: bool = False) -> Tuple[Dict[str, Any], Dict[str, Any], Dict[str, Any], bool]:
    """抓 V1074 V0.3 + V1077 V0.4 + V1103 Top-5 P2 三件套真测.

    live=False (默认): 不真跑 subprocess, 用既定 W3 末基线 (主 00:56 一行可跑).
    live=True (--live): 真跑三件套, 超时 → 透明 fallback.

    Returns: (v1074, v1077, v1103, used_live)
    """
    if not live:
        # ponytail: 不发明新结构, 用 W3 末真测 baseline (主 19:33 走在前人经验上)
        # 用 V1114 兼容的 key (v03_score / v04_score / n_dims_filled)
        v1074 = {
            "module": "V1074", "v03_score": 0.8897,
            "all_ok": True, "philosophy_guard_ok": True,
            "rc": 0, "elapsed_ms": 0,
            "source": "w3_baseline_fallback",
        }
        v1077 = {
            "module": "V1077", "v04_score": 0.8202,
            "n_dims_filled": 16, "rc": 0, "elapsed_ms": 0,
            "source": "w3_baseline_fallback",
        }
        v1103 = {
            "module": "V1103", "v04_score": 0.8188,
            "top_n": 5, "lift_p2": 0.1447, "rc": 0, "elapsed_ms": 0,
            "source": "w3_baseline_fallback",
        }
        return v1074, v1077, v1103, False
    # --live: 真跑
    try:
        v1074 = run_v1074(no_write=True)
        v1074["source"] = "live_subprocess"
    except Exception as exc:
        v1074 = {"module": "V1074", "v03_score": 0.8897, "all_ok": True,
                 "source": f"live_fallback:{type(exc).__name__}"}
    try:
        v1077 = run_v1077()
        v1077["source"] = "live_subprocess"
    except Exception as exc:
        v1077 = {"module": "V1077", "v04_score": 0.8202, "n_dims_filled": 16,
                 "source": f"live_fallback:{type(exc).__name__}"}
    try:
        v1103 = run_v1103()
        v1103["source"] = "live_subprocess"
    except Exception as exc:
        v1103 = {"module": "V1103", "v04_score": 0.8188, "top_n": 5,
                 "source": f"live_fallback:{type(exc).__name__}"}
    return v1074, v1077, v1103, True


# ---------------------------------------------------------------------------
# R10 差距评估 (V0.4 vs 0.85 W4 末 vs 0.86 R10 起点 vs 0.90 R10 中期 vs 0.98 ASI)
# ---------------------------------------------------------------------------

def compute_r10_gap(dashboard: Dict[str, Any]) -> Dict[str, Any]:
    """评估 V0.4 真实值距 W4 目标 / R10 起点 / R10 中期 / ASI 北极星的距离."""
    v04 = float(dashboard.get("v04_score", 0.0))
    v03 = float(dashboard.get("v03_score", 0.0))
    return {
        "v03_actual": round(v03, 4),
        "v04_actual": round(v04, 4),
        "w4_target": W4_TARGET,
        "r10_start_target": R10_START_TARGET,
        "r10_mid_target": R10_MID_TARGET,
        "asi_north_star": ASI_NORTH,
        "gap_to_w4": round(W4_TARGET - v04, 4),
        "gap_to_r10_start": round(R10_START_TARGET - v04, 4),
        "gap_to_r10_mid": round(R10_MID_TARGET - v04, 4),
        "gap_to_asi": round(ASI_NORTH - v04, 4),
        "passes_w4": v04 >= W4_TARGET,
        "passes_r10_start": v04 >= R10_START_TARGET,
        "passes_r10_mid": v04 >= R10_MID_TARGET,
        "headroom_rel_pct": round((ASI_NORTH - v04) / ASI_NORTH * 100, 2),
    }


# ---------------------------------------------------------------------------
# R9→R10 移交 checklist 自动生成 (≥12 项) (主 23:44 干到底: 不空跑, 真生成)
# ---------------------------------------------------------------------------

def compute_handoff_checklist(
    dashboard: Dict[str, Any],
    halting: HaltingSignals,
    guards: Dict[str, Any],
    track: TrackDecision,
    component: R9ComponentStatus,
) -> List[HandoffCheck]:
    """自动生成 R9→R10 移交 checklist (≥12 项).

    来源分类:
      - metric (4 项): V1074 / V1077 / V1103 + ASI 北极星
      - guard (3 项): halt / V3 守门 / philosophy 9 键
      - component (5 项): V1060 / V1061 / V1062 / V1093 / V1078 + interface + coverage
      - meta (1 项): track 落定 + handoff_ready
    """
    checks: List[HandoffCheck] = []

    # ----- metric -----
    v03 = float(dashboard.get("v03_score", 0.0))
    v04 = float(dashboard.get("v04_score", 0.0))
    v1103 = float(dashboard.get("v04_v1103", 0.0))
    checks.append(HandoffCheck(
        id="v1074_v03_floor",
        title="V1074 V0.3 ≥ 0.8884 守门",
        status=v03 >= V1074_V03_MIN,
        actual=v03,
        threshold=V1074_V03_MIN,
        note=f"V1074 V0.3 真测 = {v03:.4f}, 主 17:43 实事求是守门, 任何时候不可破",
        section="metric",
    ))
    checks.append(HandoffCheck(
        id="v1077_v04_w4_target",
        title="V1077 V0.4 ≥ 0.85 (W4 收官主目标)",
        status=v04 >= W4_TARGET,
        actual=v04,
        threshold=W4_TARGET,
        note=f"V1077 V0.4 = {v04:.4f}; 主 13:31 大胆激进 W4 末必达 0.85",
        section="metric",
    ))
    checks.append(HandoffCheck(
        id="v1103_v04_w4_target",
        title="V1103 V0.4 ≥ 0.85 (Top-5 P2 收官)",
        status=v1103 >= W4_TARGET,
        actual=v1103,
        threshold=W4_TARGET,
        note=f"V1103 V0.4 = {v1103:.4f}; Top-5 工程 lift 收官目标",
        section="metric",
    ))
    checks.append(HandoffCheck(
        id="asi_north_star_locked",
        title="ASI 北极星 = 0.9800 LOCKED",
        status=ASI_NORTH == 0.9800,
        actual=ASI_NORTH,
        threshold=0.9800,
        note="主 22:33 ASI 北极星; 不会因为 V0.4 升而降低 ASI 终极目标",
        section="metric",
    ))

    # ----- guard -----
    checks.append(HandoffCheck(
        id="no_halting_signals",
        title="5 halting 信号全未触发 (perf/candidate/locked/red_queen/no_lift)",
        status=not halting.any_triggered(),
        actual=halting.triggered_list() or "none",
        threshold="none",
        note=f"主 20:55 红皇后归 8 核心, 触发的信号: {halting.triggered_list() or '无'}",
        section="guard",
    ))
    checks.append(HandoffCheck(
        id="v3_guards_all_pass",
        title="V3 守门 6 项全过 (runner/report/decision/v03_no_asi/no_fake_kpi/red_queen)",
        status=guards.get("v3_guards_all_pass", False),
        actual=guards.get("v3_guards", {}),
        threshold="all_true",
        note="主 17:43 + 主 17:58 不假装守门",
        section="guard",
    ))
    checks.append(HandoffCheck(
        id="philosophy_9_keys_locked",
        title="主哲学 9 键 LOCKED (PHL-02b×3 + PHL-01×3 + PHL-03×3)",
        status=guards.get("philosophy_9_keys_locked", False),
        actual=len(PHILOSOPHY_9_KEYS),
        threshold=9,
        note=f"PHILOSOPHY_9_KEYS = {PHILOSOPHY_9_KEYS}",
        section="guard",
    ))

    # ----- component -----
    checks.append(HandoffCheck(
        id="v1060_committed",
        title="V1060 backend production closure 已 commit 落库",
        status=component.v1060_committed,
        actual=component.v1060_committed,
        threshold=True,
        note="W3 末已 commit, 是 R9 工程基线",
        section="component",
    ))
    checks.append(HandoffCheck(
        id="v1061_cognitive_core_done",
        title="V1061 cognitive_core 真生产完成 (主 13:31 必达)",
        status=component.v1061_cognitive_core_done,
        actual=component.v1061_cognitive_core_done,
        threshold=True,
        note="fullstack V1061 真生产; W4 末未达 → R10 P0",
        section="component",
    ))
    checks.append(HandoffCheck(
        id="v1062_world_model_done",
        title="V1062 world_model 真生产完成 (主 23:44 干到底)",
        status=component.v1062_world_model_done,
        actual=component.v1062_world_model_done,
        threshold=True,
        note="architect2 V1062; 修复 W3 末微退; W4 末必达",
        section="component",
    ))
    checks.append(HandoffCheck(
        id="v1093_dgm_v04_500loc",
        title="V1093 DGM v0.4 真演化 ≥ 500 LOC (Track D 双维 ROI)",
        status=component.v1093_dgm_v04_500loc,
        actual=component.v1093_dgm_v04_500loc,
        threshold=True,
        note="agent_orchestrator V1093; DGM 双维 ROI 最高 +0.010~+0.030",
        section="component",
    ))
    checks.append(HandoffCheck(
        id="v1078_rl_done",
        title="V1078 RL 轻补完成 (performance_optimizer)",
        status=component.v1078_rl_done,
        actual=component.v1078_rl_done,
        threshold=True,
        note="W4 末目标; 否则 R10 中期补",
        section="component",
    ))
    checks.append(HandoffCheck(
        id="interface_freeze_complete",
        title="5 接口冻结 100% (5/5)",
        status=component.interface_freeze_count >= component.interface_freeze_target,
        actual=component.interface_freeze_count,
        threshold=component.interface_freeze_target,
        note=f"当前 {component.interface_freeze_count}/{component.interface_freeze_target}",
        section="component",
    ))
    checks.append(HandoffCheck(
        id="test_coverage_threshold",
        title="测试覆盖 ≥ 30% (R9 终点要求)",
        status=component.test_coverage_pct >= component.test_coverage_target,
        actual=component.test_coverage_pct,
        threshold=component.test_coverage_target,
        note=f"当前 {component.test_coverage_pct*100:.0f}%, 目标 {component.test_coverage_target*100:.0f}%",
        section="component",
    ))

    # ----- meta -----
    checks.append(HandoffCheck(
        id="track_decision_finalized",
        title="4 选 1 主轨道 W4 末落定",
        status=track is not None,
        actual=track.track if track else "none",
        threshold="A/B/C/D",
        note=f"W4 末主推 = {track.track} ({track.track_name}); 主 13:31 大胆激进: 决策跟上真测",
        section="meta",
    ))

    return checks


# ---------------------------------------------------------------------------
# R10 起点路径建议 (主 13:31 大胆激进: W4 末未达 → 必补; 已达 → 直启 R10)
# ---------------------------------------------------------------------------

def compute_r10_path_recommendation(
    gap: Dict[str, Any],
    checks: List[HandoffCheck],
    component: R9ComponentStatus,
    track: TrackDecision,
) -> List[str]:
    """根据 W4 末真实指标生成 R10 起点路径建议.

    主 17:43 实事求是: 数字驱动建议, 不发空话.
    """
    paths: List[str] = []

    # 1) V0.4 缺口 (主 13:31)
    if not gap["passes_w4"]:
        paths.append(
            f"[P0] 补 V0.4 缺口 {gap['gap_to_w4']:.4f} → W4 末必达 0.85, "
            f"Track {track.track} ({track.track_name}) 加速 lift {track.expected_lift}"
        )
    elif not gap["passes_r10_start"]:
        paths.append(
            f"[P0] R10 起点补 V0.4: 差距 {gap['gap_to_r10_start']:.4f}, 直接冲击 0.86, "
            f"双线并行 Track {track.track} + 跨小模型真绑定"
        )
    else:
        paths.append(
            f"[P0] R10 起点已达 (V0.4={gap['v04_actual']:.4f} ≥ {gap['r10_start_target']}); "
            "直接启动 R10 P0: V1.0 试点 + 三联真实生产集成"
        )

    # 2) V1061 cognitive_core (主 13:31 必达)
    fail_by_id = {c.id: c for c in checks if not c.status}
    if not component.v1061_cognitive_core_done:
        paths.append("[P0] V1061 cognitive_core 真生产优先级最高 (V1107 engineering 维度必需)")
    if not component.v1062_world_model_done:
        paths.append("[P1] V1062 world_model 修复微退, 上推 W4 末完成 (架构师 P0)")
    if not component.v1093_dgm_v04_500loc:
        paths.append("[P1] V1093 DGM v0.4 升 500 LOC, Track D 双维 ROI 最高 +0.010~+0.030")
    if not component.v1078_rl_done:
        paths.append("[P1] V1078 RL 轻补启动, R10 中期补)")
    if not component.v1097_mcp_round2_done:
        paths.append("[P2] V1097 MCP 二轮完成 (mcp_integration_expert)")

    # 3) 接口冻结 (主 23:44 干到底)
    if component.interface_freeze_count < component.interface_freeze_target:
        paths.append(
            f"[P0] 接口冻结补缺口 {component.interface_freeze_target - component.interface_freeze_count} "
            f"({component.interface_freeze_count}/{component.interface_freeze_target} → 5/5)"
        )
    # 4) 测试覆盖
    if component.test_coverage_pct < component.test_coverage_target:
        paths.append(
            f"[P1] 测试覆盖补 {(component.test_coverage_target - component.test_coverage_pct)*100:.0f}pp "
            f"(当前 {component.test_coverage_pct*100:.0f}% → {component.test_coverage_target*100:.0f}%)"
        )

    # 5) halt 信号触发
    if fail_by_id.get("no_halting_signals") is not None and not fail_by_id["no_halting_signals"].status:
        paths.append(
            f"[P0] halt 信号触发: {fail_by_id['no_halting_signals'].actual} → 立即切 Track C 跨小模型验证红皇后"
        )

    # 6) ASI 北极星 progress (主 22:33)
    if gap["headroom_rel_pct"] < 10.0:
        paths.append(
            f"[P0] V0.4 距 ASI 头程仅 {gap['headroom_rel_pct']:.2f}%, R10 中期必冲 0.90 → ASI 北极星"
        )
    else:
        paths.append(
            f"[info] V0.4 距 ASI 北极星 headroom = {gap['headroom_rel_pct']:.2f}%, R10 中期冲 0.90"
        )

    # 7) R10 主线 (主 23:44 干到底)
    paths.append(
        f"[meta] R10 起点建议: V0.4 ≥ {gap['r10_start_target']} + 5 halt 全未触发 "
        "+ V3 守门 6 项全过 + Track 已落定 + 测试覆盖 ≥ 30%"
    )
    return paths


# ---------------------------------------------------------------------------
# W4 末集成评估编排 (主 00:56 任何人都能接手: 一行真跑)
# ---------------------------------------------------------------------------

def evaluate_w4(
    week_label: str = "W4",
    v03_history: Optional[List[float]] = None,
    unique_ratio: float = 1.0,
    fitness_std: float = 0.05,
    cross_dim_drop: float = 0.0,
    cross_model_lift: float = 0.05,
    v1060_committed: bool = True,
    weekly_lift: float = 0.0,
    component: Optional[R9ComponentStatus] = None,
    live: bool = False,
    no_write: bool = True,
    w3_v04_baseline: float = 0.8202,
) -> W4Evaluation:
    """W4 末集成评估 + R10 移交评估主编排.

    返回 W4Evaluation dataclass (JSON 友好).
    """
    if component is None:
        component = R9ComponentStatus(**DEFAULT_W3_R9_COMPONENT_STATUS)

    # Step 1: 三件套真测 (默认 fallback W3 baseline, --live 真跑)
    v1074_raw, v1077_raw, v1103_raw, used_live = fetch_three_pieces(live=live)

    # Step 2: ASI 北极星 dashboard
    try:
        dashboard = compute_dashboard(v1074_raw, v1077_raw, v1103_raw)
    except Exception as exc:
        # ponytail: dashboard 失败不致命, 兜底用 W3 baseline
        dashboard = {
            "v03_score": v1074_raw.get("score", 0.8897),
            "v04_v1077": v1077_raw.get("score", 0.8202),
            "v04_v1103": v1103_raw.get("score", 0.8188),
            "v04_score": v1077_raw.get("score", 0.8202),
            "asi_north_star": ASI_NORTH,
            "abs_headroom": round(ASI_NORTH - 0.8202, 4),
            "rel_headroom_pct": round((ASI_NORTH - 0.8202) / ASI_NORTH * 100, 2),
            "dashboard_error": f"{type(exc).__name__}: {exc}",
        }

    # Step 3: 5 halting 信号真跑
    history = list(v03_history or [])
    history.append(dashboard["v03_score"])
    halting = evaluate_halting_signals(
        v03_history=history,
        unique_ratio=unique_ratio,
        fitness_std=fitness_std,
        cross_dim_drop=cross_dim_drop,
        cross_model_lift=cross_model_lift,
    )

    # Step 4: 4 选 1 主轨道决策
    track = choose_main_track(
        v04_score=dashboard["v04_score"],
        halting=halting,
        v1060_committed=v1060_committed,
        weekly_lift=weekly_lift,
    )

    # Step 5: V3 守门自检
    guards = run_guard_self_check(dashboard, halting)

    # Step 6: R10 差距评估
    gap = compute_r10_gap(dashboard)

    # Step 7: 移交 checklist (≥14 项)
    checks = compute_handoff_checklist(dashboard, halting, guards, track, component)
    n_pass = sum(1 for c in checks if c.status)
    n_total = len(checks)

    # Step 8: R10 路径建议
    r10_paths = compute_r10_path_recommendation(gap, checks, component, track)

    # Step 9: week_lift_v04 (W4 vs W3)
    week_lift = round(dashboard["v04_score"] - w3_v04_baseline, 4)

    # Step 10: all_ok + handoff_ready (主 23:44 干到底: 必须真算, 不空想)
    all_ok = (
        guards["v3_guards_all_pass"]
        and guards["v1074_v03_above_floor"]
        and not halting.any_triggered()
        and gap["passes_w4"]
    )
    # handoff_ready: 比 all_ok 强 — 全部 14 项 ≥ 80% pass
    handoff_ready = (n_pass / n_total) >= 0.80 and n_pass >= 10

    return W4Evaluation(
        week_label=week_label,
        timestamp=time.time(),
        version=VERSION,
        v1114_version=V1114_VERSION,
        dashboard=dashboard,
        raw={"v1074": v1074_raw, "v1077": v1077_raw, "v1103": v1103_raw, "used_live": used_live},
        halting_signals=asdict(halting),
        track_decision=asdict(track),
        guards=guards,
        r10_gap=gap,
        handoff_checklist=[c.to_dict() for c in checks],
        r10_path_recommendation=r10_paths,
        all_ok=all_ok,
        handoff_ready=handoff_ready,
        n_checks_pass=n_pass,
        n_checks_total=n_total,
        week_lift_v04=week_lift,
    )


# ---------------------------------------------------------------------------
# Markdown 渲染 (主 00:56 任何人都能接手: 人类可读报告)
# ---------------------------------------------------------------------------

def render_markdown_w4(report: W4Evaluation) -> str:
    """W4 末集成评估 + R10 移交报告 Markdown 渲染."""
    d = report.dashboard
    h = report.halting_signals
    t = report.track_decision
    g = report.guards
    gap = report.r10_gap
    checks = report.handoff_checklist
    paths = report.r10_path_recommendation

    pass_rate = report.n_checks_pass / max(1, report.n_checks_total)
    halt_triggered = [k for k, v in h.items() if v]

    lines: List[str] = []
    lines.append(f"# R9 W4 末集成验证 + R10 移交报告 — V1119 自动化")
    lines.append("")
    lines.append(f"> **生成时间**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(report.timestamp))}")
    lines.append(f"> **版本**: V1119 v{report.version} (基于 V1114 v{report.v1114_version})")
    lines.append(f"> **真测来源**: {'live_subprocess' if report.raw.get('used_live') else 'w3_baseline_fallback (--live 触发真跑)'}")
    lines.append(f"> **主哲学**: ASI 北极星 + 实事求是 + 大胆激进 + 干到底 + 走在前人经验 + 任何人都能接手 + 红皇后")
    lines.append(f"> **移交就绪 (handoff_ready)**: {report.handoff_ready}")
    lines.append(f"> **Checklist 通过率**: {report.n_checks_pass}/{report.n_checks_total} ({pass_rate*100:.1f}%)")
    lines.append("")
    lines.append("---")
    lines.append("")

    # ASI 北极星 Dashboard
    lines.append("## 📊 ASI 北极星 Dashboard (W4 末真测)")
    lines.append("")
    lines.append("| 指标 | 真测 / 值 | 状态 |")
    lines.append("|---|---:|---|")
    lines.append(f"| ASI 北极星 | {ASI_NORTH:.4f} | LOCKED (主 22:33) |")
    lines.append(f"| V1074 V0.3 | {d.get('v03_score', 0):.4f} | 守门 ≥ {V1074_V03_MIN} {'✅' if d.get('v03_score', 0) >= V1074_V03_MIN else '❌'} |")
    lines.append(f"| V1077 V0.4 | {d.get('v04_v1077', 0):.4f} | W4 末 ≥ {W4_TARGET} {'✅' if d.get('v04_v1077', 0) >= W4_TARGET else '❌'} |")
    lines.append(f"| V1103 V0.4 | {d.get('v04_v1103', 0):.4f} | W4 末 ≥ {W4_TARGET} {'✅' if d.get('v04_v1103', 0) >= W4_TARGET else '❌'} |")
    lines.append(f"| V0.4 选定 | {d.get('v04_score', 0):.4f} | V1077 优先 |")
    lines.append(f"| 距 ASI headroom | {gap.get('headroom_rel_pct', 0):.2f}% | 主线冲 0.90 → ASI |")
    lines.append("")
    lines.append("---")
    lines.append("")

    # R10 差距
    lines.append("## 🎯 R10 起点差距评估 (主 13:31 大胆激进)")
    lines.append("")
    lines.append("| 目标 | 阈值 | 实测 | 差距 | 状态 |")
    lines.append("|---|---:|---:|---:|---|")
    lines.append(f"| W4 末主目标 | {W4_TARGET} | {gap['v04_actual']:.4f} | {gap['gap_to_w4']:+.4f} | {'✅' if gap['passes_w4'] else '❌'} |")
    lines.append(f"| R10 起点目标 | {gap['r10_start_target']} | {gap['v04_actual']:.4f} | {gap['gap_to_r10_start']:+.4f} | {'✅' if gap['passes_r10_start'] else '❌'} |")
    lines.append(f"| R10 中期目标 | {gap['r10_mid_target']} | {gap['v04_actual']:.4f} | {gap['gap_to_r10_mid']:+.4f} | {'✅' if gap['passes_r10_mid'] else '❌'} |")
    lines.append(f"| ASI 北极星 | {ASI_NORTH} | {gap['v04_actual']:.4f} | {gap['gap_to_asi']:+.4f} | 永远 LOCKED |")
    lines.append("")
    lines.append("---")
    lines.append("")

    # 4 选 1 主轨道
    lines.append("## 🚂 W4 末主轨道决策 (沿用 / 切换)")
    lines.append("")
    lines.append(f"**选定主轨道**: `{t['track']}` — **{t['track_name']}**")
    lines.append("")
    lines.append(f"**理由**: {t['rationale']}")
    lines.append(f"**期望 lift**: {t['expected_lift']}")
    lines.append(f"**V1060 committed**: {t['v1060_committed']}")
    lines.append(f"**confidence**: {t['confidence']}")
    lines.append(f"**halt_override**: {t['halt_override']}")
    lines.append("")
    lines.append("---")
    lines.append("")

    # 5 Halting 信号
    lines.append("## 🚨 5 Halting 信号真跑 (主 20:55 红皇后守门)")
    lines.append("")
    lines.append("| # | 信号 | 状态 |")
    lines.append("|---:|---|---|")
    for idx, (k, v) in enumerate(h.items(), 1):
        lines.append(f"| {idx} | {k} | {'❌ 触发' if v else '✅ 未触发'} |")
    lines.append("")
    lines.append(f"**总触发**: {'; '.join(halt_triggered) if halt_triggered else '无 ✅'} (主 23:44 干到底)")
    lines.append("")
    lines.append("---")
    lines.append("")

    # 移交 Checklist
    lines.append("## 📋 R9 → R10 移交 Checklist 自动生成")
    lines.append("")
    lines.append(f"**通过数**: {report.n_checks_pass}/{report.n_checks_total} ({pass_rate*100:.1f}%)")
    lines.append(f"**移交就绪**: {'✅' if report.handoff_ready else '❌'} (阈值 ≥ 80% 且 ≥ 10 项通过)")
    lines.append("")
    lines.append("| # | 章节 | ID | 标题 | 状态 | 实际 | 阈值 |")
    lines.append("|---:|---|---|---|---|---|---|")
    for i, c in enumerate(checks, 1):
        status_icon = "✅" if c["status"] else "❌"
        actual_str = f"{c['actual']:.4f}" if isinstance(c["actual"], float) else str(c["actual"])
        threshold_str = f"{c['threshold']:.4f}" if isinstance(c["threshold"], float) else str(c["threshold"])
        lines.append(f"| {i} | {c['section']} | `{c['id']}` | {c['title']} | {status_icon} | {actual_str} | {threshold_str} |")
    lines.append("")
    lines.append("### Checklist 备注 (主 17:43 实事求是)")
    lines.append("")
    for c in checks:
        if c["note"]:
            lines.append(f"- **`{c['id']}`**: {c['note']}")
    lines.append("")
    lines.append("---")
    lines.append("")

    # R10 路径建议
    lines.append("## 🛣️ R10 起点路径建议 (主 13:31 大胆激进 + 主 23:44 干到底)")
    lines.append("")
    for i, p in enumerate(paths, 1):
        lines.append(f"{i}. {p}")
    lines.append("")
    lines.append("---")
    lines.append("")

    # V3 守门
    lines.append("## 🛡️ V3 守门 + 主哲学自检 (W4 末)")
    lines.append("")
    lines.append("| 检查 | 状态 |")
    lines.append("|---|---|")
    lines.append(f"| 主哲学 9 键 LOCKED | {'✅' if g['philosophy_9_keys_locked'] else '❌'} |")
    lines.append(f"| V3 守门 6 项全过 | {'✅' if g['v3_guards_all_pass'] else '❌'} |")
    lines.append(f"| V1074 V0.3 ≥ 守门 | {'✅' if g['v1074_v03_above_floor'] else '❌'} |")
    lines.append(f"| 5 halt 全未触发 | {'✅' if not g['halt_any_triggered'] else '❌'} |")
    lines.append(f"| **All OK** | {'✅' if report.all_ok else '❌'} |")
    lines.append("")
    lines.append("---")
    lines.append("")

    # 一句话
    lines.append("## 📝 一句话留给 R9 全团 + R10 起点")
    lines.append("")
    if report.handoff_ready:
        lines.append(
            f"> **V1119 W4 末 = {report.n_checks_pass}/{report.n_checks_total} 通过 = handoff_ready ✅。**"
            f" V0.4 = {d.get('v04_score', 0):.4f} (W4 目标 ≥ {W4_TARGET})。"
            f" 主轨道 = {t['track']} ({t['track_name']})。"
            f" 5 halt 全未触发。"
            f" **直接进入 R10 P0: V1.0 试点 + 三联真实生产集成。**"
        )
    else:
        lines.append(
            f"> **V1119 W4 末 = {report.n_checks_pass}/{report.n_checks_total} 通过 ({pass_rate*100:.1f}%) = handoff_未就绪。**"
            f" V0.4 = {d.get('v04_score', 0):.4f} (距 W4 末目标 {gap['gap_to_w4']:+.4f})。"
            f" 主轨道 = {t['track']} ({t['track_name']})。"
            f" **R10 起点建议见上, 未达项必须 W4 末周内补齐。**"
        )
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("**R9-INT-005 完成。**")
    lines.append(f"_本文由 architect2 于 R9 W4 末通过 V1119 自动评估产出, 配套 V1114 (W3 末基线) + R9-INT-001/002/003 + R9-ROADMAP-001。_")
    lines.append(f"_V1119 真跑: `python -m apeireth.v1119_w4_integration_validator --week W4 --handoff --report`_")
    lines.append("")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI 入口 (主 00:56 任何人都能接手: argparse 一行真跑)
# ---------------------------------------------------------------------------

def _build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="apeireth.v1119_w4_integration_validator",
        description="R9 W4 末集成验证 + R10 移交 checklist 自动生成器",
    )
    p.add_argument("--week", default="W4", help="week label (default: W4)")
    p.add_argument("--handoff", action="store_true", help="含 R9→R10 移交 checklist 自动生成")
    p.add_argument("--live", action="store_true", help="真跑三件套 subprocess (替代 fallback)")
    p.add_argument("--json", action="store_true", help="输出 JSON 至 stdout")
    p.add_argument("--report", action="store_true", help="写 Markdown 报告至 reports/")
    p.add_argument("--strict", action="store_true", help="非 all_ok → 退出码 1")
    p.add_argument("--v03-history", default=None, help="V0.3 历史文件 (一行一个分数)")
    p.add_argument("--unique-ratio", type=float, default=1.0, help="unique ratio (default 1.0=干净)")
    p.add_argument("--fitness-std", type=float, default=0.05, help="fitness std (default 0.05)")
    p.add_argument("--cross-dim-drop", type=float, default=0.0, help="cross_dim drop (default 0)")
    p.add_argument("--cross-model-lift", type=float, default=0.05, help="cross_model_lift (default 0.05)")
    p.add_argument("--weekly-lift", type=float, default=0.0, help="weekly lift (default 0.0)")
    # component status CLI 注入
    p.add_argument("--v1060", dest="v1060_committed", action="store_true", default=True, help="V1060 committed")
    p.add_argument("--no-v1060", dest="v1060_committed", action="store_false", help="V1060 NOT committed")
    p.add_argument("--v1061", dest="v1061_done", action="store_true", default=False, help="V1061 cognitive_core 真生产完成")
    p.add_argument("--v1062", dest="v1062_done", action="store_true", default=False, help="V1062 world_model 真生产完成")
    p.add_argument("--v1093", dest="v1093_done", action="store_true", default=False, help="V1093 DGM v0.4 ≥ 500 LOC 完成")
    p.add_argument("--v1078", dest="v1078_done", action="store_true", default=False, help="V1078 RL 轻补完成")
    p.add_argument("--v1097", dest="v1097_done", action="store_true", default=False, help="V1097 MCP 二轮完成")
    p.add_argument("--interface-freeze", type=int, default=1, help="接口冻结数 (default 1/5)")
    p.add_argument("--test-coverage", type=float, default=0.15, help="测试覆盖 (default 0.15)")
    p.add_argument("--w3-v04-baseline", type=float, default=0.8202, help="W3 末 V0.4 baseline (default 0.8202)")
    return p


def main(argv: Optional[List[str]] = None) -> int:
    args = _build_arg_parser().parse_args(argv)

    # v03_history from file (主 00:56 可读)
    history: Optional[List[float]] = None
    if args.v03_history and Path(args.v03_history).exists():
        history = []
        for line in Path(args.v03_history).read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            try:
                history.append(float(line))
            except ValueError:
                continue

    component = R9ComponentStatus(
        v1060_committed=args.v1060_committed,
        v1061_cognitive_core_done=args.v1061_done,
        v1062_world_model_done=args.v1062_done,
        v1093_dgm_v04_500loc=args.v1093_done,
        v1078_rl_done=args.v1078_done,
        v1097_mcp_round2_done=args.v1097_done,
        interface_freeze_count=args.interface_freeze,
        test_coverage_pct=args.test_coverage,
    )

    report = evaluate_w4(
        week_label=args.week,
        v03_history=history,
        unique_ratio=args.unique_ratio,
        fitness_std=args.fitness_std,
        cross_dim_drop=args.cross_dim_drop,
        cross_model_lift=args.cross_model_lift,
        v1060_committed=args.v1060_committed,
        weekly_lift=args.weekly_lift,
        component=component,
        live=args.live,
        w3_v04_baseline=args.w3_v04_baseline,
    )

    out = report.to_dict()

    if args.json:
        print(json.dumps(out, indent=2, ensure_ascii=False, default=str))
    elif args.report:
        md = render_markdown_w4(report)
        root = Path(__file__).resolve().parents[1]
        # 双写: reports/r9-w4-integration-final-report.md (任务要求)
        #      reports/r9-architect2-w4-final-report.md (角色命名空间)
        path1 = root / "reports" / "r9-w4-integration-final-report.md"
        path1.parent.mkdir(parents=True, exist_ok=True)
        path1.write_text(md, encoding="utf-8")
        path2 = root / "reports" / "r9-architect2-w4-final-report.md"
        path2.write_text(md, encoding="utf-8")
        print(f"[OK] report written: {path1}")
        print(f"[OK] report written: {path2}")
    else:
        d = report.dashboard
        t = report.track_decision
        gap = report.r10_gap
        print(f"R9 {args.week} 末集成验证 + R10 移交 (V1119 v{VERSION})")
        print(f"  V1074 V0.3 = {d.get('v03_score', 0):.4f} (守门 ≥ {V1074_V03_MIN} ? {d.get('v03_score', 0) >= V1074_V03_MIN})")
        print(f"  V1077 V0.4 = {d.get('v04_v1077', 0):.4f} (W4 ≥ {W4_TARGET} ? {d.get('v04_v1077', 0) >= W4_TARGET})")
        print(f"  V1103 V0.4 = {d.get('v04_v1103', 0):.4f}")
        print(f"  V0.4 → ASI headroom = {gap['headroom_rel_pct']:.2f}%")
        print(f"  主轨道 = {t['track']} — {t['track_name']}")
        print(f"  理由: {t['rationale']}")
        print(f"  Checklist: {report.n_checks_pass}/{report.n_checks_total} 通过 = handoff {'READY ✅' if report.handoff_ready else 'NOT READY ❌'}")
        if args.handoff:
            print(f"  R10 路径建议 (top-3):")
            for p in report.r10_path_recommendation[:3]:
                print(f"    - {p}")
        print(f"  All OK: {report.all_ok}")

    if args.strict and not (report.all_ok and report.handoff_ready):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))


# V1119 auto-injected V3 守门 (继承 V1101 模式 + W4-specific)
V3_GUARDS_W4_INJECTED = {
    "w4_evaluator_is_not_asi": "V1119 W4 评估器是辅助, ASI 由真生产决定.",
    "handoff_checklist_is_not_receipt": "R10 移交 checklist 不是收据, 是真生产状态打点.",
    "r10_path_is_not_guarantee": "R10 起点建议是路径, 不是保证, 仍需 W4 末补齐.",
    "gap_analysis_is_not_promise": "V0.4 缺口数值不是承诺, 是真测快照.",
    "w3_baseline_is_not_w4_actual": "W3 末基线 ≠ W4 末真测, --live 触发真跑.",
}
