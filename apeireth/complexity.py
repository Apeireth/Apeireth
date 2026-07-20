"""Phase 37 ComplexityHub — 复杂度科学综合 (CSH 跨域调研借鉴).

主人 21:30 跨域调研 AnySearch:
  "Complexity Science Hub" (https://csh.ac.at)
  Hildegard Meyer-Ortmanns 等

Complexity Hub 真生产内容:
  - 多学科综合 = 物理学/生物学/信息学/社会学
  - 涌现是 cross-domain 现象
  - Complex systems 有 common mathematical laws
  - Phase 37 = 中央 AI 的 cross-domain 综合器

Karpathy 准则:
  1. Think Before Coding: cross-domain = systematic borrowing
  2. Simplicity First: ComplexityHub = (domains, laws, applications)
  3. Surgical Changes: 不改 CrossDomainResearch, 加 ComplexityHub
  4. Goal-Driven Execution: verifiable = 跨域借鉴可追溯
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import List


COMPLEXITY_VERSION = "0.1.0"


# Complexity Hub 跨域数学规律 (CSH 真生产借鉴)
COMPLEXITY_LAWS: dict = {
    "power_law": "幂律 (Zipf / Pareto): scale-free networks, 复杂度科学核心",
    "self_organized_criticality": "自组织临界态 (SOC): 沙堆模型",
    "phase_transition": "相变 (Meyer-Ortmanns): 涌现的数学语言",
    "scale_invariance": "尺度不变: 不同尺度看系统都有相似结构",
    "1_f_noise": "1/f 噪声: 长程关联, memory of past",
}


@dataclass
class CrossDomainApplication:
    """跨域应用记录."""
    app_id: str
    domain_from: str                # 借鉴自哪个域
    domain_to: str                  # 应用到中央 AI 的哪个域
    law_name: str
    analogy: str                    # 借鉴的类比
    evidence: str = ""
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class ComplexityHub:
    """Complexity Science Hub 跨域综合 — 中央 AI 是复杂系统.

    主人 21:00 "跨多个界调研" + 主人 21:14 AnySearch + 主人 21:22 并行:
      - 中央 AI 是复杂系统 (complex system)
      - 跨域借鉴是 systematic, 不随机
      - 数学规律可借鉴 (power law, SOC, phase transition, etc.)
    """

    def __init__(self):
        self.laws = COMPLEXITY_LAWS.copy()
        self.applications: list[CrossDomainApplication] = []

    def record_application(self, domain_from: str, domain_to: str,
                         law_name: str, analogy: str, evidence: str = "") -> CrossDomainApplication:
        """记录一个跨域应用."""
        if law_name not in self.laws:
            raise ValueError(f"Unknown law: {law_name}. Available: {list(self.laws.keys())}")
        a = CrossDomainApplication(
            app_id=uuid.uuid4().hex[:12],
            domain_from=domain_from,
            domain_to=domain_to,
            law_name=law_name,
            analogy=analogy,
            evidence=evidence,
        )
        self.applications.append(a)
        return a

    def stats(self) -> dict:
        domains = sorted(set([a.domain_from for a in self.applications] +
                            [a.domain_to for a in self.applications]))
        return {
            "n_laws": len(self.laws),
            "n_applications": len(self.applications),
            "domains_covered": domains,
            "complexity_hub": "CSH Vienna 真生产借鉴 — 跨域数学规律是 complex system 通用语言",
        }


__all__ = [
    "COMPLEXITY_VERSION",
    "COMPLEXITY_LAWS",
    "CrossDomainApplication",
    "ComplexityHub",
]