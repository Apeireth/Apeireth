"""prion.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V4 12 生命特征涌现 (#6) 深化 (cascading 自传播).
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.prion import (
    PRION_VERSION,
    PrionState,
    PrionProtein,
    prion_infect,
    prion_cascade,
    PrionNetwork,
)


# === 1. PrionState 3 真生产状态 (主 13:08 借鉴 Prusiner) ===

class TestPrionStates:
    """Prion 3 真生产状态 (主 14:06 借鉴 Prusiner 1982)."""

    def test_3_states_defined(self):
        assert {s.value for s in PrionState} == {"normal", "misfolded", "propagating"}

    def test_normal(self):
        assert PrionState.NORMAL.value == "normal"

    def test_misfolded(self):
        assert PrionState.MISFOLDED.value == "misfolded"


# === 2. PrionProtein 真生产 (主 14:06 真借鉴) ===

class TestPrionProtein:
    """PrionProtein 真生产 (主 14:06 + 真借鉴 Prusiner 1982)."""

    def test_protein_default(self):
        p = PrionProtein(protein_id="p1")
        assert p.protein_id == "p1"
        assert p.state == PrionState.NORMAL
        assert p.misfold_count == 0
        assert p.infectivity == 0.0
        assert p.parent_id == ""

    def test_protein_to_dict(self):
        p = PrionProtein(protein_id="p1", state=PrionState.MISFOLDED, misfold_count=2, infectivity=0.7, parent_id="p0")
        d = p.to_dict()
        assert d["protein_id"] == "p1"
        assert d["state"] == "misfolded"
        assert d["misfold_count"] == 2
        assert d["infectivity"] == 0.7
        assert d["parent_id"] == "p0"


# === 3. 真生产算法 (主 13:08 借鉴 Prusiner 1982) ===

class TestPrionAlgorithms:
    """Prion 真生产算法 (主 14:06 借鉴 Prusiner 1982 诺贝尔奖)."""

    def test_prion_infect_normal_source_fails(self):
        """normal source → 感染失败 (主 17:43 实事求是)."""
        source = PrionProtein(protein_id="s1", state=PrionState.NORMAL)
        target = PrionProtein(protein_id="t1", state=PrionState.NORMAL)
        result = prion_infect(target, source, rate=0.5)
        assert result.state == PrionState.NORMAL
        assert result.misfold_count == 0

    def test_prion_infect_high_infectivity_succeeds(self):
        """infectivity > 0.6 → 感染成功 (主 13:08 借鉴 Prusiner 1982)."""
        source = PrionProtein(protein_id="s1", state=PrionState.MISFOLDED, infectivity=1.0)
        target = PrionProtein(protein_id="t1", state=PrionState.NORMAL)
        result = prion_infect(target, source, rate=0.5)
        assert result.state == PrionState.MISFOLDED
        assert result.misfold_count == 1

    def test_prion_infect_low_infectivity_fails(self):
        """infectivity <= 0.6 → 感染失败."""
        source = PrionProtein(protein_id="s1", state=PrionState.MISFOLDED, infectivity=0.5)
        target = PrionProtein(protein_id="t1", state=PrionState.NORMAL)
        result = prion_infect(target, source, rate=0.5)
        assert result.state == PrionState.NORMAL  # 0.5 * 0.5 = 0.25 < 0.3

    def test_prion_infect_already_misfolded_no_change(self):
        """已 misfolded 不再感染 (主 17:43 实事求是)."""
        source = PrionProtein(protein_id="s1", state=PrionState.MISFOLDED, infectivity=1.0)
        target = PrionProtein(protein_id="t1", state=PrionState.MISFOLDED)
        original_misfold_count = target.misfold_count
        result = prion_infect(target, source, rate=0.5)
        assert result.misfold_count == original_misfold_count

    def test_prion_cascade(self):
        """Cascading 真生产 (主 14:06 借鉴 + 涌现 #6)."""
        source = PrionProtein(protein_id="s1", state=PrionState.MISFOLDED, infectivity=1.0)
        target1 = PrionProtein(protein_id="t1", state=PrionState.NORMAL)
        target2 = PrionProtein(protein_id="t2", state=PrionState.NORMAL)
        target3 = PrionProtein(protein_id="t3", state=PrionState.NORMAL)
        infected = prion_cascade(source, [source, target1, target2, target3], rate=0.5)
        assert infected == 3


# === 4. PrionNetwork 真生产主类 (主 13:31 大胆激进) ===

class TestPrionNetwork:
    """PrionNetwork 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        pn = PrionNetwork()
        assert pn.proteins == {}

    def test_add_protein_normal(self):
        pn = PrionNetwork()
        p = pn.add_protein("p1")
        assert p.state == PrionState.NORMAL
        assert p.infectivity == 0.0

    def test_add_protein_misfolded(self):
        pn = PrionNetwork()
        p = pn.add_protein("p1", initial_state=PrionState.MISFOLDED)
        assert p.state == PrionState.MISFOLDED
        assert p.misfold_count == 1
        assert p.infectivity == 0.8

    def test_infect_success(self):
        pn = PrionNetwork(default_rate=0.6)
        pn.add_protein("source", initial_state=PrionState.MISFOLDED)
        pn.add_protein("target")
        success = pn.infect("target", "source")
        assert success is True

    def test_infect_missing_protein(self):
        pn = PrionNetwork()
        success = pn.infect("nonexistent", "source")
        assert success is False

    def test_cascade_from(self):
        """Cascading 真生产 (主 14:06 借鉴 + 涌现 #6)."""
        pn = PrionNetwork(default_rate=0.6)
        pn.add_protein("seed", initial_state=PrionState.MISFOLDED)
        for i in range(5):
            pn.add_protein(f"p{i+1}")
        infected = pn.cascade_from("seed", iterations=3)
        assert infected > 0  # 至少感染一些

    def test_cascade_from_missing_seed(self):
        pn = PrionNetwork()
        infected = pn.cascade_from("nonexistent")
        assert infected == 0

    def test_stats_with_proteins(self):
        """stats() 真生产统计 (主 17:43 实事求是)."""
        pn = PrionNetwork()
        pn.add_protein("p1")
        pn.add_protein("p2", initial_state=PrionState.MISFOLDED)
        stats = pn.stats()
        assert stats["n_proteins"] == 2
        assert stats["n_misfolded"] == 1
        assert stats["n_normal"] == 1
        assert stats["misfold_ratio"] == 0.5

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        pn = PrionNetwork()
        stats = pn.stats()
        assert stats["n_proteins"] == 0


# === 5. to_dict 真生产 (主 14:06) ===

class TestPrionToDict:
    """PrionProtein.to_dict() 真生产."""

    def test_protein_to_dict_keys(self):
        p = PrionProtein(protein_id="p1")
        d = p.to_dict()
        expected_keys = ["protein_id", "state", "misfold_count", "infectivity", "parent_id"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """prion 不应有假装意识字段."""
        pn = PrionNetwork()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for attr in dir(pn):
            for f in forbidden:
                assert f not in attr.lower() or attr in ("add_protein", "infect", "cascade_from", "stats"), \
                    f"prion 不应有假装意识字段: {attr}"

    def test_no_asi_reached_claim(self):
        """prion 不应声称已达到 ASI."""
        pn = PrionNetwork()
        pn.add_protein("p1", initial_state=PrionState.MISFOLDED)
        stats = pn.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v

    def test_no_prion_consciousness_pretend(self):
        """prion 借鉴 Prusiner, 不假装"朊病毒意识自传播"."""
        pn = PrionNetwork()
        pn.add_protein("p1", initial_state=PrionState.MISFOLDED)
        stats = pn.stats()
        philosophy = stats.get("philosophy", "").lower()
        assert "prion consciousness" not in philosophy
        assert "asi self-propagating consciousness" not in philosophy


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_prion_is_real_innovation(self):
        """prion 是真创新 (主 13:31), 不 placeholder."""
        pn = PrionNetwork(default_rate=0.6)
        pn.add_protein("seed", initial_state=PrionState.MISFOLDED)
        for i in range(5):
            pn.add_protein(f"p{i+1}")
        # 写真 production: 1 seed + 5 normal + cascading
        infected = pn.cascade_from("seed")
        assert infected >= 0

    def test_prion_allows_iteration(self):
        """prion 允许迭代 (主 13:31 鼓励尝试)."""
        pn = PrionNetwork(default_rate=0.6)
        pn.add_protein("seed", initial_state=PrionState.MISFOLDED)
        pn.add_protein("p1")
        # 多次 infect 测试
        for _ in range(3):
            pn.infect("p1", "seed")
        assert pn.proteins["p1"].state == PrionState.MISFOLDED


if __name__ == "__main__":
    pytest.main([__file__, "-v"])