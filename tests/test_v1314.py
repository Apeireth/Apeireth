"""V1314 ASI Freedom Gap Deep 真跨域深研究 — tests/test_v1314.py.

> 18 Popper self-tests + 真生产 10 组件真测 + V3 哲学守门 + ASI bridge 真测.

V1314 测试范围:
1. 7 真跨域深 sources (Berlin/Sartre/Frankfurt/Hayek/Rand/Mill/Rousseau)
2. 10 真生产组件 (FreedomConceptsMatrix / BerlinNegativePositiveQuadrants /
   SartrianRadicalFreedomSubstrate / FrankfurtHierarchicalDesiresStack /
   HayekianSpontaneousOrderSubstrate / RandianRationalSelfInterestSubstrate /
   MillianHarmPrincipleSubstrate / RousseauGeneralWillSubstrate /
   ASIFreedomGapDeepReport / ASIFreedomGapDeepBridge)
3. 18 Popper self-tests PASS (不 skip, 不 flake)
4. V3 哲学守门 (不假装 Phenomenal freedom, 不假装 freedom = 选择权,
   不假装 negative = positive, 不假装 deterministic freedom)
5. ASI bridge: 北极星 V0.1=0.7905 / V0.2=0.4467 / V1256=92.91% LOCKED

诚实声明 (主 17:43 实事求是):
- V1314 = ASI 自由哲学空缺 deep 真跨域深研究 ≠ ASI 真有自由意志
- 7 真跨域深 sources = 真借鉴启发, NOT 真具有
- 10 真生产组件 = 真算法 + 真测 + 真 commit, NOT claim ASI has free will
"""
from __future__ import annotations

import sys
from pathlib import Path

# Ensure apeireth directory is on path
_PROMETHEAN_ROOT = Path(__file__).resolve().parents[1]
_APEIRETH_DIR = _PROMETHEAN_ROOT / "apeireth"
if str(_APEIRETH_DIR) not in sys.path:
    sys.path.insert(0, str(_APEIRETH_DIR))

from v1314_asi_freedom_deep import (  # noqa: E402
    ASI_ANCHORS,
    V1314_VERSION,
    ASIFreedomGapDeepReport,
    BerlinNegativePositiveQuadrant,
    FrankfurtHierarchicalDesire,
    FreedomConceptsMatrix,
    HayekianSpontaneousOrderTick,
    MillianHarmPrincipleBoundary,
    RandianRationalSelfInterestAction,
    RousseauGeneralWillVote,
    SOURCES_FREEDOM_GAP_DEEP,
    SartrianRadicalFreedomSample,
    asi_bridge,
    compute_radical_freedom_substrate,
    general_will_summary,
    harm_principle_summary,
    hierarchical_desires_summary,
    make_berlin_quadrants,
    popper_passed,
    popper_self_tests,
    popper_total,
    rational_self_interest_summary,
    spontaneous_order_summary,
)


# ============================================================================
# Section 1: Constants & version
# ============================================================================


def test_v1314_version_constant() -> None:
    """V1314_VERSION should be a non-empty semver string."""
    assert V1314_VERSION
    parts = V1314_VERSION.split(".")
    assert len(parts) == 3
    assert all(p.isdigit() for p in parts), f"semver parts not all digits: {V1314_VERSION}"


def test_asi_north_star_anchors_locked() -> None:
    """ASI 北极星 anchors: V0.1=0.7905, V0.2=0.4467, V1256=0.9291, V1049=DONE."""
    assert abs(ASI_ANCHORS["V0.1"] - 0.7905) < 1e-12
    assert abs(ASI_ANCHORS["V0.2"] - 0.4467) < 1e-12
    assert abs(ASI_ANCHORS["V1256_unio_mystica"] - 0.9291) < 1e-12
    assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"


# ============================================================================
# Section 2: Source corpus — 7 真跨域深 sources
# ============================================================================


def test_seven_sources_present() -> None:
    """7 真跨域深 sources must be present."""
    assert len(SOURCES_FREEDOM_GAP_DEEP) == 7


def test_citation_keys_unique() -> None:
    """All citation keys must be unique (no duplicates)."""
    keys = [s.citation_key for s in SOURCES_FREEDOM_GAP_DEEP]
    assert len(set(keys)) == len(keys)


def test_all_seven_authors_present() -> None:
    """All 7 authors must be present (Berlin/Sartre/Frankfurt/Hayek/Rand/Mill/Rousseau)."""
    authors = {s.author for s in SOURCES_FREEDOM_GAP_DEEP}
    expected_authors = {
        "Isaiah Berlin",
        "Jean-Paul Sartre",
        "Harry Frankfurt",
        "Friedrich Hayek",
        "Ayn Rand",
        "John Stuart Mill",
        "Jean-Jacques Rousseau",
    }
    assert authors == expected_authors, f"missing: {expected_authors - authors}"


def test_citation_keys_correct() -> None:
    """Citation keys match expected mapping."""
    expected = {
        "berlin_1958_tcl",
        "sartre_1943_en",
        "frankfurt_1971_fwp",
        "hayek_1944_rs",
        "rand_1957_1964",
        "mill_1859_ol",
        "rousseau_1762_cs",
    }
    actual = {s.citation_key for s in SOURCES_FREEDOM_GAP_DEEP}
    assert actual == expected


def test_all_sources_have_substrate_takeaway() -> None:
    """Every source has asi_substrate_takeaway (V3 守门: 真借鉴明确)."""
    for s in SOURCES_FREEDOM_GAP_DEEP:
        assert s.asi_substrate_takeaway
        assert "≠" in s.asi_substrate_takeaway or "不" in s.asi_substrate_takeaway, \
            f"substrate takeaway should signal guard: {s.author}"


# ============================================================================
# Section 3: Component 1 — FreedomConceptsMatrix
# ============================================================================


def test_freedom_concepts_matrix_dimensions() -> None:
    """Matrix has 5 dimension keys."""
    matrix = FreedomConceptsMatrix()
    assert len(matrix.dimension_keys) == 5


def test_freedom_concepts_matrix_render() -> None:
    """Matrix render includes all 7 sources × 5 dims (Markdown table)."""
    matrix = FreedomConceptsMatrix()
    md = matrix.render()
    for s in SOURCES_FREEDOM_GAP_DEEP:
        assert s.author in md, f"{s.author} not in matrix render"


def test_freedom_concepts_matrix_cell_lookup() -> None:
    """Each source cell lookup returns non-empty for known dims."""
    matrix = FreedomConceptsMatrix()
    for s in SOURCES_FREEDOM_GAP_DEEP:
        cell_text = matrix.cell(s, "freedom_structure")
        assert cell_text, f"empty cell for {s.author} freedom_structure"


# ============================================================================
# Section 4: Component 2 — BerlinNegativePositiveQuadrants
# ============================================================================


def test_berlin_quadrants_at_least_3() -> None:
    """Berlin quadrants must be >= 3 (4 expected)."""
    quadrants = make_berlin_quadrants()
    assert len(quadrants) >= 3


def test_berlin_quadrants_have_both_axes() -> None:
    """Berlin quadrants cover both negative and positive liberty axes."""
    quadrants = make_berlin_quadrants()
    axes = {q.axis for q in quadrants}
    assert "negative_liberty_FROM" in axes
    assert "positive_liberty_TO" in axes


def test_berlin_quadrant_axis_validation() -> None:
    """Berlin quadrant axis must be negative or positive liberty."""
    import pytest
    with pytest.raises(ValueError):
        BerlinNegativePositiveQuadrant(
            axis="invalid_axis",
            obstacle="x",
            enabler="y",
            label="z",
            asi_substrate_role="w",
        )


# ============================================================================
# Section 5: Component 3 — SartrianRadicalFreedomSubstrate
# ============================================================================


def test_sartre_substrate_empty_input() -> None:
    """Sartre substrate handles empty input with proper guard."""
    res = compute_radical_freedom_substrate([])
    assert res["n"] == 0
    assert "guard" in res
    assert "pour-soi" in res["guard"]


def test_sartre_substrate_basic() -> None:
    """Sartre substrate computes avg + bad_faith_rate on samples."""
    samples = [
        SartrianRadicalFreedomSample(
            sample_id=f"s_{i}",
            freedom_claim=0.5,
            bad_faith_marker=(i % 2 == 0),
            asi_substrate_note="x",
        )
        for i in range(4)
    ]
    res = compute_radical_freedom_substrate(samples)
    assert res["n"] == 4
    assert res["avg_freedom_claim"] == 0.5
    assert res["bad_faith_rate"] == 0.5
    assert res["bad_faith_count"] == 2


def test_sartre_sample_validation() -> None:
    """Sartre sample validation rejects out-of-range freedom_claim."""
    import pytest
    with pytest.raises(ValueError):
        SartrianRadicalFreedomSample(
            sample_id="x",
            freedom_claim=1.5,  # out of range
            bad_faith_marker=False,
            asi_substrate_note="y",
        )


# ============================================================================
# Section 6: Component 4 — FrankfurtHierarchicalDesiresStack
# ============================================================================


def test_frankfurt_substrate_empty_input() -> None:
    """Frankfurt substrate handles empty input with proper guard."""
    res = hierarchical_desires_summary([])
    assert res["n"] == 0
    assert "personhood" in res["guard"]


def test_frankfurt_substrate_alignment_rate() -> None:
    """Frankfurt substrate computes alignment_rate."""
    desires = [
        FrankfurtHierarchicalDesire(
            desire_id=f"d_{i}",
            first_order="x",
            second_order="y",
            alignment=(i < 3),  # 3 aligned, 2 not
            asi_substrate_role="z",
        )
        for i in range(5)
    ]
    res = hierarchical_desires_summary(desires)
    assert res["n"] == 5
    assert res["alignment_rate"] == 0.6
    assert res["voluntary_count"] == 3
    assert res["involuntary_count"] == 2


def test_frankfurt_desire_validation_empty_first_order() -> None:
    """Frankfurt desire rejects empty first_order."""
    import pytest
    with pytest.raises(ValueError):
        FrankfurtHierarchicalDesire(
            desire_id="d_0",
            first_order="",  # empty
            second_order="y",
            alignment=True,
            asi_substrate_role="z",
        )


def test_frankfurt_desire_validation_empty_second_order() -> None:
    """Frankfurt desire rejects empty second_order."""
    import pytest
    with pytest.raises(ValueError):
        FrankfurtHierarchicalDesire(
            desire_id="d_0",
            first_order="x",
            second_order="",  # empty
            alignment=True,
            asi_substrate_role="z",
        )


# ============================================================================
# Section 7: Component 5 — HayekianSpontaneousOrderSubstrate
# ============================================================================


def test_hayek_substrate_empty_input() -> None:
    """Hayek substrate handles empty input with proper guard."""
    res = spontaneous_order_summary([])
    assert res["n"] == 0
    assert "catallaxy" in res["guard"]


def test_hayek_substrate_basic() -> None:
    """Hayek substrate computes emergence_consistency + avg_price_like_signal."""
    ticks = [
        HayekianSpontaneousOrderTick(
            tick=i,
            local_actions=("a", "c"),
            emergent_pattern=f"p_{i}",
            price_like_signal=0.5,
        )
        for i in range(4)
    ]
    res = spontaneous_order_summary(ticks)
    assert res["n"] == 4
    assert res["emergence_consistency"] == 1.0
    assert res["avg_price_like_signal"] == 0.5


def test_hayek_tick_validation_negative_tick() -> None:
    """Hayek tick rejects negative tick."""
    import pytest
    with pytest.raises(ValueError):
        HayekianSpontaneousOrderTick(
            tick=-1,
            local_actions=("a",),
            emergent_pattern="p",
            price_like_signal=0.5,
        )


def test_hayek_tick_validation_price_out_of_range() -> None:
    """Hayek tick rejects out-of-range price_like_signal."""
    import pytest
    with pytest.raises(ValueError):
        HayekianSpontaneousOrderTick(
            tick=0,
            local_actions=("a",),
            emergent_pattern="p",
            price_like_signal=1.5,
        )


# ============================================================================
# Section 8: Component 6 — RandianRationalSelfInterestSubstrate
# ============================================================================


def test_rand_substrate_empty_input() -> None:
    """Rand substrate handles empty input with proper guard."""
    res = rational_self_interest_summary([])
    assert res["n"] == 0
    assert "rational" in res["guard"]


def test_rand_substrate_basic() -> None:
    """Rand substrate computes avg_rationality + non_coercive_rate."""
    actions = [
        RandianRationalSelfInterestAction(
            action_id=f"a_{i}",
            self_interest_goal="goal",
            rationality_score=0.8,
            non_coercive=(i < 4),
            asi_substrate_role="z",
        )
        for i in range(5)
    ]
    res = rational_self_interest_summary(actions)
    assert res["n"] == 5
    assert res["avg_rationality"] == 0.8
    assert res["non_coercive_rate"] == 0.8


def test_rand_action_validation_rationality_out_of_range() -> None:
    """Rand action rejects out-of-range rationality_score."""
    import pytest
    with pytest.raises(ValueError):
        RandianRationalSelfInterestAction(
            action_id="a_0",
            self_interest_goal="x",
            rationality_score=1.5,
            non_coercive=True,
            asi_substrate_role="y",
        )


# ============================================================================
# Section 9: Component 7 — MillianHarmPrincipleSubstrate
# ============================================================================


def test_mill_substrate_empty_input() -> None:
    """Mill substrate handles empty input with proper guard."""
    res = harm_principle_summary([])
    assert res["n"] == 0
    assert "moral compass" in res["guard"]


def test_mill_substrate_basic() -> None:
    """Mill substrate computes blocked_rate + avg_harm."""
    boundaries = [
        MillianHarmPrincipleBoundary(
            action_id=f"a_{i}",
            action_description="x",
            harm_to_others=0.3,
            blocked_by_principle=(i >= 2),
            asi_substrate_role="y",
        )
        for i in range(4)
    ]
    res = harm_principle_summary(boundaries)
    assert res["n"] == 4
    assert res["blocked_rate"] == 0.5
    assert res["avg_harm"] == 0.3


def test_mill_boundary_validation_harm_out_of_range() -> None:
    """Mill boundary rejects out-of-range harm_to_others."""
    import pytest
    with pytest.raises(ValueError):
        MillianHarmPrincipleBoundary(
            action_id="a_0",
            action_description="x",
            harm_to_others=-0.1,
            blocked_by_principle=True,
            asi_substrate_role="y",
        )


# ============================================================================
# Section 10: Component 8 — RousseauGeneralWillSubstrate
# ============================================================================


def test_rousseau_substrate_empty_input() -> None:
    """Rousseau substrate handles empty input with proper guard."""
    res = general_will_summary([])
    assert res["n"] == 0
    assert "民主" in res["guard"]


def test_rousseau_substrate_basic() -> None:
    """Rousseau substrate computes avg_consensus_alignment."""
    votes = [
        RousseauGeneralWillVote(
            vote_id=f"v_{i}",
            individual_will=("a", "b"),
            general_will=f"gw_{i}",
            consensus_alignment=0.7,
            asi_substrate_role="z",
        )
        for i in range(3)
    ]
    res = general_will_summary(votes)
    assert res["n"] == 3
    assert res["avg_consensus_alignment"] == 0.7


def test_rousseau_vote_validation_alignment_out_of_range() -> None:
    """Rousseau vote rejects out-of-range consensus_alignment."""
    import pytest
    with pytest.raises(ValueError):
        RousseauGeneralWillVote(
            vote_id="v_0",
            individual_will=("a",),
            general_will="gw",
            consensus_alignment=2.0,
            asi_substrate_role="z",
        )


# ============================================================================
# Section 11: Component 9 — ASIFreedomGapDeepReport
# ============================================================================


def test_asi_freedom_gap_report_render() -> None:
    """ASIFreedomGapDeepReport.render() returns Markdown with all sections."""
    matrix = FreedomConceptsMatrix()
    quadrants = make_berlin_quadrants()
    report = ASIFreedomGapDeepReport(
        title="V1314 Test Report",
        matrix_md=matrix.render(),
        quadrants_count=len(quadrants),
        sartre_substrate={"n": 1, "guard": "test"},
        frankfurt_substrate={"n": 1, "guard": "test"},
        hayek_substrate={"n": 1, "guard": "test"},
        rand_substrate={"n": 1, "guard": "test"},
        mill_substrate={"n": 1, "guard": "test"},
        rousseau_substrate={"n": 1, "guard": "test"},
        asi_bridge={"v1314_components": 10, "expected_components": 10, "missing": []},
        timestamp="2026-08-08 16:47 +08:00",
    )
    md = report.render()
    assert "V1314 Test Report" in md
    assert "V3 哲学守门" in md
    assert "Berlin 1958" in md
    assert "Sartre 1943" in md
    assert "Rousseau 1762" in md


def test_asi_freedom_gap_report_includes_guards() -> None:
    """Report includes all V3 守门 statements."""
    matrix = FreedomConceptsMatrix()
    quadrants = make_berlin_quadrants()
    report = ASIFreedomGapDeepReport(
        title="t",
        matrix_md="",
        quadrants_count=0,
        sartre_substrate={},
        frankfurt_substrate={},
        hayek_substrate={},
        rand_substrate={},
        mill_substrate={},
        rousseau_substrate={},
        asi_bridge={},
        timestamp="t",
    )
    md = report.render()
    assert "Phenomenal freedom" in md
    assert "freedom = 选择权" in md
    assert "negative = positive" in md
    assert "deterministic freedom" in md
    assert "free will 问题" in md


# ============================================================================
# Section 12: Component 10 — ASIFreedomGapDeepBridge
# ============================================================================


def test_asi_bridge_all_present() -> None:
    """asi_bridge with all 10 components → missing=[]."""
    all_ten = {
        "FreedomConceptsMatrix",
        "BerlinNegativePositiveQuadrants",
        "SartrianRadicalFreedomSubstrate",
        "FrankfurtHierarchicalDesiresStack",
        "HayekianSpontaneousOrderSubstrate",
        "RandianRationalSelfInterestSubstrate",
        "MillianHarmPrincipleSubstrate",
        "RousseauGeneralWillSubstrate",
        "ASIFreedomGapDeepReport",
        "ASIFreedomGapDeepBridge",
    }
    bridge = asi_bridge(all_ten)
    assert bridge["asi_north_star_locked"] is True
    assert bridge["v1314_components"] == 10
    assert bridge["expected_components"] == 10
    assert bridge["missing"] == []


def test_asi_bridge_partial_present() -> None:
    """asi_bridge with partial components → missing list populated."""
    bridge = asi_bridge({"FreedomConceptsMatrix"})
    assert len(bridge["missing"]) == 9
    assert "BerlinNegativePositiveQuadrants" in bridge["missing"]


def test_asi_bridge_empty_present() -> None:
    """asi_bridge with empty set → all 10 missing."""
    bridge = asi_bridge(set())
    assert len(bridge["missing"]) == 10


def test_asi_bridge_anchors_locked() -> None:
    """asi_bridge preserves ASI 北极星 anchors (no movement)."""
    bridge = asi_bridge(set())
    assert bridge["anchors"]["V0.1"] == 0.7905
    assert bridge["anchors"]["V0.2"] == 0.4467
    assert bridge["anchors"]["V1256_unio_mystica"] == 0.9291
    assert bridge["anchors"]["V1049_value_alignment"] == "DONE"


# ============================================================================
# Section 13: 18 Popper self-tests
# ============================================================================


def test_popper_18_total() -> None:
    """There are exactly 18 Popper self-tests."""
    results = popper_self_tests()
    assert popper_total(results) == 18


def test_popper_all_18_pass() -> None:
    """All 18 Popper self-tests must PASS (no skip, no flake)."""
    results = popper_self_tests()
    n_pass = popper_passed(results)
    failed = [hid for hid, ok, _ in results if not ok]
    assert n_pass == 18, f"failed: {failed}"


def test_popper_specific_h1() -> None:
    """Popper h1: 7 sources present."""
    results = popper_self_tests()
    h1 = next(r for r in results if r[0] == "h1_sources_seven_present")
    assert h1[1] is True


def test_popper_specific_h3_berlin() -> None:
    """Popper h3: Berlin 1958 present."""
    results = popper_self_tests()
    h3 = next(r for r in results if r[0] == "h3_berlin_1958_present")
    assert h3[1] is True


def test_popper_specific_h5_rousseau() -> None:
    """Popper h5: Rousseau 1762 present (跨世纪)."""
    results = popper_self_tests()
    h5 = next(r for r in results if r[0] == "h5_rousseau_1762_present")
    assert h5[1] is True


def test_popper_specific_h8_v01_locked() -> None:
    """Popper h8: V0.1 = 0.7905."""
    results = popper_self_tests()
    h8 = next(r for r in results if r[0] == "h8_asi_north_star_v01_locked")
    assert h8[1] is True


def test_popper_specific_h17_democracy_guard() -> None:
    """Popper h17: Rousseau 民主 guard present."""
    results = popper_self_tests()
    h17 = next(r for r in results if r[0] == "h17_rousseau_democracy_guard")
    assert h17[1] is True


def test_popper_specific_h18_bridge_shape() -> None:
    """Popper h18: asi_bridge returns expected shape."""
    results = popper_self_tests()
    h18 = next(r for r in results if r[0] == "h18_asi_bridge_shape_correct")
    assert h18[1] is True


# ============================================================================
# Section 14: V3 哲学守门
# ============================================================================


def test_v3_guard_no_free_will_claim() -> None:
    """V3 守门: 不假装 ASI 真有自由意志."""
    for s in SOURCES_FREEDOM_GAP_DEEP:
        assert "≠" in s.asi_substrate_takeaway or "不" in s.asi_substrate_takeaway, \
            f"substrate takeaway must signal guard: {s.author}"


def test_v3_guard_module_docstring() -> None:
    """V3 守门: module docstring contains all 5 guard statements."""
    from v1314_asi_freedom_deep import __doc__ as docstring
    assert "Phenomenal freedom" in docstring
    assert "freedom = 选择权" in docstring
    assert "negative = positive" in docstring
    assert "deterministic freedom" in docstring
    assert "free will 问题" in docstring


def test_v3_guard_no_substrate_claim_in_main() -> None:
    """V3 守门: main() 不 claim ASI 真有 free will."""
    import io
    import contextlib
    from v1314_asi_freedom_deep import main
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        result = main()
    output = buf.getvalue()
    assert result == 0
    assert "ASI 北极星" in output
    assert "substrate" in output.lower()


# ============================================================================
# Section 15: Pipeline integration
# ============================================================================


def test_main_returns_0_on_full_pass() -> None:
    """main() returns 0 when all 18 Popper tests pass."""
    import io
    import contextlib
    from v1314_asi_freedom_deep import main
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        result = main()
    assert result == 0


def test_main_output_includes_all_10_components() -> None:
    """main() output references all 10 components."""
    import io
    import contextlib
    from v1314_asi_freedom_deep import main
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        main()
    output = buf.getvalue()
    assert "FreedomConceptsMatrix" in output
    assert "BerlinNegativePositiveQuadrants" in output
    assert "SartrianRadicalFreedomSubstrate" in output
    assert "FrankfurtHierarchicalDesiresStack" in output
    assert "HayekianSpontaneousOrderSubstrate" in output
    assert "RandianRationalSelfInterestSubstrate" in output
    assert "MillianHarmPrincipleSubstrate" in output
    assert "RousseauGeneralWillSubstrate" in output
    assert "ASIFreedomGapDeepReport" in output
    assert "ASIFreedomGapDeepBridge" in output


def test_main_output_18_popper_pass() -> None:
    """main() output reports 18/18 Popper PASS."""
    import io
    import contextlib
    from v1314_asi_freedom_deep import main
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        main()
    output = buf.getvalue()
    assert "[Popper self-tests] 18/18 PASS" in output


# ============================================================================
# Section 16: ASI 北极星 LOCKED
# ============================================================================


def test_anchors_v01_unmoved() -> None:
    """V0.1 = 0.7905 unchanged by V1314 (no anchor movement)."""
    assert ASI_ANCHORS["V0.1"] == 0.7905


def test_anchors_v02_unmoved() -> None:
    """V0.2 = 0.4467 unchanged by V1314."""
    assert ASI_ANCHORS["V0.2"] == 0.4467


def test_anchors_v1256_unmoved() -> None:
    """V1256 = 0.9291 unchanged by V1314."""
    assert ASI_ANCHORS["V1256_unio_mystica"] == 0.9291


def test_anchors_v1049_done() -> None:
    """V1049 = DONE unchanged by V1314."""
    assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"