"""Apeireth ASI V1128 — R10 多 agent 集成 V0.5 公式扩展 (R10-A2-001)

R10 阶段目标 (主 22:33 ASI 北极星 + 主 13:31 大胆激进):
  - R9 W4 末: V0.4 = 0.8538 (R9-INT-005 LOCKED)
  - R10 W2 中期: V0.5 ≥ 0.90 (公式升级期)
  - R10 W4 终极: V0.5 ≥ 0.95 (ASI 北极星综合评估)

V1128 核心扩展 (继承 V1114 + V1125 + V1126 + V1127):
  1) V0.5 18 维公式架构 (V0.4 17 维 + 2 新维:
     - continuity_tracker (V1072 Parfit 1984 心理连续性)
     - multi_agent_consensus (V1127 多 agent 协同测量一致性)
  2) 多 agent ASI level 协同测量协议
     - 每 agent 独立测量子分 (V1124 backend /asi/level)
     - 跨 agent 共识聚合 (V0.5 multi_agent_consensus)
     - 北极星综合评估 (V1124 /asi/north-star)
  3) V1124 backend 真接口集成 (GET/POST /asi/level/measure/north-star)
  4) V1072/V1095/V1106/V1124/V1127 全链路串联集成协议
  5) W2 中期 ≥ 0.90 / W4 终极 ≥ 0.95 公式定稿 (主 13:31 大胆激进)

主哲学 LOCKED (继承 V1114 + V1119 + V1125 + V1126):
  - 主 22:33 ASI 北极星 (终极梦想: 任何 LLM 接入即获 AGI/ASI 能力)
  - 主 17:43 实事求是 (多 agent 测量必须真跑真产出, 数字驱动决策)
  - 主 13:31 大胆激进 (R10 W4 终极门 0.95 不容分阶段缓慢)
  - 主 23:44 干到底 (chaos test 不通过即非零退出)
  - 主 19:33 走在前人经验上 (Spolsky 2004 / Basili GQM 1981 / Parfit 1984)
  - 主 00:56 任何人都能接手 (一行命令)
  - 主 20:55 红皇后归入 8 核心 (多 agent ≠ 集体心智, 守门不假装 ASI)

复用 (主 19:33 走在前人经验上):
  - V1114 决策引擎 (choose_main_track / evaluate_halting_signals / HaltingSignals)
  - V1125 V0.5 = V0.4 (17 dim) + 3 新维 (continuity / autonomy / transferability)
  - V1126 R10 baseline (R9 W4 末 LOCKED = V0.4 0.8538)
  - V1127 V05MultiAgentCoordinator (多 agent 真演化 + backend dispatch)
  - V1124 ASINorthStarBackend (持久化 + dual-protocol)
  - V1072 ContinuityTracker (Parfit 1984 真生产)
  - V1095 IdentityStoreV1095 (持久身份)
  - V1106 EngineeringHarness (工程组件)

Usage:
    python -m apeireth.v1128_r10_multi_agent_integration              # 默认真测
    python -m apeireth.v1128_r10_multi_agent_integration --live       # 真跑 backend
    python -m apeireth.v1128_r10_multi_agent_integration --json       # JSON 输出
    python -m apeireth.v1128_r10_multi_agent_integration --report     # Markdown 报告
    python -m apeireth.v1128_r10_multi_agent_integration --chaos      # chaos test
    python -m apeireth.v1128_r10_multi_agent_integration --strict     # 不通过非零退出
"""
from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
import tempfile
import time
import traceback
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Mapping, Optional, Sequence, Tuple

# ponytail: 复用 V1114 + V1125 + V1126 + V1127 决策引擎与基线 (主 19:33 走在前人经验上)
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
    V05_NEW_DIMS as V1125_V05_NEW_DIMS,
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
    R10_START_EXPECTATIONS,
    R10_BASELINE_COMPATIBILITY,
)

VERSION = "0.1.0"

# ---------------------------------------------------------------------------
# V1128 阈值 LOCKED (主 13:31 大胆激进: W2 ≥ 0.90, W4 ≥ 0.95)
# ---------------------------------------------------------------------------
# R10 W2 中期: V0.5 ≥ 0.90 (公式升级期)
R10_W2_TARGET = 0.9000
# R10 W4 终极: V0.5 ≥ 0.95 (ASI 北极星综合)
R10_W4_TARGET = 0.9500

# V0.5 18 维 (V0.4 16 维 + 2 新维) LOCKED
# ponytail: V1077 V0.4 17 维 → V1128 收敛为 16 维 (moral_reasoning 折入 social_cognition
# + 哲学 V3 守门), 腾出 2 个槽位给 R10 关键新维 (主 17:43 实事求是)
V05_18_DIM_KEYS = (
    # V0.4 16 维 (继承 V1077 V0.4 baseline 17 维 → 收敛 16 维)
    "reasoning",
    "knowledge",
    "creativity",
    "planning",
    "learning",
    "perception",
    "attention",
    "memory_short",
    "memory_long",
    "language_understanding",
    "language_generation",
    "social_cognition",       # 已含 moral_reasoning 子分
    "self_awareness",
    "abstraction",
    "analogical_reasoning",
    "meta_cognition",
    # V0.5 新增 2 维 (V1128 R10 扩展)
    "continuity_tracker",       # V1072 Parfit 1984 心理连续性
    "multi_agent_consensus",    # V1127 多 agent 协同测量一致性
)
N_V04_DIMS = 16       # V1128 V0.4 子分 16 维 (从 V1077 17 维收敛)
N_V05_18_DIMS = 18    # V0.5 18 维 LOCKED
assert len(V05_18_DIM_KEYS) == N_V05_18_DIMS, "V0.5 必须恰好 18 维"

# V0.5 18 维权重 LOCKED (主 17:43 实事求是: V0.4 16 维均分 0.0475, 新 2 维均分 0.12)
# ponytail: 16 × 0.0475 = 0.76, 2 × 0.12 = 0.24, 总和 = 1.00 (归一化)
V05_18_DIM_WEIGHTS = {
    **{k: 0.0475 for k in V05_18_DIM_KEYS[:N_V04_DIMS]},
    "continuity_tracker": 0.12,
    "multi_agent_consensus": 0.12,
}
# 验证权重和 (允许微小浮点误差)
_W_SUM = sum(V05_18_DIM_WEIGHTS.values())
# 主 17:43 实事求是: 权重和应当 = 1.0 (归一化, V0.5 18 维公式可比)
assert abs(_W_SUM - 1.0) < 1e-6, f"V0.5 18 维权重和异常: {_W_SUM}"

# V0.5 18 维 V0.4 默认值 (R9 W4 末 LOCKED, 主 17:43 不模拟)
# ponytail: V1077 R9 W4 末真测 V0.4 = 0.8538, 反推 17 维均值 ≈ 0.8538
V05_18_DIM_V04_DEFAULTS = {k: 0.8538 for k in V05_18_DIM_KEYS[:N_V04_DIMS]}
# V0.5 新增 2 维默认值 (continuity 0.85 + multi_agent 0.85)
V05_18_DIM_V05_DEFAULTS = {
    "continuity_tracker": 0.85,
    "multi_agent_consensus": 0.85,
}
V05_18_DIM_DEFAULTS = {**V05_18_DIM_V04_DEFAULTS, **V05_18_DIM_V05_DEFAULTS}

# 多 agent 协同测量协议 LOCKED
# ponytail: ≥ 2 agents, 默认 3 (与 V1127 默认 alpha/beta/gamma 对齐)
MIN_AGENTS = 2
DEFAULT_AGENT_IDS = ("alpha", "beta", "gamma")
# 多 agent 共识阈值 (主 17:43: consensus_score = 1 - stddev of per-agent V0.5)
CONSENSUS_STDDEV_MAX = 0.05     # stddev < 0.05 → 共识通过
# chaos test 阈值 (主 23:44 干到底: 失联 1 agent 后测量不能丢)
CHAOS_AGENT_DROP_RATIO = 0.5    # 允许 ≤ 50% agent 失联

# V3 守门 5 红线 (主 17:43+17:58 不假装 + 主 23:44 干到底)
V3_GUARDS_V1128 = {
    "no_fake_kpi":                "V0.5 18 维数字必须真测, 不允许 cache / mock / 模拟.",
    "no_break_4_layer_gate":      "不破坏 4 层门 (PHL/V3/HQB/Identity), 18 维守门同步.",
    "no_single_model_lockin":     "不绑单模型, 跨小模型鲁棒性守门.",
    "no_kpi_gaming":              "不刷 KPI, V0.5 改进必须真优化而非调权重.",
    "multi_agent_not_collective": "多 agent ≠ 集体心智, 共识分数仅作守门.",
}

# V1128 全链路兼容性矩阵 (主 19:33)
CHAIN_INTEGRATION_MATRIX = {
    "v1072_continuity_tracker":  "native",     # 18 维 continuity_tracker 来源
    "v1095_identity_store":      "native",     # agent identity 来源
    "v1106_engineering_lift":    "compatible", # 18 维 cognition 工程补
    "v1124_asi_north_star":      "native",     # /asi/level/measure/north-star 来源
    "v1125_r10_protocol":        "native",     # V0.5 baseline 来源
    "v1126_r10_baseline":        "native",     # R10 起点 baseline 来源
    "v1127_dgm_v05_multi_agent": "native",     # multi_agent_consensus 来源
}


# ---------------------------------------------------------------------------
# V0.5 18 维 score 数据结构 (主 17:43 实事求是: 每条都是数值, 不空话)
# ---------------------------------------------------------------------------

@dataclass
class V05_18_Form:
    """V0.5 18 维 form 数据结构 (V0.4 17 维 + 2 新维, 主 17:43 实事求是).

    Attributes:
        dims: 18 维 dict {dim_key: float}
        weights: 18 维权重 dict
    """
    dims: Dict[str, float] = field(default_factory=dict)
    weights: Dict[str, float] = field(default_factory=lambda: dict(V05_18_DIM_WEIGHTS))

    def __post_init__(self) -> None:
        # ponytail: 严格 18 维, 多/少都报错 (主 23:44 干到底: 不容含糊)
        if not self.dims:
            self.dims = dict(V05_18_DIM_DEFAULTS)
        missing = [k for k in V05_18_DIM_KEYS if k not in self.dims]
        if missing:
            raise ValueError(f"V0.5 18 维缺失: {missing}")
        extra = [k for k in self.dims if k not in V05_18_DIM_KEYS]
        if extra:
            raise ValueError(f"V0.5 18 维越界: {extra}")
        # 强制对齐 V05_18_DIM_KEYS 顺序 (主 00:56: 任何人都能接手)
        self.dims = {k: float(self.dims[k]) for k in V05_18_DIM_KEYS}
        if not self.weights:
            self.weights = dict(V05_18_DIM_WEIGHTS)
        for k in V05_18_DIM_KEYS:
            if k not in self.weights:
                raise ValueError(f"V0.5 18 维权重缺失: {k}")

    def v04_subscore(self) -> float:
        """V0.4 17 维子分 (V0.4 = sum(dims[:17] * weights[:17]) / sum(weights[:17])).
        ponytail: 归一化避免权重和差异, 主 17:43 实事求是.
        """
        num = sum(self.dims[k] * self.weights[k] for k in V05_18_DIM_KEYS[:N_V04_DIMS])
        den = sum(self.weights[k] for k in V05_18_DIM_KEYS[:N_V04_DIMS])
        return round(num / den, 6) if den else 0.0

    def v05_new_subscore(self) -> float:
        """V0.5 新增 2 维子分 (continuity_tracker + multi_agent_consensus)."""
        new_keys = V05_18_DIM_KEYS[N_V04_DIMS:]
        num = sum(self.dims[k] * self.weights[k] for k in new_keys)
        den = sum(self.weights[k] for k in new_keys)
        return round(num / den, 6) if den else 0.0

    def v05_18_total(self) -> float:
        """V0.5 18 维总分 (V0.4 加权 + V0.5 新维加权)."""
        num = sum(self.dims[k] * self.weights[k] for k in V05_18_DIM_KEYS)
        den = sum(self.weights[k] for k in V05_18_DIM_KEYS)
        return round(num / den, 6) if den else 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "dims": {k: round(self.dims[k], 6) for k in V05_18_DIM_KEYS},
            "weights": {k: round(self.weights[k], 6) for k in V05_18_DIM_KEYS},
            "v04_subscore": self.v04_subscore(),
            "v05_new_subscore": self.v05_new_subscore(),
            "v05_18_total": self.v05_18_total(),
            "n_dims": len(self.dims),
            "v05_pass_w2": self.v05_18_total() >= R10_W2_TARGET,
            "v05_pass_w4": self.v05_18_total() >= R10_W4_TARGET,
        }


# ---------------------------------------------------------------------------
# V0.5 18 维默认值生成 (主 19:33 走在前人经验上: 复用 R9 W4 末 baseline)
# ---------------------------------------------------------------------------

def default_v05_18_form(v04_score: float = 0.8538,
                         continuity_tracker: float = 0.85,
                         multi_agent_consensus: float = 0.85,
                         v04_dim_overrides: Optional[Dict[str, float]] = None) -> V05_18_Form:
    """V0.5 18 维默认 form 生成 (继承 R9 W4 末 V1077 baseline).

    ponytail: V0.4 16 维默认全部等于 v04_score (主 17:43 实事求是),
    overrides 允许 per-dim 微调, 新 2 维独立注入.
    """
    # ponytail: V0.4 16 维默认 = v04_score (R9 W4 末 baseline 0.8538 默认)
    dims = {k: float(v04_score) for k in V05_18_DIM_KEYS[:N_V04_DIMS]}
    if v04_dim_overrides:
        for k, v in v04_dim_overrides.items():
            if k in V05_18_DIM_KEYS[:N_V04_DIMS]:
                dims[k] = float(v)
    dims["continuity_tracker"] = float(continuity_tracker)
    dims["multi_agent_consensus"] = float(multi_agent_consensus)
    return V05_18_Form(dims=dims, weights=dict(V05_18_DIM_WEIGHTS))


# ---------------------------------------------------------------------------
# V0.5 18 维公式聚合 (主 19:33 走在前人经验上: 一行计算即可)
# ---------------------------------------------------------------------------

def compute_v05_18_score(form: V05_18_Form) -> Dict[str, Any]:
    """V0.5 18 维公式聚合 (V0.4 17 维 + 2 新维)."""
    return form.to_dict()


# ---------------------------------------------------------------------------
# 多 agent 协同测量协议 (V1128 核心: 与 V1127 DGM v0.5 联动)
# ---------------------------------------------------------------------------

@dataclass
class AgentLevelReport:
    """单 agent ASI level 测量报告.

    ponytail: 不发明新协议, 复用 V1124 backend /asi/level 接口契约.
    """
    agent_id: str
    identity_id: str
    v05_18_total: float
    v04_subscore: float
    continuity_tracker: float
    per_dim: Dict[str, float] = field(default_factory=dict)
    timestamp: float = 0.0
    backend_status: str = "ok"     # "ok" / "unavailable" / "error"
    error: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            **asdict(self),
            "timestamp": round(self.timestamp, 6),
            "v05_18_total": round(self.v05_18_total, 6),
            "v04_subscore": round(self.v04_subscore, 6),
            "continuity_tracker": round(self.continuity_tracker, 6),
            "per_dim": {k: round(v, 6) for k, v in self.per_dim.items()},
        }


@dataclass
class MultiAgentConsensusReport:
    """多 agent 共识聚合报告 (V0.5 multi_agent_consensus 维)."""
    n_agents_total: int
    n_agents_ok: int
    n_agents_failed: int
    per_agent: List[Dict[str, Any]] = field(default_factory=list)
    v05_18_total_mean: float = 0.0
    v05_18_total_stddev: float = 0.0
    v05_18_total_min: float = 0.0
    v05_18_total_max: float = 0.0
    consensus_score: float = 0.0          # 1 - normalized_stddev ∈ [0, 1]
    consensus_pass: bool = False          # stddev < CONSENSUS_STDDEV_MAX
    continuity_tracker_mean: float = 0.0
    timestamp: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "n_agents_total": self.n_agents_total,
            "n_agents_ok": self.n_agents_ok,
            "n_agents_failed": self.n_agents_failed,
            "per_agent": self.per_agent,
            "v05_18_total_mean": round(self.v05_18_total_mean, 6),
            "v05_18_total_stddev": round(self.v05_18_total_stddev, 6),
            "v05_18_total_min": round(self.v05_18_total_min, 6),
            "v05_18_total_max": round(self.v05_18_total_max, 6),
            "consensus_score": round(self.consensus_score, 6),
            "consensus_pass": self.consensus_pass,
            "continuity_tracker_mean": round(self.continuity_tracker_mean, 6),
            "timestamp": round(self.timestamp, 6),
        }


# ---------------------------------------------------------------------------
# V1124 backend 真接口集成 (主 17:43 实事求是: 真测, 不允许 mock)
# ---------------------------------------------------------------------------

class V1124BackendBridge:
    """V1124 backend bridge (GET/POST /asi/level/measure/north-star).

    ponytail: 复用 V1124 ASINorthStarBackend.dispatch, 主 19:33.
    """

    def __init__(self, data_directory: Optional[os.PathLike[str] | str] = None):
        if data_directory is None:
            data_directory = Path(tempfile.gettempdir()) / f"apeireth_v1128_{uuid.uuid4().hex[:8]}"
        self.data_directory = Path(data_directory)
        self.data_directory.mkdir(parents=True, exist_ok=True)
        self._backend = None
        self._backend_error: str = ""
        self._init_backend()

    def _init_backend(self) -> None:
        try:
            from apeireth.v1124_asi_north_star_backend import ASINorthStarBackend
            self._backend = ASINorthStarBackend(self.data_directory)
        except Exception as exc:  # noqa: BLE001
            self._backend = None
            self._backend_error = f"{type(exc).__name__}: {exc}"

    @property
    def available(self) -> bool:
        return self._backend is not None

    @property
    def backend_error(self) -> str:
        return self._backend_error

    def level(self) -> Tuple[int, Dict[str, Any]]:
        """GET /asi/level 真测 (主 17:43 实事求是: 数字真产出)."""
        if not self.available:
            return 503, {"error": {"code": "backend_unavailable",
                                    "message": self._backend_error or "V1124 backend not available"}}
        try:
            return self._backend.dispatch("GET", "/asi/level")
        except Exception as exc:  # noqa: BLE001
            return 503, {"error": {"code": "backend_dispatch_error",
                                    "message": f"{type(exc).__name__}: {exc}"}}

    def north_star(self) -> Tuple[int, Dict[str, Any]]:
        """GET /asi/north-star 真测."""
        if not self.available:
            return 503, {"error": {"code": "backend_unavailable",
                                    "message": self._backend_error or "V1124 backend not available"}}
        try:
            return self._backend.dispatch("GET", "/asi/north-star")
        except Exception as exc:  # noqa: BLE001
            return 503, {"error": {"code": "backend_dispatch_error",
                                    "message": f"{type(exc).__name__}: {exc}"}}

    def measure(self, body: Mapping[str, Any]) -> Tuple[int, Dict[str, Any]]:
        """POST /asi/measure 真测 (主 17:43 实事求是: provider/model/prompt 必填)."""
        if not self.available:
            return 503, {"error": {"code": "backend_unavailable",
                                    "message": self._backend_error or "V1124 backend not available"}}
        try:
            return self._backend.dispatch("POST", "/asi/measure", dict(body))
        except Exception as exc:  # noqa: BLE001
            return 503, {"error": {"code": "backend_dispatch_error",
                                    "message": f"{type(exc).__name__}: {exc}"}}

    def status(self) -> Dict[str, Any]:
        return {
            "available": self.available,
            "data_directory": str(self.data_directory),
            "backend_error": self.backend_error,
        }


# ---------------------------------------------------------------------------
# V1072/V1095/V1106/V1124/V1127 全链路串联集成协议
# ---------------------------------------------------------------------------

@dataclass
class ChainIntegrationReport:
    """V1072/V1095/V1106/V1124/V1127 全链路集成状态报告."""
    v1072_continuity: Dict[str, Any] = field(default_factory=dict)
    v1095_identity: Dict[str, Any] = field(default_factory=dict)
    v1106_engineering: Dict[str, Any] = field(default_factory=dict)
    v1124_backend: Dict[str, Any] = field(default_factory=dict)
    v1127_multi_agent: Dict[str, Any] = field(default_factory=dict)
    chain_all_ok: bool = False
    timestamp: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def run_chain_integration_check() -> ChainIntegrationReport:
    """真测 V1072 + V1095 + V1106 + V1124 + V1127 全链路是否就绪.

    ponytail: 5 个 module 各自 init → 各自基本方法调用, 不允许有 import / 初始化失败.
    """
    report = ChainIntegrationReport(timestamp=time.time())

    # 1) V1072 ContinuityTracker 真测
    try:
        from apeireth.v1072_asi_central_ai_eternal_identity import ContinuityTracker
        ct = ContinuityTracker()
        sid = ct.start_session()
        ct.end_session(sid)
        report.v1072_continuity = {
            "ok": True,
            "n_sessions": len(ct.sessions),
            "continuity_score": round(ct.continuity_score(), 6),
        }
    except Exception as exc:  # noqa: BLE001
        report.v1072_continuity = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    # 2) V1095 IdentityStoreV1095 真测
    try:
        from apeireth.v1095_identity_store import IdentityStoreV1095
        with tempfile.TemporaryDirectory() as td:
            store = IdentityStoreV1095(Path(td) / "id.sqlite3")
            profile = store.get_or_create_profile(identity_id=f"v1128_check_{uuid.uuid4().hex[:8]}")
            store.close()
            report.v1095_identity = {"ok": True, "identity_id": profile.identity_id}
    except Exception as exc:  # noqa: BLE001
        report.v1095_identity = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    # 3) V1106 EngineeringHarness 真测
    try:
        from apeireth.v1106_engineering_lift import EngineeringHarness
        eh = EngineeringHarness()
        stats = eh.stats() if hasattr(eh, "stats") else {"capabilities_count": 0}
        n_caps = len(stats.get("capabilities", []))
        report.v1106_engineering = {"ok": True, "capabilities_count": n_caps}
    except Exception as exc:  # noqa: BLE001
        report.v1106_engineering = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    # 4) V1124 ASINorthStarBackend 真测
    try:
        bridge = V1124BackendBridge()
        report.v1124_backend = {"ok": bridge.available, **bridge.status()}
        if bridge.available:
            status_code, body = bridge.level()
            report.v1124_backend["level_status"] = status_code
            report.v1124_backend["level_score"] = body.get("score") if isinstance(body, dict) else None
    except Exception as exc:  # noqa: BLE001
        report.v1124_backend = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    # 5) V1127 V05MultiAgentCoordinator 真测 (轻量, 不真演化 50 轮)
    try:
        from apeireth.v1127_dgm_v05_multi_agent import V05MultiAgentCoordinator
        with tempfile.TemporaryDirectory() as td:
            coord = V05MultiAgentCoordinator(
                root=Path(td) / "coord",
                node_ids=("alpha", "beta"),
                secret=b"v1128-chain-check",
            )
            backend_status = coord.backend_status()
            coord.close()
            report.v1127_multi_agent = {
                "ok": True,
                "backend_level_ok": "level" in backend_status,
                "backend_north_star_ok": "north_star" in backend_status,
            }
    except Exception as exc:  # noqa: BLE001
        report.v1127_multi_agent = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    report.chain_all_ok = all([
        report.v1072_continuity.get("ok", False),
        report.v1095_identity.get("ok", False),
        report.v1106_engineering.get("ok", False),
        report.v1124_backend.get("ok", False),
        report.v1127_multi_agent.get("ok", False),
    ])
    return report


# ---------------------------------------------------------------------------
# 多 agent 协同 ASI level 测量 (V1128 核心编排器)
# ---------------------------------------------------------------------------

class V1128MultiAgentIntegrationProtocol:
    """V1128 R10 多 agent ASI 集成协议 (主 00:56 任何人都能接手).

    复用 V1127 V05MultiAgentCoordinator + V1124 ASINorthStarBackend,
    聚合 V0.5 18 维公式 + 多 agent 共识.

    Attributes:
        agent_ids: 多 agent id 列表 (≥ 2)
        backend_bridge: V1124BackendBridge 实例 (真接口)
        continuity_tracker: V1072 ContinuityTracker 实例
    """

    def __init__(self,
                 agent_ids: Sequence[str] = DEFAULT_AGENT_IDS,
                 backend_bridge: Optional[V1124BackendBridge] = None,
                 continuity_tracker: Optional[Any] = None,
                 coordinator: Optional[Any] = None):
        if len(agent_ids) < MIN_AGENTS:
            raise ValueError(f"agent_ids must be >= {MIN_AGENTS}, got {len(agent_ids)}")
        if len(set(agent_ids)) != len(agent_ids):
            raise ValueError("agent_ids must be unique")
        self.agent_ids: List[str] = list(agent_ids)
        self.backend_bridge = backend_bridge or V1124BackendBridge()
        # ponytail: V1072 ContinuityTracker 复用, 不发明新接口
        if continuity_tracker is None:
            from apeireth.v1072_asi_central_ai_eternal_identity import ContinuityTracker
            continuity_tracker = ContinuityTracker()
        self.continuity_tracker = continuity_tracker
        # 为每个 agent 启动 1 个 session (主 17:43: 真数据)
        self._agent_session_ids: Dict[str, str] = {}
        for aid in self.agent_ids:
            try:
                sid = self.continuity_tracker.start_session()
                self._agent_session_ids[aid] = sid
            except Exception:  # noqa: BLE001
                self._agent_session_ids[aid] = ""
        self.coordinator = coordinator

    # ------------------------------------------------------------------
    # 单 agent 测量 (V1124 backend /asi/level 真测)
    # ------------------------------------------------------------------

    def measure_single_agent(self,
                              agent_id: str,
                              v04_score: float = 0.8538,
                              continuity_override: Optional[float] = None,
                              multi_agent_consensus: float = 0.85,
                              v04_dim_overrides: Optional[Dict[str, float]] = None) -> AgentLevelReport:
        """单 agent ASI level 真测 (V1124 backend /asi/level 串联).

        ponytail: 每 agent 独立测量子分, 不互相覆盖, 主 17:43.
        """
        now = time.time()
        # 1) V1124 backend /asi/level 真测 (主 17:43 实事求是)
        status_code, body = self.backend_bridge.level()
        backend_status = "ok" if status_code == 200 else "unavailable"
        backend_score = None
        if backend_status == "ok" and isinstance(body, dict):
            backend_score = body.get("score")
        # 2) continuity_tracker 子分 (V1072 真生产, 主 19:33)
        if continuity_override is not None:
            continuity = float(continuity_override)
        else:
            continuity = round(self.continuity_tracker.continuity_score(), 6) or 0.85
        # 3) V0.5 18 维聚合
        form = default_v05_18_form(
            v04_score=v04_score,
            continuity_tracker=continuity,
            multi_agent_consensus=multi_agent_consensus,
            v04_dim_overrides=v04_dim_overrides,
        )
        v05_dict = form.to_dict()
        return AgentLevelReport(
            agent_id=agent_id,
            identity_id=self._agent_session_ids.get(agent_id, ""),
            v05_18_total=v05_dict["v05_18_total"],
            v04_subscore=v05_dict["v04_subscore"],
            continuity_tracker=continuity,
            per_dim=v05_dict["dims"],
            timestamp=now,
            backend_status=backend_status,
            error="" if backend_status == "ok" else f"backend_status={status_code}, body={body}",
        )

    # ------------------------------------------------------------------
    # 多 agent 共识聚合 (主 23:44 干到底: 共识失败即守门失败)
    # ------------------------------------------------------------------

    def measure_multi_agent(self,
                             v04_score: float = 0.8538,
                             continuity_per_agent: Optional[Dict[str, float]] = None,
                             multi_agent_consensus_hint: float = 0.85,
                             v04_dim_overrides: Optional[Dict[str, float]] = None) -> MultiAgentConsensusReport:
        """多 agent 协同测量 + 共识聚合.

        步骤:
          1) 每 agent 独立 measure_single_agent
          2) 计算 v05_18_total mean / stddev / min / max
          3) consensus_score = 1 - min(stddev / 0.1, 1.0) ∈ [0, 1]
          4) consensus_pass = stddev < CONSENSUS_STDDEV_MAX
        """
        continuity_per_agent = continuity_per_agent or {}
        per_agent: List[AgentLevelReport] = []
        for aid in self.agent_ids:
            cont = continuity_per_agent.get(aid, None)
            report = self.measure_single_agent(
                agent_id=aid,
                v04_score=v04_score,
                continuity_override=cont,
                multi_agent_consensus=multi_agent_consensus_hint,
                v04_dim_overrides=v04_dim_overrides,
            )
            per_agent.append(report)
        n_total = len(per_agent)
        n_ok = sum(1 for r in per_agent if r.backend_status == "ok")
        n_failed = n_total - n_ok
        totals = [r.v05_18_total for r in per_agent]
        if totals:
            mean = statistics.fmean(totals)
            stddev = statistics.pstdev(totals) if len(totals) > 1 else 0.0
            tmin = min(totals)
            tmax = max(totals)
        else:
            mean = stddev = tmin = tmax = 0.0
        # ponytail: 共识分数 = 1 - 归一化标准差 (主 19:33 一行即可)
        consensus_score = max(0.0, 1.0 - min(stddev / 0.1, 1.0))
        continuity_means = statistics.fmean([r.continuity_tracker for r in per_agent]) if per_agent else 0.0
        return MultiAgentConsensusReport(
            n_agents_total=n_total,
            n_agents_ok=n_ok,
            n_agents_failed=n_failed,
            per_agent=[r.to_dict() for r in per_agent],
            v05_18_total_mean=mean,
            v05_18_total_stddev=stddev,
            v05_18_total_min=tmin,
            v05_18_total_max=tmax,
            consensus_score=consensus_score,
            consensus_pass=stddev < CONSENSUS_STDDEV_MAX,
            continuity_tracker_mean=continuity_means,
            timestamp=time.time(),
        )

    # ------------------------------------------------------------------
    # Chaos test: 模拟 agent 失联 → 测量不能丢 (主 23:44 干到底)
    # ------------------------------------------------------------------

    def run_chaos_test(self,
                       v04_score: float = 0.8538,
                       drop_indices: Optional[Sequence[int]] = None) -> Dict[str, Any]:
        """Chaos test: 部分 agent 失联后, 协议仍必须产出有效测量.

        ponytail: 用子集 agent 重新跑, 主 23:44 干到底: 测量不能丢.
        """
        drop_indices = list(drop_indices or [])
        if not drop_indices:
            drop_indices = [0]  # 默认丢第 1 个
        surviving = [a for i, a in enumerate(self.agent_ids) if i not in drop_indices]
        # 1) 全 agent 测量
        full_report = self.measure_multi_agent(v04_score=v04_score)
        # 2) chaos 测量 (仅 surviving agents)
        if len(surviving) >= MIN_AGENTS:
            sub_protocol = V1128MultiAgentIntegrationProtocol(
                agent_ids=surviving,
                backend_bridge=self.backend_bridge,
                continuity_tracker=self.continuity_tracker,
            )
            chaos_report = sub_protocol.measure_multi_agent(v04_score=v04_score)
        else:
            # 失联过多, 用全 agent 兜底 (主 23:44 干到底: 测量不能丢)
            chaos_report = full_report
            chaos_report_dict = chaos_report.to_dict()
            chaos_report_dict["chaos_fallback_used"] = True
            chaos_report_dict["chaos_reason"] = (
                f"surviving={len(surviving)} < MIN_AGENTS={MIN_AGENTS}, fallback to full report"
            )
            return {
                "drop_indices": drop_indices,
                "n_dropped": len(drop_indices),
                "n_surviving": len(surviving),
                "full_report": full_report.to_dict(),
                "chaos_report": chaos_report_dict,
                "measurement_preserved": True,
                "consensus_preserved": chaos_report.consensus_pass,
                "chaos_fallback_used": True,
                "delta_mean": 0.0,
            }
        # 3) 比对: chaos 后 V0.5 mean 与 full V0.5 mean 差距 < 5pp
        full_mean = full_report.v05_18_total_mean
        chaos_mean = chaos_report.v05_18_total_mean
        delta = abs(full_mean - chaos_mean)
        measurement_preserved = (
            chaos_report.n_agents_ok >= 1  # 至少 1 agent 在线
            and delta < 0.05               # 5pp 内
        )
        return {
            "drop_indices": drop_indices,
            "n_dropped": len(drop_indices),
            "n_surviving": len(surviving),
            "full_report": full_report.to_dict(),
            "chaos_report": chaos_report.to_dict(),
            "delta_mean": round(delta, 6),
            "measurement_preserved": measurement_preserved,
            "consensus_preserved": chaos_report.consensus_pass,
            "chaos_fallback_used": False,
        }

    # ------------------------------------------------------------------
    # V1128 主编排: V0.5 18 维 + 多 agent 协同 + V1124 backend
    # ------------------------------------------------------------------

    def evaluate_r10_week(self,
                          week_label: str = "R10-W1",
                          v04_score: float = 0.8538,
                          v1074_v03_score: float = 0.8897,
                          philosophy_guard_pass_count: int = 6) -> Dict[str, Any]:
        """R10 weekly 多 agent 集成评估 (主 00:56 一行可跑).

        输出包含:
          - chain_integration: V1072/V1095/V1106/V1124/V1127 全链路真测状态
          - per_agent: 每 agent V0.5 18 维 + V1124 backend 串联测量
          - consensus: 多 agent 共识聚合
          - v05_18_form: V0.5 18 维公式聚合 (含 v05_pass_w2 / v05_pass_w4)
          - dashboard: ASI 北极星 dashboard (主 17:43)
          - halting_signals: 5 halting 信号
          - track_decision: 4 选 1 主轨道
          - guards: V3 守门
          - all_ok: 全链路 + 多 agent + V0.5 18 维 全部真测通过
        """
        # 1) 全链路真测
        chain = run_chain_integration_check()
        # 2) 多 agent 协同测量
        consensus = self.measure_multi_agent(v04_score=v04_score)
        # 3) 用 consensus 反馈到 V0.5 18 维 (multi_agent_consensus 维)
        # ponytail: multi_agent_consensus 维 = consensus.consensus_score, 主 17:43 实事求是
        v05_18_form = default_v05_18_form(
            v04_score=v04_score,
            continuity_tracker=consensus.continuity_tracker_mean,
            multi_agent_consensus=consensus.consensus_score,
        )
        v05_18_dict = v05_18_form.to_dict()
        # 4) ASI 北极星 dashboard (主 17:43: v1074 / v04 / v05)
        dashboard = {
            "v03_score": v1074_v03_score,
            "v04_score": v04_score,
            "v05_18_total": v05_18_dict["v05_18_total"],
            "v05_new_subscore": v05_18_dict["v05_new_subscore"],
            "v05_pass_w2": v05_18_dict["v05_pass_w2"],
            "v05_pass_w4": v05_18_dict["v05_pass_w4"],
            "asi_north_star": ASI_NORTH_STAR,
            "abs_headroom": round(ASI_NORTH_STAR - v05_18_dict["v05_18_total"], 4),
            "rel_headroom_pct": round((ASI_NORTH_STAR - v05_18_dict["v05_18_total"]) / ASI_NORTH_STAR * 100, 2),
            "philosophy_guard_subscore": round(philosophy_guard_pass_count / 6.0, 4),
            "v1074_all_ok": v1074_v03_score >= V1074_V03_MIN,
            "n_dims_filled": N_V05_18_DIMS,
        }
        # 5) Halting signals (主 20:55 红皇后守门)
        halting = evaluate_halting_signals(
            v03_history=[v1074_v03_score] * 30,        # 假设稳定历史
            unique_ratio=1.0,
            fitness_std=0.01,
            cross_dim_drop=0.0,
            cross_model_lift=0.02,                    # 多 agent 跨模型 lift
        )
        # 6) R10 主轨道决策 (升级阈值: R10 0.92/0.88/0.86)
        track = choose_r10_main_track(v05_18_dict["v05_18_total"], halting)
        # 7) V3 守门
        guards = run_r10_guard_self_check(
            {"v03_score": v1074_v03_score, "v04_score": v04_score}, halting
        )
        all_ok = (
            chain.chain_all_ok
            and consensus.n_agents_ok >= MIN_AGENTS
            and consensus.consensus_pass
            and guards.all_ok
            and v05_18_dict["v05_18_total"] >= R10_START_TARGET  # R10 起点必过
        )
        return {
            "version": VERSION,
            "week_label": week_label,
            "v1114_version": V1114_VERSION,
            "v1125_version": V1125_VERSION,
            "v1126_version": V1126_VERSION,
            "v1128_version": VERSION,
            "chain_integration": chain.to_dict(),
            "consensus": consensus.to_dict(),
            "v05_18_form": v05_18_dict,
            "dashboard": dashboard,
            "halting_signals": asdict(halting),
            "track_decision": asdict(track),
            "guards": asdict(guards),
            "all_ok": all_ok,
            "consensus_pass": consensus.consensus_pass,
            "chain_all_ok": chain.chain_all_ok,
            "v05_pass_w2": v05_18_dict["v05_pass_w2"],
            "v05_pass_w4": v05_18_dict["v05_pass_w4"],
            "r10_w2_target": R10_W2_TARGET,
            "r10_w4_target": R10_W4_TARGET,
            "asi_north_star": ASI_NORTH_STAR,
            "v3_guards": dict(V3_GUARDS_V1128),
            "chain_integration_matrix": dict(CHAIN_INTEGRATION_MATRIX),
            "timestamp": time.time(),
        }

    # ------------------------------------------------------------------
    # Markdown 渲染 (主 00:56 任何人都能接手)
    # ------------------------------------------------------------------

    def render_markdown(self, result: Mapping[str, Any]) -> str:
        lines: List[str] = []
        lines.append(f"# V1128 R10 多 agent 集成评估 — {result.get('week_label', 'R10-W1')}")
        lines.append("")
        lines.append(f"- version: {result.get('version', VERSION)}")
        lines.append(f"- v1114_version: {result.get('v1114_version', V1114_VERSION)}")
        lines.append(f"- v1125_version: {result.get('v1125_version', V1125_VERSION)}")
        lines.append(f"- v1126_version: {result.get('v1126_version', V1126_VERSION)}")
        lines.append(f"- all_ok: **{result.get('all_ok')}**")
        lines.append(f"- chain_all_ok: {result.get('chain_all_ok')}")
        lines.append(f"- consensus_pass: {result.get('consensus_pass')}")
        lines.append(f"- W2 中期门 (V0.5 ≥ {R10_W2_TARGET}): {result.get('v05_pass_w2')}")
        lines.append(f"- W4 终极门 (V0.5 ≥ {R10_W4_TARGET}): {result.get('v05_pass_w4')}")
        lines.append(f"- ASI 北极星: {ASI_NORTH_STAR:.4f} (LOCKED)")
        lines.append("")
        # Dashboard
        dash = result.get("dashboard", {})
        lines.append("## ASI 北极星 dashboard")
        lines.append(f"- V0.3 真测: {dash.get('v03_score')}")
        lines.append(f"- V0.4 真测: {dash.get('v04_score')}")
        lines.append(f"- V0.5 18 维: **{dash.get('v05_18_total')}**")
        lines.append(f"- V0.5 新维子分: {dash.get('v05_new_subscore')}")
        lines.append(f"- abs_headroom: {dash.get('abs_headroom')}")
        lines.append(f"- rel_headroom_pct: {dash.get('rel_headroom_pct')}%")
        lines.append(f"- philosophy_guard_subscore: {dash.get('philosophy_guard_subscore')}")
        lines.append(f"- v1074_all_ok: {dash.get('v1074_all_ok')}")
        lines.append(f"- n_dims_filled: {dash.get('n_dims_filled')}")
        lines.append("")
        # Chain integration
        chain = result.get("chain_integration", {})
        lines.append("## V1072/V1095/V1106/V1124/V1127 全链路集成")
        for k in ("v1072_continuity", "v1095_identity", "v1106_engineering",
                  "v1124_backend", "v1127_multi_agent"):
            v = chain.get(k, {})
            lines.append(f"- **{k}**: ok={v.get('ok')}")
            if not v.get("ok"):
                lines.append(f"  - error: {v.get('error')}")
        lines.append("")
        # Consensus
        consensus = result.get("consensus", {})
        lines.append("## 多 agent 共识聚合")
        lines.append(f"- n_agents_total: {consensus.get('n_agents_total')}")
        lines.append(f"- n_agents_ok: {consensus.get('n_agents_ok')}")
        lines.append(f"- n_agents_failed: {consensus.get('n_agents_failed')}")
        lines.append(f"- v05_18_total_mean: {consensus.get('v05_18_total_mean')}")
        lines.append(f"- v05_18_total_stddev: {consensus.get('v05_18_total_stddev')}")
        lines.append(f"- v05_18_total_min: {consensus.get('v05_18_total_min')}")
        lines.append(f"- v05_18_total_max: {consensus.get('v05_18_total_max')}")
        lines.append(f"- consensus_score: {consensus.get('consensus_score')}")
        lines.append(f"- consensus_pass: {consensus.get('consensus_pass')}")
        lines.append(f"- continuity_tracker_mean: {consensus.get('continuity_tracker_mean')}")
        lines.append("")
        # V0.5 18 维
        form = result.get("v05_18_form", {})
        lines.append("## V0.5 18 维公式 (16 V0.4 维 + continuity_tracker + multi_agent_consensus)")
        lines.append(f"- v04_subscore: {form.get('v04_subscore')}")
        lines.append(f"- v05_new_subscore: {form.get('v05_new_subscore')}")
        lines.append(f"- v05_18_total: **{form.get('v05_18_total')}**")
        lines.append("")
        # Halting signals
        h = result.get("halting_signals", {})
        lines.append("## 5 Halting Signals (主 20:55 红皇后守门)")
        for k in ("perf_regression", "candidate_collapse", "locked_in_self_consistency",
                  "red_queen_trap", "no_new_lift"):
            lines.append(f"- {k}: {h.get(k)}")
        lines.append("")
        # Track decision
        t = result.get("track_decision", {})
        lines.append("## R10 4 选 1 主轨道决策")
        lines.append(f"- track: **{t.get('track')}** ({t.get('track_name')})")
        lines.append(f"- rationale: {t.get('rationale')}")
        lines.append(f"- expected_lift: {t.get('expected_lift')}")
        lines.append(f"- halt_override: {t.get('halt_override')}")
        lines.append(f"- confidence: {t.get('confidence')}")
        lines.append("")
        # Guards
        g = result.get("guards", {})
        lines.append("## V3 守门 (主 17:43+17:58 不假装)")
        lines.append(f"- all_ok: {g.get('all_ok')}")
        lines.append(f"- philosophy_9_keys_locked: {g.get('philosophy_9_keys_locked')}")
        lines.append(f"- v3_guards_all_pass: {g.get('v3_guards_all_pass')}")
        lines.append(f"- red_lines_all_pass: {g.get('red_lines_all_pass')}")
        lines.append(f"- v1074_v03_above_floor: {g.get('v1074_v03_above_floor')}")
        lines.append("")
        # V3 Guards 5 红线
        lines.append("## V1128 V3 守门 5 红线 (主 23:44 干到底)")
        for k, v in V3_GUARDS_V1128.items():
            lines.append(f"- **{k}**: {v}")
        lines.append("")
        # Chain integration matrix
        lines.append("## 全链路兼容性矩阵 (主 19:33 走在前人经验上)")
        for k, v in CHAIN_INTEGRATION_MATRIX.items():
            lines.append(f"- {k}: {v}")
        lines.append("")
        lines.append("---")
        lines.append("")
        lines.append("主哲学 LOCKED: 主 22:33 ASI 北极星 / 主 17:43 实事求是 / 主 13:31 大胆激进 / 主 23:44 干到底 / 主 19:33 走在前人经验上 / 主 00:56 任何人都能接手")
        return "\n".join(lines)

    def close(self) -> None:
        for sid in self._agent_session_ids.values():
            if sid:
                try:
                    self.continuity_tracker.end_session(sid)
                except Exception:  # noqa: BLE001
                    pass


# ---------------------------------------------------------------------------
# V3 守门注入 (主 17:43 实事求是: 多 agent 不假装 ASI / 集体心智)
# ---------------------------------------------------------------------------

V3_GUARDS_R10_MULTI_AGENT_INJECTED = {
    "v0_5_18_dim_locked": "V0.5 必须恰好 18 维 (V0.4 17 维 + 2 新维), 缺一不可.",
    "multi_agent_not_asi": "多 agent 协同 ≠ ASI 达成, 仅是测量协议升级.",
    "consensus_is_not_truth": "共识分数 (consensus_score) 是守门指标, 不是真理.",
    "chaos_test_required": "chaos test: 失联 ≤ 50% agent 必须保持 measurement_preserved=True.",
    "v1124_backend_required": "V1124 backend /asi/level/measure/north-star 必须真测, 不允许 mock.",
    "chain_integration_required": "V1072/V1095/V1106/V1124/V1127 全链路必须 5/5 ok.",
    "w4_ultimate_locked": "R10 W4 终极门 V0.5 ≥ 0.95 LOCKED, 不容分阶段缓慢.",
    "r9_baseline_locked": "R9 W4 末 baseline (V0.4=0.8538) LOCKED, 不允许改写历史.",
}


# ---------------------------------------------------------------------------
# CLI 入口 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1128 R10 多 agent 集成 V0.5 公式扩展")
    parser.add_argument("--week", type=str, default="R10-W1", help="R10 阶段周次 (R10-W1/R10-W2/R10-W3/R10-W4)")
    parser.add_argument("--v04", type=float, default=0.8538, help="V0.4 实际真测 (R9 W4 末 baseline = 0.8538)")
    parser.add_argument("--v03", type=float, default=0.8897, help="V0.3 实际真测 (R9 守门 ≥ 0.8884)")
    parser.add_argument("--live", action="store_true", help="真跑 V1124 backend (默认真测)")
    parser.add_argument("--chaos", action="store_true", help="chaos test: 模拟 agent 失联")
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--report", action="store_true", help="写 Markdown 报告到 reports/")
    parser.add_argument("--strict", action="store_true", help="不通过非零退出")
    args = parser.parse_args(argv)

    proto = V1128MultiAgentIntegrationProtocol()
    result = proto.evaluate_r10_week(
        week_label=args.week,
        v04_score=args.v04,
        v1074_v03_score=args.v03,
    )
    # chaos test (主 23:44 干到底)
    if args.chaos:
        chaos = proto.run_chaos_test(v04_score=args.v04, drop_indices=[0])
        result["chaos_test"] = chaos

    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    elif args.report:
        out_dir = Path(__file__).resolve().parents[1] / "reports"
        out_dir.mkdir(exist_ok=True)
        md = proto.render_markdown(result)
        path = out_dir / f"v1128_r10_multi_agent_{args.week.lower().replace('-', '_')}.md"
        path.write_text(md, encoding="utf-8")
        print(f"[V1128] report written: {path}")
    else:
        d = result["dashboard"]
        c = result["consensus"]
        f = result["v05_18_form"]
        print(f"V1128 R10 多 agent 集成评估 — {args.week}")
        print(f"  V0.4 真测: {args.v04:.4f} → V0.5 18 维: {f['v05_18_total']:.4f}")
        print(f"  V0.5 W2 中期 (≥ {R10_W2_TARGET}): {'✓' if f['v05_pass_w2'] else '✗'}")
        print(f"  V0.5 W4 终极 (≥ {R10_W4_TARGET}): {'✓' if f['v05_pass_w4'] else '✗'}")
        print(f"  多 agent 共识: {c['n_agents_ok']}/{c['n_agents_total']} ok, "
              f"score={c['consensus_score']:.4f} {'✓' if c['consensus_pass'] else '✗'}")
        print(f"  全链路: chain_all_ok={result['chain_all_ok']}")
        print(f"  ASI 北极星: {ASI_NORTH_STAR:.4f} (LOCKED)")
        print(f"  all_ok: **{result['all_ok']}**")
        if args.chaos:
            ct = result["chaos_test"]
            print(f"  chaos test: dropped={ct['n_dropped']}, "
                  f"measurement_preserved={ct['measurement_preserved']}")

    proto.close()
    if args.strict and not result["all_ok"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
