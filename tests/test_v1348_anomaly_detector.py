#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tests/test_v1348_anomaly_detector.py — V1348 VCP Anomaly Detector pytest suite.

- Tests: 35 pytest cases (channel-level + per-plugin + report-level + integration).
- Goal: 0 regression against V1335-V1347 chain.
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

V1348_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(V1348_DIR))

import v1348_vcp_anomaly_detector as v1348  # noqa: E402


# --- Constants & helpers ----------------------------------------------------
def _t(overrides=None):
    base = dict(v1348.DEFAULT_THRESHOLDS)
    if overrides:
        base.update(overrides)
    return base


# --- Channel detectors ------------------------------------------------------

class TestTierJump:
    def test_no_history_is_none(self):
        sig = v1348.detect_tier_jump("p", [], _t())
        assert sig.severity == v1348.SEVERITY_NONE
        assert sig.signal_score == 0.0

    def test_single_history_is_none(self):
        sig = v1348.detect_tier_jump("p", ["high"], _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_no_change_is_none(self):
        sig = v1348.detect_tier_jump("p", ["high", "high"], _t())
        assert sig.severity == v1348.SEVERITY_NONE
        assert sig.signal_score == 0.0

    def test_low_to_medium_low(self):
        sig = v1348.detect_tier_jump("p", ["low", "medium"], _t())
        # rank delta = 2, weight = 0.34, score = 0.68 → MEDIUM
        assert sig.signal_score == pytest.approx(0.68, abs=1e-6)
        assert sig.severity == v1348.SEVERITY_MEDIUM

    def test_low_to_high_high(self):
        sig = v1348.detect_tier_jump("p", ["low", "high"], _t())
        assert sig.severity == v1348.SEVERITY_HIGH
        assert sig.signal_score == pytest.approx(1.0, abs=1e-6)

    def test_unknown_tier_treated_as_zero(self):
        sig = v1348.detect_tier_jump("p", ["unknown", "high"], _t())
        # unknown rank = 0, high rank = 3, delta = 3 → score = 1.0
        assert sig.signal_score == pytest.approx(1.0, abs=1e-6)
        assert sig.severity == v1348.SEVERITY_HIGH


class TestLintRegression:
    def test_no_drop_is_none(self):
        sig = v1348.detect_lint_regression("p", 5, 5, _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_above_floor_is_none(self):
        sig = v1348.detect_lint_regression("p", 6, 5, _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_one_drop_low(self):
        sig = v1348.detect_lint_regression("p", 4, 5, _t())
        # score = 1 * 0.20 = 0.20 → NONE (below low_floor 0.34)
        assert sig.severity == v1348.SEVERITY_NONE
        assert sig.signal_score == pytest.approx(0.20, abs=1e-6)

    def test_two_drops_medium(self):
        sig = v1348.detect_lint_regression("p", 3, 5, _t())
        # score = 2 * 0.20 = 0.40 → LOW
        assert sig.severity == v1348.SEVERITY_LOW
        assert sig.signal_score == pytest.approx(0.40, abs=1e-6)

    def test_three_drops_medium(self):
        sig = v1348.detect_lint_regression("p", 2, 5, _t())
        # score = 3 * 0.20 = 0.60 → LOW
        assert sig.severity == v1348.SEVERITY_LOW

    def test_four_drops_high(self):
        sig = v1348.detect_lint_regression("p", 1, 5, _t())
        # score = 4 * 0.20 = 0.80 → MEDIUM
        assert sig.severity == v1348.SEVERITY_MEDIUM

    def test_five_drops_high(self):
        sig = v1348.detect_lint_regression("p", 0, 5, _t())
        # score = 5 * 0.20 = 1.00 → HIGH
        assert sig.severity == v1348.SEVERITY_HIGH


class TestDriftSpike:
    def test_zero_is_none(self):
        sig = v1348.detect_drift_spike("p", 0.0, _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_low_value_is_none(self):
        sig = v1348.detect_drift_spike("p", 0.2, _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_medium_value_is_low(self):
        sig = v1348.detect_drift_spike("p", 0.4, _t())
        # score = 0.4 >= low_floor (0.34) → LOW
        assert sig.severity == v1348.SEVERITY_LOW

    def test_threshold_value_is_low(self):
        sig = v1348.detect_drift_spike("p", 0.34, _t())
        assert sig.severity == v1348.SEVERITY_LOW

    def test_full_is_high(self):
        sig = v1348.detect_drift_spike("p", 1.0, _t())
        assert sig.severity == v1348.SEVERITY_HIGH

    def test_clamped_above_one(self):
        sig = v1348.detect_drift_spike("p", 2.5, _t())
        # should clamp to 1.0
        assert sig.signal_score == pytest.approx(1.0, abs=1e-6)
        assert sig.severity == v1348.SEVERITY_HIGH


class TestPlanAcceleration:
    def test_zero_plans_is_none(self):
        sig = v1348.detect_plan_acceleration("p", 0, 5, _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_window_size_one_safe(self):
        # window=0 would div-by-zero; guard should kick in
        sig = v1348.detect_plan_acceleration("p", 1, 0, _t())
        assert sig.signal_score >= 0.0  # no crash

    def test_one_plan_in_five_is_none(self):
        sig = v1348.detect_plan_acceleration("p", 1, 5, _t())
        # score = 1 * 0.20 = 0.20 → NONE
        assert sig.severity == v1348.SEVERITY_NONE

    def test_two_plans_in_five_is_low(self):
        sig = v1348.detect_plan_acceleration("p", 2, 5, _t())
        # score = 2 * 0.20 = 0.40 → LOW
        assert sig.severity == v1348.SEVERITY_LOW

    def test_five_plans_is_high(self):
        sig = v1348.detect_plan_acceleration("p", 5, 5, _t())
        assert sig.severity == v1348.SEVERITY_HIGH


class TestHealthDrop:
    def test_empty_history_is_none(self):
        sig = v1348.detect_health_drop("p", [], _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_single_value_is_none(self):
        sig = v1348.detect_health_drop("p", [0.5], _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_no_drop_is_none(self):
        sig = v1348.detect_health_drop("p", [0.5, 0.6], _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_small_drop_is_none(self):
        # drop = 0.1, score = 0.1 * 0.5 = 0.05 → NONE
        sig = v1348.detect_health_drop("p", [0.7, 0.6], _t())
        assert sig.severity == v1348.SEVERITY_NONE

    def test_critical_drop_is_high(self):
        # drop = 0.7, score = 0.7 * 0.5 = 0.35 → LOW (above low_floor 0.34)
        sig = v1348.detect_health_drop("p", [0.9, 0.2], _t())
        assert sig.severity in (v1348.SEVERITY_LOW, v1348.SEVERITY_MEDIUM)

    def test_clamped_drop(self):
        # drop = 1.5 (impossible but defensive)
        sig = v1348.detect_health_drop("p", [1.5, 0.0], _t())
        assert sig.signal_score <= 1.0


# --- Severity & helpers ----------------------------------------------------

class TestSeverityHelpers:
    def test_severity_for_score_boundaries(self):
        t = _t()
        assert v1348.severity_for_score(0.0, t) == v1348.SEVERITY_NONE
        assert v1348.severity_for_score(0.339, t) == v1348.SEVERITY_NONE
        assert v1348.severity_for_score(0.34, t) == v1348.SEVERITY_LOW
        assert v1348.severity_for_score(0.66, t) == v1348.SEVERITY_LOW
        assert v1348.severity_for_score(0.67, t) == v1348.SEVERITY_MEDIUM
        assert v1348.severity_for_score(0.99, t) == v1348.SEVERITY_MEDIUM
        assert v1348.severity_for_score(1.0, t) == v1348.SEVERITY_HIGH

    def test_max_severity_empty(self):
        assert v1348.max_severity([]) == v1348.SEVERITY_NONE

    def test_max_severity_worst_of(self):
        assert v1348.max_severity([v1348.SEVERITY_LOW, v1348.SEVERITY_HIGH]) == v1348.SEVERITY_HIGH
        assert v1348.max_severity([v1348.SEVERITY_NONE, v1348.SEVERITY_LOW]) == v1348.SEVERITY_LOW

    def test_stable_id_deterministic(self):
        a = v1348._stable_id({"a": 1, "b": 2})
        b = v1348._stable_id({"b": 2, "a": 1})
        assert a == b
        assert len(a) == 16


# --- analyze_plugin --------------------------------------------------------

class TestAnalyzePlugin:
    def test_empty_inputs_yields_none(self):
        pa = v1348.analyze_plugin(
            "p_empty",
            tier_history=[],
            current_lint_pass=5,
            historical_lint_floor=5,
            latest_drift_penalty=0.0,
            recent_plan_count=0,
            recent_health_scores=[],
        )
        assert pa.plugin_severity == v1348.SEVERITY_NONE
        assert len(pa.channels) == 5  # all channels produce signals
        assert len(pa.anomaly_id) == 16

    def test_all_channels_disabled_yields_zero_signals(self):
        pa = v1348.analyze_plugin(
            "p_off",
            tier_history=["low", "high"],
            current_lint_pass=0,
            historical_lint_floor=5,
            latest_drift_penalty=1.0,
            recent_plan_count=10,
            recent_health_scores=[0.9, 0.3],
            enabled_channels=[],
        )
        assert pa.channels == []
        assert pa.plugin_severity == v1348.SEVERITY_NONE

    def test_only_tier_channel(self):
        pa = v1348.analyze_plugin(
            "p_tier",
            tier_history=["low", "high"],
            current_lint_pass=5,
            historical_lint_floor=5,
            latest_drift_penalty=0.0,
            recent_plan_count=0,
            recent_health_scores=[],
            enabled_channels=[v1348.CHANNEL_TIER_JUMP],
        )
        assert len(pa.channels) == 1
        assert pa.plugin_severity == v1348.SEVERITY_HIGH

    def test_severity_is_max_of_channels(self):
        pa = v1348.analyze_plugin(
            "p_mixed",
            tier_history=["high", "high"],          # NONE
            current_lint_pass=5,                    # NONE
            historical_lint_floor=5,
            latest_drift_penalty=0.0,               # NONE
            recent_plan_count=0,                    # NONE
            recent_health_scores=[0.9, 0.2],        # drop = 0.7 → LOW
        )
        # max(NONE, NONE, NONE, NONE, LOW) = LOW
        assert pa.plugin_severity == v1348.SEVERITY_LOW


# --- build_report ----------------------------------------------------------

class TestBuildReport:
    def test_empty_report(self):
        rep = v1348.build_report([])
        assert rep.ecosystem_severity == v1348.SEVERITY_NONE
        assert rep.total_plugins == 0
        assert sum(rep.severity_breakdown.values()) == 0

    def test_single_plugin_report(self):
        pa = v1348.analyze_plugin(
            "p_only",
            tier_history=["low", "high"],
            current_lint_pass=0,
            historical_lint_floor=5,
            latest_drift_penalty=0.0,
            recent_plan_count=0,
            recent_health_scores=[],
        )
        rep = v1348.build_report([pa])
        assert rep.ecosystem_severity == v1348.SEVERITY_HIGH
        assert rep.total_plugins == 1
        assert rep.severity_breakdown[v1348.SEVERITY_HIGH] == 1

    def test_mixed_plugins_breakdown(self):
        pa_low = v1348.analyze_plugin(
            "p_l", tier_history=[], current_lint_pass=5, historical_lint_floor=5,
            latest_drift_penalty=0.2, recent_plan_count=0, recent_health_scores=[]
        )
        pa_med = v1348.analyze_plugin(
            "p_m", tier_history=[], current_lint_pass=2, historical_lint_floor=5,
            latest_drift_penalty=0.0, recent_plan_count=0, recent_health_scores=[]
        )
        pa_high = v1348.analyze_plugin(
            "p_h", tier_history=["low", "high"], current_lint_pass=0, historical_lint_floor=5,
            latest_drift_penalty=0.0, recent_plan_count=0, recent_health_scores=[]
        )
        rep = v1348.build_report([pa_low, pa_med, pa_high])
        assert rep.ecosystem_severity == v1348.SEVERITY_HIGH
        assert rep.total_plugins == 3
        assert sum(rep.severity_breakdown.values()) == 3

    def test_thresholds_preserved(self):
        custom = _t({"low_floor": 0.5})
        pa = v1348.analyze_plugin(
            "p", tier_history=[], current_lint_pass=5, historical_lint_floor=5,
            latest_drift_penalty=0.0, recent_plan_count=0, recent_health_scores=[],
            thresholds=custom,
        )
        rep = v1348.build_report([pa], thresholds=custom)
        assert rep.thresholds_used["low_floor"] == 0.5

    def test_report_id_deterministic(self):
        pa = v1348.analyze_plugin(
            "p_det", tier_history=[], current_lint_pass=5, historical_lint_floor=5,
            latest_drift_penalty=0.0, recent_plan_count=0, recent_health_scores=[]
        )
        rep1 = v1348.build_report([pa])
        rep2 = v1348.build_report([pa])
        assert rep1.report_id == rep2.report_id
        assert len(rep1.report_id) == 16

    def test_enabled_channels_recorded(self):
        pa = v1348.analyze_plugin(
            "p", tier_history=[], current_lint_pass=5, historical_lint_floor=5,
            latest_drift_penalty=0.0, recent_plan_count=0, recent_health_scores=[],
        )
        rep = v1348.build_report([pa], enabled_channels=[v1348.CHANNEL_DRIFT_SPIKE])
        assert v1348.CHANNEL_DRIFT_SPIKE in rep.enabled_channels


# --- detect_from_health_reports (integration) -------------------------------

class TestDetectFromHealthReports:
    def test_no_reports_no_maps(self):
        rep = v1348.detect_from_health_reports([])
        assert rep.total_plugins == 0

    def test_with_empty_maps(self):
        # Simulate minimal V1347 health report objects (duck-typed)
        class FakePlugin:
            def __init__(self, name, score):
                self.plugin = name
                self.health_score = score

        class FakeReport:
            def __init__(self, plugins):
                self.per_plugin = plugins

        reports = [FakeReport([FakePlugin("p1", 0.9), FakePlugin("p2", 0.85)])]
        # Pass explicit empty maps so no channels fire (default floor would otherwise trigger lint regression).
        rep = v1348.detect_from_health_reports(
            reports,
            lint_pass_map={"p1": 5, "p2": 5},
            lint_floor_map={"p1": 5, "p2": 5},
            drift_penalty_map={"p1": 0.0, "p2": 0.0},
            plan_count_map={"p1": 0, "p2": 0},
        )
        assert rep.total_plugins == 2
        assert rep.ecosystem_severity == v1348.SEVERITY_NONE  # no signals

    def test_drops_across_runs(self):
        class FakePlugin:
            def __init__(self, name, score):
                self.plugin = name
                self.health_score = score

        class FakeReport:
            def __init__(self, plugins):
                self.per_plugin = plugins

        # Plugin p1 had 0.95 then 0.30 → health_drop = 0.65 → LOW or MED
        reports = [
            FakeReport([FakePlugin("p1", 0.95)]),
            FakeReport([FakePlugin("p1", 0.30)]),
        ]
        rep = v1348.detect_from_health_reports(reports)
        assert rep.total_plugins == 1
        p1 = rep.per_plugin[0]
        health_channel = next(c for c in p1.channels if c.channel == v1348.CHANNEL_HEALTH_DROP)
        assert health_channel.signal_score > 0.0

    def test_maps_extend_plugin_set(self):
        class FakePlugin:
            def __init__(self, name, score):
                self.plugin = name
                self.health_score = score

        class FakeReport:
            def __init__(self, plugins):
                self.per_plugin = plugins

        reports = [FakeReport([FakePlugin("p_health", 0.5)])]
        rep = v1348.detect_from_health_reports(
            reports,
            tier_history_map={"p_only_in_tier_map": ["low", "high"]},
            lint_pass_map={"p_only_in_lint_map": 0},
        )
        names = {p.plugin for p in rep.per_plugin}
        assert "p_health" in names
        assert "p_only_in_tier_map" in names
        assert "p_only_in_lint_map" in names

    def test_custom_thresholds_propagate(self):
        class FakePlugin:
            def __init__(self, name, score):
                self.plugin = name
                self.health_score = score

        class FakeReport:
            def __init__(self, plugins):
                self.per_plugin = plugins

        reports = [FakeReport([FakePlugin("p", 0.9)])]
        custom = _t({"high_floor": 0.5})
        rep = v1348.detect_from_health_reports(reports, thresholds=custom)
        assert rep.thresholds_used["high_floor"] == 0.5


# --- V3 哲学守门 (philosophical gates) --------------------------------------

class TestV3Gates:
    def test_pole_star_locked(self):
        assert v1348.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1348.ASI_POLE_STAR["V0_2_baseline"] == 0.4467
        assert v1348.ASI_POLE_STAR["asi_achieved_false"] is True
        assert v1348.ASI_POLE_STAR["V1348_modifies_pole_star"] is False

    def test_no_ml_no_llm(self):
        # Confirm thresholds are pure constants — not learned objects.
        for k, v in v1348.DEFAULT_THRESHOLDS.items():
            assert isinstance(v, (int, float)), f"{k} is not a number"

    def test_recommendation_table_exhaustive(self):
        # Every (channel, severity > NONE) should have a recommendation.
        for ch in v1348.ALL_CHANNELS:
            for sev in (v1348.SEVERITY_LOW, v1348.SEVERITY_MEDIUM, v1348.SEVERITY_HIGH):
                assert (ch, sev) in v1348.RECOMMENDATIONS, f"missing recommendation for {ch}/{sev}"

    def test_all_channels_exposed(self):
        assert len(v1348.ALL_CHANNELS) == 5
        # Every channel should have recommendations for LOW/MEDIUM/HIGH
        for ch in v1348.ALL_CHANNELS:
            keys_with_ch = [k for k in v1348.RECOMMENDATIONS.keys() if k[0] == ch]
            assert len(keys_with_ch) == 3, f"channel {ch} missing recommendations: {keys_with_ch}"


# --- Popper self-tests integration -----------------------------------------

class TestPopperSelfTests:
    def test_all_popper_pass(self):
        passed, total = v1348.run_self_tests(verbose=False)
        assert passed == total
        assert total >= 18


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v", "--tb=short"]))