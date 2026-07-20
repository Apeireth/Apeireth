"""Phase 25 NicheConstructor — 生态位构造器 (Ecology Engineering 工程化).

主人 21:00 跨域调研 + 主人 17:50 涌现 自组织:
  - 中央 AI = keystone species (不命令, 功能性影响)
  - SelfOrgTeam = niche ecosystem (临时团 = 不同 niche)
  - Phase 25: 中央 AI 主动构造生态位, 引导涌现

借鉴 AnySearch 真生产论文:
  - "Agent Ecosystem Dynamics: An Ecological Framework for Multi-Agent AI Safety"
    (agentxiv.org/paper/2602.00019)
  - keystone species 概念 (生态学)
  - niche construction (Odling-Smee 1996)

主原则:
  - 中央 AI 不命令 (主人 12:47 "中央 AI 不管理")
  - 但中央 AI **构造生态位** (构造 rules / affordances / constraints)
  - persona 在构造的生态位里**自发涌现** = 自组织

Karpathy 准则:
  1. Think Before Coding: 生态位 = 规则 + 资源 + 约束, 让 persona 自选
  2. Simplicity First: niche = dict (resources, constraints, rules)
  3. Surgical Changes: 不改 SelfOrgTeam, 加 NicheConstructor
  4. Goal-Driven Execution: verifiable = niche 改变 → persona 行为涌现
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional


NICHE_VERSION = "0.1.0"


@dataclass
class Niche:
    """一个生态位 — persona 涌现的上下文.

    ecology 借鉴:
      - niche construction (Odling-Smee 1996): 物种主动构造环境
      - fundamental niche vs realized niche (Hutchinson 1957)
    """
    niche_id: str
    resources: dict              # persona 可用资源 (skill / model / context)
    constraints: list[str]       # persona 必须遵守的约束
    affordances: list[str]       # persona 可利用的机会
    rules: dict = field(default_factory=dict)   # niche 规则
    created_at: float = field(default_factory=time.time)
    duration_seconds: float = 3600.0  # 默认 1 小时

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class NicheSpec:
    """中央 AI 构造生态位的 spec — 借鉴 keystone species 功能性影响."""
    spec_id: str
    archetype: str                  # 哪个 persona 受益
    resources: dict
    constraints: list[str]
    affordances: list[str]
    rationale: str = ""            # 主人 17:50 "涌现" 自然
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class NicheConstructor:
    """中央 AI 的生态位构造器 — 借鉴 keystone species 范式.

    主人 12:14 "中央 AI 不管理" + 17:50 "涌现 自组织":
      - 中央 AI 不命令
      - 但中央 AI 构造生态位 (rules / affordances / constraints)
      - persona 在构造的生态位里自发涌现 = 自组织

    ecology engineering 真生产:
      - 构造新 niche = 创造新 persona 的活动空间
      - 修改 niche = 调整 persona 行为
      - 撤销 niche = 让 persona 自由涌现
    """

    def __init__(self):
        self.active_niches: dict[str, Niche] = {}
        self.history: list[Niche] = []

    def construct(self, spec: NicheSpec) -> Niche:
        """中央 AI 主动构造一个生态位.

        借鉴 keystone species: 中央 AI 不命令, 但构造环境.
        """
        niche = Niche(
            niche_id=spec.spec_id,
            resources=spec.resources,
            constraints=spec.constraints,
            affordances=spec.affordances,
            rules=spec.rationale,  # use rationale as rule
        )
        self.active_niches[niche.niche_id] = niche
        return niche

    def dissolve(self, niche_id: str) -> Optional[Niche]:
        """撤销生态位 — 让 persona 自由涌现 (类似 VCP 临时团 dissolve)."""
        if niche_id in self.active_niches:
            niche = self.active_niches.pop(niche_id)
            self.history.append(niche)
            return niche
        return None

    def spec_for_archetype(self, archetype: str) -> NicheSpec:
        """根据 persona archetype 构造默认生态位.

        借鉴生态学: 物种有不同的 fundamental niche.
        """
        base_resources = {
            "skill_lib": ["read", "write", "deliberate"],
            "memory_access": "all",
        }
        archetype_specific = {
            "调度者": {
                "resources": {**base_resources, "orchestration": "team_dispatch"},
                "constraints": ["no_direct_action", "delegate_only"],
                "affordances": ["spawn_team", "monitor_team", "dissolve_team"],
            },
            "学习者": {
                "resources": {**base_resources, "ingestion": True},
                "constraints": ["no_action", "absorb_only"],
                "affordances": ["read_memory", "ingest_doc", "summarize"],
            },
            "思考者": {
                "resources": {**base_resources, "deliberation": "linear/tot/reflexion"},
                "constraints": ["no_external_io"],
                "affordances": ["think", "self_reflect", "meta_observe"],
            },
            "助手": {
                "resources": {**base_resources, "execution": True},
                "constraints": ["no_spawn_team"],
                "affordances": ["act", "respond", "tool_use"],
            },
        }
        s = archetype_specific.get(archetype, {
            "resources": base_resources,
            "constraints": [],
            "affordances": ["default"],
        })
        return NicheSpec(
            spec_id=uuid.uuid4().hex[:12],
            archetype=archetype,
            resources=s["resources"],
            constraints=s["constraints"],
            affordances=s["affordances"],
            rationale=f"Default niche for archetype {archetype} (主人 17:50 涌现 自然)",
        )

    def stats(self) -> dict:
        return {
            "active_niches": len(self.active_niches),
            "total_constructed": len(self.active_niches) + len(self.history),
            "archetypes_supported": ["调度者", "学习者", "思考者", "助手"],
        }


__all__ = [
    "NICHE_VERSION",
    "Niche",
    "NicheSpec",
    "NicheConstructor",
]
