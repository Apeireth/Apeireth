"""Tests for Apeireth V1131 R10-W2 末综合 dashboard + ASI 北极星真测验证 (R10-A2-004).

12 测试类别 (主 17:43 实事求是 + 主 22:33 ASI 北极星):

  T1: V1131 模块导入与版本
  T2: 4 类 dashboard 输出 (kickoff / main_track / real_run / decision)
  T3: ASI 北极星 LOCKED (主 22:33)
  T4: 多 agent 共识复用 V1129
  T5: ASI level 真接口 (V1124 backend bridge /asi/level)
  T6: V0.5 公式真跑 (V1125 compute_v05_score)
  T7: 主轨道决策 (V1125 choose_r10_main_track)
  T8: R10 守门自检 (V1125 run_r10_guard_self_check)
  T9: W2/W4 真测目标 (主 13:31 大胆激进)
  T10: dashboard 跑时 <2.5s (V1118 perf 借鉴, 主 19:33)
  T11: chaos test (主 23:44 干到底)
  T12: Markdown 报告渲染 (主 17:43 实事求是)
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.v1131_r10_w2_comprehensive_dashboard import (  # noqa: E402
    VERSION,
    ASI_NORTH_STAR,
    W2_MID_TARGET,
    W4_ULTIMATE_TARGET,
    V1074_TARGET_S,
    DASHBOARD_CATEGORIES,
    V1131R10W2ComprehensiveDashboard,
    V1131R10W2ComprehensiveRunner,
    V1131ChaosReport,
    render_markdown_v1131,
    main,
)


# ---------------------------------------------------------------------------
# F1: 模块导入与版本 (3 tests)
# ---------------------------------------------------------------------------

class TestV1131Module:
    """T1: V1131 模块导入与版本 (主 17:43 实事求是)."""

    def test_version_is_string(self):
        assert isinstance(VERSION, str)
        assert len(VERSION) > 0

    def test_targets_locked(self):
        # ponytail: 继承 V1129 真测目标 (主 19:33)
        assert W2_MID_TARGET == 0.9000
        assert W4_ULTIMATE_TARGET == 0.9500

    def test_dashboard_categories_complete(self):
        assert "kickoff_summary" in DASHBOARD_CATEGORIES
        assert "main_track_summary" in DASHBOARD_CATEGORIES
        assert "real_run_summary" in DASHBOARD_CATEGORIES
        assert "decision_summary" in DASHBOARD_CATEGORIES
        assert len(DASHBOARD_CATEGORIES) == 4


# ---------------------------------------------------------------------------
# F2: 4 类 dashboard 输出 (4 tests)
# ---------------------------------------------------------------------------

class TestFourDashboardOutputs:
    """T2: 4 类 dashboard 输出 (主 17:43 实事求是: 每条都是数字 + 真测来源)."""

    def test_kickoff_summary_has_v05_total(self):
        runner = V1131R10W2ComprehensiveRunner()
        kick = runner.build_kickoff_summary()
        assert "v05_total" in kick
        assert 0.0 <= kick["v05_total"] <= 1.0
        assert "category" in kick
        assert kick["category"] == "kickoff_summary"

    def test_main_track_summary_has_multi_agent_consensus(self):
        runner = V1131R10W2ComprehensiveRunner()
        mt = runner.build_main_track_summary()
        assert "multi_agent_consensus" in mt
        assert 0.0 <= mt["multi_agent_consensus"] <= 1.0
        assert "main_track" in mt
        assert "main_track_name" in mt

    def test_real_run_summary_has_asi_north_star(self):
        runner = V1131R10W2ComprehensiveRunner()
        rr = runner.build_real_run_summary()
        # ponytail: ASI 北极星 LOCKED (主 22:33)
        assert rr["asi_north_star"] == ASI_NORTH_STAR
        assert rr["category"] == "real_run_summary"

    def test_decision_summary_has_main_track(self):
        runner = V1131R10W2ComprehensiveRunner()
        ds = runner.build_decision_summary()
        assert ds["main_track"] in {"A", "B", "C", "D"}
        assert ds["guard_pass"] is True
        assert ds["r10_mid_target"] == 0.90


# ---------------------------------------------------------------------------
# F3: ASI 北极星 LOCKED (主 22:33) (3 tests)
# ---------------------------------------------------------------------------

class TestNorthStarLocked:
    """T3: ASI 北极星 LOCKED (主 22:33) — 任何 LLM 接入即获 AGI/ASI 能力."""

    def test_asi_north_star_constant(self):
        # ponytail: 0.9800 LOCKED (主 22:33), 主 12:14 中央 AI 是永恒身份
        assert ASI_NORTH_STAR == 0.9800

    def test_dashboard_carries_asi_north_star(self):
        runner = V1131R10W2ComprehensiveRunner()
        dash = runner.build_dashboard()
        assert dash.asi_north_star == ASI_NORTH_STAR

    def test_comprehensive_includes_philosophy_guard(self):
        runner = V1131R10W2ComprehensiveRunner()
        rr = runner.build_real_run_summary()
        assert rr["philosophy_guard_subscore"] >= 0.0
        assert rr["philosophy_guard_subscore"] <= 1.0


# ---------------------------------------------------------------------------
# F4: 多 agent 共识复用 V1129 (3 tests)
# ---------------------------------------------------------------------------

class TestMultiAgentConsensus:
    """T4: 多 agent 共识复用 V1129 (主 19:33 走在前人经验上)."""

    def test_consensus_reused_from_v1129(self):
        runner = V1131R10W2ComprehensiveRunner()
        mt = runner.build_main_track_summary()
        # ponytail: MultiAgentDashboard 默认 3 agent (V1129 DEFAULT_AGENT_IDS)
        assert mt["n_agents_total"] >= 2
        assert "multi_agent_consensus" in mt

    def test_comprehensive_dashboard_has_consensus(self):
        runner = V1131R10W2ComprehensiveRunner()
        dash = runner.build_dashboard()
        assert dash.multi_agent_consensus >= 0.0
        assert dash.multi_agent_consensus <= 1.0

    def test_consensus_in_dashboard_dict(self):
        runner = V1131R10W2ComprehensiveRunner()
        dash = runner.build_dashboard()
        d = dash.to_dict()
        assert "multi_agent_consensus" in d
        assert d["multi_agent_consensus"] == round(dash.multi_agent_consensus, 6)


# ---------------------------------------------------------------------------
# F5: ASI level 真接口 (V1124 backend bridge /asi/level) (2 tests)
# ---------------------------------------------------------------------------

class TestASILevelRealInterface:
    """T5: ASI level 真接口 (V1124 backend bridge /asi/level) — 真测, 不 mock."""

    def test_asi_level_via_v1124_bridge(self):
        runner = V1131R10W2ComprehensiveRunner()
        kick = runner.build_kickoff_summary()
        asi_level = kick["asi_level"]
        # ponytail: 真接口 status=200 + score 有值 (主 17:43 实事求是)
        assert asi_level["status"] == 200
        assert asi_level["available"] is True
        assert asi_level["score"] is not None

    def test_asi_level_baseline_v04(self):
        runner = V1131R10W2ComprehensiveRunner()
        kick = runner.build_kickoff_summary()
        asi_level = kick["asi_level"]
        # ponytail: V0.4 baseline = 0.8538 (R9 W4 末 baseline)
        assert asi_level["baseline_v04"] == 0.8538


# ---------------------------------------------------------------------------
# F6: V0.5 公式真跑 (V1125 compute_v05_score) (3 tests)
# ---------------------------------------------------------------------------

class TestV05Formula:
    """T6: V0.5 公式真跑 (V1125 compute_v05_score, V0.4*0.85 + continuity*0.05 + ...)."""

    def test_v05_total_with_default_inputs(self):
        runner = V1131R10W2ComprehensiveRunner()
        v05 = runner.compute_v05_18dim()
        # ponytail: 0.8538*0.85 + 0.85*0.05*3 = 0.72573 + 0.1275 = 0.85323
        assert abs(v05["v05_total"] - 0.85323) < 0.001

    def test_v05_total_with_higher_inputs(self):
        runner = V1131R10W2ComprehensiveRunner(
            v04_score=0.92,
            continuity=0.95,
            autonomy=0.95,
            transferability=0.95,
        )
        v05 = runner.compute_v05_18dim()
        # 0.92*0.85 + 0.95*0.05*3 = 0.782 + 0.1425 = 0.9245
        assert v05["v05_total"] > 0.92
        assert v05["v05_total"] < 1.0

    def test_v05_total_lower_bound(self):
        runner = V1131R10W2ComprehensiveRunner(
            v04_score=0.5,
            continuity=0.5,
            autonomy=0.5,
            transferability=0.5,
        )
        v05 = runner.compute_v05_18dim()
        # 0.5*0.85 + 0.5*0.05*3 = 0.425 + 0.075 = 0.5
        assert v05["v05_total"] >= 0.5
        assert v05["v05_total"] <= 1.0


# ---------------------------------------------------------------------------
# F7: 主轨道决策 (V1125 choose_r10_main_track) (3 tests)
# ---------------------------------------------------------------------------

class TestMainTrackDecision:
    """T7: 主轨道决策 (V1125 choose_r10_main_track, R10 升级阈值上移)."""

    def test_default_main_track_decision(self):
        runner = V1131R10W2ComprehensiveRunner()
        ds = runner.build_decision_summary()
        # ponytail: 默认 V0.5 ≈ 0.8532 < 0.86 → Track A
        assert ds["main_track"] == "A"

    def test_main_track_with_v05_above_dgm_threshold(self):
        # ponytail: 0.88 ≤ V0.5 < 0.92 → Track D (DGM v0.5 真演化 ROI)
        runner = V1131R10W2ComprehensiveRunner(
            v04_score=0.95,
            continuity=0.95,
            autonomy=0.95,
            transferability=0.95,
        )
        ds = runner.build_decision_summary()
        # 0.95*0.85 + 0.95*0.05*3 = 0.8075 + 0.1425 = 0.95
        assert ds["main_track"] in {"C", "D"}
        assert ds["track_reason"] != ""

    def test_decision_summary_threshold_field(self):
        runner = V1131R10W2ComprehensiveRunner()
        ds = runner.build_decision_summary()
        # ponytail: track_threshold 复用 R10_TRACK_ULTIMATE_THRESHOLD = 0.92
        assert ds["track_threshold"] == 0.92


# ---------------------------------------------------------------------------
# F8: R10 守门自检 (V1125 run_r10_guard_self_check) (2 tests)
# ---------------------------------------------------------------------------

class TestR10GuardSelfCheck:
    """T8: R10 守门自检 (V1125 run_r10_guard_self_check, 主 17:43 实事求是)."""

    def test_guard_passes_for_valid_inputs(self):
        runner = V1131R10W2ComprehensiveRunner(
            v04_score=0.91,
            continuity=0.92,
            autonomy=0.92,
            transferability=0.92,
            philosophy_guard_pass_count=6,
        )
        ds = runner.build_decision_summary()
        assert ds["guard_pass"] is True

    def test_guard_violations_field_exists(self):
        runner = V1131R10W2ComprehensiveRunner()
        ds = runner.build_decision_summary()
        assert "guard_violations" in ds
        assert isinstance(ds["guard_violations"], list)


# ---------------------------------------------------------------------------
# F9: W2/W4 真测目标 (主 13:31 大胆激进) (3 tests)
# ---------------------------------------------------------------------------

class TestW2W4Targets:
    """T9: W2/W4 真测目标 (主 13:31 大胆激进, W2 ≥ 0.90 + W4 ≥ 0.95)."""

    def test_w2_pass_at_higher_v05(self):
        # ponytail: V0.5=0.9245 ≥ 0.90 → W2 PASS (主 13:31)
        runner = V1131R10W2ComprehensiveRunner(
            v04_score=0.92,
            continuity=0.95,
            autonomy=0.95,
            transferability=0.95,
        )
        dash = runner.build_dashboard()
        assert dash.w2_pass is True

    def test_w2_fail_at_default_v05(self):
        # ponytail: V0.5=0.8532 < 0.90 → W2 FAIL (默认 baseline)
        runner = V1131R10W2ComprehensiveRunner()
        dash = runner.build_dashboard()
        assert dash.w2_pass is False

    def test_w4_pass_at_max_v05(self):
        # ponytail: V0.5 ≥ 0.95 → W4 PASS (主 13:31)
        runner = V1131R10W2ComprehensiveRunner(
            v04_score=1.0,
            continuity=1.0,
            autonomy=1.0,
            transferability=1.0,
        )
        dash = runner.build_dashboard()
        assert dash.w4_pass is True


# ---------------------------------------------------------------------------
# F10: dashboard 跑时 <2.5s (V1118 perf 借鉴, 主 19:33) (2 tests)
# ---------------------------------------------------------------------------

class TestDashboardPerf:
    """T10: dashboard 跑时 <2.5s (V1074_TARGET_S, 借鉴 V1118 perf)."""

    def test_dashboard_under_target(self):
        runner = V1131R10W2ComprehensiveRunner()
        dash = runner.build_dashboard()
        # ponytail: 借鉴 V1118 V1074_TARGET_S = 2.50
        assert dash.elapsed_seconds < V1074_TARGET_S
        assert dash.perf_target_met is True

    def test_dashboard_target_constant(self):
        # ponytail: V1074_TARGET_S = 2.50 (借鉴 V1118, 主 19:33)
        assert V1074_TARGET_S == 2.50


# ---------------------------------------------------------------------------
# F11: chaos test (主 23:44 干到底) (3 tests)
# ---------------------------------------------------------------------------

class TestChaosTest:
    """T11: chaos test (主 23:44 干到底: dashboard 渲染失联时不丢数据)."""

    def test_chaos_report_dataclass(self):
        report = V1131ChaosReport(
            n_kickoff_dropped=1,
            n_main_track_dropped=1,
            n_real_run_dropped=1,
            measurement_preserved=True,
            dashboard_preserved=True,
            delta_mean=0.0,
            note="test chaos",
        )
        d = report.to_dict()
        assert d["measurement_preserved"] is True
        assert d["dashboard_preserved"] is True

    def test_chaos_run_via_method(self):
        runner = V1131R10W2ComprehensiveRunner()
        report = runner.chaos_test(n_drop=1)
        # ponytail: 渲染失联时 baseline 已保留, measurement_preserved = True (主 23:44)
        assert report.measurement_preserved is True
        assert report.dashboard_preserved is True

    def test_full_run_with_chaos(self):
        runner = V1131R10W2ComprehensiveRunner()
        result = runner.run(chaos=True)
        assert "chaos_test_summary" in result
        assert result["chaos_test_summary"]["measurement_preserved"] is True


# ---------------------------------------------------------------------------
# F12: Markdown 报告渲染 (主 17:43 实事求是) (3 tests)
# ---------------------------------------------------------------------------

class TestMarkdownReport:
    """T12: Markdown 报告渲染 (主 17:43 实事求是)."""

    def test_markdown_contains_north_star(self):
        runner = V1131R10W2ComprehensiveRunner()
        result = runner.run()
        md = render_markdown_v1131(result)
        # ponytail: ASI 北极星 0.9800 LOCKED (主 22:33)
        assert "0.9800" in md
        assert "LOCKED" in md

    def test_markdown_contains_4_summaries(self):
        runner = V1131R10W2ComprehensiveRunner()
        result = runner.run()
        md = render_markdown_v1131(result)
        # ponytail: 4 类 dashboard 都在
        assert "启动综合" in md
        assert "主推综合" in md
        assert "真跑综合" in md
        assert "决策综合" in md

    def test_markdown_contains_philosophy(self):
        runner = V1131R10W2ComprehensiveRunner()
        result = runner.run()
        md = render_markdown_v1131(result)
        # ponytail: 主哲学 LOCKED section
        assert "主哲学 LOCKED" in md


# ---------------------------------------------------------------------------
# F13: 综合真跑 / 端到端 (4 tests)
# ---------------------------------------------------------------------------

class TestEndToEndRun:
    """F13: 综合真跑 / 端到端 (主 00:56 一行可跑)."""

    def test_run_returns_all_4_summaries(self):
        runner = V1131R10W2ComprehensiveRunner()
        result = runner.run()
        assert "kickoff_summary" in result
        assert "main_track_summary" in result
        assert "real_run_summary" in result
        assert "decision_summary" in result

    def test_run_with_benchmark(self):
        runner = V1131R10W2ComprehensiveRunner()
        result = runner.run(benchmark=True)
        assert "benchmark" in result
        bench = result["benchmark"]
        assert bench["target_s"] == 2.5
        assert bench["mean_s"] < V1074_TARGET_S

    def test_dashboard_to_dict_serializable(self):
        runner = V1131R10W2ComprehensiveRunner()
        dash = runner.build_dashboard()
        d = dash.to_dict()
        # ponytail: 主 17:43 实事求是 — 全 JSON-serializable
        json.dumps(d, ensure_ascii=False)

    def test_full_chain_v04_91_w2_pass(self):
        """全链路真测: V0.4=0.91 → V0.5=0.9245 → W2 PASS (主 13:31)."""
        runner = V1131R10W2ComprehensiveRunner(
            v04_score=0.91,
            continuity=0.92,
            autonomy=0.92,
            transferability=0.92,
        )
        result = runner.run(chaos=True, benchmark=True)
        assert result["w2_pass"] is True
        assert result["multi_agent_consensus"] >= 0.0
        assert result["perf_target_met"] is True
        assert "chaos_test_summary" in result
        assert result["chaos_test_summary"]["measurement_preserved"] is True
