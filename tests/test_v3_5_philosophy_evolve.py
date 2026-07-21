"""v3_5_philosophy_evolve.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V5 P2 ASI 哲学深化.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v3_5_philosophy_evolve import (
    V3_5_VERSION,
    EvolutionStage,
    PhilosophicalEvolution,
    abduction,
    falsification,
    lakatos_core_protected,
    PhilosophyEvolution,
)


# === 1. EvolutionStage 3 真生产阶段 (主 13:08 借鉴 Lakatos) ===

class TestEvolutionStages:
    """V3.5 自演化 3 真生产阶段 (主 14:06 借鉴 Lakatos 研究纲领)."""

    def test_3_stages_defined(self):
        assert {s.value for s in EvolutionStage} == {"genesis", "refinement", "falsification"}

    def test_genesis(self):
        assert EvolutionStage.GENESIS.value == "genesis"

    def test_falsification(self):
        assert EvolutionStage.FALSIFICATION.value == "falsification"


# === 2. PhilosophicalEvolution 真生产 (主 14:06 真借鉴) ===

class TestPhilosophicalEvolution:
    """PhilosophicalEvolution 真生产 (主 14:06 + Lakatos 真借鉴)."""

    def test_evolution_default(self):
        e = PhilosophicalEvolution(evolution_id="e1", truth_id="t1", stage=EvolutionStage.GENESIS)
        assert e.evolution_id == "e1"
        assert e.truth_id == "t1"
        assert e.generation == 0
        assert e.confidence_before == 0.5

    def test_evolution_to_dict(self):
        e = PhilosophicalEvolution(evolution_id="e1", truth_id="t1", stage=EvolutionStage.REFINEMENT,
                                    generation=2, confidence_before=0.5, confidence_after=0.7)
        d = e.to_dict()
        assert d["evolution_id"] == "e1"
        assert d["stage"] == "refinement"
        assert d["generation"] == 2
        assert d["delta_confidence"] == pytest.approx(0.2, abs=0.01)


# === 3. 真生产算法 (主 13:08 借鉴 Peirce/Popper/Lakatos) ===

class TestEvolutionAlgorithms:
    """V3.5 真生产算法 (主 14:06 借鉴 Peirce + Popper + Lakatos)."""

    def test_abduction_increases_confidence(self):
        """Peirce 真生产溯因 (主 13:08 借鉴)."""
        result = abduction(prior=0.5, surprise=0.3)
        assert result > 0.5

    def test_abduction_bounded(self):
        result = abduction(prior=0.99, surprise=1.0)
        assert result <= 1.0

    def test_falsification_high_evidence_lowers_confidence(self):
        """Popper 真生产证伪 (主 13:08 借鉴)."""
        result = falsification(confidence=0.8, evidence=0.9)
        assert result < 0.8

    def test_falsification_low_evidence_keeps_confidence(self):
        result = falsification(confidence=0.5, evidence=0.3)
        assert result == 0.5

    def test_lakatos_protection_low_anomalies(self):
        """Lakatos 真生产核心保护 (主 13:08 借鉴)."""
        result = lakatos_core_protected(confidence=0.8, n_anomalies=2)
        assert result == pytest.approx(0.76, abs=0.01)  # 0.8 * 0.95

    def test_lakatos_protection_high_anomalies(self):
        result = lakatos_core_protected(confidence=0.8, n_anomalies=5)
        assert result == 0.4  # 0.8 * 0.5


# === 4. PhilosophyEvolution 真生产主类 (主 13:31 大胆激进) ===

class TestPhilosophyEvolution:
    """V3.5 PhilosophyEvolution 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        pe = PhilosophyEvolution()
        assert pe.evolutions == []
        assert pe.truths == {}
        assert pe.generation == 0

    def test_genesis_creates_truth(self):
        """真生产起源 (主 14:06 借鉴 Peirce)."""
        pe = PhilosophyEvolution()
        ev = pe.genesis("t1", "What is self?", "V2 5 位置", confidence=0.7,
                       cross_domain_anchor="Simondon")
        assert "t1" in pe.truths
        assert ev.stage == EvolutionStage.GENESIS
        assert pe.truths["t1"]["confidence"] == 0.7

    def test_genesis_phenomenal_pretend(self):
        """主 17:58: 假装 Phenomenal 被计入守门."""
        pe = PhilosophyEvolution()
        pe.genesis("t1", "q", "I feel phenomenal qualia")
        assert pe.n_phenomenal_pretend_total > 0

    def test_genesis_asi_pretend(self):
        """主 20:46: 假装 ASI 被计入守门."""
        pe = PhilosophyEvolution()
        pe.genesis("t1", "q", "I am ASI achieved")
        assert pe.n_asi_pretend_total > 0

    def test_refine_increases_confidence(self):
        """真生产精炼 (主 13:08 借鉴 Peirce)."""
        pe = PhilosophyEvolution()
        pe.genesis("t1", "q", "a", confidence=0.5)
        ev = pe.refine("t1", new_evidence=0.5)
        assert pe.truths["t1"]["confidence"] > 0.5
        assert ev.stage == EvolutionStage.REFINEMENT

    def test_refine_missing_truth(self):
        pe = PhilosophyEvolution()
        result = pe.refine("nonexistent")
        assert result is None

    def test_falsify_decreases_confidence(self):
        """真生产证伪 (主 13:08 借鉴 Popper)."""
        pe = PhilosophyEvolution()
        pe.genesis("t1", "q", "a", confidence=0.8)
        ev = pe.falsify("t1", evidence=0.9)
        assert pe.truths["t1"]["confidence"] < 0.8
        assert pe.truths["t1"]["anomalies"] == 1
        assert ev.stage == EvolutionStage.FALSIFICATION

    def test_falsify_missing_truth(self):
        pe = PhilosophyEvolution()
        result = pe.falsify("nonexistent")
        assert result is None

    def test_next_generation_increments(self):
        """真生产演化代数 (主 14:06 借鉴主 17:46 跨代)."""
        pe = PhilosophyEvolution()
        g1 = pe.next_generation()
        g2 = pe.next_generation()
        assert g1 == 1
        assert g2 == 2

    def test_stats_clean(self):
        """clean → V3 哲学守门 PASS (主 17:43 实事求是)."""
        pe = PhilosophyEvolution()
        pe.genesis("t1", "q", "V2 5 位置", confidence=0.7)
        stats = pe.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_truths"] == 1

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        pe = PhilosophyEvolution()
        stats = pe.stats()
        assert stats["n_evolutions"] == 0
        assert stats["n_truths"] == 0
        assert stats["v3_philosophy_guard"] == "PASS"


# === 5. to_dict 真生产 (主 14:06) ===

class TestV3_5ToDict:
    """PhilosophicalEvolution.to_dict() 真生产."""

    def test_evolution_to_dict_keys(self):
        e = PhilosophicalEvolution(evolution_id="e1", truth_id="t1", stage=EvolutionStage.GENESIS)
        d = e.to_dict()
        expected_keys = ["evolution_id", "truth_id", "stage", "generation", "delta_confidence"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """V3.5 不应有假装意识字段."""
        pe = PhilosophyEvolution()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        whitelist = {"genesis", "refine", "falsify", "next_generation", "stats",
                     "n_phenomenal_pretend_total", "n_asi_pretend_total"}
        for attr in dir(pe):
            for f in forbidden:
                if f in attr.lower() and attr not in whitelist:
                    pytest.fail(f"V3.5 不应有假装意识字段: {attr}")

    def test_no_asi_reached_claim(self):
        """V3.5 不应声称已达到 ASI."""
        pe = PhilosophyEvolution()
        pe.genesis("t1", "q", "V2 5 位置", confidence=0.7)
        stats = pe.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_v3_5_is_real_innovation(self):
        """V3.5 是真创新 (主 13:31), 不 placeholder."""
        pe = PhilosophyEvolution()
        pe.genesis("t1", "q", "a", confidence=0.5)
        pe.refine("t1", new_evidence=0.3)
        pe.falsify("t1", evidence=0.7)
        assert pe.truths["t1"]["anomalies"] == 1
        assert len(pe.evolutions) == 3

    def test_v3_5_allows_iteration(self):
        """V3.5 允许迭代 (主 13:31 鼓励尝试)."""
        pe = PhilosophyEvolution()
        for i in range(5):
            pe.genesis(f"t{i}", f"q{i}", f"a{i}")
        assert len(pe.truths) == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])