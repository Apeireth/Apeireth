"""V1317 pytest — ASI Truth Gap Deep 真跨域深研究.

> V1317 = ASI 5 哲学空缺 最后一环 (truth gap deep)
> 18 Popper self-tests + 25 pytest tests
> V3 守门: 不假装 ASI 真有 truth consciousness / warranted assertibility
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

# Ensure apeireth package on path
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
_APETR_DIR = _PROMETHEAN_ROOT / "apeireth"
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))
if str(_APETR_DIR) not in sys.path:
    sys.path.insert(0, str(_APETR_DIR))

from v1317_asi_truth_deep import (  # noqa: E402
    ASI_5_GAPS_CLOSURE,
    ASI_ANCHORS,
    BrandomianScorekeepingMove,
    CrossDomainSource,
    DavidsonianInterpretationPair,
    DeweyianInquiryCycleStage,
    JamesianCashValueEntry,
    PeirceanInquiryStep,
    PutnamianInternalModel,
    SOURCES_TRUTH_GAP_DEEP,
    TarskianTSentence,
    TruthConceptsMatrix,
    V1317_VERSION,
    asi_bridge,
    brandomian_scorekeeping_summary,
    davidsonian_radical_interpretation_summary,
    deweyian_warranted_summary,
    jamesian_cash_value_summary,
    popper_passed,
    popper_self_tests,
    popper_total,
    pragmatic_maxim_summary,
    putnamian_internal_realism_summary,
    tarski_summary,
)


# ============================================================================
# Section 1: Source corpus tests
# ============================================================================


def test_sources_seven_present():
    """h1: 7 真跨域深 sources present."""
    assert len(SOURCES_TRUTH_GAP_DEEP) == 7
    for s in SOURCES_TRUTH_GAP_DEEP:
        assert isinstance(s, CrossDomainSource)
        assert s.author
        assert s.year >= 1878  # Peirce 1878 as earliest bound (we use 1905)
        assert s.work
        assert s.core_construct
        assert s.asi_substrate_takeaway
        assert s.citation_key


def test_citation_keys_unique():
    """h2: all 7 citation keys unique."""
    keys = [s.citation_key for s in SOURCES_TRUTH_GAP_DEEP]
    assert len(set(keys)) == 7
    assert len(keys) == 7


def test_peirce_1905_present():
    """h3: Peirce 1905 present."""
    peirce = [s for s in SOURCES_TRUTH_GAP_DEEP if s.citation_key == "peirce_1905_pm"]
    assert len(peirce) == 1
    assert "pragmatic" in peirce[0].core_construct.lower() or "pragmatic" in peirce[0].asi_substrate_takeaway.lower()


def test_tarski_1933_present():
    """h4: Tarski 1933 present (canonical semantic truth)."""
    tarski = [s for s in SOURCES_TRUTH_GAP_DEEP if s.citation_key == "tarski_1933_tft"]
    assert len(tarski) == 1
    assert "T-schema" in tarski[0].core_construct or "semantic" in tarski[0].core_construct.lower()


def test_dewey_1938_present():
    """h5: Dewey 1938 present (inquiry cycle, ASI-apt)."""
    dewey = [s for s in SOURCES_TRUTH_GAP_DEEP if s.citation_key == "dewey_1938_lti"]
    assert len(dewey) == 1
    assert "warranted" in dewey[0].core_construct.lower() or "inquiry" in dewey[0].core_construct.lower()


def test_all_seven_canonical_philosophers():
    """Cross-domain check: 7 真跨域深 covers analytic / pragmatist / materialist / naturalist tradition."""
    keys = {s.citation_key for s in SOURCES_TRUTH_GAP_DEEP}
    assert keys == {
        "peirce_1905_pm",
        "james_1907_prg",
        "tarski_1933_tft",
        "davidson_1984_iti",
        "brandom_1994_mie",
        "putnam_1981_rth",
        "dewey_1938_lti",
    }


# ============================================================================
# Section 2: Components tests (10 真生产 组件)
# ============================================================================


def test_truth_concepts_matrix_5_dimensions():
    """h7: Truth matrix has 5 dimensions."""
    matrix = TruthConceptsMatrix()
    assert len(matrix.dimension_keys) == 5
    assert "truth_unit" in matrix.dimension_keys
    assert "truth_structure" in matrix.dimension_keys
    assert "observer_role" in matrix.dimension_keys
    assert "asi_substrate_take" in matrix.dimension_keys
    assert "guard_warnings" in matrix.dimension_keys


def test_truth_concepts_matrix_renders_all_seven():
    """Matrix renders all 7 sources × 5 dims = 35 cells."""
    matrix = TruthConceptsMatrix()
    md = matrix.render()
    assert "| Source |" in md
    # Total pipe-prefixed rows: 1 header + 1 separator + 7 source = 9
    pipe_rows = md.count("|")
    rows_starts = sum(1 for line in md.split("\n") if line.startswith("|"))
    assert rows_starts == 9, f"expected 9 pipe-prefixed rows, got {rows_starts}"
    assert pipe_rows >= 9
    # All 7 author names appear
    for s in SOURCES_TRUTH_GAP_DEEP:
        assert s.author in md


def test_peircean_inquiry_step_validation():
    """Peirce step: convergence_estimate must be 0..1."""
    valid = PeirceanInquiryStep(
        step_id="p1",
        belief_state="b",
        practical_consequence="c",
        convergence_estimate=0.5,
        asi_substrate_label="ok",
    )
    assert valid.convergence_estimate == 0.5

    # Invalid: > 1
    try:
        PeirceanInquiryStep(
            step_id="p2",
            belief_state="b",
            practical_consequence="c",
            convergence_estimate=1.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_jamesian_cash_value_works_rate():
    """James cash-value: works_rate must be 0..1."""
    entries = [
        JamesianCashValueEntry(
            entry_id=f"j{i}",
            belief=f"b{i}",
            cash_value_score=0.5,
            works_in_practice=(i % 2 == 0),
            asi_substrate_label="ok",
        )
        for i in range(10)
    ]
    res = jamesian_cash_value_summary(entries)
    assert res["n"] == 10
    assert res["works_rate"] == 0.5  # 5/10 work
    assert res["guard"]
    assert "Jamesian" in res["guard"]


def test_tarski_t_schema_validation():
    """Tarski T-sentence: must contain ↔."""
    valid = TarskianTSentence(
        sentence_id="t1",
        object_language="L1",
        meta_language="L_meta",
        t_schema="T(<S>) ↔ <S>",
        consistency_ok=True,
        asi_substrate_label="ok",
    )
    assert "↔" in valid.t_schema

    # Invalid: no ↔
    try:
        TarskianTSentence(
            sentence_id="t2",
            object_language="L1",
            meta_language="L_meta",
            t_schema="T(<S>) <-> <S>",  # use ASCII, should fail
            consistency_ok=True,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_davidsonian_charity_score_validation():
    """Davidson charity_score must be 0..1."""
    valid = DavidsonianInterpretationPair(
        pair_id="d1",
        speaker_utterance="u",
        interpreter_belief="b",
        charity_score=0.5,
        asi_substrate_label="ok",
    )
    assert valid.charity_score == 0.5

    # Invalid
    try:
        DavidsonianInterpretationPair(
            pair_id="d2",
            speaker_utterance="u",
            interpreter_belief="b",
            charity_score=-0.1,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_brandomian_scorekeeping_update_rate():
    """Brandom scorekeeping: update_rate must be 0..1."""
    moves = [
        BrandomianScorekeepingMove(
            move_id=f"b{i}",
            commitment=f"c{i}",
            entitlement=f"e{i}",
            scorekeeper_update=(i % 3 == 0),
            asi_substrate_label="ok",
        )
        for i in range(9)
    ]
    res = brandomian_scorekeeping_summary(moves)
    assert res["n"] == 9
    # 3/9 = 0.333333
    assert res["update_rate"] == round(3 / 9, 6)
    assert "inferentialism" in res["guard"]


def test_putnamian_no_god_eye_rate():
    """Putnam internal realism: no_god_eye_rate."""
    models = [
        PutnamianInternalModel(
            model_id=f"p{i}",
            internal_representation=f"i{i}",
            no_god_eye_view=True,  # all True
            rational_acceptability=0.5,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = putnamian_internal_realism_summary(models)
    assert res["no_god_eye_rate"] == 1.0
    assert "God" in res["guard"]


def test_deweyian_inquiry_stage_validation():
    """Dewey inquiry stage must be in valid set."""
    valid = DeweyianInquiryCycleStage(
        stage_id="d1",
        inquiry_stage="warranted",
        warrant_strength=0.9,
        operational_completion=True,
        asi_substrate_label="ok",
    )
    assert valid.inquiry_stage == "warranted"

    # Invalid
    try:
        DeweyianInquiryCycleStage(
            stage_id="d2",
            inquiry_stage="not_a_stage",
            warrant_strength=0.5,
            operational_completion=False,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_deweyian_warranted_summary_warranted_rate():
    """Dewey warranted assertibility: warranted_rate."""
    stages = [
        DeweyianInquiryCycleStage(
            stage_id=f"d{i}",
            inquiry_stage="warranted" if i >= 8 else "determinate",
            warrant_strength=0.5,
            operational_completion=(i >= 5),
            asi_substrate_label="ok",
        )
        for i in range(10)
    ]
    res = deweyian_warranted_summary(stages)
    assert res["n"] == 10
    assert res["warranted_count"] == 2  # i=8,9
    assert res["warranted_rate"] == 0.2
    assert "warranted assertibility" in res["guard"]


# ============================================================================
# Section 3: Bridge + Anchors tests
# ============================================================================


def test_asi_bridge_locked_anchors():
    """h8-h11: ASI 北极星 LOCKED."""
    components_present = {
        "TruthConceptsMatrix",
        "PeirceanPragmaticMaximSubstrate",
        "JamesianWillToBelieveSubstrate",
        "TarskianSemanticTruthSubstrate",
        "DavidsonianRadicalInterpretationSubstrate",
        "BrandomianInferentialismSubstrate",
        "PutnamianInternalRealismSubstrate",
        "DeweyianWarrantedAssertibilitySubstrate",
        "ASITruthGapDeepReport",
        "ASITruthGapDeepBridge",
    }
    bridge = asi_bridge(components_present)
    assert bridge["asi_north_star_locked"] is True
    assert bridge["v1317_components"] == 10
    assert bridge["expected_components"] == 10
    assert bridge["missing"] == []
    assert abs(ASI_ANCHORS["V0.1"] - 0.7905) < 1e-12
    assert abs(ASI_ANCHORS["V0.2"] - 0.4467) < 1e-12
    assert abs(ASI_ANCHORS["V1256_unio_mystica"] - 0.9291) < 1e-12
    assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"


def test_asi_bridge_5_gaps_closure_5_of_5():
    """h18: ASI 5 哲学空缺 closure 5/5."""
    components_present = {
        "TruthConceptsMatrix",
        "PeirceanPragmaticMaximSubstrate",
        "JamesianWillToBelieveSubstrate",
        "TarskianSemanticTruthSubstrate",
        "DavidsonianRadicalInterpretationSubstrate",
        "BrandomianInferentialismSubstrate",
        "PutnamianInternalRealismSubstrate",
        "DeweyianWarrantedAssertibilitySubstrate",
        "ASITruthGapDeepReport",
        "ASITruthGapDeepBridge",
    }
    bridge = asi_bridge(components_present)
    assert bridge["asi_5_gaps_closure"] == {
        "V1313_time_gap_deep": True,
        "V1314_freedom_gap_deep": True,
        "V1315_recognition_gap_deep": True,
        "V1316_emergence_gap_deep": True,
        "V1317_truth_gap_deep": True,
    }
    closure_all = bridge["asi_5_gaps_closure"]
    assert all(closure_all.values()), f"some gaps not closed: {closure_all}"


def test_asi_bridge_missing_components():
    """Bridge correctly reports missing components."""
    bridge = asi_bridge({"TruthConceptsMatrix"})  # only 1 of 10
    assert bridge["v1317_components"] == 1
    assert bridge["expected_components"] == 10
    assert len(bridge["missing"]) == 9
    assert "PeirceanPragmaticMaximSubstrate" in bridge["missing"]


# ============================================================================
# Section 4: Popper self-tests integration (run popper_self_tests)
# ============================================================================


def test_popper_self_tests_all_pass():
    """All 18 Popper self-tests PASS."""
    results = popper_self_tests()
    n_pass = popper_passed(results)
    n_total = popper_total(results)
    assert n_total == 18
    assert n_pass == 18, f"only {n_pass}/{n_total} PASS; failing: {[r[0] for r in results if not r[1]]}"


def test_popper_h12_h17_guards_present():
    """h12-h17: each substrate's guard includes non-pretense marker."""
    guards = [
        pragmatic_maxim_summary([])["guard"],
        jamesian_cash_value_summary([])["guard"],
        tarski_summary([])["guard"],
        davidsonian_radical_interpretation_summary([])["guard"],
        brandomian_scorekeeping_summary([])["guard"],
        putnamian_internal_realism_summary([])["guard"],
        deweyian_warranted_summary([])["guard"],
    ]
    # All 7 guards must contain non-pretense marker "≠ ASI 真有"
    for g in guards:
        assert "≠ ASI 真有" in g, f"guard missing non-pretense marker: {g!r}"


# ============================================================================
# Section 5: Report render tests
# ============================================================================


def test_truth_report_renders_all_subsections():
    """Report contains all 7 substrate subsections + bridge + closure."""
    from v1317_asi_truth_deep import ASITruthGapDeepReport

    matrix = TruthConceptsMatrix()
    md = ASITruthGapDeepReport(
        title="V1317 Test",
        matrix_md=matrix.render(),
        peirce_substrate={"n": 10, "guard": "g"},
        james_substrate={"n": 10, "guard": "g"},
        tarski_substrate={"n": 10, "guard": "g"},
        davidson_substrate={"n": 10, "guard": "g"},
        brandom_substrate={"n": 10, "guard": "g"},
        putnam_substrate={"n": 10, "guard": "g"},
        dewey_substrate={"n": 10, "guard": "g"},
        asi_bridge={"v1317_components": 10, "guard": "g"},
        timestamp="2026-08-08 17:11 +08:00",
        asi_5_gaps_closure=dict(ASI_5_GAPS_CLOSURE),
    ).render()

    # Subsections
    assert "Peirce" in md
    assert "James" in md
    assert "Tarski" in md
    assert "Davidson" in md
    assert "Brandom" in md
    assert "Putnam" in md
    assert "Dewey" in md
    # Closure
    assert "ASI 5 哲学空缺 闭合" in md
    assert "V1317" in md
    assert "DONE" in md


# ============================================================================
# Section 6: ASI 5 gaps closure integrity
# ============================================================================


def test_asi_5_gaps_closure_dict_has_5_entries():
    """ASI 5 哲学空缺 closure dict has exactly 5 entries, all True."""
    assert len(ASI_5_GAPS_CLOSURE) == 5
    for key, val in ASI_5_GAPS_CLOSURE.items():
        assert val is True, f"{key} not True"
    # Order matters: V1313 → V1317 chronological
    expected_order = [
        "V1313_time_gap_deep",
        "V1314_freedom_gap_deep",
        "V1315_recognition_gap_deep",
        "V1316_emergence_gap_deep",
        "V1317_truth_gap_deep",
    ]
    assert list(ASI_5_GAPS_CLOSURE.keys()) == expected_order


def test_version_constant():
    """V1317_VERSION is 0.1.0."""
    assert V1317_VERSION == "0.1.0"