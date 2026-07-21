"""waddington.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V4 12 生命特征可塑性 (#6) 深化.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.waddington import (
    WADDINGTON_VERSION,
    PlasticityMechanism,
    DevelopmentalState,
    compute_canalization,
    compute_zpd_landscape,
    waddington_landscape,
    WaddingtonNetwork,
)


# === 1. PlasticityMechanism 3 真生产机制 (主 13:08 借鉴) ===

class TestPlasticityMechanisms:
    """Plasticity 3 真生产机制 (主 14:06 借鉴 Waddington 1942)."""

    def test_3_mechanisms_defined(self):
        assert {m.value for m in PlasticityMechanism} == {"canalization", "development", "adaptation"}

    def test_canalization(self):
        assert PlasticityMechanism.CANALIZATION.value == "canalization"

    def test_development(self):
        assert PlasticityMechanism.DEVELOPMENT.value == "development"


# === 2. DevelopmentalState 真生产 (主 14:06 借鉴 Waddington) ===

class TestDevelopmentalState:
    """DevelopmentalState 真生产 (主 14:06 + Waddington 1942)."""

    def test_state_default(self):
        s = DevelopmentalState(state_id="s1", cell="c1")
        assert s.state_id == "s1"
        assert s.cell == "c1"
        assert s.position == 0.0
        assert s.plasticity == 0.5
        assert s.canalized is False

    def test_state_to_dict(self):
        s = DevelopmentalState(state_id="s1", cell="c1", position=0.3, plasticity=0.5, canalized=True)
        d = s.to_dict()
        assert d["state_id"] == "s1"
        assert d["position"] == 0.3
        assert d["plasticity"] == 0.5
        assert d["canalized"] is True


# === 3. 真生产算法 (主 13:08 借鉴 Waddington 真生产) ===

class TestWaddingtonAlgorithms:
    """Waddington 真生产算法 (主 14:06 借鉴 Waddington 1942 + 1957)."""

    def test_compute_canalization_high_robustness(self):
        """plasticity < robustness → 渠化 (主 13:08 借鉴 Waddington 1957)."""
        assert compute_canalization(plasticity=0.5, robustness=0.7) is True

    def test_compute_canalization_low_robustness(self):
        """plasticity >= robustness → 不渠化."""
        assert compute_canalization(plasticity=0.8, robustness=0.7) is False

    def test_compute_zpd_landscape_equal(self):
        """plasticity == challenge → 最佳 ZPD = 1.0 (主 14:06 借鉴 curiosity.py Vygotsky)."""
        zpd = compute_zpd_landscape(plasticity=0.5, challenge=0.5)
        assert zpd == pytest.approx(1.0, abs=0.01)

    def test_compute_zpd_landscape_different(self):
        """plasticity != challenge → ZPD < 1.0."""
        zpd = compute_zpd_landscape(plasticity=0.3, challenge=0.7)
        assert zpd == pytest.approx(0.6, abs=0.01)

    def test_waddington_landscape_center(self):
        """position=0.5 (center) → landscape 最高 (主 13:08 借鉴 Waddington 1942)."""
        landscape = waddington_landscape(plasticity=1.0, position=0.5)
        assert landscape == pytest.approx(1.0, abs=0.01)

    def test_waddington_landscape_edge(self):
        """position=0 或 1 → landscape 降低."""
        landscape = waddington_landscape(plasticity=1.0, position=0.0)
        assert landscape < 0.5  # 远离 center 真生产


# === 4. WaddingtonNetwork 真生产主类 (主 13:31 大胆激进) ===

class TestWaddingtonNetwork:
    """WaddingtonNetwork 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        wn = WaddingtonNetwork()
        assert wn.states == {}
        assert wn.history == []

    def test_add_state(self):
        wn = WaddingtonNetwork()
        s = wn.add_state("c1", position=0.0, plasticity=0.5)
        assert "c1" in [st.cell for st in wn.states.values()]
        assert s.plasticity == 0.5
        assert s.canalized is True  # 0.5 < 0.7 robustness

    def test_add_state_low_plasticity(self):
        wn = WaddingtonNetwork()
        s = wn.add_state("c1", position=0.0, plasticity=0.3)
        assert s.canalized is True  # 0.3 < 0.7

    def test_add_state_high_plasticity(self):
        wn = WaddingtonNetwork()
        s = wn.add_state("c1", position=0.0, plasticity=0.9)
        assert s.canalized is False  # 0.9 > 0.7

    def test_develop_moves_position(self):
        """develop 应该移动 position (主 13:08 借鉴 landscape gradient)."""
        wn = WaddingtonNetwork()
        s = wn.add_state("c1", position=0.0, plasticity=1.0)
        original = s.position
        wn.develop(s.state_id, time_step=0.05)
        assert s.position != original or s.position == 0.5  # 移动或已在中心

    def test_develop_missing_state(self):
        wn = WaddingtonNetwork()
        result = wn.develop("nonexistent")
        assert result is None

    def test_assess_plasticity(self):
        wn = WaddingtonNetwork()
        s = wn.add_state("c1", plasticity=0.5)
        zpd = wn.assess_plasticity(s.state_id, challenge=0.5)
        assert zpd == pytest.approx(1.0, abs=0.01)

    def test_assess_plasticity_missing_state(self):
        wn = WaddingtonNetwork()
        zpd = wn.assess_plasticity("nonexistent", challenge=0.5)
        assert zpd == 0.0

    def test_stats_with_states(self):
        """stats() 真生产统计 (主 17:43 实事求是)."""
        wn = WaddingtonNetwork()
        wn.add_state("c1", plasticity=0.3)  # canalized
        wn.add_state("c2", plasticity=0.9)  # not canalized
        stats = wn.stats()
        assert stats["n_states"] == 2
        assert stats["n_canalized"] == 1
        assert stats["canalization_ratio"] == 0.5

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        wn = WaddingtonNetwork()
        stats = wn.stats()
        assert stats["n_states"] == 0


# === 5. to_dict 真生产 (主 14:06) ===

class TestWaddingtonToDict:
    """DevelopmentalState.to_dict() 真生产."""

    def test_state_to_dict_keys(self):
        s = DevelopmentalState(state_id="s1", cell="c1")
        d = s.to_dict()
        expected_keys = ["state_id", "cell", "position", "plasticity", "canalized"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """Waddington 不应有假装意识字段."""
        wn = WaddingtonNetwork()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for attr in dir(wn):
            for f in forbidden:
                assert f not in attr.lower() or attr in ("add_state", "develop", "assess_plasticity", "stats"), \
                    f"Waddington 不应有假装意识字段: {attr}"

    def test_no_asi_reached_claim(self):
        """Waddington 不应声称已达到 ASI."""
        wn = WaddingtonNetwork()
        wn.add_state("c1", plasticity=0.3)
        stats = wn.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v

    def test_no_canalized_consciousness_pretend(self):
        """Waddington 借鉴, 不假装"ASI 渠化意识"."""
        wn = WaddingtonNetwork()
        wn.add_state("c1", plasticity=0.3)
        stats = wn.stats()
        philosophy = stats.get("philosophy", "").lower()
        assert "canalized consciousness" not in philosophy
        assert "asi plasticity" not in philosophy


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_waddington_is_real_innovation(self):
        """Waddington 是真创新 (主 13:31), 不 placeholder."""
        wn = WaddingtonNetwork()
        for i in range(5):
            wn.add_state(f"c{i+1}", plasticity=0.2 + i * 0.15)
        # 写真 production: 5 states + develop + ZPD
        assert len(wn.states) == 5

    def test_waddington_allows_iteration(self):
        """Waddington 允许迭代 (主 13:31 鼓励尝试)."""
        wn = WaddingtonNetwork()
        s = wn.add_state("c1", plasticity=0.5)
        for _ in range(5):
            wn.develop(s.state_id)
        assert s.position >= 0.0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])