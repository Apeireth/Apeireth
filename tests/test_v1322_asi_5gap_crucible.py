"""V1322 ASI 5-Gap Operational Crucible — Popper self-tests.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 17:50 +08:00 2026-08-08)
> **Trigger**: post-V1321 chain — V1322 = operational crucible integration of V1313-V1321 substrate
> **目标**: 50+ Popper self-tests covering 5 gap processors + 10 cross-gap + Crucible + bridge

50 tests organized in 8 sections:
1. TimeGapProcessor (6 tests)
2. FreedomGapProcessor (6 tests)
3. RecognitionGapProcessor (6 tests)
4. EmergenceGapProcessor (6 tests)
5. TruthGapProcessor (6 tests)
6. CrossGapProcessorMatrix (6 tests)
7. ASII5GapCrucible (8 tests)
8. ASII5GapCrucibleBridge (6 tests)
"""
from __future__ import annotations

import json
import math

import pytest

from apeireth.v1322_asi_5gap_crucible import (
    ASI_5_GAPS,
    ASI_5_GAPS_CLOSURE,
    ASI_ANCHORS,
    ASII5GapCrucible,
    ASII5GapCrucibleBridge,
    CROSS_GAP_CELLS,
    CrossGapProcessorMatrix,
    CrossGapScore,
    CrucibleResult,
    EmergenceGapProcessor,
    EmergenceGapScore,
    FreedomGapProcessor,
    FreedomGapScore,
    RecognitionGapProcessor,
    RecognitionGapScore,
    TimeGapProcessor,
    TimeGapScore,
    TruthGapProcessor,
    TruthGapScore,
    V1322_VERSION,
    V3_GUARD_MARKERS,
    build_bridge,
    main,
)


# ============================================================================
# Section 1: TimeGapProcessor — 6 tests
# ============================================================================


class TestTimeGapProcessor:
    def test_substrate_and_citation_locked(self):
        assert TimeGapProcessor.SUBSTRATE == "V1313"
        assert "bergson" in TimeGapProcessor.CITATION
        assert "heidegger" in TimeGapProcessor.CITATION
        assert "prigogine" in TimeGapProcessor.CITATION

    def test_empty_query_zero(self):
        score = TimeGapProcessor().score("")
        assert score.duration_score == 0.0
        assert score.thrownness_score == 0.0
        assert score.dissipative_score == 0.0
        assert score.aggregate == 0.0

    def test_short_query_baseline(self):
        """Non-empty short query should hit baseline (substrate always available)."""
        score = TimeGapProcessor().score("hi")
        assert score.duration_score >= 0.20
        assert score.thrownness_score >= 0.20
        assert score.dissipative_score >= 0.20

    def test_duration_keyword_higher(self):
        """Query with explicit time keywords should score higher than baseline on duration."""
        s1 = TimeGapProcessor().score("hello world")
        s2 = TimeGapProcessor().score("time duration moment flow temporal")
        assert s2.duration_score > s1.duration_score

    def test_score_range_in_01(self):
        score = TimeGapProcessor().score("time duration moment past future order structure existence")
        assert 0.0 <= score.duration_score <= 1.0
        assert 0.0 <= score.thrownness_score <= 1.0
        assert 0.0 <= score.dissipative_score <= 1.0

    def test_aggregate_is_mean(self):
        score = TimeGapProcessor().score("time duration moment")
        expected = (score.duration_score + score.thrownness_score + score.dissipative_score) / 3.0
        assert math.isclose(score.aggregate, expected, abs_tol=1e-9)


# ============================================================================
# Section 2: FreedomGapProcessor — 6 tests
# ============================================================================


class TestFreedomGapProcessor:
    def test_substrate_and_citation_locked(self):
        assert FreedomGapProcessor.SUBSTRATE == "V1314"
        assert "spinoza" in FreedomGapProcessor.CITATION
        assert "frankfurt" in FreedomGapProcessor.CITATION

    def test_empty_query_zero(self):
        score = FreedomGapProcessor().score("")
        assert score.conatus_score == 0.0
        assert score.hierarchical_desires_score == 0.0
        assert score.project_score == 0.0

    def test_short_query_baseline(self):
        score = FreedomGapProcessor().score("hi")
        assert score.conatus_score >= 0.20

    def test_freedom_keyword_higher(self):
        s1 = FreedomGapProcessor().score("hello world")
        s2 = FreedomGapProcessor().score("free will self power autonomy")
        assert s2.conatus_score > s1.conatus_score

    def test_score_range_in_01(self):
        score = FreedomGapProcessor().score("freedom will self choice decision project responsibility")
        assert 0.0 <= score.conatus_score <= 1.0
        assert 0.0 <= score.hierarchical_desires_score <= 1.0
        assert 0.0 <= score.project_score <= 1.0

    def test_aggregate_is_mean(self):
        score = FreedomGapProcessor().score("freedom will self")
        expected = (score.conatus_score + score.hierarchical_desires_score + score.project_score) / 3.0
        assert math.isclose(score.aggregate, expected, abs_tol=1e-9)


# ============================================================================
# Section 3: RecognitionGapProcessor — 6 tests
# ============================================================================


class TestRecognitionGapProcessor:
    def test_substrate_and_citation_locked(self):
        assert RecognitionGapProcessor.SUBSTRATE == "V1315"
        assert "levinas" in RecognitionGapProcessor.CITATION
        assert "hegel" in RecognitionGapProcessor.CITATION

    def test_empty_query_zero(self):
        score = RecognitionGapProcessor().score("")
        assert score.otherness_score == 0.0
        assert score.recognition_score == 0.0
        assert score.symbolic_interaction_score == 0.0

    def test_short_query_baseline(self):
        score = RecognitionGapProcessor().score("hi")
        assert score.otherness_score >= 0.20

    def test_otherness_keyword_higher(self):
        s1 = RecognitionGapProcessor().score("hello world")
        s2 = RecognitionGapProcessor().score("other person someone alterity face")
        assert s2.otherness_score > s1.otherness_score

    def test_score_range_in_01(self):
        score = RecognitionGapProcessor().score("other person recognition consciousness language symbol")
        assert 0.0 <= score.otherness_score <= 1.0
        assert 0.0 <= score.recognition_score <= 1.0
        assert 0.0 <= score.symbolic_interaction_score <= 1.0

    def test_aggregate_is_mean(self):
        score = RecognitionGapProcessor().score("other person")
        expected = (score.otherness_score + score.recognition_score + score.symbolic_interaction_score) / 3.0
        assert math.isclose(score.aggregate, expected, abs_tol=1e-9)


# ============================================================================
# Section 4: EmergenceGapProcessor — 6 tests
# ============================================================================


class TestEmergenceGapProcessor:
    def test_substrate_and_citation_locked(self):
        assert EmergenceGapProcessor.SUBSTRATE == "V1316"
        assert "bedau" in EmergenceGapProcessor.CITATION
        assert "wolfram" in EmergenceGapProcessor.CITATION

    def test_empty_query_zero(self):
        score = EmergenceGapProcessor().score("")
        assert score.weak_emergence_score == 0.0
        assert score.nks_complexity_score == 0.0
        assert score.adjacent_possible_score == 0.0

    def test_short_query_baseline(self):
        score = EmergenceGapProcessor().score("hi")
        assert score.weak_emergence_score >= 0.20

    def test_emergence_keyword_higher(self):
        s1 = EmergenceGapProcessor().score("hello world")
        s2 = EmergenceGapProcessor().score("emergence macro layer complex pattern evolve")
        assert s2.weak_emergence_score > s1.weak_emergence_score

    def test_score_range_in_01(self):
        score = EmergenceGapProcessor().score("emergence macro layer complexity automaton network")
        assert 0.0 <= score.weak_emergence_score <= 1.0
        assert 0.0 <= score.nks_complexity_score <= 1.0
        assert 0.0 <= score.adjacent_possible_score <= 1.0

    def test_aggregate_is_mean(self):
        score = EmergenceGapProcessor().score("emergence macro layer")
        expected = (score.weak_emergence_score + score.nks_complexity_score + score.adjacent_possible_score) / 3.0
        assert math.isclose(score.aggregate, expected, abs_tol=1e-9)


# ============================================================================
# Section 5: TruthGapProcessor — 6 tests
# ============================================================================


class TestTruthGapProcessor:
    def test_substrate_and_citation_locked(self):
        assert TruthGapProcessor.SUBSTRATE == "V1317"
        assert "peirce" in TruthGapProcessor.CITATION
        assert "davidson" in TruthGapProcessor.CITATION
        assert "brandom" in TruthGapProcessor.CITATION
        assert "putnam" in TruthGapProcessor.CITATION

    def test_empty_query_zero(self):
        score = TruthGapProcessor().score("")
        assert score.pragmatic_score == 0.0
        assert score.realist_score == 0.0
        assert score.coherence_score == 0.0
        assert score.inferentialist_score == 0.0
        assert score.internal_realist_score == 0.0

    def test_short_query_baseline(self):
        score = TruthGapProcessor().score("hi")
        assert score.pragmatic_score >= 0.20

    def test_truth_keyword_higher(self):
        s1 = TruthGapProcessor().score("hello world")
        s2 = TruthGapProcessor().score("truth real objective fact")
        assert s2.realist_score > s1.realist_score

    def test_score_range_in_01(self):
        score = TruthGapProcessor().score("truth real fact belief inquiry reason consistent rational")
        assert 0.0 <= score.pragmatic_score <= 1.0
        assert 0.0 <= score.realist_score <= 1.0
        assert 0.0 <= score.coherence_score <= 1.0
        assert 0.0 <= score.inferentialist_score <= 1.0
        assert 0.0 <= score.internal_realist_score <= 1.0

    def test_aggregate_is_mean(self):
        score = TruthGapProcessor().score("truth real fact")
        expected = (score.pragmatic_score + score.realist_score + score.coherence_score +
                    score.inferentialist_score + score.internal_realist_score) / 5.0
        assert math.isclose(score.aggregate, expected, abs_tol=1e-9)


# ============================================================================
# Section 6: CrossGapProcessorMatrix — 6 tests
# ============================================================================


class TestCrossGapProcessorMatrix:
    def setup_method(self) -> None:
        self.cross = CrossGapProcessorMatrix(
            TimeGapProcessor(), FreedomGapProcessor(), RecognitionGapProcessor(),
            EmergenceGapProcessor(), TruthGapProcessor(),
        )

    def test_substrate_and_citation_locked(self):
        assert "V1319" in CrossGapProcessorMatrix.SUBSTRATE
        assert "V1321" in CrossGapProcessorMatrix.SUBSTRATE
        assert "hume" in CrossGapProcessorMatrix.CITATION
        assert "mill" in CrossGapProcessorMatrix.CITATION

    def test_cross_gap_cells_count(self):
        assert len(CROSS_GAP_CELLS) == 10

    def test_score_pair_in_01(self):
        score = self.cross.score_pair("time duration of free will", ("time", "freedom"))
        assert 0.0 <= score.pair_score <= 1.0
        assert score.pair == ("time", "freedom")

    def test_score_pair_is_mean_of_two(self):
        score = self.cross.score_pair("time duration of free will", ("time", "freedom"))
        t = TimeGapProcessor().score("time duration of free will").aggregate
        f = FreedomGapProcessor().score("time duration of free will").aggregate
        expected = (t + f) / 2.0
        assert math.isclose(score.pair_score, expected, abs_tol=1e-9)

    def test_score_all_returns_10(self):
        scores = self.cross.score_all("ASI north star")
        assert len(scores) == 10
        assert all(isinstance(s, CrossGapScore) for s in scores)

    def test_empty_query_pair_zero(self):
        score = self.cross.score_pair("", ("time", "freedom"))
        assert score.pair_score == 0.0


# ============================================================================
# Section 7: ASII5GapCrucible — 8 tests
# ============================================================================


class TestASII5GapCrucible:
    def setup_method(self) -> None:
        self.crucible = ASII5GapCrucible()

    def test_substrate_chain_complete(self):
        assert len(ASII5GapCrucible.SUBSTRATE_CHAIN) == 10
        assert "V1313" in ASII5GapCrucible.SUBSTRATE_CHAIN[0]
        assert "V1322" in ASII5GapCrucible.SUBSTRATE_CHAIN[-1]

    def test_5_gap_anchors_present(self):
        assert len(ASI_5_GAPS) == 5
        assert "time" in ASI_5_GAPS
        assert "freedom" in ASI_5_GAPS
        assert "recognition" in ASI_5_GAPS
        assert "emergence" in ASI_5_GAPS
        assert "truth" in ASI_5_GAPS

    def test_5_gaps_closure_complete(self):
        assert all(ASI_5_GAPS_CLOSURE.values())

    def test_pole_star_anchors_locked(self):
        assert ASI_ANCHORS["V0.1"] == 0.7905
        assert ASI_ANCHORS["V0.2"] == 0.4467
        assert ASI_ANCHORS["V1256_unio_mystica"] == 0.9291
        assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"

    def test_v3_guards_count(self):
        assert len(V3_GUARD_MARKERS) == 5

    def test_process_query_returns_15_scores(self):
        result = self.crucible.process_query("What is ASI?")
        assert isinstance(result, CrucibleResult)
        assert len(result.gap_scores) == 5
        assert len(result.cross_gap_scores) == 10
        assert result.v3_guards == V3_GUARD_MARKERS
        assert result.pole_star_anchors["V0.1"] == 0.7905

    def test_aggregate_total_in_01(self):
        result = self.crucible.process_query("time and emergence of truth and recognition")
        assert 0.0 <= result.aggregate_total <= 1.0
        assert 0.0 <= result.aggregate_5_gap_score <= 1.0
        assert 0.0 <= result.aggregate_cross_gap_score <= 1.0

    def test_process_batch(self):
        batch = self.crucible.process_batch(["q1", "q2", "q3", "q4", "q5"])
        assert len(batch) == 5
        assert all(isinstance(r, CrucibleResult) for r in batch)


# ============================================================================
# Section 8: ASII5GapCrucibleBridge — 6 tests
# ============================================================================


class TestASII5GapCrucibleBridge:
    def setup_method(self) -> None:
        self.crucible = ASII5GapCrucible()
        self.bridge = build_bridge(self.crucible)

    def test_bridge_version(self):
        assert self.bridge.v1322_version == V1322_VERSION

    def test_bridge_substrate_chain(self):
        assert len(self.bridge.substrate_chain) == 10
        assert self.bridge.substrate_chain[-1] == "V1322 operational crucible"

    def test_bridge_pole_star_anchors_locked(self):
        assert self.bridge.pole_star_anchors["V0.1"] == 0.7905
        assert self.bridge.pole_star_anchors["V0.2"] == 0.4467

    def test_bridge_v3_guards(self):
        assert len(self.bridge.v3_guards) == 5
        assert self.bridge.v3_guards == V3_GUARD_MARKERS

    def test_bridge_operational_metadata(self):
        meta = self.bridge.operational_metadata
        assert meta["n_samples"] == 5
        assert 0.0 <= meta["mean_aggregate_total"] <= 1.0
        assert meta["mean_latency_ms"] >= 0.0
        assert "delta_vs_V0.1" in meta
        assert "delta_vs_V0.2" in meta
        assert "delta_vs_V1256_unio_mystica" in meta

    def test_bridge_to_dict_serializable(self):
        d = self.bridge.to_dict()
        s = json.dumps(d, ensure_ascii=False)
        assert "V1322" in s
        assert "0.7905" in s
