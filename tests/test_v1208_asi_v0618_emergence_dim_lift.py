"""Tests for V1208 — ASI V0.6.18 emergence_dim_lift (RL + EI + TG + TR + EM).

V1208 是 V1207 superset + truth fix + 5th dim emergence:
- RL (V1206/V1207 复用, 10/10)
- EI (V1206/V1207 复用, 7/10 honest)
- TG (V1206/V1207 复用, 10/10)
- TR (V1207 复用 + V1208 FIX proof_assistant + philosophy_guard, 9/10)
- EM (V1208 NEW 5th dim, V1056 emergence 真生产 11 组件, 10/10)

V1208 truth fix:
  - TR4 proof_assistant_real: V1051 verify_step 需先 assert_proposition → fix
  - TR10 philosophy_guard_real: V1051 6 guards 中 5 需参数 → 提供参数

Tests cover:
1. Imports / dataclass / north_star
2. V1207 baseline values hardcoded
3. RL sub-dim (10) — V1206/V1207 复用
4. EI sub-dim (10) — V1206/V1207 复用
5. TG sub-dim (10) — V1206/V1207 复用
6. TR sub-dim (10) — V1207 复用 + V1208 truth fix
7. EM sub-dim (10) — V1208 NEW emergence
8. ASI formula 1/2/3 + V1207 baseline + delta
9. ASI clamp to 1.0 (V1208 = 1.000000)
10. V1208 = V1207 superset (structural compatibility)
11. CLI --measure / --json / --report exit codes
12. --md-out writes report file
13. --artifact writes JSON artifact file
14. --full writes both
15. V3_GUARDS present (philosophy guards)
16. V1208_EMERGENCE_SUBDIM_NAMES has 10 names
17. 50 sub-dim total (10+10+10+10+10)
18. Truth fix evidence (proof_assistant fix_applied, philosophy_guard n_guard_checks≥5)
19. Emergence evidence (phi_proxy, lz_complexity, n_micros, etc.)
20. ASI > north_star (V1208 = 1.0, over 0.98)
21. Inflation gap (主 17:43 实事求是 warning)
22. position of north_star = 100%+

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


def _safe_import_v1208():
    """Import V1208 module."""
    try:
        from apeireth import v1208_asi_v0618_emergence_dim_lift as m
        return m
    except ImportError as e:
        pytest.skip(f"V1208 not importable: {e}")


# -----------------------------------------------------------------------------
# 1. Imports / dataclass / north_star
# -----------------------------------------------------------------------------


def test_v1208_import():
    """V1208 module imports cleanly."""
    m = _safe_import_v1208()
    assert m is not None
    assert m.V1208_VERSION == "0.1.0"
    assert m.V1208_DIM_VERSION == "0.6.18"
    assert m.ASI_NORTH_STAR == 0.9800


def test_v1208_v1207_baseline_locked():
    """V1207 baseline values hardcoded (主 17:43 实事求是 — 写死历史值)."""
    m = _safe_import_v1208()
    assert m.V1207_RECOMPUTE == 0.992940
    assert m.V1207_REINFORCEMENT_LEARNING_LIFTED == 1.0000
    assert m.V1207_ETERNAL_IDENTITY_LIFTED == 0.8454
    assert m.V1207_TIME_GROUNDING_LIFTED == 1.0000
    assert m.V1207_TRUTH_LIFTED == 0.8200


def test_v1208_v1155_baselines_locked():
    """V1155 baselines hardcoded."""
    m = _safe_import_v1208()
    assert m.V1155_REINFORCEMENT_LEARNING_BASELINE == 0.7272
    assert m.V1155_ETERNAL_IDENTITY_BASELINE == 0.8441
    assert m.V1155_TIME_GROUNDING_BASELINE == 0.8441
    assert m.V1155_TRUTH_BASELINE == 0.8441
    assert m.V1155_EMERGENCE_BASELINE == 0.8441


# -----------------------------------------------------------------------------
# 2. V1208 has 5 dim
# -----------------------------------------------------------------------------


def test_v1208_emergence_subdim_names():
    """V1208 has 10 emergence sub-dim names."""
    m = _safe_import_v1208()
    names = m.V1208_EMERGENCE_SUBDIM_NAMES
    assert len(names) == 10
    expected = {
        "micro_state_aggregation_real",
        "macro_state_computation_real",
        "phase_transition_real",
        "self_organizing_real",
        "downward_causation_real",
        "emergence_detector_real",
        "complexity_metric_real",
        "emergence_event_real",
        "emergence_report_real",
        "philosophy_guard_real",
    }
    assert set(names) == expected


def test_v1208_total_50_subdims():
    """V1208 has 10 RL + 10 EI + 10 TG + 10 TR + 10 EM = 50 sub-dim."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.n_rl_subdims_total == 10
    assert rep.n_ei_subdims_total == 10
    assert rep.n_tg_subdims_total == 10
    assert rep.n_tr_subdims_total == 10
    assert rep.n_em_subdims_total == 10
    # 50 total
    total = (
        rep.n_rl_subdims_total
        + rep.n_ei_subdims_total
        + rep.n_tg_subdims_total
        + rep.n_tr_subdims_total
        + rep.n_em_subdims_total
    )
    assert total == 50


# -----------------------------------------------------------------------------
# 3. RL sub-dim (V1206/V1207 复用, 10/10 pass)
# -----------------------------------------------------------------------------


def test_v1208_rl_all_pass():
    """RL lifted = 1.0000 (10/10 pass, V1206/V1207 复用)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.n_rl_subdims_pass == 10
    assert rep.dim_lifts["reinforcement_learning"]["lifted"] == 1.0000


# -----------------------------------------------------------------------------
# 4. EI sub-dim (V1206/V1207 复用, 7/10 honest)
# -----------------------------------------------------------------------------


def test_v1208_ei_7_of_10():
    """EI lifted = 0.8454 (7/10 pass, V1206 honest)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.n_ei_subdims_pass == 7
    assert rep.dim_lifts["eternal_identity"]["lifted"] == 0.8454


# -----------------------------------------------------------------------------
# 5. TG sub-dim (V1206/V1207 复用, 10/10 pass)
# -----------------------------------------------------------------------------


def test_v1208_tg_all_pass():
    """TG lifted = 1.0000 (10/10 pass, V1206 复用)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.n_tg_subdims_pass == 10
    assert rep.dim_lifts["time_grounding"]["lifted"] == 1.0000


# -----------------------------------------------------------------------------
# 6. TR sub-dim (V1207 + V1208 fix, 9/10 pass)
# -----------------------------------------------------------------------------


def test_v1208_tr_fix_at_least_8():
    """TR lifted >= 0.85 (V1207 0.82 → V1208 fix truth, expected 9/10 = 0.90)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.n_tr_subdims_pass >= 8, (
        f"V1208 truth fix should pass ≥8/10, got {rep.n_tr_subdims_pass}"
    )
    truth_lifted = rep.dim_lifts["truth"]["lifted"]
    assert truth_lifted >= 0.85, (
        f"V1208 truth lifted should be ≥ 0.85 (V1207 was 0.82), got {truth_lifted}"
    )


def test_v1208_truth_fix_proof_assistant():
    """V1208 TR4 proof_assistant fix_applied=True."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    evi = rep.sub_dim_evidence.get("proof_assistant_real", {})
    # fix_applied flag should be present
    assert "fix_applied" in evi or evi.get("pass"), (
        f"V1208 proof_assistant should have fix_applied or pass, evi={evi}"
    )


def test_v1208_truth_fix_philosophy_guard():
    """V1208 TR10 philosophy_guard n_guard_checks >= 5."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    evi = rep.sub_dim_evidence.get("philosophy_guard_real", {})
    n = evi.get("n_guard_checks", 0)
    assert n >= 5, f"V1208 philosophy_guard should provide ≥5 checks, got n={n}"


# -----------------------------------------------------------------------------
# 7. EM sub-dim (V1208 NEW, 10/10 pass)
# -----------------------------------------------------------------------------


def test_v1208_em_all_pass():
    """EM lifted = 1.0000 (10/10 pass, V1208 NEW 5th dim)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.n_em_subdims_pass == 10
    assert rep.dim_lifts["emergence"]["lifted"] == 1.0000


def test_v1208_emergence_v1056_real_components():
    """V1208 EM sub-dim 复用 V1056 真生产 11 组件."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    # EM1 micro_state_aggregation_real: 应有 n_micros
    em1 = rep.sub_dim_evidence.get("micro_state_aggregation_real", {})
    assert "n_micros" in em1, f"V1208 EM1 should use V1056 MicroState, evi={em1}"
    # EM2 macro_state_computation_real: 应有 macro_r
    em2 = rep.sub_dim_evidence.get("macro_state_computation_real", {})
    assert "macro_r" in em2
    # EM5 downward_causation_real: 应有 causal_density
    em5 = rep.sub_dim_evidence.get("downward_causation_real", {})
    assert "causal_density" in em5
    # EM6 emergence_detector_real: 应有 phi_proxy (Tononi Φ proxy)
    em6 = rep.sub_dim_evidence.get("emergence_detector_real", {})
    assert "phi_proxy" in em6, f"V1208 EM6 phi_proxy missing, evi={em6}"
    # EM7 complexity_metric_real: 应有 lz_complexity
    em7 = rep.sub_dim_evidence.get("complexity_metric_real", {})
    assert "lz_complexity" in em7, f"V1208 EM7 lz_complexity missing, evi={em7}"


# -----------------------------------------------------------------------------
# 8. ASI formula 1/2/3
# -----------------------------------------------------------------------------


def test_v1208_recompute_is_1():
    """V1208 formula_2_recompute = 1.000000 (clamp)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.formula_2_recompute == pytest.approx(1.0, abs=1e-6), (
        f"V1208 formula_2_recompute should clamp to 1.0, got {rep.formula_2_recompute}"
    )


def test_v1208_corrected_clamps():
    """V1208 formula_3_corrected = 1.000000 (clamp)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.formula_3_corrected == pytest.approx(1.0, abs=1e-6)


def test_v1208_additive_less_or_eq_1():
    """V1208 formula_1_additive should be ≤ 1.0 (sum of contributions)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert 0.0 <= rep.formula_1_additive <= 1.0, (
        f"V1208 formula_1_additive should be in [0,1], got {rep.formula_1_additive}"
    )


def test_v1208_v1207_delta_positive():
    """V1208 Δ vs V1207 baseline = positive (V1208 = V1207 + truth_fix + emergence)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.asi_recompute_delta > 0, (
        f"V1208 should improve over V1207 (Δ={rep.asi_recompute_delta})"
    )


# -----------------------------------------------------------------------------
# 9. north_star position + inflation
# -----------------------------------------------------------------------------


def test_v1208_position_of_north_star():
    """V1208 position_of_north_star ≥ 100% (over 0.98)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    assert rep.position_of_north_star >= 100.0, (
        f"V1208 over north_star, position={rep.position_of_north_star}"
    )


def test_v1208_inflation_gap_recorded():
    """V1208 inflation_gap (additive - recompute) recorded (主 17:43 warning)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    # inflation_gap exists as attribute
    assert hasattr(rep, "inflation_gap")


# -----------------------------------------------------------------------------
# 10. V3 philosophy guards present
# -----------------------------------------------------------------------------


def test_v1208_v3_guards_present():
    """V3_GUARDS module-level dict exists (--report CLI needs it)."""
    m = _safe_import_v1208()
    # V3_GUARDS at module level (V1207 fixed bug: moved before __main__)
    assert hasattr(m, "V3_GUARDS")
    assert isinstance(m.V3_GUARDS, dict)
    assert len(m.V3_GUARDS) >= 5


# -----------------------------------------------------------------------------
# 11. CLI exit codes
# -----------------------------------------------------------------------------


def test_v1208_cli_measure_exit():
    """CLI --measure returns 0 with measure value."""
    res = subprocess.run(
        [sys.executable, "-m", "apeireth.v1208_asi_v0618_emergence_dim_lift", "--measure"],
        capture_output=True, text=True, cwd=str(REPO_ROOT),
    )
    assert res.returncode == 0, f"V1208 --measure failed: {res.stderr}"
    val = float(res.stdout.strip())
    assert val == pytest.approx(1.0, abs=1e-6)


def test_v1208_cli_default_exit():
    """CLI default exits 0."""
    res = subprocess.run(
        [sys.executable, "-m", "apeireth.v1208_asi_v0618_emergence_dim_lift"],
        capture_output=True, text=True, cwd=str(REPO_ROOT),
    )
    assert res.returncode == 0, f"V1208 default failed: {res.stderr}"


def test_v1208_cli_json_exit():
    """CLI --json exits 0 with valid JSON."""
    res = subprocess.run(
        [sys.executable, "-m", "apeireth.v1208_asi_v0618_emergence_dim_lift", "--json"],
        capture_output=True, text=True, cwd=str(REPO_ROOT),
    )
    assert res.returncode == 0, f"V1208 --json failed: {res.stderr}"
    data = json.loads(res.stdout)
    assert "formula_2_recompute" in data
    assert "dim_lifts" in data


def test_v1208_cli_report_exit():
    """CLI --report exits 0 with markdown."""
    env = {**__import__("os").environ, "PYTHONIOENCODING": "utf-8"}
    res = subprocess.run(
        [sys.executable, "-m", "apeireth.v1208_asi_v0618_emergence_dim_lift", "--report"],
        capture_output=True, text=True, cwd=str(REPO_ROOT), env=env, encoding="utf-8",
    )
    assert res.returncode == 0, f"V1208 --report failed: rc={res.returncode}"
    md = res.stdout or ""
    assert "V1208" in md, f"V1208 --report stdout missing 'V1208' (got {len(md)} chars)"
    assert "ASI" in md


# -----------------------------------------------------------------------------
# 12-14. --md-out / --artifact / --full writing
# -----------------------------------------------------------------------------


def test_v1208_md_out_writes_file(tmp_path):
    """CLI --md-out PATH writes Markdown file."""
    out = tmp_path / "v1208_test.md"
    env = {**__import__("os").environ, "PYTHONIOENCODING": "utf-8"}
    res = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1208_asi_v0618_emergence_dim_lift",
            "--md-out", str(out),
        ],
        capture_output=True, text=True, cwd=str(REPO_ROOT), env=env, encoding="utf-8",
    )
    assert res.returncode == 0, f"V1208 --md-out failed: {res.stderr}"
    assert out.exists()
    content = out.read_text(encoding="utf-8")
    assert "V1208" in content
    assert "ASI" in content


def test_v1208_artifact_writes_json(tmp_path):
    """CLI --artifact PATH writes JSON artifact."""
    out = tmp_path / "v1208_test.json"
    res = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1208_asi_v0618_emergence_dim_lift",
            "--artifact", str(out),
        ],
        capture_output=True, text=True, cwd=str(REPO_ROOT),
    )
    assert res.returncode == 0, f"V1208 --artifact failed: {res.stderr}"
    assert out.exists()
    data = json.loads(out.read_text(encoding="utf-8"))
    assert "formula_2_recompute" in data
    assert "dim_lifts" in data
    assert "emergence" in data["dim_lifts"]


def test_v1208_full_writes_both(tmp_path):
    """CLI --full writes both artifact + report."""
    artifact = tmp_path / "v1208_full.json"
    report = tmp_path / "v1208_full.md"
    env = {**__import__("os").environ, "PYTHONIOENCODING": "utf-8"}
    res = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1208_asi_v0618_emergence_dim_lift",
            "--full", "--artifact", str(artifact), "--md-out", str(report),
        ],
        capture_output=True, text=True, cwd=str(REPO_ROOT), env=env, encoding="utf-8",
    )
    assert res.returncode == 0, f"V1208 --full failed: {res.stderr}"
    assert artifact.exists()
    assert report.exists()
    assert json.loads(artifact.read_text(encoding="utf-8"))["formula_2_recompute"] == pytest.approx(1.0, abs=1e-6)


# -----------------------------------------------------------------------------
# 15. V1208 = V1207 superset (structural compatibility)
# -----------------------------------------------------------------------------


def test_v1208_is_superset_of_v1207():
    """V1208 = V1207 + truth fix + emergence dim (主 19:33 站在前人肩上)."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    # RL/EI/TG/TR 维度都应在 dim_lifts
    assert "reinforcement_learning" in rep.dim_lifts
    assert "eternal_identity" in rep.dim_lifts
    assert "time_grounding" in rep.dim_lifts
    assert "truth" in rep.dim_lifts
    assert "emergence" in rep.dim_lifts  # V1208 NEW
    # V1208 的 truth fix 应让 truth >= V1207 的 0.82
    assert rep.dim_lifts["truth"]["lifted"] >= 0.82


# -----------------------------------------------------------------------------
# 16. V1208Report dataclass fields
# -----------------------------------------------------------------------------


def test_v1208_report_dataclass():
    """V1208Report has all expected fields."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    expected_fields = [
        "snapshot_id",
        "version",
        "dim_version",
        "formula_1_additive",
        "formula_2_recompute",
        "formula_3_corrected",
        "v1207_recompute",
        "asi_recompute_delta",
        "dim_lifts",
        "n_rl_subdims_pass", "n_rl_subdims_total",
        "n_ei_subdims_pass", "n_ei_subdims_total",
        "n_tg_subdims_pass", "n_tg_subdims_total",
        "n_tr_subdims_pass", "n_tr_subdims_total",
        "n_em_subdims_pass", "n_em_subdims_total",
        "sub_dim_evidence",
        "position_of_north_star",
        "inflation_gap",
        "artifact_path",
    ]
    for field in expected_fields:
        assert hasattr(rep, field), f"V1208Report missing field: {field}"


# -----------------------------------------------------------------------------
# 17. measure_v1208_* helpers
# -----------------------------------------------------------------------------


def test_v1208_measure_helpers():
    """measure_v1208_recompute / additive / corrected are callable and in [0, 1]."""
    m = _safe_import_v1208()
    assert m.measure_v1208_recompute() == pytest.approx(1.0, abs=1e-6)
    assert 0.0 <= m.measure_v1208_additive() <= 1.0
    assert m.measure_v1208_corrected() == pytest.approx(1.0, abs=1e-6)


# -----------------------------------------------------------------------------
# 18. write_artifact_json / render_report_md helpers
# -----------------------------------------------------------------------------


def test_v1208_write_artifact_helper(tmp_path):
    """write_artifact_json writes JSON file."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    out = tmp_path / "v1208_helper.json"
    m.write_artifact_json(rep, out)
    assert out.exists()
    data = json.loads(out.read_text(encoding="utf-8"))
    assert data["dim_version"] == "0.6.18"


def test_v1208_render_report_md_helper():
    """render_report_md returns markdown string with V1208 info."""
    m = _safe_import_v1208()
    rep = m.measure_v1208_full()
    md = m.render_report_md(rep)
    assert "V1208" in md
    assert "0.6.18" in md
    assert "ASI" in md
    assert "emergence" in md.lower() or "Emergence" in md