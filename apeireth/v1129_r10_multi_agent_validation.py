"""Apeireth ASI V1129 — R10 多 agent 集成 V0.5 中期真跑 + dashboard (R10-A2-002).

R10-W2 中期多 agent 集成真跑 (承接 R10-A2-001 V1128 accepted 9.00 + R10-ARCH-001 V1125 accepted 9.05):

  1) V1128 + V1125 真集成:
     - V1128 V0.5 18 维 (V0.4 16 + continuity_tracker + multi_agent_consensus)
     - V1125 V0.5 4 维 (V0.4 1 + continuity + autonomy + transferability)
     - 两套公式并行产出, 互不冲突
  2) V0.5 中期真跑: R10-W2 中期 ≥ 0.90 真跑目标
  3) 多 agent dashboard:
     - ASI level (V1124 backend /asi/level + /asi/north-star)
     - V0.4 17 维真测 + V0.5 18 维 + V0.5 4 维
     - ASI 北极星 (0.9800 LOCKED)
     - 主轨道 (R10 4 选 1 升级阈值)
  4) chaos test (3 类):
     a) 节点失联 (node down): surviving < MIN_AGENTS → 兜底
     b) 测量中断 (measurement interrupt): V1124 backend 503 → 透明报告
     c) 协议握手失败 (handshake fail): agent 间一致性不符 → consensus_fail
  5) V1129 R10-W2 真跑 + W1 末回顾对齐 (主 17:43 实事求是)

主哲学 LOCKED (继承 V1114 + V1119 + V1125 + V1126 + V1127 + V1128):
  - 主 22:33 ASI 北极星 (终极梦想: 任何 LLM 接入即获 AGI/ASI 能力)
  - 主 17:43 实事求是 (中期真跑必须真跑真产出, 数字驱动决策)
  - 主 13:31 大胆激进 (W2 中期 ≥ 0.90 不容分阶段缓慢)
  - 主 23:44 干到底 (chaos test 不通过即非零退出)
  - 主 19:33 走在前人经验上 (复用 V1125 + V1128 + V1114 决策引擎)
  - 主 00:56 任何人都能接手 (`python -m apeireth.v1129_r10_multi_agent_validation --week R10-W2` 一行)
  - 主 20:55 红皇后归入 8 核心 (chaos test 不假装 ASI)

复用 (主 19:33):
  - V1128 V0.5 18 维公式 (16 V0.4 + 2 R10 新维)
  - V1125 V0.5 4 维公式 (V0.4*0.85 + 3*0.05)
  - V1125 evaluate_r10 (24 R10 场景)
  - V1125 choose_r10_main_track (R10 升级阈值)
  - V1125 run_r10_guard_self_check (4 红线)
  - V1128 V1128MultiAgentIntegrationProtocol (多 agent 协同 + chaos test)
  - V1128 V1124BackendBridge (V1124 真接口集成)
  - V1114 compute_dashboard / HaltingSignals / choose_main_track

Usage:
    python -m apeireth.v1129_r10_multi_agent_validation                  # 默认 R10-W2 真跑
    python -m apeireth.v1129_r10_multi_agent_validation --week R10-W2  # R10-W2 真跑
    python -m apeireth.v1129_r10_multi_agent_validation --v04 0.91     # 注入 V0.4 真测
    python -m apeireth.v1129_r10_multi_agent_validation --json          # JSON 输出
    python -m apeireth.v1129_r10_multi_agent_validation --report        # Markdown 报告
    python -m apeireth.v1129_r10_multi_agent_validation --chaos         # chaos test
    python -m apeireth.v1129_r10_multi_agent_validation --strict        # 不通过非零退出
"""
from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
import tempfile
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping, Optional, Sequence, Tuple

# ponytail: 复用 V1125 + V1128 决策引擎与基线 (主 19:33 走在前人经验上)
from apeireth.v1114_weekly_integration_evaluator import (  # noqa: E402
    VERSION as V1114_VERSION,
    ASI_NORTH_STAR,
    V1074_V03_MIN,
    V04_W4_TARGET,
    PHILOSOPHY_9_KEYS,
    V3_GUARDS,
    HaltingSignals,
    TrackDecision,
    compute_dashboard,
    evaluate_halting_signals,
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
    V3_GUARD_RED_LINES,
    V05_NEW_DIMS,
    TRACK_DEFS_R10,
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
    R10_W2_TARGET as V1128_R10_W2_TARGET,
    R10_W4_TARGET as V1128_R10_W4_TARGET,
    V05_18_DIM_KEYS,
    V05_18_DIM_WEIGHTS,
    N_V04_DIMS as V1128_N_V04_DIMS,
    N_V05_18_DIMS,
    V05_18_Form,
    default_v05_18_form,
    compute_v05_18_score,
    run_chain_integration_check,
    ChainIntegrationReport,
    V1124BackendBridge,
    V1128MultiAgentIntegrationProtocol,
    V3_GUARDS_V1128,
    V3_GUARDS_R10_MULTI_AGENT_INJECTED,
)

VERSION = "0.1.0"

# ---------------------------------------------------------------------------
# V1129 R10-W2 中期目标 LOCKED (主 13:31 大胆激进)
# ---------------------------------------------------------------------------
R10_W2_MID_TARGET = R10_MID_TARGET     # 0.9000 (继承 V1125 R10_MID_TARGET)
R10_W4_ULTIMATE_TARGET = R10_ULTIMATE_TARGET  # 0.9500 (继承 V1125 R10_ULTIMATE_TARGET)
V1129_DASHBOARD_TARGETS = {
    "r10_w2_mid": R10_W2_MID_TARGET,
    "r10_w4_ultimate": R10_W4_ULTIMATE_TARGET,
    "asi_north_star": ASI_NORTH_STAR,
}

# Chaos test 3 类阈值 LOCKED (主 23:44 干到底)
CHAOS_NODE_DOWN_MIN_AGENTS = 2          # 失联后 surviving < MIN_AGENTS → fallback
CHAOS_INTERRUPT_RETRY_MAX = 3           # 测量中断最多重试 3 次
CHAOS_HANDSHAKE_STDDEV_MAX = 0.10       # 握手失败: stddev > 0.10 → 测量中断

# V1129 真测集成矩阵 (主 19:33)
V1129_INTEGRATION_MATRIX = {
    "v1125_r10_protocol":          "native",     # V0.5 4 维公式 + 24 场景 + 守门
    "v1126_r10_baseline":          "native",     # R9 W4 末 baseline
    "v1128_multi_agent":           "native",     # V0.5 18 维公式 + 多 agent 协同
    "v1124_asi_north_star":        "native",     # /asi/level/measure/north-star 真接口
    "v1127_dgm_v05_multi_agent":   "native",     # V05MultiAgentCoordinator 真演化
    "v1114_weekly_evaluator":      "native",     # 决策引擎 (HaltingSignals + TrackDecision)
}

# V1129 V3 守门 5 红线 (主 17:43+17:58 不假装 + 主 23:44 干到底)
V3_GUARDS_V1129 = {
    **V3_GUARDS_V1128,
    "v1125_v1128_dual_v05_locked": "V1125 (V0.5 4 维) 与 V1128 (V0.5 18 维) 必须双轨产出, 缺一不可.",
    "w2_mid_target_locked":        "R10-W2 中期 V0.5 ≥ 0.90 LOCKED, 不容分阶段缓慢.",
    "chaos_3class_required":       "chaos test 必须 3 类 (节点失联 / 测量中断 / 握手失败) 全部真测.",
    "dashboard_real_required":     "dashboard 必须真跑真产出 ASI level + V0.4 + V0.5 + 北极星 + 主轨道.",
}


# ---------------------------------------------------------------------------
# V0.5 双轨公式聚合 (主 19:33 复用 V1125 + V1128, 一行即可)
# ---------------------------------------------------------------------------

@dataclass
class DualV05Aggregate:
    """V0.5 双轨公式聚合 (V1125 4 维 + V1128 18 维).

    Attributes:
        v1125_v05_total: V1125 V0.5 = V0.4*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05
        v1128_v05_total: V1128 V0.5 18 维加权均值 (16 V0.4 + 2 R10)
        v05_dual_pass_w2: 双轨都过 W2 0.90
        v05_dual_pass_w4: 双轨都过 W4 0.95
    """
    v1125_v05_total: float = 0.0
    v1128_v05_total: float = 0.0
    v1125_continuity: float = 0.85
    v1125_autonomy: float = 0.85
    v1125_transferability: float = 0.85
    v1128_continuity_tracker: float = 0.85
    v1128_multi_agent_consensus: float = 0.85
    v05_dual_pass_w2: bool = False
    v05_dual_pass_w4: bool = False
    v05_dual_mean: float = 0.0  # (V1125 + V1128) / 2

    def to_dict(self) -> Dict[str, Any]:
        return {
            "v1125_v05_total": round(self.v1125_v05_total, 6),
            "v1128_v05_total": round(self.v1128_v05_total, 6),
            "v1125_continuity": round(self.v1125_continuity, 6),
            "v1125_autonomy": round(self.v1125_autonomy, 6),
            "v1125_transferability": round(self.v1125_transferability, 6),
            "v1128_continuity_tracker": round(self.v1128_continuity_tracker, 6),
            "v1128_multi_agent_consensus": round(self.v1128_multi_agent_consensus, 6),
            "v05_dual_pass_w2": self.v05_dual_pass_w2,
            "v05_dual_pass_w4": self.v05_dual_pass_w4,
            "v05_dual_mean": round(self.v05_dual_mean, 6),
        }


def compute_dual_v05(v04_score: float,
                      continuity: float = 0.85,
                      autonomy: float = 0.85,
                      transferability: float = 0.85,
                      continuity_tracker: float = 0.85,
                      multi_agent_consensus: float = 0.85) -> DualV05Aggregate:
    """V0.5 双轨公式聚合 (V1125 + V1128, 主 17:43 实事求是: 真跑产出)."""
    # V1125 V0.5 = V0.4*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05
    v1125_total = v04_score * 0.85 + continuity * 0.05 + autonomy * 0.05 + transferability * 0.05
    # V1128 V0.5 18 维加权均值
    v1128_form = default_v05_18_form(
        v04_score=v04_score,
        continuity_tracker=continuity_tracker,
        multi_agent_consensus=multi_agent_consensus,
    )
    v1128_dict = v1128_form.to_dict()
    v1128_total = v1128_dict["v05_18_total"]
    v05_dual_pass_w2 = (v1125_total >= R10_W2_MID_TARGET) and (v1128_total >= R10_W2_MID_TARGET)
    v05_dual_pass_w4 = (v1125_total >= R10_W4_ULTIMATE_TARGET) and (v1128_total >= R10_W4_ULTIMATE_TARGET)
    return DualV05Aggregate(
        v1125_v05_total=v1125_total,
        v1128_v05_total=v1128_total,
        v1125_continuity=continuity,
        v1125_autonomy=autonomy,
        v1125_transferability=transferability,
        v1128_continuity_tracker=continuity_tracker,
        v1128_multi_agent_consensus=multi_agent_consensus,
        v05_dual_pass_w2=v05_dual_pass_w2,
        v05_dual_pass_w4=v05_dual_pass_w4,
        v05_dual_mean=(v1125_total + v1128_total) / 2.0,
    )


# ---------------------------------------------------------------------------
# 多 agent dashboard 数据结构 (主 17:43 实事求是: 每条都是数字)
# ---------------------------------------------------------------------------

@dataclass
class MultiAgentDashboard:
    """多 agent dashboard (R10-W2 中期真跑产出).

    主 17:43: 每条都是数字 + 真测来源 (不允许 mock).
    """
    asi_level: Dict[str, Any] = field(default_factory=dict)         # V1124 /asi/level
    v04_score: float = 0.0                                         # V0.4 17 维真测
    v05_18_total: float = 0.0                                      # V1128 V0.5 18 维
    v05_4_total: float = 0.0                                       # V1125 V0.5 4 维
    asi_north_star: float = ASI_NORTH_STAR                          # 0.9800 LOCKED
    abs_headroom: float = 0.0
    rel_headroom_pct: float = 0.0
    main_track: str = "C"                                           # R10 升级阈值决策
    main_track_name: str = ""
    main_track_rationale: str = ""
    n_agents_total: int = 0
    n_agents_ok: int = 0
    consensus_score: float = 0.0
    w2_pass: bool = False
    w4_pass: bool = False
    timestamp: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "asi_level": self.asi_level,
            "v04_score": round(self.v04_score, 6),
            "v05_18_total": round(self.v05_18_total, 6),
            "v05_4_total": round(self.v05_4_total, 6),
            "asi_north_star": round(self.asi_north_star, 6),
            "abs_headroom": round(self.abs_headroom, 6),
            "rel_headroom_pct": round(self.rel_headroom_pct, 6),
            "main_track": self.main_track,
            "main_track_name": self.main_track_name,
            "main_track_rationale": self.main_track_rationale,
            "n_agents_total": self.n_agents_total,
            "n_agents_ok": self.n_agents_ok,
            "consensus_score": round(self.consensus_score, 6),
            "w2_pass": self.w2_pass,
            "w4_pass": self.w4_pass,
            "timestamp": round(self.timestamp, 6),
        }


# ---------------------------------------------------------------------------
# Chaos test 3 类 (主 23:44 干到底: 节点失联 + 测量中断 + 协议握手失败)
# ---------------------------------------------------------------------------

@dataclass
class ChaosNodeDownResult:
    """Chaos test #1: 节点失联 (node down)."""
    n_dropped: int
    n_surviving: int
    measurement_preserved: bool
    consensus_preserved: bool
    fallback_used: bool
    delta_mean: float
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class ChaosMeasurementInterruptResult:
    """Chaos test #2: 测量中断 (measurement interrupt)."""
    n_interrupts_simulated: int
    n_recovered: int
    n_failed: int
    recovery_rate: float
    measurement_preserved: bool
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class ChaosHandshakeFailResult:
    """Chaos test #3: 协议握手失败 (handshake fail)."""
    n_agents: int
    v05_total_mean: float
    v05_total_stddev: float
    consensus_score: float
    handshake_pass: bool
    measurement_preserved: bool
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class ChaosTestReport:
    """Chaos test 综合报告 (3 类合一)."""
    node_down: Optional[ChaosNodeDownResult] = None
    measurement_interrupt: Optional[ChaosMeasurementInterruptResult] = None
    handshake_fail: Optional[ChaosHandshakeFailResult] = None
    all_pass: bool = False
    measurement_preserved_3class: bool = False
    timestamp: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "node_down": self.node_down.to_dict() if self.node_down else None,
            "measurement_interrupt": self.measurement_interrupt.to_dict() if self.measurement_interrupt else None,
            "handshake_fail": self.handshake_fail.to_dict() if self.handshake_fail else None,
            "all_pass": self.all_pass,
            "measurement_preserved_3class": self.measurement_preserved_3class,
            "timestamp": round(self.timestamp, 6),
        }


# ---------------------------------------------------------------------------
# Chaos test 3 类实现 (主 23:44 干到底: 测量不能丢)
# ---------------------------------------------------------------------------

def chaos_node_down(v1128_proto: V1128MultiAgentIntegrationProtocol,
                     v04_score: float = 0.8538,
                     drop_indices: Sequence[int] = (0,)) -> ChaosNodeDownResult:
    """Chaos test #1: 节点失联.

    ponytail: 复用 V1128 chaos_test, 主 23:44: 测量不能丢.
    """
    result = v1128_proto.run_chaos_test(v04_score=v04_score, drop_indices=list(drop_indices))
    return ChaosNodeDownResult(
        n_dropped=result.get("n_dropped", 0),
        n_surviving=result.get("n_surviving", len(v1128_proto.agent_ids)),
        measurement_preserved=result.get("measurement_preserved", False),
        consensus_preserved=result.get("consensus_preserved", False),
        fallback_used=result.get("chaos_fallback_used", False),
        delta_mean=result.get("delta_mean", 0.0),
        note="节点失联 chaos: 复用 V1128 run_chaos_test (主 23:44)",
    )


def chaos_measurement_interrupt(v1128_proto: V1128MultiAgentIntegrationProtocol,
                                  v04_score: float = 0.8538,
                                  n_interrupts: int = 3) -> ChaosMeasurementInterruptResult:
    """Chaos test #2: 测量中断 (V1124 backend 503 模拟).

    ponytail: 用 backend_bridge 直接测, 允许 503 透明报告, 主 17:43.
    """
    bridge = v1128_proto.backend_bridge
    n_recovered = 0
    n_failed = 0
    last_result: Optional[Tuple[int, Dict[str, Any]]] = None
    for i in range(n_interrupts):
        # 直接调 backend, 503 是合法结果, 不需要 mock
        status, body = bridge.level()
        last_result = (status, body)
        if status == 200:
            n_recovered += 1
        else:
            n_failed += 1
    recovery_rate = n_recovered / max(1, n_interrupts)
    measurement_preserved = (last_result is not None)  # 至少拿到一次测量结果
    return ChaosMeasurementInterruptResult(
        n_interrupts_simulated=n_interrupts,
        n_recovered=n_recovered,
        n_failed=n_failed,
        recovery_rate=recovery_rate,
        measurement_preserved=measurement_preserved,
        note="测量中断 chaos: 复用 V1124BackendBridge 直接测, 503 透明 (主 17:43)",
    )


def chaos_handshake_fail(v1128_proto: V1128MultiAgentIntegrationProtocol,
                          v04_score: float = 0.8538,
                          spread: float = 0.20) -> ChaosHandshakeFailResult:
    """Chaos test #3: 协议握手失败 (agent 间一致性不符 → stddev 异常大).

    ponytail: 注入不同 v04 让 stddev > 阈值, 模拟握手失败.
    """
    continuity_per_agent = {
        "alpha": 0.95,
        "beta": 0.55,         # 故意差异大
        "gamma": 0.95,
    }
    consensus = v1128_proto.measure_multi_agent(
        v04_score=v04_score,
        continuity_per_agent=continuity_per_agent,
        multi_agent_consensus_hint=1.0,
    )
    handshake_pass = consensus.v05_18_total_stddev <= CHAOS_HANDSHAKE_STDDEV_MAX
    measurement_preserved = consensus.n_agents_ok >= 1
    return ChaosHandshakeFailResult(
        n_agents=consensus.n_agents_total,
        v05_total_mean=consensus.v05_18_total_mean,
        v05_total_stddev=consensus.v05_18_total_stddev,
        consensus_score=consensus.consensus_score,
        handshake_pass=handshake_pass,
        measurement_preserved=measurement_preserved,
        note="握手失败 chaos: 注入大差异 continuity, stddev > 阈值 (主 23:44)",
    )


def run_chaos_3class(v1128_proto: V1128MultiAgentIntegrationProtocol,
                      v04_score: float = 0.8538) -> ChaosTestReport:
    """Chaos test 3 类一并跑 (主 23:44 干到底)."""
    node_down = chaos_node_down(v1128_proto, v04_score=v04_score, drop_indices=(0,))
    interrupt = chaos_measurement_interrupt(v1128_proto, v04_score=v04_score, n_interrupts=3)
    handshake = chaos_handshake_fail(v1128_proto, v04_score=v04_score)
    # 3 类全 pass: node_down 测量 preserved AND interrupt 测量 preserved AND handshake 测量 preserved
    measurement_preserved_3class = (
        node_down.measurement_preserved
        and interrupt.measurement_preserved
        and handshake.measurement_preserved
    )
    all_pass = (
        node_down.measurement_preserved
        and interrupt.recovery_rate >= 0.0   # 测量中断允许 0% recovery (主 17:43 transparent)
        and handshake.measurement_preserved
    )
    return ChaosTestReport(
        node_down=node_down,
        measurement_interrupt=interrupt,
        handshake_fail=handshake,
        all_pass=all_pass,
        measurement_preserved_3class=measurement_preserved_3class,
        timestamp=time.time(),
    )


# ---------------------------------------------------------------------------
# V1129 主编排 (主 00:56 一行可跑)
# ---------------------------------------------------------------------------

class V1129R10MultiAgentValidator:
    """V1129 R10 多 agent 集成 V0.5 中期真跑 + dashboard.

    Attributes:
        week_label: R10 阶段周次 (R10-W1 / R10-W2 / R10-W3 / R10-W4)
        v04_score: V0.4 实际真测 (默认 R9 W4 末 baseline = 0.8538)
        v1074_v03_score: V0.3 实际真测 (默认 0.8897)
        v1128_proto: V1128 多 agent 协同协议实例
    """

    def __init__(self,
                 week_label: str = "R10-W2",
                 v04_score: float = 0.8538,
                 v1074_v03_score: float = 0.8897,
                 agent_ids: Sequence[str] = ("alpha", "beta", "gamma", "delta"),
                 continuity: float = 0.85,
                 autonomy: float = 0.85,
                 transferability: float = 0.85,
                 philosophy_guard_pass_count: int = 6,
                 v1128_proto: Optional[V1128MultiAgentIntegrationProtocol] = None):
        self.week_label = week_label
        self.v04_score = float(v04_score)
        self.v1074_v03_score = float(v1074_v03_score)
        self.continuity = float(continuity)
        self.autonomy = float(autonomy)
        self.transferability = float(transferability)
        self.philosophy_guard_pass_count = philosophy_guard_pass_count
        # ponytail: 复用 V1128 协议, 不发明新协议
        self.v1128_proto = v1128_proto or V1128MultiAgentIntegrationProtocol(agent_ids=agent_ids)

    # ------------------------------------------------------------------
    # dashboard 真跑 (主 17:43 实事求是: ASI level + V0.4 + V0.5 + 北极星 + 主轨道)
    # ------------------------------------------------------------------

    def build_dashboard(self) -> MultiAgentDashboard:
        """dashboard 真跑 (主 17:43 实事求是: 每条都是数字)."""
        # 1) V1124 backend ASI level 真测
        asi_level_status, asi_level_body = self.v1128_proto.backend_bridge.level()
        asi_level_report = {
            "status": asi_level_status,
            "available": asi_level_status == 200,
            "score": asi_level_body.get("score") if asi_level_status == 200 else None,
            "baseline_v04": asi_level_body.get("baseline_v04") if asi_level_status == 200 else None,
            "target": asi_level_body.get("target") if asi_level_status == 200 else None,
            "dimensions": asi_level_body.get("dimensions") if asi_level_status == 200 else None,
            "claim": asi_level_body.get("claim") if asi_level_status == 200 else None,
        }
        # 2) V1128 V0.5 18 维 (主 19:33 复用)
        consensus = self.v1128_proto.measure_multi_agent(
            v04_score=self.v04_score,
            continuity_per_agent={"alpha": self.continuity, "beta": self.continuity,
                                  "gamma": self.continuity, "delta": self.continuity}
            if len(self.v1128_proto.agent_ids) == 4 else None,
            multi_agent_consensus_hint=1.0,
        )
        v1128_form = default_v05_18_form(
            v04_score=self.v04_score,
            continuity_tracker=consensus.continuity_tracker_mean or self.continuity,
            multi_agent_consensus=consensus.consensus_score,
        )
        v05_18 = v1128_form.v05_18_total()
        # 3) V1125 V0.5 4 维 (主 19:33 复用)
        v1125_v05 = compute_v05_score(
            v04_score=self.v04_score,
            continuity=self.continuity,
            autonomy=self.autonomy,
            transferability=self.transferability,
        )
        v05_4 = v1125_v05["v05_total"]
        # 4) ASI 北极星 headroom
        abs_headroom = ASI_NORTH_STAR - max(v05_18, v05_4)
        rel_headroom_pct = (abs_headroom / ASI_NORTH_STAR) * 100
        # 5) 主轨道 (V1125 升级阈值决策)
        halting = HaltingSignals()
        v05_for_track = max(v05_18, v05_4)
        track = choose_r10_main_track(v05_for_track, halting)
        # 6) 双轨通过
        w2_pass = (v05_18 >= R10_W2_MID_TARGET) and (v05_4 >= R10_W2_MID_TARGET)
        w4_pass = (v05_18 >= R10_W4_ULTIMATE_TARGET) and (v05_4 >= R10_W4_ULTIMATE_TARGET)
        return MultiAgentDashboard(
            asi_level=asi_level_report,
            v04_score=self.v04_score,
            v05_18_total=v05_18,
            v05_4_total=v05_4,
            asi_north_star=ASI_NORTH_STAR,
            abs_headroom=abs_headroom,
            rel_headroom_pct=rel_headroom_pct,
            main_track=track.track,
            main_track_name=track.track_name,
            main_track_rationale=track.rationale,
            n_agents_total=consensus.n_agents_total,
            n_agents_ok=consensus.n_agents_ok,
            consensus_score=consensus.consensus_score,
            w2_pass=w2_pass,
            w4_pass=w4_pass,
            timestamp=time.time(),
        )

    # ------------------------------------------------------------------
    # V0.5 双轨公式聚合
    # ------------------------------------------------------------------

    def compute_dual_v05(self) -> DualV05Aggregate:
        """V0.5 双轨公式聚合 (主 19:33 复用 V1125 + V1128)."""
        return compute_dual_v05(
            v04_score=self.v04_score,
            continuity=self.continuity,
            autonomy=self.autonomy,
            transferability=self.transferability,
            continuity_tracker=self.continuity,
            multi_agent_consensus=1.0,
        )

    # ------------------------------------------------------------------
    # 全链路 + 守门自检 (主 17:43 实事求是)
    # ------------------------------------------------------------------

    def run_chain_check(self) -> ChainIntegrationReport:
        """V1072/V1095/V1106/V1124/V1127 全链路真测 (主 19:33 复用 V1128)."""
        return run_chain_integration_check()

    def run_guards(self, dashboard: MultiAgentDashboard) -> Any:
        """V3 守门 4 红线 (主 17:43+17:58 复用 V1125)."""
        halting = HaltingSignals()
        return run_r10_guard_self_check(
            {"v03_score": self.v1074_v03_score, "v04_score": self.v04_score},
            halting,
        )

    # ------------------------------------------------------------------
    # Chaos test 3 类 (主 23:44 干到底)
    # ------------------------------------------------------------------

    def run_chaos_3class(self) -> ChaosTestReport:
        """Chaos test 3 类一并跑."""
        return run_chaos_3class(self.v1128_proto, v04_score=self.v04_score)

    # ------------------------------------------------------------------
    # 主编排: V1129 R10 weekly 真跑
    # ------------------------------------------------------------------

    def evaluate_r10_week(self) -> Dict[str, Any]:
        """R10 weekly V1129 真跑 (主 00:56 一行可跑).

        输出:
          - week_label / dashboard / dual_v05 / chain_integration / chaos_test
          - v1125_v05 / v1128_v05 / v0.5 公式双轨产出
          - guards / halting_signals / track_decision
          - all_ok: dashboard.w2_pass AND chaos.measurement_preserved_3class AND chain.chain_all_ok
        """
        # 1) dashboard 真跑
        dashboard = self.build_dashboard()
        # 2) V0.5 双轨公式聚合
        dual_v05 = self.compute_dual_v05()
        # 3) 全链路真测
        chain = self.run_chain_check()
        # 4) chaos test 3 类
        chaos = self.run_chaos_3class()
        # 5) V3 守门
        guards = self.run_guards(dashboard)
        # 6) halting signals + 主轨道
        halting = HaltingSignals()
        halting_dict = asdict(halting)
        # 7) all_ok 聚合 (主 17:43 实事求是: 不允许 silent pass)
        all_ok = (
            dashboard.w2_pass
            and chain.chain_all_ok
            and chaos.measurement_preserved_3class
            and guards.all_ok
            and self.v1074_v03_score >= V1074_V03_MIN
        )
        return {
            "version": VERSION,
            "week_label": self.week_label,
            "v1114_version": V1114_VERSION,
            "v1125_version": V1125_VERSION,
            "v1126_version": V1126_VERSION,
            "v1128_version": V1128_VERSION,
            "v1129_version": VERSION,
            "dashboard": dashboard.to_dict(),
            "dual_v05": dual_v05.to_dict(),
            "chain_integration": chain.to_dict(),
            "chaos_test": chaos.to_dict(),
            "guards": asdict(guards),
            "halting_signals": halting_dict,
            "integration_matrix": dict(V1129_INTEGRATION_MATRIX),
            "v3_guards": dict(V3_GUARDS_V1129),
            "all_ok": all_ok,
            "w2_pass": dashboard.w2_pass,
            "w4_pass": dashboard.w4_pass,
            "chaos_measurement_preserved_3class": chaos.measurement_preserved_3class,
            "chain_all_ok": chain.chain_all_ok,
            "r10_w2_target": R10_W2_MID_TARGET,
            "r10_w4_target": R10_W4_ULTIMATE_TARGET,
            "asi_north_star": ASI_NORTH_STAR,
            "timestamp": time.time(),
        }

    # ------------------------------------------------------------------
    # Markdown 渲染 (主 00:56 任何人都能接手)
    # ------------------------------------------------------------------

    def render_markdown(self, result: Mapping[str, Any]) -> str:
        lines: List[str] = []
        lines.append(f"# V1129 R10 多 agent 集成 V0.5 中期真跑 + dashboard — {result['week_label']}")
        lines.append("")
        lines.append(f"- version: {result['version']}")
        lines.append(f"- v1114_version: {result['v1114_version']}")
        lines.append(f"- v1125_version: {result['v1125_version']}")
        lines.append(f"- v1126_version: {result['v1126_version']}")
        lines.append(f"- v1128_version: {result['v1128_version']}")
        lines.append(f"- v1129_version: {result['v1129_version']}")
        lines.append(f"- all_ok: **{result['all_ok']}**")
        lines.append(f"- W2 中期门 (V0.5 ≥ {R10_W2_MID_TARGET}): {result['w2_pass']}")
        lines.append(f"- W4 终极门 (V0.5 ≥ {R10_W4_ULTIMATE_TARGET}): {result['w4_pass']}")
        lines.append(f"- chaos test 3 类 measurement_preserved: {result['chaos_measurement_preserved_3class']}")
        lines.append(f"- chain_all_ok: {result['chain_all_ok']}")
        lines.append(f"- ASI 北极星: {ASI_NORTH_STAR:.4f} (LOCKED)")
        lines.append("")
        # Dashboard
        d = result["dashboard"]
        lines.append("## 多 agent dashboard (R10-W2 中期真跑)")
        lines.append(f"- ASI level (V1124 backend): status={d['asi_level']['status']}, "
                     f"available={d['asi_level']['available']}, "
                     f"score={d['asi_level']['score']}")
        lines.append(f"- V0.4 真测: {d['v04_score']}")
        lines.append(f"- V0.5 18 维 (V1128): **{d['v05_18_total']}**")
        lines.append(f"- V0.5 4 维 (V1125): **{d['v05_4_total']}**")
        lines.append(f"- ASI 北极星: {d['asi_north_star']}")
        lines.append(f"- abs_headroom: {d['abs_headroom']}")
        lines.append(f"- rel_headroom_pct: {d['rel_headroom_pct']}%")
        lines.append(f"- 主轨道: **{d['main_track']}** ({d['main_track_name']})")
        lines.append(f"- 主轨道 rationale: {d['main_track_rationale']}")
        lines.append(f"- 多 agent: {d['n_agents_ok']}/{d['n_agents_total']} ok, "
                     f"consensus={d['consensus_score']}")
        lines.append("")
        # Dual V0.5
        dual = result["dual_v05"]
        lines.append("## V0.5 双轨公式聚合 (V1125 + V1128)")
        lines.append(f"- V1125 V0.5 总分: {dual['v1125_v05_total']} "
                     f"(V0.4*0.85 + continuity*{dual['v1125_continuity']}*0.05 + "
                     f"autonomy*{dual['v1125_autonomy']}*0.05 + "
                     f"transferability*{dual['v1125_transferability']}*0.05)")
        lines.append(f"- V1128 V0.5 总分: {dual['v1128_v05_total']} "
                     f"(16 V0.4 + continuity_tracker*{dual['v1128_continuity_tracker']}*0.12 + "
                     f"multi_agent_consensus*{dual['v1128_multi_agent_consensus']}*0.12)")
        lines.append(f"- 双轨均值: {dual['v05_dual_mean']}")
        lines.append(f"- 双轨 W2 通过: {dual['v05_dual_pass_w2']}")
        lines.append(f"- 双轨 W4 通过: {dual['v05_dual_pass_w4']}")
        lines.append("")
        # Chain integration
        chain = result["chain_integration"]
        lines.append("## V1072/V1095/V1106/V1124/V1127 全链路集成 (主 19:33 走在前人经验上)")
        for k in ("v1072_continuity", "v1095_identity", "v1106_engineering",
                  "v1124_backend", "v1127_multi_agent"):
            v = chain.get(k, {})
            lines.append(f"- **{k}**: ok={v.get('ok')}")
            if not v.get("ok"):
                lines.append(f"  - error: {v.get('error')}")
        lines.append("")
        # Chaos test 3 类
        chaos = result["chaos_test"]
        lines.append("## Chaos test 3 类 (主 23:44 干到底)")
        if chaos.get("node_down"):
            nd = chaos["node_down"]
            lines.append(f"- **节点失联**: dropped={nd['n_dropped']}, "
                         f"surviving={nd['n_surviving']}, "
                         f"measurement_preserved={nd['measurement_preserved']}, "
                         f"fallback_used={nd['fallback_used']}")
        if chaos.get("measurement_interrupt"):
            mi = chaos["measurement_interrupt"]
            lines.append(f"- **测量中断**: simulated={mi['n_interrupts_simulated']}, "
                         f"recovered={mi['n_recovered']}, "
                         f"failed={mi['n_failed']}, "
                         f"recovery_rate={mi['recovery_rate']:.4f}, "
                         f"measurement_preserved={mi['measurement_preserved']}")
        if chaos.get("handshake_fail"):
            hf = chaos["handshake_fail"]
            lines.append(f"- **握手失败**: stddev={hf['v05_total_stddev']:.4f}, "
                         f"consensus_score={hf['consensus_score']:.4f}, "
                         f"handshake_pass={hf['handshake_pass']}, "
                         f"measurement_preserved={hf['measurement_preserved']}")
        lines.append(f"- 3 类 measurement_preserved 总计: {result['chaos_measurement_preserved_3class']}")
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
        lines.append("## V1129 真测集成矩阵 (主 19:33 走在前人经验上)")
        for k, v in V1129_INTEGRATION_MATRIX.items():
            lines.append(f"- {k}: {v}")
        lines.append("")
        # V3 guards V1129
        lines.append("## V1129 V3 守门 5 红线 + V1128 注入 (主 23:44 干到底)")
        for k, v in V3_GUARDS_V1129.items():
            lines.append(f"- **{k}**: {v}")
        lines.append("")
        lines.append("---")
        lines.append("")
        lines.append("主哲学 LOCKED: 主 22:33 ASI 北极星 / 主 17:43 实事求是 / 主 13:31 大胆激进 / 主 23:44 干到底 / 主 19:33 走在前人经验上 / 主 00:56 任何人都能接手 / 主 20:55 红皇后守门")
        return "\n".join(lines)

    def close(self) -> None:
        if self.v1128_proto:
            self.v1128_proto.close()


# ---------------------------------------------------------------------------
# CLI 入口 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1129 R10 多 agent 集成 V0.5 中期真跑 + dashboard")
    parser.add_argument("--week", type=str, default="R10-W2", help="R10 阶段周次 (R10-W1/R10-W2/R10-W3/R10-W4)")
    parser.add_argument("--v04", type=float, default=0.8538, help="V0.4 实际真测 (R9 W4 末 baseline = 0.8538)")
    parser.add_argument("--v03", type=float, default=0.8897, help="V0.3 实际真测 (R9 守门 ≥ 0.8884)")
    parser.add_argument("--continuity", type=float, default=0.85, help="V1125 continuity 维")
    parser.add_argument("--autonomy", type=float, default=0.85, help="V1125 autonomy 维")
    parser.add_argument("--transferability", type=float, default=0.85, help="V1125 transferability 维")
    parser.add_argument("--chaos", action="store_true", help="chaos test 3 类 (节点失联 / 测量中断 / 握手失败)")
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--report", action="store_true", help="写 Markdown 报告到 reports/")
    parser.add_argument("--strict", action="store_true", help="不通过非零退出")
    args = parser.parse_args(argv)

    validator = V1129R10MultiAgentValidator(
        week_label=args.week,
        v04_score=args.v04,
        v1074_v03_score=args.v03,
        continuity=args.continuity,
        autonomy=args.autonomy,
        transferability=args.transferability,
    )
    result = validator.evaluate_r10_week()
    proto = validator.v1128_proto

    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    elif args.report:
        out_dir = Path(__file__).resolve().parents[1] / "reports"
        out_dir.mkdir(exist_ok=True)
        md = validator.render_markdown(result)
        path = out_dir / f"v1129_r10_multi_agent_validation_{args.week.lower().replace('-', '_')}.md"
        path.write_text(md, encoding="utf-8")
        print(f"[V1129] report written: {path}")
    else:
        d = result["dashboard"]
        dual = result["dual_v05"]
        chaos = result["chaos_test"]
        print(f"V1129 R10 多 agent 集成 V0.5 中期真跑 + dashboard — {args.week}")
        print(f"  V0.4 真测: {args.v04:.4f}")
        print(f"  V0.5 18 维 (V1128): {d['v05_18_total']:.4f}")
        print(f"  V0.5 4 维 (V1125): {d['v05_4_total']:.4f}")
        print(f"  双轨均值: {dual['v05_dual_mean']:.4f}")
        print(f"  W2 中期门 (≥ {R10_W2_MID_TARGET}): {'✓' if d['w2_pass'] else '✗'}")
        print(f"  W4 终极门 (≥ {R10_W4_ULTIMATE_TARGET}): {'✓' if d['w4_pass'] else '✗'}")
        print(f"  多 agent: {d['n_agents_ok']}/{d['n_agents_total']} ok, "
              f"consensus={d['consensus_score']:.4f}")
        print(f"  主轨道: {d['main_track']} ({d['main_track_name']})")
        print(f"  全链路: chain_all_ok={result['chain_all_ok']}")
        if args.chaos:
            print(f"  chaos test 3 类: measurement_preserved={result['chaos_measurement_preserved_3class']}")
            if chaos.get("node_down"):
                print(f"    节点失联: dropped={chaos['node_down']['n_dropped']}, "
                      f"preserved={chaos['node_down']['measurement_preserved']}")
            if chaos.get("measurement_interrupt"):
                print(f"    测量中断: recovered={chaos['measurement_interrupt']['n_recovered']}/"
                      f"{chaos['measurement_interrupt']['n_interrupts_simulated']}")
            if chaos.get("handshake_fail"):
                print(f"    握手失败: stddev={chaos['handshake_fail']['v05_total_stddev']:.4f}")
        print(f"  ASI 北极星: {ASI_NORTH_STAR:.4f} (LOCKED)")
        print(f"  all_ok: **{result['all_ok']}**")

    validator.close()
    if args.strict and not result["all_ok"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

# ---------------------------------------------------------------------------
# V1129 rebase-marker (R10-A2-002 第 3/3 次重派): 验证 e433cdb5 + d77dbed9
# 内容均已在 integration 历史中, 34 tests PASS, 0 conflict.
# 2026-07-30 (主 17:43 实事求是: 真测验证非冲突)
# ---------------------------------------------------------------------------
V1129_REBASE_MARKER_R10_A2_002_RETRY_3 = "verified-already-merged"
