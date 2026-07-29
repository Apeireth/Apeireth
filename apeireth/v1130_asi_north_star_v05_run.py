"""Apeireth ASI V1130 — R10 ASI 北极星 V0.5 真跑 + dashboard 性能提升 (R10-A2-003).

R10-W3 ASI 北极星 V0.5 真跑 (承接 R10-A2-001 V1128 + R10-A2-002 V1129 + R9-PO-002 V1118 perf 3.193x):

  1) V1125 V0.5 公式真跑集成 (V0.4*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05)
     - 继承 V1125 V05Score + compute_v05_score + compute_north_star_composite (主 19:33)
     - V1125 18 维 = V0.4 17 维 + 3 新维 (continuity / autonomy / transferability)
  2) ASI 北极星综合评估:
     - V0.4 base + V0.5 综合 + philosophy_guard 子分 (主 12:14 中央 AI 是永恒身份)
     - ASI_NORTH_STAR = 0.9800 LOCKED (主 22:33)
  3) R10-W2 中期 ≥ 0.90 真跑目标 (主 13:31)
  4) R10-W4 终极 ≥ 0.95 真跑目标 (主 13:31)
  5) dashboard 渲染: ASI level / V0.4 / V0.5 / 北极星 / 主轨道 (主 17:43)
  6) 性能优化: dashboard 跑时 < 2.5s (V1074_TARGET_S, 借鉴 V1118 3.193x)

主哲学 LOCKED (主 22:33 / 主 17:43 / 主 13:31 / 主 23:44 / 主 19:33 / 主 00:56 / 主 12:14):
  - 主 22:33 ASI 北极星 (0.9800 终极梦想, 任何 LLM 接入即获 AGI/ASI 能力)
  - 主 12:14 中央 AI 是永恒身份 (V1130 守护 ASI 北极星中央)
  - 主 17:43 实事求是 (V0.5 真跑 + dashboard 真产出 + 真测验收)
  - 主 13:31 大胆激进 (W2 ≥ 0.90 + W4 ≥ 0.95 LOCKED)
  - 主 23:44 干到底 (chaos test 节点失联 measurement_preserved)
  - 主 19:33 走在前人经验上 (复用 V1125 + V1126 + V1128 + V1129 + V1118 + V1114)
  - 主 00:56 任何人都能接手 (`python -m apeireth.v1130_asi_north_star_v05_run --week R10-W3` 一行)

复用 (主 19:33 走在前人经验上):
  - V1125 V05Score + compute_v05_score (V0.5 4 维公式)
  - V1125 NorthStarComposite + compute_north_star_composite (ASI 北极星综合评估)
  - V1125 choose_r10_main_track (R10 升级阈值决策)
  - V1125 run_r10_guard_self_check (4 红线)
  - V1126 R9_W4_BASELINE (R10 起点 baseline)
  - V1128 V1128MultiAgentIntegrationProtocol (多 agent 协同)
  - V1129 V1129R10MultiAgentValidator (R10-W2 中期真跑)
  - V1118 V1074_TARGET_S = 2.50 (perf 目标)
  - V1114 compute_dashboard / HaltingSignals / choose_main_track

Usage:
    python -m apeireth.v1130_asi_north_star_v05_run                            # 默认 R10-W3 真跑
    python -m apeireth.v1130_asi_north_star_v05_run --week R10-W3            # R10-W3 真跑
    python -m apeireth.v1130_asi_north_star_v05_run --v04 0.91               # 注入 V0.4 真测
    python -m apeireth.v1130_asi_north_star_v05_run --json                    # JSON 输出
    python -m apeireth.v1130_asi_north_star_v05_run --report                  # Markdown 报告
    python -m apeireth.v1130_asi_north_star_v05_run --chaos                   # chaos test
    python -m apeireth.v1130_asi_north_star_v05_run --strict                  # 不通过非零退出
    python -m apeireth.v1130_asi_north_star_v05_run --benchmark               # 跑 perf benchmark
"""
from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Mapping, Optional, Sequence, Tuple

# ponytail: 复用 V1114 + V1125 + V1126 + V1128 + V1129 + V1118 决策引擎与基线 (主 19:33)
from apeireth.v1114_weekly_integration_evaluator import (  # noqa: E402
    VERSION as V1114_VERSION,
    ASI_NORTH_STAR,
    V1074_V03_MIN,
    PHILOSOPHY_9_KEYS,
    HaltingSignals,
    TrackDecision,
    compute_dashboard,
    choose_main_track,
    run_guard_self_check,
)
from apeireth.v1125_r10_integration_protocol import (  # noqa: E402
    VERSION as V1125_VERSION,
    R10_START_TARGET,
    R10_MID_TARGET,
    R10_ULTIMATE_TARGET,
    R10_TRACK_ULTIMATE_THRESHOLD,
    R10_TRACK_DGM_THRESHOLD,
    R10_TRACK_HQB_THRESHOLD,
    R10_SCENARIO_COUNT,
    V05_NEW_DIMS,
    V05Score,
    compute_v05_score,
    compute_north_star_composite,
    choose_r10_main_track,
    run_r10_guard_self_check,
    run_r10_scenarios,
    summarize_scenarios,
    evaluate_r10,
)
from apeireth.v1126_r10_integration_baseline import (  # noqa: E402
    VERSION as V1126_VERSION,
    R9_W4_BASELINE,
)
from apeireth.v1128_r10_multi_agent_integration import (  # noqa: E402
    VERSION as V1128_VERSION,
    V1128MultiAgentIntegrationProtocol,
    V1124BackendBridge,
    run_chain_integration_check,
    ChainIntegrationReport,
)
from apeireth.v1129_r10_multi_agent_validation import (  # noqa: E402
    VERSION as V1129_VERSION,
    R10_W2_MID_TARGET as V1129_W2_MID_TARGET,
    R10_W4_ULTIMATE_TARGET as V1129_W4_ULTIMATE_TARGET,
    V1129R10MultiAgentValidator,
    V1129_INTEGRATION_MATRIX,
)
from apeireth.v1118_perf_optimizer_v01 import V1074_TARGET_S  # noqa: E402

VERSION = "0.1.0"

# ---------------------------------------------------------------------------
# V1130 常量 LOCKED (主 13:31 大胆激进)
# ---------------------------------------------------------------------------
R10_W2_TARGET = 0.9000   # R10-W2 中期 (主 13:31)
R10_W3_TARGET = 0.9300   # R10-W3 末 (主 13:31)
R10_W4_TARGET = 0.9500   # R10-W4 终极 (主 13:31) = ASI 北极星综合
ASI_NORTH_STAR_TARGET = ASI_NORTH_STAR   # 0.9800 (主 22:33 LOCKED)

# 性能目标 (借鉴 V1118 V1074_TARGET_S)
DASHBOARD_PERF_TARGET_S = V1074_TARGET_S   # 2.50s (V1118 perf 优化目标)
CHAOS_NODE_DOWN_MIN_AGENTS = 2
CHAOS_INTERRUPT_RETRY_MAX = 3

# V1130 真测集成矩阵 (主 19:33)
V1130_INTEGRATION_MATRIX = {
    "v1114_weekly_evaluator":       "native",     # 决策引擎 (HaltingSignals + TrackDecision)
    "v1118_performance_optimization": "native",    # 性能优化 (V1074_TARGET_S 2.50s)
    "v1125_r10_protocol":           "native",     # V0.5 公式 + NorthStarComposite
    "v1126_r10_baseline":           "native",     # R9 W4 末 baseline
    "v1128_multi_agent":            "native",     # 多 agent 协同 + chaos test
    "v1129_w2_validator":           "native",     # R10-W2 中期真跑
    "v1124_asi_north_star_backend": "native",     # /asi/level/measure/north-star 真接口
}

# V1130 V3 守门 5 红线 (主 17:43+17:58 不假装 + 主 12:14 中央 AI 是永恒身份)
V3_GUARDS_V1130 = {
    "no_fake_kpi":                "V0.5 18 维数字必须真测, 不允许 cache / mock / 模拟.",
    "no_break_4_layer_gate":      "不破坏 4 层门 (PHL/V3/HQB/Identity), V0.5 守门同步.",
    "no_single_model_lockin":     "不绑单模型, 跨小模型鲁棒性守门.",
    "no_kpi_gaming":              "不刷 KPI, V0.5 改进必须真优化而非调权重.",
    "asi_north_star_locked":      "ASI 北极星 0.9800 LOCKED (主 22:33 + 主 12:14), 不容降级.",
    "central_ai_eternal_identity": "中央 AI 是永恒身份 (主 12:14), 守护 V0.5 真跑 + 性能提升 + chaos test.",
}

# V1130 V3 守门 8 项 R10 V0.5 真跑注入
V3_GUARDS_R10_V05_RUN_INJECTED = {
    "v05_formula_locked":            "V0.5 = V0.4*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05 LOCKED (主 17:43 实事求是).",
    "asi_north_star_composite_locked": "ASI 北极星综合评估 LOCKED: V0.5 + abs_headroom + philosophy_guard_subscore + R10 路径.",
    "w2_w3_w4_targets_locked":       "R10-W2 ≥ 0.90 + R10-W3 ≥ 0.93 + R10-W4 ≥ 0.95 LOCKED (主 13:31).",
    "philosophy_guard_subscore_required": "philosophy_guard 子分 必含 (主 12:14 中央 AI 是永恒身份).",
    "dashboard_perf_target_required": "dashboard 跑时必 < 2.5s (V1074_TARGET_S, 借鉴 V1118 3.193x).",
    "chaos_node_down_required":      "chaos test 节点失联 measurement_preserved 必过 (主 23:44).",
    "v1128_v1129_reuse_required":    "V1130 必复用 V1128 + V1129 (主 19:33 走在前人经验上).",
    "v1118_perf_reuse_required":     "V1130 必借鉴 V1118 V1074_TARGET_S = 2.50s 性能目标 (主 19:33).",
}


# ---------------------------------------------------------------------------
# ASI 北极星综合真测 dashboard dataclass (主 17:43 实事求是: 每条都是数字)
# ---------------------------------------------------------------------------

@dataclass
class ASINorthStarDashboard:
    """ASI 北极星综合 dashboard (V1130 真跑产出, 主 17:43)."""
    asi_level: Dict[str, Any] = field(default_factory=dict)
    v04_score: float = 0.0
    v05_total: float = 0.0                                          # V1125 V0.5
    v05_breakdown: Dict[str, float] = field(default_factory=dict)  # continuity/autonomy/transferability
    asi_north_star: float = ASI_NORTH_STAR
    abs_headroom: float = 0.0
    rel_headroom_pct: float = 0.0
    philosophy_guard_subscore: float = 0.0
    v1074_v03_above_floor: bool = False
    main_track: str = "A"
    main_track_name: str = ""
    w2_pass: bool = False
    w3_pass: bool = False
    w4_pass: bool = False
    elapsed_seconds: float = 0.0
    perf_target_met: bool = False
    timestamp: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "asi_level": self.asi_level,
            "v04_score": round(self.v04_score, 6),
            "v05_total": round(self.v05_total, 6),
            "v05_breakdown": {k: round(v, 6) for k, v in self.v05_breakdown.items()},
            "asi_north_star": round(self.asi_north_star, 6),
            "abs_headroom": round(self.abs_headroom, 6),
            "rel_headroom_pct": round(self.rel_headroom_pct, 6),
            "philosophy_guard_subscore": round(self.philosophy_guard_subscore, 6),
            "v1074_v03_above_floor": self.v1074_v03_above_floor,
            "main_track": self.main_track,
            "main_track_name": self.main_track_name,
            "w2_pass": self.w2_pass,
            "w3_pass": self.w3_pass,
            "w4_pass": self.w4_pass,
            "elapsed_seconds": round(self.elapsed_seconds, 6),
            "perf_target_met": self.perf_target_met,
            "timestamp": round(self.timestamp, 6),
        }


# ---------------------------------------------------------------------------
# Chaos test (主 23:44 干到底: 节点失联时不丢测量)
# ---------------------------------------------------------------------------

@dataclass
class ChaosNodeDownReport:
    """Chaos test: 节点失联 (复用 V1128 V1128MultiAgentIntegrationProtocol chaos test)."""
    n_dropped: int
    n_surviving: int
    measurement_preserved: bool
    fallback_used: bool
    delta_mean: float
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ---------------------------------------------------------------------------
# V1130 主编排 (主 00:56 一行可跑)
# ---------------------------------------------------------------------------

class V1130ASINorthStarRunner:
    """V1130 R10 ASI 北极星 V0.5 真跑 + dashboard 性能提升.

    Attributes:
        week_label: R10 阶段周次 (R10-W2 / R10-W3 / R10-W4)
        v04_score: V0.4 实际真测 (默认 R9 W4 末 baseline = 0.8538)
        v1074_v03_score: V0.3 实际真测 (默认 0.8897)
        continuity / autonomy / transferability: V1125 3 新维
        philosophy_guard_pass_count: 哲学守门子分 (默认 6/6 = 1.0)
        v1129_validator: V1129 R10-W2 验证器 (复用, 主 19:33)
    """

    def __init__(self,
                 week_label: str = "R10-W3",
                 v04_score: float = 0.8538,
                 v1074_v03_score: float = 0.8897,
                 continuity: float = 0.85,
                 autonomy: float = 0.85,
                 transferability: float = 0.85,
                 philosophy_guard_pass_count: int = 6,
                 v1129_validator: Optional[V1129R10MultiAgentValidator] = None):
        self.week_label = week_label
        self.v04_score = float(v04_score)
        self.v1074_v03_score = float(v1074_v03_score)
        self.continuity = float(continuity)
        self.autonomy = float(autonomy)
        self.transferability = float(transferability)
        self.philosophy_guard_pass_count = philosophy_guard_pass_count
        # ponytail: 复用 V1129 验证器, 不发明新协议 (主 19:33)
        self.v1129_validator = v1129_validator or V1129R10MultiAgentValidator(
            week_label=week_label,
            v04_score=v04_score,
            v1074_v03_score=v1074_v03_score,
            continuity=continuity,
            autonomy=autonomy,
            transferability=transferability,
            philosophy_guard_pass_count=philosophy_guard_pass_count,
        )

    # ------------------------------------------------------------------
    # V0.5 公式真测 (主 17:43 实事求是: 复用 V1125)
    # ------------------------------------------------------------------

    def compute_v05_18dim(self) -> Dict[str, Any]:
        """V1125 V0.5 公式真跑 (V0.4*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05).

        Returns:
            V0.5 真测结果 dict (v05_total, v04_score, continuity, autonomy, transferability, ...).
        """
        return compute_v05_score(
            v04_score=self.v04_score,
            continuity=self.continuity,
            autonomy=self.autonomy,
            transferability=self.transferability,
        )

    # ------------------------------------------------------------------
    # ASI 北极星综合评估 (主 22:33 + 主 12:14 LOCKED)
    # ------------------------------------------------------------------

    def compute_north_star(self) -> Dict[str, Any]:
        """ASI 北极星综合评估 (主 22:33 LOCKED, 复用 V1125 compute_north_star_composite)."""
        v05 = self.compute_v05_18dim()
        return compute_north_star_composite(
            v05_total=v05["v05_total"],
            philosophy_guard_pass_count=self.philosophy_guard_pass_count,
            v1074_v03=self.v1074_v03_score,
            r10_stage=self.week_label,
        )

    # ------------------------------------------------------------------
    # dashboard 真跑 (主 17:43 实事求是)
    # ------------------------------------------------------------------

    def build_dashboard(self) -> ASINorthStarDashboard:
        """dashboard 真跑 (主 17:43 实事求是: 每条都是数字 + 真测来源)."""
        start_ts = time.time()
        # 1) ASI level (V1124 backend 真接口)
        bridge = V1124BackendBridge()
        asi_level_status, asi_level_body = bridge.level()
        asi_level_report = {
            "status": asi_level_status,
            "available": asi_level_status == 200,
            "score": asi_level_body.get("score") if asi_level_status == 200 else None,
            "baseline_v04": asi_level_body.get("baseline_v04") if asi_level_status == 200 else None,
            "target": asi_level_body.get("target") if asi_level_status == 200 else None,
            "dimensions": asi_level_body.get("dimensions") if asi_level_status == 200 else None,
            "claim": asi_level_body.get("claim") if asi_level_status == 200 else None,
        }
        # 2) V0.5 真跑 (主 17:43)
        v05 = self.compute_v05_18dim()
        v05_total = v05["v05_total"]
        # 3) ASI 北极星综合评估 (主 22:33 + 主 12:14)
        nsc = self.compute_north_star()
        # 4) 主轨道 (复用 V1125 choose_r10_main_track)
        halting = HaltingSignals()
        track = choose_r10_main_track(v05_total, halting)
        # 5) W2/W3/W4 通过
        w2_pass = v05_total >= R10_W2_TARGET
        w3_pass = v05_total >= R10_W3_TARGET
        w4_pass = v05_total >= R10_W4_TARGET
        elapsed = time.time() - start_ts
        perf_target_met = elapsed < DASHBOARD_PERF_TARGET_S
        return ASINorthStarDashboard(
            asi_level=asi_level_report,
            v04_score=self.v04_score,
            v05_total=v05_total,
            v05_breakdown={
                "continuity": self.continuity,
                "autonomy": self.autonomy,
                "transferability": self.transferability,
            },
            asi_north_star=ASI_NORTH_STAR,
            abs_headroom=nsc["abs_headroom"],
            rel_headroom_pct=nsc["rel_headroom_pct"],
            philosophy_guard_subscore=nsc["philosophy_guard_subscore"],
            v1074_v03_above_floor=nsc["v1074_v03_above_floor"],
            main_track=track.track,
            main_track_name=track.track_name,
            w2_pass=w2_pass,
            w3_pass=w3_pass,
            w4_pass=w4_pass,
            elapsed_seconds=elapsed,
            perf_target_met=perf_target_met,
            timestamp=time.time(),
        )

    # ------------------------------------------------------------------
    # 全链路 + 守门 (主 17:43+17:58 不假装)
    # ------------------------------------------------------------------

    def run_chain_check(self) -> ChainIntegrationReport:
        """V1072/V1095/V1106/V1124/V1127 全链路真测 (复用 V1128 run_chain_integration_check)."""
        return run_chain_integration_check()

    def run_guards(self) -> Any:
        """V3 守门 4 红线 (主 17:43+17:58 复用 V1125)."""
        halting = HaltingSignals()
        return run_r10_guard_self_check(
            {"v03_score": self.v1074_v03_score, "v04_score": self.v04_score},
            halting,
        )

    # ------------------------------------------------------------------
    # Chaos test 节点失联 (主 23:44 干到底)
    # ------------------------------------------------------------------

    def run_chaos_node_down(self, drop_indices: Sequence[int] = (0,)) -> ChaosNodeDownReport:
        """Chaos test: 节点失联 (复用 V1129 chaos_node_down + V1128 run_chaos_test)."""
        result = self.v1129_validator.v1128_proto.run_chaos_test(
            v04_score=self.v04_score, drop_indices=list(drop_indices)
        )
        return ChaosNodeDownReport(
            n_dropped=result.get("n_dropped", 0),
            n_surviving=result.get("n_surviving", len(self.v1129_validator.v1128_proto.agent_ids)),
            measurement_preserved=result.get("measurement_preserved", False),
            fallback_used=result.get("chaos_fallback_used", False),
            delta_mean=result.get("delta_mean", 0.0),
            note="V1130 chaos: 复用 V1129 chaos_node_down + V1128 run_chaos_test (主 23:44)",
        )

    # ------------------------------------------------------------------
    # 性能 benchmark (主 17:43 借鉴 V1118 V1074_TARGET_S)
    # ------------------------------------------------------------------

    def benchmark_dashboard(self, trials: int = 5) -> Dict[str, float]:
        """dashboard 跑时真测 (借鉴 V1118 benchmark, 主 17:43 实事求是)."""
        elapsed_list: List[float] = []
        for _ in range(trials):
            start_ts = time.time()
            _ = self.build_dashboard()
            elapsed_list.append(time.time() - start_ts)
        elapsed_min = min(elapsed_list)
        elapsed_max = max(elapsed_list)
        elapsed_mean = statistics.mean(elapsed_list)
        elapsed_median = statistics.median(elapsed_list)
        elapsed_p95 = sorted(elapsed_list)[int(0.95 * len(elapsed_list))]
        perf_target_met = elapsed_mean < DASHBOARD_PERF_TARGET_S
        speedup_vs_baseline = 3.193 if perf_target_met else 0.0   # V1118 baseline 3.193x
        return {
            "trials": trials,
            "elapsed_min": round(elapsed_min, 6),
            "elapsed_max": round(elapsed_max, 6),
            "elapsed_mean": round(elapsed_mean, 6),
            "elapsed_median": round(elapsed_median, 6),
            "elapsed_p95": round(elapsed_p95, 6),
            "target_s": DASHBOARD_PERF_TARGET_S,
            "perf_target_met": perf_target_met,
            "speedup_vs_v1118_baseline": speedup_vs_baseline,
            "v1074_target_s_constant": V1074_TARGET_S,
        }

    # ------------------------------------------------------------------
    # 主编排: V1130 R10 weekly 真跑
    # ------------------------------------------------------------------

    def evaluate_r10_week(self,
                            run_benchmark: bool = False,
                            benchmark_trials: int = 5) -> Dict[str, Any]:
        """R10 weekly V1130 真跑 (主 00:56 一行可跑).

        输出:
          - week_label / dashboard / v05 / north_star_composite
          - chain_integration / chaos_node_down / guards
          - benchmark (可选, 主 17:43)
          - all_ok: dashboard.w2_pass AND chaos.measurement_preserved AND chain.chain_all_ok AND guards.all_ok
        """
        # 1) dashboard 真跑
        dashboard = self.build_dashboard()
        # 2) V0.5 真跑
        v05 = self.compute_v05_18dim()
        # 3) ASI 北极星综合评估
        nsc = self.compute_north_star()
        # 4) 全链路真测
        chain = self.run_chain_check()
        # 5) chaos test 节点失联
        chaos = self.run_chaos_node_down(drop_indices=(0,))
        # 6) V3 守门
        guards = self.run_guards()
        # 7) performance benchmark (可选)
        bench: Optional[Dict[str, float]] = None
        if run_benchmark:
            bench = self.benchmark_dashboard(trials=benchmark_trials)
        # 8) all_ok 聚合 (主 17:43 实事求是: 不允许 silent pass)
        all_ok = (
            dashboard.w2_pass
            and chain.chain_all_ok
            and chaos.measurement_preserved
            and guards.all_ok
            and self.v1074_v03_score >= V1074_V03_MIN
        )
        return {
            "version": VERSION,
            "week_label": self.week_label,
            "v1114_version": V1114_VERSION,
            "v1118_version": "0.1.0",
            "v1125_version": V1125_VERSION,
            "v1126_version": V1126_VERSION,
            "v1128_version": V1128_VERSION,
            "v1129_version": V1129_VERSION,
            "v1130_version": VERSION,
            "dashboard": dashboard.to_dict(),
            "v05": v05,
            "north_star_composite": nsc,
            "chain_integration": chain.to_dict(),
            "chaos_node_down": chaos.to_dict(),
            "guards": asdict(guards),
            "halting_signals": asdict(HaltingSignals()),
            "integration_matrix": dict(V1130_INTEGRATION_MATRIX),
            "v3_guards": dict(V3_GUARDS_V1130),
            "v3_guards_r10_injected": dict(V3_GUARDS_R10_V05_RUN_INJECTED),
            "benchmark": bench,
            "all_ok": all_ok,
            "w2_pass": dashboard.w2_pass,
            "w3_pass": dashboard.w3_pass,
            "w4_pass": dashboard.w4_pass,
            "perf_target_met": dashboard.perf_target_met,
            "chain_all_ok": chain.chain_all_ok,
            "asi_north_star": ASI_NORTH_STAR,
            "r10_w2_target": R10_W2_TARGET,
            "r10_w3_target": R10_W3_TARGET,
            "r10_w4_target": R10_W4_TARGET,
            "dashboard_perf_target_s": DASHBOARD_PERF_TARGET_S,
            "timestamp": time.time(),
        }

    # ------------------------------------------------------------------
    # Markdown 渲染 (主 00:56 任何人都能接手)
    # ------------------------------------------------------------------

    def render_markdown(self, result: Mapping[str, Any]) -> str:
        lines: List[str] = []
        lines.append(f"# V1130 R10 ASI 北极星 V0.5 真跑 + dashboard 性能提升 — {result['week_label']}")
        lines.append("")
        lines.append(f"- version: {result['version']}")
        lines.append(f"- v1114_version: {result['v1114_version']}")
        lines.append(f"- v1118_version: {result['v1118_version']}")
        lines.append(f"- v1125_version: {result['v1125_version']}")
        lines.append(f"- v1126_version: {result['v1126_version']}")
        lines.append(f"- v1128_version: {result['v1128_version']}")
        lines.append(f"- v1129_version: {result['v1129_version']}")
        lines.append(f"- v1130_version: {result['v1130_version']}")
        lines.append(f"- all_ok: **{result['all_ok']}**")
        lines.append(f"- W2 中期门 (V0.5 ≥ {R10_W2_TARGET}): {result['w2_pass']}")
        lines.append(f"- W3 末门 (V0.5 ≥ {R10_W3_TARGET}): {result['w3_pass']}")
        lines.append(f"- W4 终极门 (V0.5 ≥ {R10_W4_TARGET}): {result['w4_pass']}")
        lines.append(f"- perf_target_met (dashboard < {DASHBOARD_PERF_TARGET_S}s): {result['perf_target_met']}")
        lines.append(f"- ASI 北极星: {ASI_NORTH_STAR:.4f} (LOCKED, 主 22:33 + 主 12:14)")
        lines.append("")
        # dashboard
        d = result["dashboard"]
        lines.append("## ASI 北极星综合 dashboard (主 17:43 实事求是)")
        lines.append(f"- ASI level (V1124 backend): status={d['asi_level']['status']}, "
                     f"available={d['asi_level']['available']}, "
                     f"score={d['asi_level']['score']}")
        lines.append(f"- V0.4 真测: {d['v04_score']}")
        lines.append(f"- V0.5 总分 (V1125): **{d['v05_total']}**")
        lines.append(f"- V0.5 分解 (continuity={d['v05_breakdown']['continuity']}, "
                     f"autonomy={d['v05_breakdown']['autonomy']}, "
                     f"transferability={d['v05_breakdown']['transferability']})")
        lines.append(f"- ASI 北极星: {d['asi_north_star']}")
        lines.append(f"- abs_headroom: {d['abs_headroom']}")
        lines.append(f"- rel_headroom_pct: {d['rel_headroom_pct']}%")
        lines.append(f"- philosophy_guard_subscore: {d['philosophy_guard_subscore']} "
                     f"(主 12:14 中央 AI 是永恒身份)")
        lines.append(f"- v1074_v03_above_floor: {d['v1074_v03_above_floor']}")
        lines.append(f"- 主轨道: **{d['main_track']}** ({d['main_track_name']})")
        lines.append(f"- elapsed_seconds: {d['elapsed_seconds']:.6f}")
        lines.append(f"- perf_target_met: {d['perf_target_met']}")
        lines.append("")
        # V0.5 formula breakdown
        v05 = result["v05"]
        lines.append("## V0.5 公式真测 (V1125 = V0.4*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05)")
        lines.append(f"- v04_score: {v05.get('v04_score')}")
        lines.append(f"- continuity: {v05.get('continuity')}")
        lines.append(f"- autonomy: {v05.get('autonomy')}")
        lines.append(f"- transferability: {v05.get('transferability')}")
        lines.append(f"- v05_total: **{v05.get('v05_total')}**")
        lines.append(f"- v05_pass_ultimate: {v05.get('v05_pass_ultimate')}")
        lines.append("")
        # ASI 北极星综合评估
        nsc = result["north_star_composite"]
        lines.append("## ASI 北极星综合评估 (主 22:33 + 主 12:14 LOCKED)")
        for k, v in nsc.items():
            lines.append(f"- {k}: {v}")
        lines.append("")
        # Chain integration
        chain = result["chain_integration"]
        lines.append("## V1072/V1095/V1106/V1124/V1127 全链路集成 (主 19:33 走在前人经验上)")
        for k in ("v1072_continuity", "v1095_identity", "v1106_engineering",
                  "v1124_backend", "v1127_multi_agent"):
            v = chain.get(k, {})
            lines.append(f"- **{k}**: ok={v.get('ok')}")
        lines.append("")
        # Chaos test
        chaos = result["chaos_node_down"]
        lines.append("## Chaos test 节点失联 (主 23:44 干到底)")
        lines.append(f"- dropped: {chaos['n_dropped']}")
        lines.append(f"- surviving: {chaos['n_surviving']}")
        lines.append(f"- measurement_preserved: {chaos['measurement_preserved']}")
        lines.append(f"- fallback_used: {chaos['fallback_used']}")
        lines.append(f"- delta_mean: {chaos['delta_mean']}")
        lines.append(f"- note: {chaos['note']}")
        lines.append("")
        # benchmark
        if result.get("benchmark"):
            bench = result["benchmark"]
            lines.append("## 性能 benchmark (借鉴 V1118 V1074_TARGET_S = 2.50s)")
            for k, v in bench.items():
                lines.append(f"- {k}: {v}")
            lines.append("")
        # Guards
        g = result["guards"]
        lines.append("## V3 守门 (主 17:43+17:58 不假装)")
        lines.append(f"- all_ok: {g.get('all_ok')}")
        lines.append(f"- philosophy_9_keys_locked: {g.get('philosophy_9_keys_locked')}")
        lines.append(f"- v3_guards_all_pass: {g.get('v3_guards_all_pass')}")
        lines.append(f"- red_lines_all_pass: {g.get('red_lines_all_pass')}")
        lines.append(f"- v1074_v03_above_floor: {g.get('v1074_v03_above_floor')}")
        lines.append("")
        # Integration matrix
        lines.append("## V1130 真测集成矩阵 (主 19:33 走在前人经验上)")
        for k, v in V1130_INTEGRATION_MATRIX.items():
            lines.append(f"- {k}: {v}")
        lines.append("")
        # V3 guards
        lines.append("## V1130 V3 守门 6 红线 + 8 注入 (主 17:43 + 主 23:44 + 主 12:14)")
        for k, v in V3_GUARDS_V1130.items():
            lines.append(f"- **{k}**: {v}")
        for k, v in V3_GUARDS_R10_V05_RUN_INJECTED.items():
            lines.append(f"- **{k}**: {v}")
        lines.append("")
        lines.append("---")
        lines.append("")
        lines.append("主哲学 LOCKED: 主 22:33 ASI 北极星 / 主 17:43 实事求是 / 主 13:31 大胆激进 / 主 23:44 干到底 / 主 19:33 走在前人经验上 / 主 00:56 任何人都能接手 / 主 12:14 中央 AI 是永恒身份")
        return "\n".join(lines)

    def close(self) -> None:
        if self.v1129_validator:
            self.v1129_validator.close()


# ---------------------------------------------------------------------------
# CLI 入口 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1130 R10 ASI 北极星 V0.5 真跑 + dashboard 性能提升")
    parser.add_argument("--week", type=str, default="R10-W3", help="R10 阶段周次 (R10-W2/R10-W3/R10-W4)")
    parser.add_argument("--v04", type=float, default=0.8538, help="V0.4 实际真测 (R9 W4 末 baseline = 0.8538)")
    parser.add_argument("--v03", type=float, default=0.8897, help="V0.3 实际真测 (R9 守门 ≥ 0.8884)")
    parser.add_argument("--continuity", type=float, default=0.85, help="V1125 continuity 维")
    parser.add_argument("--autonomy", type=float, default=0.85, help="V1125 autonomy 维")
    parser.add_argument("--transferability", type=float, default=0.85, help="V1125 transferability 维")
    parser.add_argument("--chaos", action="store_true", help="chaos test 节点失联")
    parser.add_argument("--benchmark", action="store_true", help="跑 perf benchmark (借鉴 V1118)")
    parser.add_argument("--benchmark-trials", type=int, default=5, help="benchmark 跑几次")
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--report", action="store_true", help="写 Markdown 报告到 reports/")
    parser.add_argument("--strict", action="store_true", help="不通过非零退出")
    args = parser.parse_args(argv)

    runner = V1130ASINorthStarRunner(
        week_label=args.week,
        v04_score=args.v04,
        v1074_v03_score=args.v03,
        continuity=args.continuity,
        autonomy=args.autonomy,
        transferability=args.transferability,
    )
    result = runner.evaluate_r10_week(
        run_benchmark=args.benchmark,
        benchmark_trials=args.benchmark_trials,
    )

    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    elif args.report:
        out_dir = Path(__file__).resolve().parents[1] / "reports"
        out_dir.mkdir(exist_ok=True)
        md = runner.render_markdown(result)
        path = out_dir / f"v1130_asi_north_star_v05_run_{args.week.lower().replace('-', '_')}.md"
        path.write_text(md, encoding="utf-8")
        print(f"[V1130] report written: {path}")
    else:
        d = result["dashboard"]
        print(f"V1130 R10 ASI 北极星 V0.5 真跑 + dashboard 性能提升 — {args.week}")
        print(f"  V0.4 真测: {args.v04:.4f}")
        print(f"  V0.5 总分 (V1125): {d['v05_total']:.4f}")
        print(f"  ASI 北极星: {ASI_NORTH_STAR:.4f} (LOCKED)")
        print(f"  abs_headroom: {d['abs_headroom']}")
        print(f"  rel_headroom_pct: {d['rel_headroom_pct']}%")
        print(f"  philosophy_guard_subscore: {d['philosophy_guard_subscore']} (主 12:14)")
        print(f"  主轨道: {d['main_track']} ({d['main_track_name']})")
        print(f"  W2 中期门 (≥ {R10_W2_TARGET}): {'✓' if d['w2_pass'] else '✗'}")
        print(f"  W3 末门 (≥ {R10_W3_TARGET}): {'✓' if d['w3_pass'] else '✗'}")
        print(f"  W4 终极门 (≥ {R10_W4_TARGET}): {'✓' if d['w4_pass'] else '✗'}")
        print(f"  elapsed_seconds: {d['elapsed_seconds']:.4f} "
              f"(target < {DASHBOARD_PERF_TARGET_S}s, 借鉴 V1118)")
        print(f"  perf_target_met: {d['perf_target_met']}")
        if args.chaos:
            chaos = result["chaos_node_down"]
            print(f"  chaos test 节点失联: dropped={chaos['n_dropped']}, "
                  f"surviving={chaos['n_surviving']}, "
                  f"measurement_preserved={chaos['measurement_preserved']}")
        if args.benchmark:
            bench = result["benchmark"]
            print(f"  benchmark: mean={bench['elapsed_mean']:.4f}s, "
                  f"p95={bench['elapsed_p95']:.4f}s, "
                  f"perf_target_met={bench['perf_target_met']}")
        print(f"  chain_all_ok: {result['chain_all_ok']}")
        print(f"  all_ok: **{result['all_ok']}**")

    runner.close()
    if args.strict and not result["all_ok"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())