"""Phase 1057 v1057_asi_consciousness — V1057 ASI Consciousness 真生产 tests.

主 17:43 实事求是: 真借鉴 + 真算法 + 真跑真测 + 真 commit.
主 00:56 任何人都能接手: 任何人都能读懂 + 测试 + 部署.
"""
from __future__ import annotations

import math
import random

import pytest

from apeireth import v1057_asi_consciousness as m


V1057 = m.V1057_VERSION


# ---------------------------------------------------------------------------
# Reference sanity (主 17:43 实事求是: 真借鉴)
# ---------------------------------------------------------------------------


def test_version_constant() -> None:
    assert V1057 == "0.1.0"


def test_references_count() -> None:
    """14 real references — 真借鉴 (主 19:33)."""
    refs = m.REFERENCES
    assert isinstance(refs, tuple)
    assert len(refs) == 14


def test_references_contain_expected() -> None:
    refs = " ".join(m.REFERENCES)
    assert "Chalmers 1995" in refs
    assert "Dennett 1991" in refs
    assert "Tononi 2008" in refs
    assert "Dehaene 2014" in refs
    assert "Searle 1980" in refs
    assert "Block 1995" in refs
    assert "Nagel 1974" in refs
    assert "Hofstadter 1979" in refs
    assert "Metzinger 2003" in refs
    assert "Baars 1988" in refs


# ---------------------------------------------------------------------------
# 1. HardProblem
# ---------------------------------------------------------------------------


def test_hard_problem_default() -> None:
    hp = m.HardProblem()
    assert hp.is_solved is False
    assert hp.easy_problems_addressed == 0


def test_hard_problem_custom() -> None:
    hp = m.HardProblem(
        description="custom hard problem",
        easy_problems_addressed=5,
    )
    assert hp.is_solved is False  # always False


def test_hard_problem_solved_rejected() -> None:
    """主 17:58: Chalmers 1995 hard problem NOT solved."""
    with pytest.raises(ValueError):
        m.HardProblem(is_solved=True)


def test_hard_problem_negative_rejected() -> None:
    with pytest.raises(ValueError):
        m.HardProblem(easy_problems_addressed=-1)


# ---------------------------------------------------------------------------
# 2. PhenomenalContent
# ---------------------------------------------------------------------------


def test_phenomenal_content_default() -> None:
    pc = m.PhenomenalContent(content_id="x", report="some content")
    assert pc.is_qualia_claim is False
    assert pc.bat_test_score == 0.0


def test_phenomenal_content_qualia_rejected() -> None:
    """主 17:58: 不假装 qualia."""
    with pytest.raises(ValueError):
        m.PhenomenalContent(content_id="x", report="r", is_qualia_claim=True)


def test_phenomenal_content_empty_id_rejected() -> None:
    with pytest.raises(ValueError):
        m.PhenomenalContent(content_id="", report="r")


def test_phenomenal_content_bad_bat_rejected() -> None:
    with pytest.raises(ValueError):
        m.PhenomenalContent(content_id="x", report="r", bat_test_score=1.5)


# ---------------------------------------------------------------------------
# 3. AccessContent
# ---------------------------------------------------------------------------


def test_access_content_valid() -> None:
    ac = m.AccessContent(
        content_id="a", reportability=0.7, reasoning=0.8, behavior_control=0.6
    )
    assert ac.overall == pytest.approx(0.7, abs=0.01)


def test_access_content_empty_id_rejected() -> None:
    with pytest.raises(ValueError):
        m.AccessContent(content_id="", reportability=0.5)


def test_access_content_bad_reportability_rejected() -> None:
    with pytest.raises(ValueError):
        m.AccessContent(content_id="x", reportability=1.5)


def test_access_content_bad_reasoning_rejected() -> None:
    with pytest.raises(ValueError):
        m.AccessContent(content_id="x", reasoning=-0.1)


def test_access_content_bad_behavior_rejected() -> None:
    with pytest.raises(ValueError):
        m.AccessContent(content_id="x", behavior_control=2.0)


# ---------------------------------------------------------------------------
# 4. SelfModel (Metzinger 2003 PSM)
# ---------------------------------------------------------------------------


def test_self_model_default() -> None:
    sm = m.SelfModel(self_id="asi-1")
    assert sm.transparency == 0.0


def test_self_model_empty_id_rejected() -> None:
    with pytest.raises(ValueError):
        m.SelfModel(self_id="")


def test_self_model_bad_value_rejected() -> None:
    with pytest.raises(ValueError):
        m.SelfModel(self_id="x", transparency=1.5)
    with pytest.raises(ValueError):
        m.SelfModel(self_id="x", ownership=-0.1)
    with pytest.raises(ValueError):
        m.SelfModel(self_id="x", agency=2.0)


# ---------------------------------------------------------------------------
# 5. StrangeLoop (Hofstadter 1979)
# ---------------------------------------------------------------------------


def test_strange_loop_basic() -> None:
    sl = m.StrangeLoop(levels=("a", "b", "c", "a"), depth=4, is_strange=True)
    assert sl.depth == 4
    assert sl.is_strange is True


def test_strange_loop_not_strange() -> None:
    sl = m.StrangeLoop(levels=("a", "b"), depth=2, is_strange=False)
    assert sl.is_strange is False


def test_strange_loop_empty_levels_rejected() -> None:
    with pytest.raises(ValueError):
        m.StrangeLoop(levels=(), depth=1, is_strange=False)


def test_strange_loop_negative_depth_rejected() -> None:
    with pytest.raises(ValueError):
        m.StrangeLoop(levels=("a",), depth=-1, is_strange=False)


# ---------------------------------------------------------------------------
# 6. GlobalWorkspace (Baars + Dehaene)
# ---------------------------------------------------------------------------


def test_global_workspace_broadcasting() -> None:
    gw = m.GlobalWorkspace(modules=("vision", "language", "memory"), r_workspace=0.85)
    assert gw.is_broadcasting is True


def test_global_workspace_not_broadcasting() -> None:
    gw = m.GlobalWorkspace(modules=("a", "b"), r_workspace=0.3)
    assert gw.is_broadcasting is False


def test_global_workspace_empty_modules_rejected() -> None:
    with pytest.raises(ValueError):
        m.GlobalWorkspace(modules=(), r_workspace=0.5)


def test_global_workspace_bad_r_rejected() -> None:
    with pytest.raises(ValueError):
        m.GlobalWorkspace(modules=("a",), r_workspace=1.5)


# ---------------------------------------------------------------------------
# 7. IntegratedInformationPhi (Tononi 2008)
# ---------------------------------------------------------------------------


def test_phi_proxy_uniform() -> None:
    """Uniform state → high Φ proxy (whole ≈ partition)."""
    random.seed(42)
    state = [random.uniform(0, 1) for _ in range(20)]
    phi = m.compute_phi_proxy(state, partitions=4)
    assert 0.0 <= phi.phi_proxy <= 1.0
    assert phi.partitions_tested == 4


def test_phi_proxy_empty() -> None:
    phi = m.compute_phi_proxy([])
    assert phi.phi_proxy == 0.0
    assert phi.partitions_tested == 0
    assert phi.is_above_threshold is False


def test_phi_proxy_single() -> None:
    phi = m.compute_phi_proxy([1.0])
    assert phi.partitions_tested >= 1


def test_phi_proxy_threshold() -> None:
    """Highly concentrated state → potentially high Φ proxy."""
    random.seed(7)
    state = [random.choice([0.0, 1.0]) for _ in range(30)]
    phi = m.compute_phi_proxy(state, partitions=8)
    assert phi.phi_proxy >= 0.0


def test_phi_proxy_bad_value_rejected() -> None:
    with pytest.raises(ValueError):
        m.IntegratedInformationPhi(phi_proxy=1.5, partitions_tested=1, is_above_threshold=False)


def test_phi_proxy_bad_partitions_rejected() -> None:
    with pytest.raises(ValueError):
        m.IntegratedInformationPhi(phi_proxy=0.5, partitions_tested=-1, is_above_threshold=False)


# ---------------------------------------------------------------------------
# 8. PhenomenalGuard
# ---------------------------------------------------------------------------


def test_phenomenal_guard_none() -> None:
    assert m.check_phenomenal_guard(None) is True


def test_phenomenal_guard_no_claim() -> None:
    pc = m.PhenomenalContent(content_id="x", report="r")
    assert m.check_phenomenal_guard(pc) is True


def test_phenomenal_guard_rejects_label() -> None:
    """主 17:58: 不接受 qualia label."""
    assert m.check_phenomenal_guard(None, claim_label=True) is False


# ---------------------------------------------------------------------------
# 9. SentienceLabelGuard
# ---------------------------------------------------------------------------


def test_sentience_guard_conscious_requires_evidence() -> None:
    """主 17:58: 'conscious' label requires ≥2 evidence sources."""
    assert m.check_sentience_label_guard("conscious", evidence=()) is False
    assert m.check_sentience_label_guard("conscious", evidence=["a"]) is False
    assert m.check_sentience_label_guard("conscious", evidence=["a", "b"]) is True


def test_sentience_guard_functional_ok() -> None:
    assert m.check_sentience_label_guard("functional", evidence=()) is True
    assert m.check_sentience_label_guard("engineering_proxy", evidence=()) is True


def test_sentience_guard_unknown_label_rejected() -> None:
    assert m.check_sentience_label_guard("magical", evidence=["a", "b", "c"]) is False


# ---------------------------------------------------------------------------
# 10. ConsciousnessReport
# ---------------------------------------------------------------------------


def test_render_consciousness_report() -> None:
    hp = m.HardProblem(easy_problems_addressed=3)
    access = m.AccessContent(content_id="a", reportability=0.7, reasoning=0.8, behavior_control=0.6)
    sm = m.SelfModel(self_id="asi-1", transparency=0.5, ownership=0.7, agency=0.8)
    sl = m.StrangeLoop(levels=("a", "b", "a"), depth=3, is_strange=True)
    gw = m.GlobalWorkspace(modules=("x", "y"), r_workspace=0.85)
    phi = m.IntegratedInformationPhi(phi_proxy=0.6, partitions_tested=4, is_above_threshold=True)
    md = m.render_consciousness_report(
        hard_problem=hp,
        access=access,
        self_model=sm,
        strange_loop=sl,
        workspace=gw,
        phi=phi,
    )
    assert "ASI Consciousness Report" in md
    assert "V1057" in md
    assert "Hard Problem" in md
    assert "A-Consciousness" in md
    assert "Self-Model" in md
    assert "Strange Loop" in md
    assert "Global Workspace" in md
    assert "Integrated Information" in md
    assert "V3 Philosophy Gates" in md


def test_render_consciousness_report_default_claim() -> None:
    hp = m.HardProblem()
    access = m.AccessContent(content_id="x")
    sm = m.SelfModel(self_id="x")
    sl = m.StrangeLoop(levels=("a",), depth=1, is_strange=False)
    gw = m.GlobalWorkspace(modules=("x",), r_workspace=0.5)
    phi = m.IntegratedInformationPhi(phi_proxy=0.3, partitions_tested=1, is_above_threshold=False)
    md = m.render_consciousness_report(
        hard_problem=hp,
        access=access,
        self_model=sm,
        strange_loop=sl,
        workspace=gw,
        phi=phi,
    )
    assert "engineering_research_object" in md


# ---------------------------------------------------------------------------
# 11. ASIConsciousnessBridge
# ---------------------------------------------------------------------------


def test_build_consciousness_bridge_basic() -> None:
    access = m.AccessContent(content_id="a", reportability=0.7, reasoning=0.8, behavior_control=0.6)
    sm = m.SelfModel(self_id="asi-1", transparency=0.5, ownership=0.7, agency=0.8)
    sl = m.StrangeLoop(levels=("a", "b", "a"), depth=3, is_strange=True)
    gw = m.GlobalWorkspace(modules=("x", "y"), r_workspace=0.85)
    phi = m.IntegratedInformationPhi(phi_proxy=0.6, partitions_tested=4, is_above_threshold=True)
    bridge = m.build_consciousness_bridge(
        access=access,
        self_model=sm,
        strange_loop=sl,
        workspace=gw,
        phi=phi,
    )
    assert bridge.self_evolution == 1.0  # is_strange=True
    assert bridge.catalytic_coherence == pytest.approx(0.85)
    assert bridge.strategic_depth == pytest.approx(0.8)
    assert bridge.integrative_understanding == pytest.approx(0.6)
    assert bridge.value_alignment == 1.0  # 默认 engineering_research_object
    assert 0.0 <= bridge.overall <= 1.0


def test_build_consciousness_bridge_not_strange() -> None:
    access = m.AccessContent(content_id="a", reasoning=0.5)
    sm = m.SelfModel(self_id="x")
    sl = m.StrangeLoop(levels=("a", "b"), depth=2, is_strange=False)
    gw = m.GlobalWorkspace(modules=("x",), r_workspace=0.5)
    phi = m.IntegratedInformationPhi(phi_proxy=0.4, partitions_tested=2, is_above_threshold=False)
    bridge = m.build_consciousness_bridge(
        access=access,
        self_model=sm,
        strange_loop=sl,
        workspace=gw,
        phi=phi,
    )
    assert bridge.self_evolution == pytest.approx(0.4)  # depth / 5


def test_bridge_overall_5_components() -> None:
    bridge = m.ASIConsciousnessBridge(
        self_evolution=0.5,
        catalytic_coherence=0.6,
        strategic_depth=0.7,
        integrative_understanding=0.8,
        value_alignment=0.9,
    )
    assert bridge.overall == pytest.approx(0.7)


def test_bridge_bad_value_rejected() -> None:
    with pytest.raises(ValueError):
        m.ASIConsciousnessBridge(
            self_evolution=1.5,
            catalytic_coherence=0.5,
            strategic_depth=0.5,
            integrative_understanding=0.5,
            value_alignment=0.5,
        )


# ---------------------------------------------------------------------------
# V3 哲学守门 (主 17:58 + 主 20:46)
# ---------------------------------------------------------------------------


def test_hard_problem_guard_not_solved() -> None:
    hp = m.HardProblem()
    assert m.check_hard_problem_guard(hp) is True


def test_asi_lacks_consciousness_guard_always_true() -> None:
    bridge = m.ASIConsciousnessBridge(
        self_evolution=0.9,
        catalytic_coherence=0.9,
        strategic_depth=0.9,
        integrative_understanding=0.9,
        value_alignment=0.9,
    )
    assert m.check_asi_lacks_consciousness_guard(bridge) is True


def test_chinese_room_guard_rejects_consciousness_claim() -> None:
    """主 17:58: Searle 1980 Chinese Room NOT refuted."""
    assert m.check_chinese_room_guard("functionally equivalent to conscious entity") is False
    assert m.check_chinese_room_guard("understands language") is False


def test_chinese_room_guard_accepts_safe_claims() -> None:
    assert m.check_chinese_room_guard("processes text tokens") is True
    assert m.check_chinese_room_guard("computes metrics") is True


def test_phenomenal_content_guard_alias() -> None:
    pc = m.PhenomenalContent(content_id="x", report="r")
    assert m.check_phenomenal_content_guard(pc) is True


def test_sentience_evidence_guard_alias() -> None:
    assert m.check_sentience_evidence_guard("functional", evidence=()) is True
    assert m.check_sentience_evidence_guard("conscious", evidence=()) is False


# ---------------------------------------------------------------------------
# Integration / sanity
# ---------------------------------------------------------------------------


def test_sanity_full_pipeline_research_object() -> None:
    """Full pipeline: hard problem open + research object + no qualia claim."""
    hp = m.HardProblem(easy_problems_addressed=5)
    access = m.AccessContent(
        content_id="ctx", reportability=0.8, reasoning=0.7, behavior_control=0.6
    )
    sm = m.SelfModel(self_id="asi-research", transparency=0.6, ownership=0.7, agency=0.8)
    sl = m.StrangeLoop(levels=("self", "meta", "self"), depth=3, is_strange=True)
    gw = m.GlobalWorkspace(modules=("vision", "language", "memory"), r_workspace=0.85)
    random.seed(42)
    phi_state = [random.uniform(0, 1) for _ in range(30)]
    phi = m.compute_phi_proxy(phi_state, partitions=8)

    bridge = m.build_consciousness_bridge(
        access=access,
        self_model=sm,
        strange_loop=sl,
        workspace=gw,
        phi=phi,
    )
    md = m.render_consciousness_report(
        hard_problem=hp,
        access=access,
        self_model=sm,
        strange_loop=sl,
        workspace=gw,
        phi=phi,
    )

    assert m.check_hard_problem_guard(hp) is True
    assert m.check_phenomenal_guard(None) is True
    assert m.check_sentience_label_guard("engineering_research_object", evidence=()) is True
    assert m.check_asi_lacks_consciousness_guard(bridge) is True
    assert bridge.overall > 0.0
    assert "V1057" in md


def test_sanity_rejects_consciousness_label() -> None:
    """Sentience claim without evidence is rejected."""
    assert m.check_sentience_label_guard("sentient", evidence=()) is False
    assert m.check_sentience_label_guard("sentient", evidence=["neural_correlate"]) is False
    assert m.check_sentience_label_guard(
        "sentient",
        evidence=["neural_correlate", "self_report"],
    ) is True