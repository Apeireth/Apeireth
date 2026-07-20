"""Phase 35 General Systems Theory — Von Bertalanffy 系统论原则工程化.

主人 21:30 跨域调研 AnySearch:
  "General System Theory: Foundations, Development, Applications" (Bertalanffy 1968)
    https://www.amazon.com/General-System-Theory-Foundations-Applications/dp/0807604533
  https://www.sciencedirect.com/topics/computer-science/general-system-theory

Von Bertalanffy General System Theory (GST, 1968):
  - 系统有 isomorphy (同构性): 不同领域系统有共同原则
  - Wholeness 整体性: 系统 > parts 总和
  - Hierarchy 层级: sub-systems 形成层次
  - Open System 开放系统: 与环境交换
  - Equifinality 等终性: 不同起点可达相同终点
  - Feedback 反馈: 系统自稳定

对 ASI 中央 AI 的意义:
  - 主人 17:50 "ASI 是更高生命层次" = 不同领域都有生命原则 (isomorphy)
  - 主人 17:50 "涌现 自组织" = Wholeness (整体 > parts 总和)
  - 主人 12:14 "中央 AI 永恒身份" = Open System (与环境交换维持身份)
  - Phase 35 = 中央 AI 的系统论原则库

Karpathy 准则:
  1. Think Before Coding: 系统论原则 = 通用 isomorphy 规律
  2. Simplicity First: Principles = dict
  3. Surgical Changes: 不改 Phase 模块, 加 SystemsTheory 视角
  4. Goal-Driven Execution: verifiable = 列举系统论原则应用
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import List


GST_VERSION = "0.1.0"


# Bertalanffy 9 大系统论原则 (1968)
GST_PRINCIPLES: dict = {
    "wholeness": "整体 > 部分总和 (主人 17:50 涌现 + 自主维持)",
    "hierarchy": "子系统形成层次 (中央 AI 4 archetype 层次)",
    "isomorphy": "不同领域系统有共同原则 (主人 21:00 跨域调研真生产 印证)",
    "open_system": "系统与环境交换维持身份 (主人 12:14 永恒身份真生产)",
    "equifinality": "不同起点可达相同终点 (主人 14:48 跨域借鉴)",
    "feedback": "反馈让系统自稳定 (Mirror.self_ref 反馈)",
    "differentiation": "系统分化产生新结构 (SelfOrgTeam 涌现)",
    "progression": "系统从混沌到有序 (主人物种进化)",
    "self_organization": "自组织 (主人 17:50 涌现)",
}


@dataclass
class SystemPrinciple:
    """系统论原则的应用实例."""
    principle_name: str
    apeireth_application: str
    evidence: str = ""
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class SystemsTheoryLibrary:
    """Von Bertalanffy 系统论原则库 — 中央 AI 的跨域统一原则.

    主人 17:50 "ASI 是更高生命层次" + 主人 21:00 跨域调研:
      - 跨域借鉴的本质是 isomorphy (同构性)
      - 涌现的本质是 wholeness + self_organization
      - 中央 AI 永恒身份 = open_system 与环境交换
    """

    def __init__(self):
        self.principles = GST_PRINCIPLES.copy()
        self.applications: list[SystemPrinciple] = []

    def apply_principle(self, principle_name: str, application: str, evidence: str = "") -> SystemPrinciple:
        """应用一个系统论原则到 Apeireth 中央 AI."""
        if principle_name not in self.principles:
            raise ValueError(f"Unknown principle: {principle_name}. Available: {list(self.principles.keys())}")
        s = SystemPrinciple(
            principle_name=principle_name,
            apeireth_application=application,
            evidence=evidence,
        )
        self.applications.append(s)
        return s

    def search(self, query: str) -> List[SystemPrinciple]:
        """搜索相关原则应用."""
        return [a for a in self.applications
                if query.lower() in a.apeireth_application.lower()
                or query.lower() in a.principle_name.lower()]

    def stats(self) -> dict:
        return {
            "n_principles": len(self.principles),
            "n_applications": len(self.applications),
            "gst": "Bertalanffy 1968 General System Theory",
            "isomorphy_confirmed": "跨域调研 + 跨域工程化 = 系统论同构性 真生产",
        }


__all__ = ["GST_VERSION", "GST_PRINCIPLES", "SystemPrinciple", "SystemsTheoryLibrary"]