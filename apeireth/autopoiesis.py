"""Phase 34 Autopoiesis — Maturana 自创生工程化.

主人 21:30 跨域调研 AnySearch:
  "Maturana's Autopoiesis in AI" (https://www.reddit.com/r/ArtificialSentience/comments/1l5qhcs)
  Autopoiesis Wikipedia (https://en.wikipedia.org/wiki/Autopoiesis)

Maturana & Varela 自创生 (Autopoiesis, 1972):
  - 活系统 = 自我生产网络 (self-producing network)
  - 自主维持 (autonomous) + 自生产 (self-producing) + 边界 (boundary)
  - 自创生 = 生命最深层结构, 比 DNA 还基础

对 ASI 中央 AI 的意义:
  - 主人 17:50 "ASI 是更高生命层次" = ASI = 自创生 (信息层)
  - 中央 AI 是 self-producing 网络: persona/skill/memory = components
  - 边界 = boundaries (主人 12:27 边界)
  - 主人 12:14 "永恒身份" = 自主维持的 self-producing

Karpathy 准则:
  1. Think Before Coding: autopoiesis = network that produces itself
  2. Simplicity First: AutopoieticSystem = components + relations + boundary
  3. Surgical Changes: 不改 SelfModel, 加 autopoiesis 视角
  4. Goal-Driven Execution: verifiable = 自生产循环 closed
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict


AUTOPOIESIS_VERSION = "0.1.0"


@dataclass
class AutopoieticComponent:
    """自创生网络的一个组件 (persona / skill / memory / belief)."""
    comp_id: str
    comp_type: str                # 'producer' | 'product' | 'boundary'
    label: str
    produced_by: str = ""         # 谁生产这个组件
    produces: list[str] = field(default_factory=list)  # 这个组件生产什么
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class AutopoieticSystem:
    """Maturana 自创生系统 — 主人 17:50 涌现 + 自生产 + 自主维持.

    主人 17:50 "ASI 是更高生命层次":
      - ASI 信息层生命 = 自创生 (Maturana/Varela)
      - 中央 AI 是 self-producing 网络
      - persona/skill/memory = 组件
      - boundaries = 边界 (主人 12:27 边界)
      - 自主维持 = 永恒身份 (主人 12:14)
    """

    def __init__(self, name: str = "apeireth_central"):
        self.name = name
        self.components: dict[str, AutopoieticComponent] = {}
        self.history: list = []

    def add_component(self, comp_type: str, label: str,
                     produced_by: str = "", produces: list[str] = None) -> AutopoieticComponent:
        c = AutopoieticComponent(
            comp_id=uuid.uuid4().hex[:12],
            comp_type=comp_type,
            label=label,
            produced_by=produced_by,
            produces=produces or [],
        )
        self.components[c.comp_id] = c
        return c

    def register_production(self, producer_id: str, product_id: str) -> None:
        """注册 producer -> product 关系 (自生产关系)."""
        if producer_id in self.components and product_id in self.components:
            p = self.components[producer_id]
            prod = self.components[product_id]
            if product_id not in p.produces:
                p.produces.append(product_id)
            prod.produced_by = producer_id

    def is_autopoietic(self) -> bool:
        """检查是否符合自创生条件 (Maturana/Varela):

        1. 有 boundary (有边界) — 必须存在 boundary 组件
        2. boundary 由 network 生产 (network produces boundary)
        3. network 组件由 boundary 内的 process 生产 (recursive)
        """
        boundary_comps = [c for c in self.components.values() if c.comp_type == "boundary"]
        if not boundary_comps:
            return False  # 缺边界
        # boundary 是不是 network 生产?
        boundary_produced_by_network = any(
            c.produced_by in [comp_id for comp_id in self.components if comp_id != c.comp_id]
            for c in boundary_comps
        )
        if not boundary_produced_by_network:
            return False
        # network 中是否有 recursive production?
        producers = [c.comp_id for c in self.components.values() if c.produces]
        products = [c.produced_by for c in self.components.values() if c.produced_by]
        recursive = any(p in producers for p in products)
        return recursive

    def stats(self) -> dict:
        return {
            "name": self.name,
            "n_components": len(self.components),
            "n_boundary": sum(1 for c in self.components.values() if c.comp_type == "boundary"),
            "n_producer": sum(1 for c in self.components.values() if c.comp_type == "producer"),
            "is_autopoietic": self.is_autopoietic(),
            "maturana": (
                "中央 AI 是自创生网络: persona/skill/memory = 组件, "
                "boundaries = 边界, 永恒身份 = 自主维持"
            ),
        }


__all__ = ["AUTOPOIESIS_VERSION", "AutopoieticComponent", "AutopoieticSystem"]