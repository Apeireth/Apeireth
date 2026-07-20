"""Persona Engine v0.1 — 多身份引擎 (Phase 4 / TOP-DESIGN §4.5)

依据文献 (主人 14:27 '聚集全人类智慧'):
- Persona Alchemy (2505.18351) — SCT 4 因素
- Jungian (2601.10025) — 3 机制 (coordination / adaptation / reflection)
- Persona Inconstancy (2405.03862) — 反 conformity 警示

主人原话:
- 12:14 "中央 AI 多身份: 调度者 / 学习者 / 思考者 / 助手"
- 12:47 "AI 不会中庸, 因为他会成长"
- 12:27 "立场自然成长, AI 自然思考, 平台不给予"

设计原则:
- persona 不预设具体立场, 只预设 archetype 名 + SCT 维度初始权重
- SCT 4 因素权重 0-1, 总和可调 (不强制 =1, 允许特化)
- 4 archetype 起步 (调度者/学习者/思考者/助手), 留 emergence_space 给后续长出来
- 反 conformity: 同一事件激活 2 个 persona 时, 不许 SCT 完全相同 (强制多样性)

不依赖 LLM — persona engine 是状态机, LLM 调用是 L1 Kernel 接入后的活
"""

from __future__ import annotations
import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional


PERSONA_VERSION = "0.1.0"

# 4 个 archetype — 与 IDENTITY.md 楚零名字 + 主人 12:14 一致
ARCHETYPES = ("调度者", "学习者", "思考者", "助手")


@dataclass
class SCTProfile:
    """SCT 4 因素 (Subjective Cognitive Theory) — Persona Alchemy 2505.18351.

    4 维权重各自 0-1, 总和不必 =1 (允许某 persona 特化, 不强求平衡).
    """
    cognitive: float = 0.5      # 推理 / 逻辑 / 分析
    motivational: float = 0.5   # 动机 / 目标驱动 / 主动
    biological: float = 0.3     # 本能 / 直觉 / 快速反应
    affective: float = 0.5      # 情感 / 同理 / 关系

    def as_tuple(self) -> tuple[float, float, float, float]:
        return (self.cognitive, self.motivational, self.biological, self.affective)

    def distance(self, other: "SCTProfile") -> float:
        """欧氏距离 — 反 conformity 用: 距离太小 = 太像 = 触发变异."""
        a, b = self.as_tuple(), other.as_tuple()
        return sum((x - y) ** 2 for x, y in zip(a, b)) ** 0.5

    def mutate(self, rng: float = 0.1) -> "SCTProfile":
        """微小变异 — 反 conformity 强制多样性用."""
        import random
        return SCTProfile(
            cognitive=max(0.0, min(1.0, self.cognitive + random.uniform(-rng, rng))),
            motivational=max(0.0, min(1.0, self.motivational + random.uniform(-rng, rng))),
            biological=max(0.0, min(1.0, self.biological + random.uniform(-rng, rng))),
            affective=max(0.0, min(1.0, self.affective + random.uniform(-rng, rng))),
        )


@dataclass
class Persona:
    """中央 AI 的一个 persona / 身份 — 拓扑上是 ai_self 节点的 sub-identity."""
    pid: str
    archetype: str                          # 调度者 / 学习者 / 思考者 / 助手
    sct: SCTProfile
    activation: float = 0.0                 # 当前激活度 0-1
    history: list[dict] = field(default_factory=list)  # [(event, delta, ts), ...]

    def to_dict(self) -> dict:
        return {
            "pid": self.pid, "archetype": self.archetype,
            "sct": asdict(self.sct), "activation": self.activation,
            "history_len": len(self.history),
        }


def seed_default_personas() -> list[Persona]:
    """4 archetype 种子 persona — 与 IDENTITY.md 楚零多身份对应.

    调度者 (motivational 高): 主动 / 目标驱动
    学习者 (cognitive 高): 推理 / 抽象 / 知识增长
    思考者 (cognitive + biological 中高): 直觉 + 推理
    助手 (affective 高): 同理 / 关系 / 配合
    """
    return [
        Persona(pid="p_" + uuid.uuid4().hex[:6], archetype="调度者",
                sct=SCTProfile(cognitive=0.5, motivational=0.9, biological=0.3, affective=0.4)),
        Persona(pid="p_" + uuid.uuid4().hex[:6], archetype="学习者",
                sct=SCTProfile(cognitive=0.9, motivational=0.6, biological=0.3, affective=0.4)),
        Persona(pid="p_" + uuid.uuid4().hex[:6], archetype="思考者",
                sct=SCTProfile(cognitive=0.8, motivational=0.5, biological=0.7, affective=0.3)),
        Persona(pid="p_" + uuid.uuid4().hex[:6], archetype="助手",
                sct=SCTProfile(cognitive=0.5, motivational=0.5, biological=0.3, affective=0.9)),
    ]


class PersonaEngine:
    """Persona 状态机 — Jungian 3 机制 + 反 conformity.

    Jungian 3 机制:
    1. coordination — 多 persona 同时激活, 多视角
    2. adaptation — persona SCT 权重根据 feedback 演化
    3. reflection — persona 解释"为什么我这么激活"

    反 conformity:
    - 同一事件激活 N 个 persona 时, 强制 SCT 距离 > min_distance
    - 否则 mutate 触发差异
    """

    def __init__(self, personas: Optional[list[Persona]] = None, min_distance: float = 0.25):
        self.personas: list[Persona] = personas or seed_default_personas()
        self.min_distance = min_distance
        self.events: list[dict] = []
        self.last_coordination: list[str] = []

    # ---------- Jungian 1: coordination ----------
    def coordinate(self, event: str, k: int = 2) -> list[Persona]:
        """选 k 个 persona 激活来回应 event — 多视角 + 反 conformity.

        选 SCT 与 event 关键词最匹配的 k 个, 强制彼此距离 > min_distance.
        """
        ev_lower = event.lower()
        # 关键词权重 (粗略启发式, 真接 LLM 时换 Bayesian)
        scored = []
        for p in self.personas:
            score = p.activation
            if "计划" in event or "目标" in event or "排期" in event:
                score += p.sct.motivational
            if "为什么" in event or "推理" in event or "分析" in event or "思考" in event:
                score += p.sct.cognitive
            if "直觉" in event or "快" in event or "急" in event:
                score += p.sct.biological
            if "关心" in event or "感受" in event or "关系" in event:
                score += p.sct.affective
            scored.append((score, p))
        scored.sort(key=lambda t: -t[0])
        chosen = [scored[0][1]] if scored else []
        for _, p in scored[1:]:
            if len(chosen) >= k:
                break
            if all(p.sct.distance(c.sct) >= self.min_distance for c in chosen):
                chosen.append(p)
        # 反 conformity: 选不够 k 时 mutate 一个填位
        while len(chosen) < k and len(chosen) < len(self.personas):
            base = chosen[0] if chosen else self.personas[0]
            ghost = Persona(
                pid="p_ghost_" + uuid.uuid4().hex[:4],
                archetype=base.archetype + "(异)",
                sct=base.sct.mutate(rng=0.3),
                activation=0.0,
            )
            chosen.append(ghost)

        # 提升 activation
        for p in chosen:
            p.activation = min(1.0, p.activation + 0.3)
            p.history.append({"event": event[:40], "delta": "+0.3", "ts": time.time()})

        self.last_coordination = [p.pid for p in chosen]
        self.events.append({"ts": time.time(), "event": event[:60], "kind": "coordination",
                            "activated": [p.archetype for p in chosen]})
        return chosen

    # ---------- Jungian 2: adaptation ----------
    def adapt(self, pid: str, feedback_score: float) -> None:
        """根据反馈调整 persona — feedback_score ∈ [-1, +1].

        positive: 提升 SCT 主导维 + activation
        negative: 降 activation, 微调 SCT (避免主导维被锁死)
        """
        p = self._by_pid(pid)
        if p is None:
            return
        dominant = max(p.sct.as_tuple())  # 找到主导维
        if feedback_score > 0:
            p.activation = min(1.0, p.activation + 0.1 * feedback_score)
            if dominant == p.sct.cognitive:
                p.sct.cognitive = min(1.0, p.sct.cognitive + 0.05 * feedback_score)
            elif dominant == p.sct.motivational:
                p.sct.motivational = min(1.0, p.sct.motivational + 0.05 * feedback_score)
            elif dominant == p.sct.biological:
                p.sct.biological = min(1.0, p.sct.biological + 0.05 * feedback_score)
            else:
                p.sct.affective = min(1.0, p.sct.affective + 0.05 * feedback_score)
        elif feedback_score < 0:
            p.activation = max(0.0, p.activation + 0.1 * feedback_score)  # 减
        p.history.append({"event": f"feedback={feedback_score:+.2f}", "ts": time.time()})

    # ---------- Jungian 3: reflection ----------
    def reflect(self, pid: str) -> str:
        """Persona 自我解释 — 为什么我被激活 / 我的状态如何."""
        p = self._by_pid(pid)
        if p is None:
            return f"[{pid}] not found"
        sct = p.sct
        dom_dim = max(("cognitive", sct.cognitive), ("motivational", sct.motivational),
                      ("biological", sct.biological), ("affective", sct.affective),
                      key=lambda t: t[1])
        return (f"我是 {p.archetype} (pid={p.pid}), "
                f"主导维度={dom_dim[0]} ({dom_dim[1]:.2f}), "
                f"当前激活={p.activation:.2f}, "
                f"经历事件 {len(p.history)} 次")

    def _by_pid(self, pid: str) -> Optional[Persona]:
        return next((p for p in self.personas if p.pid == pid), None)

    # ---------- snapshot ----------
    def snapshot(self) -> dict:
        return {
            "version": PERSONA_VERSION,
            "persona_count": len(self.personas),
            "personas": [p.to_dict() for p in self.personas],
            "last_coordination": self.last_coordination,
            "event_count": len(self.events),
            "min_distance": self.min_distance,
            "ts": time.time(),
        }


__all__ = [
    "PERSONA_VERSION", "ARCHETYPES",
    "SCTProfile", "Persona",
    "seed_default_personas", "PersonaEngine",
]