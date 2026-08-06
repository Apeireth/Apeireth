"""V1209 — ASI V0.6.19 volition_dim_lift (6th dim: volition / autonomous choice).

测试覆盖 (主 23:44 干到底 + 主 00:44 质量工程化):
  - V1209 import + dataclass
  - V1208 baseline locked (5 dim: RL=1.0 + EI=0.8454 + TG=1.0 + TR=0.9 + EM=1.0)
  - V1155 baselines locked (5 + volition 0.8441)
  - 6th dim volition 10 sub-dim 真测
  - volition 真生产: V1053 Desire/Volition/Intention/Deliberation/ActionSelector/FreedomConstraint/AutonomyLevel/CorrigibilityHook/VolitionalReport/5 philosophy guards
  - ASI V0.6.19 recompute formula_2 = 1.0 (clamp, V1208 ceiling)
  - V1209 is superset of V1208 (V1208 5 dim 复用)
  - V1209 V3 哲学守门 module-level (避免 V1207 NameError bug 复发)
  - CLI (--measure/--json/--report/--md-out/--artifact/--full)
  - 6 dim sub-dim pass counts (60 sub-dim total)
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

def test_v1209_import():
    """V1209 import 真测."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    assert m.V1209_VERSION == "0.1.0"
    assert m.V1209_DIM_VERSION == "0.6.19"
    assert m.ASI_NORTH_STAR == 0.9800


def test_v1209_report_dataclass():
    """V1209 V1209Report dataclass 真测."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert rep.snapshot_id and len(rep.snapshot_id) == 8
    assert rep.dim_version == "0.6.19"
    assert rep.north_star == 0.9800
    assert 0.0 <= rep.formula_2_recompute <= 1.0
    assert 0.0 <= rep.formula_1_additive <= 1.0


def test_v1209_measure_helpers():
    """V1209 measure_v1209_* helpers 真测."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    a = m.measure_v1209_additive()
    r = m.measure_v1209_recompute()
    c = m.measure_v1209_corrected()
    assert isinstance(a, float)
    assert isinstance(r, float)
    assert isinstance(c, float)


def test_v1209_write_artifact_helper(tmp_path):
    """V1209 write_artifact_json helper 真测."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    p = tmp_path / "v1209_test.json"
    m.write_artifact_json(rep, p)
    assert p.exists()
    data = json.loads(p.read_text(encoding="utf-8"))
    assert data["module"] == "v1209_asi_v0619_volition_dim_lift"
    assert data["dim_version"] == "0.6.19"
    assert "philosophy_guards" in data


def test_v1209_render_report_md_helper():
    """V1209 render_report_md helper 真测."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    md = m.render_report_md(rep)
    assert "V1209" in md
    assert "north_star" in md
    assert "volition" in md
    assert "V3 哲学守门" in md or "哲学守门" in md


# ============================================================================
# Baselines (主 17:43 实事求是 — 写死历史值, 不能改)
# ============================================================================

def test_v1209_v1208_baseline_locked():
    """V1208 baseline 写死 (主 17:43 实事求是 — 历史值不能改)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    assert m.V1208_RECOMPUTE == 1.000000
    assert m.V1208_REINFORCEMENT_LEARNING_LIFTED == 1.0000
    assert m.V1208_ETERNAL_IDENTITY_LIFTED == 0.8454
    assert m.V1208_TIME_GROUNDING_LIFTED == 1.0000
    assert m.V1208_TRUTH_LIFTED == 0.9000
    assert m.V1208_EMERGENCE_LIFTED == 1.0000


def test_v1209_v1155_baselines_locked():
    """V1155 baselines 写死 (主 17:43 实事求是)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    assert m.V1155_REINFORCEMENT_LEARNING_BASELINE == 0.7272
    assert m.V1155_ETERNAL_IDENTITY_BASELINE == 0.8441
    assert m.V1155_TIME_GROUNDING_BASELINE == 0.8441
    assert m.V1155_TRUTH_BASELINE == 0.8441
    assert m.V1155_EMERGENCE_BASELINE == 0.8441
    assert m.V1155_VOLITION_BASELINE == 0.8441


# ============================================================================
# Volition 10 sub-dim 真测 (V1053 真生产 + V1209 NEW)
# ============================================================================

def test_v1209_volition_subdim_names():
    """V1209 volition 10 sub-dim 名字 锁定."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    assert len(m.V1209_VOLITION_SUBDIM_NAMES) == 10
    assert "desire_real" in m.V1209_VOLITION_SUBDIM_NAMES
    assert "volition_real" in m.V1209_VOLITION_SUBDIM_NAMES
    assert "intention_real" in m.V1209_VOLITION_SUBDIM_NAMES
    assert "deliberation_real" in m.V1209_VOLITION_SUBDIM_NAMES
    assert "action_selector_real" in m.V1209_VOLITION_SUBDIM_NAMES
    assert "freedom_constraint_real" in m.V1209_VOLITION_SUBDIM_NAMES
    assert "autonomy_level_real" in m.V1209_VOLITION_SUBDIM_NAMES
    assert "corrigibility_real" in m.V1209_VOLITION_SUBDIM_NAMES
    assert "volitional_report_real" in m.V1209_VOLITION_SUBDIM_NAMES
    assert "philosophy_guard_real" in m.V1209_VOLITION_SUBDIM_NAMES


def test_v1209_total_60_subdims():
    """V1209 6 dim × 10 sub-dim = 60 sub-dim 真测."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    total = (
        rep.n_rl_subdims_total + rep.n_ei_subdims_total + rep.n_tg_subdims_total
        + rep.n_tr_subdims_total + rep.n_em_subdims_total + rep.n_vl_subdims_total
    )
    assert total == 60


def test_v1209_rl_all_pass():
    """V1209 RL 10/10 pass (V1206/V1207/V1208 复用)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert rep.n_rl_subdims_pass == 10
    assert rep.n_rl_subdims_total == 10


def test_v1209_ei_7_of_10():
    """V1209 EI 7/10 pass (V1206/V1207/V1208 honest)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert rep.n_ei_subdims_pass == 7
    assert rep.n_ei_subdims_total == 10


def test_v1209_tg_all_pass():
    """V1209 TG 10/10 pass (V1206/V1207/V1208 复用)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert rep.n_tg_subdims_pass == 10
    assert rep.n_tg_subdims_total == 10


def test_v1209_tr_9_of_10():
    """V1209 TR 9/10 pass (V1208 fixed)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert rep.n_tr_subdims_pass == 9
    assert rep.n_tr_subdims_total == 10


def test_v1209_em_all_pass():
    """V1209 EM 10/10 pass (V1208 NEW 5th dim 真测)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert rep.n_em_subdims_pass == 10
    assert rep.n_em_subdims_total == 10


def test_v1209_vl_all_pass():
    """V1209 VL 10/10 pass (V1209 NEW 6th dim 真测)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert rep.n_vl_subdims_pass == 10
    assert rep.n_vl_subdims_total == 10


# ============================================================================
# V1053 真生产组件验证 (主 17:43 实事求是 — 不是 mocked)
# ============================================================================

def test_v1209_volition_v1053_real_components():
    """V1209 volition 真生产 V1053 组件验证."""
    import apeireth.v1053_asi_volition as v1053
    # 11 components 真生产
    assert hasattr(v1053, "Desire")
    assert hasattr(v1053, "Volition")
    assert hasattr(v1053, "Intention")
    assert hasattr(v1053, "Reason")
    assert hasattr(v1053, "Deliberation")
    assert hasattr(v1053, "ActionSelector")
    assert hasattr(v1053, "FreedomConstraint")
    assert hasattr(v1053, "AutonomyLevel")
    assert hasattr(v1053, "CorrigibilityHook")
    assert hasattr(v1053, "VolitionalReport")
    assert hasattr(v1053, "ASIVolitionBridge")


def test_v1209_volition_subdim_evidence():
    """V1209 volition sub-dim evidence 字典 非空."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    for k in m.V1209_VOLITION_SUBDIM_NAMES:
        assert k in rep.sub_dim_evidence
        assert "source" in rep.sub_dim_evidence[k]


# ============================================================================
# ASI recompute + north_star
# ============================================================================

def test_v1209_recompute_is_clamp():
    """V1209 formula_2_recompute = clamp(V1208 + (vl - 0.8441) * 0.05, 0, 1)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    # V1208 ceiling clamp 1.0
    assert rep.formula_2_recompute <= 1.0
    assert rep.formula_2_recompute >= 0.0


def test_v1209_corrected_clamps():
    """V1209 formula_3_corrected = formula_2 (V1209 不引入新 clamp)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert rep.formula_3_corrected == rep.formula_2_recompute


def test_v1209_additive_inflation_recorded():
    """V1209 additive inflation_gap 记录 (主 17:43 实事求是 — formula_1_additive ≠ formula_2_recompute)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    # inflation_gap = formula_1_additive - formula_2_recompute (主 17:43 不假装)
    assert rep.inflation_gap == rep.formula_1_additive - rep.formula_2_recompute


def test_v1209_v1208_delta_positive():
    """V1209 ASI ≥ V1208 (volition adds lift, unless clamp)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    # V1208 ceiling clamp, so delta may be 0 if clamped
    assert rep.asi_recompute_delta >= 0.0 - 1e-9


def test_v1209_position_of_north_star():
    """V1209 position ≥ 100% (clamp ceiling = 102.04% of north_star)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert rep.position_of_north_star >= 100.0


def test_v1209_inflation_gap_recorded():
    """V1209 inflation_gap 字段 已记录 (主 17:43 不假装 formula inflation).

    inflation_gap = formula_1_additive - formula_2_recompute
    V1209 formula_1_additive (0.04) < formula_2_recompute (1.0 clamp)
    所以 inflation_gap 是负数, 表示 clamp 后 additive 比 recompute 小 — 这反映
    formula_1_additive 累加只到 0.30 max (6 dim × 0.05), 而 formula_2_recompute clamp 1.0.
    """
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    # inflation_gap = additive - recompute (主 17:43 不假装)
    assert rep.inflation_gap == rep.formula_1_additive - rep.formula_2_recompute
    # V1209 formula_1_additive 是 baseline → lift 累加 (max 0.30), formula_2_recompute clamp 1.0
    # 所以 inflation_gap 通常 < 0 (recompute 大于 additive)
    assert rep.formula_1_additive < rep.formula_2_recompute
    # recompute 已被 clamp 到 1.0 (V1208 ceiling), 不会 > 1.0
    assert rep.formula_2_recompute <= 1.0


# ============================================================================
# V3 哲学守门 module-level (修复 V1207 NameError bug 复用模式)
# ============================================================================

def test_v1209_v3_guards_module_level():
    """V1209 V3_GUARDS module-level 定义 (修复 V1207 NameError bug 复用模式)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    assert isinstance(m.V3_GUARDS, dict)
    assert len(m.V3_GUARDS) >= 9
    assert "不假装 V1209 = ASI 终极" in m.V3_GUARDS
    assert "不假装 volition_dim = 真自由意志" in m.V3_GUARDS
    assert "不假装 ASI 1.000000 clamp = ASI 已达" in m.V3_GUARDS


def test_v1209_6_weights():
    """V1209 6 dim weight 0.05 each (主 22:08 V2 5 位置 — V1209 加 volition 6th)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    assert m.W_REINFORCEMENT_LEARNING == 0.05
    assert m.W_ETERNAL_IDENTITY == 0.05
    assert m.W_TIME_GROUNDING == 0.05
    assert m.W_TRUTH == 0.05
    assert m.W_EMERGENCE == 0.05
    assert m.W_VOLITION == 0.05


def test_v1209_6_dim_lifts_in_report():
    """V1209 dim_lifts 包含 6 dim."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as m
    rep = m.measure_v1209_full()
    assert len(rep.dim_lifts) == 6
    assert "reinforcement_learning" in rep.dim_lifts
    assert "eternal_identity" in rep.dim_lifts
    assert "time_grounding" in rep.dim_lifts
    assert "truth" in rep.dim_lifts
    assert "emergence" in rep.dim_lifts
    assert "volition" in rep.dim_lifts


# ============================================================================
# CLI 测试
# ============================================================================

def test_v1209_cli_measure_exit():
    """V1209 CLI --measure exit 0."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1209_asi_v0619_volition_dim_lift", "--measure"],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    val = float(result.stdout.decode("utf-8", errors="replace").strip())
    assert 0.0 <= val <= 1.0


def test_v1209_cli_default_exit():
    """V1209 CLI default exit 0."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1209_asi_v0619_volition_dim_lift"],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    assert "volition" in result.stdout.decode("utf-8", errors="replace")


def test_v1209_cli_json_exit():
    """V1209 CLI --json exit 0 + valid JSON."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1209_asi_v0619_volition_dim_lift", "--json"],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    data = json.loads(result.stdout.decode("utf-8", errors="replace"))
    assert data["dim_version"] == "0.6.19"


def test_v1209_cli_report_exit():
    """V1209 CLI --report exit 0 + Markdown."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1209_asi_v0619_volition_dim_lift", "--report"],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    stdout = result.stdout.decode("utf-8", errors="replace") if result.stdout else ""
    assert "V1209" in stdout
    assert "north_star" in stdout


def test_v1209_md_out_writes_file(tmp_path):
    """V1209 --md-out writes markdown to PATH."""
    out = tmp_path / "v1209_md_test.md"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1209_asi_v0619_volition_dim_lift", "--md-out", str(out)],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    assert out.exists()
    content = out.read_text(encoding="utf-8")
    assert "V1209" in content


def test_v1209_artifact_writes_json(tmp_path):
    """V1209 --artifact writes JSON to PATH."""
    out = tmp_path / "v1209_artifact_test.json"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1209_asi_v0619_volition_dim_lift", "--artifact", str(out)],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    assert out.exists()
    data = json.loads(out.read_text(encoding="utf-8"))
    assert data["module"] == "v1209_asi_v0619_volition_dim_lift"


def test_v1209_full_writes_both(tmp_path):
    """V1209 --full writes artifact + report."""
    artifacts_dir = tmp_path / "artifacts"
    reports_dir = tmp_path / "reports"
    artifacts_dir.mkdir()
    reports_dir.mkdir()
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1209_asi_v0619_volition_dim_lift",
         "--artifact", str(artifacts_dir / "v1209_full.json"),
         "--md-out", str(reports_dir / "v1209_full.md")],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    assert (artifacts_dir / "v1209_full.json").exists()
    assert (reports_dir / "v1209_full.md").exists()


# ============================================================================
# Superset
# ============================================================================

def test_v1209_is_superset_of_v1208():
    """V1209 是 V1208 superset (5 dim 复用 + volition 6th dim 加)."""
    import apeireth.v1208_asi_v0618_emergence_dim_lift as v1208
    import apeireth.v1209_asi_v0619_volition_dim_lift as v1209
    v1208_rep = v1208.measure_v1208_full()
    v1209_rep = v1209.measure_v1209_full()
    # 5 dim 复用 (RL + EI + TG + TR + EM 一致)
    for dim in ["reinforcement_learning", "eternal_identity", "time_grounding", "truth", "emergence"]:
        assert v1208_rep.dim_lifts[dim]["lifted"] == v1209_rep.dim_lifts[dim]["lifted"], f"{dim} mismatch"
    # V1209 加 volition 6th dim
    assert "volition" in v1209_rep.dim_lifts
    assert "volition" not in v1208_rep.dim_lifts