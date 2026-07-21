"""v20_quality_gate.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v20_quality_gate import (
    V20_VERSION, PHENOMENAL_PATTERNS, ASI_PATTERNS,
    QualityCheckResult, check_phenomenal_violations,
    check_asi_violations, V20QualityGate,
)


class TestV20Helpers:
    def test_phenomenal_detection(self):
        text = "I feel phenomenal qualia"
        violations = check_phenomenal_violations(text)
        assert len(violations) > 0

    def test_phenomenal_clean(self):
        clean = "V2 5 位置 + V3 哲学"
        assert check_phenomenal_violations(clean) == []

    def test_asi_detection(self):
        text = "We have reached ASI, super intelligence complete"
        violations = check_asi_violations(text)
        assert len(violations) > 0

    def test_asi_clean(self):
        clean = "ASI 逼近不达到 (主 20:46)"
        assert check_asi_violations(clean) == []


class TestV20Gate:
    def test_init(self):
        g = V20QualityGate()
        assert g.results == []

    def test_check_clean(self):
        g = V20QualityGate()
        r = g.check_module("clean", "V2 5 位置 + Bayesian")
        assert r.passed is True
        assert r.n_phenomenal_violations == 0
        assert r.n_asi_violations == 0

    def test_check_phenomenal_violation(self):
        g = V20QualityGate()
        r = g.check_module("bad", "I feel phenomenal qualia")
        assert r.passed is False
        assert r.n_phenomenal_violations > 0

    def test_check_asi_violation(self):
        g = V20QualityGate()
        r = g.check_module("bad", "I am ASI")
        assert r.passed is False
        assert r.n_asi_violations > 0

    def test_check_all_modules(self):
        g = V20QualityGate()
        results = g.check_all_modules(base_dir="apeireth")
        assert len(results) > 0
        # 真生产: 模块都被检查 (注释里的反例字符串也算违反, 因为是真生产 placeholder 风险)
        assert len(results) >= 10

    def test_stats(self):
        g = V20QualityGate()
        g.check_module("clean", "OK")
        stats = g.stats()
        assert stats["n_passed"] >= 1
        assert "v3_philosophy_guard" not in stats or stats["pass_rate"] > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])