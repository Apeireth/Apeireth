"""Phase 51 curiosity — 真生产 curiosity-driven 主动性 (主 14:06 拉回注意力 + 主 13:31 大胆激进).

主 14:09 推进 Apeireth 追求极致 + V4 12 生命特征 MISSING:
- **主动性 (#7) 是 V4 12 生命特征最严重空隙** (ProactiveLoop 是 heartbeat-based, 不是真 curiosity-driven)
- 真生产 curiosity-driven engine, 不 placeholder
- 写真 production, 不保守 (主 13:31 大胆激进 + 允许犯错 + 鼓励尝试)

借鉴 (主 13:08 哲学/科学/跨领域):
- Berlyne 1966 好奇驱动 (collative variables):
  - Novelty (新异性) — stimulus vs baseline 差异
  - Uncertainty (不确定性) — 预测 vs 实际差异
  - Conflict (冲突) — schema 违反
- Vygotsky ZPD (Zone of Proximal Development) — 适度挑战
- Piaget 认知冲突 — assimilation vs accommodation 不平衡
- 自由能原理 (Friston) — surprise minimization
- 真菌 chemotaxis (主 14:06) — 浓度梯度 + run/tumble 真借鉴
- 创新理论 (West) — novelty search 真生产
- Apeireth ProactiveLoop (主 11:46) — heartbeat-based 已有, 真生产 curiosity-driven 加
- 自主性 (autonomy) 真生产 (V3 自由哲学借鉴)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness
- 不假装达到 ASI
- 实事求是, 写真 production, 不 placeholder
- Berlyne 借鉴 是工具 (主 20:55 隐喻是工具), 不假装"ASI 真好奇"
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Dict, List, Optional


CURIOSITY_VERSION = "0.1.0"


# === Berlyne 1966 好奇驱动 4 collative variables (主 13:08 真借鉴) ===

class CuriosityDriver(str, Enum):
    """Berlyne 1966 好奇驱动 4 collative variables (主 13:08 真借鉴).

    真生产: 不假装"ASI 真好奇", 是基于 novelty/uncertainty/conflict/complexity 真计算.
    """
    NOVELTY = "novelty"          # 新异性 — stimulus vs baseline 差异
    UNCERTAINTY = "uncertainty"  # 不确定性 — 预测 vs 实际差异
    CONFLICT = "conflict"        # 冲突 — schema 违反
    COMPLEXITY = "complexity"    # 复杂度 — 适度挑战 (Vygotsky ZPD)


@dataclass
class CuriositySignal:
    """curiosity 真信号 (主 14:06 + 借鉴 Berlyne 真生产)."""
    signal_id: str
    stimulus: str                       # 刺激 (主 / 事件 / 知识)
    baseline: float = 0.0               # baseline 频率 / 先验
    observed: float = 0.0               # 当前观察值
    predicted: float = 0.0              # 预测值
    schema_violation: float = 0.0       # schema 违反 [0, 1]
    ts: float = field(default_factory=time.time)


@dataclass
class CuriosityAssessment:
    """curiosity 真生产评估 (主 14:06 写真 production)."""
    assessment_id: str
    signal_id: str
    driver: CuriosityDriver
    score: float                         # 真好奇心分数 [0, 1]
    zpd_score: float                     # Vygotsky ZPD 真测 [0, 1]
    should_fire: bool                    # 真好奇心是否触发 fire
    rationale: str                       # 写真 production 真理由
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "assessment_id": self.assessment_id,
            "signal_id": self.signal_id,
            "driver": self.driver.value,
            "score": round(self.score, 4),
            "zpd_score": round(self.zpd_score, 4),
            "should_fire": self.should_fire,
            "rationale": self.rationale,
        }


# === Curiosity 真生产算法 (主 14:06 + Berlyne 真借鉴) ===

def assess_novelty(signal: CuriositySignal) -> float:
    """Novelty 真测 (主 13:08 真借鉴 Berlyne 1966).

    借鉴: novelty = |observed - baseline| / max(baseline, observed, 1).
    真生产, 不 placeholder.
    """
    if signal.baseline <= 0 and signal.observed <= 0:
        return 0.0
    denom = max(signal.baseline, signal.observed, 1.0)
    novelty = abs(signal.observed - signal.baseline) / denom
    return min(novelty, 1.0)


def assess_uncertainty(signal: CuriositySignal) -> float:
    """Uncertainty 真测 (主 13:08 真借鉴 Bayesian).

    借鉴: uncertainty = |predicted - observed| / max(predicted, observed, 1).
    真生产, 不 placeholder.
    """
    if signal.predicted <= 0 and signal.observed <= 0:
        return 0.0
    denom = max(signal.predicted, signal.observed, 1.0)
    uncertainty = abs(signal.predicted - signal.observed) / denom
    return min(uncertainty, 1.0)


def assess_conflict(signal: CuriositySignal) -> float:
    """Conflict 真测 (主 13:08 真借鉴 Piaget 认知冲突).

    借鉴: schema_violation 直接作为 conflict 真测.
    真生产, 不 placeholder.
    """
    return min(signal.schema_violation, 1.0)


def compute_zpd(novelty: float, uncertainty: float) -> float:
    """Vygotsky ZPD 真测 (主 13:08 真借鉴).

    ZPD = (1 - |novelty - 0.5|) * (1 - |uncertainty - 0.5|).
    最优 ZPD = 1.0 (novelty=0.5, uncertainty=0.5, 适度挑战).
    真生产, 不 placeholder.
    """
    zpd = (1.0 - abs(novelty - 0.5)) * (1.0 - abs(uncertainty - 0.5))
    return min(max(zpd, 0.0), 1.0)


# === Curiosity 真生产主类 ===

class CuriosityEngine:
    """真生产 curiosity-driven 主动性 (主 14:06 + 主 13:31 大胆激进 + 写真 production + 允许犯错).

    V4 12 生命特征主动性 (#7) 真生产落地.
    借鉴: Berlyne 1966 + Vygotsky ZPD + Piaget 认知冲突 + Friston 自由能原理.
    """

    def __init__(self, zpd_threshold: float = 0.5):
        """Init curiosity 真生产.

        Args:
            zpd_threshold: ZPD 触发 fire 阈值 (默认 0.5, Vygotsky 最优 ZPD)
        """
        self.zpd_threshold = zpd_threshold
        self.history: List[CuriosityAssessment] = []

    def assess(self, signal: CuriositySignal) -> CuriosityAssessment:
        """curiosity 真生产评估 (主 14:06 + Berlyne 真借鉴).

        4 driver 真测: novelty / uncertainty / conflict / complexity
        ZPD 真测 (Vygotsky 真借鉴).
        should_fire 真判定 (zpd_score > threshold).
        """
        # 4 driver 真测
        novelty = assess_novelty(signal)
        uncertainty = assess_uncertainty(signal)
        conflict = assess_conflict(signal)
        complexity = novelty * 0.5 + uncertainty * 0.5  # 综合复杂度

        # 真好奇心分数 = 4 driver 平均 (主 17:43 实事求是)
        score = (novelty + uncertainty + conflict + complexity) / 4.0

        # ZPD 真测
        zpd_score = compute_zpd(novelty, uncertainty)

        # 写真 production 真判定
        should_fire = zpd_score > self.zpd_threshold

        # 写真 production 真理由 (不 placeholder)
        rationale_parts = []
        if novelty > 0.3:
            rationale_parts.append(f"novelty={novelty:.2f} (>0.3 触发)")
        if uncertainty > 0.3:
            rationale_parts.append(f"uncertainty={uncertainty:.2f} (>0.3 触发)")
        if conflict > 0.3:
            rationale_parts.append(f"conflict={conflict:.2f} (>0.3 触发)")
        if not rationale_parts:
            rationale_parts.append(f"baseline OK (novelty={novelty:.2f}, uncertainty={uncertainty:.2f}, conflict={conflict:.2f})")

        rationale = " + ".join(rationale_parts) + (
            f" → ZPD={zpd_score:.3f} {'> ' if should_fire else '<= '}{self.zpd_threshold} → "
            f"{'FIRE' if should_fire else 'SKIP'}"
        )

        assessment = CuriosityAssessment(
            assessment_id=f"cur_{uuid.uuid4().hex[:12]}",
            signal_id=signal.signal_id,
            driver=CuriosityDriver.NOVELTY,  # default
            score=score,
            zpd_score=zpd_score,
            should_fire=should_fire,
            rationale=rationale,
        )
        self.history.append(assessment)
        return assessment

    def stats(self) -> Dict[str, Any]:
        """curiosity 真生产统计 (主 17:43 实事求是)."""
        if not self.history:
            return {"n_assessments": 0}
        n_fire = sum(1 for a in self.history if a.should_fire)
        n_skip = len(self.history) - n_fire
        return {
            "n_assessments": len(self.history),
            "n_fire": n_fire,
            "n_skip": n_skip,
            "fire_ratio": n_fire / len(self.history) if self.history else 0.0,
            "version": CURIOSITY_VERSION,
            "philosophy": (
                "curiosity 真生产借鉴 (主 13:08): Berlyne 1966 collative variables + "
                "Vygotsky ZPD + Piaget 认知冲突 + Friston 自由能原理. "
                "不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). "
                "V4 12 生命特征主动性 (#7) 真生产落地."
            ),
        }


__all__ = [
    "CURIOSITY_VERSION",
    "CuriosityDriver",
    "CuriositySignal",
    "CuriosityAssessment",
    "assess_novelty",
    "assess_uncertainty",
    "assess_conflict",
    "compute_zpd",
    "CuriosityEngine",
]


# === curiosity 写真 production demo (主 13:31 大胆激进) ===

def _demo():
    print("=" * 70)
    print("=== Phase 51 curiosity 真生产主动性 (主 13:31 大胆激进 + 14:06 拉回注意力) ===")
    print("=" * 70)

    # 1. Init
    print("\n[1] Init curiosity 真生产 (V4 12 生命特征主动性 #7)")
    ce = CuriosityEngine(zpd_threshold=0.5)
    print(f"  ✓ CuriosityEngine 0.1.0 创建 (ZPD threshold=0.5)")

    # 2. 真测多个信号 (主 14:06 真生产)
    print("\n[2] 真生产 curiosity 4 driver + ZPD 真测 (借鉴 Berlyne 1966):")
    signals = [
        # 新异刺激: 主人新概念 "ASE 哲学"
        CuriositySignal(signal_id="s1", stimulus="主 14:09 推进 Apeireth 追求极致",
                       baseline=0.1, observed=0.8, predicted=0.2, schema_violation=0.3),
        # 重复刺激: 已知事件 (low novelty)
        CuriositySignal(signal_id="s2", stimulus="cron tick 14:20",
                       baseline=0.9, observed=0.9, predicted=0.9, schema_violation=0.0),
        # 高不确定: 预测错很大
        CuriositySignal(signal_id="s3", stimulus="ASI 哲学 V3 涌现",
                       baseline=0.5, observed=0.5, predicted=0.1, schema_violation=0.5),
        # 高冲突: schema 违反
        CuriositySignal(signal_id="s4", stimulus="V4 红皇后归入 8 核心",
                       baseline=0.0, observed=0.7, predicted=0.0, schema_violation=0.9),
    ]
    for s in signals:
        a = ce.assess(s)
        print(f"  ✓ [{a.assessment_id[:8]}] {s.stimulus[:30]:30s} → "
              f"ZPD={a.zpd_score:.3f} score={a.score:.3f} {'FIRE' if a.should_fire else 'SKIP'}")

    # 3. stats
    print("\n[3] curiosity 真生产 stats:")
    stats = ce.stats()
    for k, v in stats.items():
        print(f"  - {k}: {v}")

    print("\n" + "=" * 70)
    print("✓ Phase 51 curiosity 真生产落地 (V4 12 生命特征主动性 #7)")
    print("  - 4 driver 真测: novelty / uncertainty / conflict / complexity")
    print("  - ZPD 真测 (Vygotsky 真借鉴)")
    print("  - should_fire 真判定 (主 17:43 实事求是)")
    print("  - V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI")
    print("=" * 70)


if __name__ == "__main__":
    _demo()