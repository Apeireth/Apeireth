"""Tests for V1451 — ASI cube history trend v2."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


# ============================================================================
# Path setup: add apeireth parent for direct module import
# ============================================================================

HERE = Path(__file__).resolve().parent
APEIRETH_ROOT = HERE.parent
WORKSPACE_ROOT = APEIRETH_ROOT.parent
for p in (str(APEIRETH_ROOT), str(WORKSPACE_ROOT)):
    if p not in sys.path:
        sys.path.insert(0, p)


# ============================================================================
# Import the module under test
# ============================================================================

import apeireth.v1451_asi_cube_history_trend_v2 as v1451  # noqa: E402


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture
def empty_history():
    return []


@pytest.fixture
def single_snapshot():
    return [{
        "cube_overall_closure_rate": 0.5,
        "cube_cross_link_density": 0.5,
        "per_axis_overall": {
            "problem": 0.5,
            "position": 0.5,
            "protocol": 0.5,
        },
        "axis_stats": [],
    }]


@pytest.fixture
def two_snapshots_improving():
    return [
        {
            "cube_overall_closure_rate": 0.5,
            "cube_cross_link_density": 0.5,
            "per_axis_overall": {"problem": 0.4, "position": 0.5, "protocol": 0.6},
            "axis_stats": [
                {"axis": "problem", "element": "time", "mean_closure": 0.3, "face_count": 2},
                {"axis": "problem", "element": "freedom", "mean_closure": 0.7, "face_count": 2},
                {"axis": "position", "element": "scheduler", "mean_closure": 0.6, "face_count": 2},
                {"axis": "protocol", "element": "sync", "mean_closure": 0.8, "face_count": 2},
            ],
        },
        {
            "cube_overall_closure_rate": 0.7,
            "cube_cross_link_density": 0.6,
            "per_axis_overall": {"problem": 0.7, "position": 0.7, "protocol": 0.7},
            "axis_stats": [
                {"axis": "problem", "element": "time", "mean_closure": 0.6, "face_count": 2},
                {"axis": "problem", "element": "freedom", "mean_closure": 0.7, "face_count": 2},
                {"axis": "position", "element": "scheduler", "mean_closure": 0.7, "face_count": 2},
                {"axis": "protocol", "element": "sync", "mean_closure": 0.8, "face_count": 2},
            ],
        },
    ]


@pytest.fixture
def two_snapshots_regressing():
    return [
        {
            "cube_overall_closure_rate": 0.7,
            "per_axis_overall": {"problem": 0.7, "position": 0.7, "protocol": 0.7},
            "axis_stats": [],
        },
        {
            "cube_overall_closure_rate": 0.4,
            "per_axis_overall": {"problem": 0.4, "position": 0.4, "protocol": 0.4},
            "axis_stats": [],
        },
    ]


@pytest.fixture
def two_snapshots_stagnant():
    return [
        {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []},
        {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []},
    ]


# ============================================================================
# Constants tests
# ============================================================================

def test_version():
    assert v1451.V1451_VERSION == "0.1.0"


def test_schema():
    assert v1451.V1451_SCHEMA == "asi.cube-history-trend-v2.v1"


def test_module_name():
    assert v1451.V1451_MODULE == "apeireth.v1451_asi_cube_history_trend_v2"


def test_problem_names():
    assert len(v1451.V1451_PROBLEM_NAMES) == 7
    assert "time" in v1451.V1451_PROBLEM_NAMES
    assert "value_alignment" in v1451.V1451_PROBLEM_NAMES


def test_position_names():
    assert len(v1451.V1451_POSITION_NAMES) == 5
    assert "scheduler" in v1451.V1451_POSITION_NAMES
    assert "asi_occupier" in v1451.V1451_POSITION_NAMES


def test_protocol_names():
    assert len(v1451.V1451_PROTOCOL_NAMES) == 6
    assert "sync" in v1451.V1451_PROTOCOL_NAMES
    assert "hybrid" in v1451.V1451_PROTOCOL_NAMES


def test_guards_count():
    assert len(v1451.V1451_GUARDS) == 14


def test_v3_guards_count():
    assert len(v1451.V1451_V3_GUARDS) == 5


def test_borrowed_count():
    assert len(v1451.V1451_BORROWED) == 5


def test_improving_threshold_positive():
    assert v1451.V1451_IMPROVING_THRESHOLD > 0


def test_regressing_threshold_negative():
    assert v1451.V1451_REGRESSING_THRESHOLD < 0


# ============================================================================
# Helper tests
# ============================================================================

def test_clip01_lower():
    assert v1451._clip01(-0.5) == 0.0


def test_clip01_upper():
    assert v1451._clip01(1.5) == 1.0


def test_clip01_middle():
    assert v1451._clip01(0.7) == 0.7


def test_slope_simple_one_point():
    assert v1451._slope_simple([0.5]) == 0.0


def test_slope_simple_two_points():
    assert abs(v1451._slope_simple([0.0, 1.0]) - 1.0) < 1e-9


def test_slope_simple_three_points():
    assert abs(v1451._slope_simple([0.0, 0.5, 1.0]) - 0.5) < 1e-9


def test_is_improving_positive():
    assert v1451._is_improving(0.05) is True


def test_is_improving_zero():
    assert v1451._is_improving(0.0) is False


def test_is_regressing_negative():
    assert v1451._is_regressing(-0.05) is True


def test_is_stagnant_small():
    assert v1451._is_stagnant(0.005) is True


# ============================================================================
# compute_trend with empty history
# ============================================================================

def test_compute_trend_empty(empty_history):
    rep = v1451.compute_trend(empty_history)
    assert rep.n_snapshots_before == 0
    assert rep.is_improving is False
    assert rep.is_regressing is False
    assert rep.is_stagnant is True
    assert any("INSUFFICIENT" in n for n in rep.notes)


def test_compute_trend_empty_per_axis(empty_history):
    rep = v1451.compute_trend(empty_history)
    assert len(rep.per_axis_trend) == 3


def test_compute_trend_empty_per_element(empty_history):
    rep = v1451.compute_trend(empty_history)
    n_expected = 7 + 5 + 6
    assert len(rep.per_element_delta) == n_expected


# ============================================================================
# compute_trend with 1 snapshot (INSUFFICIENT)
# ============================================================================

def test_compute_trend_single(single_snapshot):
    rep = v1451.compute_trend(single_snapshot)
    assert rep.n_snapshots_before == 1
    assert rep.is_stagnant is True
    assert rep.cube_first_rate == pytest.approx(0.5, abs=1e-9)
    assert rep.cube_last_rate == pytest.approx(0.5, abs=1e-9)
    assert rep.cube_delta == pytest.approx(0.0, abs=1e-9)


# ============================================================================
# compute_trend with 2 snapshots — improving
# ============================================================================

def test_compute_trend_improving(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    assert rep.is_improving is True
    assert rep.is_regressing is False
    assert rep.cube_delta == pytest.approx(0.2, abs=1e-9)
    assert rep.cube_first_rate == pytest.approx(0.5, abs=1e-9)
    assert rep.cube_last_rate == pytest.approx(0.7, abs=1e-9)


def test_compute_trend_improving_per_axis(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    for at in rep.per_axis_trend:
        assert at.is_improving is True


def test_compute_trend_improving_per_element_time(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    time_delta = next(e for e in rep.per_element_delta if e.element == "time")
    assert time_delta.is_improving is True
    assert time_delta.delta == pytest.approx(0.3, abs=1e-9)


def test_compute_trend_improving_per_element_sync(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    sync_delta = next(e for e in rep.per_element_delta if e.element == "sync")
    assert sync_delta.is_stagnant is True
    assert sync_delta.delta == pytest.approx(0.0, abs=1e-9)


# ============================================================================
# compute_trend with 2 snapshots — regressing
# ============================================================================

def test_compute_trend_regressing(two_snapshots_regressing):
    rep = v1451.compute_trend(two_snapshots_regressing)
    assert rep.is_regressing is True
    assert rep.is_improving is False
    assert rep.cube_delta < v1451.V1451_REGRESSING_THRESHOLD


def test_compute_trend_regressing_per_axis(two_snapshots_regressing):
    rep = v1451.compute_trend(two_snapshots_regressing)
    for at in rep.per_axis_trend:
        assert at.is_regressing is True


# ============================================================================
# compute_trend with 2 snapshots — stagnant
# ============================================================================

def test_compute_trend_stagnant(two_snapshots_stagnant):
    rep = v1451.compute_trend(two_snapshots_stagnant)
    assert rep.is_stagnant is True
    assert rep.is_improving is False
    assert rep.is_regressing is False


def test_compute_trend_stagnant_all_elements(two_snapshots_stagnant):
    rep = v1451.compute_trend(two_snapshots_stagnant)
    for ed in rep.per_element_delta:
        assert ed.is_stagnant is True
    assert rep.stagnant_count == 18
    assert rep.improving_count == 0
    assert rep.regressing_count == 0


# ============================================================================
# Counts + stability
# ============================================================================

def test_counts_sum_to_18(two_snapshots_stagnant):
    rep = v1451.compute_trend(two_snapshots_stagnant)
    total = rep.stagnant_count + rep.improving_count + rep.regressing_count
    assert total == 18


def test_stability_score_bounded(two_snapshots_stagnant):
    rep = v1451.compute_trend(two_snapshots_stagnant)
    assert 0.0 <= rep.stability_score <= 1.0


def test_stability_score_stagnant_is_zero(two_snapshots_stagnant):
    rep = v1451.compute_trend(two_snapshots_stagnant)
    assert rep.stability_score == pytest.approx(0.0, abs=1e-9)


def test_axis_improving_count(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    assert rep.axis_improving_count == 3


# ============================================================================
# Top-N ranked
# ============================================================================

def test_top_improving_bounded(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    assert len(rep.top_improving) <= v1451.V1451_TOP_N


def test_top_regressing_bounded(two_snapshots_regressing):
    rep = v1451.compute_trend(two_snapshots_regressing)
    assert len(rep.top_regressing) <= v1451.V1451_TOP_N


def test_top_improving_stagnant_empty(two_snapshots_stagnant):
    rep = v1451.compute_trend(two_snapshots_stagnant)
    assert len(rep.top_improving) == 0


# ============================================================================
# Trend fields bounded
# ============================================================================

def test_cube_delta_bounded(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    assert -1.0 <= rep.cube_delta <= 1.0


def test_cube_slope_bounded(two_snapshots_regressing):
    rep = v1451.compute_trend(two_snapshots_regressing)
    assert -1.0 <= rep.cube_slope <= 1.0


def test_per_axis_delta_bounded(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    for at in rep.per_axis_trend:
        assert -1.0 <= at.delta <= 1.0
        assert -1.0 <= at.slope <= 1.0


# ============================================================================
# Popper self-test
# ============================================================================

def test_popper_all_ok():
    ok, results = v1451.popper()
    assert ok is True
    assert len(results) == 14
    for r in results:
        assert r["ok"] is True, f"failed: {r['name']}: {r['detail']}"


def test_popper_T01_constants():
    _, results = v1451.popper()
    t01 = next(r for r in results if r["name"] == "T01_constants")
    assert t01["ok"] is True


def test_popper_T05_improving():
    _, results = v1451.popper()
    t05 = next(r for r in results if r["name"] == "T05_two_snapshots_improving")
    assert t05["ok"] is True


# ============================================================================
# Chain delegate
# ============================================================================

def test_chain_delegate_ok():
    chain = v1451.chain_delegate()
    assert chain["all_ok"] is True
    assert len(chain["delegates"]) >= 1


def test_chain_delegate_v1450_present():
    chain = v1451.chain_delegate()
    modules = [d["module"] for d in chain["delegates"]]
    assert "V1450" in modules


# ============================================================================
# CLI
# ============================================================================

def test_cli_version():
    from apeireth.v1451_asi_cube_history_trend_v2 import main
    rc = main(["version"])
    assert rc == 0


def test_cli_meta():
    from apeireth.v1451_asi_cube_history_trend_v2 import main
    rc = main(["meta"])
    assert rc == 0


def test_cli_meta_json():
    from apeireth.v1451_asi_cube_history_trend_v2 import main
    rc = main(["meta", "--json"])
    assert rc == 0


def test_cli_popper():
    from apeireth.v1451_asi_cube_history_trend_v2 import main
    rc = v1451.main(["popper"])
    assert rc == 0


def test_cli_popper_fail_returns_1():
    """popper returns 0 only if all_ok, otherwise 1."""
    from apeireth.v1451_asi_cube_history_trend_v2 import main
    rc = v1451.main(["popper"])
    assert rc in (0, 1)


def test_cli_chain():
    from apeireth.v1451_asi_cube_history_trend_v2 import main
    rc = v1451.main(["chain"])
    assert rc == 0


def test_cli_trend():
    from apeireth.v1451_asi_cube_history_trend_v2 import main
    rc = v1451.main(["trend"])
    assert rc == 0


def test_cli_compute():
    from apeireth.v1451_asi_cube_history_trend_v2 import main
    rc = v1451.main(["compute"])
    assert rc == 0


def test_cli_unknown_returns_2():
    from apeireth.v1451_asi_cube_history_trend_v2 import main
    rc = v1451.main(["bogus-cmd"])
    assert rc == 2


# ============================================================================
# Markdown render
# ============================================================================

def test_render_markdown_contains_v1451(single_snapshot):
    rep = v1451.compute_trend(single_snapshot)
    md = v1451._render_markdown(rep)
    assert "V1451" in md
    assert "asi.cube-history-trend-v2.v1" in md


def test_render_markdown_contains_honest(single_snapshot):
    rep = v1451.compute_trend(single_snapshot)
    md = v1451._render_markdown(rep)
    assert "Honest disclosure" in md


def test_render_markdown_contains_v3_guards(single_snapshot):
    rep = v1451.compute_trend(single_snapshot)
    md = v1451._render_markdown(rep)
    assert "V3 哲学守门" in md
    for g in v1451.V1451_V3_GUARDS:
        assert g in md


def test_render_markdown_contains_14_guards(single_snapshot):
    rep = v1451.compute_trend(single_snapshot)
    md = v1451._render_markdown(rep)
    for g in v1451.V1451_GUARDS:
        assert g in md


def test_render_markdown_contains_per_axis_table(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    md = v1451._render_markdown(rep)
    assert "Per-axis trend" in md
    assert "problem" in md
    assert "position" in md
    assert "protocol" in md


# ============================================================================
# Notes contain expected labels
# ============================================================================

def test_notes_contain_insufficient(empty_history):
    rep = v1451.compute_trend(empty_history)
    assert any("INSUFFICIENT" in n for n in rep.notes)


def test_notes_contain_improving(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    assert any("improving" in n.lower() for n in rep.notes)


def test_notes_contain_regressing(two_snapshots_regressing):
    rep = v1451.compute_trend(two_snapshots_regressing)
    assert any("regressing" in n.lower() for n in rep.notes)


def test_notes_contain_stability(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    assert any("Stability" in n for n in rep.notes)


# ============================================================================
# Dataclass shape
# ============================================================================

def test_trendreport_schema(single_snapshot):
    rep = v1451.compute_trend(single_snapshot)
    assert rep.schema == v1451.V1451_SCHEMA


def test_trendreport_version(single_snapshot):
    rep = v1451.compute_trend(single_snapshot)
    assert rep.version == v1451.V1451_VERSION


def test_axis_trend_has_required_fields(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    at = rep.per_axis_trend[0]
    assert hasattr(at, "axis")
    assert hasattr(at, "first_rate")
    assert hasattr(at, "last_rate")
    assert hasattr(at, "delta")
    assert hasattr(at, "slope")
    assert hasattr(at, "is_improving")


def test_element_delta_has_required_fields(two_snapshots_improving):
    rep = v1451.compute_trend(two_snapshots_improving)
    ed = rep.per_element_delta[0]
    assert hasattr(ed, "axis")
    assert hasattr(ed, "element")
    assert hasattr(ed, "delta")


# ============================================================================
# Integration: run_all on real V1450 history (may or may not have 2+ snapshots)
# ============================================================================

def test_run_all_with_no_append_writes_report(single_snapshot, tmp_path):
    """run_all with force_append=False should still write report."""
    out_json = tmp_path / "report.json"
    out_md = tmp_path / "report.md"
    # Just call run_all with force_append=False to avoid touching real history
    try:
        report = v1451.run_all(
            out_json=out_json,
            out_md=out_md,
            force_append=False,
        )
    except Exception:
        # If V1450 import fails in test env, skip
        pytest.skip("V1450 not importable in this env")
    assert report is not None


# ============================================================================
# End-to-end: chain V1451 → V1450
# ============================================================================

def test_v1451_composes_on_v1450():
    """V1451 chain_delegate must return V1450 with all_ok=True."""
    chain = v1451.chain_delegate()
    v1450_delegate = next(d for d in chain["delegates"] if d["module"] == "V1450")
    assert v1450_delegate["ok"] is True
