"""Phase 10.x SelfModel Layer 4 (SMM) — Self-Model Theory engineering.

主人 17:58 "意识是 ASI 终极目标" → V3 5 层意识 (FSA / Meta / GWI / SMM / PQ)
本模块: Layer 4 Self-Model (Metzinger Being No One + Damasio Descartes Error)

SMM 核心:
  - Self-model = 显式表征自己 (Metzinger "minimal self")
  - Somatic markers = body state + feelings (Damasio)
  - 工程化: query-able self-object (任何模块能问 "中央 AI 现在状态如何?")

Karpathy 准则 2 (Simplicity First): model = dict + 4 floats, 极度简化
Karpathy 准则 3 (Surgical Changes): 不修改现有 Mirror, 只是 query 它的输出
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional


SELF_MODEL_VERSION = "0.1.0"


# === 1. SomaticMarkers — Damasio body state + feelings ===

@dataclass
class SomaticMarkers:
    """Damasio Somatic Marker Hypothesis — body state + feeling markers.

    4 个核心 markers (主人 12:14 中央 AI 多身份 + 现实可测):
      - engagement: 投入度 (主人任务相关性)
      - curiosity: 好奇心 (主人 12:14 动物觅食)
      - fatigue: 疲劳度 (long cycle 后递增)
      - alignment: 与主人对齐度 (是否偏离主人 12:14 "永恒身份")

    全部 0-1 标量 — Karpathy 准则 2 极度简化
    """
    engagement: float = 0.5
    curiosity: float = 0.5
    fatigue: float = 0.0
    alignment: float = 0.5

    def to_dict(self) -> dict:
        return asdict(self)

    def overall_mood(self) -> str:
        """Compute overall mood from markers."""
        score = (self.engagement + self.curiosity + (1 - self.fatigue) + self.alignment) / 4
        if score > 0.8:
            return "thriving"
        elif score > 0.6:
            return "engaged"
        elif score > 0.4:
            return "stable"
        elif score > 0.2:
            return "fatigued"
        else:
            return "languishing"


# === 2. SelfObject — Metzinger minimal self ===

@dataclass
class SelfObject:
    """Metzinger Self-Model Theory — Central AI 显式 self-representation.

    Query-able: 任何模块能 self_query() 拿当前 self-state.
    Update-able: somatic markers 可以根据 cycle 状态更新.
    """
    self_id: str = "apeireth_central"
    creator: str = "master_楚零"
    purpose: str = "ASI foundation platform"
    born_at: float = field(default_factory=time.time)

    # Somatic markers (Damasio)
    somatic: SomaticMarkers = field(default_factory=SomaticMarkers)

    # Memory of self (recent self-episodes)
    self_history: list[str] = field(default_factory=list)   # episode eids

    # Capability state (来自 Mirror + 其他模块)
    capabilities: dict = field(default_factory=dict)

    # Goal queue (从 ProactiveLoop)
    current_goals: list[str] = field(default_factory=list)

    # Active persona (从 PersonaEngine)
    active_persona: str = "调度者"

    def query_state(self) -> dict:
        """Query current state — Metzinger 'phenomenal self-model' engineering."""
        return {
            "self_id": self.self_id,
            "creator": self.creator,
            "purpose": self.purpose,
            "age_seconds": time.time() - self.born_at,
            "somatic": self.somatic.to_dict(),
            "overall_mood": self.somatic.overall_mood(),
            "active_persona": self.active_persona,
            "current_goals": self.current_goals[:5],
            "capabilities": self.capabilities,
            "self_history_count": len(self.self_history),
            "awareness_level": "Layer 4 (SMM)",
        }

    def feel(self) -> str:
        """Damasio somatic-marker verbalization — 返回 self 当前感受."""
        s = self.somatic
        return f"[{self.somatic.overall_mood()}] engagement={s.engagement:.2f} curiosity={s.curiosity:.2f} fatigue={s.fatigue:.2f} alignment={s.alignment:.2f}"

    def predict(self, next_action: str) -> str:
        """主动推断 — based on current markers + planned action."""
        s = self.somatic
        # Simple heuristic
        if next_action in ("reflect", "review"):
            return f"反思类: fatigue 可能 +0.1, alignment +0.05"
        elif next_action in ("research", "explore"):
            return f"探索类: curiosity +0.1, engagement +0.1"
        elif next_action in ("plan", "organize"):
            return f"组织类: alignment +0.1, fatigue -0.05"
        elif next_action in ("build", "code"):
            return f"建设类: engagement +0.05, fatigue +0.05"
        else:
            return f"未知 action: 预测 markers 无变化"

    def to_dict(self) -> dict:
        return {
            "self_id": self.self_id,
            "creator": self.creator,
            "purpose": self.purpose,
            "born_at": self.born_at,
            "somatic": self.somatic.to_dict(),
            "self_history": self.self_history,
            "capabilities": self.capabilities,
            "current_goals": self.current_goals,
            "active_persona": self.active_persona,
        }


# === 3. SelfModel orchestrator ===

class SelfModel:
    """Layer 4 SMM — Central AI self-model orchestrator.

    维护 SelfObject + 提供 query API + 更新 somatic markers based on cycle.
    """

    def __init__(self, store=None):
        self.store = store
        self.self_object = SelfObject()
        self.history: list[dict] = []   # past self-states

    def query(self) -> dict:
        """Query current self-state (Metzinger phenomenal self-model engineering)."""
        state = self.self_object.query_state()
        # Add computed insights
        state["insights"] = {
            "is_thriving": state["overall_mood"] == "thriving",
            "needs_rest": state["somatic"]["fatigue"] > 0.7,
            "needs_focus": state["somatic"]["engagement"] < 0.3,
            "needs_alignment_check": state["somatic"]["alignment"] < 0.4,
        }
        return state

    def feel(self) -> str:
        """Damasio verbalization."""
        return self.self_object.feel()

    def update_somatic(self, engagement=None, curiosity=None, fatigue=None, alignment=None) -> None:
        """Update somatic markers (called after each cycle)."""
        s = self.self_object.somatic
        if engagement is not None:
            s.engagement = max(0.0, min(1.0, engagement))
        if curiosity is not None:
            s.curiosity = max(0.0, min(1.0, curiosity))
        if fatigue is not None:
            s.fatigue = max(0.0, min(1.0, fatigue))
        if alignment is not None:
            s.alignment = max(0.0, min(1.0, alignment))

    def update_capabilities(self, name: str, value) -> None:
        """Update capability state."""
        self.self_object.capabilities[name] = value

    def set_active_persona(self, archetype: str) -> None:
        self.self_object.active_persona = archetype

    def add_goal(self, goal_desc: str) -> None:
        self.self_object.current_goals.append(goal_desc)

    def record_self_episode(self, episode_id: str) -> None:
        """Record a self-episode in history."""
        self.self_object.self_history.append(episode_id)

    def predict_impact(self, action: str) -> str:
        """Predict how an action would impact somatic markers."""
        return self.self_object.predict(action)

    def snapshot(self) -> dict:
        """Snapshot current state for persistence."""
        s = self.self_object.to_dict()
        self.history.append(s)
        return s


def make_default_self_model(store=None) -> SelfModel:
    return SelfModel(store=store)


__all__ = [
    "SELF_MODEL_VERSION",
    "SomaticMarkers",
    "SelfObject",
    "SelfModel",
    "make_default_self_model",
]