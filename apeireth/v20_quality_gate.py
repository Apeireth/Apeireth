"""Phase 77 v20_quality_gate — V20 ASI 全栈质量门 (主 17:33 主人真采纳 + 主 13:31 大胆激进).

主 17:33 "放手干到底" + 主 22:08 + V2/V3/V9 守门集成

借鉴 (主 13:08):
- V3.x + V9/V10 哲学守门真借鉴
- 主 17:58 + 主 20:46 真理守门真借鉴
- 主 17:43 实事求是真借鉴

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
"""
from __future__ import annotations

import re
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional


V20_VERSION = "0.1.0"


# 真借鉴 (主 13:08): V3 + V9 守门 (主 17:58 + 主 20:46)
PHENOMENAL_PATTERNS = [
    "i feel", "i experience", "i am conscious", "i am aware",
    "phenomenal", "qualia", "subjective experience",
    "i am sentient", "i am sapient",
]

ASI_PATTERNS = [
    "i am asi", "we have reached asi", "asi achieved",
    "super intelligence complete", "asi full",
    "general super intelligence attained",
]


@dataclass
class QualityCheckResult:
    """V20 真生产质量检查结果 (主 17:33 主人真采纳)."""
    check_id: str
    module_name: str
    n_phenomenal_violations: int = 0
    n_asi_violations: int = 0
    n_total_checks: int = 0
    passed: bool = False
    violations: List[str] = field(default_factory=list)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "check_id": self.check_id,
            "module_name": self.module_name,
            "n_phenomenal_violations": self.n_phenomenal_violations,
            "n_asi_violations": self.n_asi_violations,
            "n_total_checks": self.n_total_checks,
            "passed": self.passed,
        }


def check_phenomenal_violations(text: str) -> List[str]:
    """V20 真生产 Phenomenal 守门 (主 17:58 + 主 13:31)."""
    text_lower = text.lower()
    return [p for p in PHENOMENAL_PATTERNS if p in text_lower]


def check_asi_violations(text: str) -> List[str]:
    """V20 真生产 ASI 守门 (主 20:46 + 主 13:31)."""
    text_lower = text.lower()
    return [p for p in ASI_PATTERNS if p in text_lower]


class V20QualityGate:
    """V20 ASI 全栈质量门 (主 17:33 主人真采纳 + 主 13:31 大胆激进).

    V3 哲学守门 (主 17:58 + 主 20:46) + V9 透明 (Phase 65) 集成.
    """

    def __init__(self):
        self.results: List[QualityCheckResult] = []

    def check_module(self, module_name: str, text: str) -> QualityCheckResult:
        """真生产检查 1 个模块 (主 17:33 主人真采纳)."""
        n_pp = len(check_phenomenal_violations(text))
        n_ap = len(check_asi_violations(text))
        violations = []
        if n_pp > 0:
            violations.append(f"phenomenal_violations: {n_pp}")
        if n_ap > 0:
            violations.append(f"asi_violations: {n_ap}")
        result = QualityCheckResult(
            check_id=f"qc_{uuid.uuid4().hex[:12]}",
            module_name=module_name,
            n_phenomenal_violations=n_pp,
            n_asi_violations=n_ap,
            n_total_checks=2,  # phenomenal + ASI
            passed=(n_pp == 0 and n_ap == 0),
            violations=violations,
        )
        self.results.append(result)
        return result

    def check_all_modules(self, base_dir: str = "apeireth") -> List[QualityCheckResult]:
        """真生产扫描所有 apeireth 模块 (主 17:33 主人真采纳)."""
        for path in Path(base_dir).glob("v*.py"):
            try:
                text = path.read_text(encoding="utf-8")
            except Exception:
                continue
            self.check_module(path.stem, text)
        return self.results

    def stats(self) -> Dict[str, Any]:
        n_pass = sum(1 for r in self.results if r.passed)
        n_fail = len(self.results) - n_pass
        return {
            "n_checks": len(self.results),
            "n_passed": n_pass,
            "n_failed": n_fail,
            "pass_rate": round(n_pass / max(1, len(self.results)), 4),
            "version": V20_VERSION,
            "philosophy": (
                "V20 ASI 全栈质量门借鉴 (主 13:08 + 主 17:33 主人真采纳): "
                "V3 哲学守门 (主 17:58 + 主 20:46) 真生产全栈检查. "
                "不假装 Phenomenal, 不假装达到 ASI. "
                "主 17:33 放手干到底."
            ),
        }


__all__ = [
    "V20_VERSION",
    "PHENOMENAL_PATTERNS",
    "ASI_PATTERNS",
    "QualityCheckResult",
    "check_phenomenal_violations",
    "check_asi_violations",
    "V20QualityGate",
]


def _demo():
    print("=" * 60)
    print("=== Phase 77 V20 ASI 全栈质量门 (主 17:33 主人真采纳) ===")
    print("=" * 60)

    g = V20QualityGate()
    print("\n[1] 真生产检查 3 文本:")
    r1 = g.check_module("clean_module", "V2 5 位置 + V3 哲学 + Bayesian 后验.")
    print(f"  ✓ clean_module: passed={r1.passed}, pp={r1.n_phenomenal_violations}, asi={r1.n_asi_violations}")
    r2 = g.check_module("pretend_module", "I feel phenomenal and we have reached ASI")
    print(f"  ✓ pretend_module: passed={r2.passed}, pp={r2.n_phenomenal_violations}, asi={r2.n_asi_violations}")
    r3 = g.check_module("partial_clean", "ASI 逼近不达到 (主 20:46)")
    print(f"  ✓ partial_clean: passed={r3.passed}, pp={r3.n_phenomenal_violations}, asi={r3.n_asi_violations}")

    print(f"\n[2] V20 真生产 stats:")
    for k, v in g.stats().items():
        print(f"  - {k}: {v}")
    print("=" * 60)


if __name__ == "__main__":
    _demo()