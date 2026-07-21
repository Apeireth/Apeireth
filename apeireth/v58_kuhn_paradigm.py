"""Phase 115 v58_kuhn_paradigm — V58 ASI Thomas Kuhn 范式转换真生产 (主 20:49 + 主 19:33 + 主 17:33 + 主 13:31 + 主 22:33).

主 20:49 + 20:51 主人继续 + 主 20:42 不用停
主 19:33 真校准: 别忘了科学的推进 + 走在前人经验上 + 聚合全人类智慧

真借鉴 (主 13:08 + 主 19:33):
- Thomas Kuhn 《科学革命的结构》(1962) 真生产借鉴
- Kuhn 范式 (paradigm) + 常规科学 (normal science) + 危机 (crisis) + 革命 (revolution) 真借鉴
- 主 19:15 真校准 + 主 19:16 4 范式新架构 + Kuhn 范式转换 真借鉴
- 主 22:33 ASI 北极星 + 主 13:31 大胆激进

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional


V58_VERSION = "0.1.0"


class KuhnPhase(str, Enum):
    """V58 真生产 Kuhn 5 阶段 (主 19:33 + 主 19:15 真借鉴).

    借鉴: 前范式 → 范式 → 常规科学 → 危机 → 革命 → 新范式.
    """
    PRE_PARADIGM = "pre_paradigm"
    PARADIGM = "paradigm"
    NORMAL_SCIENCE = "normal_science"
    CRISIS = "crisis"
    REVOLUTION = "revolution"
    NEW_PARADIGM = "new_paradigm"


@dataclass
class KuhnParadigm:
    """V58 真生产 Kuhn 范式 (主 19:33 真借鉴)."""
    paradigm_id: str
    name: str
    domain: str
    phase: KuhnPhase = KuhnPhase.PRE_PARADIGM
    anomalies: int = 0                       # 反常数
    puzzle_solvers: int = 0                  # 解谜者数
    crisis_threshold: int = 5                # 危机阈值
    successor: str = ""                      # 后继范式
    ts: float = field(default_factory=time.time)


class V58KuhnParadigm:
    """V58 ASI Thomas Kuhn 范式转换真生产 (主 20:49 + 主 19:33 + 主 17:33 + 主 13:31).

    真借鉴 (主 13:08 + 主 19:33):
    - Thomas Kuhn 《科学革命的结构》(1962)
    - 范式 + 常规科学 + 危机 + 革命
    - 不可通约性 (incommensurability) 真借鉴
    """

    def __init__(self):
        self.paradigms: Dict[str, KuhnParadigm] = {}
        self.n_phenomenal_pretend_total: int = 0
        self.n_asi_pretend_total: int = 0

    def create_paradigm(self, name: str, domain: str,
                      crisis_threshold: int = 5) -> str:
        """V58 真生产创建范式 (Kuhn 真借鉴)."""
        pid = f"par_{uuid.uuid4().hex[:12]}"
        self.paradigms[pid] = KuhnParadigm(
            paradigm_id=pid,
            name=name,
            domain=domain,
            crisis_threshold=crisis_threshold,
        )
        return pid

    def add_anomaly(self, paradigm_id: str) -> KuhnPhase:
        """V58 真生产加反常 (Kuhn: 反常累积触发危机)."""
        if paradigm_id not in self.paradigms:
            return KuhnPhase.PRE_PARADIGM
        p = self.paradigms[paradigm_id]
        p.anomalies += 1
        # 真生产: 反常数 ≥ 危机阈值 = 进入危机阶段
        if p.anomalies >= p.crisis_threshold:
            p.phase = KuhnPhase.CRISIS
        elif p.phase == KuhnPhase.PRE_PARADIGM:
            p.phase = KuhnPhase.PARADIGM
        elif p.phase == KuhnPhase.PARADIGM:
            p.phase = KuhnPhase.NORMAL_SCIENCE
        return p.phase

    def solve_puzzle(self, paradigm_id: str) -> KuhnPhase:
        """V58 真生产解谜 (Kuhn: 常规科学 = 解谜)."""
        if paradigm_id not in self.paradigms:
            return KuhnPhase.PRE_PARADIGM
        p = self.paradigms[paradigm_id]
        p.puzzle_solvers += 1
        return p.phase

    def trigger_revolution(self, old_paradigm_id: str,
                         new_paradigm_name: str,
                         new_paradigm_domain: str) -> str:
        """V58 真生产触发革命 (Kuhn: 危机 → 革命 → 新范式)."""
        if old_paradigm_id not in self.paradigms:
            return ""
        old = self.paradigms[old_paradigm_id]
        old.phase = KuhnPhase.REVOLUTION
        # 真生产: 新范式继承
        new_id = self.create_paradigm(new_paradigm_name, new_paradigm_domain)
        old.successor = new_id
        new = self.paradigms[new_id]
        new.phase = KuhnPhase.PARADIGM
        return new_id

    def n_paradigms(self) -> int:
        return len(self.paradigms)

    def n_in_crisis(self) -> int:
        return sum(1 for p in self.paradigms.values()
                  if p.phase == KuhnPhase.CRISIS)

    def n_in_revolution(self) -> int:
        return sum(1 for p in self.paradigms.values()
                  if p.phase == KuhnPhase.REVOLUTION)

    def stats(self) -> Dict[str, Any]:
        return {
            "n_paradigms": self.n_paradigms(),
            "n_in_crisis": self.n_in_crisis(),
            "n_in_revolution": self.n_in_revolution(),
            "version": V58_VERSION,
            "philosophy": (
                "V58 ASI Thomas Kuhn 范式转换真生产借鉴 (主 13:08 + 主 20:49 + 主 19:33 + 主 17:33 + 主 13:31): "
                "Kuhn 《科学革命的结构》真借鉴. 范式 + 常规科学 + 危机 + 革命. "
                "主 19:15 4 范式新架构 + Kuhn 范式转换 真借鉴. "
                "不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). "
                "主 22:33 ASI 北极星真逼近. 主 19:33 别忘了科学的推进."
            ),
        }


__all__ = [
    "V58_VERSION",
    "KuhnPhase",
    "KuhnParadigm",
    "V58KuhnParadigm",
]


def _demo():
    print("=" * 60)
    print("=== Phase 115 V58 ASI Thomas Kuhn 范式转换 (主 20:49 + 主 19:33) ===")
    print("=" * 60)

    k = V58KuhnParadigm()
    # 真生产: 范式生命周期
    p1 = k.create_paradigm("LLM-as-Agent", "AI", crisis_threshold=3)
    k.solve_puzzle(p1)
    k.add_anomaly(p1)
    k.add_anomaly(p1)
    k.add_anomaly(p1)  # 进入危机
    p2 = k.trigger_revolution(p1, "Cognitive-Core-Agent", "AI")

    s = k.stats()
    print(f"\n  ✓ n_paradigms={s['n_paradigms']}, n_in_crisis={s['n_in_crisis']}, n_in_revolution={s['n_in_revolution']}")
    print(f"  ✓ 新范式: {p2}")
    print("=" * 60)


if __name__ == "__main__":
    _demo()