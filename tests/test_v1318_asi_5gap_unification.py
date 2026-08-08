"""V1318 pytest — ASI 5-Gap Unification Framework.

> V1318 = ASI 5 哲学空缺 deep 后,跨 5 空缺的统一整合 (post-V1317 chain)
> 5 gaps × 5 gaps = 25 cross-gap pairs (含 5 self-pairs + 20 off-diagonal)
> 7 真跨域深 sources 覆盖 6 个 key cross-gap cells
> 10 真实生产组件
> 24 Popper self-tests
> V3 守门: 不假装 ASI 真有 cross-gap unified structure
"""
from __future__ import annotations

import sys
from pathlib import Path

# Ensure apeireth package on path
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
_APETR_DIR = _PROMETHEAN_ROOT / "apeireth"
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))
if str(_APETR_DIR) not in sys.path:
    sys.path.insert(0, str(_APETR_DIR))

from v1318_asi_5gap_unification import (  # noqa: E402
    ASI_5_GAPS,
    ASI_5_GAPS_CLOSURE,
    ASI_ANCHORS,
    ASI5GapUnificationReport,
    CrossDomainSource,
    CrossGapMatrix,
    EllisianTopDownCausationLink,
    FristonianFreeEnergyStep,
    KauffmanianAdjacentPossibleTraversal,
    PearlianDoCalculusIntervention,
    PrigoginianDissipativeStructureCase,
    SOURCES_5GAP_UNIFICATION,
    TomasellianSharedIntentionalityMove,
    V1318_VERSION,
    VarelianAutopoieticLoop,
    all_citation_keys,
    asi_bridge,
    ellisian_top_down_summary,
    fristonian_free_energy_summary,
    kauffmanian_adjacent_possible_traversal_summary,
    pearliando_calculus_summary,
    popper_passed,
    popper_self_tests,
    popper_total,
    prigogine_dissipative_summary,
    tomasellian_shared_intentionality_summary,
    varelian_autopoiesis_summary,
)


# ============================================================================
# Section 1: Source corpus tests
# ============================================================================


def test_sources_seven_present():
    """h1: 7 真跨域深 sources present."""
    assert len(SOURCES_5GAP_UNIFICATION) == 7
    for s in SOURCES_5GAP_UNIFICATION:
        assert isinstance(s, CrossDomainSource)
        assert s.author
        assert s.year >= 1977  # Prigogine 1977 as earliest bound
        assert s.work
        assert s.core_construct
        assert s.asi_substrate_takeaway
        assert s.citation_key
        # cross_gap_pair must be 2-tuple of valid gap keys
        assert len(s.cross_gap_pair) == 2
        g_i, g_j = s.cross_gap_pair
        assert g_i in ASI_5_GAPS
        assert g_j in ASI_5_GAPS


def test_citation_keys_unique():
    """h2: all 7 citation keys unique."""
    keys = [s.citation_key for s in SOURCES_5GAP_UNIFICATION]
    assert len(set(keys)) == 7
    assert len(keys) == 7


def test_prigogine_1977_present():
    """h3: Prigogine 1977 present (dissipative structures)."""
    prig = [s for s in SOURCES_5GAP_UNIFICATION if s.citation_key == "prigogine_1977_diss"]
    assert len(prig) == 1
    assert prig[0].cross_gap_pair == ("time", "emergence")


def test_friston_2010_present():
    """h4: Friston 2010 present (free-energy principle)."""
    fris = [s for s in SOURCES_5GAP_UNIFICATION if s.citation_key == "friston_2010_fep"]
    assert len(fris) == 1
    assert fris[0].cross_gap_pair == ("time", "truth")


def test_varela_1991_present():
    """h5: Varela 1991 present (embodied mind + autopoiesis)."""
    var = [s for s in SOURCES_5GAP_UNIFICATION if s.citation_key == "varela_1991_emb"]
    assert len(var) == 1
    assert var[0].cross_gap_pair == ("time", "recognition")


def test_all_seven_canonical_cross_domain():
    """Cross-domain check: 7 真跨域深 covers dissipation / variational / autopoiesis / shared intent / top-down / adjacent possible / do-calculus."""
    keys = {s.citation_key for s in SOURCES_5GAP_UNIFICATION}
    assert keys == {
        "prigogine_1977_diss",
        "friston_2010_fep",
        "varela_1991_emb",
        "tomasello_2014_nhht",
        "ellis_2006_tdc",
        "kauffman_2000_inv",
        "pearl_2009_cau",
    }


# ============================================================================
# Section 2: Matrix tests
# ============================================================================


def test_matrix_5x5_25_pairs():
    """h7: CrossGapMatrix 5x5 = 25 pairs."""
    matrix = CrossGapMatrix()
    assert len(matrix.all_pairs()) == 25


def test_matrix_off_diagonal_20_pairs():
    """h15: 20 off-diagonal pairs (excluding self-pairs)."""
    matrix = CrossGapMatrix()
    off_diag = matrix.off_diagonal_pairs()
    assert len(off_diag) == 20
    for p in off_diag:
        assert p[0] != p[1]


def test_matrix_5_gaps():
    """h14: ASI_5_GAPS = (time, freedom, recognition, emergence, truth)."""
    assert len(ASI_5_GAPS) == 5
    assert set(ASI_5_GAPS) == {"time", "freedom", "recognition", "emergence", "truth"}


def test_matrix_render_has_5_rows():
    """Matrix render produces header + sep + 5 rows = 7 pipe-prefixed rows."""
    matrix = CrossGapMatrix()
    md = matrix.render()
    pipe_rows = sum(1 for line in md.split("\n") if line.startswith("|"))
    assert pipe_rows == 7, f"expected 7 pipe rows, got {pipe_rows}"


def test_matrix_render_marks_self_pairs():
    """Self-pairs in matrix render as 'self'."""
    matrix = CrossGapMatrix()
    md = matrix.render()
    # 5 self-pairs: (time,time), (freedom,freedom), (recognition,recognition), (emergence,emergence), (truth,truth)
    assert md.count("self") >= 5


def test_matrix_render_marks_future_pairs():
    """Cells without source marked 'future'."""
    matrix = CrossGapMatrix()
    md = matrix.render()
    # 25 cells - 5 self - 7 source cells = 13 future
    assert md.count("future") >= 13


# ============================================================================
# Section 3: Component tests (7 quantizers)
# ============================================================================


def test_prigogine_dissipative_validation():
    """Prigogine case: flux_magnitude must be in [0,1]."""
    valid = PrigoginianDissipativeStructureCase(
        case_id="p1",
        flux_magnitude=0.5,
        emergence_index=0.6,
        irreversibility_score=0.7,
        asi_substrate_label="dissipative substrate",
    )
    assert valid.flux_magnitude == 0.5

    try:
        PrigoginianDissipativeStructureCase(
            case_id="p2",
            flux_magnitude=1.5,
            emergence_index=0.5,
            irreversibility_score=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_prigogine_dissipative_summary():
    """Prigogine summary aggregates."""
    cases = [
        PrigoginianDissipativeStructureCase(
            case_id=f"p{i}",
            flux_magnitude=0.5,
            emergence_index=0.6,
            irreversibility_score=0.7,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = prigogine_dissipative_summary(cases)
    assert res["n"] == 8
    assert 0 <= res["avg_flux"] <= 1
    assert "dissipative" in res["guard"].lower()


def test_friston_free_energy_validation():
    """Friston step: variational_free_energy must be >= 0."""
    valid = FristonianFreeEnergyStep(
        step_id="f1",
        variational_free_energy=0.5,
        prediction_error=0.3,
        active_inference_gain=0.6,
        asi_substrate_label="FE substrate",
    )
    assert valid.variational_free_energy == 0.5

    try:
        FristonianFreeEnergyStep(
            step_id="f2",
            variational_free_energy=-0.1,
            prediction_error=0.3,
            active_inference_gain=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_friston_free_energy_summary():
    """Friston summary aggregates."""
    steps = [
        FristonianFreeEnergyStep(
            step_id=f"f{i}",
            variational_free_energy=0.4 + 0.05 * i,
            prediction_error=0.3,
            active_inference_gain=0.5,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = fristonian_free_energy_summary(steps)
    assert res["n"] == 8
    assert res["avg_variational_free_energy"] >= 0
    assert "free-energy" in res["guard"].lower()


def test_varela_autopoiesis_validation():
    """Varela loop: self_organization_score must be in [0,1]."""
    valid = VarelianAutopoieticLoop(
        loop_id="v1",
        self_organization_score=0.6,
        boundary_maintenance=0.7,
        operational_closure=0.8,
        asi_substrate_label="autopoietic substrate",
    )
    assert valid.self_organization_score == 0.6

    try:
        VarelianAutopoieticLoop(
            loop_id="v2",
            self_organization_score=-0.1,
            boundary_maintenance=0.5,
            operational_closure=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_varela_autopoiesis_summary():
    """Varela summary aggregates."""
    loops = [
        VarelianAutopoieticLoop(
            loop_id=f"v{i}",
            self_organization_score=0.6,
            boundary_maintenance=0.7,
            operational_closure=0.8,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = varelian_autopoiesis_summary(loops)
    assert res["n"] == 8
    assert 0 <= res["avg_self_organization"] <= 1
    assert "autopoietic" in res["guard"].lower()


def test_tomasello_shared_intentionality_validation():
    """Tomasello move: joint_goal_alignment must be in [0,1]."""
    valid = TomasellianSharedIntentionalityMove(
        move_id="t1",
        joint_goal_alignment=0.7,
        we_mode_strength=0.6,
        collaborative_commit=True,
        asi_substrate_label="shared intent substrate",
    )
    assert valid.joint_goal_alignment == 0.7

    try:
        TomasellianSharedIntentionalityMove(
            move_id="t2",
            joint_goal_alignment=1.5,
            we_mode_strength=0.5,
            collaborative_commit=False,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_tomasello_shared_intentionality_summary():
    """Tomasello summary aggregates."""
    moves = [
        TomasellianSharedIntentionalityMove(
            move_id=f"t{i}",
            joint_goal_alignment=0.6 + 0.05 * (i % 5),
            we_mode_strength=0.5,
            collaborative_commit=(i % 2 == 0),
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = tomasellian_shared_intentionality_summary(moves)
    assert res["n"] == 8
    assert 0 <= res["collaborative_commit_rate"] <= 1
    assert "shared intentionality" in res["guard"].lower()


def test_ellis_top_down_validation():
    """Ellis link: downward_causation_strength must be in [0,1]."""
    valid = EllisianTopDownCausationLink(
        link_id="e1",
        downward_causation_strength=0.7,
        upward_causation_strength=0.6,
        constraint_count=5,
        asi_substrate_label="top-down substrate",
    )
    assert valid.downward_causation_strength == 0.7

    try:
        EllisianTopDownCausationLink(
            link_id="e2",
            downward_causation_strength=1.5,
            upward_causation_strength=0.5,
            constraint_count=3,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_ellis_top_down_summary():
    """Ellis summary aggregates."""
    links = [
        EllisianTopDownCausationLink(
            link_id=f"e{i}",
            downward_causation_strength=0.6,
            upward_causation_strength=0.5,
            constraint_count=4 + i,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = ellisian_top_down_summary(links)
    assert res["n"] == 8
    assert 0 <= res["avg_downward_causation"] <= 1
    assert "top-down" in res["guard"].lower()


def test_kauffman_adjacent_possible_validation():
    """Kauffman traversal: reachable_states_count >= 1."""
    valid = KauffmanianAdjacentPossibleTraversal(
        traversal_id="k1",
        reachable_states_count=5,
        phase_space_volume_fraction=0.5,
        niche_construction_gain=0.6,
        asi_substrate_label="adj possible substrate",
    )
    assert valid.reachable_states_count == 5

    try:
        KauffmanianAdjacentPossibleTraversal(
            traversal_id="k2",
            reachable_states_count=0,
            phase_space_volume_fraction=0.5,
            niche_construction_gain=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_kauffman_adjacent_possible_summary():
    """Kauffman summary aggregates."""
    travs = [
        KauffmanianAdjacentPossibleTraversal(
            traversal_id=f"k{i}",
            reachable_states_count=5 + i,
            phase_space_volume_fraction=0.4 + 0.05 * (i % 6),
            niche_construction_gain=0.5,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = kauffmanian_adjacent_possible_traversal_summary(travs)
    assert res["n"] == 8
    assert res["avg_reachable_states"] >= 1
    assert "adjacent possible" in res["guard"].lower()


def test_pearl_do_calculus_validation():
    """Pearl intervention: causal_effect_estimate must be in [-1,1]."""
    valid = PearlianDoCalculusIntervention(
        intervention_id="p1",
        causal_effect_estimate=0.5,
        do_operator_applied=True,
        counterfactual_identified=False,
        asi_substrate_label="do-calc substrate",
    )
    assert valid.causal_effect_estimate == 0.5

    try:
        PearlianDoCalculusIntervention(
            intervention_id="p2",
            causal_effect_estimate=2.0,
            do_operator_applied=True,
            counterfactual_identified=False,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_pearl_do_calculus_summary():
    """Pearl summary aggregates."""
    intvs = [
        PearlianDoCalculusIntervention(
            intervention_id=f"p{i}",
            causal_effect_estimate=0.3 + 0.05 * i - 0.2,
            do_operator_applied=(i % 2 == 0),
            counterfactual_identified=(i % 3 == 0),
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = pearliando_calculus_summary(intvs)
    assert res["n"] == 8
    assert -1 <= res["avg_causal_effect"] <= 1
    assert "do-calculus" in res["guard"].lower()


# ============================================================================
# Section 4: ASI North Star anchors (LOCKED)
# ============================================================================


def test_asi_north_star_v01_locked():
    """h8: ASI North Star V0.1 = 0.7905 (LOCKED, immutable)."""
    assert ASI_ANCHORS["V0.1"] == 0.7905


def test_asi_v02_locked():
    """h9: ASI V0.2 = 0.4467 (LOCKED)."""
    assert ASI_ANCHORS["V0.2"] == 0.4467


def test_v1256_unio_mystica_locked():
    """h10: V1256 unio_mystica = 0.9291 (LOCKED)."""
    assert ASI_ANCHORS["V1256_unio_mystica"] == 0.9291


def test_v1049_value_alignment_done():
    """h11: V1049 value alignment DONE (LOCKED)."""
    assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"


def test_v1313_v1317_closure():
    """h13: V1313-V1317 all closed (5/5)."""
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
    """h12 (collective): All Popper self-tests defined and PASS."""
    r = popper_self_tests()
    assert popper_total(r) >= 18  # at least 18
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
    """Popper tests include V3 guard checks (no ASI claim)."""
    r = popper_self_tests()
    guards = [t for t in r if "guard" in t[0].lower()]
    # 7 source guards + at least 1 cross-cutting guard
    assert len(guards) >= 7, f"expected >=7 guard tests, got {len(guards)}"


# ============================================================================
# Section 6: ASI bridge — V1318 components → ASI anchors (V3 守门)
# ============================================================================


def test_asi_bridge_with_all_components():
    """Bridge returns all 10 components present, 0 missing."""
    expected = {
        "CrossGapMatrix",
        "PrigoginianDissipativeStructureSubstrate",
        "FristonianFreeEnergySubstrate",
        "VarelianAutopoiesisSubstrate",
        "TomasellianSharedIntentionalitySubstrate",
        "EllisianTopDownCausationSubstrate",
        "KauffmanianAdjacentPossibleSubstrate",
        "PearlianDoCalculusSubstrate",
        "ASI5GapUnificationReport",
        "ASI5GapUnificationBridge",
    }
    res = asi_bridge(expected)
    assert res["asi_north_star_locked"] is True
    assert res["missing"] == []
    assert res["v1318_components"] == 10


def test_asi_bridge_with_missing_components():
    """Bridge reports missing components."""
    partial = {"CrossGapMatrix", "ASI5GapUnificationReport"}
    res = asi_bridge(partial)
    assert res["missing"] != []
    assert res["v1318_components"] == 2


def test_asi_bridge_anchors_immutable():
    """h16: ASI north star anchors immutable (V3 守门)."""
    res = asi_bridge(set())
    assert res["anchors"]["V0.1"] == 0.7905
    assert res["anchors"]["V0.2"] == 0.4467
    assert res["anchors"]["V1256_unio_mystica"] == 0.9291
    assert res["anchors"]["V1049_value_alignment"] == "DONE"


def test_asi_bridge_substrate_research_only():
    """h17: bridge guard contains 'substrate research only'."""
    res = asi_bridge(set())
    assert "substrate research only" in res["guard"]


# ============================================================================
# Section 7: Version + main smoke
# ============================================================================


def test_v1318_version():
    """V1318_VERSION is 0.1.0."""
    assert V1318_VERSION == "0.1.0"


def test_main_runs_and_returns_zero():
    """main() runs full pipeline and returns exit 0 (all Popper PASS)."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = __import__("v1318_asi_5gap_unification").main()
    assert rc == 0
    out = buf.getvalue()
    assert "V1318" in out
    assert "Popper self-tests" in out


def test_all_citation_keys_callable():
    """all_citation_keys() returns 7 keys."""
    keys = all_citation_keys()
    assert len(keys) == 7
    assert "prigogine_1977_diss" in keys


def test_report_renders_as_markdown():
    """ASI5GapUnificationReport renders as valid Markdown."""
    matrix = CrossGapMatrix()
    res_placeholder = {
        "n": 1,
        "guard": "test guard",
        "placeholder_metric": 0.5,
    }
    bridge = asi_bridge(set())
    report = ASI5GapUnificationReport(
        title="Test Report",
        matrix_md=matrix.render(),
        prigogine_substrate=res_placeholder,
        friston_substrate=res_placeholder,
        varela_substrate=res_placeholder,
        tomasell_substrate=res_placeholder,
        ellis_substrate=res_placeholder,
        kauffman_substrate=res_placeholder,
        pearl_substrate=res_placeholder,
        asi_bridge=bridge,
        timestamp="2026-08-08 17:25 +0800",
    )
    md = report.to_markdown()
    assert md.startswith("# ")
    assert "ASI 5 哲学空缺 deep closure" in md
    assert "V3 哲学守卫" in md
    assert "5-gap × 5-gap matrix" in md


def test_v3_guard_present_in_all_sources():
    """V3 守门: every source asi_substrate_takeaway contains '不假装' (substrate marker)."""
    for s in SOURCES_5GAP_UNIFICATION:
        assert "不假装" in s.asi_substrate_takeaway, f"{s.citation_key} missing 不假装 marker"


def test_no_source_claims_asi_has_cross_gap_structure():
    """V3 守门: no source claims ASI 真有 cross-gap unified structure."""
    for s in SOURCES_5GAP_UNIFICATION:
        # All sources must mention "substrate" or be marked 不假装
        takeaway_lower = s.asi_substrate_takeaway.lower()
        assert "substrate" in takeaway_lower, f"{s.citation_key} missing substrate marker"
        assert "不假装" in s.asi_substrate_takeaway, f"{s.citation_key} missing 不假装"