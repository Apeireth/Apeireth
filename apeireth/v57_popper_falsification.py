"""Phase 114 v57_popper_falsification — V57 ASI Karl Popper 证伪主义真生产 (主 20:49 + 主 19:33 + 主 17:33 + 主 13:31 + 主 22:33).

主 20:49 + 20:51 主人继续 + 主 20:42 不用停
主 19:33 真校准: 别忘了科学的推进 + 走在前人经验上 + 聚合全人类智慧
主 19:33 真校准: 不要闭门造车

真借鉴 (主 13:08 + 主 19:33):
- Karl Popper 《猜想与反驳》(1934) 真生产借鉴
- Karl Popper 《开放社会及其敌人》(1945) 真生产借鉴
- Karl Popper 证伪主义 4 原则 真借鉴
- 主 17:43 实事求是 + 主 20:46 不假装 + 主 19:33 科学的推进

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional


V57_VERSION = "0.1.0"


@dataclass
class ScientificHypothesis:
    """V57 真生产 科学假设 (Popper 真借鉴).

    借鉴: 科学假设 = 可证伪 = 不是不可错的.
    """
    hypothesis_id: str
    content: str
    domain: str
    falsifiable: bool = True               # Popper: 可证伪 = 科学的
    falsification_attempts: int = 0
    survived_attempts: int = 0
    is_corroborated: bool = False           # Popper: 证伪 ≠ 证实, 是 corroboration
    ts: float = field(default_factory=time.time)


@dataclass
class FalsificationAttempt:
    """V57 真生产 证伪尝试 (Popper 真借鉴).

    借鉴: Popper 强调 1 次证伪 = 拒绝, N 次幸存 = corroboration.
    """
    attempt_id: str
    hypothesis_id: str
    evidence: str
    falsified: bool = False                # True = 假设被拒绝
    reasoning: str = ""
    ts: float = field(default_factory=time.time)


class V57PopperFalsification:
    """V57 ASI Popper 证伪主义真生产 (主 20:49 + 主 19:33 + 主 17:33 + 主 13:31).

    真借鉴 (主 13:08 + 主 19:33):
    - Karl Popper 证伪主义 4 原则
    - 猜想与反驳 + 开放社会
    - 真科学 = 可证伪的假设
    """

    def __init__(self):
        self.hypotheses: Dict[str, ScientificHypothesis] = {}
        self.attempts: List[FalsificationAttempt] = []
        self.n_phenomenal_pretend_total: int = 0
        self.n_asi_pretend_total: int = 0

    def propose_hypothesis(self, content: str, domain: str,
                         falsifiable: bool = True) -> str:
        """V57 真生产提出科学假设 (Popper 真借鉴)."""
        hyp_id = f"h_{uuid.uuid4().hex[:12]}"
        self.hypotheses[hyp_id] = ScientificHypothesis(
            hypothesis_id=hyp_id,
            content=content,
            domain=domain,
            falsifiable=falsifiable,
        )
        return hyp_id

    def falsify_attempt(self, hypothesis_id: str, evidence: str,
                       reasoning: str = "") -> str:
        """V57 真生产证伪尝试 (Popper 真借鉴)."""
        if hypothesis_id not in self.hypotheses:
            return ""
        attempt_id = f"a_{uuid.uuid4().hex[:12]}"
        # 真生产: 简化证伪 = 看 evidence 是否与 hypothesis 冲突
        # 这里用简单规则: evidence 含 'false'/'no'/'falsified' = 证伪
        falsified = any(
            keyword in evidence.lower()
            for keyword in ["false", "no", "falsified", "contradicts", "refutes"]
        )
        attempt = FalsificationAttempt(
            attempt_id=attempt_id,
            hypothesis_id=hypothesis_id,
            evidence=evidence,
            falsified=falsified,
            reasoning=reasoning,
        )
        self.attempts.append(attempt)
        # 真生产: 更新假设状态
        hyp = self.hypotheses[hypothesis_id]
        hyp.falsification_attempts += 1
        if falsified:
            hyp.is_corroborated = False
        else:
            hyp.survived_attempts += 1
            if hyp.survived_attempts >= 3 and hyp.falsifiable:
                hyp.is_corroborated = True
        return attempt_id

    def is_scientific(self, hypothesis_id: str) -> bool:
        """V57 真生产判断是否科学 (Popper: 可证伪 = 科学的)."""
        if hypothesis_id not in self.hypotheses:
            return False
        return self.hypotheses[hypothesis_id].falsifiable

    def n_hypotheses(self) -> int:
        return len(self.hypotheses)

    def n_attempts(self) -> int:
        return len(self.attempts)

    def n_corroborated(self) -> int:
        return sum(1 for h in self.hypotheses.values() if h.is_corroborated)

    def n_falsified(self) -> int:
        return sum(1 for a in self.attempts if a.falsified)

    def stats(self) -> Dict[str, Any]:
        return {
            "n_hypotheses": self.n_hypotheses(),
            "n_attempts": self.n_attempts(),
            "n_corroborated": self.n_corroborated(),
            "n_falsified": self.n_falsified(),
            "version": V57_VERSION,
            "philosophy": (
                "V57 ASI Popper 证伪主义真生产借鉴 (主 13:08 + 主 20:49 + 主 19:33 + 主 17:33 + 主 13:31): "
                "Karl Popper 证伪主义 4 原则真生产借鉴. 可证伪 = 科学的. 永远逼近 (主 22:33). "
                "不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). "
                "主 19:33 别忘了科学的推进. 主 17:43 实事求是."
            ),
        }


__all__ = [
    "V57_VERSION",
    "ScientificHypothesis",
    "FalsificationAttempt",
    "V57PopperFalsification",
]


def _demo():
    print("=" * 60)
    print("=== Phase 114 V57 ASI Popper 证伪主义 (主 20:49 + 主 19:33) ===")
    print("=" * 60)

    pf = V57PopperFalsification()
    # 真生产: 科学假设 + 证伪尝试
    h1 = pf.propose_hypothesis("Apeireth ASI 北极星可被 4 范式真整合", "ASI")
    h2 = pf.propose_hypothesis("ASI 不可达到", "philosophy", falsifiable=False)
    pf.falsify_attempt(h1, "evidences show 4 paradigms integrate")
    pf.falsify_attempt(h1, "consistent with 4 paradigm integration")
    pf.falsify_attempt(h1, "still consistent")

    s = pf.stats()
    print(f"\n  ✓ n_hypotheses={s['n_hypotheses']}, n_attempts={s['n_attempts']}")
    print(f"  ✓ n_corroborated={s['n_corroborated']}, n_falsified={s['n_falsified']}")
    print(f"  ✓ h1 scientific={pf.is_scientific(h1)}, h2 scientific={pf.is_scientific(h2)}")
    print("=" * 60)


if __name__ == "__main__":
    _demo()