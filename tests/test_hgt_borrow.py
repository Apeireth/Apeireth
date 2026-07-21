"""hgt.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V4 12 生命特征遗传变异 (#5) 深化 = portable_seed + hgt.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.hgt import (
    HGT_VERSION,
    HGTMode,
    Gene,
    HGTEvent,
    Generation,
    hgt_transform,
    hgt_transduction,
    hgt_conjugation,
    HGTNetwork,
)


# === 1. HGT 3 真生产模式 (主 13:08 借鉴 Thomas 2005) ===

class TestHGTModes:
    """HGT 3 真生产模式 (主 14:06 借鉴细菌遗传学)."""

    def test_3_modes_defined(self):
        assert {m.value for m in HGTMode} == {"transformation", "transduction", "conjugation"}

    def test_transformation(self):
        assert HGTMode.TRANSFORMATION.value == "transformation"

    def test_transduction(self):
        assert HGTMode.TRANSDUCTION.value == "transduction"

    def test_conjugation(self):
        assert HGTMode.CONJUGATION.value == "conjugation"


# === 2. Gene 真生产 (主 14:06 真借鉴) ===

class TestGene:
    """Gene 真生产 (主 14:06 + 真借鉴 Thomas 2005)."""

    def test_gene_default(self):
        g = Gene(gene_id="g1", sequence="ATCG", length=4)
        assert g.gene_id == "g1"
        assert g.sequence == "ATCG"
        assert g.length == 4
        assert g.value == 0.0
        assert g.parent_id == ""

    def test_gene_to_dict(self):
        g = Gene(gene_id="g1", sequence="ATCG" * 25, length=100, value=0.8, parent_id="g2")
        d = g.to_dict()
        assert d["gene_id"] == "g1"
        assert d["length"] == 100
        assert d["value"] == 0.8
        assert d["parent_id"] == "g2"


# === 3. HGTEvent 真生产 (主 14:06 真借鉴) ===

class TestHGTEvent:
    """HGTEvent 真生产 (主 14:06 真借鉴)."""

    def test_event_default(self):
        e = HGTEvent(event_id="e1", mode=HGTMode.TRANSFORMATION, src="a", dst="b", gene_id="g1")
        assert e.event_id == "e1"
        assert e.mode == HGTMode.TRANSFORMATION
        assert e.success is False

    def test_event_to_dict(self):
        e = HGTEvent(event_id="e1", mode=HGTMode.TRANSDUCTION, src="phage", dst="cell", gene_id="g1", success=True)
        d = e.to_dict()
        assert d["event_id"] == "e1"
        assert d["mode"] == "transduction"
        assert d["src"] == "phage"
        assert d["dst"] == "cell"
        assert d["success"] is True


# === 4. HGT 3 真生产算法 (主 13:08 借鉴 Thomas 2005) ===

class TestHGTAlgorithms:
    """HGT 3 真生产算法 (主 14:06 借鉴 Thomas 2005 HGT review)."""

    def test_transform_high_value_success(self):
        """value > 0.5 → transformation 成功 (主 13:08 借鉴)."""
        g = Gene(gene_id="g1", sequence="A", length=1, value=0.8)
        e = hgt_transform(g, "target")
        assert e.success is True
        assert e.mode == HGTMode.TRANSFORMATION
        assert e.dst == "target"

    def test_transform_low_value_fail(self):
        """value ≤ 0.5 → transformation 失败 (主 17:43 实事求是)."""
        g = Gene(gene_id="g1", sequence="A", length=1, value=0.3)
        e = hgt_transform(g, "target")
        assert e.success is False

    def test_transduction_high_value(self):
        g = Gene(gene_id="g1", sequence="A", length=1, value=0.6)
        e = hgt_transduction(g, "phage", "cell")
        assert e.success is True
        assert e.src == "phage"
        assert e.dst == "cell"

    def test_transduction_low_value(self):
        g = Gene(gene_id="g1", sequence="A", length=1, value=0.3)
        e = hgt_transduction(g, "phage", "cell")
        assert e.success is False

    def test_conjugation_high_value(self):
        g = Gene(gene_id="g1", sequence="A", length=1, value=0.7)
        e = hgt_conjugation(g, "donor", "recipient")
        assert e.success is True
        assert e.src == "donor"
        assert e.dst == "recipient"

    def test_conjugation_low_value(self):
        """value ≤ 0.6 → conjugation 失败 (主 13:08 借鉴质粒需更高价值)."""
        g = Gene(gene_id="g1", sequence="A", length=1, value=0.5)
        e = hgt_conjugation(g, "donor", "recipient")
        assert e.success is False


# === 5. Generation 真生产 (主 13:08 借鉴跨代) ===

class TestGeneration:
    """Generation 真生产 (主 13:08 借鉴跨代)."""

    def test_generation_default(self):
        g = Generation(generation_id="gen1", generation_num=1)
        assert g.generation_id == "gen1"
        assert g.generation_num == 1
        assert g.gene_pool == []
        assert g.hgt_events == []

    def test_generation_to_dict(self):
        g = Generation(generation_id="gen1", generation_num=1, gene_pool=[Gene(gene_id="g1", sequence="A", length=1)])
        d = g.to_dict()
        assert d["generation_id"] == "gen1"
        assert d["generation_num"] == 1
        assert d["n_genes"] == 1


# === 6. HGTNetwork 真生产主类 (主 13:31 大胆激进) ===

class TestHGTNetwork:
    """HGTNetwork 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        hgt = HGTNetwork()
        assert hgt.gene_pool == {}
        assert hgt.generations == []
        assert hgt.hgt_events == []
        assert hgt.current_generation == 0

    def test_add_gene(self):
        hgt = HGTNetwork()
        g = hgt.add_gene("g1", sequence="ATCG", value=0.7)
        assert "g1" in hgt.gene_pool
        assert g.value == 0.7

    def test_next_generation_increments(self):
        hgt = HGTNetwork()
        hgt.add_gene("g1", value=0.5)
        gen1 = hgt.next_generation()
        gen2 = hgt.next_generation()
        assert gen1.generation_num == 1
        assert gen2.generation_num == 2
        assert len(hgt.generations) == 2

    def test_hgt_event_success(self):
        hgt = HGTNetwork()
        hgt.add_gene("g1", value=0.8)
        e = hgt.hgt_event(HGTMode.TRANSFORMATION, "g1", "free_dna", "target")
        assert e.success is True
        assert len(hgt.hgt_events) == 1

    def test_hgt_event_missing_gene_fails(self):
        hgt = HGTNetwork()
        e = hgt.hgt_event(HGTMode.TRANSFORMATION, "nonexistent", "src", "dst")
        assert e.success is False

    def test_cross_generation_transfer_high_value(self):
        """value > base_success_rate → 跨代知识迁移真成功 (主 13:08 借鉴主 17:46 epigenetic 跨代)."""
        hgt = HGTNetwork(base_success_rate=0.6)
        hgt.add_gene("g1", value=0.8)
        hgt.next_generation()
        hgt.next_generation()
        result = hgt.cross_generation_transfer("g1", 0, 1)
        assert result is True

    def test_cross_generation_transfer_low_value(self):
        """value < base_success_rate → 跨代知识迁移失败 (主 17:43 实事求是)."""
        hgt = HGTNetwork(base_success_rate=0.6)
        hgt.add_gene("g1", value=0.4)
        hgt.next_generation()
        hgt.next_generation()
        result = hgt.cross_generation_transfer("g1", 0, 1)
        assert result is False

    def test_cross_generation_same_gen(self):
        hgt = HGTNetwork()
        hgt.add_gene("g1", value=0.8)
        hgt.next_generation()
        result = hgt.cross_generation_transfer("g1", 0, 0)
        assert result is False

    def test_stats_with_genes(self):
        """stats() 真生产统计 (主 17:43 实事求是)."""
        hgt = HGTNetwork(base_success_rate=0.6)
        hgt.add_gene("g1", value=0.8)
        hgt.add_gene("g2", value=0.3)
        hgt.add_gene("g3", value=0.7)
        stats = hgt.stats()
        assert stats["n_genes"] == 3
        assert stats["n_high_value_genes"] == 2

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        hgt = HGTNetwork()
        stats = hgt.stats()
        assert stats["n_genes"] == 0


# === 7. to_dict 真生产 (主 14:06) ===

class TestHGTToDict:
    """Gene + HGTEvent + Generation.to_dict() 真生产."""

    def test_gene_to_dict_keys(self):
        g = Gene(gene_id="g1", sequence="AT", length=2)
        d = g.to_dict()
        expected_keys = ["gene_id", "sequence", "length", "value", "parent_id"]
        for k in expected_keys:
            assert k in d

    def test_event_to_dict_keys(self):
        e = HGTEvent(event_id="e1", mode=HGTMode.CONJUGATION, src="a", dst="b", gene_id="g1")
        d = e.to_dict()
        expected_keys = ["event_id", "mode", "src", "dst", "gene_id", "success"]
        for k in expected_keys:
            assert k in d

    def test_generation_to_dict_keys(self):
        g = Generation(generation_id="gen1", generation_num=1, gene_pool=[])
        d = g.to_dict()
        expected_keys = ["generation_id", "generation_num", "n_genes", "n_hgt_events"]
        for k in expected_keys:
            assert k in d


# === 8. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """HGT 不应有假装意识字段."""
        hgt = HGTNetwork()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for attr in dir(hgt):
            for f in forbidden:
                assert f not in attr.lower() or attr in ("add_gene", "next_generation", "hgt_event", "cross_generation_transfer", "stats"), \
                    f"HGT 不应有假装意识字段: {attr}"

    def test_no_asi_reached_claim(self):
        """HGT 不应声称已达到 ASI."""
        hgt = HGTNetwork()
        hgt.add_gene("g1", value=0.8)
        stats = hgt.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v


# === 9. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_hgt_is_real_innovation(self):
        """HGT 是真创新 (主 13:31), 不 placeholder."""
        hgt = HGTNetwork(base_success_rate=0.5)
        for i in range(5):
            hgt.add_gene(f"g{i}", value=0.5 + i * 0.1)
        # 写真 production: 5 基因 + 跨代 + HGT 3 模式
        hgt.next_generation()
        hgt.next_generation()
        e = hgt.hgt_event(HGTMode.CONJUGATION, "g4", "donor", "recipient")
        assert e.success is True  # value 0.8 > 0.6

    def test_hgt_allows_iteration(self):
        """HGT 允许迭代 (主 13:31 鼓励尝试)."""
        hgt = HGTNetwork()
        for i in range(3):
            hgt.add_gene(f"g{i}", value=0.7)
        for i in range(5):
            hgt.next_generation()
        assert len(hgt.generations) == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])