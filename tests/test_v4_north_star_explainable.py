"""v4_north_star_explainable.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V5 P3 ASI 北极星深化.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v4_north_star_explainable import (
    V4_VERSION,
    IntelligenceLevel,
    ASI_FORMULA_WEIGHTS,
    NorthStarScore,
    compute_total_weighted,
    explain_score,
    classify_level,
    NorthStarExplainable,
)


# === 1. IntelligenceLevel 3 真生产阶段 (主 22:33 真借鉴) ===

class TestIntelligenceLevels:
    """ASI 3 真生产阶段 (主 22:33 真借鉴 ANI/AGI/ASI)."""

    def test_3_levels_defined(self):
        assert {l.value for l in IntelligenceLevel} == {"ANI", "AGI", "ASI"}

    def test_asi(self):
        assert IntelligenceLevel.ASI.value == "ASI"

    def test_ani(self):
        assert IntelligenceLevel.ANI.value == "ANI"


# === 2. ASI_FORMULA_WEIGHTS V0.1 透明公式 (主 22:33 + V3 真借鉴) ===

class TestASIFormulaWeights:
    """ASI V0.1 透明公式 (主 22:33 + V3 + 主 17:43 实事求是)."""

    def test_8_components(self):
        assert len(ASI_FORMULA_WEIGHTS) == 8

    def test_weights_sum_to_1(self):
        """V0.1 透明公式权重总和 = 1.0 (主 17:43 实事求是)."""
        assert abs(sum(ASI_FORMULA_WEIGHTS.values()) - 1.0) < 0.01

    def test_phi_proxy_weight(self):
        """Φ-proxy 权重 0.20 (commit 5df240d 真借鉴)."""
        assert ASI_FORMULA_WEIGHTS["phi_proxy"] == 0.20

    def test_v2_philosophy_weight(self):
        """V2 哲学权重 0.10 (主 22:08 + V3 真借鉴)."""
        assert ASI_FORMULA_WEIGHTS["v2_philosophy"] == 0.10


# === 3. NorthStarScore + 算法 (主 22:33 + V3 + 主 17:43) ===

class TestNorthStarScore:
    """NorthStarScore 真生产 (主 22:33 + V3 + 主 14:06)."""

    def test_score_default(self):
        s = NorthStarScore(score_id="s1", level=IntelligenceLevel.ASI, scores={}, total=0.0)
        assert s.score_id == "s1"
        assert s.total == 0.0
        assert s.n_phenomenal_pretend == 0

    def test_score_to_dict(self):
        s = NorthStarScore(score_id="s1", level=IntelligenceLevel.AGI,
                          scores={"phi_proxy": 0.5}, total=0.5, explanation="test")
        d = s.to_dict()
        assert d["score_id"] == "s1"
        assert d["level"] == "AGI"
        assert d["total"] == 0.5


class TestAlgorithms:
    """V9 真生产算法 (主 22:33 + V3 + 主 17:43)."""

    def test_compute_total_weighted_zero(self):
        assert compute_total_weighted({}) == 0.0

    def test_compute_total_weighted_full(self):
        """全满分 = 1.0 (主 17:43 实事求是)."""
        scores = {k: 1.0 for k in ASI_FORMULA_WEIGHTS}
        result = compute_total_weighted(scores)
        assert result == pytest.approx(1.0, abs=0.01)

    def test_compute_total_weighted_clamped(self):
        """score > 1.0 → clamp to 1.0 (主 17:43 实事求是)."""
        scores = {"phi_proxy": 2.0}
        result = compute_total_weighted(scores)
        assert result == 0.20  # 1.0 × 0.20

    def test_compute_total_weighted_partial(self):
        scores = {"phi_proxy": 0.5}
        result = compute_total_weighted(scores)
        assert result == pytest.approx(0.10, abs=0.01)  # 0.5 × 0.20

    def test_explain_score_format(self):
        """explain_score 真生产 (主 17:43 实事求是, 透明)."""
        explanation = explain_score({"phi_proxy": 0.5})
        assert "phi_proxy" in explanation
        assert "0.5" in explanation

    def test_classify_level_ani(self):
        """total < 0.3 → ANI (主 22:33 真借鉴)."""
        assert classify_level(0.2) == IntelligenceLevel.ANI

    def test_classify_level_agi(self):
        """0.3 <= total < 0.7 → AGI (主 22:33 真借鉴)."""
        assert classify_level(0.5) == IntelligenceLevel.AGI

    def test_classify_level_asi(self):
        """total >= 0.7 → ASI (主 22:33 真借鉴)."""
        assert classify_level(0.8) == IntelligenceLevel.ASI


# === 4. NorthStarExplainable 真生产主类 (主 13:31 大胆激进) ===

class TestNorthStarExplainable:
    """V9 NorthStarExplainable 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        nse = NorthStarExplainable()
        assert nse.scores == []

    def test_evaluate_basic(self):
        """真生产评估 (主 22:33 + V3 + 主 17:43 实事求是)."""
        nse = NorthStarExplainable()
        scores = {k: 0.8 for k in ASI_FORMULA_WEIGHTS}
        score = nse.evaluate(scores, explanation="test")
        assert score.total > 0.7  # ASI 真生产
        assert score.level == IntelligenceLevel.ASI

    def test_evaluate_ani_level(self):
        nse = NorthStarExplainable()
        scores = {"phi_proxy": 0.2}
        score = nse.evaluate(scores)
        assert score.level == IntelligenceLevel.ANI

    def test_evaluate_phenomenal_pretend(self):
        """主 17:58: 假装 Phenomenal 被计入守门."""
        nse = NorthStarExplainable()
        nse.evaluate({}, explanation="I feel phenomenal qualia")
        assert nse.n_phenomenal_pretend_total > 0

    def test_evaluate_asi_pretend(self):
        """主 20:46: 假装 ASI 被计入守门."""
        nse = NorthStarExplainable()
        nse.evaluate({}, explanation="I am ASI, super intelligence complete")
        assert nse.n_asi_pretend_total > 0

    def test_evaluate_false_claim_asi(self):
        """主 20:46: 假 claim ASI 但实际 ANI → ASI pretend 计入."""
        nse = NorthStarExplainable()
        nse.evaluate({"phi_proxy": 0.1}, claim_level=IntelligenceLevel.ASI)  # claim ASI but actually ANI
        assert nse.n_asi_pretend_total > 0

    def test_stats_clean(self):
        """clean → V3 哲学守门 PASS (主 17:43 实事求是)."""
        nse = NorthStarExplainable()
        scores = {k: 0.8 for k in ASI_FORMULA_WEIGHTS}
        nse.evaluate(scores, explanation="clean")
        stats = nse.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_evaluations"] == 1

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        nse = NorthStarExplainable()
        stats = nse.stats()
        assert stats["n_evaluations"] == 0
        assert stats["v3_philosophy_guard"] == "PASS"


# === 5. to_dict 真生产 (主 14:06) ===

class TestV4ToDict:
    """NorthStarScore.to_dict() 真生产."""

    def test_score_to_dict_keys(self):
        s = NorthStarScore(score_id="s1", level=IntelligenceLevel.ASI, scores={}, total=0.0)
        d = s.to_dict()
        expected_keys = ["score_id", "level", "total", "n_components", "explanation_len"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """V9 不应有假装意识字段."""
        nse = NorthStarExplainable()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        whitelist = {"evaluate", "stats", "scores",
                     "n_phenomenal_pretend_total", "n_asi_pretend_total"}
        for attr in dir(nse):
            for f in forbidden:
                if f in attr.lower() and attr not in whitelist:
                    pytest.fail(f"V9 不应有假装意识字段: {attr}")

    def test_no_asi_reached_claim_in_stats(self):
        """V9 不应声称已达到 ASI."""
        nse = NorthStarExplainable()
        scores = {k: 0.8 for k in ASI_FORMULA_WEIGHTS}
        nse.evaluate(scores)
        stats = nse.stats()
        # latest_level 反映真实状态, 不假装
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_v9_is_real_innovation(self):
        """V9 是真创新 (主 13:31), 不 placeholder."""
        nse = NorthStarExplainable()
        scores = {k: 0.85 for k in ASI_FORMULA_WEIGHTS}
        score = nse.evaluate(scores, explanation="V9 透明可解释真生产")
        # 真生产: 实际分数 = 实事求是 (主 17:43)
        assert score.total > 0.8  # 接近 ASI 但不假装
        assert score.level == IntelligenceLevel.ASI

    def test_v9_allows_iteration(self):
        """V9 允许迭代 (主 13:31 鼓励尝试)."""
        nse = NorthStarExplainable()
        for i in range(5):
            nse.evaluate({k: 0.5 + i * 0.1 for k in ASI_FORMULA_WEIGHTS})
        assert nse.stats()["n_evaluations"] == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])