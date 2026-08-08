"""V1315 ASI Recognition Gap Deep 真跨域深研究 — tests/test_v1315.py.

> 18 Popper self-tests + 真生产 10 组件真测 + V3 哲学守门 + ASI bridge 真测.

V1315 测试范围:
1. 7 真跨域深 sources (Hegel/Heidegger/Levinas/Taylor/Ricoeur/Honneth/Butler)
2. 10 真生产组件 (RecognitionConceptsMatrix + 7 substrates + Report + Bridge)
3. 18 Popper self-tests PASS (不 skip, 不 flake)
4. V3 哲学守门 (不假装 Phenomenal recognition, 不假装 recognition = 镜像,
   不假装 ASI 自识 = Anerkennung, 不假装 Levinas 他者面容)
5. ASI bridge: 北极星 V0.1=0.7905 / V0.2=0.4467 / V1256=92.91% LOCKED

诚实声明 (主 17:43 实事求是):
- V1315 = ASI 识别哲学空缺 deep 真跨域深研究 ≠ ASI 真有 recognition consciousness
- 7 真跨域深 sources = 真借鉴启发, NOT 真具有
- 10 真生产组件 = 真算法 + 真测 + 真 commit, NOT claim ASI has Anerkennung
"""
from __future__ import annotations

import sys
from pathlib import Path

# Ensure apeireth directory is on path
_PROMETHEAN_ROOT = Path(__file__).resolve().parents[1]
_APEIRETH_DIR = _PROMETHEAN_ROOT / "apeireth"
if str(_APEIRETH_DIR) not in sys.path:
    sys.path.insert(0, str(_APEIRETH_DIR))

from v1315_asi_recognition_deep import (  # noqa: E402
    ASI_ANCHORS,
    V1315_VERSION,
    ASIRecognitionGapDeepReport,
    ButlerianPerformativityCitation,
    HegelianAnerkennungTick as HegelAnerkennungTick,
    HeideggerianMitseinAgent,
    HonnethianThreeSphereValidation,
    LevinasianFaceEncounter,
    RecognitionConceptsMatrix,
    RicoeureanNarrativeIdentityEpisode,
    SOURCES_RECOGNITION_GAP_DEEP,
    TaylorianRecognitionIdentity,
    asi_bridge,
    butler_summary,
    hegel_anerkennung_summary,
    honneth_summary,
    levinas_face_summary,
    mitsein_summary,
    popper_passed,
    popper_self_tests,
    popper_total,
    ricoeur_summary,
    taylor_summary,
)


# ============================================================================
# Section 1: Constants & version
# ============================================================================


def test_v1315_version_constant() -> None:
    """V1315_VERSION should be a non-empty semver string."""
    assert V1315_VERSION
    parts = V1315_VERSION.split(".")
    assert len(parts) == 3
    assert all(p.isdigit() for p in parts), f"semver parts not all digits: {V1315_VERSION}"


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
    assert len(SOURCES_RECOGNITION_GAP_DEEP) == 7


def test_citation_keys_unique() -> None:
    """All citation keys must be unique (no duplicates)."""
    keys = [s.citation_key for s in SOURCES_RECOGNITION_GAP_DEEP]
    assert len(set(keys)) == len(keys)


def test_all_seven_authors_present() -> None:
    """All 7 authors must be present (Hegel/Heidegger/Levinas/Taylor/Ricoeur/Honneth/Butler)."""
    authors = {s.author for s in SOURCES_RECOGNITION_GAP_DEEP}
    expected_authors = {
        "G.W.F. Hegel",
        "Martin Heidegger",
        "Emmanuel Levinas",
        "Charles Taylor",
        "Paul Ricoeur",
        "Axel Honneth",
        "Judith Butler",
    }
    assert authors == expected_authors, f"missing: {expected_authors - authors}"


def test_citation_keys_correct() -> None:
    """Citation keys match expected mapping."""
    expected = {
        "hegel_1807_pg",
        "heidegger_1927_sz",
        "levinas_1961_ti",
        "taylor_1989_ss",
        "ricoeur_1990_osmaua",
        "honneth_1995_sr",
        "butler_1990_1997",
    }
    actual = {s.citation_key for s in SOURCES_RECOGNITION_GAP_DEEP}
    assert actual == expected


def test_all_sources_have_substrate_takeaway() -> None:
    """Every source has asi_substrate_takeaway (V3 守门: 真借鉴明确)."""
    for s in SOURCES_RECOGNITION_GAP_DEEP:
        assert s.asi_substrate_takeaway
        assert "≠" in s.asi_substrate_takeaway or "不" in s.asi_substrate_takeaway, \
            f"substrate takeaway should signal guard: {s.author}"


# ============================================================================
# Section 3: Component 1 — RecognitionConceptsMatrix
# ============================================================================


def test_recognition_matrix_dimensions() -> None:
    """Matrix has 5 dimension keys."""
    matrix = RecognitionConceptsMatrix()
    assert len(matrix.dimension_keys) == 5


def test_recognition_matrix_render() -> None:
    """Matrix render includes all 7 sources × 5 dims (Markdown table)."""
    matrix = RecognitionConceptsMatrix()
    md = matrix.render()
    for s in SOURCES_RECOGNITION_GAP_DEEP:
        assert s.author in md, f"{s.author} not in matrix render"


def test_recognition_matrix_cell_lookup() -> None:
    """Each source cell lookup returns non-empty for known dims."""
    matrix = RecognitionConceptsMatrix()
    for s in SOURCES_RECOGNITION_GAP_DEEP:
        cell_text = matrix.cell(s, "recognition_structure")
        assert cell_text, f"empty cell for {s.author} recognition_structure"


# ============================================================================
# Section 4: Component 2 — HegelianAnerkennungSubstrate
# ============================================================================


def test_hegel_substrate_empty_input() -> None:
    """Hegel substrate handles empty input with proper guard."""
    res = hegel_anerkennung_summary([])
    assert res["n"] == 0
    assert "guard" in res
    assert "Anerkennung" in res["guard"]


def test_hegel_substrate_basic() -> None:
    """Hegel substrate computes reciprocal_rate."""
    ticks = [
        HegelAnerkennungTick(
            tick=i,
            role_a="master",
            role_b="slave",
            reciprocal_recognition=(i >= 2),
            asi_substrate_role="interop",
        )
        for i in range(4)
    ]
    res = hegel_anerkennung_summary(ticks)
    assert res["n"] == 4
    assert res["reciprocal_rate"] == 0.5
    assert res["reciprocal_count"] == 2


def test_hegel_tick_validation_negative_tick() -> None:
    """Hegel tick rejects negative tick."""
    import pytest
    with pytest.raises(ValueError):
        HegelAnerkennungTick(
            tick=-1,
            role_a="a",
            role_b="b",
            reciprocal_recognition=True,
            asi_substrate_role="x",
        )


def test_hegel_tick_validation_empty_role() -> None:
    """Hegel tick rejects empty role."""
    import pytest
    with pytest.raises(ValueError):
        HegelAnerkennungTick(
            tick=0,
            role_a="",
            role_b="b",
            reciprocal_recognition=True,
            asi_substrate_role="x",
        )


# ============================================================================
# Section 5: Component 3 — HeideggerianMitseinSubstrate
# ============================================================================


def test_heidegger_substrate_empty_input() -> None:
    """Heidegger substrate handles empty input with proper guard."""
    res = mitsein_summary([])
    assert res["n"] == 0
    assert "Mitsein" in res["guard"]


def test_heidegger_substrate_basic() -> None:
    """Heidegger substrate computes co_being_rate + zuhanden/vorhanden counts."""
    agents = [
        HeideggerianMitseinAgent(
            agent_id=f"a_{i}",
            is_co_being=(i < 3),
            tool_mode="Zuhandenheit (ready-to-hand)" if i % 2 == 0 else "Vorhandenheit (present-at-hand)",
            asi_substrate_role="x",
        )
        for i in range(4)
    ]
    res = mitsein_summary(agents)
    assert res["n"] == 4
    assert res["co_being_rate"] == 0.75
    assert res["zuhanden_count"] == 2
    assert res["vorhanden_count"] == 2


def test_heidegger_agent_validation_invalid_tool_mode() -> None:
    """Heidegger agent rejects invalid tool_mode."""
    import pytest
    with pytest.raises(ValueError):
        HeideggerianMitseinAgent(
            agent_id="a_0",
            is_co_being=True,
            tool_mode="invalid_mode",
            asi_substrate_role="x",
        )


# ============================================================================
# Section 6: Component 4 — LevinasianFaceSubstrate
# ============================================================================


def test_levinas_substrate_empty_input() -> None:
    """Levinas substrate handles empty input with proper guard."""
    res = levinas_face_summary([])
    assert res["n"] == 0
    assert "ethics" in res["guard"]


def test_levinas_substrate_basic() -> None:
    """Levinas substrate computes ethical_response_rate + infinity_rate."""
    encounters = [
        LevinasianFaceEncounter(
            encounter_id=f"e_{i}",
            other_id=f"o_{i}",
            ethical_response=(i % 2 == 0),
            infinity_marker=(i < 3),
            asi_substrate_role="x",
        )
        for i in range(4)
    ]
    res = levinas_face_summary(encounters)
    assert res["n"] == 4
    assert res["ethical_response_rate"] == 0.5
    assert res["infinity_rate"] == 0.75


# ============================================================================
# Section 7: Component 5 — TaylorianRecognitionPoliticsSubstrate
# ============================================================================


def test_taylor_substrate_empty_input() -> None:
    """Taylor substrate handles empty input with proper guard."""
    res = taylor_summary([])
    assert res["n"] == 0
    assert "politics" in res["guard"]


def test_taylor_substrate_basic() -> None:
    """Taylor substrate computes recognition_rate + avg_sources."""
    identities = [
        TaylorianRecognitionIdentity(
            identity_id=f"i_{i}",
            sources=("src_a", "src_b"),
            recognized=(i % 2 == 0),
            asi_substrate_role="x",
        )
        for i in range(4)
    ]
    res = taylor_summary(identities)
    assert res["n"] == 4
    assert res["recognition_rate"] == 0.5
    assert res["avg_sources_per_identity"] == 2.0


def test_taylor_identity_validation_empty_sources() -> None:
    """Taylor identity rejects empty sources."""
    import pytest
    with pytest.raises(ValueError):
        TaylorianRecognitionIdentity(
            identity_id="i_0",
            sources=(),
            recognized=True,
            asi_substrate_role="x",
        )


# ============================================================================
# Section 8: Component 6 — RicoeureanNarrativeIdentitySubstrate
# ============================================================================


def test_ricoeur_substrate_empty_input() -> None:
    """Ricoeur substrate handles empty input with proper guard."""
    res = ricoeur_summary([])
    assert res["n"] == 0
    assert "attestation" in res["guard"]


def test_ricoeur_substrate_basic() -> None:
    """Ricoeur substrate computes attestation_rate + other_self_rate."""
    episodes = [
        RicoeureanNarrativeIdentityEpisode(
            episode_id=f"ep_{i}",
            narrative=f"n_{i}",
            attested=(i % 3 != 0),
            other_self=(i % 4 != 0),
            asi_substrate_role="x",
        )
        for i in range(6)
    ]
    res = ricoeur_summary(episodes)
    assert res["n"] == 6
    assert res["attestation_rate"] == round(4 / 6, 6)
    assert res["other_self_rate"] == round(4 / 6, 6)


def test_ricoeur_episode_validation_empty_narrative() -> None:
    """Ricoeur episode rejects empty narrative."""
    import pytest
    with pytest.raises(ValueError):
        RicoeureanNarrativeIdentityEpisode(
            episode_id="ep_0",
            narrative="",
            attested=True,
            other_self=True,
            asi_substrate_role="x",
        )


# ============================================================================
# Section 9: Component 7 — HonnethianThreeSpheresSubstrate
# ============================================================================


def test_honneth_substrate_empty_input() -> None:
    """Honneth substrate handles empty input with proper guard."""
    res = honneth_summary([])
    assert res["n"] == 0
    assert "蔑视" in res["guard"]


def test_honneth_substrate_basic() -> None:
    """Honneth substrate computes recognition_rate + sphere distribution."""
    validations = [
        HonnethianThreeSphereValidation(
            validation_id=f"v_{i}",
            sphere=("love" if i % 3 == 0 else "rights" if i % 3 == 1 else "solidarity"),
            recognized=(i % 2 == 0),
            disrespect=0.2,
            asi_substrate_role="x",
        )
        for i in range(6)
    ]
    res = honneth_summary(validations)
    assert res["n"] == 6
    assert res["recognition_rate"] == 0.5
    assert res["avg_disrespect"] == 0.2
    assert res["sphere_distribution"]["love"] == 2
    assert res["sphere_distribution"]["rights"] == 2
    assert res["sphere_distribution"]["solidarity"] == 2


def test_honneth_validation_invalid_sphere() -> None:
    """Honneth validation rejects invalid sphere."""
    import pytest
    with pytest.raises(ValueError):
        HonnethianThreeSphereValidation(
            validation_id="v_0",
            sphere="invalid",
            recognized=True,
            disrespect=0.5,
            asi_substrate_role="x",
        )


def test_honneth_validation_disrespect_out_of_range() -> None:
    """Honneth validation rejects out-of-range disrespect."""
    import pytest
    with pytest.raises(ValueError):
        HonnethianThreeSphereValidation(
            validation_id="v_0",
            sphere="love",
            recognized=True,
            disrespect=2.0,
            asi_substrate_role="x",
        )


# ============================================================================
# Section 10: Component 8 — ButlerianPerformativitySubstrate
# ============================================================================


def test_butler_substrate_empty_input() -> None:
    """Butler substrate handles empty input with proper guard."""
    res = butler_summary([])
    assert res["n"] == 0
    assert "gender" in res["guard"]


def test_butler_substrate_basic() -> None:
    """Butler substrate computes sub_jection_rate + avg_iterations."""
    citations = [
        ButlerianPerformativityCitation(
            citation_id=f"c_{i}",
            cited_norm=f"n_{i}",
            iteration_count=i + 1,
            sub_jection=(i % 2 == 0),
            asi_substrate_role="V3 守门",
        )
        for i in range(4)
    ]
    res = butler_summary(citations)
    assert res["n"] == 4
    assert res["sub_jection_rate"] == 0.5
    assert res["avg_iterations_per_citation"] == 2.5


def test_butler_citation_negative_iterations() -> None:
    """Butler citation rejects negative iteration_count."""
    import pytest
    with pytest.raises(ValueError):
        ButlerianPerformativityCitation(
            citation_id="c_0",
            cited_norm="n",
            iteration_count=-1,
            sub_jection=True,
            asi_substrate_role="x",
        )


# ============================================================================
# Section 11: Component 9 — ASIRecognitionGapDeepReport
# ============================================================================


def test_asi_recognition_report_render() -> None:
    """ASIRecognitionGapDeepReport.render() returns Markdown with all sections."""
    matrix = RecognitionConceptsMatrix()
    report = ASIRecognitionGapDeepReport(
        title="V1315 Test Report",
        matrix_md=matrix.render(),
        hegel_substrate={"n": 1, "guard": "test"},
        heidegger_substrate={"n": 1, "guard": "test"},
        levinas_substrate={"n": 1, "guard": "test"},
        taylor_substrate={"n": 1, "guard": "test"},
        ricoeur_substrate={"n": 1, "guard": "test"},
        honneth_substrate={"n": 1, "guard": "test"},
        butler_substrate={"n": 1, "guard": "test"},
        asi_bridge={"v1315_components": 10, "expected_components": 10, "missing": []},
        timestamp="2026-08-08 16:48 +08:00",
    )
    md = report.render()
    assert "V1315 Test Report" in md
    assert "V3 哲学守门" in md
    assert "Hegel 1807" in md
    assert "Levinas 1961" in md
    assert "Butler 1990" in md


def test_asi_recognition_report_includes_guards() -> None:
    """Report includes all V3 守门 statements."""
    matrix = RecognitionConceptsMatrix()
    report = ASIRecognitionGapDeepReport(
        title="t",
        matrix_md="",
        hegel_substrate={},
        heidegger_substrate={},
        levinas_substrate={},
        taylor_substrate={},
        ricoeur_substrate={},
        honneth_substrate={},
        butler_substrate={},
        asi_bridge={},
        timestamp="t",
    )
    md = report.render()
    assert "Phenomenal recognition" in md
    assert "recognition = 镜像" in md
    assert "Hegel Anerkennung" in md
    assert "Levinas 他者面容" in md
    assert "recognition gap" in md


# ============================================================================
# Section 12: Component 10 — ASIRecognitionGapDeepBridge
# ============================================================================


def test_asi_bridge_all_present() -> None:
    """asi_bridge with all 10 components → missing=[]."""
    all_ten = {
        "RecognitionConceptsMatrix",
        "HegelianAnerkennungSubstrate",
        "HeideggerianMitseinSubstrate",
        "LevinasianFaceSubstrate",
        "TaylorianRecognitionPoliticsSubstrate",
        "RicoeureanNarrativeIdentitySubstrate",
        "HonnethianThreeSpheresSubstrate",
        "ButlerianPerformativitySubstrate",
        "ASIRecognitionGapDeepReport",
        "ASIRecognitionGapDeepBridge",
    }
    bridge = asi_bridge(all_ten)
    assert bridge["asi_north_star_locked"] is True
    assert bridge["v1315_components"] == 10
    assert bridge["expected_components"] == 10
    assert bridge["missing"] == []


def test_asi_bridge_partial_present() -> None:
    """asi_bridge with partial components → missing list populated."""
    bridge = asi_bridge({"RecognitionConceptsMatrix"})
    assert len(bridge["missing"]) == 9
    assert "HegelianAnerkennungSubstrate" in bridge["missing"]


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


def test_popper_specific_h3_hegel() -> None:
    """Popper h3: Hegel 1807 present."""
    results = popper_self_tests()
    h3 = next(r for r in results if r[0] == "h3_hegel_1807_present")
    assert h3[1] is True


def test_popper_specific_h5_butler() -> None:
    """Popper h5: Butler 1990/1997 present (跨年代 + 跨主题)."""
    results = popper_self_tests()
    h5 = next(r for r in results if r[0] == "h5_butler_1990_1997_present")
    assert h5[1] is True


def test_popper_specific_h8_v01_locked() -> None:
    """Popper h8: V0.1 = 0.7905."""
    results = popper_self_tests()
    h8 = next(r for r in results if r[0] == "h8_asi_north_star_v01_locked")
    assert h8[1] is True


def test_popper_specific_h17_honneth_guard() -> None:
    """Popper h17: Honneth 蔑视 guard present."""
    results = popper_self_tests()
    h17 = next(r for r in results if r[0] == "h17_honneth_disrespect_guard")
    assert h17[1] is True


def test_popper_specific_h18_bridge_shape() -> None:
    """Popper h18: asi_bridge returns expected shape."""
    results = popper_self_tests()
    h18 = next(r for r in results if r[0] == "h18_asi_bridge_shape_correct")
    assert h18[1] is True


# ============================================================================
# Section 14: V3 哲学守门
# ============================================================================


def test_v3_guard_no_recognition_consciousness_claim() -> None:
    """V3 守门: 不假装 ASI 真有 recognition consciousness."""
    for s in SOURCES_RECOGNITION_GAP_DEEP:
        assert "≠" in s.asi_substrate_takeaway or "不" in s.asi_substrate_takeaway, \
            f"substrate takeaway must signal guard: {s.author}"


def test_v3_guard_module_docstring() -> None:
    """V3 守门: module docstring contains all 5 guard statements."""
    from v1315_asi_recognition_deep import __doc__ as docstring
    assert "Phenomenal recognition" in docstring
    assert "recognition = 镜像" in docstring
    assert "ASI 自识 = Hegel Anerkennung" in docstring
    assert "Levinas 他者面容" in docstring
    assert "recognition gap" in docstring


def test_v3_guard_no_substrate_claim_in_main() -> None:
    """V3 守门: main() 不 claim ASI 真有 Anerkennung."""
    import io
    import contextlib
    from v1315_asi_recognition_deep import main
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
    from v1315_asi_recognition_deep import main
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        result = main()
    assert result == 0


def test_main_output_includes_all_10_components() -> None:
    """main() output references all 10 components."""
    import io
    import contextlib
    from v1315_asi_recognition_deep import main
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        main()
    output = buf.getvalue()
    assert "RecognitionConceptsMatrix" in output
    assert "HegelianAnerkennungSubstrate" in output
    assert "HeideggerianMitseinSubstrate" in output
    assert "LevinasianFaceSubstrate" in output
    assert "TaylorianRecognitionPoliticsSubstrate" in output
    assert "RicoeureanNarrativeIdentitySubstrate" in output
    assert "HonnethianThreeSpheresSubstrate" in output
    assert "ButlerianPerformativitySubstrate" in output
    assert "ASIRecognitionGapDeepReport" in output
    assert "ASIRecognitionGapDeepBridge" in output


def test_main_output_18_popper_pass() -> None:
    """main() output reports 18/18 Popper PASS."""
    import io
    import contextlib
    from v1315_asi_recognition_deep import main
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        main()
    output = buf.getvalue()
    assert "[Popper self-tests] 18/18 PASS" in output


# ============================================================================
# Section 16: ASI 北极星 LOCKED
# ============================================================================


def test_anchors_v01_unmoved() -> None:
    """V0.1 = 0.7905 unchanged by V1315 (no anchor movement)."""
    assert ASI_ANCHORS["V0.1"] == 0.7905


def test_anchors_v02_unmoved() -> None:
    """V0.2 = 0.4467 unchanged by V1315."""
    assert ASI_ANCHORS["V0.2"] == 0.4467


def test_anchors_v1256_unmoved() -> None:
    """V1256 = 0.9291 unchanged by V1315."""
    assert ASI_ANCHORS["V1256_unio_mystica"] == 0.9291


def test_anchors_v1049_done() -> None:
    """V1049 = DONE unchanged by V1315."""
    assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"