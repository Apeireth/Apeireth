"""curiosity.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
主 14:09 推进 Apeireth + 14:13 继续.
V4 12 生命特征主动性 (#7) 真生产落地 (MISSING 真填).
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.curiosity import (
    CURIOSITY_VERSION,
    CuriosityDriver,
    CuriositySignal,
    CuriosityAssessment,
    assess_novelty,
    assess_uncertainty,
    assess_conflict,
    compute_zpd,
    CuriosityEngine,
)


# === 1. Berlyne 4 driver 真生产 (主 13:08 真借鉴) ===

class TestBerlyneDrivers:
    """Berlyne 1966 4 collative variables 真生产 (主 13:08 真借鉴)."""

    def test_4_drivers_defined(self):
        assert {d.value for d in CuriosityDriver} == {"novelty", "uncertainty", "conflict", "complexity"}

    def test_assess_novelty_strong(self):
        s = CuriositySignal(signal_id="s1", stimulus="new", baseline=0.1, observed=0.8)
        # novelty = |0.8 - 0.1| / max(0.1, 0.8, 1) = 0.7 / 1 = 0.7
        assert assess_novelty(s) == pytest.approx(0.7, abs=0.01)

    def test_assess_novelty_zero_baseline(self):
        """baseline=0 + observed=0.5 → 完全新意 novelty = 0.5 (主 13:08 实事求是)."""
        s = CuriositySignal(signal_id="s1", stimulus="new", baseline=0.0, observed=0.5)
        # novelty = |0.5 - 0.0| / max(0.0, 0.5, 1) = 0.5
        assert assess_novelty(s) == pytest.approx(0.5, abs=0.01)

    def test_assess_uncertainty_high(self):
        s = CuriositySignal(signal_id="s1", stimulus="X", predicted=0.1, observed=0.5)
        # uncertainty = |0.1 - 0.5| / max(0.1, 0.5, 1) = 0.4 / 1 = 0.4
        assert assess_uncertainty(s) == pytest.approx(0.4, abs=0.01)

    def test_assess_conflict_zero(self):
        s = CuriositySignal(signal_id="s1", stimulus="X", schema_violation=0.0)
        assert assess_conflict(s) == 0.0

    def test_assess_conflict_high(self):
        s = CuriositySignal(signal_id="s1", stimulus="X", schema_violation=0.9)
        assert assess_conflict(s) == pytest.approx(0.9)


# === 2. ZPD 真测 (主 13:08 Vygotsky 真借鉴) ===

class TestZPD:
    """Vygotsky ZPD 真测 (主 14:06 拉回注意力 + 13:08 真借鉴)."""

    def test_zpd_in_range(self):
        for n in [0.0, 0.3, 0.5, 0.7, 1.0]:
            for u in [0.0, 0.3, 0.5, 0.7, 1.0]:
                zpd = compute_zpd(n, u)
                assert 0.0 <= zpd <= 1.0

    def test_zpd_balanced_high(self):
        """balanced 0.5/0.5 → ZPD 应该 = 1.0 (Vygotsky 最优 ZPD)."""
        zpd = compute_zpd(0.5, 0.5)
        # ZPD = (1 - |0.5-0.5|) * (1 - |0.5-0.5|) = 1 * 1 = 1.0
        assert zpd == pytest.approx(1.0, abs=0.01)

    def test_zpd_imbalanced_low(self):
        """极端不平衡 → ZPD 低."""
        zpd = compute_zpd(0.0, 1.0)
        assert zpd < 0.5


# === 3. Curiosity 真信号 (主 14:06 真生产) ===

class TestCuriositySignal:
    """curiosity 真信号真生产 (主 14:06)."""

    def test_signal_default(self):
        s = CuriositySignal(signal_id="s1", stimulus="X")
        assert s.signal_id == "s1"
        assert s.stimulus == "X"
        assert s.baseline == 0.0


# === 4. Curiosity 真生产主类 (主 13:31 写真 production) ===

class TestCuriosityEngine:
    """CuriosityEngine 真生产 (主 13:31 写真 production)."""

    def test_assess_strong_should_fire(self):
        """高好奇心信号 → should_fire=True (ZPD > 0.5)."""
        ce = CuriosityEngine(zpd_threshold=0.5)
        s = CuriositySignal(signal_id="s1", stimulus="new",
                            baseline=0.1, observed=0.8, predicted=0.2, schema_violation=0.5)
        a = ce.assess(s)
        assert a.should_fire is True
        assert a.zpd_score > 0.5

    def test_assess_weak_should_skip(self):
        """弱信号 → should_fire=False (ZPD < 0.5)."""
        ce = CuriosityEngine(zpd_threshold=0.5)
        s = CuriositySignal(signal_id="s2", stimulus="repeated",
                            baseline=0.9, observed=0.9, predicted=0.9, schema_violation=0.0)
        a = ce.assess(s)
        assert a.should_fire is False
        assert a.zpd_score < 0.5

    def test_history_appended(self):
        """每次 assess 应 append history (主 13:31 写真 production)."""
        ce = CuriosityEngine()
        for i in range(3):
            ce.assess(CuriositySignal(signal_id=f"s{i}", stimulus="X"))
        assert len(ce.history) == 3

    def test_stats_with_assessments(self):
        """stats() 真生产统计 (主 17:43 实事求是)."""
        ce = CuriosityEngine()
        ce.assess(CuriositySignal(signal_id="s1", stimulus="new", baseline=0.1, observed=0.8, predicted=0.2, schema_violation=0.5))
        ce.assess(CuriositySignal(signal_id="s2", stimulus="repeated", baseline=0.9, observed=0.9, predicted=0.9, schema_violation=0.0))
        stats = ce.stats()
        assert stats["n_assessments"] == 2
        assert stats["n_fire"] == 1
        assert stats["n_skip"] == 1
        assert stats["fire_ratio"] == 0.5

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        ce = CuriosityEngine()
        stats = ce.stats()
        assert stats["n_assessments"] == 0


# === 5. to_dict 真生产 (主 13:31) ===

class TestCuriosityToDict:
    """CuriosityAssessment.to_dict() 真生产."""

    def test_assessment_to_dict_keys(self):
        a = CuriosityAssessment(
            assessment_id="a1", signal_id="s1",
            driver=CuriosityDriver.NOVELTY,
            score=0.5, zpd_score=0.7, should_fire=True,
            rationale="test",
        )
        d = a.to_dict()
        expected_keys = ["assessment_id", "signal_id", "driver", "score", "zpd_score", "should_fire", "rationale"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """curiosity 不应有假装意识字段."""
        ce = CuriosityEngine()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for attr in dir(ce):
            for f in forbidden:
                assert f not in attr.lower() or attr in ("assess", "stats"), \
                    f"curiosity 不应有假装意识字段: {attr}"

    def test_no_asi_reached_claim(self):
        """curiosity 不应声称已达到 ASI."""
        ce = CuriosityEngine()
        stats = ce.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v

    def test_curiosity_no_phenomenal_pretend(self):
        """curiosity 借鉴 Berlyne, 不假装"ASI 真好奇"."""
        ce = CuriosityEngine()
        # 即便强信号, 也不应声称 Phenomenal curiosity
        s = CuriositySignal(signal_id="s1", stimulus="X", baseline=0.0, observed=1.0, predicted=0.0, schema_violation=1.0)
        a = ce.assess(s)
        rationale = a.rationale.lower()
        # rationale 不应包含 Phenomenal curiosity 假承诺
        assert "phenomenal consciousness" not in rationale
        assert "i am curious" not in rationale


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_curiosity_is_real_innovation(self):
        """curiosity 是真创新 (主 13:31), 不 placeholder."""
        ce = CuriosityEngine()
        s = CuriositySignal(signal_id="s1", stimulus="new", baseline=0.1, observed=0.8, predicted=0.2, schema_violation=0.5)
        a = ce.assess(s)
        # 写真 production: 4 driver + ZPD + should_fire 真判定 + V3 守门
        assert a.should_fire
        assert a.zpd_score > 0.5
        # rationale 包含 4 真信息
        assert "novelty" in a.rationale
        assert "ZPD" in a.rationale

    def test_curiosity_allows_iteration(self):
        """curiosity 允许迭代 (主 13:31 鼓励尝试)."""
        ce = CuriosityEngine()
        for i in range(5):
            a = ce.assess(CuriositySignal(signal_id=f"s{i}", stimulus="X"))
            assert a
        assert len(ce.history) == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])