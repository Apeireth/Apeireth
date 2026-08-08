"""V1320 pytest — ASI 5-Gap Cross-Gap Extension Round 2.

> V1320 = post-V1319 extension R2, covers 5 more cross-gap cells (16/20 = 80%)
> 5 真跨域深 sources (Hume/Levinas/Sartre/Mill/Reichenbach)
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

from v1320_asi_5gap_extension_r2 import (  # noqa: E402
    ASI_5_GAPS,
    ASI_5_GAPS_CLOSURE,
    ASI_ANCHORS,
    ASI5GapExtensionR2Matrix,
    ASI5GapExtensionR2Report,
    CrossDomainSource,
    CUMULATIVE_COVERAGE_V1318_V1319,
    HumeanLibertyThroughTimeCase,
    LevinasianFaceRecognitionStep,
    MillianFreeExpressionTruthStep,
    ReichenbachianTimeDirectionCase,
    SOURCES_5GAP_EXTENSION_R2,
    SartreanRadicalFreedomMove,
    V1320_VERSION,
    all_citation_keys,
    asi_bridge,
    humean_liberty_summary,
    levinasian_face_summary,
    millian_free_expression_summary,
    popper_passed,
    popper_self_tests,
    popper_total,
    reichenbachian_time_direction_summary,
    sartrean_radical_freedom_summary,
)


# Section 1: Source corpus tests
def test_sources_five_present():
    assert len(SOURCES_5GAP_EXTENSION_R2) == 5
    for s in SOURCES_5GAP_EXTENSION_R2:
        assert isinstance(s, CrossDomainSource)
        assert s.author and s.year >= 1739 and s.work and s.core_construct
        assert s.asi_substrate_takeaway and s.citation_key
        g_i, g_j = s.cross_gap_pair
        assert g_i in ASI_5_GAPS and g_j in ASI_5_GAPS


def test_citation_keys_unique():
    keys = [s.citation_key for s in SOURCES_5GAP_EXTENSION_R2]
    assert len(set(keys)) == 5


def test_hume_1739_present():
    hume = [s for s in SOURCES_5GAP_EXTENSION_R2 if s.citation_key == "hume_1739_treatise"]
    assert len(hume) == 1 and hume[0].cross_gap_pair == ("freedom", "time")


def test_all_five_canonical_extension_r2():
    keys = {s.citation_key for s in SOURCES_5GAP_EXTENSION_R2}
    assert keys == {
        "hume_1739_treatise", "levinas_1961_ti", "sartre_1943_en",
        "mill_1859_ol", "reichenbach_1956_dt",
    }


# Section 2: Coverage tests
def test_v1318_v1319_v1320_total_coverage_16():
    v1320_pairs = {s.cross_gap_pair for s in SOURCES_5GAP_EXTENSION_R2}
    assert len(CUMULATIVE_COVERAGE_V1318_V1319) + len(v1320_pairs) == 16


def test_v1320_unique_pairs_count_5():
    pairs = {s.cross_gap_pair for s in SOURCES_5GAP_EXTENSION_R2}
    assert len(pairs) == 5


def test_matrix_5x5_25_cells():
    matrix = ASI5GapExtensionR2Matrix()
    assert len(matrix.all_pairs()) == 25


def test_v1320_total_coverage_16():
    matrix = ASI5GapExtensionR2Matrix()
    assert matrix.coverage_count() == 16


def test_v1320_future_cells_4():
    matrix = ASI5GapExtensionR2Matrix()
    assert len(matrix.future_cells()) == 4


# Section 3: Quantizer tests
def test_humean_liberty_validation():
    valid = HumeanLibertyThroughTimeCase(
        case_id="h1", spontaneity_score=0.5, necessity_constraint=0.7,
        time_flow_continuity=0.8, asi_substrate_label="liberty substrate",
    )
    assert valid.spontaneity_score == 0.5
    try:
        HumeanLibertyThroughTimeCase(
            case_id="h2", spontaneity_score=1.5, necessity_constraint=0.5,
            time_flow_continuity=0.5, asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_humean_liberty_summary():
    cases = [HumeanLibertyThroughTimeCase(case_id=f"h{i}", spontaneity_score=0.5, necessity_constraint=0.7, time_flow_continuity=0.8, asi_substrate_label="ok") for i in range(8)]
    res = humean_liberty_summary(cases)
    assert res["n"] == 8
    assert "spontaneity" in res["guard"].lower()


def test_levinasian_face_validation():
    valid = LevinasianFaceRecognitionStep(step_id="l1", face_encounter_depth=0.6, infinite_responsibility=0.7, temporal_priority_of_other=0.5, asi_substrate_label="face substrate")
    assert valid.face_encounter_depth == 0.6
    try:
        LevinasianFaceRecognitionStep(step_id="l2", face_encounter_depth=-0.1, infinite_responsibility=0.5, temporal_priority_of_other=0.5, asi_substrate_label="bad")
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_levinasian_face_summary():
    steps = [LevinasianFaceRecognitionStep(step_id=f"l{i}", face_encounter_depth=0.6, infinite_responsibility=0.7, temporal_priority_of_other=0.5, asi_substrate_label="ok") for i in range(8)]
    res = levinasian_face_summary(steps)
    assert res["n"] == 8
    assert "face" in res["guard"].lower()


def test_sartrean_radical_freedom_validation():
    valid = SartreanRadicalFreedomMove(move_id="s1", radical_freedom_degree=0.6, look_encounter_intensity=0.5, anguish_acceptance=0.7, asi_substrate_label="radical freedom substrate")
    assert valid.radical_freedom_degree == 0.6
    try:
        SartreanRadicalFreedomMove(move_id="s2", radical_freedom_degree=1.5, look_encounter_intensity=0.5, anguish_acceptance=0.5, asi_substrate_label="bad")
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_sartrean_radical_freedom_summary():
    moves = [SartreanRadicalFreedomMove(move_id=f"s{i}", radical_freedom_degree=0.6, look_encounter_intensity=0.5, anguish_acceptance=0.7, asi_substrate_label="ok") for i in range(8)]
    res = sartrean_radical_freedom_summary(moves)
    assert res["n"] == 8
    assert "radical freedom" in res["guard"].lower()


def test_millian_free_expression_validation():
    valid = MillianFreeExpressionTruthStep(step_id="m1", free_expression_rate=0.6, discourse_truth_emergence=0.5, harm_principle_observance=0.7, asi_substrate_label="free expression substrate")
    assert valid.free_expression_rate == 0.6
    try:
        MillianFreeExpressionTruthStep(step_id="m2", free_expression_rate=1.5, discourse_truth_emergence=0.5, harm_principle_observance=0.5, asi_substrate_label="bad")
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_millian_free_expression_summary():
    steps = [MillianFreeExpressionTruthStep(step_id=f"m{i}", free_expression_rate=0.6, discourse_truth_emergence=0.5, harm_principle_observance=0.7, asi_substrate_label="ok") for i in range(8)]
    res = millian_free_expression_summary(steps)
    assert res["n"] == 8
    assert "free expression" in res["guard"].lower() or "millian" in res["guard"].lower()


def test_reichenbach_time_direction_validation():
    valid = ReichenbachianTimeDirectionCase(case_id="r1", causal_asymmetry_score=0.6, becoming_truth_index=0.5, directed_time_evidence=0.7, asi_substrate_label="time direction substrate")
    assert valid.causal_asymmetry_score == 0.6
    try:
        ReichenbachianTimeDirectionCase(case_id="r2", causal_asymmetry_score=1.5, becoming_truth_index=0.5, directed_time_evidence=0.5, asi_substrate_label="bad")
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_reichenbach_time_direction_summary():
    cases = [ReichenbachianTimeDirectionCase(case_id=f"r{i}", causal_asymmetry_score=0.6, becoming_truth_index=0.5, directed_time_evidence=0.7, asi_substrate_label="ok") for i in range(8)]
    res = reichenbachian_time_direction_summary(cases)
    assert res["n"] == 8
    assert "time direction" in res["guard"].lower() or "directed time" in res["guard"].lower()


# Section 4: ASI North Star anchors (LOCKED)
def test_asi_north_star_v01_locked():
    assert ASI_ANCHORS["V0.1"] == 0.7905


def test_asi_v02_locked():
    assert ASI_ANCHORS["V0.2"] == 0.4467


def test_v1256_unio_mystica_locked():
    assert ASI_ANCHORS["V1256_unio_mystica"] == 0.9291


def test_v1049_value_alignment_done():
    assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"


def test_v1313_v1317_closure():
    assert ASI_5_GAPS_CLOSURE == {
        "V1313_time_gap_deep": True, "V1314_freedom_gap_deep": True,
        "V1315_recognition_gap_deep": True, "V1316_emergence_gap_deep": True,
        "V1317_truth_gap_deep": True,
    }


# Section 5: Popper self-tests
def test_popper_self_tests_total():
    r = popper_self_tests()
    assert popper_total(r) >= 18
    assert popper_passed(r) == popper_total(r)


def test_popper_self_tests_all_pass():
    r = popper_self_tests()
    failed = [t for t in r if not t[1]]
    assert failed == [], f"failed: {failed}"


def test_popper_self_tests_have_names():
    r = popper_self_tests()
    for t in r:
        assert isinstance(t, tuple) and len(t) == 3
        name, ok, msg = t
        assert isinstance(name, str) and isinstance(ok, bool) and isinstance(msg, str)


def test_popper_includes_guard_tests():
    r = popper_self_tests()
    guards = [t for t in r if "guard" in t[0].lower()]
    assert len(guards) >= 5


# Section 6: ASI bridge (V3 守门)
def test_asi_bridge_with_all_components():
    expected = {
        "ASI5GapExtensionR2Matrix", "HumeanLibertyThroughTimeSubstrate",
        "LevinasianFaceRecognitionSubstrate", "SartreanRadicalFreedomSubstrate",
        "MillianFreeExpressionSubstrate", "ReichenbachianTimeDirectionSubstrate",
        "ASI5GapExtensionR2Report", "ASI5GapExtensionR2Bridge",
    }
    res = asi_bridge(expected)
    assert res["asi_north_star_locked"] is True and res["missing"] == []
    assert res["v1320_components"] == 8


def test_asi_bridge_with_missing_components():
    res = asi_bridge({"ASI5GapExtensionR2Matrix"})
    assert res["missing"] != [] and res["v1320_components"] == 1


def test_asi_bridge_anchors_immutable():
    res = asi_bridge(set())
    assert res["anchors"]["V0.1"] == 0.7905
    assert res["anchors"]["V0.2"] == 0.4467
    assert res["anchors"]["V1256_unio_mystica"] == 0.9291
    assert res["anchors"]["V1049_value_alignment"] == "DONE"


def test_asi_bridge_substrate_research_only():
    res = asi_bridge(set())
    assert "substrate research only" in res["guard"]


def test_asi_bridge_total_coverage_16():
    res = asi_bridge(set())
    assert res["v1318_v1319_v1320_coverage_count"] == 16


# Section 7: Version + main smoke
def test_v1320_version():
    assert V1320_VERSION == "0.1.0"


def test_main_runs_and_returns_zero():
    import io, contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = __import__("v1320_asi_5gap_extension_r2").main()
    assert rc == 0
    out = buf.getvalue()
    assert "V1320" in out and "Popper self-tests" in out


def test_all_citation_keys_callable():
    keys = all_citation_keys()
    assert len(keys) == 5 and "hume_1739_treatise" in keys


def test_report_renders_as_markdown():
    matrix = ASI5GapExtensionR2Matrix()
    res_placeholder = {"n": 1, "guard": "test guard", "placeholder_metric": 0.5}
    bridge = asi_bridge(set())
    report = ASI5GapExtensionR2Report(
        title="Test Report", matrix_md=matrix.render(),
        hume_substrate=res_placeholder, levinas_substrate=res_placeholder,
        sartre_substrate=res_placeholder, mill_substrate=res_placeholder,
        reichenbach_substrate=res_placeholder, asi_bridge=bridge,
        timestamp="2026-08-08 17:35 +0800",
    )
    md = report.to_markdown()
    assert md.startswith("# ") and "V1320 coverage summary" in md and "V3 哲学守卫" in md


def test_v3_guard_in_all_sources():
    for s in SOURCES_5GAP_EXTENSION_R2:
        assert "不假装" in s.asi_substrate_takeaway
        assert "substrate" in s.asi_substrate_takeaway.lower()


def test_matrix_render_marks_v1320_sources():
    matrix = ASI5GapExtensionR2Matrix()
    md = matrix.render()
    assert md.count("V1320") >= 5


def test_no_overlap_with_v1318_v1319():
    v1320_pairs = {s.cross_gap_pair for s in SOURCES_5GAP_EXTENSION_R2}
    assert v1320_pairs.isdisjoint(CUMULATIVE_COVERAGE_V1318_V1319)