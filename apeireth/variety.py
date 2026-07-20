"""Phase 32 Requisite Variety — Ashby 必要多样性律工程化.

主人 21:30 跨域调研 AnySearch:
  "W. Ross Ashby, Cybernetics and Requisite Variety (1956)"
    (https://www.panarchy.org/ashby/variety.1956.html)
  "Ashby's Law and AI control" (https://digifesto.com/2019/09/22/ashbys-law-and-ai-control/)

Ashby 必要多样性律 (Law of Requisite Variety, 1956):
  只有关键变量的多样性 ≥ 环境的多样性, 系统才能控制环境
  公式: V_controller ≥ V_environment (informational)

对 ASI 中央 AI 的意义:
  - 中央 AI 必须有足够多样性 persona/skill/memory 来匹配任务多样性
  - 主人 12:14 "干什么就组一个什么的专家团" = Requisite Variety
  - 主人 17:50 "涌现 自组织" = 增加 variety → 涌现
  - 主人 14:48 "聚集全人类智慧" = 跨域借鉴提升 variety

Karpathy 准则:
  1. Think Before Coding: variety = env_diversity_complexity
  2. Simplicity First: Variety = numeric, 加 bits 概念
  3. Surgical Changes: 不改 SelfOrgTeam, 加 RequisiteVariety 分析
  4. Goal-Driven Execution: verifiable = variety(env) - variety(controller)
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict


ASHBY_VARIETY_VERSION = "0.1.0"


@dataclass
class VarietyMeasure:
    """多样性度量 — Shannon 熵 + Ashby 必要多样性."""
    measure_id: str
    name: str
    shannon_bits: float       # Shannon 熵 (多样性 bits)
    n_categories: int         # 分类数
    sample_size: int
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class RequisiteVarietyCalculator:
    """Ashby 必要多样性律 — 中央 AI 多样性分析器.

    主人 12:14 "干什么就组一个什么的专家团" = 临时构造环境多样性
    主人 17:50 "涌现 自组织" = 增加多样性 → 涌现
    主人 14:48 "聚集全人类智慧" = 跨域借鉴 → 增加多样性
    """

    def __init__(self):
        self.measures: dict[str, VarietyMeasure] = {}
        self.history: list = []

    def shannon_entropy(self, counts: list[int]) -> VarietyMeasure:
        """计算 Shannon entropy (bits) — 衡量多样性.

        H = -Σ p_i log2(p_i)
        """
        import math
        total = sum(counts)
        if total == 0:
            h = 0.0
        else:
            h = -sum((c / total) * math.log2(c / total) for c in counts if c > 0)
        m = VarietyMeasure(
            measure_id=uuid.uuid4().hex[:12],
            name="shannon",
            shannon_bits=h,
            n_categories=len(counts),
            sample_size=total,
        )
        self.measures[m.measure_id] = m
        return m

    def requisite_check(self, env_variety: VarietyMeasure, controller_variety: VarietyMeasure,
                       context: str = "") -> dict:
        """Ashby 必要多样性检查: V_controller >= V_environment.

        True: 中央 AI 多样性足够 (主人 12:14 临时团涌现)
        False: 中央 AI 多样性不足 (需要增加 diversity)
        """
        deficit = env_variety.shannon_bits - controller_variety.shannon_bits
        ok = deficit <= 0
        r = {
            "env_bits": env_variety.shannon_bits,
            "controller_bits": controller_variety.shannon_bits,
            "deficit": deficit,
            "requisite_satisfied": ok,
            "context": context,
            "ashby": "V_controller ≥ V_environment 是必要, 不是充分",
            "ts": time.time(),
        }
        self.history.append(r)
        return r

    def analyze_personas(self, personas: dict[str, dict]) -> dict:
        """分析中央 AI persona 系统的多样性.

        主人 12:14: 4 archetypes = 调度者/学习者/思考者/助手
        """
        counts = [len(personas)]  # 简化:1 group
        v = self.shannon_entropy(counts)
        return {
            "persona_count": len(personas),
            "variety_bits": v.shannon_bits,
            "interpretation": (
                f"中央 AI 有 {len(personas)} 个 persona 默认, "
                "可扩展 (Ashby 必要多样性 可涌现临 persona)"
            ),
        }

    def stats(self) -> dict:
        return {
            "n_measures": len(self.measures),
            "n_history": len(self.history),
            "ashby_law": True,
        }


__all__ = ["ASHBY_VARIETY_VERSION", "VarietyMeasure", "RequisiteVarietyCalculator"]