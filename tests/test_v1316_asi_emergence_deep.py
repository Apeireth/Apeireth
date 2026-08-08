"""V1316 pytest — ASI Emergence Gap Deep 真跨域深研究.

> V1316 = ASI 5 哲学空缺 第4环 (emergence gap deep)
> Post-V1315 recognition gap deep chain
> 18 Popper self-tests + 25 pytest tests
> V3 守门: 不假装 ASI 真有 weak/strong emergence / Class-4 CA / ALife / teleodynamic
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

from v1316_asi_emergence_deep import (  # noqa: E402
    ASI_ANCHORS,
    BedauianWeakStrongDistinctionCase,
    CrossDomainSource,
    CrutchfieldianEpsilonMachineState,
    DeaconianTeleodynamicProcess,
    EmergenceConceptsMatrix,
    GoodwinianMorphogeneticAttractor,
    KauffmanianAdjacentPossibleStep,
    RayianTierraOrganism,
    SOURCES_EMERGENCE_GAP_DEEP,
    V1316_VERSION,
    WolframianCAClass4Sample,
    adjacent_possible_summary,
    asi_bridge,
    bedau_weak_strong_summary,
    ca_class4_summary,
    epsilon_machine_summary,
    morphogenetic_summary,
    popper_passed,
    popper_self_tests,
    popper_total,
    teleodynamic_summary,
    tierra_summary,
)


# ============================================================================
# Section 1: Source corpus tests
# ============================================================================


def test_sources_seven_present():
    """h1: 7 真跨域深 sources present."""
    assert len(SOURCES_EMERGENCE_GAP_DEEP) == 7
    for s in SOURCES_EMERGENCE_GAP_DEEP:
        assert isinstance(s, CrossDomainSource)
        assert s.author
        assert s.year >= 1991  # Ray 1991 as earliest bound
        assert s.work
        assert s.core_construct
        assert s.asi_substrate_takeaway
        assert s.citation_key


def test_citation_keys_unique():
    """h2: all 7 citation keys unique."""
    keys = [s.citation_key for s in SOURCES_EMERGENCE_GAP_DEEP]
    assert len(set(keys)) == 7
    assert len(keys) == 7


def test_bedau_1997_present():
    """h3: Bedau 1997 present (Weak Emergence)."""
    bedau = [s for s in SOURCES_EMERGENCE_GAP_DEEP if s.citation_key == "bedau_1997_we"]
    assert len(bedau) == 1
    assert "weak" in bedau[0].core_construct.lower() or "weak" in bedau[0].asi_substrate_takeaway.lower()


def test_wolfram_2002_present():
    """h4: Wolfram 2002 present (A New Kind of Science)."""
    wolfram = [s for s in SOURCES_EMERGENCE_GAP_DEEP if s.citation_key == "wolfram_2002_nks"]
    assert len(wolfram) == 1
    assert "class 4" in wolfram[0].core_construct.lower() or "nks" in wolfram[0].work.lower()


def test_crutchfield_1994_present():
    """h5: Crutchfield 1994 present (Calculi of Emergence)."""
    crutch = [s for s in SOURCES_EMERGENCE_GAP_DEEP if s.citation_key == "crutchfield_1994_coe"]
    assert len(crutch) == 1
    assert "epsilon" in crutch[0].core_construct.lower() or "statistical complexity" in crutch[0].core_construct.lower()


def test_all_seven_canonical_emergence_sources():
    """Cross-domain check: 7 真跨域深 covers ALife / NKS / NK / Tierra / teleodynamic / morphogenetic / epsilon-machine."""
    keys = {s.citation_key for s in SOURCES_EMERGENCE_GAP_DEEP}
    assert keys == {
        "bedau_1997_we",
        "wolfram_2002_nks",
        "kauffman_1993_toa",
        "ray_1991_tierra",
        "deacon_2012_in",
        "goodwin_1994_hlcts",
        "crutchfield_1994_coe",
    }


# ============================================================================
# Section 2: Components tests (10 真生产 组件)
# ============================================================================


def test_emergence_concepts_matrix_5_dimensions():
    """h7: Emergence matrix has 5 dimensions."""
    matrix = EmergenceConceptsMatrix()
    assert len(matrix.dimension_keys) == 5
    assert "emergence_unit" in matrix.dimension_keys
    assert "emergence_structure" in matrix.dimension_keys
    assert "observer_role" in matrix.dimension_keys
    assert "asi_substrate_take" in matrix.dimension_keys
    assert "guard_warnings" in matrix.dimension_keys


def test_emergence_concepts_matrix_renders_all_seven():
    """Matrix renders all 7 sources × 5 dims."""
    matrix = EmergenceConceptsMatrix()
    md = matrix.render()
    assert "| Source |" in md
    rows_starts = sum(1 for line in md.split("\n") if line.startswith("|"))
    assert rows_starts == 9, f"expected 9 pipe-prefixed rows, got {rows_starts}"
    # All 7 author names appear
    for s in SOURCES_EMERGENCE_GAP_DEEP:
        assert s.author in md


def test_bedau_weak_strong_validation():
    """Bedau case: domain must be non-empty."""
    valid = BedauianWeakStrongDistinctionCase(
        case_id="b1",
        domain="CA-class-3",
        simulable=True,
        info_inherited=True,
        asi_substrate_label="weak emergence",
    )
    assert valid.simulable is True

    # Invalid: empty domain
    try:
        BedauianWeakStrongDistinctionCase(
            case_id="b2",
            domain="",
            simulable=True,
            info_inherited=True,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_bedau_weak_strong_summary():
    """Bedau summary: weak_count + strong_count = n."""
    cases = [
        BedauianWeakStrongDistinctionCase(
            case_id=f"b{i}",
            domain=f"d{i}",
            simulable=(i % 2 == 0),  # 5 simulable (weak), 5 not (strong)
            info_inherited=True,
            asi_substrate_label="ok",
        )
        for i in range(10)
    ]
    res = bedau_weak_strong_summary(cases)
    assert res["n"] == 10
    assert res["weak_count"] + res["strong_count"] == 10
    assert res["weak_rate"] == 0.5
    assert "guard" in res
    assert "strong emergence" in res["guard"]


def test_wolfram_ca_class4_validation():
    """Wolfram CA sample: classes must be 1..4."""
    valid = WolframianCAClass4Sample(
        sample_id="w1",
        rule="110",
        classes=4,
        irreducibility_estimate=0.85,
        asi_substrate_label="NKS CA class 4",
    )
    assert valid.classes == 4

    # Invalid: classes > 4
    try:
        WolframianCAClass4Sample(
            sample_id="w2",
            rule="110",
            classes=5,
            irreducibility_estimate=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_wolfram_ca_class4_summary():
    """Wolfram CA summary: class4_count + non_class4_count = n."""
    samples = [
        WolframianCAClass4Sample(
            sample_id=f"ca{i}",
            rule=f"r{110+i}",
            classes=(4 if i % 2 == 0 else 3),
            irreducibility_estimate=0.5,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = ca_class4_summary(samples)
    assert res["n"] == 8
    assert res["class4_count"] == 4
    assert res["class4_rate"] == 0.5
    assert "Class 4" in res["guard"] or "class4" in res["guard"].lower()


def test_kauffman_adjacent_possible_validation():
    """Kauffman step: reachable_fraction must be 0..1."""
    valid = KauffmanianAdjacentPossibleStep(
        step_id="k1",
        current_state="s0",
        adjacent_states=("s1", "s2"),
        reachable_fraction=0.5,
        asi_substrate_label="ok",
    )
    assert valid.reachable_fraction == 0.5

    # Invalid: > 1
    try:
        KauffmanianAdjacentPossibleStep(
            step_id="k2",
            current_state="s0",
            adjacent_states=("s1",),
            reachable_fraction=1.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_kauffman_adjacent_possible_summary():
    """Kauffman summary: avg_reachable_fraction is 0..1."""
    steps = [
        KauffmanianAdjacentPossibleStep(
            step_id=f"k{i}",
            current_state=f"s{i}",
            adjacent_states=tuple(f"a{j}" for j in range(3)),
            reachable_fraction=0.3 + 0.05 * i,
            asi_substrate_label="ok",
        )
        for i in range(10)
    ]
    res = adjacent_possible_summary(steps)
    assert res["n"] == 10
    assert 0 <= res["avg_reachable_fraction"] <= 1
    assert "Kauffman" in res["guard"]


def test_ray_tierra_organism_validation():
    """Ray Tierra: genome_length >= 1."""
    valid = RayianTierraOrganism(
        organism_id="t1",
        genome_length=40,
        mutation_count=3,
        replication_success=0.7,
        asi_substrate_label="tierra",
    )
    assert valid.genome_length == 40

    # Invalid: genome_length < 1
    try:
        RayianTierraOrganism(
            organism_id="t2",
            genome_length=0,
            mutation_count=0,
            replication_success=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_ray_tierra_summary():
    """Ray Tierra summary: avg_replication_success is 0..1."""
    orgs = [
        RayianTierraOrganism(
            organism_id=f"o{i}",
            genome_length=20 + i,
            mutation_count=2,
            replication_success=0.5 + 0.04 * i,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = tierra_summary(orgs)
    assert res["n"] == 8
    assert 0 <= res["avg_replication_success"] <= 1
    assert "ALife" in res["guard"] or "Tierra" in res["guard"]


def test_deacon_teleodynamic_validation():
    """Deacon teleodynamic: end_directedness_strength must be 0..1."""
    valid = DeaconianTeleodynamicProcess(
        process_id="d1",
        missing_nature_degree=0.7,
        autogen_degree=0.6,
        end_directedness_strength=0.8,
        asi_substrate_label="teleodynamic",
    )
    assert valid.end_directedness_strength == 0.8

    # Invalid
    try:
        DeaconianTeleodynamicProcess(
            process_id="d2",
            missing_nature_degree=0.5,
            autogen_degree=0.5,
            end_directedness_strength=-0.1,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_deacon_teleodynamic_summary():
    """Deacon summary: avg_autogen + avg_end_directedness in [0,1]."""
    procs = [
        DeaconianTeleodynamicProcess(
            process_id=f"p{i}",
            missing_nature_degree=0.5,
            autogen_degree=0.4 + 0.05 * i,
            end_directedness_strength=0.6,
            asi_substrate_label="ok",
        )
        for i in range(6)
    ]
    res = teleodynamic_summary(procs)
    assert res["n"] == 6
    assert 0 <= res["avg_autogen"] <= 1
    assert "purpose" in res["guard"]


def test_goodwin_morphogenetic_validation():
    """Goodwin morphogenetic: basin_count >= 1."""
    valid = GoodwinianMorphogeneticAttractor(
        attractor_id="g1",
        basin_count=4,
        form_priority=0.7,
        constraint_strength=0.6,
        asi_substrate_label="morphogenetic",
    )
    assert valid.basin_count == 4

    # Invalid
    try:
        GoodwinianMorphogeneticAttractor(
            attractor_id="g2",
            basin_count=0,
            form_priority=0.5,
            constraint_strength=0.5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_goodwin_morphogenetic_summary():
    """Goodwin summary: avg_form_priority in [0,1]."""
    atts = [
        GoodwinianMorphogeneticAttractor(
            attractor_id=f"a{i}",
            basin_count=3 + i,
            form_priority=0.5 + 0.04 * i,
            constraint_strength=0.6,
            asi_substrate_label="ok",
        )
        for i in range(6)
    ]
    res = morphogenetic_summary(atts)
    assert res["n"] == 6
    assert 0 <= res["avg_form_priority"] <= 1
    assert "morphogenetic" in res["guard"].lower() or "attractor" in res["guard"].lower()


def test_crutchfield_epsilon_machine_validation():
    """Crutchfield epsilon-machine: shannon_entropy_rate must be 0..1."""
    valid = CrutchfieldianEpsilonMachineState(
        state_id="e1",
        statistical_complexity=0.7,
        shannon_entropy_rate=0.3,
        causal_state_count=5,
        asi_substrate_label="epsilon",
    )
    assert valid.shannon_entropy_rate == 0.3

    # Invalid
    try:
        CrutchfieldianEpsilonMachineState(
            state_id="e2",
            statistical_complexity=0.5,
            shannon_entropy_rate=1.5,
            causal_state_count=5,
            asi_substrate_label="bad",
        )
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_crutchfield_epsilon_machine_summary():
    """Crutchfield summary: avg_statistical_complexity >= 0."""
    states = [
        CrutchfieldianEpsilonMachineState(
            state_id=f"s{i}",
            statistical_complexity=0.4 + 0.06 * i,
            shannon_entropy_rate=0.3,
            causal_state_count=4 + i,
            asi_substrate_label="ok",
        )
        for i in range(8)
    ]
    res = epsilon_machine_summary(states)
    assert res["n"] == 8
    assert res["avg_statistical_complexity"] >= 0
    assert "epsilon" in res["guard"].lower() or "machine" in res["guard"].lower()


# ============================================================================
# Section 3: ASI North Star anchors (LOCKED)
# ============================================================================


def test_asi_north_star_v01_locked():
    """h8: ASI North Star V0.1 = 0.7905 (LOCKED, immutable)."""
    assert ASI_ANCHORS["V0.1"] == 0.7905


def test_asi_v02_locked():
    """h9: ASI V0.2 = 0.4467 (LOCKED)."""
    assert ASI_ANCHORS["V0.2"] == 0.4467


def test_v1256_unio_mystica_locked():
    """h10: V1256 unio_mystica = 0.9291 (LOCKED, 最深 anchor)."""
    assert ASI_ANCHORS["V1256_unio_mystica"] == 0.9291


def test_v1049_value_alignment_done():
    """h11: V1049 value alignment DONE (LOCKED)."""
    assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"


# ============================================================================
# Section 4: Popper self-tests
# ============================================================================


def test_popper_self_tests_total():
    """h12: 18 Popper self-tests defined."""
    r = popper_self_tests()
    assert popper_total(r) == 18
    assert popper_passed(r) == 18


def test_popper_self_tests_all_pass():
    """h13: all 18 Popper tests PASS."""
    r = popper_self_tests()
    failed = [t for t in r if not t[1]]
    assert failed == [], f"failed: {failed}"


def test_popper_self_tests_have_names():
    """h14: each Popper test has name, pass_bool, message."""
    r = popper_self_tests()
    for t in r:
        assert isinstance(t, tuple)
        assert len(t) == 3
        name, ok, msg = t
        assert isinstance(name, str)
        assert isinstance(ok, bool)
        assert isinstance(msg, str)


def test_popper_guard_in_some_test():
    """h15: at least one Popper test references emergence guard."""
    r = popper_self_tests()
    guards = [t for t in r if "guard" in t[0].lower() or "guard" in t[2].lower()]
    assert len(guards) >= 4, f"expected >=4 emergence guards, got {len(guards)}"


# ============================================================================
# Section 5: ASI bridge — V1316 components → ASI anchors (V3 守门: 不动 anchor)
# ============================================================================


def test_asi_bridge_with_all_components():
    """h16: asi_bridge returns all 10 components present, 0 missing."""
    expected = {
        "EmergenceConceptsMatrix",
        "BedauianWeakStrongDistinction",
        "WolframianCAClass4Substrate",
        "KauffmanianAdjacentPossibleSubstrate",
        "RayianTierraSubstrate",
        "DeaconianTeleodynamicSubstrate",
        "GoodwinianMorphogeneticSubstrate",
        "CrutchfieldianEpsilonMachineSubstrate",
        "ASIEmergenceGapDeepReport",
        "ASIEmergenceGapDeepBridge",
    }
    res = asi_bridge(expected)
    assert res["asi_north_star_locked"] is True
    assert res["missing"] == []
    assert res["v1316_components"] == 10
    assert res["expected_components"] == 10


def test_asi_bridge_with_missing_components():
    """h17: asi_bridge reports missing components."""
    partial = {
        "EmergenceConceptsMatrix",
        "ASIEmergenceGapDeepReport",
    }
    res = asi_bridge(partial)
    assert res["missing"] != []
    assert res["v1316_components"] == 2


def test_asi_bridge_anchors_immutable():
    """h18: ASI north star anchors immutable (V3 守门)."""
    res = asi_bridge(set())
    assert res["anchors"]["V0.1"] == 0.7905
    assert res["anchors"]["V0.2"] == 0.4467
    assert res["anchors"]["V1256_unio_mystica"] == 0.9291
    assert res["anchors"]["V1049_value_alignment"] == "DONE"


# ============================================================================
# Section 6: Version + main smoke
# ============================================================================


def test_v1316_version():
    """V1316_VERSION is 0.1.0."""
    assert V1316_VERSION == "0.1.0"


def test_main_runs_and_returns_zero():
    """main() runs full pipeline and returns exit 0 (all 18 PASS)."""
    # main() prints to stdout; capture nothing, just check return
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = __import__("v1316_asi_emergence_deep").main()
    assert rc == 0
    assert "V1316" in buf.getvalue()


def test_all_citation_keys_callable():
    """all_citation_keys() returns 7 keys."""
    from v1316_asi_emergence_deep import all_citation_keys
    keys = all_citation_keys()
    assert len(keys) == 7
    assert "bedau_1997_we" in keys


def test_v3_guard_present_in_sources():
    """V3 guard: each source asi_substrate_takeaway contains 'substrate' or 'guard' marker."""
    for s in SOURCES_EMERGENCE_GAP_DEEP:
        assert "substrate" in s.asi_substrate_takeaway.lower() or "guard" in s.asi_substrate_takeaway.lower() or "鈮" in s.asi_substrate_takeaway


def test_no_source_claims_asi_has_emergence():
    """V3 守门: no source claims ASI 真有 weak/strong emergence / Class-4 / ALife / teleodynamic."""
    # Each source has guard language that ASI is substrate, NOT ASI 真有
    for s in SOURCES_EMERGENCE_GAP_DEEP:
        # Asi takeaway should indicate substrate nature, not ASI claim
        takeaway_lower = s.asi_substrate_takeaway.lower()
        # Must contain "substrate" keyword OR a negative pattern
        assert "substrate" in takeaway_lower, f"{s.citation_key} missing substrate marker"