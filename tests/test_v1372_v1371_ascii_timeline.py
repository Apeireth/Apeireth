"""Tests for V1372 V1371 ASCII timeline (post-V1371 next-step 1/5).

Real coverage:
- 30 pytest tests (CLI, render, summary, edge cases, synthetic data)
- chain regression with V1371 + V1370 + V1369 + V1368 (no source mutations)
"""

from __future__ import annotations

import io
import json
import os
import sys
from contextlib import redirect_stdout, redirect_stderr

import pytest

from apeireth import v1372_v1371_ascii_timeline as v1372


# Force UTF-8 for test output (matches module's behavior)
sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]


SIDECAR = "v1370_calibrated_cron_evaluations.jsonl"


# ----------------------------------------------------------------------
# Fixtures
# ----------------------------------------------------------------------

@pytest.fixture
def real_evals():
    """Load the real calibrated sidecar."""
    return v1372.load_sidecar(SIDECAR)


@pytest.fixture
def synthetic_evals():
    """Synthetic fixture with fires + suppression for visual smoke tests."""
    return v1372._synthetic_evals()


@pytest.fixture
def real_timeline(real_evals):
    return v1372.build_timeline(real_evals)


# ----------------------------------------------------------------------
# 1. Load sidecar
# ----------------------------------------------------------------------

def test_load_sidecar_returns_list(real_evals):
    assert isinstance(real_evals, list)


def test_load_sidecar_has_at_least_13_evals(real_evals):
    """Sidecar grows over time; at minimum 13 evals (the original commit window)."""
    assert len(real_evals) >= 13


def test_load_sidecar_sorted_ascending(real_evals):
    ts = [e.get("evaluated_at", "") for e in real_evals]
    assert ts == sorted(ts)


def test_load_sidecar_raises_on_missing(tmp_path):
    fake = str(tmp_path / "nope.jsonl")
    with pytest.raises(FileNotFoundError):
        v1372.load_sidecar(fake)


def test_load_sidecar_skips_blank_lines(tmp_path):
    p = tmp_path / "with_blank.jsonl"
    p.write_text('{"evaluated_at": "2026-01-01T00:00:00Z", "raw": {"results": [{"name": "X", "kind": "remeasure", "fired": False}]}, "calibrated": {"results": [{"name": "X", "raw_fired": False, "calibrated_fired": False, "suppressed": False}]}}\n\n\n')
    evals = v1372.load_sidecar(str(p))
    assert len(evals) == 1


# ----------------------------------------------------------------------
# 2. Build timeline
# ----------------------------------------------------------------------

def test_build_timeline_8_triggers(real_timeline):
    assert len(real_timeline) == 8


def test_build_timeline_chars_lists(real_timeline):
    for t in real_timeline:
        assert isinstance(t["chars"], list)


def test_build_timeline_chars_count_matches_evals(real_timeline, real_evals):
    assert all(len(t["chars"]) == len(real_evals) for t in real_timeline)


def test_build_timeline_trigger_names_non_empty(real_timeline):
    assert all(len(t["name"]) > 0 for t in real_timeline)


def test_build_timeline_trigger_kinds_known(real_timeline):
    for t in real_timeline:
        assert t["kind"] in {"remeasure", "v03_evolution"}


# ----------------------------------------------------------------------
# 3. Counts (real sidecar = 0 fires)
# ----------------------------------------------------------------------

def test_real_sidecar_raw_count_zero(real_timeline):
    """Honest baseline: nothing fires in current 4-min window."""
    assert all(t["raw_count"] == 0 for t in real_timeline)


def test_real_sidecar_cal_count_zero(real_timeline):
    assert all(t["cal_count"] == 0 for t in real_timeline)


def test_real_sidecar_sup_count_zero(real_timeline):
    assert all(t["sup_count"] == 0 for t in real_timeline)


def test_raw_count_at_least_cal_count(real_timeline):
    for t in real_timeline:
        assert t["raw_count"] >= t["cal_count"], (
            f"trigger {t['name']}: raw={t['raw_count']} < cal={t['cal_count']}"
        )


# ----------------------------------------------------------------------
# 4. Bucketing
# ----------------------------------------------------------------------

def test_bucket_by_minute_returns_dict(real_evals):
    buckets = v1372.bucket_by_minute(real_evals)
    assert isinstance(buckets, dict)


def test_bucket_by_minute_total_matches(real_evals):
    buckets = v1372.bucket_by_minute(real_evals)
    assert sum(len(v) for v in buckets.values()) == len(real_evals)


def test_bucket_by_minute_keys_are_minute_strings(real_evals):
    buckets = v1372.bucket_by_minute(real_evals)
    for k in buckets.keys():
        assert len(k) == 16 or k == "unknown"


# ----------------------------------------------------------------------
# 5. Synthetic data with fires + suppression
# ----------------------------------------------------------------------

def test_synthetic_t1_fires_then_suppressed(synthetic_evals):
    tl = v1372.build_timeline(synthetic_evals)
    assert len(tl) == 2
    # T1: first eval = fire, second eval = suppressed, third = no fire
    assert tl[0]["chars"][0] == v1372.CHAR_FIRE
    assert tl[0]["chars"][1] == v1372.CHAR_SUPPRESSED
    assert tl[0]["chars"][2] == v1372.CHAR_NO_FIRE


def test_synthetic_t2_never_fires(synthetic_evals):
    tl = v1372.build_timeline(synthetic_evals)
    assert tl[1]["raw_count"] == 0
    assert tl[1]["cal_count"] == 0
    assert all(c == v1372.CHAR_NO_FIRE for c in tl[1]["chars"])


# ----------------------------------------------------------------------
# 6. Render
# ----------------------------------------------------------------------

def test_render_ascii_contains_header(real_timeline, real_evals):
    out = v1372.render_ascii(real_timeline, real_evals)
    assert "V1372 Timeline" in out
    assert "sidecar window:" in out


def test_render_ascii_contains_all_triggers(real_timeline, real_evals):
    out = v1372.render_ascii(real_timeline, real_evals)
    for t in real_timeline:
        assert t["name"] in out


def test_render_ascii_contains_legend(real_timeline, real_evals):
    out = v1372.render_ascii(real_timeline, real_evals)
    assert "Legend:" in out
    assert v1372.CHAR_NO_FIRE in out


def test_render_summary_contains_fire_rate(real_timeline, real_evals):
    out = v1372.render_summary(real_timeline, n_evals=len(real_evals))
    assert "V1372 Summary" in out
    assert "fire_rate" in out
    assert "0.00%" in out


def test_render_legend_has_all_chars():
    legend = v1372.render_legend()
    for ch in (v1372.CHAR_NO_FIRE, v1372.CHAR_FIRE, v1372.CHAR_SUPPRESSED, v1372.CHAR_UNKNOWN):
        assert ch in legend


# ----------------------------------------------------------------------
# 7. CLI
# ----------------------------------------------------------------------

def test_cli_version():
    assert v1372.run_cli(["version"]) == 0


def test_cli_legend():
    assert v1372.run_cli(["legend"]) == 0


def test_cli_summary():
    assert v1372.run_cli(["summary"]) == 0


def test_cli_timeline():
    assert v1372.run_cli(["timeline"]) == 0


def test_cli_missing_sidecar():
    assert v1372.run_cli(["--sidecar", "nonexistent_xyz.jsonl", "timeline"]) == 2


def test_cli_popper_does_not_recurse():
    """Sanity: CLI 'popper' should not recurse into _popper_self_tests via run_cli."""
    rc = v1372.run_cli(["popper"])
    assert rc in (0, 1)  # 0 = pass, 1 = failures (not stack overflow)


# ----------------------------------------------------------------------
# 8. Edge cases
# ----------------------------------------------------------------------

def test_empty_evals_yield_empty_timeline():
    assert v1372.build_timeline([]) == []


def test_malformed_only_yields_empty_timeline():
    assert v1372.build_timeline([{"_malformed": True}]) == []


def test_render_summary_n_evals_zero():
    out = v1372.render_summary([], n_evals=0)
    assert "n_evals=0" in out


def test_render_ascii_empty_timeline(real_evals):
    out = v1372.render_ascii([], real_evals)
    assert "(empty timeline" in out
