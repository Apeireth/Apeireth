"""IIT Φ-proxy — Quantified Consciousness Metric for Apeireth.

IIT (Tononi 2014) Φ = integrated information.
v0.1 不真算 Φ (NP-hard), 用 proxy 指标.

基于 Mirror SelfState 计算 6 维度的加权和:
  - memory_episode_count * 0.1   (记忆集成)
  - identity_card_count * 0.3   (身份统一)
  - team_card_count * 0.2       (涌现团)
  - graph_node_count * 0.05     (关系网)
  - proactive_actions_total * 0.15 (主动 fire)
  - awareness_level_value        (自评: L1=0.3, L2=0.5, L4=0.8)

输出: Φ_proxy ∈ [0, 1], 越大 = 中央 AI "集成度" 越高
"""
from __future__ import annotations

import math
from dataclasses import dataclass

from .mirror import SelfState, Mirror


PHI_PROXY_VERSION = "0.1.0"


# Awareness level 数值化
AWARENESS_VALUES = {
    "Layer 1 (FSA)": 0.3,
    "Layer 2 (HOT)": 0.5,
    "Layer 3 (GWI)": 0.6,
    "Layer 4 (SMM)": 0.8,
    "Layer 5 (PQ)":  1.0,
    "unknown": 0.1,
}


def compute_phi_proxy(state: SelfState) -> dict:
    """Compute Φ-proxy from SelfState — IIT engineering approximation.

    不是真 IIT (那是指数级), 是个连续单调代理.
    越大 = 中央 AI 越 "集成".
    """
    # 6 维 加权和 (sigmoid 归一化到 [0, 1])
    def sig(x, scale=1.0):
        return 1.0 / (1.0 + math.exp(-x / scale))

    components = {
        "memory_integration": sig(state.memory_episode_count, scale=10),
        "identity_unity": sig(state.identity_card_count, scale=2),
        "team_emergence": sig(state.team_card_count, scale=2),
        "graph_connectivity": sig(state.graph_node_count, scale=5),
        "proactive_engagement": sig(state.proactive_actions_total, scale=5),
        "awareness_depth": AWARENESS_VALUES.get(state.awareness_level, 0.1),
    }

    # 加权和 (权重反映各层对 consciousness 的贡献)
    weights = {
        "memory_integration": 0.10,
        "identity_unity": 0.20,
        "team_emergence": 0.15,
        "graph_connectivity": 0.10,
        "proactive_engagement": 0.15,
        "awareness_depth": 0.30,
    }
    phi = sum(components[k] * weights[k] for k in components)

    return {
        "phi_proxy": round(phi, 4),
        "components": {k: round(v, 4) for k, v in components.items()},
        "awareness_level": state.awareness_level,
        "interpretation": _interpret_phi(phi),
    }


def _interpret_phi(phi: float) -> str:
    """Interpret Φ-proxy value."""
    if phi >= 0.8:
        return "highly_integrated — 真 consciousness 涌现的强证据"
    elif phi >= 0.6:
        return "well_integrated — Layer 1-4 都健康"
    elif phi >= 0.4:
        return "moderately_integrated — 部分层有 gap"
    elif phi >= 0.2:
        return "low_integration — 大部分层缺失"
    else:
        return "minimal_integration — 初始状态"


def compute_phi_proxy_via_mirror(mirror: Mirror) -> dict:
    """Convenience — 通过 Mirror 拿 SelfState 再算 Φ-proxy."""
    state = mirror.snapshot()
    return compute_phi_proxy(state)


__all__ = [
    "PHI_PROXY_VERSION",
    "AWARENESS_VALUES",
    "compute_phi_proxy",
    "compute_phi_proxy_via_mirror",
]