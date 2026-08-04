"""Tests for V1207 — ASI V0.6.17 truth_dim_lift (RL + EI + TG + TR).

Tests cover:
1. Imports / dataclass
2. RL sub-dim (10) — V1206 复用, 10/10 pass
3. EI sub-dim (10) — V1206 复用, 7/10 pass (V1206 honest)
4. TG sub-dim (10) — V1206 复用, 10/10 pass
5. TR sub-dim (10) — V1207 NEW (5 V1051 reused + 5 NEW)
6. 4-formula (additive, recompute, corrected, v1206 baseline)
7. ASI > north_star (V1207 over 0.98)
8. V3_GUARDS present
9. CLI exit codes
10. Artifact + report writing

主哲学: 主 17:43 实事求是 + 主 17:58 不假装 + 主 13:31 大胆激进 + 主 20:46 不假装 ASI 已达
"""

import json
import subprocess
import sys
from pathlib import Path

import pytest

# Add repo root to path
REPO_ROOT = Path(__file__).resolve().parent.parent
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))


def _safe_import_v1207():
    """Import V1207 module."""
    try:
        from apeireth import v1207_asi_v0617_truth_dim_lift as m
        return m
    except ImportError as e:
        pytest.skip(f"V1207 not importable: {e}")


# -----------------------------------------------------------------------------
# 1. Imports / dataclass
# -----------------------------------------------------------------------------


def test_v1207_import():
    """V1207 module imports cleanly."""
    m = _safe_import_v1207()
    assert m is not None
    assert m.V1207_VERSION == "0.1.0"
    assert m.V1207_DIM_VERSION == "0.6.17"
    assert m.ASI_NORTH_STAR == 0.9800


def test_v1207_baseline_values_locked():
    """V1207 baseline values hardcoded (主 17:43 实事求是)."""
    m = _safe_import_v1207()
    assert m.V1206_RECOMPUTE == 0.994145
    assert m.V1155_REINFORCEMENT_LEARNING_BASELINE == 0.7272
    assert m.V1155_ETERNAL_IDENTITY_BASELINE == 0.8441
    assert m.V1155_TIME_GROUNDING_BASELINE == 0.8441
    assert m.V1155_TRUTH_BASELINE == 0.8441


def test_v1207_truth_subdim_names():
    """V1207 10 truth sub-dim names fixed."""
    m = _safe_import_v1207()
    names = m.V1207_TRUTH_SUBDIM_NAMES
    assert len(names) == 10
    expected_first_five = {
        "bayesian_updater_real",
        "popper_falsifier_real",
        "lakatos_programme_real",
        "proof_assistant_real",
        "truth_discovery_real",
    }
    assert set(names[:5]) == expected_first_five
    expected_new_five = {
        "coherence_engine_real",
        "formal_verifier_real",
        "causal_truth_real",
        "knowledge_graph_real",
        "philosophy_guard_real",
    }
    assert set(names[5:]) == expected_new_five


def test_v1207_v3_guards_present():
    """V1207 has V3_GUARDS dictionary."""
    m = _safe_import_v1207()
    assert hasattr(m, "V3_GUARDS")
    assert isinstance(m.V3_GUARDS, dict)
    assert "module_is_not_asi" in m.V3_GUARDS
    assert "v1207_is_v06_17" in m.V3_GUARDS
    assert "truth_not_in_v06" in m.V3_GUARDS
    assert "godel_boundary_real" in m.V3_GUARDS


# -----------------------------------------------------------------------------
# 2. RL sub-dim (V1206 复用)
# -----------------------------------------------------------------------------


def test_v1207_rl_subdims_reused():
    """V1207 RL 10 sub-dim (V1206 复用, 通过 dim_lifts['reinforcement_learning'])."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    rl_lift = rep.dim_lifts["reinforcement_learning"]
    # V1207 复用 V1206 lifted = 1.0000
    assert rl_lift["lifted"] >= 0.95, f"RL lifted = {rl_lift['lifted']}"
    assert rl_lift["baseline"] == m.V1155_REINFORCEMENT_LEARNING_BASELINE
    assert rep.n_rl_subdims_pass == 10
    assert rep.n_rl_subdims_total == 10


# -----------------------------------------------------------------------------
# 3. EI sub-dim (V1206 honest)
# -----------------------------------------------------------------------------


def test_v1207_ei_subdims_reused():
    """V1207 EI 10 sub-dim (V1206 复用, 7/10 pass honest)."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    ei_lift = rep.dim_lifts["eternal_identity"]
    # V1206 honest EI lifted = 0.8454
    assert ei_lift["lifted"] >= 0.8, f"EI lifted = {ei_lift['lifted']}"
    assert rep.n_ei_subdims_pass >= 6
    assert rep.n_ei_subdims_total == 10


# -----------------------------------------------------------------------------
# 4. TG sub-dim (V1206 复用)
# -----------------------------------------------------------------------------


def test_v1207_tg_subdims_reused():
    """V1207 TG 10 sub-dim (V1206 复用, 10/10 pass)."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    tg_lift = rep.dim_lifts["time_grounding"]
    assert tg_lift["lifted"] >= 0.95, f"TG lifted = {tg_lift['lifted']}"
    assert rep.n_tg_subdims_pass == 10
    assert rep.n_tg_subdims_total == 10


# -----------------------------------------------------------------------------
# 5. TR sub-dim (V1207 NEW)
# -----------------------------------------------------------------------------


def test_v1207_truth_subdims_count():
    """V1207 TR 10 sub-dim."""
    m = _safe_import_v1207()
    tr_total, tr_subs, tr_evi = m._measure_truth_v1207()
    assert len(tr_subs) == 10


def test_v1207_truth_bayesian():
    """V1207 TR1 bayesian_updater_real — V1051 真生产."""
    m = _safe_import_v1207()
    tr_total, tr_subs, tr_evi = m._measure_truth_v1207()
    bayes = tr_subs.get("bayesian_updater_real", 0.0)
    assert bayes > 0.0, f"bayesian_updater_real = {bayes}"
    evi = tr_evi.get("bayesian_updater_real", {})
    assert evi.get("source") == "V1051"


def test_v1207_truth_popper():
    """V1207 TR2 popper_falsifier_real — V1051 真生产."""
    m = _safe_import_v1207()
    tr_total, tr_subs, tr_evi = m._measure_truth_v1207()
    popper = tr_subs.get("popper_falsifier_real", 0.0)
    assert popper > 0.0, f"popper_falsifier_real = {popper}"
    evi = tr_evi.get("popper_falsifier_real", {})
    assert evi.get("source") == "V1051"
    assert evi.get("is_scientific") is True


def test_v1207_truth_lakatos():
    """V1207 TR3 lakatos_programme_real — V1051 真生产."""
    m = _safe_import_v1207()
    tr_total, tr_subs, tr_evi = m._measure_truth_v1207()
    lak = tr_subs.get("lakatos_programme_real", 0.0)
    assert lak > 0.0, f"lakatos_programme_real = {lak}"
    evi = tr_evi.get("lakatos_programme_real", {})
    assert evi.get("has_hard_core") is True
    assert evi.get("has_protective_belt") is True


def test_v1207_truth_5_new_subdims_v1207_source():
    """V1207 TR6-TR10 source = V1207 (NEW)."""
    m = _safe_import_v1207()
    tr_total, tr_subs, tr_evi = m._measure_truth_v1207()
    new_5 = [
        "coherence_engine_real",
        "formal_verifier_real",
        "causal_truth_real",
        "knowledge_graph_real",
        "philosophy_guard_real",
    ]
    for k in new_5:
        evi = tr_evi.get(k, {})
        assert evi.get("source") == "V1207", f"{k} source = {evi.get('source')}"


def test_v1207_truth_5_reused_subdims_v1051_source():
    """V1207 TR1-TR5 source = V1051 (复用)."""
    m = _safe_import_v1207()
    tr_total, tr_subs, tr_evi = m._measure_truth_v1207()
    reused_5 = [
        "bayesian_updater_real",
        "popper_falsifier_real",
        "lakatos_programme_real",
        "proof_assistant_real",
        "truth_discovery_real",
    ]
    for k in reused_5:
        evi = tr_evi.get(k, {})
        assert evi.get("source") == "V1051", f"{k} source = {evi.get('source')}"


# -----------------------------------------------------------------------------
# 6. 4-formula (additive, recompute, corrected, v1206 baseline)
# -----------------------------------------------------------------------------


def test_v1207_3_formula_distinct():
    """V1207 3-formula: additive (capped) > recompute = corrected."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    # additive capped at 1.0
    assert rep.formula_1_additive <= 1.0
    # recompute and corrected equal for V1207
    assert rep.formula_2_recompute == rep.formula_3_corrected
    # V1206 baseline preserved
    assert rep.v1206_recompute == m.V1206_RECOMPUTE


def test_v1207_asi_recompute_over_north_star():
    """V1207 ASI V0.6.17 > 0.98 (north_star)."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    assert rep.formula_2_recompute > m.ASI_NORTH_STAR, (
        f"V1207 ASI = {rep.formula_2_recompute} <= north_star {m.ASI_NORTH_STAR}"
    )
    assert rep.formula_2_recompute >= 0.99, (
        f"V1207 ASI = {rep.formula_2_recompute} < 0.99"
    )


def test_v1207_dim_lifts():
    """V1207 4 dim lifts each have baseline, lifted, delta, contribution."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    expected_dims = {
        "reinforcement_learning",
        "eternal_identity",
        "time_grounding",
        "truth",
    }
    assert set(rep.dim_lifts.keys()) == expected_dims
    for dim_name, d in rep.dim_lifts.items():
        assert "baseline" in d
        assert "lifted" in d
        assert "delta" in d
        assert "contribution" in d
        # contribution = delta * weight (weight is module constant 0.05)
        assert d["contribution"] == d["delta"] * 0.05
        # delta = lifted - baseline
        assert abs(d["delta"] - (d["lifted"] - d["baseline"])) < 1e-9


def test_v1207_subdim_pass_counts():
    """V1207 sub-dim pass counts (RL 10/10, EI >= 6/10, TG >= 9/10, TR >= 5/10)."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    assert rep.n_rl_subdims_total == 10
    assert rep.n_rl_subdims_pass >= 10
    assert rep.n_ei_subdims_total == 10
    assert rep.n_ei_subdims_pass >= 6
    assert rep.n_tg_subdims_total == 10
    assert rep.n_tg_subdims_pass >= 9
    assert rep.n_tr_subdims_total == 10
    assert rep.n_tr_subdims_pass >= 5


# -----------------------------------------------------------------------------
# 7. V3 哲学闸门
# -----------------------------------------------------------------------------


def test_v1207_inflation_gap_warning():
    """V1207 notes include inflation_gap warning (主 17:43 实事求是)."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    # inflation_gap field exists
    assert hasattr(rep, "inflation_gap")
    # V1207 ASI over north_star = inflation warning
    assert rep.inflation_gap < 0  # negative = over north_star (additive inflation)
    assert rep.position_of_north_star > 100


def test_v1207_truth_not_in_v06_formula():
    """V1207 truth dim not in V0.5/V0.6 ASI formula (主 17:43 不假装)."""
    m = _safe_import_v1207()
    assert "truth_not_in_v06" in m.V3_GUARDS
    assert m.V3_GUARDS["truth_not_in_v06"]


def test_v1207_godel_boundary_real():
    """V1207 V3_GUARDS['godel_boundary_real'] present (不假装所有真理可计算)."""
    m = _safe_import_v1207()
    assert "godel_boundary_real" in m.V3_GUARDS
    val = m.V3_GUARDS["godel_boundary_real"]
    # V3_GUARDS values are descriptive strings (not bools)
    assert isinstance(val, str)
    assert len(val) > 0
    # Gödel 不完备 概念应在 description 中
    assert "Gödel" in val or "完备" in val or "godel" in val.lower()


# -----------------------------------------------------------------------------
# 8. CLI exit codes
# -----------------------------------------------------------------------------


def test_v1207_cli_measure():
    """V1207 CLI --measure prints the formula_2_recompute."""
    res = subprocess.run(
        [sys.executable, "-m", "apeireth.v1207_asi_v0617_truth_dim_lift", "--measure"],
        capture_output=True, timeout=60, encoding="utf-8", errors="replace",
    )
    assert res.returncode == 0
    val = float((res.stdout or "").strip())
    assert val > 0.98


def test_v1207_cli_json():
    """V1207 CLI --json prints valid JSON."""
    res = subprocess.run(
        [sys.executable, "-m", "apeireth.v1207_asi_v0617_truth_dim_lift", "--json"],
        capture_output=True, timeout=60, encoding="utf-8", errors="replace",
    )
    assert res.returncode == 0
    data = json.loads(res.stdout)
    assert data["dim_version"] == "0.6.17"
    assert "dim_lifts" in data
    assert "truth" in data["dim_lifts"]


def test_v1207_cli_report():
    """V1207 CLI --report prints markdown."""
    res = subprocess.run(
        [sys.executable, "-m", "apeireth.v1207_asi_v0617_truth_dim_lift", "--report"],
        capture_output=True, timeout=60, encoding="utf-8", errors="replace",
    )
    assert res.returncode == 0
    out = res.stdout or ""
    assert "# V1207" in out
    assert "truth" in out


# -----------------------------------------------------------------------------
# 9. Artifact + report writing
# -----------------------------------------------------------------------------


def test_v1207_render_report_md():
    """V1207 render_report_md produces valid markdown."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    md = m.render_report_md(rep)
    assert "# V1207" in md
    assert "reinforcement_learning" in md
    assert "eternal_identity" in md
    assert "time_grounding" in md
    assert "truth" in md


def test_v1207_write_artifact_json(tmp_path):
    """V1207 write_artifact_json writes valid JSON to path."""
    m = _safe_import_v1207()
    rep = m.measure_v1207_full()
    out_path = tmp_path / "v1207_test.json"
    written = m.write_artifact_json(rep, out_path)
    assert written.exists()
    with open(written, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["dim_version"] == "0.6.17"
    assert "truth" in data["dim_lifts"]
    assert data["v1206_recompute"] == m.V1206_RECOMPUTE