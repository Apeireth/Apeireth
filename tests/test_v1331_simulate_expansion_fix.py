#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1331_simulate_expansion_fix.py — V1331 bug fix tests

- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Tests: 8 tests proving the V1330 simulate_expansion bug is fixed
"""
import importlib
import os
import sys
from pathlib import Path

WORKSPACE = Path(r".openclaw\workspace\promethean")
sys.path.insert(0, str(WORKSPACE))
sys.path.insert(0, str(WORKSPACE / "apeireth"))

MODULE_NAME = "apeireth.v1331_simulate_expansion_fixed"


def _import_v1331():
    return importlib.import_module(MODULE_NAME)


# ============================================================
# Section 1: Module imports + pole star
# ============================================================

def test_module_imports():
    mod = _import_v1331()
    assert mod is not None


def test_v1331_pole_star_locked():
    """V1331 does NOT modify the pole star."""
    mod = _import_v1331()
    ps = mod.ASI_POLE_STAR
    assert ps["V0_1_actual_measured"] == 0.7905
    assert ps["asi_achieved_false"] is True
    assert ps["V1330_modifies_pole_star"] is False


def test_v1331_fix_notes():
    mod = _import_v1331()
    notes = mod.V1331_FIX_NOTES
    assert notes["v3_guards_locked"] is True
    assert notes["pole_star_modified"] is False
    assert notes["asi_achieved"] is False
    assert notes["fix_constant_recent"] == 3
    assert notes["fix_constant_mid"] == 2


# ============================================================
# Section 2: simulate_expansion fixed behavior (5)
# ============================================================

def test_simulate_expansion_no_expansion():
    """Enough files -> no expansion, boundaries stay at initial."""
    mod = _import_v1331()
    t = mod.TimelineBucketSubstrate
    recent, mid = t.simulate_expansion(recent_files_count=100, mid_files_count=100)
    assert recent == t.INITIAL_RECENT_DAYS  # 7
    assert mid == t.INITIAL_MID_DAYS        # 90


def test_simulate_expansion_full_expansion():
    """Zero files -> full expansion to max."""
    mod = _import_v1331()
    t = mod.TimelineBucketSubstrate
    recent, mid = t.simulate_expansion(recent_files_count=0, mid_files_count=0)
    assert recent == t.RECENT_EXPAND_MAX  # 30
    assert mid == t.MID_EXPAND_MAX        # 180


def test_simulate_expansion_partial_recent():
    """Few recent files (< 3) -> recent expands to RECENT_EXPAND_MAX (30)."""
    mod = _import_v1331()
    t = mod.TimelineBucketSubstrate
    recent, mid = t.simulate_expansion(recent_files_count=2, mid_files_count=100)
    # recent: 2 < 3, expand 7->14->21->28->30 (capped at 30)
    assert recent == 30
    # mid: 100 >= 2, no expansion
    assert mid == t.INITIAL_MID_DAYS  # 90


def test_simulate_expansion_partial_mid():
    """Few mid files (< 2) -> mid expands."""
    mod = _import_v1331()
    t = mod.TimelineBucketSubstrate
    recent, mid = t.simulate_expansion(recent_files_count=100, mid_files_count=1)
    # recent: 100 >= 3, no expansion
    assert recent == t.INITIAL_RECENT_DAYS  # 7
    # mid: 1 < 2, expand 90->120->150->180 (capped at 180)
    assert mid == 180


def test_simulate_expansion_monotonic():
    """More files -> same or smaller boundary (monotonic)."""
    mod = _import_v1331()
    t = mod.TimelineBucketSubstrate
    r1, m1 = t.simulate_expansion(0, 0)
    r2, m2 = t.simulate_expansion(100, 100)
    assert r1 >= r2, f"Not monotonic: r1={r1}, r2={r2}"
    assert m1 >= m2, f"Not monotonic: m1={m1}, m2={m2}"


# ============================================================
# Section 3: V1331 = re-export of V1330 (3)
# ============================================================

def test_v1331_re_exports_v1330_classes():
    mod = _import_v1331()
    # All V1330 classes should be re-exported
    assert hasattr(mod, "AgentDreamFileSubstrate")
    assert hasattr(mod, "AgentDreamPluginMatrix")
    assert hasattr(mod, "AgentDreamDeepReadReport")
    assert hasattr(mod, "AgentDreamDeepReadBridge")
    assert hasattr(mod, "DreamSchedulerSubstrate")
    assert hasattr(mod, "DreamConfigSubstrate")
    assert hasattr(mod, "AgentRegistrySubstrate")
    assert hasattr(mod, "AuthorExtractSubstrate")
    assert hasattr(mod, "BroadcastSubstrate")
    assert hasattr(mod, "BroadcastEvent")
    assert hasattr(mod, "DreamPromptSubstrate")
    assert hasattr(mod, "DreamStatePersistSubstrate")
    assert hasattr(mod, "DreamAgentEntry")


def test_v1331_re_exports_v1330_constants():
    mod = _import_v1331()
    assert mod.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
    assert mod.TOTAL_DECLARED_LINES == 1815
    assert len(mod.AGENTDREAM_4_FILES) == 4


def test_v1331_v1330_functionality_preserved():
    """V1331 should preserve all V1330 functionality (just patches simulate_expansion)."""
    mod = _import_v1331()
    # Test that other V1330 methods still work via re-export
    scheduler = mod.DreamSchedulerSubstrate.describe()
    assert scheduler.check_interval_ms == 900_000
    
    config = mod.DreamConfigSubstrate.from_env({})
    assert config.frequency_hours == 8
    
    registry = mod.AgentRegistrySubstrate.from_env({})
    assert registry.agents == {}


# ============================================================
# Section 4: V3 哲学守门 (2)
# ============================================================

def test_v1331_not_pretend_fix_bigger_than_it_is():
    """V1331 is a bug fix, NOT a redesign of the timeline algorithm."""
    mod = _import_v1331()
    # The fix matches V1330's documented contract (literal 3 for recent, 2 for mid)
    assert mod.V1331_FIX_NOTES["fix_strategy"] == "monkey-patch simulate_expansion with corrected implementation"


def test_v1331_does_not_modify_v1330():
    """V1331 should be a separate file, NOT modify V1330 in-place."""
    v1330_path = WORKSPACE / "apeireth" / "v1330_agentdream_plugin_deep_read.py"
    v1331_path = WORKSPACE / "apeireth" / "v1331_simulate_expansion_fixed.py"
    assert v1330_path.exists()
    assert v1331_path.exists()
    assert v1330_path != v1331_path
    # V1330 module should still have the buggy simulate_expansion (untouched)
    v1330_src = v1330_path.read_text(encoding="utf-8")
    assert "cls.min_recent_files.__class__(3)" in v1330_src  # bug preserved in V1330 for honesty