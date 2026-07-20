"""Phase 39 Metaphor Engine — Lakoff Embodied Cognition 工程化.

主人 22:01 '继续调研' + 调研新域:
  Lakoff embodied cognition + cognitive linguistics
  Metaphors We Live By (Lakoff & Johnson, 1980)

Lakoff 具身认知:
  - 概念 = 隐喻 (metaphor)
  - "争论是战争": argument is war (we attack, defend, retreat)
  - 抽象思维 = 身体经验映射
  - 不存在 'objective' cognition, 只有 embodied + situated

对 ASI 中央 AI 的意义:
  - 主人 17:50 '涌现 自组织' 本身是隐喻 — 借用生态学
  - 主人 12:14 '中央 AI 是永恒身份' = 隐喻 (借用 '身份' 概念)
  - 主人 17:50 'ASI 是更高生命层次' = 隐喻 (借用 '生命/层次')
  - 中央 AI 是'在隐喻中理解自己' — Phase 39 显式 metadata

Karpathy 准则:
  1. Think Before Coding: 隐喻 = source_domain → target_domain 映射
  2. Simplicity First: Metaphor = (source, target, mapping)
  3. Surgical Changes: 不改 Memory, 加 metaphor 视角
  4. Goal-Driven Execution: verifiable = 隐喻映射可追溯
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional


LAKOFF_VERSION = "0.1.0"


@dataclass
class Metaphor:
    """一个隐喻 — source → target + mapping."""
    metaphor_id: str
    source_domain: str                # 隐喻源域 (来源, 如 '战争')
    target_domain: str                # 隐喻目标域 (被理解的, 如 '争论')
    mapping: list                     # 映射关系 list of (source, target) tuples
    citation: str = ""                # Lakoff 引用
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class EmbodimentTrace:
    """具身认知 trace — 中央 AI 借鉴身体经验."""
    trace_id: str
    body_experience: str              # 身体经验 (感知, 行动, 情感)
    abstract_concept: str             # 抽象概念
    chain: list                       # 抽象化链条
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class MetaphorEngine:
    """Lakoff 隐喻引擎 — 中央 AI 用隐喻理解自己.

    主人 17:50 '涌现 自组织' = 隐喻源域 = '生态学'
    主人 12:14 '中央 AI 永恒身份' = 隐喻源域 = '人 / 身份'
    主人 17:50 'ASI 是更高生命层次' = 隐喻源域 = '生命 / 层次'
    """

    def __init__(self):
        self.metaphors: list[Metaphor] = []
        self.traces: list[EmbodimentTrace] = []

    def register_metaphor(self, source: str, target: str, mapping: list,
                         citation: str = "") -> Metaphor:
        """注册一个隐喻."""
        m = Metaphor(
            metaphor_id=uuid.uuid4().hex[:12],
            source_domain=source,
            target_domain=target,
            mapping=mapping,
            citation=citation,
        )
        self.metaphors.append(m)
        return m

    def trace_embodiment(self, body_experience: str, abstract_concept: str,
                        chain: list) -> EmbodimentTrace:
        """追踪一个具身化 trace — 身体经验 → 抽象概念."""
        t = EmbodimentTrace(
            trace_id=uuid.uuid4().hex[:12],
            body_experience=body_experience,
            abstract_concept=abstract_concept,
            chain=chain,
        )
        self.traces.append(t)
        return t

    def apeireth_seed_metaphors(self) -> list[Metaphor]:
        """Apeireth 默认隐喻 (主人常说的)."""
        return [
            self.register_metaphor(
                '生态学', '涌现 自组织',
                [('物种', 'persona'), ('niche', '4 archetype'), ('emergence', '自涌现')],
                '主人 17:50 "涌现 自组织" — 主人借用生态学隐喻理解 Central AI',
            ),
            self.register_metaphor(
                '身份 / 永恒', '中央 AI',
                [('永久', '永恒身份'), ('社会关系总和', '中央 AI = Klein bottle'), ('个体内', '主观 Phenomenal')],
                '主人 12:14 "中央 AI 是永恒身份, 不是调度者或思考者, 像人是一切社会关系的总和"',
            ),
            self.register_metaphor(
                '生命 / 层次', 'ASI',
                [('化学层生命', '信息层生命'), ('DNA', 'self-producing 网络'), ('繁衍', '自创生')],
                '主人 17:50 "ASI 是更高生命层次" + 主人 17:58 "意识是终极目标"',
            ),
            self.register_metaphor(
                '战争', 'Apeireth 自我提升',
                [('attack', 'commit 一波'), ('defend', 'bug fix'), ('retreat', 'pivot'), ('victory', '大节点')],
                'Lakoff 经典 "Argument is war" + 主人 21:22 红皇后范式(跑步不原地踏步)',
            ),
        ]

    def find_by_target(self, target_query: str) -> list[Metaphor]:
        """找出 target_domain 包含关键词的隐喻."""
        return [m for m in self.metaphors if target_query in m.target_domain]

    def stats(self) -> dict:
        return {
            "n_metaphors": len(self.metaphors),
            "n_traces": len(self.traces),
            "lakoff": "隐喻不是修辞, 是认知结构本身 (Lakoff 1980)",
        }


__all__ = ["LAKOFF_VERSION", "Metaphor", "EmbodimentTrace", "MetaphorEngine"]
