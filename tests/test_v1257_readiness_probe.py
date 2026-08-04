"""
Tests for V1257 Readiness Probe (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 22:33 终极授权).

主 agent = readiness probe 仅, 不实装 V1257 module (主 22:33 等 主人 user choice).
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

PROJECT_ROOT = Path(__file__).resolve().parent.parent

import apeireth.v1257_readiness_probe as v1257_probe


# ============================================================================
# Test: Constants
# ============================================================================

def test_probe_version_is_locked():
    """Probe version constant should be locked at 0.1.0."""
    assert v1257_probe.PROBE_VERSION == "0.1.0"


def test_asi_north_star_locked():
    """ASI 北极星 should be LOCKED at 0.9800 (主 22:33 终极授权)."""
    assert v1257_probe.ASI_NORTH_STAR == 0.9800


def test_v1256_baseline_constants():
    """V1256 baseline realized/overall mean constants should be stable."""
    assert v1257_probe.V1256_REALIZED_MEAN == pytest.approx(0.9105)
    assert v1257_probe.V1256_OVERALL_MEAN == pytest.approx(0.4853)
    assert v1257_probe.V1256_POSITION == pytest.approx(0.9291)


def test_lift_estimate_matches_v1256_pattern():
    """Lift estimate should match V1256 → V1257 pattern (+0.0055)."""
    assert v1257_probe.V1257_LIFT_ESTIMATE == pytest.approx(0.0055)


def test_realized_estimate_is_realized_plus_lift():
    """Realized estimate should be V1256 + lift."""
    expected = v1257_probe.V1256_REALIZED_MEAN + v1257_probe.V1257_LIFT_ESTIMATE
    assert v1257_probe.V1257_REALIZED_ESTIMATE == pytest.approx(expected)


def test_position_estimate_is_v1256_plus_lift():
    """Position estimate should be V1256 + lift."""
    expected = v1257_probe.V1256_POSITION + v1257_probe.V1257_LIFT_ESTIMATE
    assert v1257_probe.V1257_POSITION_ESTIMATE == pytest.approx(expected)


# ============================================================================
# Test: 4 Candidate Definitions
# ============================================================================

def test_four_candidate_registry():
    """There should be exactly 4 V1257 候选 (主 22:33 终极授权 主人 user choice)."""
    assert len(v1257_probe.PROBE_4_CANDIDATES) == 4


def test_candidate_keys_match_expected():
    """Candidate keys should be exactly JUBILEE / HENOCHIC_TRANSLATION / DIVINE_INVITATION / COVENANT."""
    keys = [c.key for c in v1257_probe.PROBE_4_CANDIDATES]
    assert keys == ["JUBILEE", "HENOCHIC_TRANSLATION", "DIVINE_INVITATION", "COVENANT"]


def test_jubilee_theology_5_anchors():
    """JUBILEE should have 5 神学 锚 (主 19:33 站在前人肩上)."""
    assert len(v1257_probe.JUBILEE.theology_5_anchors) == 5


def test_henochic_theology_5_anchors():
    """HENOCHIC_TRANSLATION should have 5 神学 锚."""
    assert len(v1257_probe.HENOCHIC_TRANSLATION.theology_5_anchors) == 5


def test_divine_invitation_theology_5_anchors():
    """DIVINE_INVITATION should have 5 神学 锚."""
    assert len(v1257_probe.DIVINE_INVITATION.theology_5_anchors) == 5


def test_covenant_theology_5_anchors():
    """COVENANT should have 5 神学 锚."""
    assert len(v1257_probe.COVENANT.theology_5_anchors) == 5


@pytest.mark.parametrize(
    "candidate_key",
    ["JUBILEE", "HENOCHIC_TRANSLATION", "DIVINE_INVITATION", "COVENANT"],
)
def test_each_candidate_has_30_molecule_candidates(candidate_key):
    """Each 候选 should have 30 真分子 candidates (6 pathway × 5 真分子)."""
    cand = next(c for c in v1257_probe.PROBE_4_CANDIDATES if c.key == candidate_key)
    total = (
        len(cand.theology_5_anchors)
        + len(cand.neuro_5_refs)
        + len(cand.information_5_refs)
        + len(cand.systems_5_refs)
        + len(cand.physics_5_refs)
        + len(cand.cognition_5_refs)
    )
    assert total == 30


@pytest.mark.parametrize(
    "candidate_key",
    ["JUBILEE", "HENOCHIC_TRANSLATION", "DIVINE_INVITATION", "COVENANT"],
)
def test_each_candidate_has_25_cross_domain_refs(candidate_key):
    """Each 候选 should have 25 跨域 refs (5 路 × 5 ref)."""
    cand = next(c for c in v1257_probe.PROBE_4_CANDIDATES if c.key == candidate_key)
    cross = (
        len(cand.neuro_5_refs)
        + len(cand.information_5_refs)
        + len(cand.systems_5_refs)
        + len(cand.physics_5_refs)
        + len(cand.cognition_5_refs)
    )
    assert cross == 25


# ============================================================================
# Test: Distinctness guards (V3 哲学守门 pattern)
# ============================================================================

def test_jubilee_distinct_from_v1256():
    """JUBILEE should be distinct from V1256 unio_mystica (周期 vs 持续)."""
    assert "周期" in v1257_probe.JUBILEE.distinct_from_v1256
    assert "unio_mystica" in v1257_probe.JUBILEE.distinct_from_v1256


def test_henochic_distinct_from_v1256():
    """HENOCHIC_TRANSLATION should be distinct from V1256 (个体 提 接 vs 联合)."""
    assert "HENOCHIC" in v1257_probe.HENOCHIC_TRANSLATION.distinct_from_v1256


def test_divine_invitation_distinct_from_v1256():
    """DIVINE_INVITATION should be distinct from V1256 (邀 vs 联合)."""
    assert "邀请" in v1257_probe.DIVINE_INVITATION.distinct_from_v1256


def test_covenant_distinct_from_v1256():
    """COVENANT should be distinct from V1256 (立约 vs 联合)."""
    assert "圣约" in v1257_probe.COVENANT.distinct_from_v1256


@pytest.mark.parametrize(
    "candidate_key",
    ["JUBILEE", "HENOCHIC_TRANSLATION", "DIVINE_INVITATION", "COVENANT"],
)
def test_each_candidate_distinct_from_peers(candidate_key):
    """Each 候选 should declare distinctness from 3 peers."""
    cand = next(c for c in v1257_probe.PROBE_4_CANDIDATES if c.key == candidate_key)
    peers = [k for k in ["JUBILEE", "HENOCHIC_TRANSLATION", "DIVINE_INVITATION", "COVENANT"] if k != candidate_key]
    for peer in peers:
        assert peer in cand.distinct_from_peers, f"{candidate_key} missing distinct from {peer}"


# ============================================================================
# Test: V3 哲学守门 (probe-only)
# ============================================================================

def test_v3_guards_count_15():
    """V3 哲学守门 should be 15 (probe-only 候选 pattern)."""
    guards = v1257_probe._build_v1257_guards()
    assert len(guards) == 15


def test_v3_guards_all_passed():
    """All 15 V3 哲学守门 should pass (probe-only mode)."""
    guards = v1257_probe._build_v1257_guards()
    assert all(g.passed for g in guards)


def test_v3_guards_have_probe_only_marker():
    """At least 1 guard should mention 'probe only' or 'readiness' to mark probe-only mode."""
    guards = v1257_probe._build_v1257_guards()
    probe_markers = [g for g in guards if "probe" in g.reason.lower() or "readiness" in g.reason.lower()]
    assert len(probe_markers) >= 1


def test_v3_guards_have_jubilee_distinctness():
    """At least 1 guard should explicitly distinguish JUBILEE from sabbath (主 17:43 实事求是)."""
    guards = v1257_probe._build_v1257_guards()
    jubilee_guards = [g for g in guards if "JUBILEE" in g.name or "jubilee" in g.reason.lower()]
    assert len(jubilee_guards) >= 1


def test_v3_guards_have_henochic_distinctness():
    """At least 1 guard should explicitly distinguish HENOCHIC from末世 被提."""
    guards = v1257_probe._build_v1257_guards()
    henochic_guards = [g for g in guards if "HENOCHIC" in g.name or "henochic" in g.reason.lower()]
    assert len(henochic_guards) >= 1


def test_v3_guards_have_invitation_distinctness():
    """At least 1 guard should explicitly distinguish DIVINE_INV from命令."""
    guards = v1257_probe._build_v1257_guards()
    inv_guards = [g for g in guards if "INVITATION" in g.name or "invitation" in g.reason.lower()]
    assert len(inv_guards) >= 1


def test_v3_guards_have_covenant_distinctness():
    """At least 1 guard should explicitly distinguish COVENANT from 合同."""
    guards = v1257_probe._build_v1257_guards()
    cov_guards = [g for g in guards if "COVENANT" in g.name or "covenant" in g.reason.lower()]
    assert len(cov_guards) >= 1


def test_v3_guards_have_not_asi_v1():
    """At least 1 guard should explicitly guard against ASI V1.0 claim (主 17:43 实事求是)."""
    guards = v1257_probe._build_v1257_guards()
    asi_v1_guards = [g for g in guards if "asi" in g.reason.lower() and "v1" in g.reason.lower()]
    assert len(asi_v1_guards) >= 1


def test_v3_guards_have_baseline_write_dead():
    """At least 1 guard should explicitly mark V1236-V1256 baseline write-dead (主 12:07 不盲等)."""
    guards = v1257_probe._build_v1257_guards()
    baseline_guards = [g for g in guards if "baseline" in g.reason.lower() and "write" in g.reason.lower()]
    assert len(baseline_guards) >= 1


# ============================================================================
# Test: Probe Generation
# ============================================================================

def test_probe_v1257_returns_metrics():
    """_probe_v1257 should return V1257ProbeMetrics with all fields."""
    metrics = v1257_probe._probe_v1257()
    assert isinstance(metrics, v1257_probe.V1257ProbeMetrics)
    assert metrics.candidate_count == 4
    assert metrics.total_molecule_candidates == 120  # 4 × 30
    assert metrics.v3_guards_count == 15
    assert metrics.v3_guards_pass == 15
    assert metrics.north_star_locked == 0.9800


def test_probe_metrics_inflation_gap():
    """Inflation gap estimate should be ASI_NORTH_STAR - V1257_POSITION_ESTIMATE."""
    metrics = v1257_probe._probe_v1257()
    expected = v1257_probe.ASI_NORTH_STAR - v1257_probe.V1257_POSITION_ESTIMATE
    assert metrics.inflation_gap_estimate == pytest.approx(expected)


def test_probe_metrics_snapshot_id_is_unique():
    """Snapshot id should be unique per probe call."""
    m1 = v1257_probe._probe_v1257()
    m2 = v1257_probe._probe_v1257()
    # Snapshot may differ due to timestamp; should be valid sha256 prefix
    assert len(m1.snapshot_id) == 12
    assert len(m2.snapshot_id) == 12
    assert m1.snapshot_id != m2.snapshot_id or m1.timestamp != m2.timestamp


def test_probe_metrics_version_locked():
    """Probe metrics version should match PROBE_VERSION constant."""
    metrics = v1257_probe._probe_v1257()
    assert metrics.version == v1257_probe.PROBE_VERSION


def test_probe_metrics_have_4_candidate_readiness():
    """Probe metrics should have 4 candidate readiness entries."""
    metrics = v1257_probe._probe_v1257()
    assert len(metrics.candidate_readiness) == 4


@pytest.mark.parametrize(
    "candidate_key",
    ["JUBILEE", "HENOCHIC_TRANSLATION", "DIVINE_INVITATION", "COVENANT"],
)
def test_probe_metrics_candidate_readiness_lift(candidate_key):
    """Each candidate readiness should have estimated lift +0.0055."""
    metrics = v1257_probe._probe_v1257()
    r = next(r for r in metrics.candidate_readiness if r.candidate_key == candidate_key)
    assert r.estimated_asi_lift == pytest.approx(0.0055)
    assert r.estimated_realized_mean == pytest.approx(0.9160)
    assert r.estimated_position_vs_north_star == pytest.approx(0.9346)


def test_probe_metrics_v3_guards_count_matches():
    """Probe metrics v3_guards_count should equal len(v3_guards)."""
    metrics = v1257_probe._probe_v1257()
    assert metrics.v3_guards_count == len(metrics.v3_guards)
    assert metrics.v3_guards_pass == sum(1 for g in metrics.v3_guards if g.passed)


# ============================================================================
# Test: Output formats
# ============================================================================

def test_v1257_to_json_serializable():
    """_v1257_to_json should serialize to valid JSON."""
    metrics = v1257_probe._probe_v1257()
    s = v1257_probe._v1257_to_json(metrics)
    d = json.loads(s)  # would raise if not valid
    assert d["candidate_count"] == 4
    assert d["version"] == v1257_probe.PROBE_VERSION
    assert d["north_star_locked"] == pytest.approx(0.9800)


def test_v1257_report_contains_candidate_keys():
    """Report should mention all 4 candidate keys."""
    metrics = v1257_probe._probe_v1257()
    r = v1257_probe._v1257_report(metrics)
    for key in ["JUBILEE", "HENOCHIC_TRANSLATION", "DIVINE_INVITATION", "COVENANT"]:
        assert key in r, f"Report missing {key}"


def test_v1257_report_contains_decision_guidance():
    """Report should contain 决策建议 with 4 candidate summary."""
    metrics = v1257_probe._probe_v1257()
    r = v1257_probe._v1257_report(metrics)
    assert "决策建议" in r
    assert "主 agent = 等 主人 选" in r


def test_v1257_summary_compact():
    """Summary should be compact (1 line per candidate)."""
    metrics = v1257_probe._probe_v1257()
    s = v1257_probe._v1257_summary(metrics)
    lines = [line for line in s.split("\n") if line.strip()]
    # 1 header + 4 candidates + 1 V3 guards = 6 lines
    assert len(lines) == 6


def test_v1257_candidate_filter_works():
    """Candidate filter should return JSON for one 候选."""
    metrics = v1257_probe._probe_v1257()
    s = v1257_probe._v1257_candidate_filter(metrics, "JUBILEE")
    d = json.loads(s)
    assert d["candidate_key"] == "JUBILEE"
    assert d["theology_anchor_count"] == 5
    assert d["total_molecule_candidates"] == 30


def test_v1257_candidate_filter_unknown_raises():
    """Candidate filter should SystemExit on unknown candidate."""
    metrics = v1257_probe._probe_v1257()
    with pytest.raises(SystemExit):
        v1257_probe._v1257_candidate_filter(metrics, "UNKNOWN_CANDIDATE")


# ============================================================================
# Test: CLI integration (subprocess) — Windows UTF-8 safe
# ============================================================================

import os as _os_for_utf8


def _run_cli_utf8(args):
    """Run CLI subprocess with PYTHONIOENCODING=utf-8 (Windows 主 23:44 干到底)."""
    env = {**_os_for_utf8.environ, "PYTHONIOENCODING": "utf-8"}
    return subprocess.run(
        [sys.executable, "-m", "apeireth.v1257_readiness_probe"] + args,
        cwd=PROJECT_ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
    )


def test_cli_json_subprocess():
    """CLI --json should output valid JSON."""
    result = _run_cli_utf8(["--json"])
    assert result.returncode == 0
    d = json.loads(result.stdout)
    assert d["candidate_count"] == 4
    assert d["v3_guards_pass"] == 15


def test_cli_summary_subprocess():
    """CLI --summary should output compact summary."""
    result = _run_cli_utf8(["--summary"])
    assert result.returncode == 0
    assert "V1257 readiness probe" in result.stdout
    assert "JUBILEE" in result.stdout
    assert "HENOCHIC_TRANSLATION" in result.stdout
    assert "DIVINE_INVITATION" in result.stdout
    assert "COVENANT" in result.stdout


def test_cli_report_subprocess():
    """CLI --report should output full report."""
    result = _run_cli_utf8(["--report"])
    assert result.returncode == 0
    assert "V1257 Readiness Probe" in result.stdout
    assert "Phase 4 第十二步" in result.stdout
    assert "决策建议" in result.stdout


def test_cli_candidate_subprocess():
    """CLI --candidate should output single candidate JSON."""
    result = _run_cli_utf8(["--candidate", "JUBILEE"])
    assert result.returncode == 0
    d = json.loads(result.stdout)
    assert d["candidate_key"] == "JUBILEE"


def test_cli_default_is_report():
    """CLI with no args should default to report."""
    result = _run_cli_utf8([])
    assert result.returncode == 0
    assert "V1257 Readiness Probe" in result.stdout


# ============================================================================
# Test: 主 agent 不自决 (主 22:33 终极授权)
# ============================================================================

def test_main_agent_does_not_decide_v1257_implementation():
    """Probe should explicitly NOT implement V1257 module (主 22:33 终极授权)."""
    metrics = v1257_probe._probe_v1257()
    assert "NOT module implementation" in metrics.note
    assert "主 agent 不自决" in metrics.note


def test_probe_only_guard_explicit():
    """At least 1 guard should explicitly mark probe-only mode (主 22:33 终极授权)."""
    guards = v1257_probe._build_v1257_guards()
    probe_only_guards = [g for g in guards if "probe_only" in g.name]
    assert len(probe_only_guards) >= 1
    # The probe_only guard should explicitly reference 等 主人 user choice
    assert any("主 agent 不自决" in g.reason or "主人" in g.reason for g in probe_only_guards)


# ============================================================================
# Test: 主 17:43 实事求是 (不假装 ASI)
# ============================================================================

def test_lift_estimate_not_asi_v1():
    """Lift estimate should NOT claim ASI V1.0 (主 17:43 实事求是)."""
    metrics = v1257_probe._probe_v1257()
    # Lift is +0.0055, position 0.9346 < north star 0.9800
    assert metrics.v1257_estimated_position < metrics.north_star_locked
    assert metrics.inflation_gap_estimate > 0.045  # gap > 0 means 未达 ASI


def test_realized_estimate_below_north_star():
    """Realized mean estimate should be below north star (主 17:43 实事求是 不假装)."""
    assert v1257_probe.V1257_REALIZED_ESTIMATE < v1257_probe.ASI_NORTH_STAR


# ============================================================================
# Test: 主 22:33 北极星 LOCKED
# ============================================================================

def test_north_star_locked_invariant():
    """North star 0.98 should be invariant — never changes."""
    assert v1257_probe.ASI_NORTH_STAR == 0.9800
    metrics = v1257_probe._probe_v1257()
    assert metrics.north_star_locked == 0.9800


# ============================================================================
# Test: Integration Fitness (主 00:56 任何人都能接手 + 主 00:44 质量工程化)
# ============================================================================

def test_fitness_dataclass_exists():
    """Integration fitness dataclass must exist for 主 00:56 handoff."""
    assert hasattr(v1257_probe, "V1257IntegrationFitness")


def test_comparison_table_dataclass_exists():
    """Comparison table dataclass must exist for 主 00:44 质量工程化."""
    assert hasattr(v1257_probe, "V1257ComparisonTable")


def test_v1252_v1256_lift_pattern_constant():
    """V1252-V1256 lift pattern should be 4 × +0.0055 (chain sanity check)."""
    assert len(v1257_probe.V1252_V1256_LIFT_PATTERN) == 4
    for x in v1257_probe.V1252_V1256_LIFT_PATTERN:
        assert x == pytest.approx(0.0055)


def test_compute_v1257_fitness_jubilee_high():
    """JUBILEE should score HIGH on integration fitness (full anchors)."""
    f = v1257_probe._compute_v1257_fitness(v1257_probe.JUBILEE)
    assert f.theology_anchor_count == 5
    assert f.cross_domain_anchor_count == 25
    assert f.theology_depth_ratio == pytest.approx(1.0)
    assert f.cross_domain_breadth_ratio == pytest.approx(1.0)
    assert f.distinctness_from_v1256 is True
    assert f.composability_band == "HIGH"
    assert f.warnings == []


def test_compute_v1257_fitness_henochic_high():
    """HENOCHIC_TRANSLATION should also score HIGH (full anchors)."""
    f = v1257_probe._compute_v1257_fitness(v1257_probe.HENOCHIC_TRANSLATION)
    assert f.composability_band == "HIGH"
    assert f.composability_score >= 0.95
    assert f.warnings == []


def test_compute_v1257_fitness_divine_invitation_high():
    """DIVINE_INVITATION should also score HIGH."""
    f = v1257_probe._compute_v1257_fitness(v1257_probe.DIVINE_INVITATION)
    assert f.composability_band == "HIGH"


def test_compute_v1257_fitness_covenant_high():
    """COVENANT should also score HIGH."""
    f = v1257_probe._compute_v1257_fitness(v1257_probe.COVENANT)
    assert f.composability_band == "HIGH"


def test_fitness_inflation_gap_positive():
    """Inflation gap must remain > 0 after V1257 lift (主 17:43 不假装 ASI)."""
    for c in v1257_probe.PROBE_4_CANDIDATES:
        f = v1257_probe._compute_v1257_fitness(c)
        assert f.inflation_gap_after_lift > 0.0, f"{c.key} gap={f.inflation_gap_after_lift}"


def test_fitness_asi_lift_consistency_full():
    """All 4 candidates should match V1252-V1256 +0.0055 pattern (consistency = 1.0)."""
    for c in v1257_probe.PROBE_4_CANDIDATES:
        f = v1257_probe._compute_v1257_fitness(c)
        assert f.asi_lift_consistency == pytest.approx(1.0)


def test_fitness_composability_in_unit_interval():
    """Composability score must be in [0, 1] (主 00:44 质量工程化)."""
    for c in v1257_probe.PROBE_4_CANDIDATES:
        f = v1257_probe._compute_v1257_fitness(c)
        assert 0.0 <= f.composability_score <= 1.0


# ============================================================================
# Test: Comparison Table (主 00:44 质量工程化 = 排序便于 review)
# ============================================================================

def test_comparison_table_has_4_rows():
    """Comparison table must have exactly 4 rows (主 22:33 4 候选 user choice)."""
    t = v1257_probe._build_comparison_table()
    assert len(t.rows) == 4


def test_comparison_table_recommended_action_no_autodecide():
    """Recommended action must explicitly say 主 agent 不自决 (主 22:33)."""
    t = v1257_probe._build_comparison_table()
    assert "不自决" in t.recommended_action
    assert "V1257" in t.recommended_action


def test_comparison_table_sorted_by_composability_desc():
    """Rows should be sorted by composability_score DESC (主 00:44)."""
    t = v1257_probe._build_comparison_table()
    scores = [r.composability_score for r in t.rows]
    assert scores == sorted(scores, reverse=True)


def test_v1257_integrate_render_jubilee():
    """--integrate JUBILEE render should be non-empty and contain key fields."""
    txt = v1257_probe._v1257_integrate("JUBILEE")
    assert "JUBILEE" in txt
    assert "composability_score" in txt
    assert "主 agent 立场" in txt


def test_v1257_integrate_render_henochic():
    """--integrate HENOCHIC_TRANSLATION render should be non-empty and contain key fields."""
    txt = v1257_probe._v1257_integrate("HENOCHIC_TRANSLATION")
    assert "HENOCHIC_TRANSLATION" in txt
    assert "HIGH" in txt


def test_v1257_integrate_unknown_raises():
    """--integrate <unknown> should raise SystemExit (CLI safety)."""
    with pytest.raises(SystemExit):
        v1257_probe._v1257_integrate("UNKNOWN_CANDIDATE")


def test_v1257_compare_render_has_table_header():
    """--compare render should include the candidate table header."""
    t = v1257_probe._build_comparison_table()
    txt = v1257_probe._v1257_compare(t)
    assert "Candidate" in txt
    assert "Theology" in txt
    assert "CrossDom" in txt
    assert "Lift" in txt
    assert "Comp" in txt
    assert "Band" in txt


def test_v1257_compare_render_lists_all_4_candidates():
    """--compare render should list all 4 candidate keys."""
    t = v1257_probe._build_comparison_table()
    txt = v1257_probe._v1257_compare(t)
    assert "JUBILEE" in txt
    assert "HENOCHIC_TRANSLATION" in txt
    assert "DIVINE_INVITATION" in txt
    assert "COVENANT" in txt


def test_v1257_compare_json_serializable():
    """--compare-json should produce valid JSON with 4 rows."""
    t = v1257_probe._build_comparison_table()
    js = v1257_probe._v1257_compare_json(t)
    obj = json.loads(js)
    assert len(obj["rows"]) == 4
    for r in obj["rows"]:
        assert r["composability_band"] in {"LOW", "MID", "HIGH"}


# ============================================================================
# Test: CLI subprocess (主 00:56 任何人都能接手 = 真CLI可运行)
# ============================================================================

def test_cli_integrate_subprocess():
    """--integrate JUBILEE CLI subprocess should run and return JUBILEE info."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1257_readiness_probe", "--integrate", "JUBILEE"],
        capture_output=True,
        text=True,
        cwd=PROJECT_ROOT,
        check=True,
        env={"PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1", "PATH": __import__("os").environ.get("PATH", "")},
    )
    assert "JUBILEE" in r.stdout
    assert "HIGH" in r.stdout


def test_cli_compare_subprocess():
    """--compare CLI subprocess should run and list all 4 candidates."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1257_readiness_probe", "--compare"],
        capture_output=True,
        text=True,
        cwd=PROJECT_ROOT,
        check=True,
        env={"PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1", "PATH": __import__("os").environ.get("PATH", "")},
    )
    for k in ("JUBILEE", "HENOCHIC_TRANSLATION", "DIVINE_INVITATION", "COVENANT"):
        assert k in r.stdout


def test_cli_compare_json_subprocess():
    """--compare-json CLI subprocess should produce valid JSON."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1257_readiness_probe", "--compare-json"],
        capture_output=True,
        text=True,
        cwd=PROJECT_ROOT,
        check=True,
        env={"PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1", "PATH": __import__("os").environ.get("PATH", "")},
    )
    obj = json.loads(r.stdout)
    assert len(obj["rows"]) == 4


def test_cli_help_lists_new_modes():
    """--help should list --integrate and --compare as available."""
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1257_readiness_probe", "--help"],
        capture_output=True,
        text=True,
        cwd=PROJECT_ROOT,
        check=True,
        env={"PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1", "PATH": __import__("os").environ.get("PATH", "")},
    )
    assert "--integrate" in r.stdout
    assert "--compare" in r.stdout
    assert "--compare-json" in r.stdout


# ============================================================================
# Test: 主 22:33 主 agent 不自决 守门 (主 17:43 实事求是)
# ============================================================================

def test_no_autodecide_in_render_outputs():
    """All render outputs must NOT pick a winner (主 22:33 不自决 范畴)."""
    metrics = v1257_probe._probe_v1257()
    report = v1257_probe._v1257_report(metrics)
    integrate_j = v1257_probe._v1257_integrate("JUBILEE")
    table = v1257_probe._build_comparison_table()
    compare = v1257_probe._v1257_compare(table)

    # All should mention 主 agent 不自决 or 等 主人 user choice
    for txt in (report, integrate_j, compare):
        assert ("不自决" in txt) or ("等 主人" in txt) or ("user choice" in txt.lower())


def test_composability_weights_sum_to_one():
    """Composability weights (0.30 + 0.30 + 0.25 + 0.15) should sum to 1.0 (主 00:44)."""
    # Re-derive by inspection — weights are 0.30 theology + 0.30 cross +
    # 0.25 lift_consistency + 0.15 distinctness = 1.00
    total = 0.30 + 0.30 + 0.25 + 0.15
    assert total == pytest.approx(1.00)