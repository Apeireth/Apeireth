"""epigenetic.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V4 12 生命特征遗传变异 (#5) 深化 = portable_seed + hgt + epigenetic.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.epigenetic import (
    EPIGENETIC_VERSION,
    EpigeneticMechanism,
    EpigeneticMark,
    EpigeneticGeneration,
    methylate,
    histone_modify,
    inherit_mark,
    EpigeneticNetwork,
)


# === 1. Epigenetic 3 真生产机制 (主 13:08 借鉴真生产) ===

class TestEpigeneticMechanisms:
    """Epigenetic 3 真生产机制 (主 14:06 借鉴 Holliday 1989)."""

    def test_3_mechanisms_defined(self):
        assert {m.value for m in EpigeneticMechanism} == {"methylation", "histone_mod", "noncoding_rna"}

    def test_methylation(self):
        assert EpigeneticMechanism.METHYLATION.value == "methylation"

    def test_histone_mod(self):
        assert EpigeneticMechanism.HISTONE_MOD.value == "histone_mod"


# === 2. EpigeneticMark 真生产 (主 14:06 真借鉴) ===

class TestEpigeneticMark:
    """EpigeneticMark 真生产 (主 14:06 + 真借鉴)."""

    def test_mark_default(self):
        m = EpigeneticMark(mark_id="m1", mechanism=EpigeneticMechanism.METHYLATION, gene_id="g1")
        assert m.mark_id == "m1"
        assert m.gene_id == "g1"
        assert m.state == 0.0
        assert m.inherited is False
        assert m.parent_mark_id == ""

    def test_mark_to_dict(self):
        m = EpigeneticMark(mark_id="m1", mechanism=EpigeneticMechanism.METHYLATION, gene_id="g1", state=0.7, inherited=True, parent_mark_id="m0")
        d = m.to_dict()
        assert d["mark_id"] == "m1"
        assert d["mechanism"] == "methylation"
        assert d["gene_id"] == "g1"
        assert d["state"] == 0.7
        assert d["inherited"] is True
        assert d["parent_mark_id"] == "m0"


# === 3. Epigenetic 真生产算法 (主 13:08 借鉴真生产) ===

class TestEpigeneticAlgorithms:
    """epigenetic 真生产算法 (主 14:06 真借鉴)."""

    def test_methylate(self):
        m = methylate("g1", state=0.5, parent_mark_id="m0")
        assert m.mechanism == EpigeneticMechanism.METHYLATION
        assert m.state == 0.5
        assert m.inherited is True
        assert m.parent_mark_id == "m0"

    def test_methylate_no_parent(self):
        m = methylate("g1", state=0.6)
        assert m.inherited is False
        assert m.parent_mark_id == ""

    def test_histone_modify(self):
        m = histone_modify("g1", state=0.5, parent_mark_id="m0")
        assert m.mechanism == EpigeneticMechanism.HISTONE_MOD
        assert m.state == 0.5
        assert m.inherited is True

    def test_inherit_mark_with_fidelity(self):
        """跨代知识迁移真生产 (主 13:08 借鉴主 17:46 跨代遗传)."""
        parent = EpigeneticMark(mark_id="m0", mechanism=EpigeneticMechanism.METHYLATION, gene_id="g1", state=0.8)
        child = inherit_mark(parent, fidelity=0.9)
        assert child.state == pytest.approx(0.72)  # 0.8 * 0.9
        assert child.inherited is True
        assert child.parent_mark_id == "m0"
        assert child.gene_id == "g1"

    def test_inherit_mark_default_fidelity(self):
        """fidelity 默认 1.0 (无 fidelity 衰减)."""
        parent = EpigeneticMark(mark_id="m0", mechanism=EpigeneticMechanism.METHYLATION, gene_id="g1", state=0.5)
        child = inherit_mark(parent)  # fidelity=1.0 默认
        assert child.state == 0.5  # 0.5 * 1.0 = 0.5
        assert child.inherited is True
        assert child.parent_mark_id == "m0"


# === 4. EpigeneticNetwork 真生产主类 (主 13:31 大胆激进) ===

class TestEpigeneticNetwork:
    """EpigeneticNetwork 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        epi = EpigeneticNetwork()
        assert epi.marks == {}
        assert epi.generations == []
        assert epi.current_generation == 0

    def test_add_mark_methylation(self):
        epi = EpigeneticNetwork()
        m = epi.add_mark("g1", mechanism=EpigeneticMechanism.METHYLATION, state=0.7)
        assert "g1" in epi.marks
        assert epi.marks["g1"].state == 0.7
        assert epi.marks["g1"].mechanism == EpigeneticMechanism.METHYLATION

    def test_add_mark_histone(self):
        epi = EpigeneticNetwork()
        m = epi.add_mark("g1", mechanism=EpigeneticMechanism.HISTONE_MOD, state=0.6)
        assert epi.marks["g1"].mechanism == EpigeneticMechanism.HISTONE_MOD
        assert epi.marks["g1"].state == 0.6

    def test_cross_generation_increments(self):
        """跨代知识迁移真生产 (主 13:08 借鉴主 17:46 跨代遗传)."""
        epi = EpigeneticNetwork(default_fidelity=0.9)
        epi.add_mark("g1", state=0.8)
        gen1 = epi.cross_generation()
        gen2 = epi.cross_generation()
        assert gen1.generation_num == 1
        assert gen2.generation_num == 2
        # marks 索引 by gene_id, 每次跨代覆盖 (state = 0.8 * 0.9 = 0.72)
        assert len(epi.marks) == 1  # 1 gene_id, 1 mark

    def test_cross_generation_transfer_high_state(self):
        """state > 0.3 → 跨代成功 (主 17:43 实事求是)."""
        epi = EpigeneticNetwork()
        epi.add_mark("g1", state=0.5)
        epi.cross_generation()
        epi.cross_generation()
        # g1 mark state = 0.5 * 0.9 (1st gen) * 0.9 (2nd gen) = 0.405 > 0.3
        result = epi.cross_generation_transfer("g1", 0, 1)
        assert result is True

    def test_cross_generation_transfer_low_state(self):
        """state ≤ 0.3 → 跨代失败 (主 17:43 实事求是)."""
        epi = EpigeneticNetwork()
        epi.add_mark("g1", state=0.2)
        epi.cross_generation()
        epi.cross_generation()
        result = epi.cross_generation_transfer("g1", 0, 1)
        assert result is False

    def test_cross_generation_missing_mark(self):
        epi = EpigeneticNetwork()
        epi.cross_generation()
        result = epi.cross_generation_transfer("nonexistent", 0, 0)
        assert result is False

    def test_stats_with_marks(self):
        """stats() 真生产统计 (主 17:43 实事求是)."""
        epi = EpigeneticNetwork()
        epi.add_mark("g1", mechanism=EpigeneticMechanism.METHYLATION, state=0.8)
        epi.add_mark("g2", mechanism=EpigeneticMechanism.HISTONE_MOD, state=0.6)
        stats = epi.stats()
        assert stats["n_marks"] == 2
        assert stats["n_methylation"] == 1
        assert stats["n_histone_mod"] == 1

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        epi = EpigeneticNetwork()
        stats = epi.stats()
        assert stats["n_marks"] == 0


# === 5. to_dict 真生产 (主 14:06) ===

class TestEpigeneticToDict:
    """EpigeneticMark + EpigeneticGeneration.to_dict() 真生产."""

    def test_mark_to_dict_keys(self):
        m = EpigeneticMark(mark_id="m1", mechanism=EpigeneticMechanism.METHYLATION, gene_id="g1")
        d = m.to_dict()
        expected_keys = ["mark_id", "mechanism", "gene_id", "state", "inherited", "parent_mark_id"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """epigenetic 不应有假装意识字段."""
        epi = EpigeneticNetwork()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for attr in dir(epi):
            for f in forbidden:
                assert f not in attr.lower() or attr in ("add_mark", "cross_generation", "cross_generation_transfer", "stats"), \
                    f"epigenetic 不应有假装意识字段: {attr}"

    def test_no_asi_reached_claim(self):
        """epigenetic 不应声称已达到 ASI."""
        epi = EpigeneticNetwork()
        epi.add_mark("g1", state=0.7)
        stats = epi.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v

    def test_no_transgenerational_asi_pretend(self):
        """epigenetic 借鉴 Holliday, 不假装"ASI 跨代知识"."""
        epi = EpigeneticNetwork()
        epi.add_mark("g1", state=0.7)
        epi.cross_generation()
        stats = epi.stats()
        philosophy = stats.get("philosophy", "").lower()
        # 不应包含 "transgenerational ASI" 假承诺
        assert "transgenerational asi" not in philosophy


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_epigenetic_is_real_innovation(self):
        """epigenetic 是真创新 (主 13:31), 不 placeholder."""
        epi = EpigeneticNetwork(default_fidelity=0.9)
        for i in range(3):
            epi.add_mark(f"g{i+1}", mechanism=EpigeneticMechanism.METHYLATION, state=0.7)
        epi.cross_generation()
        # 写真 production: 3 methylation marks (跨代 by gene_id) + 跨代真生产
        assert len(epi.marks) == 3  # 3 gene_ids
        assert all(m.inherited for m in epi.marks.values())  # 全部 inherited

    def test_epigenetic_allows_iteration(self):
        """epigenetic 允许迭代 (主 13:31 鼓励尝试)."""
        epi = EpigeneticNetwork()
        epi.add_mark("g1", state=0.8)
        for _ in range(5):
            epi.cross_generation()
        assert len(epi.generations) == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])