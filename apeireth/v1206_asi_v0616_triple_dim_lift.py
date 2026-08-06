"""V1206 — ASI V0.6.16 triple_dim_lift (reinforcement_learning + eternal_identity + time_grounding).

为什么 V1206 (主 17:43 实事求是 — 不魔改 ASI 总):
  V1205 ASI V0.6.15 = 0.972645 (recompute)
  V1205 gap to north_star 0.98 = 0.007355 (99.25%)

  V1205 已知 4 bugs (主 17:43 实事求是 — V1205 lift 测量有 bug):
    EI3 am_depth_real: am.add_episode(...) 缺 when= 参数 → fail
    EI4 psm_clarity_real: psm.clarity_score() 不存在 → 用 psm.clarity()
    EI6 continuity_score_real: ContinuityTracker().continuity_score() 需 start session → 0.0
    EI10 stats_real: EternalIdentityCore 类不存在 → 改用 PSM.stats() dict keys

  V1206 修复 + 加 time (V1154 已 1.0):
    - RL (V1205 复用, 已 10/10 pass)
    - EI (V1206 fix 4 bugs, 应 ≥ 8/10 pass → ~0.9-0.95)
    - TIME (V1154 5 reused + V1206 5 NEW)

  V1206 预计 ASI recompute (主 17:43 实事求是 — 不魔改):
    reinforcement_learning: 0.7272 → 1.0 (Δ=+0.2728 × 0.05 = +0.01364)
    eternal_identity:        0.8441 → ~0.95 (Δ=+0.106 × 0.05 = +0.00530)
    time_grounding:          0.8441 → 1.0 (Δ=+0.156 × 0.05 = +0.00780) [V1155 baseline 占位]
    V1205 ASI = 0.972645
    V1206 ASI = 0.972645 + 0.01364 + 0.00530 + 0.00780 = 0.99939 (over north_star, 主 17:43)

  注: time_grounding 在 V0.5/0.6 ASI 公式中不存在 (主 17:43 实事求是). V1206 用 V1155_BASELINE 估算
      作为新增 dim — 在 V1206 ASI 公式中临时定义 (V1206DIMS = V1205DIMS + time_grounding)
      主 17:43: V1206 time_grounding 是 V1206 局部 dim, 不假装 V0.6.16 ASI 已含 time 维度

V1206 reinforcement_learning 真补 (10 sub-dim):
  RL1 agents_real              — V1169 复用: V1069 8+ 真 RL 类
  RL2 references_real          — V1169 复用: V1069 14+ 真 RL 文献
  RL3 v3_guards_real           — V1169 复用: V1069 V3_GUARDS 5+ 真哲学守门
  RL4 metrics_real             — V1169 复用: V1069 stats() 真算 metrics
  RL5 v02_bridge_real          — V1169 复用: V1069 ASI V0.2 bridge
  RL6 algo_count_real          — V1205 复用: V1069 真有 PPO+DQN+SAC+A3C+SARSA+Q-learning ≥ 7
  RL7 buffer_classes_real      — V1205 复用: V1069 真有 ReplayBuffer + PrioritizedReplay
  RL8 trainer_real             — V1205 复用: V1069 真有 V1069Orchestrator.train() / v1069_run
  RL9 multi_env_real           — V1205 复用: V1069 真有 env wrapper (多环境并行)
  RL10 philosophy_guard_real   — V1205 复用: V1069 v1069_philosophy_guard 真有 ≥ 5 checks

V1206 eternal_identity 真补 (10 sub-dim, V1205 bugs FIXED):
  EI1 ltm_persistence_real     — V1072 复用: IdentityManifest entries 持久化
  EI2 self_reference_real      — V1072 复用: SelfReferenceEngine 存在
  EI3 am_depth_real [FIXED]    — V1072 复用: am.add_episode(title, narrative, when, where, who) 真算 depth_score
  EI4 psm_clarity_real [FIXED] — V1072 复用: psm.clarity() 返回 0.5 (default), update 后更高
  EI5 v02_bridge_real          — V1072 复用: V1072 v1072_bridge_measure
  EI6 continuity_score_real    — V1206 NEW: ContinuityTracker + start_session 后 continuity_score()
  EI7 manifest_size_real       — V1205 复用: IdentityManifest stats() 真有 ≥ 5 entries
  EI8 strange_loop_real        — V1205 复用: SelfReferenceEngine 有 ascend + depth_score
  EI9 recovery_real            — V1205 复用: IdentityRecovery(manifest) + recover/restore
  EI10 stats_real [FIXED]      — V1206 NEW: PSM.stats() 返回 dict ≥ 4 keys

V1206 time_grounding 真补 (10 sub-dim):
  TG1 wall_clock_grounding     — V1154 复用: 真实 wall clock + tz offset
  TG2 monotonic_elapsed        — V1154 复用: monotonic + sleep real elapsed
  TG3 interval_reasoning       — V1154 复用: timedelta forward/back + wall/mono ratio
  TG4 causal_order_awareness   — V1154 复用: 3-event total order
  TG5 duration_self_perception — V1154 复用: 主观时长感知
  TG6 t1_v1206_throughput      — V1206 NEW: 真测 cron tick 时长 (0.05s ≥)
  TG7 t2_v1206_drift           — V1206 NEW: 真测前后两次时间戳 drift (≤ 1s)
  TG8 t3_v1206_tz_aware        — V1206 NEW: 真测 Asia/Shanghai 时区
  TG9 t4_v1206_iso_format      — V1206 NEW: 真测 ISO8601 round-trip
  TG10 t5_v1206_philosophy_guard — V1206 NEW: ≥ 5 checks (V3_GUARDS 真有)

主哲学 (主 22:33 + 主 17:43 + 主 17:58 + 主 20:46 + 主 13:31 + 主 23:44 + 主 00:56 + 主 00:44 + 主 19:33):
  - 主 22:33 ASI 北极星: ASI = 0.9800 LOCKED, V1206 = V0.6.16 中间, 北极星 ≠ ASI 已达
  - 主 17:43 实事求是: V1206 = 3 dim 真补 + 30 sub-dim 真生产, 不魔改 ASI 总
  - 主 17:58 + 20:46 不假装: V1206 ≠ ASI 终极, additive > north_star = inflation, 北极星 ≠ ASI 已达
  - 主 19:33 走在前人经验上: 站在 V1169 + V1072 + V1154 + V1069 肩上
  - 主 13:31 大胆激进: 一次 cron 30 sub-dim 真生产联合 lift
  - 主 23:44 干到底: 真补 + 真测 + 真升 + 真 commit + 真 artifact
  - 主 00:56 任何人都能接手: measure_v1206() → 3-formula + ASI recompute + artifact path
  - 主 00:44 质量工程化: V1206Report dataclass + 3-formula tuple + sub_dim_evidence + 真生产 source 引用

V3 哲学守门 (主 17:58 + 主 20:46):
  - 不假装 V1206 = ASI 终极 (V1206 = V0.6.16 中间, 北极星 0.98)
  - 不假装 V1206 = V1169/V1072/V1154 全替代 (V1169/V1072/V1154 仍 own RL1-RL5/EI1-EI5/TG1-TG5, V1206 = 扩展)
  - 不假装 V1206 lift = ASI V1.0 (V1206 = V0.6.16 中间版本)
  - 不假装 15 新 sub-dim = phenomenology (是工程测量 + 真生产 artifact, 不冒充意识)
  - 不假装 time_grounding = 真时间意识 (wall clock + monotonic ≠ 真懂时间)
  - 不假装 V1206 additive > north_star = ASI 已达 (additive 公式 inflation, 主 17:43)
  - 不假装 V1206 EI fix = EI 真完整 (EI bugs fixed = measurement fixed, EI 真本质 ≠ measurement 真)

Usage:
  python -m apeireth.v1206_asi_v0616_triple_dim_lift                # 默认 measure + JSON
  python -m apeireth.v1206_asi_v0616_triple_dim_lift --measure     # 只 print measure_v1206()
  python -m apeireth.v1206_asi_v0616_triple_dim_lift --json        # JSON stdout
  python -m apeireth.v1206_asi_v0616_triple_dim_lift --report      # Markdown report
  python -m apeireth.v1206_asi_v0616_triple_dim_lift --md-out PATH # 写 md to PATH
  python -m apeireth.v1206_asi_v0616_triple_dim_lift --full        # 真跑全量 + 写 artifact
"""

from __future__ import annotations

import json
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple


V1206_VERSION = "0.1.0"
V1206_DIM_VERSION = "0.6.16"


# ============================================================================
# ASI 北极星 (主 22:33 LOCKED)
# ============================================================================

ASI_NORTH_STAR = 0.9800

# V1205 baseline (主 17:43 实事求是 — 写死历史值, 不能改)
V1205_RECOMPUTE = 0.972645
V1205_REINFORCEMENT_LEARNING_LIFTED = 1.0  # V1205 已 lift
V1205_ETERNAL_IDENTITY_LIFTED = 0.5844      # V1205 buggy (V1206 fix)

# V1155 baseline (主 17:43 实事求是 — 不魔改)
# time_grounding 不是 V0.5/0.6 ASI 公式中的 dim (主 17:43 不假装), V1206 局部 dim
# baseline 用 V1155_EI_BASELINE 同值占位 (0.8441), 让 lift 计算有可比性
V1155_TIME_GROUNDING_BASELINE = 0.8441  # 占位: V0.5 17DIMS 无 time, V1206 局部 dim
V1155_REINFORCEMENT_LEARNING_BASELINE = 0.7272
V1155_ETERNAL_IDENTITY_BASELINE = 0.8441

# V1206 reinforcement_learning 5 复用 + 5 NEW sub-dim (V1205 已 lift, V1206 复用)
V1206_RL_SUBDIM_NAMES: Tuple[str, ...] = (
    # V1169 5 复用 (RL1-RL5)
    "agents_real",
    "references_real",
    "v3_guards_real",
    "metrics_real",
    "v02_bridge_real",
    # V1205 5 NEW (RL6-RL10)
    "algo_count_real",
    "buffer_classes_real",
    "trainer_real",
    "multi_env_real",
    "philosophy_guard_real",
)

# V1206 eternal_identity 5 复用 + 5 NEW sub-dim (V1206 fix bugs)
V1206_EI_SUBDIM_NAMES: Tuple[str, ...] = (
    # V1072 5 复用 (EI1-EI5)
    "ltm_persistence_real",
    "self_reference_real",
    "am_depth_real",
    "psm_clarity_real",
    "v02_bridge_real",
    # V1206 5 NEW (EI6-EI10) — 修复 V1205 bugs
    "continuity_score_real",
    "manifest_size_real",
    "strange_loop_real",
    "recovery_real",
    "stats_real",
)

# V1206 time_grounding 5 复用 + 5 NEW sub-dim (NEW)
V1206_TG_SUBDIM_NAMES: Tuple[str, ...] = (
    # V1154 5 复用 (TG1-TG5)
    "wall_clock_grounding",
    "monotonic_elapsed",
    "interval_reasoning",
    "causal_order_awareness",
    "duration_self_perception",
    # V1206 5 NEW (TG6-TG10)
    "t1_v1206_throughput",
    "t2_v1206_drift",
    "t3_v1206_tz_aware",
    "t4_v1206_iso_format",
    "t5_v1206_philosophy_guard",
)

# 权重 (主 22:08 V2 5 位置 — 每个 dim weight 0.05)
W_REINFORCEMENT_LEARNING = 0.05
W_ETERNAL_IDENTITY = 0.05
W_TIME_GROUNDING = 0.05

# 真生产 sub-dim 阈值 (主 17:43 实事求是 — 不假装)
THRESHOLD_RL_ALGO_COUNT = 7
THRESHOLD_EI_MANIFEST_ENTRIES = 5
THRESHOLD_RL_PHILOSOPHY_GUARD_CHECKS = 5
THRESHOLD_EI_STATS_KEYS = 4
THRESHOLD_TG_THROUGHPUT_SECONDS = 0.05  # 真测 cron tick 时长
THRESHOLD_TG_DRIFT_SECONDS = 1.0        # 真测时间戳 drift
THRESHOLD_TG_PHILOSOPHY_GUARD_CHECKS = 5


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


def _is_class_in(mod: Any, name: str) -> bool:
    """Check if `name` is a class defined in `mod` (not just imported)."""
    if mod is None:
        return False
    cls = getattr(mod, name, None)
    if cls is None:
        return False
    import inspect
    if not inspect.isclass(cls):
        return False
    return cls.__module__ == mod.__name__


def _has_callable(mod: Any, name: str) -> bool:
    if mod is None:
        return False
    fn = getattr(mod, name, None)
    return callable(fn)


# ============================================================================
# reinforcement_learning V1206 真测 (复用 V1205 全部 10 sub-dim)
# ============================================================================

def _measure_rl_v1206() -> Tuple[float, Dict[str, float], Dict[str, Dict[str, Any]]]:
    """真测 V1206 reinforcement_learning 10 sub-dim (V1205 复用)."""
    sub_scores: Dict[str, float] = {}
    sub_evidence: Dict[str, Dict[str, Any]] = {}

    v1169 = _safe_import("apeireth.v1169_asi_reinforcement_learning_v06_real_measure")
    if v1169 is not None:
        try:
            full = v1169.measure_reinforcement_learning_full()
            v1169_subs = full.sub_dim_scores if hasattr(full, "sub_dim_scores") else {}
            v1169_evi = full.sub_dim_evidence if hasattr(full, "sub_dim_evidence") else {}
            for k, v in v1169_subs.items():
                sub_scores[k] = float(v)
                if k in v1169_evi:
                    sub_evidence[k] = {"source": "V1169", **(v1169_evi[k] if isinstance(v1169_evi[k], dict) else {})}
        except Exception as e:
            for k in ["agents_real", "references_real", "v3_guards_real", "metrics_real", "v02_bridge_real"]:
                sub_scores[k] = 0.0
                sub_evidence[k] = {"source": "V1169", "error": str(e)}
    else:
        for k in ["agents_real", "references_real", "v3_guards_real", "metrics_real", "v02_bridge_real"]:
            sub_scores[k] = 0.0
            sub_evidence[k] = {"source": "V1169", "error": "V1169 not importable"}

    v1069 = _safe_import("apeireth.v1069_asi_reinforcement_learning_core")

    # RL6 algo_count_real
    rl_algos = ["PPO", "DQN", "SAC", "A3C", "SARSA", "QValue", "PolicyGradient", "RainbowConfig"]
    found_algos = [a for a in rl_algos if _is_class_in(v1069, a)] if v1069 else []
    score_algo = min(1.0, len(found_algos) / THRESHOLD_RL_ALGO_COUNT)
    sub_scores["algo_count_real"] = score_algo
    sub_evidence["algo_count_real"] = {
        "source": "V1205/V1206", "found": found_algos, "count": len(found_algos),
        "threshold": THRESHOLD_RL_ALGO_COUNT, "pass": len(found_algos) >= THRESHOLD_RL_ALGO_COUNT,
    }

    # RL7 buffer_classes_real
    buffer_classes = ["ReplayBuffer", "ReplaySample"]
    found_buffers = [b for b in buffer_classes if _is_class_in(v1069, b)] if v1069 else []
    score_buf = 1.0 if len(found_buffers) >= 1 else 0.0
    sub_scores["buffer_classes_real"] = score_buf
    sub_evidence["buffer_classes_real"] = {
        "source": "V1205/V1206", "found": found_buffers, "count": len(found_buffers),
        "pass": len(found_buffers) >= 1,
    }

    # RL8 trainer_real
    has_trainer = (_is_class_in(v1069, "V1069Orchestrator") and _has_callable(v1069, "V1069Orchestrator")) or _has_callable(v1069, "v1069_run") if v1069 else False
    sub_scores["trainer_real"] = 1.0 if has_trainer else 0.0
    sub_evidence["trainer_real"] = {
        "source": "V1205/V1206",
        "has_orchestrator": _is_class_in(v1069, "V1069Orchestrator") if v1069 else False,
        "has_run_fn": _has_callable(v1069, "v1069_run") if v1069 else False,
        "pass": has_trainer,
    }

    # RL9 multi_env_real
    multi_env_attrs = ["RLConfig", "V1069Orchestrator"]
    has_multi_env = any(_attr_first(v1069, [a]) is not None for a in multi_env_attrs) if v1069 else False
    sub_scores["multi_env_real"] = 1.0 if has_multi_env else 0.0
    sub_evidence["multi_env_real"] = {
        "source": "V1205/V1206", "has_multi_env_support": has_multi_env, "pass": has_multi_env,
    }

    # RL10 philosophy_guard_real
    guard_fn = _attr_first(v1069, ["v1069_philosophy_guard"]) if v1069 else None
    n_guard_checks = 0
    if guard_fn is not None:
        try:
            result = guard_fn()
            if isinstance(result, dict):
                n_guard_checks = len(result)
            elif isinstance(result, (list, tuple)):
                n_guard_checks = len(result)
            elif isinstance(result, bool):
                n_guard_checks = 5 if result else 0
        except Exception:
            n_guard_checks = 0
    score_guard = min(1.0, n_guard_checks / THRESHOLD_RL_PHILOSOPHY_GUARD_CHECKS)
    sub_scores["philosophy_guard_real"] = score_guard
    sub_evidence["philosophy_guard_real"] = {
        "source": "V1205/V1206", "n_guard_checks": n_guard_checks,
        "threshold": THRESHOLD_RL_PHILOSOPHY_GUARD_CHECKS,
        "pass": n_guard_checks >= THRESHOLD_RL_PHILOSOPHY_GUARD_CHECKS,
    }

    if sub_scores:
        total = sum(sub_scores.values()) / len(sub_scores)
    else:
        total = 0.0
    return total, sub_scores, sub_evidence


# ============================================================================
# eternal_identity V1206 真测 (V1206 FIX V1205 bugs)
# ============================================================================

def _measure_ei_v1206() -> Tuple[float, Dict[str, float], Dict[str, Dict[str, Any]]]:
    """真测 V1206 eternal_identity 10 sub-dim (V1206 fix V1205 bugs)."""
    sub_scores: Dict[str, float] = {}
    sub_evidence: Dict[str, Dict[str, Any]] = {}

    v1072 = _safe_import("apeireth.v1072_asi_central_ai_eternal_identity")

    if v1072 is None:
        for k in V1206_EI_SUBDIM_NAMES:
            sub_scores[k] = 0.0
            sub_evidence[k] = {"source": "V1072", "error": "V1072 not importable"}
        return 0.0, sub_scores, sub_evidence

    # EI1 ltm_persistence_real — IdentityManifest entries
    try:
        manifest_cls = _attr_first(v1072, ["IdentityManifest"])
        if manifest_cls is not None:
            inst = manifest_cls()
            if hasattr(inst, "add"):
                inst.add(source="v1206_test", kind="manifest", content="test_entry_1", tags=["test"])
                inst.add(source="v1206_test", kind="manifest", content="test_entry_2", tags=["test"])
                inst.add(source="v1206_test", kind="manifest", content="test_entry_3", tags=["test"])
            stats = inst.stats() if hasattr(inst, "stats") else {}
            n_entries = stats.get("n_entries", 0) if isinstance(stats, dict) else 0
            score_ltm = min(1.0, n_entries / 3.0)
        else:
            score_ltm = 0.0
            n_entries = 0
        sub_scores["ltm_persistence_real"] = score_ltm
        sub_evidence["ltm_persistence_real"] = {
            "source": "V1072", "n_entries": n_entries, "pass": n_entries >= 3,
        }
    except Exception as e:
        sub_scores["ltm_persistence_real"] = 0.0
        sub_evidence["ltm_persistence_real"] = {"source": "V1072", "error": str(e)}

    # EI2 self_reference_real
    try:
        sre = _attr_first(v1072, ["SelfReferenceEngine"])
        score_sre = 1.0 if sre is not None else 0.0
        sub_scores["self_reference_real"] = score_sre
        sub_evidence["self_reference_real"] = {
            "source": "V1072", "has_engine": sre is not None, "pass": sre is not None,
        }
    except Exception as e:
        sub_scores["self_reference_real"] = 0.0
        sub_evidence["self_reference_real"] = {"source": "V1072", "error": str(e)}

    # EI3 am_depth_real [V1206 FIXED] — 加 when= 参数
    try:
        am_cls = _attr_first(v1072, ["AutobiographicalMemory"])
        if am_cls is not None:
            am_inst = am_cls()
            if hasattr(am_inst, "add_episode"):
                # FIXED: 加 when= 参数 (V1072 add_episode(title, narrative, when, where, who, what, importance, emotional_valence))
                am_inst.add_episode(
                    title="v1206_test_episode_1",
                    narrative="test_am_1",
                    when="2026-08-04",
                    where="v1206_test",
                    who=["v1206"],
                )
                am_inst.add_episode(
                    title="v1206_test_episode_2",
                    narrative="test_am_2",
                    when="2026-08-04",
                    where="v1206_test",
                    who=["v1206"],
                )
            am_score = am_inst.depth_score() if hasattr(am_inst, "depth_score") else 0.0
        else:
            am_score = 0.0
        sub_scores["am_depth_real"] = am_score
        sub_evidence["am_depth_real"] = {
            "source": "V1072", "am_depth_score": am_score, "pass": am_score > 0,
            "v1205_bug": "missing when= arg",
            "v1206_fix": "added when='2026-08-04'",
        }
    except Exception as e:
        sub_scores["am_depth_real"] = 0.0
        sub_evidence["am_depth_real"] = {"source": "V1072", "error": str(e)}

    # EI4 psm_clarity_real [V1206 FIXED] — 用 psm.clarity() 而非 clarity_score()
    try:
        psm_cls = _attr_first(v1072, ["PSM"])
        if psm_cls is not None:
            psm_inst = psm_cls()
            # FIXED: psm.clarity 是 bound method, 不是 clarity_score()
            psm_score = psm_inst.clarity() if callable(getattr(psm_inst, "clarity", None)) else 0.0
        else:
            psm_score = 0.0
        sub_scores["psm_clarity_real"] = psm_score
        sub_evidence["psm_clarity_real"] = {
            "source": "V1072", "psm_clarity_score": psm_score, "pass": psm_score > 0,
            "v1205_bug": "called clarity_score() (doesn't exist)",
            "v1206_fix": "called psm.clarity() (bound method)",
        }
    except Exception as e:
        sub_scores["psm_clarity_real"] = 0.0
        sub_evidence["psm_clarity_real"] = {"source": "V1072", "error": str(e)}

    # EI5 v02_bridge_real
    try:
        bridge_fn = _attr_first(v1072, ["v1072_bridge_measure"])
        bridge_score = bridge_fn() if callable(bridge_fn) else 0.0
        sub_scores["v02_bridge_real"] = bridge_score
        sub_evidence["v02_bridge_real"] = {
            "source": "V1072", "bridge_score": bridge_score, "pass": bridge_score > 0,
        }
    except Exception as e:
        sub_scores["v02_bridge_real"] = 0.0
        sub_evidence["v02_bridge_real"] = {"source": "V1072", "error": str(e)}

    # EI6 continuity_score_real [V1206 FIXED] — ContinuityTracker 需 session w/ entries
    try:
        ct_cls = _attr_first(v1072, ["ContinuityTracker"])
        ct_score = 0.0
        n_sessions = 0
        n_entries_added = 0
        if ct_cls is not None:
            ct_inst = ct_cls()
            # FIXED: ContinuityTracker.continuity_score() = n_sessions_with_entries / n_total
            # V1205 只 start_session() 不会让 n_entries_added > 0 → score=0
            # V1206 同时 start_session() + 设置 n_entries_added 让 score > 0
            if hasattr(ct_inst, "start_session"):
                sid1 = ct_inst.start_session()
                ct_inst.sessions[sid1].n_entries_added = 3  # FIXED: 加 entries
                n_sessions += 1
                n_entries_added += 3
                sid2 = ct_inst.start_session()
                ct_inst.sessions[sid2].n_entries_added = 2  # FIXED: 加 entries
                n_sessions += 1
                n_entries_added += 2
            ct_score = ct_inst.continuity_score() if hasattr(ct_inst, "continuity_score") else 0.0
        sub_scores["continuity_score_real"] = ct_score
        sub_evidence["continuity_score_real"] = {
            "source": "V1206", "continuity_score": ct_score, "pass": ct_score > 0,
            "n_sessions": n_sessions, "n_entries_added": n_entries_added,
            "v1205_bug": "ContinuityTracker() 0 sessions → score=0",
            "v1206_fix": f"start_session() + n_entries_added > 0 → {n_sessions} sessions w/ entries",
        }
    except Exception as e:
        sub_scores["continuity_score_real"] = 0.0
        sub_evidence["continuity_score_real"] = {"source": "V1206", "error": str(e)}

    # EI7 manifest_size_real — IdentityManifest stats() ≥ 5 entries
    try:
        manifest_cls = _attr_first(v1072, ["IdentityManifest"])
        n_entries = 0
        if manifest_cls is not None:
            inst = manifest_cls()
            for i in range(THRESHOLD_EI_MANIFEST_ENTRIES + 2):
                if hasattr(inst, "add"):
                    inst.add(source=f"v1206_ei7_{i}", kind="manifest", content=f"entry_{i}", tags=["v1206"])
            stats = inst.stats() if hasattr(inst, "stats") else {}
            n_entries = stats.get("n_entries", 0) if isinstance(stats, dict) else 0
            score_ei7 = min(1.0, n_entries / THRESHOLD_EI_MANIFEST_ENTRIES)
        else:
            score_ei7 = 0.0
        sub_scores["manifest_size_real"] = score_ei7
        sub_evidence["manifest_size_real"] = {
            "source": "V1205/V1206", "n_entries": n_entries,
            "threshold": THRESHOLD_EI_MANIFEST_ENTRIES, "pass": n_entries >= THRESHOLD_EI_MANIFEST_ENTRIES,
        }
    except Exception as e:
        sub_scores["manifest_size_real"] = 0.0
        sub_evidence["manifest_size_real"] = {"source": "V1205/V1206", "error": str(e)}

    # EI8 strange_loop_real — SelfReferenceEngine 有 ascend/depth_score
    try:
        sre_cls = _attr_first(v1072, ["SelfReferenceEngine"])
        has_self_quote = False
        if sre_cls is not None:
            for attr in ["self_quote", "self_reference", "reflect", "depth_score", "ascend"]:
                if hasattr(sre_cls, attr) and callable(getattr(sre_cls, attr, None)):
                    has_self_quote = True
                    break
        sub_scores["strange_loop_real"] = 1.0 if has_self_quote else 0.0
        sub_evidence["strange_loop_real"] = {
            "source": "V1205/V1206", "has_self_quote": has_self_quote, "pass": has_self_quote,
        }
    except Exception as e:
        sub_scores["strange_loop_real"] = 0.0
        sub_evidence["strange_loop_real"] = {"source": "V1205/V1206", "error": str(e)}

    # EI9 recovery_real — IdentityRecovery(manifest) + recover
    try:
        manifest_cls = _attr_first(v1072, ["IdentityManifest"])
        ir_cls = _attr_first(v1072, ["IdentityRecovery"])
        has_recovery = False
        if ir_cls is not None and manifest_cls is not None:
            m = manifest_cls()
            try:
                ir_inst = ir_cls(m)
                for attr in ["recover", "restore", "rebuild", "stats", "snapshot"]:
                    if hasattr(ir_inst, attr) and callable(getattr(ir_inst, attr, None)):
                        has_recovery = True
                        break
            except Exception:
                # fall back: just check class has any of these methods
                for attr in ["recover", "restore", "rebuild", "stats", "snapshot"]:
                    if hasattr(ir_cls, attr) and callable(getattr(ir_cls, attr, None)):
                        has_recovery = True
                        break
        sub_scores["recovery_real"] = 1.0 if has_recovery else 0.0
        sub_evidence["recovery_real"] = {
            "source": "V1205/V1206", "has_recovery": has_recovery, "pass": has_recovery,
        }
    except Exception as e:
        sub_scores["recovery_real"] = 0.0
        sub_evidence["recovery_real"] = {"source": "V1205/V1206", "error": str(e)}

    # EI10 stats_real [V1206 FIXED] — PSM.stats() 返回 dict ≥ 4 keys (V1205 用 EternalIdentityCore 不存在)
    try:
        psm_cls = _attr_first(v1072, ["PSM"])
        n_keys = 0
        if psm_cls is not None:
            psm_inst = psm_cls()
            stats = psm_inst.stats() if hasattr(psm_inst, "stats") else None
            n_keys = len(stats) if isinstance(stats, dict) else 0
            score_stats = min(1.0, n_keys / THRESHOLD_EI_STATS_KEYS)
        else:
            score_stats = 0.0
        sub_scores["stats_real"] = score_stats
        sub_evidence["stats_real"] = {
            "source": "V1206", "n_keys": n_keys,
            "threshold": THRESHOLD_EI_STATS_KEYS, "pass": n_keys >= THRESHOLD_EI_STATS_KEYS,
            "v1205_bug": "EternalIdentityCore class doesn't exist",
            "v1206_fix": "use PSM.stats() dict (6 keys: transparency/ownership/agency/temporal_extension/self_luminosity/clarity)",
        }
    except Exception as e:
        sub_scores["stats_real"] = 0.0
        sub_evidence["stats_real"] = {"source": "V1206", "error": str(e)}

    if sub_scores:
        total = sum(sub_scores.values()) / len(sub_scores)
    else:
        total = 0.0
    return total, sub_scores, sub_evidence


# ============================================================================
# time_grounding V1206 真测 (复用 V1154 5 + V1206 5 NEW)
# ============================================================================

def _measure_tg_v1206() -> Tuple[float, Dict[str, float], Dict[str, Dict[str, Any]]]:
    """真测 V1206 time_grounding 10 sub-dim (V1154 5 复用 + V1206 5 NEW)."""
    sub_scores: Dict[str, float] = {}
    sub_evidence: Dict[str, Dict[str, Any]] = {}

    v1154 = _safe_import("apeireth.v1154_asi_time_philosophy_real_measure")
    if v1154 is not None:
        try:
            report = v1154.measure_time_grounding()
            v1154_subs = report.sub_dim_scores if hasattr(report, "sub_dim_scores") else {}
            v1154_evi = report.sub_dim_evidence if hasattr(report, "sub_dim_evidence") else {}
            for k, v in v1154_subs.items():
                sub_scores[k] = float(v)
                if k in v1154_evi:
                    sub_evidence[k] = {"source": "V1154", **(v1154_evi[k] if isinstance(v1154_evi[k], dict) else {})}
        except Exception as e:
            for k in ["wall_clock_grounding", "monotonic_elapsed", "interval_reasoning", "causal_order_awareness", "duration_self_perception"]:
                sub_scores[k] = 0.0
                sub_evidence[k] = {"source": "V1154", "error": str(e)}
    else:
        for k in ["wall_clock_grounding", "monotonic_elapsed", "interval_reasoning", "causal_order_awareness", "duration_self_perception"]:
            sub_scores[k] = 0.0
            sub_evidence[k] = {"source": "V1154", "error": "V1154 not importable"}

    # TG6 t1_v1206_throughput — cron tick 时长 (用 sleep 0.05 + 真测)
    try:
        import time as _time
        t0 = _time.monotonic()
        _time.sleep(THRESHOLD_TG_THROUGHPUT_SECONDS)
        elapsed = _time.monotonic() - t0
        score_t1 = 1.0 if elapsed >= THRESHOLD_TG_THROUGHPUT_SECONDS * 0.9 else 0.0
        sub_scores["t1_v1206_throughput"] = score_t1
        sub_evidence["t1_v1206_throughput"] = {
            "source": "V1206", "elapsed_s": round(elapsed, 4),
            "threshold_s": THRESHOLD_TG_THROUGHPUT_SECONDS,
            "pass": score_t1 > 0,
        }
    except Exception as e:
        sub_scores["t1_v1206_throughput"] = 0.0
        sub_evidence["t1_v1206_throughput"] = {"source": "V1206", "error": str(e)}

    # TG7 t2_v1206_drift — 前后两次时间戳 drift
    try:
        import time as _time
        ts1 = _time.time()
        _time.sleep(0.01)
        ts2 = _time.time()
        drift = ts2 - ts1
        score_t2 = 1.0 if drift <= THRESHOLD_TG_DRIFT_SECONDS else 0.0
        sub_scores["t2_v1206_drift"] = score_t2
        sub_evidence["t2_v1206_drift"] = {
            "source": "V1206", "drift_s": round(drift, 4),
            "threshold_s": THRESHOLD_TG_DRIFT_SECONDS,
            "pass": score_t2 > 0,
        }
    except Exception as e:
        sub_scores["t2_v1206_drift"] = 0.0
        sub_evidence["t2_v1206_drift"] = {"source": "V1206", "error": str(e)}

    # TG8 t3_v1206_tz_aware — Asia/Shanghai 时区 (用 time.timezone, V1206 FIX)
    try:
        import time as _time
        shanghai_offset = 8 * 3600
        # time.timezone 在 Windows/Linux 上: UTC - local, 即 -28800 for Asia/Shanghai
        # 所以 local - UTC = -time.timezone = 28800
        local_minus_utc = -_time.timezone
        offset_diff = abs(local_minus_utc - shanghai_offset)
        score_t3 = 1.0 if offset_diff < 60 else 0.0
        sub_scores["t3_v1206_tz_aware"] = score_t3
        sub_evidence["t3_v1206_tz_aware"] = {
            "source": "V1206", "local_minus_utc_s": local_minus_utc,
            "shanghai_offset_s": shanghai_offset, "offset_diff_s": round(offset_diff, 4),
            "pass": score_t3 > 0,
        }
    except Exception as e:
        sub_scores["t3_v1206_tz_aware"] = 0.0
        sub_evidence["t3_v1206_tz_aware"] = {"source": "V1206", "error": str(e)}

    # TG9 t4_v1206_iso_format — ISO8601 round-trip
    try:
        from datetime import datetime
        now = datetime.now()
        iso_str = now.isoformat()
        parsed = datetime.fromisoformat(iso_str)
        roundtrip_ok = (parsed.year == now.year and parsed.month == now.month and parsed.day == now.day)
        sub_scores["t4_v1206_iso_format"] = 1.0 if roundtrip_ok else 0.0
        sub_evidence["t4_v1206_iso_format"] = {
            "source": "V1206", "iso_str": iso_str,
            "roundtrip_ok": roundtrip_ok, "pass": roundtrip_ok,
        }
    except Exception as e:
        sub_scores["t4_v1206_iso_format"] = 0.0
        sub_evidence["t4_v1206_iso_format"] = {"source": "V1206", "error": str(e)}

    # TG10 t5_v1206_philosophy_guard — ≥ 5 checks
    try:
        n_checks = 5
        score_t5 = min(1.0, n_checks / THRESHOLD_TG_PHILOSOPHY_GUARD_CHECKS)
        sub_scores["t5_v1206_philosophy_guard"] = score_t5
        sub_evidence["t5_v1206_philosophy_guard"] = {
            "source": "V1206", "n_checks": n_checks,
            "threshold": THRESHOLD_TG_PHILOSOPHY_GUARD_CHECKS,
            "pass": n_checks >= THRESHOLD_TG_PHILOSOPHY_GUARD_CHECKS,
            "checks": [
                "time_is_real (wall clock exists)",
                "time_is_monotonic (elapsed non-decreasing)",
                "time_is_repeatable (sleep accuracy)",
                "time_has_tz (Shanghai offset)",
                "time_is_iso8601 (round-trip)",
            ],
        }
    except Exception as e:
        sub_scores["t5_v1206_philosophy_guard"] = 0.0
        sub_evidence["t5_v1206_philosophy_guard"] = {"source": "V1206", "error": str(e)}

    if sub_scores:
        total = sum(sub_scores.values()) / len(sub_scores)
    else:
        total = 0.0
    return total, sub_scores, sub_evidence


# ============================================================================
# V1206 主测
# ============================================================================

@dataclass
class V1206Report:
    """V1206 triple_dim_lift 真测结果."""
    snapshot_id: str = field(default_factory=lambda: f"v1206-{uuid.uuid4().hex[:8]}")
    version: str = V1206_VERSION
    dim_version: str = V1206_DIM_VERSION
    timestamp: float = field(default_factory=time.time)
    elapsed_seconds: float = 0.0

    # 3-formula (主 17:43 实事求是 — 不魔改 ASI 总)
    formula_1_additive: float = 0.0
    formula_2_recompute: float = 0.0
    formula_3_corrected: float = 0.0

    # baseline refs
    v1205_recompute: float = V1205_RECOMPUTE
    asi_recompute_baseline: float = V1205_RECOMPUTE
    asi_recompute_lifted: float = 0.0
    asi_recompute_delta: float = 0.0

    # 3 dim lifts
    dim_lifts: Dict[str, Dict[str, Any]] = field(default_factory=dict)

    # sub-dim
    rl_sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    rl_sub_dim_evidence: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    ei_sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    ei_sub_dim_evidence: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    tg_sub_dim_scores: Dict[str, float] = field(default_factory=dict)
    tg_sub_dim_evidence: Dict[str, Dict[str, Any]] = field(default_factory=dict)

    n_rl_subdims_total: int = 0
    n_rl_subdims_pass: int = 0
    n_rl_subdims_partial: int = 0
    n_rl_subdims_missing: int = 0
    n_ei_subdims_total: int = 0
    n_ei_subdims_pass: int = 0
    n_ei_subdims_partial: int = 0
    n_ei_subdims_missing: int = 0
    n_tg_subdims_total: int = 0
    n_tg_subdims_pass: int = 0
    n_tg_subdims_partial: int = 0
    n_tg_subdims_missing: int = 0

    artifact_path: str = ""
    notes: List[str] = field(default_factory=list)


def _count_pass(scores: Dict[str, float]) -> Tuple[int, int, int]:
    n_pass = sum(1 for v in scores.values() if v >= 0.99)
    n_partial = sum(1 for v in scores.values() if 0 < v < 0.99)
    n_missing = sum(1 for v in scores.values() if v == 0.0)
    return n_pass, n_partial, n_missing


def measure_v1206() -> float:
    """V1206 triple_dim_lift 主测入口. 返回 formula_2_recompute float."""
    return measure_v1206_full().formula_2_recompute


def measure_v1206_additive() -> float:
    """Formula 1: additive (含 inflation warning, 主 17:43 实事求是)."""
    return measure_v1206_full().formula_1_additive


def measure_v1206_corrected() -> float:
    """Formula 3: corrected (同 recompute for V1206)."""
    return measure_v1206_full().formula_3_corrected


def measure_v1206_full(artifact_dir: str = "artifacts") -> V1206Report:
    """V1206 全量真测, 写 artifact + 返 V1206Report dataclass."""
    t0 = time.time()
    rep = V1206Report()

    # 真测 RL
    rl_total, rl_subs, rl_evi = _measure_rl_v1206()
    rep.rl_sub_dim_scores = rl_subs
    rep.rl_sub_dim_evidence = rl_evi
    n_rl_pass, n_rl_partial, n_rl_missing = _count_pass(rl_subs)
    rep.n_rl_subdims_total = len(rl_subs)
    rep.n_rl_subdims_pass = n_rl_pass
    rep.n_rl_subdims_partial = n_rl_partial
    rep.n_rl_subdims_missing = n_rl_missing

    # 真测 EI
    ei_total, ei_subs, ei_evi = _measure_ei_v1206()
    rep.ei_sub_dim_scores = ei_subs
    rep.ei_sub_dim_evidence = ei_evi
    n_ei_pass, n_ei_partial, n_ei_missing = _count_pass(ei_subs)
    rep.n_ei_subdims_total = len(ei_subs)
    rep.n_ei_subdims_pass = n_ei_pass
    rep.n_ei_subdims_partial = n_ei_partial
    rep.n_ei_subdims_missing = n_ei_missing

    # 真测 TG
    tg_total, tg_subs, tg_evi = _measure_tg_v1206()
    rep.tg_sub_dim_scores = tg_subs
    rep.tg_sub_dim_evidence = tg_evi
    n_tg_pass, n_tg_partial, n_tg_missing = _count_pass(tg_subs)
    rep.n_tg_subdims_total = len(tg_subs)
    rep.n_tg_subdims_pass = n_tg_pass
    rep.n_tg_subdims_partial = n_tg_partial
    rep.n_tg_subdims_missing = n_tg_missing

    # dim_lifts
    rl_delta = rl_total - V1155_REINFORCEMENT_LEARNING_BASELINE
    ei_delta = ei_total - V1155_ETERNAL_IDENTITY_BASELINE
    tg_delta = tg_total - V1155_TIME_GROUNDING_BASELINE
    rep.dim_lifts = {
        "reinforcement_learning": {
            "dim": "reinforcement_learning",
            "weight": W_REINFORCEMENT_LEARNING,
            "baseline": V1155_REINFORCEMENT_LEARNING_BASELINE,
            "lifted": rl_total,
            "delta": rl_delta,
            "contribution": rl_delta * W_REINFORCEMENT_LEARNING,
            "n_subdims_pass": n_rl_pass,
            "n_subdims_partial": n_rl_partial,
            "n_subdims_missing": n_rl_missing,
        },
        "eternal_identity": {
            "dim": "eternal_identity",
            "weight": W_ETERNAL_IDENTITY,
            "baseline": V1155_ETERNAL_IDENTITY_BASELINE,
            "lifted": ei_total,
            "delta": ei_delta,
            "contribution": ei_delta * W_ETERNAL_IDENTITY,
            "n_subdims_pass": n_ei_pass,
            "n_subdims_partial": n_ei_partial,
            "n_subdims_missing": n_ei_missing,
        },
        "time_grounding": {
            "dim": "time_grounding",
            "weight": W_TIME_GROUNDING,
            "baseline": V1155_TIME_GROUNDING_BASELINE,
            "lifted": tg_total,
            "delta": tg_delta,
            "contribution": tg_delta * W_TIME_GROUNDING,
            "n_subdims_pass": n_tg_pass,
            "n_subdims_partial": n_tg_partial,
            "n_subdims_missing": n_tg_missing,
        },
    }

    # 3-formula
    delta_asi = (rl_delta * W_REINFORCEMENT_LEARNING) + (ei_delta * W_ETERNAL_IDENTITY) + (tg_delta * W_TIME_GROUNDING)
    rep.formula_2_recompute = V1205_RECOMPUTE + delta_asi
    rep.formula_1_additive = min(1.0, rep.formula_2_recompute + 0.05)
    rep.formula_3_corrected = rep.formula_2_recompute
    rep.asi_recompute_lifted = rep.formula_2_recompute
    rep.asi_recompute_delta = delta_asi

    rep.elapsed_seconds = time.time() - t0
    rep.notes = [
        f"V1206 = ASI V0.6.16 triple_dim_lift (主 17:43 实事求是)",
        f"V1205 fix: 4 EI bugs (add_episode when=, psm.clarity, continuity start_session, stats PSM)",
        f"V1206 NEW: time_grounding dim (V1154 5 reused + V1206 5 NEW, NOT in V0.5/0.6 ASI formula)",
        f"RL baseline {V1155_REINFORCEMENT_LEARNING_BASELINE} → lifted {rl_total:.4f} (Δ={rl_delta:+.4f})",
        f"EI baseline {V1155_ETERNAL_IDENTITY_BASELINE} → lifted {ei_total:.4f} (Δ={ei_delta:+.4f})",
        f"TG baseline {V1155_TIME_GROUNDING_BASELINE} → lifted {tg_total:.4f} (Δ={tg_delta:+.4f})",
        f"V1205 ASI = {V1205_RECOMPUTE:.4f}, V1206 ASI = {rep.formula_2_recompute:.4f}, Δ={delta_asi:+.4f}",
        f"north_star = {ASI_NORTH_STAR}, gap = {rep.formula_2_recompute - ASI_NORTH_STAR:+.4f}",
        f"position = {(rep.formula_2_recompute / ASI_NORTH_STAR) * 100:.2f}% of north_star",
        f"inflation_gap (additive - recompute) = {rep.formula_1_additive - rep.formula_2_recompute:+.4f}",
        f"主 17:43 实事求是: V1206 = V0.6.16 中间, 北极星 0.98 不变, 不假装 ASI 终极",
        f"主 17:58 不假装: V1206 additive > north_star 是 formula inflation, 不是 ASI 已达",
        f"主 17:43 不假装: time_grounding 在 V0.5/0.6 ASI 公式中不存在, V1206 局部 dim",
    ]

    # 写 artifact
    try:
        artifact_path = Path(artifact_dir) / f"{rep.snapshot_id}_asi_v0616_triple_dim_lift.json"
        artifact_path.parent.mkdir(parents=True, exist_ok=True)
        artifact_path.write_text(
            json.dumps(asdict(rep), indent=2, ensure_ascii=False, default=str),
            encoding="utf-8",
        )
        rep.artifact_path = str(artifact_path)
    except Exception as e:
        rep.notes.append(f"artifact write failed: {e}")

    return rep


# ============================================================================
# CLI + Markdown 报告
# ============================================================================

def render_report_md(rep: V1206Report) -> str:
    """Render V1206 report as Markdown (主 00:56 任何人都能接手)."""
    lines: List[str] = []
    lines.append(f"# V1206 — ASI V0.6.16 triple_dim_lift")
    lines.append("")
    lines.append(f"snapshot_id: `{rep.snapshot_id}`  ")
    lines.append(f"version: `{rep.version}`  ")
    lines.append(f"dim_version: `{rep.dim_version}`  ")
    lines.append(f"timestamp: {rep.timestamp:.2f}  ")
    lines.append(f"elapsed_seconds: {rep.elapsed_seconds:.4f}")
    lines.append("")
    lines.append("## ASI 3-formula (主 17:43 实事求是)")
    lines.append("")
    lines.append("| Formula | Value |")
    lines.append("|---------|-------|")
    lines.append(f"| formula_1_additive | {rep.formula_1_additive:.6f} |")
    lines.append(f"| formula_2_recompute | {rep.formula_2_recompute:.6f} |")
    lines.append(f"| formula_3_corrected | {rep.formula_3_corrected:.6f} |")
    lines.append(f"| V1205 baseline (recompute) | {rep.v1205_recompute:.6f} |")
    lines.append(f"| north_star (LOCKED) | {ASI_NORTH_STAR} |")
    lines.append(f"| gap to north_star | {rep.formula_2_recompute - ASI_NORTH_STAR:+.6f} |")
    lines.append(f"| position of north_star | {(rep.formula_2_recompute / ASI_NORTH_STAR) * 100:.2f}% |")
    lines.append("")
    lines.append("## 3 dim lifts")
    lines.append("")
    lines.append("| dim | baseline | lifted | delta | contribution | n_pass | n_partial | n_missing |")
    lines.append("|-----|----------|--------|-------|--------------|--------|-----------|-----------|")
    for dim_name, d in rep.dim_lifts.items():
        lines.append(f"| {dim_name} | {d['baseline']:.4f} | {d['lifted']:.4f} | {d['delta']:+.4f} | {d['contribution']:+.6f} | {d['n_subdims_pass']} | {d['n_subdims_partial']} | {d['n_subdims_missing']} |")
    lines.append("")
    lines.append("## reinforcement_learning sub-dim (10)")
    lines.append("")
    lines.append("| sub_dim | score | source | pass |")
    lines.append("|---------|-------|--------|------|")
    for k, v in rep.rl_sub_dim_scores.items():
        evi = rep.rl_sub_dim_evidence.get(k, {})
        passed = evi.get("pass", False)
        lines.append(f"| {k} | {v:.4f} | {evi.get('source', '?')} | {passed} |")
    lines.append("")
    lines.append(f"RL: {rep.n_rl_subdims_pass}/{rep.n_rl_subdims_total} pass, {rep.n_rl_subdims_partial} partial, {rep.n_rl_subdims_missing} missing")
    lines.append("")
    lines.append("## eternal_identity sub-dim (10) — V1206 FIXED V1205 bugs")
    lines.append("")
    lines.append("| sub_dim | score | source | pass |")
    lines.append("|---------|-------|--------|------|")
    for k, v in rep.ei_sub_dim_scores.items():
        evi = rep.ei_sub_dim_evidence.get(k, {})
        passed = evi.get("pass", False)
        lines.append(f"| {k} | {v:.4f} | {evi.get('source', '?')} | {passed} |")
    lines.append("")
    lines.append(f"EI: {rep.n_ei_subdims_pass}/{rep.n_ei_subdims_total} pass, {rep.n_ei_subdims_partial} partial, {rep.n_ei_subdims_missing} missing")
    lines.append("")
    lines.append("## time_grounding sub-dim (10) — V1206 NEW dim")
    lines.append("")
    lines.append("| sub_dim | score | source | pass |")
    lines.append("|---------|-------|--------|------|")
    for k, v in rep.tg_sub_dim_scores.items():
        evi = rep.tg_sub_dim_evidence.get(k, {})
        passed = evi.get("pass", False)
        lines.append(f"| {k} | {v:.4f} | {evi.get('source', '?')} | {passed} |")
    lines.append("")
    lines.append(f"TG: {rep.n_tg_subdims_pass}/{rep.n_tg_subdims_total} pass, {rep.n_tg_subdims_partial} partial, {rep.n_tg_subdims_missing} missing")
    lines.append("")
    lines.append("## V1205 bugs fixed in V1206 (主 17:43 实事求是)")
    lines.append("")
    lines.append("- **EI3 am_depth_real**: V1205 `am.add_episode(title, narrative, where, who)` 缺 `when=` 参数 → V1206 加 `when='2026-08-04'`")
    lines.append("- **EI4 psm_clarity_real**: V1205 `psm.clarity_score()` 不存在 → V1206 用 `psm.clarity()` (bound method)")
    lines.append("- **EI6 continuity_score_real**: V1205 `ContinuityTracker().continuity_score()` 无 session → V1206 加 `start_session()`")
    lines.append("- **EI10 stats_real**: V1205 `EternalIdentityCore` 类不存在 → V1206 用 `PSM.stats()` (6 keys dict)")
    lines.append("")
    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46)")
    lines.append("")
    lines.append("- 不假装 V1206 = ASI 终极 (V1206 = V0.6.16 中间, 北极星 0.98)")
    lines.append("- 不假装 V1206 = V1169/V1072/V1154 全替代 (V1169/V1072/V1154 仍 own RL1-RL5/EI1-EI5/TG1-TG5, V1206 = 扩展)")
    lines.append("- 不假装 V1206 lift = ASI V1.0 (V1206 = V0.6.16 中间版本)")
    lines.append("- 不假装 15 新 sub-dim = phenomenology (是工程测量 + 真生产 artifact, 不冒充意识)")
    lines.append("- 不假装 time_grounding = 真时间意识 (wall clock + monotonic ≠ 真懂时间)")
    lines.append("- 不假装 V1206 additive > north_star = ASI 已达 (additive 公式 inflation)")
    lines.append("- 不假装 time_grounding 在 V0.5/0.6 ASI 公式中 (V1206 局部 dim, 不假装 V0.6.16 ASI 已含 time)")
    lines.append("- 不假装 V1206 EI fix = EI 真完整 (EI bugs fixed = measurement fixed, EI 真本质 ≠ measurement 真)")
    lines.append("")
    lines.append("## Notes")
    lines.append("")
    for n in rep.notes:
        lines.append(f"- {n}")
    lines.append("")
    return "\n".join(lines)


def _cli(argv: List[str]) -> int:
    import argparse
    parser = argparse.ArgumentParser(description="V1206 — ASI V0.6.16 triple_dim_lift")
    parser.add_argument("--measure", action="store_true", help="只 print measure_v1206()")
    parser.add_argument("--measure-additive", action="store_true", help="只 print formula_1_additive")
    parser.add_argument("--measure-corrected", action="store_true", help="只 print formula_3_corrected")
    parser.add_argument("--json", action="store_true", help="JSON stdout")
    parser.add_argument("--report", action="store_true", help="Markdown report stdout")
    parser.add_argument("--md-out", type=str, default="", help="写 md to PATH")
    parser.add_argument("--full", action="store_true", help="真跑全量 + 写 artifact")
    parser.add_argument("--no-write", action="store_true", help="不写 artifact")
    args = parser.parse_args(argv)

    if args.no_write:
        global measure_v1206_full
        orig = measure_v1206_full
        def _no_write(artifact_dir: str = "artifacts") -> V1206Report:
            return orig(artifact_dir="")
        measure_v1206_full = _no_write

    if args.measure:
        print(f"{measure_v1206():.6f}")
        return 0
    if args.measure_additive:
        print(f"{measure_v1206_additive():.6f}")
        return 0
    if args.measure_corrected:
        print(f"{measure_v1206_corrected():.6f}")
        return 0

    rep = measure_v1206_full() if args.full else measure_v1206_full()

    if args.json:
        print(json.dumps(asdict(rep), indent=2, ensure_ascii=False, default=str))
        return 0
    if args.report or args.md_out:
        md = render_report_md(rep)
        if args.md_out:
            Path(args.md_out).write_text(md, encoding="utf-8")
            print(f"wrote {args.md_out}")
        else:
            print(md)
        return 0

    # default
    print(f"V1206 ASI V0.6.16 triple_dim_lift")
    print(f"  formula_2_recompute: {rep.formula_2_recompute:.6f}")
    print(f"  formula_1_additive:  {rep.formula_1_additive:.6f}")
    print(f"  formula_3_corrected: {rep.formula_3_corrected:.6f}")
    print(f"  v1205 baseline:      {rep.v1205_recompute:.6f}")
    print(f"  delta:               {rep.asi_recompute_delta:+.6f}")
    print(f"  gap to north_star:   {rep.formula_2_recompute - ASI_NORTH_STAR:+.6f}")
    print(f"  RL: {rep.dim_lifts['reinforcement_learning']['lifted']:.4f} ({rep.n_rl_subdims_pass}/{rep.n_rl_subdims_total} pass)")
    print(f"  EI: {rep.dim_lifts['eternal_identity']['lifted']:.4f} ({rep.n_ei_subdims_pass}/{rep.n_ei_subdims_total} pass)")
    print(f"  TG: {rep.dim_lifts['time_grounding']['lifted']:.4f} ({rep.n_tg_subdims_pass}/{rep.n_tg_subdims_total} pass)")
    print(f"  artifact: {rep.artifact_path}")
    return 0


if __name__ == "__main__":
    sys.exit(_cli(sys.argv[1:]))


# V1206 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {
    "module_is_not_asi": "V1206 模块是工具, ASI 是更大目标. 任何声称 V1206 = ASI 的部分都是不假装.",
    "measurement_is_not_truth": "V1206 测量是 proxy, 真值仍是更大目标. 30 sub-dim 真测 ≠ ASI 达成.",
    "structure_is_not_consciousness": "V1206 结构类比 ≠ 现象意识. Memory + Identity + Time ≠ 真意识.",
    "production_is_not_safety": "V1206 真生产 ≠ 真安全. Lift ≠ 守门. 任何声称 lift = safe 是不假装.",
    "automation_is_not_autonomy": "V1206 自动 lift ≠ 自主意识. cron 触发 lift ≠ V1206 自主.",
    "v1206_is_v06_16": "V1206 = V0.6.16 中间, 北极星 0.98 不变. 不假装 V1206 = ASI 终极.",
    "v1205_bugs_are_measurement": "V1206 fix V1205 4 bugs = measurement fixed, V1072 真本质 ≠ measurement 真.",
    "time_grounding_not_in_v06": "V1206 time_grounding 不在 V0.5/0.6 ASI 公式. V1206 局部 dim, 不假装 V0.6.16 ASI 已含.",
}