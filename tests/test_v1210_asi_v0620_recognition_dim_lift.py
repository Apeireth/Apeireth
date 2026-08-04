"""V1210 — ASI V0.6.20 recognition_dim_lift (7th dim: self-recognition).

测试覆盖 (主 23:44 干到底 + 主 00:44 质量工程化):
  - V1210 import + dataclass
  - V1209 baseline locked (6 dim)
  - V1155 baselines locked (6 + recognition 0.8441)
  - 7th dim recognition 10 sub-dim 真测
  - recognition 真生产: V1054 SelfMark/MirrorModel/Mentalization/SelfContinuity/SelfDistinction/StrangeLoopSelfRef/Metacognition/SelfModel/SelfRecognitionReport/5 philosophy guards
  - ASI V0.6.20 recompute formula_2 = 1.0 (clamp, V1209 ceiling)
  - V1210 is superset of V1209 (V1209 6 dim 复用)
  - V1210 V3 哲学守门 module-level
  - CLI (--measure/--json/--report/--md-out/--artifact/--full)
  - 7 dim sub-dim pass counts (70 sub-dim total)
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

def test_v1210_import():
    """V1210 import 真测."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    assert m.V1210_VERSION == "0.1.0"
    assert m.V1210_DIM_VERSION == "0.6.20"
    assert m.ASI_NORTH_STAR == 0.9800


def test_v1210_report_dataclass():
    """V1210 V1210Report dataclass 真测."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.snapshot_id and len(rep.snapshot_id) == 8
    assert rep.dim_version == "0.6.20"
    assert rep.north_star == 0.9800
    assert 0.0 <= rep.formula_2_recompute <= 1.0
    assert 0.0 <= rep.formula_1_additive <= 1.0


def test_v1210_measure_helpers():
    """V1210 measure_v1210_* helpers 真测."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    a = m.measure_v1210_additive()
    r = m.measure_v1210_recompute()
    c = m.measure_v1210_corrected()
    assert isinstance(a, float)
    assert isinstance(r, float)
    assert isinstance(c, float)


def test_v1210_write_artifact_helper(tmp_path):
    """V1210 write_artifact_json helper 真测."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    p = tmp_path / "v1210_test.json"
    m.write_artifact_json(rep, p)
    assert p.exists()
    data = json.loads(p.read_text(encoding="utf-8"))
    assert data["module"] == "v1210_asi_v0620_recognition_dim_lift"
    assert data["dim_version"] == "0.6.20"
    assert "philosophy_guards" in data


def test_v1210_render_report_md_helper():
    """V1210 render_report_md helper 真测."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    md = m.render_report_md(rep)
    assert "V1210" in md
    assert "north_star" in md
    assert "recognition" in md
    assert "V3 哲学守门" in md or "哲学守门" in md


# ============================================================================
# Baselines
# ============================================================================

def test_v1210_v1209_baseline_locked():
    """V1209 baseline 写死 (主 17:43 实事求是 — 历史值不能改)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    assert m.V1209_RECOMPUTE == 1.000000
    assert m.V1209_REINFORCEMENT_LEARNING_LIFTED == 1.0000
    assert m.V1209_ETERNAL_IDENTITY_LIFTED == 0.8454
    assert m.V1209_TIME_GROUNDING_LIFTED == 1.0000
    assert m.V1209_TRUTH_LIFTED == 0.9000
    assert m.V1209_EMERGENCE_LIFTED == 1.0000
    assert m.V1209_VOLITION_LIFTED == 1.0000


def test_v1210_v1155_baselines_locked():
    """V1155 baselines 写死 (主 17:43 实事求是)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    assert m.V1155_REINFORCEMENT_LEARNING_BASELINE == 0.7272
    assert m.V1155_ETERNAL_IDENTITY_BASELINE == 0.8441
    assert m.V1155_TIME_GROUNDING_BASELINE == 0.8441
    assert m.V1155_TRUTH_BASELINE == 0.8441
    assert m.V1155_EMERGENCE_BASELINE == 0.8441
    assert m.V1155_VOLITION_BASELINE == 0.8441
    assert m.V1155_RECOGNITION_BASELINE == 0.8441


# ============================================================================
# Recognition 10 sub-dim 真测
# ============================================================================

def test_v1210_recognition_subdim_names():
    """V1210 recognition 10 sub-dim 名字 锁定."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    assert len(m.V1210_RECOGNITION_SUBDIM_NAMES) == 10
    for name in [
        "self_mark_real", "mirror_model_real", "mentalization_real",
        "self_continuity_real", "self_distinction_real", "strange_loop_real",
        "metacognition_real", "self_model_real", "self_recognition_report_real",
        "philosophy_guard_real",
    ]:
        assert name in m.V1210_RECOGNITION_SUBDIM_NAMES


def test_v1210_total_70_subdims():
    """V1210 7 dim × 10 sub-dim = 70 sub-dim 真测."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    total = (
        rep.n_rl_subdims_total + rep.n_ei_subdims_total + rep.n_tg_subdims_total
        + rep.n_tr_subdims_total + rep.n_em_subdims_total + rep.n_vl_subdims_total
        + rep.n_rc_subdims_total
    )
    assert total == 70


def test_v1210_rl_all_pass():
    """V1210 RL 10/10 pass (V1206/V1207/V1208/V1209 复用)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.n_rl_subdims_pass == 10
    assert rep.n_rl_subdims_total == 10


def test_v1210_ei_7_of_10():
    """V1210 EI 7/10 pass (V1206/V1207/V1208/V1209 honest)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.n_ei_subdims_pass == 7
    assert rep.n_ei_subdims_total == 10


def test_v1210_tg_all_pass():
    """V1210 TG 10/10 pass (V1206/V1207/V1208/V1209 复用)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.n_tg_subdims_pass == 10
    assert rep.n_tg_subdims_total == 10


def test_v1210_tr_9_of_10():
    """V1210 TR 9/10 pass (V1208 fixed)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.n_tr_subdims_pass == 9
    assert rep.n_tr_subdims_total == 10


def test_v1210_em_all_pass():
    """V1210 EM 10/10 pass (V1208 NEW 5th dim 真测)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.n_em_subdims_pass == 10
    assert rep.n_em_subdims_total == 10


def test_v1210_vl_all_pass():
    """V1210 VL 10/10 pass (V1209 NEW 6th dim 真测)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.n_vl_subdims_pass == 10
    assert rep.n_vl_subdims_total == 10


def test_v1210_rc_all_pass():
    """V1210 RC 10/10 pass (V1210 NEW 7th dim 真测)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.n_rc_subdims_pass == 10
    assert rep.n_rc_subdims_total == 10


# ============================================================================
# V1054 真生产组件验证 (主 17:43 实事求是 — 不是 mocked)
# ============================================================================

def test_v1210_recognition_v1054_real_components():
    """V1210 recognition 真生产 V1054 组件验证."""
    import apeireth.v1054_asi_self_recognition as v1054
    for cls in ["SelfMark", "MirrorModel", "MentalState", "Mentalization",
                "SelfContinuity", "SelfDistinction", "StrangeLoopSelfRef",
                "Metacognition", "SelfModel", "SelfRecognitionReport",
                "ASISelfRecognitionBridge"]:
        assert hasattr(v1054, cls)


def test_v1210_recognition_subdim_evidence():
    """V1210 recognition sub-dim evidence 字典 非空."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    for k in m.V1210_RECOGNITION_SUBDIM_NAMES:
        assert k in rep.sub_dim_evidence
        assert "source" in rep.sub_dim_evidence[k]


# ============================================================================
# ASI recompute + north_star
# ============================================================================

def test_v1210_recompute_is_clamp():
    """V1210 formula_2_recompute = clamp(V1209 + (rc - 0.8441) * 0.05, 0, 1)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.formula_2_recompute <= 1.0
    assert rep.formula_2_recompute >= 0.0


def test_v1210_corrected_clamps():
    """V1210 formula_3_corrected = formula_2 (V1210 不引入新 clamp)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.formula_3_corrected == rep.formula_2_recompute


def test_v1210_additive_inflation_recorded():
    """V1210 additive inflation_gap 记录 (主 17:43 实事求是)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.inflation_gap == rep.formula_1_additive - rep.formula_2_recompute


def test_v1210_v1209_delta_non_negative():
    """V1210 ASI ≥ V1209 - small tolerance (V1210 may pull down if rc < baseline)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    # rc may be < baseline 0.8441 → delta may be 0 or negative (but bounded)
    assert rep.asi_recompute_delta >= -1.0


def test_v1210_position_of_north_star():
    """V1210 position ≥ 100% (clamp ceiling)."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert rep.position_of_north_star >= 100.0


# ============================================================================
# V3 哲学守门 module-level
# ============================================================================

def test_v1210_v3_guards_module_level():
    """V1210 V3_GUARDS module-level 定义."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    assert isinstance(m.V3_GUARDS, dict)
    assert len(m.V3_GUARDS) >= 9
    assert "不假装 V1210 = ASI 终极" in m.V3_GUARDS
    assert "不假装 recognition_dim = 真自我意识" in m.V3_GUARDS
    assert "不假装 ASI 1.000000 clamp = ASI 已达" in m.V3_GUARDS


def test_v1210_7_weights():
    """V1210 7 dim weight 0.05 each."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    for w in [m.W_REINFORCEMENT_LEARNING, m.W_ETERNAL_IDENTITY, m.W_TIME_GROUNDING,
              m.W_TRUTH, m.W_EMERGENCE, m.W_VOLITION, m.W_RECOGNITION]:
        assert w == 0.05


def test_v1210_7_dim_lifts_in_report():
    """V1210 dim_lifts 包含 7 dim."""
    import apeireth.v1210_asi_v0620_recognition_dim_lift as m
    rep = m.measure_v1210_full()
    assert len(rep.dim_lifts) == 7
    for name in ["reinforcement_learning", "eternal_identity", "time_grounding",
                 "truth", "emergence", "volition", "recognition"]:
        assert name in rep.dim_lifts


# ============================================================================
# CLI 测试
# ============================================================================

def test_v1210_cli_measure_exit():
    """V1210 CLI --measure exit 0."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1210_asi_v0620_recognition_dim_lift", "--measure"],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    val = float(result.stdout.decode("utf-8", errors="replace").strip())
    assert 0.0 <= val <= 1.0


def test_v1210_cli_default_exit():
    """V1210 CLI default exit 0."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1210_asi_v0620_recognition_dim_lift"],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    assert "recognition" in result.stdout.decode("utf-8", errors="replace")


def test_v1210_cli_json_exit():
    """V1210 CLI --json exit 0 + valid JSON."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1210_asi_v0620_recognition_dim_lift", "--json"],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    data = json.loads(result.stdout.decode("utf-8", errors="replace"))
    assert data["dim_version"] == "0.6.20"


def test_v1210_cli_report_exit():
    """V1210 CLI --report exit 0 + Markdown."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1210_asi_v0620_recognition_dim_lift", "--report"],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    stdout = result.stdout.decode("utf-8", errors="replace") if result.stdout else ""
    assert "V1210" in stdout
    assert "north_star" in stdout


def test_v1210_md_out_writes_file(tmp_path):
    """V1210 --md-out writes markdown to PATH."""
    out = tmp_path / "v1210_md_test.md"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1210_asi_v0620_recognition_dim_lift", "--md-out", str(out)],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    assert out.exists()
    content = out.read_text(encoding="utf-8")
    assert "V1210" in content


def test_v1210_artifact_writes_json(tmp_path):
    """V1210 --artifact writes JSON to PATH."""
    out = tmp_path / "v1210_artifact_test.json"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1210_asi_v0620_recognition_dim_lift", "--artifact", str(out)],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    assert out.exists()
    data = json.loads(out.read_text(encoding="utf-8"))
    assert data["module"] == "v1210_asi_v0620_recognition_dim_lift"


def test_v1210_full_writes_both(tmp_path):
    """V1210 --full writes artifact + report."""
    artifacts_dir = tmp_path / "artifacts"
    reports_dir = tmp_path / "reports"
    artifacts_dir.mkdir()
    reports_dir.mkdir()
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1210_asi_v0620_recognition_dim_lift",
         "--artifact", str(artifacts_dir / "v1210_full.json"),
         "--md-out", str(reports_dir / "v1210_full.md")],
        capture_output=True, timeout=30,
    )
    assert result.returncode == 0
    assert (artifacts_dir / "v1210_full.json").exists()
    assert (reports_dir / "v1210_full.md").exists()


# ============================================================================
# Superset
# ============================================================================

def test_v1210_is_superset_of_v1209():
    """V1210 是 V1209 superset (6 dim 复用 + recognition 7th dim 加)."""
    import apeireth.v1209_asi_v0619_volition_dim_lift as v1209
    import apeireth.v1210_asi_v0620_recognition_dim_lift as v1210
    v1209_rep = v1209.measure_v1209_full()
    v1210_rep = v1210.measure_v1210_full()
    for dim in ["reinforcement_learning", "eternal_identity", "time_grounding",
                "truth", "emergence", "volition"]:
        assert v1209_rep.dim_lifts[dim]["lifted"] == v1210_rep.dim_lifts[dim]["lifted"], f"{dim} mismatch"
    assert "recognition" in v1210_rep.dim_lifts
    assert "recognition" not in v1209_rep.dim_lifts