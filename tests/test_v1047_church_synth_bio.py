"""Tests for v1047 Church synthetic biology."""
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import pytest

from apeireth.v1047_church_synth_bio import (
    CodonTable,
    RecodedGenome,
    MinimalGenome,
    GibsonAssembly,
    MAGE,
    GenomeRecodingBridge,
    SynthesisCost,
    CodonUsageTable,
    GenomeSafetyGuard,
    ASISyntheticBioBridge,
    GenomeValidate,
    V1047_VERSION,
)


# ----------------------------------------------------------------------
# CodonTable tests
# ----------------------------------------------------------------------


class TestCodonTable:
    def test_standard_total(self):
        ct = CodonTable.standard()
        assert ct.n_total() == 64

    def test_standard_sense(self):
        ct = CodonTable.standard()
        # 64 - 3 stop = 61 sense
        assert ct.n_sense() == 61

    def test_standard_stop(self):
        ct = CodonTable.standard()
        assert ct.n_stop() == 3
        assert "TAA" in ct.stop_codons
        assert "TAG" in ct.stop_codons
        assert "TGA" in ct.stop_codons

    def test_is_sense_stop(self):
        ct = CodonTable.standard()
        assert ct.is_sense("ATG")  # methionine
        assert ct.is_stop("TAA")
        assert not ct.is_sense("TAA")
        assert not ct.is_stop("ATG")

    def test_recoded_57(self):
        ct = CodonTable.recoded_57()
        assert "TAG" in ct.recoded
        assert "TGA" in ct.recoded
        assert ct.is_recoded("TAG")
        assert not ct.is_recoded("ATG")

    def test_summary_keys(self):
        ct = CodonTable.standard()
        s = ct.summary()
        assert "n_total" in s
        assert "n_sense" in s
        assert "n_stop" in s
        assert "n_recoded" in s


# ----------------------------------------------------------------------
# RecodedGenome tests
# ----------------------------------------------------------------------


class TestRecodedGenome:
    def test_default_valid(self):
        rg = RecodedGenome()
        assert rg.validate()

    def test_coding_density(self):
        rg = RecodedGenome(genome_size_bp=1_000_000, n_genes=100)
        assert abs(rg.coding_density() - 100.0) < 0.01

    def test_zero_size_invalid(self):
        rg = RecodedGenome(genome_size_bp=0)
        assert not rg.validate()

    def test_zero_genes_invalid(self):
        rg = RecodedGenome(n_genes=0)
        assert not rg.validate()

    def test_pct_recoded_range(self):
        rg = RecodedGenome(pct_recoded=120.0)
        assert not rg.validate()
        rg2 = RecodedGenome(pct_recoded=-5.0)
        assert not rg2.validate()

    def test_to_dict_keys(self):
        rg = RecodedGenome()
        d = rg.to_dict()
        assert "genome_size_bp" in d
        assert "n_genes" in d
        assert "pct_recoded" in d
        assert "coding_density_per_mb" in d
        assert "valid" in d


# ----------------------------------------------------------------------
# MinimalGenome tests
# ----------------------------------------------------------------------


class TestMinimalGenome:
    def test_default_valid(self):
        mg = MinimalGenome()
        assert mg.validate()
        # JCVI-syn3.0 has 473 essential genes
        assert mg.n_essential == 473

    def test_essential_density(self):
        mg = MinimalGenome(n_essential=473, n_quasi_essential=149)
        d = mg.essential_density()
        assert 0.7 < d < 0.8

    def test_zero_essential_invalid(self):
        mg = MinimalGenome(n_essential=0)
        assert not mg.validate()

    def test_to_dict(self):
        mg = MinimalGenome()
        d = mg.to_dict()
        assert "n_essential" in d
        assert "n_quasi_essential" in d
        assert "growth_rate_doubling_h" in d


# ----------------------------------------------------------------------
# GibsonAssembly tests
# ----------------------------------------------------------------------


class TestGibsonAssembly:
    def test_zero_fragments_zero_efficiency(self):
        ga = GibsonAssembly()
        assert ga.assembly_efficiency() == 0.0

    def test_single_fragment_zero_efficiency(self):
        ga = GibsonAssembly(fragments=["ATCG"])
        assert ga.assembly_efficiency() == 0.0

    def test_two_fragments_high_efficiency(self):
        ga = GibsonAssembly(fragments=["ATCG", "GCAT"], overlap_bp=25)
        e = ga.assembly_efficiency()
        assert 0.9 < e <= 1.0

    def test_more_fragments_lower_efficiency(self):
        ga2 = GibsonAssembly(fragments=["ATCG", "GCAT"], overlap_bp=25)
        ga5 = GibsonAssembly(fragments=["A", "B", "C", "D", "E"], overlap_bp=25)
        assert ga2.assembly_efficiency() > ga5.assembly_efficiency()

    def test_higher_overlap_higher_efficiency(self):
        ga_low = GibsonAssembly(fragments=["A", "B"], overlap_bp=5)
        ga_high = GibsonAssembly(fragments=["A", "B"], overlap_bp=30)
        assert ga_high.assembly_efficiency() > ga_low.assembly_efficiency()

    def test_to_dict(self):
        ga = GibsonAssembly(fragments=["A", "B"])
        d = ga.to_dict()
        assert "n_fragments" in d
        assert "assembly_efficiency" in d


# ----------------------------------------------------------------------
# MAGE tests
# ----------------------------------------------------------------------


class TestMAGE:
    def test_cumulative_coverage_zero_oligos(self):
        m = MAGE(n_oligos=0, n_cycles=5)
        # n_oligos=0 → per_cycle=0 → coverage=0
        assert m.cumulative_coverage() == 0.0

    def test_cumulative_increases_with_cycles(self):
        m1 = MAGE(n_oligos=10, n_cycles=1)
        m5 = MAGE(n_oligos=10, n_cycles=5)
        assert m5.cumulative_coverage() > m1.cumulative_coverage()

    def test_cumulative_increases_with_oligos(self):
        m_low = MAGE(n_oligos=1, n_cycles=5)
        m_high = MAGE(n_oligos=100, n_cycles=5)
        assert m_high.cumulative_coverage() > m_low.cumulative_coverage()

    def test_to_dict(self):
        m = MAGE()
        d = m.to_dict()
        assert "cumulative_coverage" in d


# ----------------------------------------------------------------------
# GenomeRecodingBridge tests
# ----------------------------------------------------------------------


class TestGenomeRecodingBridge:
    def test_recoded_fraction_perfect(self):
        gb = GenomeRecodingBridge(success_rate=1.0, n_target_codons=3)
        assert abs(gb.recoded_fraction() - 1.0) < 0.001

    def test_recoded_fraction_realistic(self):
        gb = GenomeRecodingBridge(success_rate=0.99, n_target_codons=3)
        f = gb.recoded_fraction()
        assert 0.95 < f < 1.0

    def test_viral_resistance_increases_with_target_codons(self):
        gb1 = GenomeRecodingBridge(n_target_codons=1)
        gb3 = GenomeRecodingBridge(n_target_codons=3)
        assert gb3.viral_resistance_score() > gb1.viral_resistance_score()

    def test_to_dict(self):
        gb = GenomeRecodingBridge()
        d = gb.to_dict()
        assert "viral_resistance_score" in d


# ----------------------------------------------------------------------
# SynthesisCost tests
# ----------------------------------------------------------------------


class TestSynthesisCost:
    def test_total_cost_per_bp(self):
        sc = SynthesisCost()
        assert sc.total_cost_per_bp() > sc.cost_per_bp_usd

    def test_1mb_cost(self):
        sc = SynthesisCost()
        d = sc.to_dict()
        assert d["1mb_genome_cost_usd"] > 0

    def test_4mb_4x_1mb(self):
        sc = SynthesisCost()
        d = sc.to_dict()
        assert abs(d["4mb_genome_cost_usd"] - 4 * d["1mb_genome_cost_usd"]) < 0.01


# ----------------------------------------------------------------------
# CodonUsageTable tests
# ----------------------------------------------------------------------


class TestCodonUsageTable:
    def test_cai_empty(self):
        cu = CodonUsageTable()
        assert cu.cai("") == 0.0

    def test_cai_with_table(self):
        cu = CodonUsageTable()
        cu.add("ATG", 1.0)
        cu.add("GCG", 0.9)
        cu.add("TTT", 0.5)
        cai = cu.cai("ATGGCGTTT")
        assert 0.0 < cai < 1.0

    def test_cai_short_gene(self):
        cu = CodonUsageTable()
        cu.add("ATG", 1.0)
        # gene with 4 bases → 1 codon
        cai = cu.cai("ATGA")
        assert cai == 0.0  # not a full codon

    def test_to_dict(self):
        cu = CodonUsageTable()
        cu.add("ATG", 1.0)
        d = cu.to_dict()
        assert "n_codons" in d
        assert d["n_codons"] == 1


# ----------------------------------------------------------------------
# GenomeSafetyGuard tests
# ----------------------------------------------------------------------


class TestGenomeSafetyGuard:
    def test_default_layers(self):
        gs = GenomeSafetyGuard()
        assert gs.n_layers() == 5  # default 5 True
        assert gs.is_dual_use_safe()

    def test_minimal_unsafe(self):
        gs = GenomeSafetyGuard(auxotrophy=False, recoded_codon=False, kill_switch=False, hgt_block=False, semantic_firewall=False, physical_containment=False)
        assert gs.n_layers() == 0
        assert not gs.is_dual_use_safe()

    def test_containment_score_max(self):
        gs = GenomeSafetyGuard()
        assert gs.containment_score() == 1.0

    def test_to_dict(self):
        gs = GenomeSafetyGuard()
        d = gs.to_dict()
        assert "n_layers" in d
        assert "containment_score" in d
        assert "dual_use_safe" in d


# ----------------------------------------------------------------------
# ASISyntheticBioBridge tests
# ----------------------------------------------------------------------


class TestASISyntheticBioBridge:
    def test_weights_sum_to_1(self):
        sb = ASISyntheticBioBridge()
        assert sb.validate_weights()

    def test_coverage_zero(self):
        sb = ASISyntheticBioBridge()
        assert sb.coverage([]) == 0.0

    def test_coverage_full(self):
        sb = ASISyntheticBioBridge()
        all_keys = list(sb.weights.keys())
        cov = sb.coverage(all_keys)
        assert cov > 0.99

    def test_to_dict(self):
        sb = ASISyntheticBioBridge()
        d = sb.to_dict()
        assert "v01_real_production" in d
        assert d["n_weight_slots"] == 10


# ----------------------------------------------------------------------
# GenomeValidate tests
# ----------------------------------------------------------------------


class TestGenomeValidate:
    def test_all_channels(self):
        gv = GenomeValidate(
            genome=RecodedGenome(),
            safety=GenomeSafetyGuard(),
            cost=SynthesisCost(),
            bridge=ASISyntheticBioBridge(),
        )
        d = gv.to_dict()
        assert "channel_coding" in d
        assert "channel_safety" in d
        assert "channel_cost" in d
        assert "channel_production" in d
        assert "overall" in d
        assert 0.0 < gv.overall() < 1.0

    def test_no_components_zero_overall(self):
        gv = GenomeValidate()
        assert gv.overall() == 0.0

    def test_budget_too_low(self):
        gv = GenomeValidate(
            genome=RecodedGenome(genome_size_bp=4_000_000),
            cost=SynthesisCost(),
        )
        d = gv.to_dict()
        # Default budget is 10M, cost for 4MB ~ 0.5M
        assert d["channel_cost"] > 0.99


# ----------------------------------------------------------------------
# V3 Philosophy Guard
# ----------------------------------------------------------------------


class TestV3PhilosophyGuard:
    def test_no_phenomenal_claim(self):
        """No module name or docstring claims Phenomenal consciousness."""
        import apeireth.v1047_church_synth_bio as m
        src = open(m.__file__, encoding="utf-8").read()
        assert "phenomenal consciousness" not in src.lower()
        assert "I am conscious" not in src
        assert "I feel" not in src
        assert "I have feelings" not in src

    def test_no_asi_claim(self):
        import apeireth.v1047_church_synth_bio as m
        src = open(m.__file__, encoding="utf-8").read()
        # Should contain "不假装 ASI" type guard
        assert "不假装" in src
        # Should NOT claim "we are ASI" / "we have achieved ASI"
        assert "we are ASI" not in src.lower()
        assert "we have achieved ASI" not in src.lower()

    def test_synthesis_biology_is_means_not_end(self):
        """Synthetic biology is a component, not the goal of ASI."""
        import apeireth.v1047_church_synth_bio as m
        src = open(m.__file__, encoding="utf-8").read()
        # Should state synthetic biology is a sub-module, not ASI itself
        assert "子" in src  # 子模块 / 子结构

    def test_saturation_acknowledged(self):
        """Acknowledge open problems, not saturation."""
        import apeireth.v1047_church_synth_bio as m
        src = open(m.__file__, encoding="utf-8").read()
        # Should not claim "saturated" or "complete"
        assert "不假装饱和" in src
        # Should acknowledge open problems
        assert "开放问题" in src or "未答" in src or "open" in src.lower()


# ----------------------------------------------------------------------
# Version + Bold innovation (主 13:31)
# ----------------------------------------------------------------------


class TestBoldInnovation:
    def test_version_present(self):
        assert V1047_VERSION.startswith("0.")

    def test_novel_combinations_present(self):
        """Bridge between synthetic biology + ASI V0.1 (主 13:31 bold)."""
        from apeireth.v1047_church_synth_bio import (
            ASISyntheticBioBridge,
            GenomeRecodingBridge,
            GenomeValidate,
        )
        sb = ASISyntheticBioBridge()
        gb = GenomeRecodingBridge(n_target_codons=3, success_rate=0.99)
        gv = GenomeValidate(bridge=sb, genome=RecodedGenome())
        d = gv.to_dict()
        assert "overall" in d
        assert d["overall"] >= 0.0

    def test_real_world_anchors(self):
        """Real papers / scientists are cited (主 19:33 走在前人经验上)."""
        import apeireth.v1047_church_synth_bio as m
        src = open(m.__file__, encoding="utf-8").read()
        # Real anchor names
        assert "George Church" in src
        assert "Lajoie" in src
        assert "Hutchison" in src
        assert "Ostrov" in src
        assert "JCVI-syn3" in src
        assert "57-codon" in src
        assert "Gibson" in src
        assert "MAGE" in src


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
