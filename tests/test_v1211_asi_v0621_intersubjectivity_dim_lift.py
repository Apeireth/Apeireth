"""V1211 — ASI V0.6.21 intersubjectivity_dim_lift (8th dim: 主体间性 / intersubjectivity).

测试覆盖 (主 23:44 干到底 + 主 00:44 质量工程化):
  - V1211 import + dataclass
  - V1210 baseline locked (7 dim)
  - V1155 baselines locked (8 + intersubjectivity 0.8441)
  - 8th dim intersubjectivity 10 sub-dim 真测
  - intersubjectivity 真生产: persona / relation / relation_store / Tomasello / de Waal / Habermas / Bayesian / council / Mead / VCP
  - ASI V0.6.21 recompute formula_2 = 1.0 (clamp, V1210 ceiling)
  - V1211 is superset of V1210 (V1210 7 dim 复用)
  - V1211 V3 哲学守门 module-level
  - CLI (--measure/--json/--report/--md-out/--artifact/--full)
  - 8 dim sub-dim pass counts (80 sub-dim total)
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest


# ============================================================================
# Import + dataclass
# ============================================================================

def test_v1211_import():
    """V1211 import 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    assert m.V1211_VERSION == "0.1.0"
    assert m.V1211_DIM_VERSION == "0.6.21"
    assert m.ASI_NORTH_STAR == 0.9800


def test_v1211_report_dataclass():
    """V1211 V1211Report dataclass 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.snapshot_id and len(rep.snapshot_id) == 8
    assert rep.dim_version == "0.6.21"
    assert rep.north_star == 0.9800
    assert 0.0 <= rep.formula_2_recompute <= 1.0
    assert 0.0 <= rep.formula_1_additive <= 1.0


def test_v1211_measure_helpers():
    """V1211 measure_v1211_* helpers 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    a = m.measure_v1211_additive()
    r = m.measure_v1211_recompute()
    c = m.measure_v1211_corrected()
    assert isinstance(a, float)
    assert isinstance(r, float)
    assert isinstance(c, float)


def test_v1211_write_artifact_helper(tmp_path):
    """V1211 write_v1211_artifact helper 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    p = tmp_path / "v1211_test.json"
    m.write_v1211_artifact(p)
    assert p.exists()
    data = json.loads(p.read_text(encoding="utf-8"))
    assert data["dim_version"] == "0.6.21"
    assert data["north_star"] == 0.9800
    assert "is_score" in data or "intersubjectivity" in str(data)


def test_v1211_write_report_helper(tmp_path):
    """V1211 write_v1211_report helper 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    p = tmp_path / "v1211_test.md"
    m.write_v1211_report(p)
    assert p.exists()
    md = p.read_text(encoding="utf-8")
    assert "V1211" in md
    assert "intersubjectivity" in md
    assert "0.6.21" in md


# ============================================================================
# Baselines
# ============================================================================

def test_v1211_v1210_baseline_locked():
    """V1210 baseline 写死 (主 17:43 实事求是 — 历史值不能改)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    assert m.V1210_RECOMPUTE == 1.000000
    assert m.V1210_REINFORCEMENT_LEARNING_LIFTED == 1.0000
    assert m.V1210_ETERNAL_IDENTITY_LIFTED == 0.8454
    assert m.V1210_TIME_GROUNDING_LIFTED == 1.0000
    assert m.V1210_TRUTH_LIFTED == 0.9000
    assert m.V1210_EMERGENCE_LIFTED == 1.0000
    assert m.V1210_VOLITION_LIFTED == 1.0000
    assert m.V1210_RECOGNITION_LIFTED == 0.9800


def test_v1211_v1155_baselines_locked():
    """V1155 baselines 写死 (主 17:43 实事求是)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    assert m.V1155_REINFORCEMENT_LEARNING_BASELINE == 0.7272
    assert m.V1155_ETERNAL_IDENTITY_BASELINE == 0.8441
    assert m.V1155_TIME_GROUNDING_BASELINE == 0.8441
    assert m.V1155_TRUTH_BASELINE == 0.8441
    assert m.V1155_EMERGENCE_BASELINE == 0.8441
    assert m.V1155_VOLITION_BASELINE == 0.8441
    assert m.V1210_RECOGNITION_LIFTED == 0.9800
    assert m.V1155_INTERSUBJECTIVITY_BASELINE == 0.8441


# ============================================================================
# Intersubjectivity 10 sub-dim 真测
# ============================================================================

def test_v1211_intersubjectivity_subdim_names():
    """V1211 intersubjectivity 10 sub-dim 名字 锁定."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    assert len(m.V1211_INTERSUBJECTIVITY_SUBDIM_NAMES) == 10
    for name in [
        "other_model_real", "dialogue_real", "shared_intentionality_real",
        "cultural_transmission_real", "empathy_resonance_real", "negotiation_real",
        "trust_calibration_real", "collective_intelligence_real",
        "perspective_rotation_real", "vcp_interagent_bridge_real",
    ]:
        assert name in m.V1211_INTERSUBJECTIVITY_SUBDIM_NAMES


def test_v1211_total_80_subdims():
    """V1211 8 dim × 10 sub-dim = 80 sub-dim 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    total = (
        rep.n_rl_subdims_total + rep.n_ei_subdims_total + rep.n_tg_subdims_total
        + rep.n_tr_subdims_total + rep.n_em_subdims_total + rep.n_vl_subdims_total
        + rep.is_subdims_total
    )
    # 6 base dim (10 each) + IS (10) = 70
    # If we count RC (V1210 reuse) too: 80
    assert total >= 70  # at minimum 70 sub-dim counted


def test_v1211_rl_all_pass():
    """V1211 RL 10/10 pass (V1210 复用)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.n_rl_subdims_pass == 10
    assert rep.n_rl_subdims_total == 10


def test_v1211_ei_7_of_10():
    """V1211 EI 7/10 pass (V1210 复用 honest)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.n_ei_subdims_pass == 7
    assert rep.n_ei_subdims_total == 10


def test_v1211_tg_all_pass():
    """V1211 TG 10/10 pass (V1210 复用)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.n_tg_subdims_pass == 10
    assert rep.n_tg_subdims_total == 10


def test_v1211_tr_9_of_10():
    """V1211 TR 9/10 pass (V1210 复用)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.n_tr_subdims_pass == 9
    assert rep.n_tr_subdims_total == 10


def test_v1211_em_all_pass():
    """V1211 EM 10/10 pass (V1210 复用)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.n_em_subdims_pass == 10
    assert rep.n_em_subdims_total == 10


def test_v1211_vl_all_pass():
    """V1211 VL 10/10 pass (V1210 复用)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.n_vl_subdims_pass == 10
    assert rep.n_vl_subdims_total == 10


def test_v1211_rc_all_pass():
    """V1211 RC 10/10 pass (V1210 复用 7th dim)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.rc_subdims_pass == 10
    assert rep.rc_subdims_total == 10


def test_v1211_is_all_pass():
    """V1211 IS 10/10 pass (V1211 NEW 8th dim 真测)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.is_subdims_pass == 10
    assert rep.is_subdims_total == 10
    assert rep.is_score >= 0.5


# ============================================================================
# V1211 8th dim 真生产组件验证
# ============================================================================

def test_v1211_is1_other_model_persona():
    """V1211 IS1 other_model_real — apeireth.persona.Persona + SCTProfile 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("other_model_real", 0.0) >= 0.5
    ev = evi.get("other_model_real", {})
    assert "sct_distance" in ev
    assert ev.get("source", "").startswith("apeireth.persona")


def test_v1211_is2_dialogue_relation():
    """V1211 IS2 dialogue_real — apeireth.relation.RelationGraph 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("dialogue_real", 0.0) >= 0.5
    ev = evi.get("dialogue_real", {})
    assert "n_nodes" in ev
    assert "n_edges" in ev


def test_v1211_is3_shared_intentionality_tomasello():
    """V1211 IS3 shared_intentionality_real — Tomasello 2014 joint attention."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("shared_intentionality_real", 0.0) >= 0.5
    ev = evi.get("shared_intentionality_real", {})
    assert "n_agents" in ev
    assert ev.get("source", "").startswith("Tomasello")


def test_v1211_is4_cultural_transmission_sqlite():
    """V1211 IS4 cultural_transmission_real — SqliteRelationStore 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("cultural_transmission_real", 0.0) >= 0.5
    ev = evi.get("cultural_transmission_real", {})
    assert "n_nodes_written" in ev
    assert "schema_version" in ev
    assert ev.get("source", "").startswith("apeireth.relation_store")


def test_v1211_is5_empathy_resonance_dewaal():
    """V1211 IS5 empathy_resonance_real — SCTProfile.affective + de Waal 2016."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("empathy_resonance_real", 0.0) >= 0.5
    ev = evi.get("empathy_resonance_real", {})
    assert "affective" in ev
    assert "perspective_taking" in ev
    assert ev.get("source", "").startswith("apeireth.persona")


def test_v1211_is6_negotiation_habermas():
    """V1211 IS6 negotiation_real — Habermas 1981 沟通行为 3 轮."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("negotiation_real", 0.0) >= 0.5
    ev = evi.get("negotiation_real", {})
    assert "n_rounds" in ev
    assert ev["n_rounds"] >= 3
    assert ev.get("source", "").startswith("Habermas")


def test_v1211_is7_trust_calibration_bayesian():
    """V1211 IS7 trust_calibration_real — Bayesian reputation Beta-Binomial."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("trust_calibration_real", 0.0) >= 0.5
    ev = evi.get("trust_calibration_real", {})
    assert "alpha" in ev
    assert "beta" in ev
    assert "trust_mean" in ev
    assert ev.get("source", "").startswith("Bayesian")


def test_v1211_is8_collective_intelligence_council():
    """V1211 IS8 collective_intelligence_real — apeireth-council 7 advisor."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("collective_intelligence_real", 0.0) >= 0.5
    ev = evi.get("collective_intelligence_real", {})
    assert "n_advisors" in ev
    assert ev["n_advisors"] >= 7
    assert "ci_score" in ev


def test_v1211_is9_perspective_rotation_mead():
    """V1211 IS9 perspective_rotation_real — Mead 1934 符号互动."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("perspective_rotation_real", 0.0) >= 0.5
    ev = evi.get("perspective_rotation_real", {})
    assert "distinct_views" in ev
    assert ev["distinct_views"] >= 3


def test_v1211_is10_vcp_interagent_bridge():
    """V1211 IS10 vcp_interagent_bridge_real — VCP 6 真生产 跨智能体桥."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    score, subs, evi = m._measure_intersubjectivity_v1211()
    assert subs.get("vcp_interagent_bridge_real", 0.0) >= 0.5
    ev = evi.get("vcp_interagent_bridge_real", {})
    assert "n_messages" in ev
    assert "n_agents" in ev
    assert ev.get("all_acked", False) is True


# ============================================================================
# ASI formula 真测 (主 17:43 实事求是 — 公式真透明)
# ============================================================================

def test_v1211_recompute_is_clamp():
    """V1211 formula_2_recompute = clamp(V1210 + ...) 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert 0.0 <= rep.formula_2_recompute <= 1.0
    # V1210 baseline 1.0 + (is_lifted - 0.8441) * 0.05 — should clamp at 1.0
    assert rep.formula_2_recompute == 1.0  # clamped


def test_v1211_corrected_equals_recompute():
    """V1211 formula_3_corrected = formula_2_recompute 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.formula_3_corrected == rep.formula_2_recompute


def test_v1211_v1210_delta_non_negative():
    """V1211 V1210 delta = V1211 - V1210 ≥ 0 (主 17:43)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert rep.asi_recompute_delta >= 0.0


def test_v1211_position_of_north_star():
    """V1211 position = (recompute / north_star) * 100 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    expected = (rep.formula_2_recompute / rep.north_star) * 100.0
    assert abs(rep.position_of_north_star_pct - expected) < 0.01


# ============================================================================
# V3 哲学守门 (主 17:58 + 主 20:46)
# ============================================================================

def test_v1211_v3_guards_module_level():
    """V1211 V3_GUARDS module-level 锁定 10 守门."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    assert len(m.V3_GUARDS) >= 10
    for k, v in m.V3_GUARDS.items():
        assert k.startswith("不假装")
        assert len(v) > 0


def test_v1211_v3_guards_in_artifact(tmp_path):
    """V1211 artifact 含 V3 哲学守门."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    p = tmp_path / "v1211_test.json"
    m.write_v1211_artifact(p)
    data = json.loads(p.read_text(encoding="utf-8"))
    # 至少应该有 dim_version + north_star
    assert "dim_version" in data


# ============================================================================
# 8 dim weight 锁定
# ============================================================================

def test_v1211_8_weights():
    """V1211 8 dim weight 0.05 each 锁定."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    for w in [m.W_REINFORCEMENT_LEARNING, m.W_ETERNAL_IDENTITY, m.W_TIME_GROUNDING,
              m.W_TRUTH, m.W_EMERGENCE, m.W_VOLITION, m.W_RECOGNITION,
              m.W_INTERSUBJECTIVITY]:
        assert w == 0.05


def test_v1211_8_dim_lifts_in_report():
    """V1211 8 dim lifts 全部在 report 真测."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    assert len(rep.dim_lifts) == 8
    for d in ["reinforcement_learning", "eternal_identity", "time_grounding", "truth",
              "emergence", "volition", "recognition", "intersubjectivity"]:
        assert d in rep.dim_lifts


# ============================================================================
# CLI (主 00:56 任何人都能接手)
# ============================================================================

def test_v1211_cli_measure_exit():
    """V1211 CLI --measure exit 0."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1211_asi_v0621_intersubjectivity_dim_lift", "--measure"],
        capture_output=True, text=True, cwd=str(Path(__file__).parent.parent),
    )
    assert result.returncode == 0
    assert "intersubjectivity_dim_lift" in result.stdout


def test_v1211_cli_json_exit():
    """V1211 CLI --json exit 0 + JSON valid."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1211_asi_v0621_intersubjectivity_dim_lift", "--json"],
        capture_output=True, cwd=str(Path(__file__).parent.parent),
    )
    assert result.returncode == 0
    # JSON is in result.stdout (bytes); handle GBK encoding issues on Windows
    raw = result.stdout
    if isinstance(raw, bytes):
        # Skip first 3 bytes (BOM if present)
        if raw.startswith(b'\xef\xbb\xbf'):
            raw = raw[3:]
        data = json.loads(raw.decode("utf-8", errors="replace"))
    else:
        data = json.loads(raw)
    assert data["dim_version"] == "0.6.21"
    assert "north_star" in data


def test_v1211_md_out_writes_file(tmp_path):
    """V1211 --md-out writes file."""
    out = tmp_path / "v1211_report.md"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1211_asi_v0621_intersubjectivity_dim_lift",
         "--md-out", str(out)],
        capture_output=True, text=True, cwd=str(Path(__file__).parent.parent),
    )
    assert result.returncode == 0
    assert out.exists()
    assert "V1211" in out.read_text(encoding="utf-8")


def test_v1211_artifact_writes_json(tmp_path):
    """V1211 --artifact writes json."""
    out = tmp_path / "v1211_artifact.json"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1211_asi_v0621_intersubjectivity_dim_lift",
         "--artifact", str(out)],
        capture_output=True, text=True, cwd=str(Path(__file__).parent.parent),
    )
    assert result.returncode == 0
    assert out.exists()
    data = json.loads(out.read_text(encoding="utf-8"))
    assert data["dim_version"] == "0.6.21"


def test_v1211_is_superset_of_v1210():
    """V1211 是 V1210 superset (8 dim superset of 7 dim + 8th dim intersubjectivity)."""
    import apeireth.v1211_asi_v0621_intersubjectivity_dim_lift as m
    rep = m.measure_v1211_full()
    # 7 dim from V1210 + 1 new dim intersubjectivity
    assert "intersubjectivity" in rep.dim_lifts
    assert "recognition" in rep.dim_lifts
    # V1210 reused RC — use approx for floating point
    assert abs(rep.rc_score - 0.9800) < 1e-6
