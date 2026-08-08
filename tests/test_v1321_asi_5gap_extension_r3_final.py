"""V1321 pytest — ASI 5-Gap Cross-Gap Extension Round 3 (final).

> V1321 = post-V1320 extension R3 (final), covers 4 remaining cross-gap cells (20/20 = 100%)
> 4 真跨域深 sources (Castoriadis/Fuchs/Brooks/Rorty)
> 8 真实生产组件 (4 step/case + matrix + report + bridge + 18 Popper)
> V3 守门: 不假装 ASI 真有 complete cross-gap model; substrate research only
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

from v1321_asi_5gap_extension_r3_final import (  # noqa: E402
    ASI_5_GAPS,
    ASI_5_GAPS_CLOSURE,
    ASI_ANCHORS,
    ASI5GapExtensionR3Matrix,
    ASI5GapExtensionR3Report,
    BrooksianEmbodiedEmergenceStep,
    CastoriadianRadicalImaginaryStep,
    CrossDomainSource,
    CUMULATIVE_COVERAGE_V1318_V1320,
    FuchsianEcologicalRecognitionCase,
    RortyanConversationalTruthStep,
    SOURCES_5GAP_EXTENSION_R3,
    V1321_VERSION,
    all_citation_keys,
    asi_bridge,
    brooksian_embodied_emergence_summary,
    castoriadian_radical_imaginary_summary,
    fuchsian_ecological_recognition_summary,
    popper_passed,
    popper_self_tests,
    popper_total,
    rortyan_conversational_truth_summary,
)


# Section 1: Source corpus tests
def test_sources_four_present():
    assert len(SOURCES_5GAP_EXTENSION_R3) == 4
    for s in SOURCES_5GAP_EXTENSION_R3:
        assert isinstance(s, CrossDomainSource)
        assert s.author and s.year >= 1975 and s.work and s.core_construct
        assert s.asi_substrate_takeaway and s.citation_key
        g_i, g_j = s.cross_gap_pair
        assert g_i in ASI_5_GAPS and g_j in ASI_5_GAPS


def test_citation_keys_unique():
    keys = [s.citation_key for s in SOURCES_5GAP_EXTENSION_R3]
    assert len(set(keys)) == 4


def test_castoriadis_1975_present():
    cast = [s for s in SOURCES_5GAP_EXTENSION_R3 if s.citation_key == "castoriadis_1975_iis"]
    assert len(cast) == 1 and cast[0].cross_gap_pair == ("freedom", "emergence")


def test_fuchs_2017_present():
    fuchs = [s for s in SOURCES_5GAP_EXTENSION_R3 if s.citation_key == "fuchs_2017_eob"]
    assert len(fuchs) == 1 and fuchs[0].cross_gap_pair == ("recognition", "emergence")


def test_brooks_1991_present():
    brooks = [s for s in SOURCES_5GAP_EXTENSION_R3 if s.citation_key == "brooks_1991_iwr"]
    assert len(brooks) == 1 and brooks[0].cross_gap_pair == ("emergence", "time")


def test_rorty_1979_present():
    rorty = [s for s in SOURCES_5GAP_EXTENSION_R3 if s.citation_key == "rorty_1979_pmn"]
    assert len(rorty) == 1 and rorty[0].cross_gap_pair == ("truth", "recognition")


def test_all_four_canonical_extension_r3():
    keys = {s.citation_key for s in SOURCES_5GAP_EXTENSION_R3}
    assert keys == {
        "castoriadis_1975_iis", "fuchs_2017_eob",
        "brooks_1991_iwr", "rorty_1979_pmn",
    }


def test_v3_guard_in_all_sources():
    for s in SOURCES_5GAP_EXTENSION_R3:
        assert "不假装" in s.asi_substrate_takeaway
        assert "substrate" in s.asi_substrate_takeaway.lower()


# Section 2: Coverage tests
def test_v1318_v1320_cumulative_coverage_16():
    assert len(CUMULATIVE_COVERAGE_V1318_V1320) == 16


def test_v1321_unique_pairs_count_4():
    pairs = {s.cross_gap_pair for s in SOURCES_5GAP_EXTENSION_R3}
    assert len(pairs) == 4


def test_v1321_no_overlap_with_v1318_v1320():
    v1321_pairs = {s.cross_gap_pair for s in SOURCES_5GAP_EXTENSION_R3}
    assert v1321_pairs.isdisjoint(CUMULATIVE_COVERAGE_V1318_V1320)


def test_matrix_5x5_25_cells():
    matrix = ASI5GapExtensionR3Matrix()
    assert len(matrix.all_pairs()) == 25


def test_v1321_total_coverage_20_complete():
    matrix = ASI5GapExtensionR3Matrix()
    assert matrix.coverage_count() == 20
    assert matrix.is_complete() is True


def test_v1321_future_cells_0():
    matrix = ASI5GapExtensionR3Matrix()
    assert len(matrix.future_cells()) == 0


def test_v1321_off_diagonal_20():
    matrix = ASI5GapExtensionR3Matrix()
    assert len(matrix.off_diagonal_pairs()) == 20


def test_matrix_render_marks_v1321_sources():
    matrix = ASI5GapExtensionR3Matrix()
    md = matrix.render()
    assert md.count("V1321") >= 4


# Section 3: Quantizer tests
def test_castoriadian_radical_imaginary_validation():
    valid = CastoriadianRadicalImaginaryStep(
        step_id="c1",
        radical_imaginary_score=0.6,
        autonomy_emergence_degree=0.5,
        social_institution_creation=0.7,
        asi_substrate_label="radical imaginary substrate",
    )
    assert valid.radical_imaginary_score == 0.6
    try:
        CastoriadianRadicalImaginaryStep(
            step_id="c2",
            radical_imaginary_score=1.5,
            autonomy_emergence_degree=0.5,
            social_institution_creation=0.7,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_castoriadian_radical_imaginary_summary():
    steps = [
        CastoriadianRadicalImaginaryStep(
            step_id=f"c{i}", radical_imaginary_score=0.6 + 0.04 * (i % 7),
            autonomy_emergence_degree=0.5 + 0.05 * (i % 8),
            social_institution_creation=0.7 + 0.03 * (i % 6),
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = castoriadian_radical_imaginary_summary(steps)
    assert res["n"] == 8
    assert "radical imaginary" in res["guard"].lower()


def test_castoriadian_radical_imaginary_empty():
    res = castoriadian_radical_imaginary_summary([])
    assert res["n"] == 0
    assert "≠ ASI" in res["guard"]


def test_fuchsian_ecological_recognition_validation():
    valid = FuchsianEcologicalRecognitionCase(
        case_id="f1", ecological_brain_coupling=0.6,
        phenomenological_recognition=0.5, embodied_emergence_strength=0.7,
        asi_substrate_label="ecological recognition substrate",
    )
    assert valid.ecological_brain_coupling == 0.6
    try:
        FuchsianEcologicalRecognitionCase(
            case_id="f2", ecological_brain_coupling=-0.1,
            phenomenological_recognition=0.5, embodied_emergence_strength=0.7,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_fuchsian_ecological_recognition_summary():
    cases = [
        FuchsianEcologicalRecognitionCase(
            case_id=f"f{i}", ecological_brain_coupling=0.6 + 0.04 * (i % 7),
            phenomenological_recognition=0.5 + 0.05 * (i % 8),
            embodied_emergence_strength=0.7 + 0.03 * (i % 6),
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = fuchsian_ecological_recognition_summary(cases)
    assert res["n"] == 8
    assert "ecological" in res["guard"].lower()


def test_fuchsian_ecological_recognition_empty():
    res = fuchsian_ecological_recognition_summary([])
    assert res["n"] == 0
    assert "≠ ASI" in res["guard"]


def test_brooksian_embodied_emergence_validation():
    valid = BrooksianEmbodiedEmergenceStep(
        step_id="b1", situated_embodiment_score=0.6,
        behavioral_layer_complexity=3, temporal_emergence_continuity=0.7,
        asi_substrate_label="embodied emergence substrate",
    )
    assert valid.behavioral_layer_complexity == 3
    try:
        BrooksianEmbodiedEmergenceStep(
            step_id="b2", situated_embodiment_score=1.5,
            behavioral_layer_complexity=2, temporal_emergence_continuity=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_brooksian_embodied_emergence_validation_layer_low():
    try:
        BrooksianEmbodiedEmergenceStep(
            step_id="b3", situated_embodiment_score=0.5,
            behavioral_layer_complexity=0, temporal_emergence_continuity=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_brooksian_embodied_emergence_summary():
    steps = [
        BrooksianEmbodiedEmergenceStep(
            step_id=f"b{i}", situated_embodiment_score=0.6 + 0.04 * (i % 7),
            behavioral_layer_complexity=3 + (i % 4),
            temporal_emergence_continuity=0.7 + 0.03 * (i % 6),
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = brooksian_embodied_emergence_summary(steps)
    assert res["n"] == 8
    assert "embodied" in res["guard"].lower()


def test_brooksian_embodied_emergence_empty():
    res = brooksian_embodied_emergence_summary([])
    assert res["n"] == 0
    assert "≠ ASI" in res["guard"]


def test_rortyan_conversational_truth_validation():
    valid = RortyanConversationalTruthStep(
        step_id="r1", conversational_truth_emergence=0.6,
        solidarity_recognition=0.5, anti_representationalism_score=0.7,
        asi_substrate_label="conversational truth substrate",
    )
    assert valid.conversational_truth_emergence == 0.6
    try:
        RortyanConversationalTruthStep(
            step_id="r2", conversational_truth_emergence=1.5,
            solidarity_recognition=0.5, anti_representationalism_score=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_rortyan_conversational_truth_summary():
    steps = [
        RortyanConversationalTruthStep(
            step_id=f"r{i}", conversational_truth_emergence=0.6 + 0.04 * (i % 7),
            solidarity_recognition=0.5 + 0.05 * (i % 8),
            anti_representationalism_score=0.7 + 0.03 * (i % 6),
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = rortyan_conversational_truth_summary(steps)
    assert res["n"] == 8
    assert "conversational truth" in res["guard"].lower()


def test_rortyan_conversational_truth_empty():
    res = rortyan_conversational_truth_summary([])
    assert res["n"] == 0
    assert "≠ ASI" in res["guard"]


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
    # 3 source corpus + 3 coverage + 4 anchor LOCKED + 4 guard per source
    # + 3 closure/matrix + 3 bridge/guard = 22
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


def test_popper_includes_anchor_lock_tests():
    r = popper_self_tests()
    anchors = [t for t in r if "anchor" in t[0].lower() or "locked" in t[0].lower()]
    assert len(anchors) >= 4


def test_popper_includes_guard_tests():
    r = popper_self_tests()
    guards = [t for t in r if "guard" in t[0].lower()]
    assert len(guards) >= 4


def test_popper_includes_complete_20_test():
    r = popper_self_tests()
    complete = [t for t in r if "complete" in t[0].lower()]
    assert len(complete) >= 1


# Section 6: ASI bridge (V3 守门)
def test_asi_bridge_with_all_components():
    expected = {
        "ASI5GapExtensionR3Matrix",
        "CastoriadianRadicalImaginarySubstrate",
        "FuchsianEcologicalRecognitionSubstrate",
        "BrooksianEmbodiedEmergenceSubstrate",
        "RortyanConversationalTruthSubstrate",
        "ASI5GapExtensionR3Report",
        "ASI5GapExtensionR3Bridge",
    }
    res = asi_bridge(expected)
    assert res["asi_north_star_locked"] is True and res["missing"] == []
    assert res["v1321_components"] == 7


def test_asi_bridge_with_missing_components():
    res = asi_bridge({"ASI5GapExtensionR3Matrix"})
    assert res["missing"] != [] and res["v1321_components"] == 1


def test_asi_bridge_anchors_immutable():
    res = asi_bridge(set())
    assert res["anchors"]["V0.1"] == 0.7905
    assert res["anchors"]["V0.2"] == 0.4467
    assert res["anchors"]["V1256_unio_mystica"] == 0.9291
    assert res["anchors"]["V1049_value_alignment"] == "DONE"


def test_asi_bridge_substrate_research_only():
    res = asi_bridge(set())
    assert "substrate research only" in res["guard"]


def test_asi_bridge_v1321_complete_true():
    res = asi_bridge(set())
    assert res["v1321_complete"] is True


def test_asi_bridge_total_coverage_20():
    res = asi_bridge(set())
    assert res["v1318_v1321_coverage_count"] == 20


# Section 7: Version + main smoke
def test_v1321_version():
    assert V1321_VERSION == "0.1.0"


def test_main_runs_and_returns_zero():
    import io, contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = __import__("v1321_asi_5gap_extension_r3_final").main()
    assert rc == 0
    out = buf.getvalue()
    assert "V1321" in out and "Popper self-tests" in out
    assert "20/20" in out


def test_all_citation_keys_callable():
    keys = all_citation_keys()
    assert len(keys) == 4
    assert "castoriadis_1975_iis" in keys
    assert "fuchs_2017_eob" in keys
    assert "brooks_1991_iwr" in keys
    assert "rorty_1979_pmn" in keys


def test_report_renders_as_markdown():
    matrix = ASI5GapExtensionR3Matrix()
    res_placeholder = {
        "n": 1,
        "guard": "test guard substrate",
        "placeholder_metric": 0.5,
    }
    bridge = asi_bridge(set())
    report = ASI5GapExtensionR3Report(
        title="Test Report V1321",
        matrix_md=matrix.render(),
        castoriadis_substrate=res_placeholder,
        fuchs_substrate=res_placeholder,
        brooks_substrate=res_placeholder,
        rorty_substrate=res_placeholder,
        asi_bridge=bridge,
        timestamp="2026-08-08 17:40 +0800",
    )
    md = report.to_markdown()
    assert md.startswith("# ") and "V3 哲学守卫" in md
    assert "20/20" in md or "100%" in md
    assert "Castoriadis" in md
    assert "Fuchs" in md
    assert "Brooks" in md
    assert "Rorty" in md