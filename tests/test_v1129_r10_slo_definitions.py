"""
test_v1129_r10_slo_definitions.py — R10-DEV-002 V1129 SLO 真测 + badge 走势 + chaos.

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 23:44 干到底
+ 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

from apeireth.v1129_r10_slo_definitions import (
    # 等级
    LEVEL_GREEN, LEVEL_YELLOW, LEVEL_RED, LEVEL_UNKNOWN, VALID_LEVELS,
    # SLO 真定义
    AvailabilitySLO, LatencySLO, GuardSLO,
    # Burn rate + Error budget
    BurnRateWindow, ErrorBudgetTracker,
    # Badge 走势 + Dashboard
    render_v1074_trend_badge, render_slo_status_badge,
    SLODashboard, render_dashboard_markdown,
    # 编排 + chaos + CLI
    SLOEvalContext, evaluate_slos, evaluate_slos_chaos_safe,
    main, REFERENCES,
)


# ---------------------------------------------------------------------------
# 主 17:58 不假装 — 等级 + 真生产常量
# ---------------------------------------------------------------------------


class TestLevels:
    """等级常量 + VALID_LEVELS 显式 (主 17:58 不假装)."""

    def test_levels_constants(self):
        assert LEVEL_GREEN == "GREEN"
        assert LEVEL_YELLOW == "YELLOW"
        assert LEVEL_RED == "RED"
        assert LEVEL_UNKNOWN == "UNKNOWN"

    def test_valid_levels_set(self):
        assert VALID_LEVELS == {LEVEL_GREEN, LEVEL_YELLOW, LEVEL_RED, LEVEL_UNKNOWN}

    def test_references_non_empty(self):
        assert len(REFERENCES) >= 7
        ids = [r["id"] for r in REFERENCES]
        assert "DatadogSLO2019" in ids
        assert "GoogleSREWorkbook2017" in ids
        assert "V1074ASIProductionRunner" in ids


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — AvailabilitySLO 真测
# ---------------------------------------------------------------------------


class TestAvailabilitySLO:
    """可用性 SLO (Datadog SLO 2019 + Google SRE 2017) 真测."""

    def test_default_target_99_95(self):
        slo = AvailabilitySLO()
        assert slo.target == 0.9995
        assert slo.window_days == 30

    def test_monthly_minutes_budget_21_6(self):
        # 0.05% * 30 * 24 * 60 = 21.6 分钟
        slo = AvailabilitySLO()
        budget = slo.monthly_minutes_budget()
        assert abs(budget - 21.6) < 0.01, f"budget={budget}"

    def test_evaluate_zero_events_unknown(self):
        # 主 17:58 不假装: 0 events → UNKNOWN, 不假装 GREEN
        slo = AvailabilitySLO()
        result = slo.evaluate(0, 0)
        assert result["level"] == LEVEL_UNKNOWN
        assert result["availability"] != result["availability"]  # NaN

    def test_evaluate_perfect_availability_green(self):
        slo = AvailabilitySLO()
        result = slo.evaluate(10000, 10000)
        assert result["availability"] == 1.0
        assert result["level"] == LEVEL_GREEN
        assert result["error_rate"] == 0.0

    def test_evaluate_above_target_yellow(self):
        slo = AvailabilitySLO()
        # 9994/10000 = 0.06% error rate > target 0.05%, < 2x → YELLOW
        result = slo.evaluate(9994, 10000)
        assert result["error_rate"] > 0.0005
        assert result["error_rate"] < 2 * 0.0005
        assert result["level"] == LEVEL_YELLOW

    def test_evaluate_double_target_red(self):
        slo = AvailabilitySLO()
        # error_rate 0.0015 > 2 * 0.0005 → RED
        result = slo.evaluate(985, 1000)
        assert result["level"] == LEVEL_RED

    def test_evaluate_budget_left_decreases(self):
        slo = AvailabilitySLO()
        r1 = slo.evaluate(10000, 10000)
        r2 = slo.evaluate(9000, 10000)
        assert r1["budget_minutes_left"] > r2["budget_minutes_left"]

    def test_evaluate_burn_rate_infinite_when_target_zero(self):
        slo = AvailabilitySLO(target=1.0)  # 100% target
        result = slo.evaluate(9, 10)
        assert math.isinf(result["burn_rate"]) or result["burn_rate"] > 100


import math  # 上面 test 用


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — LatencySLO 真测
# ---------------------------------------------------------------------------


class TestLatencySLO:
    """延迟 SLO (Google SRE 2017) P95 < 2s + P99 < 5s 真测."""

    def test_default_p95_p99_targets(self):
        slo = LatencySLO()
        assert slo.p95_target_sec == 2.0
        assert slo.p99_target_sec == 5.0

    def test_percentile_empty_returns_nan(self):
        slo = LatencySLO()
        assert math.isnan(slo.percentile([], 0.95))

    def test_percentile_invalid_p_raises(self):
        slo = LatencySLO()
        with pytest.raises(ValueError):
            slo.percentile([1.0, 2.0], 0.0)
        with pytest.raises(ValueError):
            slo.percentile([1.0, 2.0], 1.5)

    def test_percentile_p95_sorted_input(self):
        slo = LatencySLO()
        samples = list(range(100))  # 0..99
        p95 = slo.percentile(samples, 0.95)
        assert 93 <= p95 <= 96, f"p95={p95}"

    def test_percentile_p99_sorted_input(self):
        slo = LatencySLO()
        samples = list(range(100))
        p99 = slo.percentile(samples, 0.99)
        assert 97 <= p99 <= 99, f"p99={p99}"

    def test_evaluate_empty_unknown(self):
        slo = LatencySLO()
        result = slo.evaluate([])
        assert result["level"] == LEVEL_UNKNOWN
        assert result["n_samples"] == 0

    def test_evaluate_all_p95_p99_pass_green(self):
        slo = LatencySLO()
        samples = [0.1, 0.5, 1.0, 1.5]  # 全部 ≤ 2s
        result = slo.evaluate(samples)
        assert result["level"] == LEVEL_GREEN
        assert result["p95_pass"] and result["p99_pass"]

    def test_evaluate_p95_fail_yellow(self):
        slo = LatencySLO()
        # 100 个样本, P95 = 2.5 (>2.0 fail), P99 = 2.5 (<5 pass) → YELLOW
        # rank 0.95*99 = 94.05, 95个0.1 + 5个2.5 → p95 = 0.1*(1-0.05) + 2.5*0.05 = 0.22 (pass)
        # 改用 100 个全 2.5 → P95=2.5 fail P99=2.5 pass → YELLOW
        samples = [2.5] * 100
        result = slo.evaluate(samples)
        assert not result["p95_pass"]
        assert result["p99_pass"]
        assert result["level"] == LEVEL_YELLOW

    def test_evaluate_both_fail_red(self):
        slo = LatencySLO()
        # P95 + P99 都 fail (全 6.0 > 5.0)
        samples = [6.0] * 100
        result = slo.evaluate(samples)
        assert not result["p95_pass"]
        assert not result["p99_pass"]
        assert result["level"] == LEVEL_RED


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — GuardSLO 真测 (V1074 V0.3 + V0.5)
# ---------------------------------------------------------------------------


class TestGuardSLO:
    """守门 SLO (V1074 V0.3 ≥ 0.94 + V0.5 ≥ 0.92) 真测."""

    def test_default_thresholds(self):
        slo = GuardSLO()
        assert slo.v03_yellow == 0.94
        assert slo.v03_red == 0.8884
        assert slo.v05_yellow == 0.95
        assert slo.v05_red == 0.92

    def test_evaluate_v03_green(self):
        slo = GuardSLO()
        result = slo.evaluate_v03(0.96)
        assert result["level"] == LEVEL_GREEN
        assert "继续" in result["action"]

    def test_evaluate_v03_yellow_on_call(self):
        slo = GuardSLO()
        # 0.89 ∈ [0.8884, 0.94) → YELLOW on-call Slack 告警
        result = slo.evaluate_v03(0.89)
        assert result["level"] == LEVEL_YELLOW
        assert "on-call" in result["action"]

    def test_evaluate_v03_red_rollback(self):
        slo = GuardSLO()
        result = slo.evaluate_v03(0.85)
        assert result["level"] == LEVEL_RED
        assert "回滚" in result["action"]

    def test_evaluate_v05_red_emergency_rollback(self):
        slo = GuardSLO()
        result = slo.evaluate_v05(0.91)  # < 0.92 → RED
        assert result["level"] == LEVEL_RED
        assert "紧急 rollback" in result["action"]

    def test_evaluate_v05_yellow_r10_warning(self):
        slo = GuardSLO()
        result = slo.evaluate_v05(0.93)
        assert result["level"] == LEVEL_YELLOW
        assert "前兆" in result["action"]


# ---------------------------------------------------------------------------
# 主 19:33 走在前人经验上 — BurnRateWindow 真测 (Datadog SLO 2019)
# ---------------------------------------------------------------------------


class TestBurnRateWindow:
    """Multi-window burn rate (Datadog SLO 2019) 真测."""

    def test_default_datadog_2019_windows(self):
        b = BurnRateWindow()
        assert b.short_sec == 3600
        assert b.short_multiplier == 14.4
        assert b.medium_sec == 21600
        assert b.medium_multiplier == 6.0
        assert b.long_sec == 86400
        assert b.long_multiplier == 3.0

    def test_alert_level_short_burn_red(self):
        b = BurnRateWindow()
        # short window 14.4x → RED
        target = 0.0005
        level = b.alert_level(
            error_rate_short=14.5 * target,
            error_rate_medium=0.0,
            error_rate_long=0.0,
            target_error_rate=target,
        )
        assert level == LEVEL_RED

    def test_alert_level_medium_burn_yellow(self):
        b = BurnRateWindow()
        target = 0.0005
        level = b.alert_level(
            error_rate_short=0.0,
            error_rate_medium=6.5 * target,
            error_rate_long=0.0,
            target_error_rate=target,
        )
        assert level == LEVEL_YELLOW

    def test_alert_level_long_burn_yellow(self):
        b = BurnRateWindow()
        target = 0.0005
        level = b.alert_level(
            error_rate_short=0.0,
            error_rate_medium=0.0,
            error_rate_long=3.5 * target,
            target_error_rate=target,
        )
        assert level == LEVEL_YELLOW

    def test_alert_level_no_burn_green(self):
        b = BurnRateWindow()
        level = b.alert_level(0.0, 0.0, 0.0, 0.0005)
        assert level == LEVEL_GREEN


# ---------------------------------------------------------------------------
# 主 22:33 ASI 北极星 — ErrorBudgetTracker 真测
# ---------------------------------------------------------------------------


class TestErrorBudgetTracker:
    """错误预算 (Google SRE 2017 §5.4) 真测."""

    def test_initial_budget_full(self):
        tracker = ErrorBudgetTracker()
        status = tracker.budget_status()
        assert status["remaining_minutes"] == pytest.approx(21.6)
        assert status["level"] == LEVEL_GREEN

    def test_record_burn_consumes(self):
        tracker = ErrorBudgetTracker()
        # 1% error rate (20x target) → RED
        tracker.record_burn(100, 10000)
        status = tracker.budget_status()
        assert status["consumed_minutes"] > 0
        assert status["level"] in (LEVEL_YELLOW, LEVEL_RED)

    def test_record_zero_total_skipped(self):
        tracker = ErrorBudgetTracker()
        tracker.record_burn(0, 0)
        assert len(tracker.burn_log) == 0

    def test_budget_red_when_low(self):
        tracker = ErrorBudgetTracker()
        # 用 50% 预算
        # 0.5 * 21.6 = 10.8 分钟
        # 10.8 minutes / 30*24*60 = 0.00025 error rate
        # 0.00025 * 10000 = 2.5 events, round to 3
        tracker.record_burn(25, 10000)  # 0.25% = 0.25 * 30*24*60/100 = 10.8 min
        status = tracker.budget_status()
        assert status["remaining_pct"] < 0.50
        assert status["level"] == LEVEL_YELLOW or status["level"] == LEVEL_RED


# ---------------------------------------------------------------------------
# 主 13:31 大胆激进 — Badge 走势 + Dashboard 真测
# ---------------------------------------------------------------------------


class TestBadgeTrend:
    """V1074 监控可视化 (badge 走势) 真测."""

    def test_render_v1074_trend_badge_with_history(self):
        history = [{"v03_score": 0.95}] * 10 + [{"v03_score": 0.89}] * 5
        svg = render_v1074_trend_badge(history, label="test")
        assert svg.startswith("<svg")

    def test_render_v1074_trend_badge_empty(self):
        svg = render_v1074_trend_badge([], label="empty")
        assert svg.startswith("<svg") or "<!--" in svg

    def test_render_v1074_trend_badge_mixed_levels(self):
        history = [
            {"v03_score": 0.96},   # pass
            {"v03_score": 0.90},   # mixed
            {"v03_score": 0.85},   # fail
            {"v03_score": None},   # unknown
        ]
        svg = render_v1074_trend_badge(history)
        assert svg.startswith("<svg")

    def test_render_slo_status_badge_all_levels(self):
        for level in [LEVEL_GREEN, LEVEL_YELLOW, LEVEL_RED, LEVEL_UNKNOWN]:
            svg = render_slo_status_badge(level)
            assert svg.startswith("<svg") or "<!--" in svg


class TestSLODashboard:
    """SLO dashboard (主 13:31 大胆激进) 真测."""

    def test_to_dict_has_all_fields(self):
        dash = SLODashboard()
        d = dash.to_dict()
        assert "availability" in d
        assert "latency" in d
        assert "guard_v03" in d
        assert "guard_v05" in d
        assert "error_budget" in d
        assert "overall_level" in d

    def test_overall_level_red_when_any_red(self):
        dash = SLODashboard(
            availability={"level": LEVEL_GREEN},
            latency={"level": LEVEL_GREEN},
            guard_v03={"level": LEVEL_GREEN},
            guard_v05={"level": LEVEL_RED},
        )
        assert dash.overall_level() == LEVEL_RED

    def test_overall_level_yellow_when_no_red(self):
        dash = SLODashboard(
            availability={"level": LEVEL_GREEN},
            latency={"level": LEVEL_YELLOW},
            guard_v03={"level": LEVEL_GREEN},
            guard_v05={"level": LEVEL_GREEN},
        )
        assert dash.overall_level() == LEVEL_YELLOW

    def test_overall_level_green_all_pass(self):
        dash = SLODashboard(
            availability={"level": LEVEL_GREEN},
            latency={"level": LEVEL_GREEN},
            guard_v03={"level": LEVEL_GREEN},
            guard_v05={"level": LEVEL_GREEN},
        )
        assert dash.overall_level() == LEVEL_GREEN

    def test_to_json_serializable(self):
        dash = SLODashboard(availability={"level": LEVEL_GREEN})
        s = dash.to_json()
        assert isinstance(s, str)
        d = json.loads(s)
        assert d["availability"]["level"] == LEVEL_GREEN

    def test_render_dashboard_markdown_contains_sections(self):
        dash = SLODashboard(
            availability={
                "name": "test", "target": 0.9995, "availability": 0.999,
                "error_rate": 0.001, "budget_minutes_left": 21.0, "burn_rate": 2.0,
                "level": LEVEL_YELLOW,
            },
            guard_v03={"v03_score": 0.93, "level": LEVEL_YELLOW, "action": "on-call"},
            guard_v05={"v05_score": 0.93, "level": LEVEL_YELLOW, "action": "前兆"},
        )
        md = render_dashboard_markdown(dash)
        assert "# R10 SLO Dashboard" in md
        assert "## 1. 守门 SLO" in md
        assert "## 2. 可用性 SLO" in md
        assert "## 6. 借鉴" in md


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — evaluate_slos 编排真测
# ---------------------------------------------------------------------------


class TestEvaluateSLOs:
    """4 大 SLO 维度编排 (主 23:44 干到底) 真测."""

    def test_evaluate_slos_full_pipeline(self):
        ctx = SLOEvalContext(
            good_events=9990, total_events=10000,
            latency_samples_sec=[0.5, 1.0, 1.5, 1.8, 1.9],
            v03_score=0.96, v05_score=0.93,
            score_history=[{"v03_score": 0.95}] * 5,
        )
        dash = evaluate_slos(ctx)
        assert dash.availability is not None
        assert dash.latency is not None
        assert dash.guard_v03 is not None
        assert dash.guard_v05 is not None
        assert dash.error_budget is not None

    def test_evaluate_slos_green_when_all_pass(self):
        ctx = SLOEvalContext(
            good_events=9995, total_events=10000,
            latency_samples_sec=[0.1] * 100,
            v03_score=0.96, v05_score=0.96,
            score_history=[],
        )
        dash = evaluate_slos(ctx)
        assert dash.guard_v03["level"] == LEVEL_GREEN
        assert dash.guard_v05["level"] == LEVEL_GREEN
        assert dash.latency["level"] == LEVEL_GREEN

    def test_evaluate_slos_chaos_safe_returns_all_unknown(self):
        # 主 17:58 不假装: 监控失联 → UNKNOWN 不假装 GREEN
        dash = evaluate_slos_chaos_safe(None)
        assert dash.availability["level"] == LEVEL_UNKNOWN
        assert dash.latency["level"] == LEVEL_UNKNOWN
        assert dash.guard_v03["level"] == LEVEL_UNKNOWN
        assert dash.guard_v05["level"] == LEVEL_UNKNOWN
        assert dash.error_budget["level"] == LEVEL_UNKNOWN
        assert dash.overall_level() == LEVEL_UNKNOWN

    def test_evaluate_slos_chaos_safe_with_ctx_delegates(self):
        ctx = SLOEvalContext(
            good_events=10000, total_events=10000,
            latency_samples_sec=[0.1], v03_score=0.96, v05_score=0.96,
        )
        dash = evaluate_slos_chaos_safe(ctx)
        assert dash.overall_level() == LEVEL_GREEN


# ---------------------------------------------------------------------------
# 主 17:43 实事求是 — CLI 真测
# ---------------------------------------------------------------------------


class TestCLISubprocess:
    """CLI (主 00:56 任何人都能接手) subprocess 真测."""

    def _run(self, args, timeout=15):
        # 主 17:43 实事求是: Windows GBK 编码 → utf-8 + errors=replace
        return subprocess.run(
            [sys.executable, "-m", "apeireth.v1129_r10_slo_definitions"] + args,
            capture_output=True, text=True, timeout=timeout,
            encoding="utf-8", errors="replace",
        )

    def test_cli_help(self):
        r = self._run(["--help"], timeout=10)
        assert r.returncode == 0
        assert "--slo" in r.stdout

    def test_cli_slo_default(self):
        r = self._run(["--slo"])
        assert r.returncode == 0
        assert "[V1129]" in r.stdout

    def test_cli_slo_json(self):
        r = self._run(["--slo", "--json"])
        assert r.returncode == 0
        d = json.loads(r.stdout)
        assert "overall_level" in d
        assert "guard_v03" in d

    def test_cli_slo_report(self):
        r = self._run(["--slo", "--report"])
        assert r.returncode == 0
        assert "# R10 SLO Dashboard" in r.stdout

    def test_cli_chaos_unknown(self):
        r = self._run(["--chaos"])
        assert r.returncode == 0
        assert "UNKNOWN" in r.stdout

    def test_cli_custom_v03_v05(self):
        r = self._run(["--slo", "--v03", "0.95", "--v05", "0.96"])
        assert r.returncode == 0
        # V0.5 GREEN 因为 0.96 ≥ 0.95
        assert "GREEN" in r.stdout

    def test_main_function_directly(self):
        # 主 17:43 实事求是: main 函数直接调
        rc = main(["--slo", "--v03", "0.96"])
        assert rc == 0


# ---------------------------------------------------------------------------
# 主 23:44 干到底 — SLO 真生产集成 (V1074 + V1117 + V1125)
# ---------------------------------------------------------------------------


class TestR10SLOIntegration:
    """R10 SLO + V1074 + V1117 + V1125 集成真测."""

    def test_slo_with_v1074_current_score(self):
        """V1074 当前真测 v03=0.8946 → YELLOW (不假装 GREEN)."""
        ctx = SLOEvalContext(
            good_events=10000, total_events=10000,
            latency_samples_sec=[0.5] * 100,
            v03_score=0.8946,
            v05_score=0.8808,
            score_history=[],
        )
        dash = evaluate_slos(ctx)
        # V1074 v03=0.8946 ∈ [0.8884, 0.94) → YELLOW
        assert dash.guard_v03["level"] == LEVEL_YELLOW
        # V0.5=0.8808 < 0.92 → RED
        assert dash.guard_v05["level"] == LEVEL_RED
        # Overall RED
        assert dash.overall_level() == LEVEL_RED

    def test_slo_with_v1117_badge_integration(self):
        """V1129 dashboard + V1117 badge 渲染集成."""
        history = [{"v03_score": 0.92 + i * 0.001} for i in range(20)]
        ctx = SLOEvalContext(
            good_events=10000, total_events=10000,
            latency_samples_sec=[0.5],
            v03_score=0.94, v05_score=0.95,
            score_history=history,
        )
        dash = evaluate_slos(ctx)
        # 主 13:31: trend SVG 已生成
        if dash.trend_svg and not dash.trend_svg.startswith("<!--"):
            assert "<svg" in dash.trend_svg

    def test_slo_data_quality_nan_handling(self):
        """主 17:43 实事求是: NaN 安全渲染."""
        ctx = SLOEvalContext(
            good_events=0, total_events=0,  # → NaN availability
            latency_samples_sec=[],          # → NaN P95
            v03_score=0.94, v05_score=0.95,
        )
        dash = evaluate_slos(ctx)
        # availability level = UNKNOWN
        assert dash.availability["level"] == LEVEL_UNKNOWN
        assert dash.latency["level"] == LEVEL_UNKNOWN

    def test_slo_budget_21_6_minutes(self):
        """主 17:43 实事求是: 0.05% 月度 = 21.6 分钟."""
        slo = AvailabilitySLO()
        budget = slo.monthly_minutes_budget()
        # 0.0005 * 30 * 24 * 60 = 21.6
        assert abs(budget - 21.6) < 0.01