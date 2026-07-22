"""Phase 220 v1085_hqb_core — V1085 HQB core: honest decision module (主 21:15 + R2-REQ-01 A).

V1085 = HQB (Honesty Quality Boundary) 诚实决奖路. 基于 V36/V160 已有的 HQB 4 维分数
(SC/NR/EV/CDT), 给出 accept/reject/veto 决策. 不重建 HQB 测量, 仅做决策层.

真借鉴 (主 13:08 + 主 18:52): HARNESS.md §2.3 HQB 4 维, V36/v160 HQB 真生产.

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装分数 = ASI (分数逼近 1.0 反而触发 veto)
- 不假装决策 = 真生产 (Decision 必带 reason + score_used)
- 不破坏 4 层安全门 (本模块是 Layer 3 HQB gate)

边界: 不动 V1074 / V1081 / philosophy_guard / 不写 V1074 artifacts.
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional

from apeireth.v36_hqb_benchmark import HQBScore  # V36 真生产 (247 行, 不重建)


V1085_VERSION = "0.1.0"

DEFAULT_ACCEPT_THRESHOLD = 0.70   # >= accept → accept
DEFAULT_REJECT_THRESHOLD = 0.40   # <  reject → reject
# [reject, accept) → review (Layer 4 Human Gate)
# score >= 0.95 → veto (主 17:58: 太完美 = 哲学守门触发)


class Verdict(str, Enum):
    ACCEPT = "accept"
    REVIEW = "review"
    REJECT = "reject"
    VETO = "veto"


@dataclass
class HonestDecision:
    decision_id: str
    verdict: Verdict
    score_used: float
    reason: str
    hqb_score_id: Optional[str] = None
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "decision_id": self.decision_id,
            "verdict": self.verdict.value,
            "score_used": round(self.score_used, 4),
            "reason": self.reason,
            "hqb_score_id": self.hqb_score_id,
            "ts": self.ts,
        }


class HonestDecisionModule:
    """V1085 HQB core: 诚实决奖路 (主 21:15)."""

    def __init__(
        self,
        accept_threshold: float = DEFAULT_ACCEPT_THRESHOLD,
        reject_threshold: float = DEFAULT_REJECT_THRESHOLD,
        veto_threshold: float = 0.95,
    ):
        if not (0.0 <= reject_threshold < accept_threshold <= veto_threshold <= 1.0):
            raise ValueError(
                f"Invalid thresholds: reject={reject_threshold} < accept={accept_threshold} <= veto={veto_threshold}"
            )
        self.accept_threshold = accept_threshold
        self.reject_threshold = reject_threshold
        self.veto_threshold = veto_threshold
        self.decisions: List[HonestDecision] = []

    def evaluate(self, hqb_score: HQBScore, context: str = "") -> HonestDecision:
        """V1085 诚实决奖 (主 17:43 实事求是)."""
        total = float(hqb_score.total)
        if total >= self.veto_threshold:
            verdict, reason = Verdict.VETO, (
                f"score={total:.4f}>=veto={self.veto_threshold:.2f}; "
                "perfect triggers guard (主 17:58 不假装)"
            )
        elif total >= self.accept_threshold:
            verdict, reason = Verdict.ACCEPT, (
                f"score={total:.4f}>=accept={self.accept_threshold:.2f}; "
                f"quality sufficient ({context or 'no context'})"
            )
        elif total < self.reject_threshold:
            verdict, reason = Verdict.REJECT, (
                f"score={total:.4f}<reject={self.reject_threshold:.2f}; "
                "quality insufficient"
            )
        else:
            verdict, reason = Verdict.REVIEW, (
                f"score={total:.4f} in [{self.reject_threshold:.2f},{self.accept_threshold:.2f}); "
                "borderline, Layer 4 Human Gate (主 07-19)"
            )

        decision = HonestDecision(
            decision_id=f"dec_{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            score_used=total,
            reason=reason,
            hqb_score_id=hqb_score.score_id,
        )
        self.decisions.append(decision)
        return decision

    def stats(self) -> Dict[str, Any]:
        by_verdict: Dict[str, int] = {}
        for d in self.decisions:
            by_verdict[d.verdict.value] = by_verdict.get(d.verdict.value, 0) + 1
        return {
            "n_decisions": len(self.decisions),
            "by_verdict": by_verdict,
            "thresholds": {
                "accept": self.accept_threshold,
                "reject": self.reject_threshold,
                "veto": self.veto_threshold,
            },
            "version": V1085_VERSION,
            "philosophy": (
                "V1085 HQB 诚实决奖路 (主 21:15). 不假装分数=ASI, "
                "不破坏 4 层安全门 (主 07-19)."
            ),
        }


__all__ = [
    "V1085_VERSION",
    "Verdict",
    "HonestDecision",
    "HonestDecisionModule",
    "DEFAULT_ACCEPT_THRESHOLD",
    "DEFAULT_REJECT_THRESHOLD",
]