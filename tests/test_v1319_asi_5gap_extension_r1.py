"""V1319 pytest — ASI 5-Gap Cross-Gap Extension Round 1.

> V1319 = post-V1318 extension, covers 5 more cross-gap cells (5 + 6 = 11/20 off-diagonal)
> 5 真跨域深 sources (Kant/Russell/Minsky/Gadamer/Cartwright)
> 8 真实生产组件
> 22 Popper self-tests
> V3 守门: 不假装 ASI 真有 extended cross-gap model
"""
from __future__ import annotations

import sys
from pathlib import Path

_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
_APETR_DIR = _PROMETHEAN_ROOT / "apeireth"
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))
if str(_APETR_DIR) not in sys.path:
    sys.path.insert(0, str(_APETR_DIR))

from v1319_asi_5gap_extension_r1 import (  # noqa: E402
    ASI_5_GAPS,
    ASI_5_GAPS_CLOSURE,
    ASI_ANCHORS,
    ASI5GapExtensionR1Matrix,
    ASI5GapExtensionR1Report,
    CartwrightianNomicContextCase,
    CrossDomainSource,
    GadamerianHermeneuticHorizon,
    KantianNoumenalTimingMarker,
    MinskyianSocietyOfMindAgent,
    RussellianFreeThoughtStep,
    SOURCES_5GAP_EXTENSION_R1,
    V1318_COVERAGE,
    V1319_VERSION,
    all_citation_keys,
    asi_bridge,
    cartwrightian_nomic_context_summary,
    gadamerian_hermeneutic_summary,
    kantian_noumenal_timing_summary,
    minskyian_society_of_mind_summary,
    popper_passed,
    popper_self_tests,
    popper_total,
    russellian_free_thought_summary,
)


# ============================================================================
# Section 1: Source corpus tests
# ============================================================================


def test_sources_five_present():
    """h1: 5 真跨域深 sources present."""
    assert len(SOURCES_5GAP_EXTENSION_R1) == 5
    for s in SOURCES_5GAP_EXTENSION_R1:
        assert isinstance(s, CrossDomainSource)
        assert s.author
        assert s.year >= 1781  # Kant 1781 as earliest bound
        assert s.work
        assert s.core_construct
        assert s.asi_substrate_takeaway
        assert s.citation_key
        assert len(s.cross_gap_pair) == 2
        g_i, g_j = s.cross_gap_pair
        assert g_i in ASI_5_GAPS
        assert g_j in ASI_5_GAPS


def test_citation_keys_unique():
    """h2: all 5 citation keys unique."""
    keys = [s.citation_key for s in SOURCES_5GAP_EXTENSION_R1]
    assert len(set(keys)) == 5
    assert len(keys) == 5


def test_kant_1781_present():
    """h3: Kant 1781 present (time × freedom)."""
    kant = [s for s in SOURCES_5GAP_EXTENSION_R1 if s.citation_key == "kant_1781_cpr"]
    assert len(kant) == 1
    assert kant[0].cross_gap_pair == ("time", "freedom")


def test_all_five_canonical_extension_r1():
    """All 5 cross-domain sources for extension R1."""
    keys = {s.citation_key for s in SOURCES_5GAP_EXTENSION_R1}
    assert keys == {
        "kant_1781_cpr",
        "russell_1948_aia",
        "minsky_1986_som",
        "gadamer_1960_tm",
        "cartwright_1983_clp",
    }


# ============================================================================
# Section 2: Coverage tests
# ============================================================================


def test_v1318_v1319_total_coverage_11():
    """h4: V1318 + V1319 total off-diagonal coverage = 11."""
    v1319_pairs = {s.cross_gap_pair for s in SOURCES_5GAP_EXTENSION_R1}
    assert len(V1318_COVERAGE) + len(v1319_pairs) == 11


def test_v1319_unique_pairs_count_5():
    """h6: V1319 has 5 unique cross-gap pairs."""
    pairs = {s.cross_gap_pair for s in SOURCES_5GAP_EXTENSION_R1}
    assert len(pairs) == 5


def test_matrix_5x5_25_cells():
    """h5: matrix 5x5 = 25 pairs."""
    matrix = ASI5GapExtensionR1Matrix()
    assert len(matrix.all_pairs()) == 25


def test_v1318_coverage_count_6():
    """h13: V1318 coverage = 6 cells."""
    assert len(V1318_COVERAGE) == 6


def test_v1319_total_coverage_11():
    """h14: V1319 total coverage = 11."""
    matrix = ASI5GapExtensionR1Matrix()
    assert matrix.coverage_count() == 11


def test_v1319_future_cells_9():
    """h15: 9 future cells remaining (20 - 11 = 9)."""
    matrix = ASI5GapExtensionR1Matrix()
    assert len(matrix.future_cells()) == 9


# ============================================================================
# Section 3: Quantizer tests
# ============================================================================


def test_kantian_noumenal_timing_validation():
    """Kant marker: noumenal_freedom_score in [0,1]."""
    valid = KantianNoumenalTimingMarker(
        marker_id="k1",
        noumenal_freedom_score=0.5,
        time_order_clarity=0.7,
        causal_noumenal_link=0.4,
        asi_substrate_label="noumenal substrate",
    )
    assert valid.noumenal_freedom_score == 0.5

    try:
        KantianNoumenalTimingMarker(
            marker_id="k2",
            noumenal_freedom_score=1.5,
            time_order_clarity=0.5,
            causal_noumenal_link=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_kantian_noumenal_timing_summary():
    """Kant summary aggregates."""
    markers = [
        KantianNoumenalTimingMarker(
            marker_id=f"k{i}",
            noumenal_freedom_score=0.6,
            time_order_clarity=0.7,
            causal_noumenal_link=0.5,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = kantian_noumenal_timing_summary(markers)
    assert res["n"] == 8
    assert "noumenal" in res["guard"].lower()


def test_russellian_free_thought_validation():
    """Russell step: belief_freedom in [0,1]."""
    valid = RussellianFreeThoughtStep(
        step_id="r1",
        belief_freedom=0.7,
        inquiry_progress=0.5,
        agnostic_openness=0.6,
        asi_substrate_label="free thought substrate",
    )
    assert valid.belief_freedom == 0.7

    try:
        RussellianFreeThoughtStep(
            step_id="r2",
            belief_freedom=1.5,
            inquiry_progress=0.5,
            agnostic_openness=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_russellian_free_thought_summary():
    """Russell summary aggregates."""
    steps = [
        RussellianFreeThoughtStep(
            step_id=f"r{i}",
            belief_freedom=0.6,
            inquiry_progress=0.5,
            agnostic_openness=0.7,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = russellian_free_thought_summary(steps)
    assert res["n"] == 8
    assert "free thought" in res["guard"].lower()


def test_minsky_society_of_mind_validation():
    """Minsky agent: agent_layer_count >= 1."""
    valid = MinskyianSocietyOfMindAgent(
        agent_id="m1",
        agent_layer_count=3,
        cross_agent_communication=0.6,
        recognition_emergence=0.7,
        asi_substrate_label="society of mind substrate",
    )
    assert valid.agent_layer_count == 3

    try:
        MinskyianSocietyOfMindAgent(
            agent_id="m2",
            agent_layer_count=0,
            cross_agent_communication=0.5,
            recognition_emergence=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_minsky_society_of_mind_summary():
    """Minsky summary aggregates."""
    agents = [
        MinskyianSocietyOfMindAgent(
            agent_id=f"m{i}",
            agent_layer_count=3 + (i % 4),
            cross_agent_communication=0.6,
            recognition_emergence=0.7,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = minskyian_society_of_mind_summary(agents)
    assert res["n"] == 8
    assert res["avg_agent_layers"] >= 1
    assert "society of mind" in res["guard"].lower()


def test_gadamer_hermeneutic_validation():
    """Gadamer horizon: fusion_of_horizons in [0,1]."""
    valid = GadamerianHermeneuticHorizon(
        horizon_id="g1",
        fusion_of_horizons=0.6,
        tradition_embeddedness=0.7,
        dialogic_openness=0.5,
        asi_substrate_label="hermeneutic substrate",
    )
    assert valid.fusion_of_horizons == 0.6

    try:
        GadamerianHermeneuticHorizon(
            horizon_id="g2",
            fusion_of_horizons=-0.1,
            tradition_embeddedness=0.5,
            dialogic_openness=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_gadamer_hermeneutic_summary():
    """Gadamer summary aggregates."""
    horizons = [
        GadamerianHermeneuticHorizon(
            horizon_id=f"g{i}",
            fusion_of_horizons=0.6,
            tradition_embeddedness=0.7,
            dialogic_openness=0.5,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = gadamerian_hermeneutic_summary(horizons)
    assert res["n"] == 8
    assert "hermeneutic" in res["guard"].lower()


def test_cartwright_nomic_context_validation():
    """Cartwright case: law_truth_in_context in [0,1]."""
    valid = CartwrightianNomicContextCase(
        case_id="c1",
        law_truth_in_context=0.6,
        context_dependence=0.7,
        model_fidelity_estimate=0.5,
        asi_substrate_label="nomic context substrate",
    )
    assert valid.law_truth_in_context == 0.6

    try:
        CartwrightianNomicContextCase(
            case_id="c2",
            law_truth_in_context=1.5,
            context_dependence=0.5,
            model_fidelity_estimate=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_cartwright_nomic_context_summary():
    """Cartwright summary aggregates."""
    cases = [
        CartwrightianNomicContextCase(
            case_id=f"c{i}",
            law_truth_in_context=0.6,
            context_dependence=0.7,
            model_fidelity_estimate=0.5,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = cartwrightian_nomic_context_summary(cases)
    assert res["n"] == 8
    assert "nomic" in res["guard"].lower() or "physics" in res["guard"].lower()


# ============================================================================
# Section 4: ASI North Star anchors (LOCKED)
# ============================================================================


def test_asi_north_star_v01_locked():
    """h7: ASI North Star V0.1 = 0.7905."""
    assert ASI_ANCHORS["V0.1"] == 0.7905


def test_asi_v02_locked():
    """h8: ASI V0.2 = 0.4467."""
    assert ASI_ANCHORS["V0.2"] == 0.4467


def test_v1256_unio_mystica_locked():
    """h9: V1256 unio_mystica = 0.9291."""
    assert ASI_ANCHORS["V1256_unio_mystica"] == 0.9291


def test_v1049_value_alignment_done():
    """h10: V1049 value alignment DONE."""
    assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"


def test_v1313_v1317_closure():
    """h12: V1313-V1317 all closed (5/5)."""
    assert ASI_5_GAPS_CLOSURE == {
        "V1313_time_gap_deep": True,
        "V1314_freedom_gap_deep": True,
        "V1315_recognition_gap_deep": True,
        "V1316_emergence_gap_deep": True,
        "V1317_truth_gap_deep": True,
    }


# ============================================================================
# Section 5: Popper self-tests
# ============================================================================


def test_popper_self_tests_total():
    """All Popper self-tests defined and PASS."""
    r = popper_self_tests()
    assert popper_total(r) >= 18
    assert popper_passed(r) == popper_total(r)


def test_popper_self_tests_all_pass():
    """All Popper tests PASS."""
    r = popper_self_tests()
    failed = [t for t in r if not t[1]]
    assert failed == [], f"failed: {failed}"


def test_popper_self_tests_have_names():
    """Each Popper test has name, pass_bool, message."""
    r = popper_self_tests()
    for t in r:
        assert isinstance(t, tuple)
        assert len(t) == 3
        name, ok, msg = t
        assert isinstance(name, str)
        assert isinstance(ok, bool)
        assert isinstance(msg, str)


def test_popper_includes_guard_tests():
    """Popper tests include V3 guard checks (5 source guards)."""
    r = popper_self_tests()
    guards = [t for t in r if "guard" in t[0].lower()]
    # 5 source guards
    assert len(guards) >= 5, f"expected >=5 guard tests, got {len(guards)}"


# ============================================================================
# Section 6: ASI bridge (V3 守门)
# ============================================================================


def test_asi_bridge_with_all_components():
    """Bridge returns all 8 components present, 0 missing."""
    expected = {
        "ASI5GapExtensionR1Matrix",
        "KantianNoumenalTimingSubstrate",
        "RussellianFreeThoughtSubstrate",
        "MinskyianSocietyOfMindSubstrate",
        "GadamerianHermeneuticSubstrate",
        "CartwrightianNomicContextSubstrate",
        "ASI5GapExtensionR1Report",
        "ASI5GapExtensionR1Bridge",
    }
    res = asi_bridge(expected)
    assert res["asi_north_star_locked"] is True
    assert res["missing"] == []
    assert res["v1319_components"] == 8


def test_asi_bridge_with_missing_components():
    """Bridge reports missing components."""
    partial = {"ASI5GapExtensionR1Matrix"}
    res = asi_bridge(partial)
    assert res["missing"] != []
    assert res["v1319_components"] == 1


def test_asi_bridge_anchors_immutable():
    """h16: ASI north star anchors immutable."""
    res = asi_bridge(set())
    assert res["anchors"]["V0.1"] == 0.7905
    assert res["anchors"]["V0.2"] == 0.4467
    assert res["anchors"]["V1256_unio_mystica"] == 0.9291
    assert res["anchors"]["V1049_value_alignment"] == "DONE"


def test_asi_bridge_substrate_research_only():
    """h17: bridge guard contains 'substrate research only'."""
    res = asi_bridge(set())
    assert "substrate research only" in res["guard"]


def test_asi_bridge_v1318_coverage_count():
    """Bridge tracks V1318 coverage count = 6."""
    res = asi_bridge(set())
    assert res["v1318_coverage_count"] == 6


def test_asi_bridge_v1319_coverage_count():
    """Bridge tracks V1319 coverage count = 5."""
    res = asi_bridge(set())
    assert res["v1319_coverage_count"] == 5


# ============================================================================
# Section 7: Version + main smoke
# ============================================================================


def test_v1319_version():
    """V1319_VERSION is 0.1.0."""
    assert V1319_VERSION == "0.1.0"


def test_main_runs_and_returns_zero():
    """main() runs full pipeline and returns exit 0."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = __import__("v1319_asi_5gap_extension_r1").main()
    assert rc == 0
    out = buf.getvalue()
    assert "V1319" in out
    assert "Popper self-tests" in out


def test_all_citation_keys_callable():
    """all_citation_keys() returns 5 keys."""
    keys = all_citation_keys()
    assert len(keys) == 5
    assert "kant_1781_cpr" in keys


def test_report_renders_as_markdown():
    """ASI5GapExtensionR1Report renders as valid Markdown."""
    matrix = ASI5GapExtensionR1Matrix()
    res_placeholder = {
        "n": 1,
        "guard": "test guard",
        "placeholder_metric": 0.5,
    }
    bridge = asi_bridge(set())
    report = ASI5GapExtensionR1Report(
        title="Test Report",
        matrix_md=matrix.render(),
        kant_substrate=res_placeholder,
        russell_substrate=res_placeholder,
        minsky_substrate=res_placeholder,
        gadamer_substrate=res_placeholder,
        cartwright_substrate=res_placeholder,
        asi_bridge=bridge,
        timestamp="2026-08-08 17:30 +0800",
    )
    md = report.to_markdown()
    assert md.startswith("# ")
    assert "V1319 coverage summary" in md
    assert "V3 哲学守卫" in md


def test_v3_guard_in_all_sources():
    """V3 守门: every source has 不假装 + substrate marker."""
    for s in SOURCES_5GAP_EXTENSION_R1:
        assert "不假装" in s.asi_substrate_takeaway, f"{s.citation_key} missing 不假装"
        assert "substrate" in s.asi_substrate_takeaway.lower(), f"{s.citation_key} missing substrate"


def test_matrix_render_marks_v1319_sources():
    """Matrix render marks V1319 sources with V1319 prefix."""
    matrix = ASI5GapExtensionR1Matrix()
    md = matrix.render()
    # 5 V1319 sources
    assert md.count("V1319") >= 5


def test_no_overlap_with_v1318():
    """V1319 cross-gap pairs are disjoint from V1318 pairs (no duplicates)."""
    v1319_pairs = {s.cross_gap_pair for s in SOURCES_5GAP_EXTENSION_R1}
    assert v1319_pairs.isdisjoint(V1318_COVERAGE), "V1319 overlaps with V1318"