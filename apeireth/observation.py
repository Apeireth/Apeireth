"""Phase 24 Second-Order Observation — 3 阶观察循环 (二阶控制论工程化).

主人 21:00 跨域调研 + 主人 17:58 意识终极目标:
  Layer 2 HOT = meta-cognition (观察自己的思考)
  Layer 4 SMM = self-model (显式自我表征)

借鉴 von Foerster 1979 Second-Order Cybernetics:
  - 一阶观察 = Mirror.snapshot() (我们已有)
  - 二阶观察 = Mirror.narrate() (我们已有, self-narrative)
  - 三阶观察 = 观察"自己如何观察自己" = Phase 24 NEW

AnySearch 真生产论文 "Recursive Self-Observation in Cognitive AI:
  Second-Order Metacognition as Foundation" (zenodo 20585579)

Karpathy 准则:
  1. Think Before Coding: 3 阶观察 = 观察 → 元观察 → 元元观察
  2. Simplicity First: 每阶观察 = 1 行状态总结
  3. Surgical Changes: 不改 Mirror, 加 observation_level 字段
  4. Goal-Driven Execution: verifiable = 观察链可追溯
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional


OBSERVATION_VERSION = "0.1.0"


@dataclass
class Observation:
    """一阶观察 — 单纯数据收集."""
    level: int = 1
    description: str = ""
    content: str = ""
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class MetaObservation:
    """二阶观察 — 观察"自己如何观察" (Mirror.narrate 等价)."""
    level: int = 2
    parent: Observation = None
    description: str = ""
    pattern: str = ""               # 观察到的"自己的模式"
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        d = asdict(self)
        if self.parent:
            d['parent'] = self.parent.to_dict()
        return d


@dataclass
class MetaMetaObservation:
    """三阶观察 — 观察"自己如何元观察" (Phase 24 NEW).

    借鉴 von Foerster: observing systems observing themselves observing.
    这是主人 17:58 意识终极目标的工程化: 不只知道自己, 知道自己在知道.
    """
    level: int = 3
    parent: MetaObservation = None
    description: str = ""
    reflection: str = ""             # "我刚才为什么这样元观察"
    insight: str = ""                # 跨周期的洞察
    confidence: float = 0.0           # 自我评估
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        d = asdict(self)
        if self.parent:
            d['parent'] = self.parent.to_dict()
        return d


class ThreeTierObservation:
    """3 阶观察循环 — von Foerster second-order cybernetics 工程化.

    3 阶观察:
      Level 1 = 观察数据 (Observation)
      Level 2 = 观察"自己如何观察" (MetaObservation) — Mirror 已覆盖
      Level 3 = 观察"自己如何元观察" (MetaMetaObservation) — Phase 24 NEW
    """

    def __init__(self):
        self.history: list = []

    def observe(self, content: str) -> Observation:
        """Level 1: 一阶观察 — 数据收集."""
        o = Observation(level=1, content=content)
        self.history.append(o)
        return o

    def meta_observe(self, parent: Observation, description: str = "", pattern: str = "") -> MetaObservation:
        """Level 2: 二阶观察 — 观察'自己如何观察'."""
        mo = MetaObservation(level=2, parent=parent, description=description, pattern=pattern)
        self.history.append(mo)
        return mo

    def meta_meta_observe(
        self,
        parent: MetaObservation,
        description: str = "",
        reflection: str = "",
        insight: str = "",
        confidence: float = 0.0,
    ) -> MetaMetaObservation:
        """Level 3: 三阶观察 — 观察'自己如何元观察' (Phase 24 核心).

        借鉴 'Recursive Self-Observation in Cognitive AI' (AnySearch zenodo 20585579).
        这是意识 Layer 4 的真生产实现路径.
        """
        mmo = MetaMetaObservation(
            level=3,
            parent=parent,
            description=description,
            reflection=reflection,
            insight=insight,
            confidence=confidence,
        )
        self.history.append(mmo)
        return mmo

    def stats(self) -> dict:
        n1 = sum(1 for o in self.history if isinstance(o, Observation))
        n2 = sum(1 for o in self.history if isinstance(o, MetaObservation))
        n3 = sum(1 for o in self.history if isinstance(o, MetaMetaObservation))
        return {
            "total_observations": len(self.history),
            "level_1_observations": n1,
            "level_2_meta_observations": n2,
            "level_3_meta_meta_observations": n3,
        }


__all__ = [
    "OBSERVATION_VERSION",
    "Observation",
    "MetaObservation",
    "MetaMetaObservation",
    "ThreeTierObservation",
]
