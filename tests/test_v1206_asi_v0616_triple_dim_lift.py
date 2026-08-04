"""Tests for V1206 — ASI V0.6.16 triple_dim_lift (RL + EI fixed + TG).

Tests cover:
1. Imports / dataclass
2. RL sub-dim (10) — V1205 复用, 10/10 pass
3. EI sub-dim (10) — V1206 fix 4 bugs (am_depth, psm_clarity, continuity, stats)
4. TG sub-dim (10) — V1206 NEW (5 V1154 reused + 5 NEW)
5. 3-formula (additive, recompute, corrected)
6. ASI > north_star (V1206 over 0.98)
7. V3_GUARDS present
8. CLI exit codes

主哲学: 主 17:43 实事求是 + 主 17:58 不假装 + 主 13:31 大胆激进
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


def _safe_import_v1206():
    """Import V1206 module."""
    try:
        from apeireth import v1206_asi_v0616_triple_dim_lift as m
        return m
    except ImportError as e:
        pytest.skip(f"V1206 not importable: {e}")


def test_v1206_import():
    """V1206 module imports cleanly."""
    m = _safe_import_v1206()
    assert m is not None
    assert m.V1206_VERSION == "0.1.0"
    assert m.V1206_DIM_VERSION == "0.6.16"
    assert m.ASI_NORTH_STAR == 0.9800


def test_v1206_rl_subdims():
    """V1206 RL 10 sub-dim (V1205 复用, 10/10 pass)."""
    m = _safe_import_v1206()
    rl_total, rl_subs, rl_evi = m._measure_rl_v1206()
    assert len(rl_subs) == 10
    # All 10 should be >= 0.5 (V1205 was 1.0 across the board)
    for k, v in rl_subs.items():
        assert v >= 0.5, f"RL {k} = {v} < 0.5"
    assert rl_total >= 0.95, f"RL total = {rl_total} < 0.95"


def test_v1206_ei_subdim_am_depth_real_fixed():
    """V1206 EI3 am_depth_real — V1205 fixed (when= arg added)."""
    m = _safe_import_v1206()
    ei_total, ei_subs, ei_evi = m._measure_ei_v1206()
    am_depth = ei_subs.get("am_depth_real", 0.0)
    assert am_depth > 0.0, f"am_depth_real = {am_depth} (V1206 should be > 0 after fix)"
    evi = ei_evi.get("am_depth_real", {})
    assert "v1205_bug" in evi
    assert "v1206_fix" in evi


def test_v1206_ei_subdim_psm_clarity_real_fixed():
    """V1206 EI4 psm_clarity_real — V1205 fixed (use psm.clarity())."""
    m = _safe_import_v1206()
    ei_total, ei_subs, ei_evi = m._measure_ei_v1206()
    psm_clarity = ei_subs.get("psm_clarity_real", 0.0)
    assert psm_clarity > 0.0, f"psm_clarity_real = {psm_clarity}"
    evi = ei_evi.get("psm_clarity_real", {})
    assert "v1205_bug" in evi
    assert "v1206_fix" in evi


def test_v1206_ei_subdim_continuity_score_real_fixed():
    """V1206 EI6 continuity_score_real — V1206 fixed (start_session + n_entries_added)."""
    m = _safe_import_v1206()
    ei_total, ei_subs, ei_evi = m._measure_ei_v1206()
    cont = ei_subs.get("continuity_score_real", 0.0)
    assert cont > 0.0, f"continuity_score_real = {cont}"
    evi = ei_evi.get("continuity_score_real", {})
    assert evi.get("n_sessions", 0) >= 2, f"n_sessions = {evi.get('n_sessions')}"
    assert evi.get("n_entries_added", 0) > 0, f"n_entries_added = {evi.get('n_entries_added')}"


def test_v1206_ei_subdim_stats_real_fixed():
    """V1206 EI10 stats_real — V1206 fixed (use PSM.stats() instead of EternalIdentityCore)."""
    m = _safe_import_v1206()
    ei_total, ei_subs, ei_evi = m._measure_ei_v1206()
    stats_score = ei_subs.get("stats_real", 0.0)
    assert stats_score >= 0.99, f"stats_real = {stats_score}"
    evi = ei_evi.get("stats_real", {})
    assert evi.get("n_keys", 0) >= 4, f"n_keys = {evi.get('n_keys')}"


def test_v1206_ei_total_lifted():
    """V1206 EI total >= 0.8 (V1205 was 0.5844, V1206 should be much higher)."""
    m = _safe_import_v1206()
    ei_total, ei_subs, ei_evi = m._measure_ei_v1206()
    assert ei_total >= 0.8, f"EI total = {ei_total} (V1205 was 0.5844)"
    # V1205 was 0.5844 (regression due to 4 bugs). V1206 should be >= 0.85
    assert ei_total > 0.5844, f"EI total = {ei_total} did not improve from V1205"


def test_v1206_tg_subdim_full_pass():
    """V1206 TG 10 sub-dim (V1154 5 reused + V1206 5 NEW)."""
    m = _safe_import_v1206()
    tg_total, tg_subs, tg_evi = m._measure_tg_v1206()
    assert len(tg_subs) == 10
    # All 10 should be >= 0.9
    for k, v in tg_subs.items():
        assert v >= 0.9, f"TG {k} = {v} < 0.9"


def test_v1206_tg_tz_aware():
    """V1206 TG8 t3_v1206_tz_aware — Asia/Shanghai."""
    m = _safe_import_v1206()
    tg_total, tg_subs, tg_evi = m._measure_tg_v1206()
    tz = tg_subs.get("t3_v1206_tz_aware", 0.0)
    assert tz >= 0.99, f"t3_v1206_tz_aware = {tz}"


def test_v1206_tg_throughput():
    """V1206 TG6 t1_v1206_throughput."""
    m = _safe_import_v1206()
    tg_total, tg_subs, tg_evi = m._measure_tg_v1206()
    tp = tg_subs.get("t1_v1206_throughput", 0.0)
    assert tp >= 0.99, f"t1_v1206_throughput = {tp}"


def test_v1206_asi_recompute_over_north_star():
    """V1206 ASI V0.6.16 > 0.98 (north_star)."""
    m = _safe_import_v1206()
    rep = m.measure_v1206_full(artifact_dir="")
    assert rep.formula_2_recompute > m.ASI_NORTH_STAR, (
        f"V1206 ASI = {rep.formula_2_recompute} <= north_star {m.ASI_NORTH_STAR}"
    )
    assert rep.formula_2_recompute >= 0.99, (
        f"V1206 ASI = {rep.formula_2_recompute} < 0.99 (expected >= 0.99)"
    )


def test_v1206_3_formula_distinct():
    """V1206 3-formula: additive (cap 1.0) > recompute = corrected."""
    m = _safe_import_v1206()
    rep = m.measure_v1206_full(artifact_dir="")
    # additive capped at 1.0
    assert rep.formula_1_additive == 1.0
    # recompute and corrected equal for V1206
    assert rep.formula_2_recompute == rep.formula_3_corrected
    # delta positive (V1206 > V1205)
    assert rep.asi_recompute_delta > 0


def test_v1206_dim_lifts():
    """V1206 3 dim lifts each have weight, baseline, lifted, delta, contribution."""
    m = _safe_import_v1206()
    rep = m.measure_v1206_full(artifact_dir="")
    assert "reinforcement_learning" in rep.dim_lifts
    assert "eternal_identity" in rep.dim_lifts
    assert "time_grounding" in rep.dim_lifts
    for dim_name, d in rep.dim_lifts.items():
        assert "weight" in d
        assert "baseline" in d
        assert "lifted" in d
        assert "delta" in d
        assert "contribution" in d
        assert d["weight"] == 0.05
        assert d["contribution"] == d["delta"] * 0.05


def test_v1206_subdim_pass_counts():
    """V1206 sub-dim pass counts (RL 10/10, EI >= 6/10, TG >= 9/10)."""
    m = _safe_import_v1206()
    rep = m.measure_v1206_full(artifact_dir="")
    assert rep.n_rl_subdims_total == 10
    assert rep.n_rl_subdims_pass >= 10
    assert rep.n_ei_subdims_total == 10
    assert rep.n_ei_subdims_pass >= 6  # V1205 was 5/10 (one bug was 0); V1206 should be 6+ after fix
    assert rep.n_tg_subdims_total == 10
    assert rep.n_tg_subdims_pass >= 9


def test_v1206_artifact_written():
    """V1206 measure_v1206_full writes artifact."""
    m = _safe_import_v1206()
    rep = m.measure_v1206_full(artifact_dir="artifacts")
    assert rep.artifact_path != ""
    p = Path(rep.artifact_path)
    assert p.exists()
    # Verify JSON parseable
    with open(p, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["dim_version"] == "0.6.16"
    assert "dim_lifts" in data
    assert "reinforcement_learning" in data["dim_lifts"]


def test_v1206_render_report_md():
    """V1206 render_report_md produces valid markdown."""
    m = _safe_import_v1206()
    rep = m.measure_v1206_full(artifact_dir="")
    md = m.render_report_md(rep)
    assert "# V1206" in md
    assert "reinforcement_learning" in md
    assert "eternal_identity" in md
    assert "time_grounding" in md
    assert "V1205 bugs fixed in V1206" in md


def test_v1206_v3_guards_present():
    """V1206 has V3_GUARDS dictionary."""
    m = _safe_import_v1206()
    assert hasattr(m, "V3_GUARDS")
    assert isinstance(m.V3_GUARDS, dict)
    assert "module_is_not_asi" in m.V3_GUARDS
    assert "v1206_is_v06_16" in m.V3_GUARDS
    assert "time_grounding_not_in_v06" in m.V3_GUARDS


def test_v1206_cli_measure():
    """V1206 CLI --measure prints the formula_2_recompute."""
    res = subprocess.run(
        [sys.executable, "-m", "apeireth.v1206_asi_v0616_triple_dim_lift", "--measure"],
        capture_output=True, timeout=60, encoding="utf-8", errors="replace",
    )
    assert res.returncode == 0
    val = float((res.stdout or "").strip())
    assert val > 0.98


def test_v1206_cli_report():
    """V1206 CLI --report prints markdown."""
    res = subprocess.run(
        [sys.executable, "-m", "apeireth.v1206_asi_v0616_triple_dim_lift", "--report"],
        capture_output=True, timeout=60, encoding="utf-8", errors="replace",
    )
    assert res.returncode == 0
    out = res.stdout or ""
    assert "# V1206" in out
    assert "reinforcement_learning" in out


def test_v1206_baseline_values_locked():
    """V1206 baseline values hardcoded (主 17:43 实事求是)."""
    m = _safe_import_v1206()
    assert m.V1205_RECOMPUTE == 0.972645
    assert m.V1155_REINFORCEMENT_LEARNING_BASELINE == 0.7272
    assert m.V1155_ETERNAL_IDENTITY_BASELINE == 0.8441


def test_v1206_ei_subdim_v1072_components():
    """V1206 EI 5 reused sub-dim all source from V1072."""
    m = _safe_import_v1206()
    ei_total, ei_subs, ei_evi = m._measure_ei_v1206()
    for k in ["ltm_persistence_real", "self_reference_real", "am_depth_real", "psm_clarity_real", "v02_bridge_real"]:
        evi = ei_evi.get(k, {})
        assert evi.get("source") == "V1072", f"{k} source = {evi.get('source')}"


def test_v1206_rl_subdim_v1169_components():
    """V1206 RL 5 reused sub-dim all source from V1169."""
    m = _safe_import_v1206()
    rl_total, rl_subs, rl_evi = m._measure_rl_v1206()
    for k in ["agents_real", "references_real", "v3_guards_real", "metrics_real", "v02_bridge_real"]:
        evi = rl_evi.get(k, {})
        assert evi.get("source") == "V1169", f"{k} source = {evi.get('source')}"


def test_v1206_tg_subdim_v1154_components():
    """V1206 TG 5 reused sub-dim all source from V1154."""
    m = _safe_import_v1206()
    tg_total, tg_subs, tg_evi = m._measure_tg_v1206()
    for k in ["wall_clock_grounding", "monotonic_elapsed", "interval_reasoning", "causal_order_awareness", "duration_self_perception"]:
        evi = tg_evi.get(k, {})
        assert evi.get("source") == "V1154", f"{k} source = {evi.get('source')}"


def test_v1206_inflation_gap_warning():
    """V1206 notes include inflation_gap warning (主 17:43 实事求是)."""
    m = _safe_import_v1206()
    rep = m.measure_v1206_full(artifact_dir="")
    notes_str = " ".join(rep.notes)
    assert "inflation" in notes_str.lower()
    assert "不假装" in notes_str or "not pretending" in notes_str.lower()


def test_v1206_position_pct():
    """V1206 position of north_star > 100% (over north_star)."""
    m = _safe_import_v1206()
    rep = m.measure_v1206_full(artifact_dir="")
    position_pct = (rep.formula_2_recompute / m.ASI_NORTH_STAR) * 100
    assert position_pct > 100, f"position = {position_pct}%"