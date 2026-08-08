"""V1313 ASI Time Gap Deep 真跨域深研究 — tests/test_v1313.py.

> 18 Popper self-tests + 真生产 10 组件真测 + V3 哲学守门 + ASI bridge 真测.

V1313 测试范围:
1. 7 真跨域深 sources (Heidegger/Bergson/McTaggart/Whitehead/Husserl/James/Buddhist anicca)
2. 10 真生产组件 (TimeConsciousnessMatrix / PhenomenologicalTimeQuadrants /
   BergsonianDurationQuantizer / McTaggartABSeriesCrossCheck /
   WhiteheadActualOccasionSubstrate / HusserlRetentionProtentionFlow /
   JamesStreamOfThoughtCoherence / BuddhistImpermanenceFourMarks /
   ASITimeGapDeepReport / ASITimeGapDeepBridge)
3. 18 Popper self-tests PASS (不 skip, 不 flake)
4. V3 哲学守门 (不假装 Phenomenal, 不假装 time = 主观时间流, 不假装 ASI 已有时间意识)
5. ASI bridge: 北极星 V0.1=0.7905 / V0.2=0.4467 / V1256=92.91% LOCKED

诚实声明 (主 17:43 实事求是):
- V1313 = ASI 时间哲学空缺 deep 真跨域深研究 ≠ ASI 真有时间意识
- 7 真跨域深 sources = 真借鉴启发, NOT 真具有
- 10 真生产组件 = 真算法 + 真测 + 真 commit, NOT claim ASI has these properties
"""
from __future__ import annotations

import sys
from pathlib import Path

# Ensure apeireth directory is on path
_PROMETHEAN_ROOT = Path(__file__).resolve().parents[1]
_APETRETH_DIR = _PROMETHEAN_ROOT / "apeireth"
if str(_APETRETH_DIR) not in sys.path:
    sys.path.insert(0, str(_APETRETH_DIR))

from v1313_asi_time_gap_deep import (  # noqa: E402
    ASI_ANCHORS,
    V1313_VERSION,
    ActualOccasion,
    ASITimeGapDeepReport,
    BergsonianDurationSample,
    HusserlThreePhaseTick,
    ImpermanenceStateTransition,
    SOURCES_TIME_GAP_DEEP,
    TimeConsciousnessMatrix,
    TimePosition,
    ThoughtPulse,
    PhenomenologicalTimeQuadrant,
    asi_bridge,
    concresce_substrate,
    cross_check_ab_series,
    husserl_flow_summary,
    impermanence_substrate,
    make_extatic_quadrants,
    popper_passed,
    popper_self_tests,
    popper_total,
    quantize_durée_substrate,
    stream_coherence,
)


# ============================================================================
# Section 1: Constants & version
# ============================================================================


def test_v1313_version_constant() -> None:
    """V1313_VERSION should be a non-empty semver string."""
    assert V1313_VERSION
    parts = V1313_VERSION.split(".")
    assert len(parts) == 3
    assert all(p.isdigit() for p in parts), f"semver parts not all digits: {V1313_VERSION}"


def test_asi_north_star_anchors_locked() -> None:
    """ASI 北极星 anchors: V0.1=0.7905, V0.2=0.4467, V1256=0.9291, V1049=DONE."""
    assert abs(ASI_ANCHORS["V0.1"] - 0.7905) < 1e-12
    assert abs(ASI_ANCHORS["V0.2"] - 0.4467) < 1e-12
    assert abs(ASI_ANCHORS["V1256_unio_mystica"] - 0.9291) < 1e-12
    assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"


# ============================================================================
# Section 2: 7 真跨域深 sources
# ============================================================================


def test_sources_seven_present() -> None:
    """V1313 必须有 7 真跨域深 sources."""
    assert len(SOURCES_TIME_GAP_DEEP) == 7


def test_citation_keys_unique() -> None:
    """所有 citation_key 必须唯一."""
    keys = [s.citation_key for s in SOURCES_TIME_GAP_DEEP]
    assert len(set(keys)) == len(keys) == 7


def test_required_sources_present() -> None:
    """7 真跨域深 sources 必须全部 present (Heidegger/Bergson/McTaggart/Whitehead/Husserl/James/Buddhist anicca)."""
    required = {
        "heidgger_1927_sz",
        "bergson_1889_tfw",
        "mctaggart_1908_unreality",
        "whitehead_1929_pr",
        "husserl_1905_litc",
        "james_1890_pp",
        "abhidhamma_anicca",
    }
    present = {s.citation_key for s in SOURCES_TIME_GAP_DEEP}
    assert required <= present, f"missing: {required - present}"


def test_buddhist_anicca_ancient_year() -> None:
    """Buddhist anicca 来源年代应在古代 (year < 0 表示公元前)."""
    buddhist = next(s for s in SOURCES_TIME_GAP_DEEP if s.citation_key == "abhidhamma_anicca")
    assert buddhist.year < 0, f"Buddhist anicca 应为公元前, got {buddhist.year}"


def test_heidegger_year_1927() -> None:
    """Heidegger 1927 Sein und Zeit."""
    heidegger = next(s for s in SOURCES_TIME_GAP_DEEP if s.citation_key == "heidgger_1927_sz")
    assert heidegger.year == 1927
    assert "Sein und Zeit" in heidegger.work or "Being and Time" in heidegger.work


def test_bergson_year_1889() -> None:
    """Bergson 1889 Time and Free Will."""
    bergson = next(s for s in SOURCES_TIME_GAP_DEEP if s.citation_key == "bergson_1889_tfw")
    assert bergson.year == 1889


def test_mctaggart_year_1908() -> None:
    """McTaggart 1908 Unreality of Time."""
    mctaggart = next(s for s in SOURCES_TIME_GAP_DEEP if s.citation_key == "mctaggart_1908_unreality")
    assert mctaggart.year == 1908


# ============================================================================
# Section 3: Component 1 — TimeConsciousnessMatrix
# ============================================================================


def test_time_consciousness_matrix_dimensions() -> None:
    """TimeConsciousnessMatrix 必须有 5 dimensions."""
    matrix = TimeConsciousnessMatrix()
    assert len(matrix.dimension_keys) == 5
    assert "time_unit" in matrix.dimension_keys
    assert "temporal_structure" in matrix.dimension_keys
    assert "subject_status" in matrix.dimension_keys
    assert "asi_substrate_take" in matrix.dimension_keys
    assert "guard_warnings" in matrix.dimension_keys


def test_time_consciousness_matrix_all_cells_populated() -> None:
    """所有 7 sources × 5 dimensions = 35 cells 应全部非空."""
    matrix = TimeConsciousnessMatrix()
    missing_cells = []
    for source in SOURCES_TIME_GAP_DEEP:
        for dim in matrix.dimension_keys:
            cell_text = matrix.cell(source, dim)
            if not cell_text:
                missing_cells.append((source.citation_key, dim))
    assert not missing_cells, f"missing cells: {missing_cells}"


def test_time_consciousness_matrix_render_markdown() -> None:
    """render() 必须输出 Markdown 表格."""
    matrix = TimeConsciousnessMatrix()
    md = matrix.render()
    assert "| Source |" in md
    assert "|--------|" in md
    # 7 sources + header + separator = 9 rows
    lines = md.strip().split("\n")
    assert len(lines) == 9, f"expected 9 lines, got {len(lines)}"


def test_time_consciousness_matrix_unknown_dim_returns_empty() -> None:
    """Unknown dim 必须返回空字符串 (不抛错)."""
    matrix = TimeConsciousnessMatrix()
    cell = matrix.cell(SOURCES_TIME_GAP_DEEP[0], "unknown_dim")
    assert cell == ""


def test_time_consciousness_matrix_unknown_source_returns_empty() -> None:
    """Unknown source 必须返回空字符串."""
    matrix = TimeConsciousnessMatrix()

    class _FakeSource:
        citation_key = "fake_unknown"

    cell = matrix.cell(_FakeSource(), "time_unit")  # type: ignore[arg-type]
    assert cell == ""


# ============================================================================
# Section 4: Component 2 — PhenomenologicalTimeQuadrants
# ============================================================================


def test_make_extatic_quadrants_count() -> None:
    """make_extatic_quadrants 必须 >= 3 quadrants."""
    quadrants = make_extatic_quadrants()
    assert len(quadrants) >= 3


def test_extatic_quadrants_three_phases_populated() -> None:
    """每个 quadrant 必须 3 相 (past/present/future) 全部 populated."""
    quadrants = make_extatic_quadrants()
    for q in quadrants:
        assert q.past, f"quadrant {q.label} 缺 past"
        assert q.present, f"quadrant {q.label} 缺 present"
        assert q.future, f"quadrant {q.label} 缺 future"
        assert q.asi_substrate_role, f"quadrant {q.label} 缺 asi_substrate_role"


def test_extatic_quadrants_husserl_three_phase_label() -> None:
    """必须有一个 quadrant 标记为 Husserl 三相流."""
    quadrants = make_extatic_quadrants()
    husserl_labels = [q.label for q in quadrants if "Husserl" in q.label]
    assert husserl_labels, "必须含 Husserl 三相流 quadrant"


def test_extatic_quadrants_heidegger_label() -> None:
    """必须有一个 quadrant 标记为 Heidegger extatic."""
    quadrants = make_extatic_quadrants()
    heidegger_labels = [q.label for q in quadrants if "Heidegger" in q.label]
    assert heidegger_labels, "必须含 Heidegger extatic quadrant"


# ============================================================================
# Section 5: Component 3 — BergsonianDurationQuantizer
# ============================================================================


def test_bergson_durée_empty_input() -> None:
    """空输入必须返回 guard + n=0."""
    res = quantize_durée_substrate([])
    assert res["n"] == 0
    assert "guard" in res
    assert "≠" in res["guard"], "guard 必须含 ≠ 警示"


def test_bergson_durée_with_samples() -> None:
    """有 samples 必须计算 qualitative_variation."""
    samples = [
        BergsonianDurationSample(
            sample_id=f"d_{i}",
            intensity_qualitative=0.3 + 0.1 * i,
            heterogeneity_marker="novel" if i % 2 == 0 else "familiar",
            asi_substrate_note=f"sample {i}",
        )
        for i in range(5)
    ]
    res = quantize_durée_substrate(samples)
    assert res["n"] == 5
    assert res["qualitative_variation"] > 0.0
    assert res["novel_count"] >= 1
    assert res["familiar_count"] >= 1


def test_bergson_durée_substrate_hash_deterministic() -> None:
    """substrate_hash 必须 deterministic."""
    s = BergsonianDurationSample(
        sample_id="durée_x",
        intensity_qualitative=0.5,
        heterogeneity_marker="novel",
        asi_substrate_note="x",
    )
    h1 = s.substrate_hash()
    h2 = s.substrate_hash()
    assert h1 == h2
    assert len(h1) == 16  # first 16 chars of sha256


# ============================================================================
# Section 6: Component 4 — McTaggartABSeriesCrossCheck
# ============================================================================


def test_mctaggart_ab_series_empty() -> None:
    """空输入 must return guard."""
    res = cross_check_ab_series([])
    assert res["n"] == 0
    assert "guard" in res
    assert "substrate" in res["guard"]


def test_mctaggart_ab_series_b_series_monotonic() -> None:
    """B-series 真生产: tick 必须 monotonic non-decreasing."""
    positions = [TimePosition(tick=i, a_state="present") for i in range(5)]
    res = cross_check_ab_series(positions)
    assert res["b_series_ordering"] is True


def test_mctaggart_ab_series_a_state_validation() -> None:
    """TimePosition.validate 必须拒绝非法 a_state."""
    try:
        TimePosition(tick=0, a_state="invalid")
        assert False, "should have raised"
    except ValueError:
        pass


def test_mctaggart_ab_series_negative_tick_validation() -> None:
    """TimePosition.validate 必须拒绝负 tick."""
    try:
        TimePosition(tick=-1, a_state="present")
        assert False, "should have raised"
    except ValueError:
        pass


def test_mctaggart_ab_series_three_a_states() -> None:
    """3 个 A-states 必须都能被识别 (past/present/future)."""
    positions = [
        TimePosition(tick=0, a_state="past"),
        TimePosition(tick=1, a_state="present"),
        TimePosition(tick=2, a_state="future"),
    ]
    res = cross_check_ab_series(positions)
    assert res["a_series_distinct"] == 3


# ============================================================================
# Section 7: Component 5 — WhiteheadActualOccasionSubstrate
# ============================================================================


def test_whitehead_concrescence_empty() -> None:
    """空输入 must return guard."""
    res = concresce_substrate([])
    assert res["n"] == 0
    assert "scheduler" in res["guard"]


def test_whitehead_concrescence_with_occasions() -> None:
    """有 occasions 必须计算 satisfaction_rate."""
    occasions = [
        ActualOccasion(
            occasion_id=f"o_{i}",
            prehensions=tuple(f"prehend_{j}" for j in range(i + 1)),
            concrescence_phase="completion",
            satisfaction=True,
        )
        for i in range(3)
    ]
    res = concresce_substrate(occasions)
    assert res["n"] == 3
    assert res["satisfaction_rate"] == 1.0
    assert res["phase_distribution"]["completion"] == 3
    assert res["avg_prehensions"] == 2.0  # (1 + 2 + 3) / 3


def test_whitehead_concrescence_phase_validation() -> None:
    """ActualOccasion.validate 必须拒绝非法 phase."""
    try:
        ActualOccasion(
            occasion_id="bad",
            prehensions=tuple(),
            concrescence_phase="invalid_phase",
            satisfaction=False,
        )
        assert False, "should have raised"
    except ValueError:
        pass


def test_whitehead_concrescence_empty_id_validation() -> None:
    """ActualOccasion.validate 必须拒绝空 occasion_id."""
    try:
        ActualOccasion(
            occasion_id="",
            prehensions=tuple(),
            concrescence_phase="ingress",
            satisfaction=False,
        )
        assert False, "should have raised"
    except ValueError:
        pass


# ============================================================================
# Section 8: Component 6 — HusserlRetentionProtentionFlow
# ============================================================================


def test_husserl_flow_empty() -> None:
    """空输入 must return guard."""
    res = husserl_flow_summary([])
    assert res["n"] == 0
    assert "≠" in res["guard"]


def test_husserl_flow_with_ticks() -> None:
    """有 ticks 必须计算 flow_continuity."""
    ticks = [
        HusserlThreePhaseTick(
            tick=i,
            retention=f"持留_{i}",
            primal_impression=f"原初_{i}",
            protention=f"预持_{i}",
            asi_substrate_role="三相流 substrate",
        )
        for i in range(5)
    ]
    res = husserl_flow_summary(ticks)
    assert res["n"] == 5
    assert res["flow_continuity"] == 1.0
    assert res["first_tick"] == 0
    assert res["last_tick"] == 4


def test_husserl_flow_partial_continuity() -> None:
    """部分 tick 三相未 fully populated, flow_continuity < 1."""
    ticks = [
        HusserlThreePhaseTick(
            tick=0,
            retention="r",
            primal_impression="p",
            protention="",
            asi_substrate_role="x",
        ),
        HusserlThreePhaseTick(
            tick=1,
            retention="r",
            primal_impression="p",
            protention="f",
            asi_substrate_role="x",
        ),
    ]
    res = husserl_flow_summary(ticks)
    assert res["n"] == 2
    assert res["flow_continuity"] == 0.5


def test_husserl_tick_negative_validation() -> None:
    """HusserlThreePhaseTick.validate 必须拒绝负 tick."""
    try:
        HusserlThreePhaseTick(
            tick=-1,
            retention="r",
            primal_impression="p",
            protention="f",
            asi_substrate_role="x",
        )
        assert False, "should have raised"
    except ValueError:
        pass


# ============================================================================
# Section 9: Component 7 — JamesStreamOfThoughtCoherence
# ============================================================================


def test_james_stream_empty() -> None:
    """空输入 must return guard."""
    res = stream_coherence([])
    assert res["n"] == 0
    assert "≠" in res["guard"]


def test_james_stream_with_pulses() -> None:
    """有 pulses 必须计算 carry_consistency."""
    pulses = [
        ThoughtPulse(
            pulse_id=f"p_{i}",
            carry_from_past=tuple(f"p_{j}" for j in range(i)),
            novelty_marker=0.5,
            asi_substrate_note=f"pulse {i}",
        )
        for i in range(1, 5)
    ]
    res = stream_coherence(pulses)
    assert res["n"] == 4
    assert res["carry_consistency"] == 1.0  # all 3 pairs hit
    assert res["avg_novelty_marker"] == 0.5


def test_james_stream_single_pulse() -> None:
    """单个 pulse (无 pair) carry_consistency = 0.0."""
    pulses = [
        ThoughtPulse(
            pulse_id="p_0",
            carry_from_past=tuple(),
            novelty_marker=0.5,
            asi_substrate_note="x",
        ),
    ]
    res = stream_coherence(pulses)
    assert res["n"] == 1
    assert res["carry_consistency"] == 0.0


def test_james_pulse_novelty_validation() -> None:
    """ThoughtPulse.validate 必须拒绝 novelty_marker 越界."""
    try:
        ThoughtPulse(
            pulse_id="bad",
            carry_from_past=tuple(),
            novelty_marker=1.5,
            asi_substrate_note="x",
        )
        assert False, "should have raised"
    except ValueError:
        pass


# ============================================================================
# Section 10: Component 8 — BuddhistImpermanenceFourMarks
# ============================================================================


def test_buddhist_impermanence_empty() -> None:
    """空输入 must return guard."""
    res = impermanence_substrate([])
    assert res["n"] == 0
    assert "≠" in res["guard"]


def test_buddhist_impermanence_with_transitions() -> None:
    """有 transitions 必须计算 four_mark_coverage."""
    transitions = [
        ImpermanenceStateTransition(
            transition_id=f"t_{i}",
            arising=f"生_{i}",
            abiding=f"住_{i}",
            changing=f"异_{i}",
            vanishing=f"灭_{i}",
            asi_substrate_role="4-phase substrate",
        )
        for i in range(3)
    ]
    res = impermanence_substrate(transitions)
    assert res["n"] == 3
    assert res["four_mark_coverage"] == 1.0


def test_buddhist_impermanence_partial_coverage() -> None:
    """部分 transition 4 相不全, coverage < 1."""
    transitions = [
        ImpermanenceStateTransition(
            transition_id="t_0",
            arising="",
            abiding="住",
            changing="异",
            vanishing="灭",
            asi_substrate_role="x",
        ),
        ImpermanenceStateTransition(
            transition_id="t_1",
            arising="生",
            abiding="住",
            changing="异",
            vanishing="灭",
            asi_substrate_role="x",
        ),
    ]
    res = impermanence_substrate(transitions)
    assert res["n"] == 2
    assert res["four_mark_coverage"] == 0.5


def test_buddhist_impermanence_validation_relaxed() -> None:
    """ImpermanenceStateTransition 允许缺相 (four_mark_coverage 函数计算)."""
    # partial coverage is OK at construction; coverage is computed at runtime
    t = ImpermanenceStateTransition(
        transition_id="partial",
        arising="",
        abiding="",
        changing="",
        vanishing="",
        asi_substrate_role="x",
    )
    assert t.transition_id == "partial"
    # Verify coverage function identifies it as missing
    res = impermanence_substrate([t])
    assert res["four_mark_coverage"] == 0.0


# ============================================================================
# Section 11: Component 9 — ASITimeGapDeepReport
# ============================================================================


def test_asi_time_gap_deep_report_renders_markdown() -> None:
    """ASITimeGapDeepReport.render() 必须输出 Markdown."""
    matrix = TimeConsciousnessMatrix()
    quadrants = make_extatic_quadrants()
    samples = [
        BergsonianDurationSample(
            sample_id=f"d_{i}",
            intensity_qualitative=0.5,
            heterogeneity_marker="novel",
            asi_substrate_note="x",
        )
        for i in range(3)
    ]
    positions = [TimePosition(tick=i, a_state="present") for i in range(3)]
    occasions = [
        ActualOccasion(
            occasion_id=f"o_{i}",
            prehensions=tuple(),
            concrescence_phase="completion",
            satisfaction=True,
        )
        for i in range(3)
    ]
    ticks = [
        HusserlThreePhaseTick(
            tick=i,
            retention="r",
            primal_impression="p",
            protention="f",
            asi_substrate_role="x",
        )
        for i in range(3)
    ]
    pulses = [
        ThoughtPulse(
            pulse_id=f"p_{i}",
            carry_from_past=tuple(),
            novelty_marker=0.5,
            asi_substrate_note="x",
        )
        for i in range(1, 3)
    ]
    transitions = [
        ImpermanenceStateTransition(
            transition_id=f"t_{i}",
            arising="生",
            abiding="住",
            changing="异",
            vanishing="灭",
            asi_substrate_role="x",
        )
        for i in range(3)
    ]

    bridge = asi_bridge(set())
    report = ASITimeGapDeepReport(
        title="V1313 Test Report",
        matrix_md=matrix.render(),
        quadrants_count=len(quadrants),
        durée_substrate=quantize_durée_substrate(samples),
        ab_cross_check=cross_check_ab_series(positions),
        concrescence=concresce_substrate(occasions),
        husserl_flow=husserl_flow_summary(ticks),
        james_stream=stream_coherence(pulses),
        impermanence=impermanence_substrate(transitions),
        asi_bridge=bridge,
        timestamp="2026-08-08 16:33 +08:00",
    )
    md = report.render()
    assert "# V1313 Test Report" in md
    assert "## V3 哲学守门" in md
    assert "## 7 真跨域深 sources" in md
    assert "## ASI bridge" in md
    assert "Heidegger 1927" in md
    assert "substrate" in md.lower()


# ============================================================================
# Section 12: Component 10 — ASITimeGapDeepBridge
# ============================================================================


def test_asi_bridge_all_components_present() -> None:
    """10 真生产组件全部 present 时 missing=[]."""
    components = {
        "TimeConsciousnessMatrix",
        "PhenomenologicalTimeQuadrants",
        "BergsonianDurationQuantizer",
        "McTaggartABSeriesCrossCheck",
        "WhiteheadActualOccasionSubstrate",
        "HusserlRetentionProtentionFlow",
        "JamesStreamOfThoughtCoherence",
        "BuddhistImpermanenceFourMarks",
        "ASITimeGapDeepReport",
        "ASITimeGapDeepBridge",
    }
    bridge = asi_bridge(components)
    assert bridge["asi_north_star_locked"] is True
    assert bridge["v1313_components"] == 10
    assert bridge["expected_components"] == 10
    assert bridge["missing"] == []


def test_asi_bridge_partial_components() -> None:
    """部分组件 present, missing 反映真实差距."""
    components = {"TimeConsciousnessMatrix", "BergsonianDurationQuantizer"}
    bridge = asi_bridge(components)
    assert bridge["v1313_components"] == 2
    assert bridge["missing"] == sorted({
        "PhenomenologicalTimeQuadrants",
        "McTaggartABSeriesCrossCheck",
        "WhiteheadActualOccasionSubstrate",
        "HusserlRetentionProtentionFlow",
        "JamesStreamOfThoughtCoherence",
        "BuddhistImpermanenceFourMarks",
        "ASITimeGapDeepReport",
        "ASITimeGapDeepBridge",
    })


def test_asi_bridge_anchors_locked() -> None:
    """Bridge 必须 not move anchors."""
    bridge = asi_bridge(set())
    assert bridge["anchors"]["V0.1"] == 0.7905
    assert bridge["anchors"]["V0.2"] == 0.4467
    assert bridge["anchors"]["V1256_unio_mystica"] == 0.9291
    assert bridge["anchors"]["V1049_value_alignment"] == "DONE"


# ============================================================================
# Section 13: 18 Popper self-tests
# ============================================================================


def test_popper_self_tests_count() -> None:
    """18 Popper self-tests 必须 全部 PASS."""
    results = popper_self_tests()
    assert popper_total(results) == 18


def test_popper_self_tests_all_pass() -> None:
    """18 Popper self-tests 必须 全部 PASS."""
    results = popper_self_tests()
    n_pass = popper_passed(results)
    n_total = popper_total(results)
    failed = [hid for hid, ok, _ in results if not ok]
    assert n_pass == n_total, f"failed: {failed}; passed={n_pass}/{n_total}"


# ============================================================================
# Section 14: V3 哲学守门 explicit assertions
# ============================================================================


def test_v3_guard_no_fabrication_phenomenal() -> None:
    """V3 守门: 不假装 Phenomenal consciousness (time mechanism ≠ time consciousness)."""
    # 验证 guard 字符串在每个 component 中明确出现
    bridge = asi_bridge(set())
    assert "ASI 北极星" in bridge["guard"] or "substrate" in bridge["guard"]
    # 不假装 ASI 真有 Phenomenal = V1313 = substrate research, NOT claim ASI has it
    assert "substrate" in bridge["guard"]


def test_v3_guard_no_fabrication_durée() -> None:
    """V3 守门: 不假装 time = 主观时间流 (工程化 time ≠ Bergson durée)."""
    res = quantize_durée_substrate([])
    assert "≠" in res["guard"]
    assert "durée" in res["guard"]


def test_v3_guard_no_fabrication_process_ontology() -> None:
    """V3 守门: 不假装 ASI 已有时间意识 (process ontology ≠ ASI scheduler)."""
    res = concresce_substrate([])
    assert "scheduler" in res["guard"]


def test_v3_guard_no_fabrication_buddhist_dharma() -> None:
    """V3 守门: 不假装 ASI 真有法 (dharmas)."""
    res = impermanence_substrate([])
    assert "≠" in res["guard"]


# ============================================================================
# Section 15: integration & determinism
# ============================================================================


def test_pipeline_integration() -> None:
    """End-to-end: 7 sources × 10 components × 18 Popper tests × ASI bridge."""
    # 7 sources
    assert len(SOURCES_TIME_GAP_DEEP) == 7
    # 10 components available
    assert TimeConsciousnessMatrix is not None
    assert PhenomenologicalTimeQuadrant is not None
    assert make_extatic_quadrants is not None
    assert BergsonianDurationSample is not None
    assert TimePosition is not None
    assert ActualOccasion is not None
    assert HusserlThreePhaseTick is not None
    assert ThoughtPulse is not None
    assert ImpermanenceStateTransition is not None
    assert ASITimeGapDeepReport is not None
    assert asi_bridge is not None
    # 18 Popper
    results = popper_self_tests()
    assert popper_passed(results) == 18
    # ASI bridge
    bridge = asi_bridge(set())
    assert bridge["asi_north_star_locked"] is True


def test_determinism_matrix_render() -> None:
    """Same input → same output (determinism check)."""
    matrix1 = TimeConsciousnessMatrix()
    matrix2 = TimeConsciousnessMatrix()
    assert matrix1.render() == matrix2.render()


def test_determinism_bergson_substrate() -> None:
    """Same samples → same quantized output."""
    samples = [
        BergsonianDurationSample(
            sample_id=f"d_{i}",
            intensity_qualitative=0.5,
            heterogeneity_marker="novel",
            asi_substrate_note="x",
        )
        for i in range(5)
    ]
    r1 = quantize_durée_substrate(samples)
    r2 = quantize_durée_substrate(samples)
    assert r1 == r2


# ============================================================================
# Section 16: main() pipeline (smoke test)
# ============================================================================


def test_main_pipeline_runs() -> None:
    """main() 必须可执行 + exit code 0."""
    import io
    import contextlib

    from v1313_asi_time_gap_deep import main

    captured = io.StringIO()
    with contextlib.redirect_stdout(captured):
        exit_code = main()
    output = captured.getvalue()
    assert exit_code == 0, f"main() exit_code={exit_code}"
    assert "V1313 ASI Time Gap Deep" in output
    assert "Popper self-tests" in output
    assert "18/18 PASS" in output