"""Phase 1059 v1059_asi_cross_domain — V1059 Cross-Domain Foundation tests.

主 17:43 实事求是: 真借鉴 + 真算法 + 真跑真测 + 真 commit.
主 00:56 任何人都能接手: 任何人都能读懂 + 测试 + 部署.
"""
from __future__ import annotations

import json
import math
from typing import Dict, Tuple

import pytest

from apeireth import v1059_asi_cross_domain as m


V1059 = m.V1059_VERSION


# ---------------------------------------------------------------------------
# Reference sanity
# ---------------------------------------------------------------------------


def test_version_constant() -> None:
    assert V1059 == "0.1.0"


def test_references_count() -> None:
    """14 real references — 7 领域 (主 19:33)."""
    refs = m.REFERENCES
    assert len(refs) >= 10, f"Expected ≥10, got {len(refs)}"
    ref_ids = {r["id"] for r in refs}
    assert "Eigen1971" in ref_ids
    assert "Landauer1961" in ref_ids
    assert "Shannon1948" in ref_ids


# ---------------------------------------------------------------------------
# Component 1: BiologyDomain
# ---------------------------------------------------------------------------


class TestBiologyDomain:
    def test_analyze_returns_result(self) -> None:
        bio = m.BiologyDomain()
        result = bio.analyze()
        assert result.domain == "biology"
        assert 0 <= result.relevance_score <= 1.0
        assert len(result.concepts) >= 4

    def test_quasispecies_score(self) -> None:
        bio = m.BiologyDomain()
        score = bio.quasispecies_score(mutation_rate=0.001, sequence_length=1000)
        assert 0.0 <= score <= 1.0

    def test_quasispecies_error_catastrophe(self) -> None:
        bio = m.BiologyDomain()
        score = bio.quasispecies_score(mutation_rate=0.5, sequence_length=10)
        assert score == 0.0  # above error threshold

    def test_autocatalytic_diversity(self) -> None:
        bio = m.BiologyDomain()
        score = bio.autocatalytic_diversity(components=10, connections=20)
        assert 0.0 <= score <= 1.0

    def test_compute_actual_formulas(self) -> None:
        bio = m.BiologyDomain()
        threshold = 1.0 / 500
        score = bio.quasispecies_score(mutation_rate=threshold * 0.5, sequence_length=500)
        assert score >= 0.5, f"Expected >=0.5, got {score}"


# ---------------------------------------------------------------------------
# Component 2: PhysicsDomain
# ---------------------------------------------------------------------------


class TestPhysicsDomain:
    def test_analyze(self) -> None:
        phy = m.PhysicsDomain()
        result = phy.analyze()
        assert result.domain == "physics"

    def test_landauer_energy(self) -> None:
        phy = m.PhysicsDomain()
        e = phy.landauer_energy(bits_erased=1)
        assert e > 0  # always positive
        assert e < 1e-20  # very small (kT ln 2 ~ 2.87e-21 J)

    def test_landauer_scales(self) -> None:
        phy = m.PhysicsDomain()
        e1 = phy.landauer_energy(bits_erased=1)
        e10 = phy.landauer_energy(bits_erased=10)
        assert math.isclose(e10 / e1, 10.0, rel_tol=1e-9)

    def test_reversibility_gain(self) -> None:
        phy = m.PhysicsDomain()
        assert phy.reversibility_gain(1.0) == 1.0
        assert phy.reversibility_gain(0.0) == 0.0
        assert phy.reversibility_gain(0.5) == 0.5


# ---------------------------------------------------------------------------
# Component 3: MathDomain
# ---------------------------------------------------------------------------


class TestMathDomain:
    def test_analyze(self) -> None:
        math_d = m.MathDomain()
        result = math_d.analyze()
        assert result.domain == "mathematics"
        assert result.relevance_score >= 0.8

    def test_kolmogorov_complexity(self) -> None:
        md = m.MathDomain()
        c1 = md.kolmogorov_complexity("a" * 1000)
        c2 = md.kolmogorov_complexity("".join(chr(i % 128) for i in range(1000)))
        # Highly repetitive should compress smaller
        assert c1 <= c2, "Repetitive string should have lower Kolmogorov complexity"

    def test_algorithmic_probability(self) -> None:
        md = m.MathDomain()
        p = md.algorithmic_probability("hello", universe_size=2**16)
        assert 0 <= p <= 1.0

    def test_empty_kolmogorov(self) -> None:
        md = m.MathDomain()
        assert md.kolmogorov_complexity("") == 0


# ---------------------------------------------------------------------------
# Component 4: CognitiveDomain
# ---------------------------------------------------------------------------


class TestCognitiveDomain:
    def test_analyze(self) -> None:
        cog = m.CognitiveDomain()
        result = cog.analyze()
        assert result.domain == "cognition"

    def test_analogy_score_perfect_match(self) -> None:
        cog = m.CognitiveDomain()
        score = cog.analogy_score(["a", "b"], ["a", "b"])
        assert score == 1.0

    def test_analogy_score_no_match(self) -> None:
        cog = m.CognitiveDomain()
        score = cog.analogy_score(["x", "y"], ["a", "b"])
        assert score == 0.0

    def test_analogy_score_partial(self) -> None:
        cog = m.CognitiveDomain()
        score = cog.analogy_score(["a", "b", "c"], ["a", "d", "e"])
        assert math.isclose(score, 1/5, rel_tol=1e-4)  # 1 common / 5 union

    def test_analogy_empty(self) -> None:
        cog = m.CognitiveDomain()
        assert cog.analogy_score([], ["a"]) == 0.0


# ---------------------------------------------------------------------------
# Component 5: EcologyDomain
# ---------------------------------------------------------------------------


class TestEcologyDomain:
    def test_analyze(self) -> None:
        eco = m.EcologyDomain()
        result = eco.analyze()
        assert result.domain == "ecology"

    def test_may_stability_index(self) -> None:
        eco = m.EcologyDomain()
        # Low complexity → high stability
        s1 = eco.may_stability_index(1, 0.1)
        # High complexity → low stability
        s2 = eco.may_stability_index(100, 0.5)
        assert s1 > s2, "May: simpler systems should be more stable"

    def test_may_stability_edge_cases(self) -> None:
        eco = m.EcologyDomain()
        assert eco.may_stability_index(0, 0) == 1.0
        assert eco.may_stability_index(1, 0) == 1.0


# ---------------------------------------------------------------------------
# Component 6: SystemDomain
# ---------------------------------------------------------------------------


class TestSystemDomain:
    def test_analyze(self) -> None:
        sys_d = m.SystemDomain()
        result = sys_d.analyze()
        assert result.domain == "systems"

    def test_feedback_stability(self) -> None:
        sd = m.SystemDomain()
        assert sd.feedback_stability(0.5, 0.1) == 1.0  # stable

    def test_feedback_stability_unstable(self) -> None:
        sd = m.SystemDomain()
        # High gain * delay > 1 → less stable
        val = sd.feedback_stability(5.0, 1.0)
        assert val < 1.0  # unstable

    def test_feedback_zero_gain(self) -> None:
        sd = m.SystemDomain()
        assert sd.feedback_stability(0.0, 1.0) == 1.0


# ---------------------------------------------------------------------------
# Component 7: InformationDomain
# ---------------------------------------------------------------------------


class TestInformationDomain:
    def test_analyze(self) -> None:
        info = m.InformationDomain()
        result = info.analyze()
        assert result.domain == "information"

    def test_shannon_entropy_uniform(self) -> None:
        info = m.InformationDomain()
        entropy = info.shannon_entropy("abcd" * 100)
        assert entropy > 1.5  # 4 uniform symbols → 2 bits

    def test_shannon_entropy_uniform_single(self) -> None:
        info = m.InformationDomain()
        entropy = info.shannon_entropy("aaaa")
        assert entropy == 0.0  # no uncertainty

    def test_mutual_information(self) -> None:
        info = m.InformationDomain()
        # Perfect correlation
        counts: Dict[Tuple[str, str], int] = {("a", "a"): 10, ("b", "b"): 10}
        mi = info.mutual_information(counts)
        assert mi > 0.5, f"High mutual info expected >0.5, got {mi}"

    def test_mutual_information_independent(self) -> None:
        info = m.InformationDomain()
        counts: Dict[Tuple[str, str], int] = {("a", "a"): 5, ("a", "b"): 5, ("b", "a"): 5, ("b", "b"): 5}
        mi = info.mutual_information(counts)
        assert mi == 0.0, f"Independent should have 0 MI, got {mi}"

    def test_mutual_information_empty(self) -> None:
        info = m.InformationDomain()
        assert info.mutual_information({}) == 0.0


# ---------------------------------------------------------------------------
# Component 8: CrossDomainBridge
# ---------------------------------------------------------------------------


class TestCrossDomainBridge:
    def test_bridge_finds_concepts(self) -> None:
        bridge = m.CrossDomainBridge()
        b = bridge.bridge("biology", "information")
        assert len(b) >= 1, "Expected at least 1 bridge between biology and information"

    def test_bridge_return_format(self) -> None:
        bridge = m.CrossDomainBridge()
        b = bridge.bridge("mathematics", "information")
        if b:
            assert "concept" in b[0]
            assert "source_relevance" in b[0]
            assert "bridge_strength" in b[0]

    def test_unrelated_domains(self) -> None:
        bridge = m.CrossDomainBridge()
        b = bridge.bridge("biology", "ecology")
        assert len(b) >= 1  # similar domains should have bridges

    def test_cross_domain_matrix(self) -> None:
        bridge = m.CrossDomainBridge()
        matrix = bridge.cross_domain_matrix(m.DOMAIN_NAMES)
        assert len(matrix) == 7
        for d in m.DOMAIN_NAMES:
            assert matrix[d][d] == 1.0  # self-bridge


# ---------------------------------------------------------------------------
# Component 9: CrossDomainMetric
# ---------------------------------------------------------------------------


class TestCrossDomainMetric:
    def test_diversity_score(self) -> None:
        metric = m.CrossDomainMetric()
        score = metric.diversity_score()
        assert 0.0 <= score <= 1.0

    def test_integration_score_self(self) -> None:
        metric = m.CrossDomainMetric()
        matrix = {"a": {"a": 1.0, "b": 0.5}, "b": {"a": 0.5, "b": 1.0}}
        score = metric.integration_score(matrix)
        assert score == 0.5

    def test_integration_score_perfect(self) -> None:
        metric = m.CrossDomainMetric()
        # Full matrix (not sparse)
        matrix = {"a": {"a": 1.0, "b": 0.8}, "b": {"a": 0.8, "b": 1.0}}
        score = metric.integration_score(matrix)
        assert score == 0.8

    def test_integration_score_single_domain(self) -> None:
        metric = m.CrossDomainMetric()
        score = metric.integration_score({"a": {"a": 1.0}})
        assert score == 1.0


# ---------------------------------------------------------------------------
# Component 10: CrossDomainReport
# ---------------------------------------------------------------------------


class TestCrossDomainReport:
    def test_to_markdown(self) -> None:
        report = m.CrossDomainReport(
            domains_analyzed=7,
            total_concepts=42,
            bridges_count=28,
            diversity=0.85,
            integration=0.72,
            asi_contribution=0.107,
        )
        md = report.to_markdown()
        assert "7" in md
        assert "42" in md
        assert "85.00%" in md or "72.00%" in md
        assert "Biology" in md
        assert "Physics" in md
        assert "Eigen" in md
        assert "Shannon" in md

    def test_default_fields(self) -> None:
        report = m.CrossDomainReport(
            domains_analyzed=0, total_concepts=0, bridges_count=0,
            diversity=0.0, integration=0.0, asi_contribution=0.0,
        )
        md = report.to_markdown()
        assert "0" in md


# ---------------------------------------------------------------------------
# Component 11: ASICrossDomainBridge
# ---------------------------------------------------------------------------


class TestASICrossDomainBridge:
    def test_score_cross_domain_all_perfect(self) -> None:
        bridge = m.ASICrossDomainBridge()
        scores = {d: 1.0 for d in bridge.domain_weights}
        score = bridge.score_cross_domain(scores)
        assert math.isclose(score, 1.0, rel_tol=1e-4)

    def test_cross_domain_weighted(self) -> None:
        bridge = m.ASICrossDomainBridge()
        scores = {d: (0.5 if d == "mathematics" else 0.0) for d in bridge.domain_weights}
        score = bridge.score_cross_domain(scores)
        # Only math contributes (weight 0.16)
        expected = 0.5 * 0.16
        assert math.isclose(score, expected, rel_tol=1e-4)

    def test_generate_report(self) -> None:
        bridge = m.ASICrossDomainBridge()
        report = bridge.generate_report(
            domain_scores={"biology": 0.8, "physics": 0.7, "mathematics": 0.9, "cognition": 0.85, "ecology": 0.6, "systems": 0.75, "information": 0.95},
            diversity=0.82,
            integration=0.68,
        )
        assert "asi_v02_cross_domain" in report
        assert "cross_domain_contribution" in report
        assert report["cross_domain_contribution"] <= 0.15  # max weight
        assert "philosophy_guard" in report

    def test_score_empty(self) -> None:
        bridge = m.ASICrossDomainBridge()
        assert bridge.score_cross_domain({}) == 0.0


# ---------------------------------------------------------------------------
# Integration: run_all
# ---------------------------------------------------------------------------


class TestRunAll:
    def test_run_all_success(self) -> None:
        result = m.run_all(verbose=False)
        assert result["status"] == "success"
        assert "domains" in result
        assert "asi_bridge" in result
        assert "philosophy_guard" in result

    def test_domains_analyzed(self) -> None:
        result = m.run_all(verbose=False)
        assert len(result["domains"]) == 7

    def test_asi_bridge(self) -> None:
        result = m.run_all(verbose=False)
        assert 0.0 <= result["asi_bridge"]["asi_v02_cross_domain"] <= 1.0

    def test_reports_generated(self) -> None:
        result = m.run_all(verbose=False)
        assert result["report"]["length"] > 100


# ---------------------------------------------------------------------------
# V3 Philosophy Guard — 主 17:58+20:46
# ---------------------------------------------------------------------------


def test_philosophy_guard_present() -> None:
    bridge = m.ASICrossDomainBridge()
    report = bridge.generate_report({}, 0.0, 0.0)
    guard = report["philosophy_guard"]
    assert guard["do_not_pretend_cross_domain_is_asi"] is True
    assert guard["do_not_pretend_analogical_is_formal"] is True


def test_cross_domain_contribution_bounded() -> None:
    bridge = m.ASICrossDomainBridge()
    report = bridge.generate_report(
        domain_scores={d: 1.0 for d in bridge.domain_weights},
        diversity=1.0,
        integration=1.0,
    )
    assert report["cross_domain_contribution"] <= 0.15
