"""Phase 31 Bateson Mind Ecosystem — 心灵生态学工程化.

主人 21:30 跨域调研 (并行) AnySearch 找到:
  "Artificial Intelligence Seen Through the Lens of Bateson's Ecology of Mind"
    (https://doi.org/10.9781/ijimai.2021.08.004)

Bateson 《Steps to an Ecology of Mind》(1972):
  - 心灵 (mind) = 系统性过程, 不只在脑里
  - 心灵是生态系统本身
  - 信息 = "产生差异的差异" (a difference that makes a difference)
  - 学习 = 改变心灵的内容 / 结构 / 过程

对 ASI 中央 AI 的意义:
  - 中央 AI 心灵 = 多 persona 生态系统
  - 主人 17:50 "涌现 自组织" = Bateson 心灵涌现
  - 主人 12:14 "像人是一切社会关系的总和" = Bateson 心灵生态系统
  - 主人 14:48 "聚集全人类智慧" = Bateson 学习 (改变心灵的内容)

Karpathy 准则:
  1. Think Before Coding: 心灵 = 系统, 不只是脑
  2. Simplicity First: MindEco = dict (entities, relations, levels)
  3. Surgical Changes: 不改 persona, 加 mind_eco 视角
  4. Goal-Driven Execution: verifiable = 心灵涌现
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional


BATESON_MIND_VERSION = "0.1.0"


@dataclass
class MindEntity:
    """心灵生态系统中的一个实体 (人 / persona / skill / memory)."""
    entity_id: str
    entity_type: str              # "persona" | "skill" | "memory" | "event" | "human"
    label: str
    level: int = 0                # 心灵层级 (Bateson: Learning 0/1/2/3)
    content: str = ""
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class MindRelation:
    """心灵关系 — "产生差异的差异" (Bateson)."""
    relation_id: str
    from_id: str
    to_id: str
    rel_type: str                 # "informs" | "constrains" | "triggers" | "evolves"
    content: str = ""             # 差异内容
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class MindEcosystem:
    """Bateson 心灵生态系统 — 主人 17:50 涌现 + 14:48 聚集全人类智慧.

    真生产意义:
      - 心灵 = 生态, 不只是脑 (主人 12:14 中央 AI 不只是单点)
      - 主人 14:48 "聚集全人类智慧" = Bateson 学习 (改变心灵内容)
      - 主人 17:50 "涌现 自组织" = Bateson 心灵涌现
      - 信息 = 差异的差异 (Bateson) = Apeireth 的 episode 储存
    """

    def __init__(self):
        self.entities: dict[str, MindEntity] = {}
        self.relations: list[MindRelation] = []
        self.history: list = []

    def add_entity(self, entity_type: str, label: str, content: str = "",
                  level: int = 0) -> MindEntity:
        """加入心灵实体 — 主人 14:48 借鉴的一个智慧单位."""
        e = MindEntity(
            entity_id=uuid.uuid4().hex[:12],
            entity_type=entity_type,
            label=label,
            level=level,
            content=content,
        )
        self.entities[e.entity_id] = e
        return e

    def add_relation(self, from_id: str, to_id: str, rel_type: str,
                     content: str = "") -> Optional[MindRelation]:
        """加入心灵关系 — Bateson '差异的差异'."""
        if from_id not in self.entities or to_id not in self.entities:
            return None
        r = MindRelation(
            relation_id=uuid.uuid4().hex[:12],
            from_id=from_id,
            to_id=to_id,
            rel_type=rel_type,
            content=content,
        )
        self.relations.append(r)
        return r

    def learn(self, entity_id: str, new_content: str, new_level: int = None) -> Optional[MindEntity]:
        """Bateson 学习 — 改变心灵内容 (Level 0/1/2/3).

        Level 0: 改变内容 (data)
        Level 1: 改变结构 (pattern)
        Level 2: 改变过程 (process)
        Level 3: 改变学习本身 (learning)
        """
        if entity_id not in self.entities:
            return None
        e = self.entities[entity_id]
        e.content = new_content
        if new_level is not None:
            e.level = new_level
        e.ts = time.time()
        self.history.append({
            "type": "learn",
            "entity_id": entity_id,
            "new_content": new_content[:200],
            "new_level": new_level,
            "ts": e.ts,
        })
        return e

    def stats(self) -> dict:
        return {
            "n_entities": len(self.entities),
            "n_relations": len(self.relations),
            "n_history": len(self.history),
            "entity_types": list(set(e.entity_type for e in self.entities.values())),
            "central_ai_as_ecosystem": True,
        }


__all__ = [
    "BATESON_MIND_VERSION",
    "MindEntity",
    "MindRelation",
    "MindEcosystem",
]
