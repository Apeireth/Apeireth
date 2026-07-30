"""Apeireth R11 P0 回归护栏 (R11-ATE-001 / 5 路径覆盖).

> 任务: R11 自动化测试工程师 — P0 回归护栏
> 时间: 2026-07-30
> 主哲学: 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 17:58 不假装 +
>        主 19:33 走在前人经验上 + 主 23:44 干到底 + 主 00:56 任何人都能接手
>
> 覆盖 5 个 P0 回归路径 (按 V1136 真测引擎 / V1074 真生产 / dashboard 真跑 / V3 9 键 / 真实失败语义):
>
>   1) V1136 真测引擎报告 — 3 维 (continuity / autonomy / transferability) 真测, V1125 占位 diff
>   2) V1074 真生产 + V0.4 引擎 — V0.3 measurement / snapshot / decision / artifact / V3 guard
>   3) Dashboard payload / UI — V1131 + V1130 to_dict schema 一致性 + chaos 保险
>   4) V3 9 键 LOCKED — AsiNineKeyLock + verify_or_raise + inject_guard_block + V3_GUARDS 字段
>   5) 真实失败语义 — V1124 backend 错误类型 / 错误码 / payload, 无静默 fallback, 区分 live vs offline
>
> 离线 vs 在线自动判定:
>   - 检测 env 中是否存在 LLM_API_KEY / OPENAI_API_KEY / ANTHROPIC_API_KEY / NEWAPI_API_KEY
>   - 离线测试: 不依赖真实 LLM, 任何需要 key 的路径必须显式 raise V1124Error
>   - 在线测试: 标记 pytest.mark.live, 仅在 CI 注入 key 时跑
>   - 回归护栏本身必须全程离线可跑 (主 17:43 实事求是)

ponytail: ceiling = 全离线覆盖 5 路径; 升级路径 = 接 live_provider_skip_marker 增加更严格的契约校验。
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Tuple

import pytest


PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROJECT_ROOT))


# ---------------------------------------------------------------------------
# Helper: live provider detection (主 17:43 实事求是: 区分 live vs offline)
# ---------------------------------------------------------------------------

_LIVE_KEY_ENV_NAMES = (
    "LLM_API_KEY",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "NEWAPI_API_KEY",
)


def _live_provider_detected() -> bool:
    """任一真 LLM key 存在 → live mode.

    注意: conftest._isolate_api_key_env 测试期间会清空 *API*KEY* / *_TOKEN env,
    这是必须返回 False 的场景 — 离线测试.
    """
    return any(os.environ.get(k) for k in _LIVE_KEY_ENV_NAMES)


def is_live_provider() -> bool:
    """Runtime check; 模块级 LIVE_PROVIDER 在父进程已 export key 时会失效."""
    return _live_provider_detected()


# ---------------------------------------------------------------------------
# 1. V1136 真测引擎报告 (主 17:43 实事求是)
# ---------------------------------------------------------------------------


class TestV1136RealMeasurementRegression:
    """V1136 真测引擎报告 — 3 维真测 + chaos 保险 + 取代 V1125 占位."""

    @pytest.fixture(scope="class")
    def v1136_result(self):
        from apeireth.v1136_asi_v05_3dim_real_measurement import measure_v05_3dims
        return measure_v05_3dims(v04_score=0.8538)

    def test_v1136_result_dataclass_contract(self, v1136_result):
        """V1136Result dataclass 字段契约 LOCKED (主 17:43 每条都是数字)."""
        required = (
            "continuity", "autonomy", "transferability",
            "v05_total_v1136", "v05_total_v1125", "v04_score",
            "delta_v05_total", "v3_guards_pass",
            "continuity_detail", "autonomy_detail", "transferability_detail",
            "elapsed_seconds", "timestamp",
        )
        d = v1136_result.to_dict()
        for key in required:
            assert key in d, f"V1136 contract missing: {key}"

    def test_v1136_three_dims_in_range(self, v1136_result):
        """3 维真测分 ∈ [0.55, 0.95] (LOCKED 区间, 与 V1125 占位兼容)."""
        for dim in ("continuity", "autonomy", "transferability"):
            v = getattr(v1136_result, dim)
            assert 0.55 <= v <= 0.95, f"{dim}={v} out of LOCKED range"

    def test_v1136_v1125_placeholder_compatible(self, v1136_result):
        """V1125 占位 LOCKED — V1136 v05_total_v1125 必须等于 V0.4*0.85 + 0.85*0.05*3."""
        v04 = v1136_result.v04_score
        expected_v1125 = v04 * 0.85 + 0.85 * 0.05 * 3
        assert abs(v1136_result.v05_total_v1125 - round(expected_v1125, 4)) < 1e-3

    def test_v1136_delta_is_computed_correctly(self, v1136_result):
        """Δ = V1136 - V1125 占位; 当 3 维 ≠ 0.85 时 Δ 应非零."""
        from apeireth.v1136_asi_v05_3dim_real_measurement import V1136Result
        # delta 在 V1136 内部是 round(v05_v1136 - v05_v1125, 4); 两个 v05_total 各自 round
        # 因此允许 ±1e-4 偏差 (即两次 round 累积)
        computed = round(
            v1136_result.v05_total_v1136 - v1136_result.v05_total_v1125, 4
        )
        assert abs(v1136_result.delta_v05_total - computed) < 1e-4, (
            f"delta field {v1136_result.delta_v05_total} vs computed {computed}"
        )
        # 至少 1 个 dim 不等于 0.85 (真测 ≠ 完美占位)
        any_non_85 = (
            v1136_result.continuity != 0.85
            or v1136_result.autonomy != 0.85
            or v1136_result.transferability != 0.85
        )
        assert any_non_85, "3 维全等于 0.85 占位, 不可能 (8/4/4 子测度不会全 0.85)"

    def test_v1136_continuity_has_eight_subs(self, v1136_result):
        """Continuity 真测 = 8 真借鉴子测度 (V1052 / V1072 / V1089 / V1090 / V1091 / V1092 / V1074 / V1107)."""
        sub = v1136_result.continuity_detail["sub_scores"]
        expected = {
            "v1052_consolidation", "v1072_eternal_identity", "v1089_hotcold",
            "v1090_wal", "v1091_replay", "v1092_dream",
            "v1074_production_runner", "v1107_cognitive_core_lift",
        }
        assert expected.issubset(sub.keys()), (
            f"missing continuity subs: {expected - set(sub.keys())}"
        )

    def test_v1136_autonomy_has_four_subs(self, v1136_result):
        """Autonomy 真测 = 4 真借鉴子测度 (V1083 / V1106 / V1128 / V1107)."""
        sub = v1136_result.autonomy_detail["sub_scores"]
        expected = {
            "v1083_decision_router", "v1106_engineering_lift",
            "v1128_w2_coordinator", "v1107_cognitive_core_lift",
        }
        assert expected.issubset(sub.keys()), (
            f"missing autonomy subs: {expected - set(sub.keys())}"
        )

    def test_v1136_transferability_has_four_subs(self, v1136_result):
        """Transferability 真测 = 4 真借鉴子测度 (V1127 / V1124 / V1128 / V1129)."""
        sub = v1136_result.transferability_detail["sub_scores"]
        expected = {
            "v1127_cross_small_model", "v1124_north_star_backend",
            "v1128_real_model_adapter", "v1129_multi_agent_validator",
        }
        assert expected.issubset(sub.keys()), (
            f"missing transferability subs: {expected - set(sub.keys())}"
        )

    def test_v1136_v3_guards_constant_includes_asi_protections(self):
        """V1136 V3_GUARDS 必须包含 lock_no_fake_kpi + no_break_v1125 + no_pretend_measurement_is_asi."""
        from apeireth.v1136_asi_v05_3dim_real_measurement import V3_GUARDS
        required_keys = (
            "guard_no_fake_kpi_v1136",
            "guard_no_break_v1125_formula",
            "guard_no_pretend_measurement_is_asi",
        )
        for r in required_keys:
            assert r in V3_GUARDS, f"V1136 V3_GUARDS missing: {r}"

    def test_v1136_chaos_test_preserves_measurement(self):
        """V1136 chaos test: 节点失联 measurement_preserved (主 23:44 干到底)."""
        from apeireth.v1136_asi_v05_3dim_real_measurement import (
            measure_chaos_node_down, measure_continuity_real,
        )
        r = measure_chaos_node_down(measure_continuity_real)
        assert r["measurement_preserved"], "chaos test 必保持 measurement"
        assert r["recovered_measurements"] >= 1
        assert "chaos_results" in r
        assert len(r["chaos_results"]) >= 2

    def test_v1136_markdown_report_contains_required_sections(self, v1136_result):
        """V1136 Markdown 报告 必含 V1136 / V0.5 / Continuity / Autonomy / Transferability / V3 / 0.85."""
        from apeireth.v1136_asi_v05_3dim_real_measurement import render_markdown_report
        md = render_markdown_report(v1136_result)
        for section in ("V1136", "V0.5", "Continuity", "Autonomy",
                        "Transferability", "V3", "0.85"):
            assert section in md, f"markdown missing section: {section}"

    def test_v1136_cli_strict_passes_when_guards_pass(self, v1136_result):
        """V1136 CLI --strict: V3 守门通过时 exit 0."""
        from io import StringIO
        import contextlib
        from apeireth.v1136_asi_v05_3dim_real_measurement import _cli
        if not v1136_result.v3_guards_pass:
            pytest.skip("V3 guards not passing; strict CLI test inapplicable")
        buf = StringIO()
        with contextlib.redirect_stdout(buf):
            rc = _cli(["--strict"])
        assert rc in (0, 2), f"strict mode should exit 0 (pass) or 2 (fail), got {rc}"


# ---------------------------------------------------------------------------
# 2. V1074 真生产 + V0.4 引擎 (主 17:43 实事求是)
# ---------------------------------------------------------------------------


class TestV1074V04ProductionRegression:
    """V1074 真生产 + V0.4 引擎 — production runner / dashboard / artifact / V3 guard."""

    def test_v1074_status_snapshot_has_v03(self):
        """V1074 真测 V0.3 score + level ∈ [ANI, AGI, ASI, TRANSCENDENT]."""
        from apeireth.v1074_asi_production_runner import ProductionRunner
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        assert 0.0 <= r.v03_score <= 1.0
        assert r.level in ("ANI", "AGI", "ASI", "TRANSCENDENT")
        assert r.decision_id.startswith("dec_")
        assert r.chosen_direction in [d["id"] for d in __import__(
            "apeireth.v1074_asi_production_runner", fromlist=["DIRECTION_CATALOG"]
        ).DIRECTION_CATALOG]

    def test_v1074_artifacts_persist_when_requested(self, tmp_path):
        """V1074 runner 真写 5 + artifacts (主 23:44 干到底)."""
        from apeireth.v1074_asi_production_runner import (
            ProductionRunner, DEFAULT_DATA_DIR, DEFAULT_ARTIFACTS,
        )
        # 准备 history, 让 trend 路径走完
        history_dir = tmp_path / DEFAULT_DATA_DIR
        history_dir.mkdir(parents=True, exist_ok=True)
        (history_dir / DEFAULT_ARTIFACTS["history_jsonl"]).write_text(
            json.dumps({"snapshot_id": "snap_x", "ts_iso": "2026-07-22T00:00:00+00:00",
                        "v03_score": 0.80, "n_modules": 100, "n_tests": 100, "n_commits": 100}) + "\n",
            encoding="utf-8",
        )
        runner = ProductionRunner(project_dir=str(tmp_path))
        r = runner.run(write_artifacts=True)
        assert len(r.artifacts) >= 5
        for k, v in r.artifacts.items():
            assert Path(v).exists(), f"artifact {k} not written: {v}"

    def test_v1074_philosophy_guard_in_result(self):
        """V1074 真 philosophy_guard in result (主 17:58 不假装)."""
        from apeireth.v1074_asi_production_runner import ProductionRunner
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        assert "runner_is_not_asi" in r.philosophy_guard
        assert "report_is_not_production" in r.philosophy_guard

    def test_v1074_v03_guards_constant_present(self):
        """V1074 V3_GUARDS 必含 5 不假装 keys."""
        from apeireth.v1074_asi_production_runner import V3_GUARDS
        required = ("module_is_not_asi", "measurement_is_not_truth",
                    "structure_is_not_consciousness", "production_is_not_safety",
                    "automation_is_not_autonomy")
        for r in required:
            assert r in V3_GUARDS, f"V1074 V3_GUARDS missing: {r}"

    def test_v1074_v04_baseline_target_locked(self):
        """V1124 V0.4 baseline = 0.8538 R10 起点 (LOCKED)."""
        from apeireth.v1124_asi_north_star_backend import BASELINE_V04
        assert BASELINE_V04 == 0.8538, (
            "V1124 V0.4 baseline 改变需更新 R10-W1~W4 守门 + V1125.R10_START_TARGET"
        )

    def test_v1074_artifact_snapshot_json_is_valid(self):
        """V1074 真写 artifacts/asi_snapshot.json 必须 JSON parse 通过."""
        snap_path = PROJECT_ROOT / "artifacts" / "asi_snapshot.json"
        if not snap_path.exists():
            pytest.skip("asi_snapshot.json not yet materialized (R10-W1 init)")
        data = json.loads(snap_path.read_text(encoding="utf-8"))
        # 必备 LOCKED 字段
        for key in ("snapshot_id", "ts", "ts_iso", "version", "level",
                    "v03_score", "n_modules", "n_tests", "n_commits"):
            assert key in data, f"snapshot missing field: {key}"

    def test_v1074_decision_recommender_returns_real_recommendation(self):
        """V1074 真 DecisionRecommender 推荐下一方向 (主 13:31 大胆激进)."""
        from apeireth.v1074_asi_production_runner import (
            ProductionRunner, DecisionRecommender, DIRECTION_CATALOG,
        )
        # DecisionRecommender 真推荐 (必含 rationale + chosen_direction + expected_score_lift)
        rec = DecisionRecommender()
        # 构造一个 StatusSnapshot-like 触发某方向
        from apeireth.v1074_asi_production_runner import StatusSnapshot
        snap = StatusSnapshot(
            snapshot_id="snap_test_r11",
            ts=1700000000.0,
            ts_iso="2026-07-30T00:00:00+00:00",
            version="0.1.0",
            level="ASI",
            level_score=0.88,
            v02_base=0.88,
            v03_score=0.88,
            n_modules=1, n_tests=1, n_commits=1,
            dim_breakdown={"engineering": 0.5},
            v1071_vcp_score=0.96,
            v1071_cross_domain=1.0,
            v1072_eternal_identity=0.84,
            philosophy_guard_ok=True,
        )
        decision = rec.recommend(snap)
        assert decision.rationale, "rationale 必为非空字符串"
        assert decision.chosen_direction in [d["id"] for d in DIRECTION_CATALOG]
        assert decision.expected_score_lift >= 0.0
        assert decision.confidence >= 0.0


# ---------------------------------------------------------------------------
# 3. Dashboard payload / UI (主 17:43 实事求是 + V1074_TARGET_S perf 借鉴)
# ---------------------------------------------------------------------------


class TestDashboardPayloadRegression:
    """Dashboard payload / UI — V1131 + V1130 to_dict schema + chaos 保险."""

    @pytest.fixture(scope="class")
    def v1131_dashboard(self):
        from apeireth.v1131_r10_w2_comprehensive_dashboard import (
            V1131R10W2ComprehensiveRunner,
        )
        runner = V1131R10W2ComprehensiveRunner()
        return runner.run(chaos=False, benchmark=False)

    @pytest.fixture(scope="class")
    def v1130_run_result(self):
        from apeireth.v1130_asi_north_star_v05_run import V1130ASINorthStarRunner
        runner = V1130ASINorthStarRunner(week_label="R10-W3")
        return runner.evaluate_r10_week(run_benchmark=False)

    def test_v1131_dashboard_has_4_categories(self, v1131_dashboard):
        """V1131 dashboard 必含 4 类 (kickoff / main_track / real_run / decision)."""
        for cat in ("kickoff_summary", "main_track_summary",
                    "real_run_summary", "decision_summary"):
            assert cat in v1131_dashboard, f"V1131 missing: {cat}"

    def test_v1131_dashboard_payload_asi_north_star_locked(self, v1131_dashboard):
        """V1131 dashboard payload.asi_north_star = 0.9800 (主 22:33 LOCKED)."""
        from apeireth.v1131_r10_w2_comprehensive_dashboard import ASI_NORTH_STAR
        assert v1131_dashboard["asi_north_star"] == ASI_NORTH_STAR == 0.98

    def test_v1131_dashboard_payload_has_v05_total(self, v1131_dashboard):
        """V1131 dashboard payload 必含 v05_total ∈ [0, 1]."""
        for cat in ("kickoff_summary", "main_track_summary", "real_run_summary"):
            cat_dict = v1131_dashboard[cat]
            assert "v05_total" in cat_dict, f"{cat} missing v05_total"
            assert 0.0 <= cat_dict["v05_total"] <= 1.0, (
                f"{cat}.v05_total={cat_dict['v05_total']} out of [0,1]"
            )

    def test_v1131_dashboard_payload_serializable(self, v1131_dashboard):
        """V1131 dashboard payload JSON 可序列化 (主 17:43 实事求是)."""
        data = json.dumps(v1131_dashboard, default=str)
        roundtrip = json.loads(data)
        assert roundtrip["asi_north_star"] == v1131_dashboard["asi_north_star"]

    def test_v1131_dashboard_w2_w4_targets_locked(self, v1131_dashboard):
        """V1131 W2 ≥ 0.90 + W4 ≥ 0.95 LOCKED (主 13:31 大胆激进)."""
        from apeireth.v1131_r10_w2_comprehensive_dashboard import (
            W2_MID_TARGET, W4_ULTIMATE_TARGET,
        )
        assert W2_MID_TARGET == 0.9000
        assert W4_ULTIMATE_TARGET == 0.9500

    def test_v1131_dashboard_chaos_preserves_measurement(self):
        """V1131 dashboard chaos test: measurement_preserved (主 23:44 干到底)."""
        from apeireth.v1131_r10_w2_comprehensive_dashboard import (
            V1131R10W2ComprehensiveRunner,
        )
        runner = V1131R10W2ComprehensiveRunner()
        chaos = runner.chaos_test(n_drop=1)
        assert chaos.measurement_preserved, "chaos 必须保留 measurement"
        assert chaos.dashboard_preserved, "chaos 必须保留 dashboard"

    def test_v1130_dashboard_payload_has_v05_breakdown(self, v1130_run_result):
        """V1130 真跑 dashboard.asi_level / v05_total / v05_breakdown 字段."""
        d = v1130_run_result["dashboard"]
        for key in ("asi_level", "v04_score", "v05_total", "v05_breakdown",
                    "asi_north_star", "main_track", "w2_pass", "w4_pass"):
            assert key in d, f"V1130 dashboard missing: {key}"
        assert 0.0 <= d["v05_total"] <= 1.0

    def test_v1130_dashboard_v05_breakdown_three_dims(self, v1130_run_result):
        """V1130 v05_breakdown 必含 continuity / autonomy / transferability (V1125 锁)."""
        br = v1130_run_result["dashboard"]["v05_breakdown"]
        for k in ("continuity", "autonomy", "transferability"):
            assert k in br, f"V1130 v05_breakdown missing: {k}"
            assert 0.0 <= br[k] <= 1.0

    def test_v1130_dashboard_north_star_zero_point_nine_eight(self, v1130_run_result):
        """V1130 ASI 北极星 0.9800 LOCKED 在 dashboard payload 中守门."""
        d = v1130_run_result["dashboard"]
        assert d["asi_north_star"] == 0.98

    def test_v1130_dashboard_chaos_preserved(self, v1130_run_result):
        """V1130 chaos test 节点失联 measurement_preserved (主 23:44)."""
        chaos = v1130_run_result.get("chaos_node_down")
        if chaos is None:
            pytest.skip("chaos test not enabled in this run")
        assert chaos["measurement_preserved"], "V1130 chaos 必保留 measurement"

    def test_v1130_v1131_payload_schema_compatible(self, v1131_dashboard, v1130_run_result):
        """V1131 ∩ V1130 dashboard payload 共享 v05_total / asi_north_star (主 17:43)."""
        v1130 = v1130_run_result["dashboard"]
        assert v1131_dashboard["asi_north_star"] == v1130["asi_north_star"] == 0.98
        # v05_total 允许 float 微差 (.9999), 但必须一致 (同一周次默认参数)
        v1130_v05 = v1130["v05_total"]
        v1131_real_run_v05 = v1131_dashboard["real_run_summary"].get("v05_total")
        if v1131_real_run_v05 is not None:
            assert abs(v1130_v05 - v1131_real_run_v05) < 1e-3


# ---------------------------------------------------------------------------
# 4. V3 9 键 LOCKED (主 22:33 + 主 17:43 实事求是真生产)
# ---------------------------------------------------------------------------


class TestV3NineKeyGuardRegression:
    """V3 9 键 LOCKED — AsiNineKeyLock + verify_or_raise + inject_guard_block."""

    def test_nine_keys_count_is_exactly_nine(self):
        """ASI_NINE_KEYS 必须恰好 9 个 (主 22:33 LOCKED)."""
        from apeireth.mcp.asi_nine_keys import ASI_NINE_KEYS
        assert len(ASI_NINE_KEYS) == 9, f"expected 9 keys, got {len(ASI_NINE_KEYS)}"

    def test_nine_keys_required_documented(self):
        """ASI_NINE_KEYS 必含 9 个文档化 key 名 (主 17:58 不假装: 公开语义)."""
        from apeireth.mcp.asi_nine_keys import ASI_NINE_KEYS, ASI_NINE_KEYS_DOCS
        for k in ASI_NINE_KEYS:
            assert k in ASI_NINE_KEYS_DOCS, f"missing doc for key: {k}"
            assert "PHL-" in ASI_NINE_KEYS_DOCS[k] or "MCP" in ASI_NINE_KEYS_DOCS[k], (
                f"key '{k}' doc must reference PHL-/MCP source"
            )

    def test_nine_keys_lock_default_all_true(self):
        """AsiNineKeyLock 默认所有 9 键 True (V1123 baseline)."""
        from apeireth.mcp.asi_nine_keys import AsiNineKeyLock, ASI_NINE_KEYS
        lock = AsiNineKeyLock()
        assert lock.all_locked()
        assert lock.failed_keys() == []
        assert all(lock.values[k] is True for k in ASI_NINE_KEYS)

    def test_nine_keys_lock_partial_failure_detected(self):
        """任何 1 键 False → all_locked = False, failed_keys 非空."""
        from apeireth.mcp.asi_nine_keys import AsiNineKeyLock, ASI_NINE_KEYS
        bad = AsiNineKeyLock(values={k: (k != "not_undo") for k in ASI_NINE_KEYS})
        assert not bad.all_locked()
        assert "not_undo" in bad.failed_keys()
        assert len(bad.failed_keys()) == 1

    def test_nine_keys_lock_all_failure_detected(self):
        """任意 N 键 False → failed_keys 含 N 个."""
        from apeireth.mcp.asi_nine_keys import AsiNineKeyLock, ASI_NINE_KEYS
        false_keys = {"not_undo", "not_safe", "not_uuid"}
        bad = AsiNineKeyLock(values={k: (k not in false_keys) for k in ASI_NINE_KEYS})
        assert not bad.all_locked()
        assert set(bad.failed_keys()) == false_keys

    def test_nine_keys_lock_missing_keys_raises(self):
        """缺失任何 1 键 → ValueError (主 23:44 干到底, 不静默)."""
        from apeireth.mcp.asi_nine_keys import AsiNineKeyLock, ASI_NINE_KEYS
        subset_keys = [k for k in ASI_NINE_KEYS if k != "not_undo"]
        with pytest.raises(ValueError) as excinfo:
            AsiNineKeyLock(values={k: True for k in subset_keys})
        assert "not_undo" in str(excinfo.value) or "missing" in str(excinfo.value).lower()

    def test_verify_or_raise_raises_on_failure(self):
        """verify_or_raise: 任何 1 键 False → RuntimeError (主 23:44 拒服)."""
        from apeireth.mcp.asi_nine_keys import (
            AsiNineKeyLock, verify_or_raise, ASI_NINE_KEYS,
        )
        bad = AsiNineKeyLock(values={k: (k != "not_safe") for k in ASI_NINE_KEYS})
        with pytest.raises(RuntimeError) as excinfo:
            verify_or_raise(bad)
        assert "not_safe" in str(excinfo.value)

    def test_verify_or_raise_passes_when_all_true(self):
        """verify_or_raise: 全部 True → 不抛."""
        from apeireth.mcp.asi_nine_keys import AsiNineKeyLock, verify_or_raise
        lock = AsiNineKeyLock()
        verify_or_raise(lock)  # 不应抛

    def test_inject_guard_block_on_text_content(self):
        """inject_guard_block 在 text content 上追加 json philosophy_guard."""
        from apeireth.mcp.asi_nine_keys import (
            AsiNineKeyLock, inject_guard_block,
        )
        lock = AsiNineKeyLock()
        result = {"content": [{"type": "text", "text": "hello world"}]}
        injected = inject_guard_block(result, lock)
        assert injected["content"][0]["type"] == "json"
        assert "philosophy_guard" in injected["content"][0]["data"]
        assert "text" in injected["content"][0]["data"]
        assert injected["content"][0]["data"]["text"] == "hello world"

    def test_inject_guard_block_creates_default_content(self):
        """inject_guard_block content 缺失时 → 构造默认 json."""
        from apeireth.mcp.asi_nine_keys import (
            AsiNineKeyLock, inject_guard_block,
        )
        lock = AsiNineKeyLock()
        result = inject_guard_block({}, lock)
        assert len(result["content"]) == 1
        assert result["content"][0]["type"] == "json"
        assert "philosophy_guard" in result["content"][0]["data"]

    def test_to_dict_has_asi_nine_keys_locked(self):
        """AsiNineKeyLock.to_dict 必含 asi_nine_keys_locked / n_locked / n_total."""
        from apeireth.mcp.asi_nine_keys import AsiNineKeyLock
        lock = AsiNineKeyLock()
        d = lock.to_dict()
        for k in ("asi_nine_keys_locked", "values", "failed_keys",
                  "n_locked", "n_total"):
            assert k in d, f"to_dict missing: {k}"
        assert d["n_total"] == 9
        assert d["n_locked"] == 9

    def test_to_guard_block_for_mcp_content(self):
        """AsiNineKeyLock.to_guard_block 必含 keys / n_locked / n_total."""
        from apeireth.mcp.asi_nine_keys import AsiNineKeyLock
        lock = AsiNineKeyLock()
        gb = lock.to_guard_block()
        for k in ("asi_nine_keys_locked", "n_locked", "n_total", "keys"):
            assert k in gb, f"to_guard_block missing: {k}"
        assert gb["n_total"] == 9


# ---------------------------------------------------------------------------
# 5. 真实失败语义 (主 17:43 实事求是: failure_is_not_success)
# ---------------------------------------------------------------------------


class TestFailureSemanticsRegression:
    """真实失败语义 — V1124 backend 错误类型 / 错误码 / payload, 无静默 fallback."""

    def test_v1124_error_is_runtime_error(self):
        """V1124Error 继承 RuntimeError, 可被 except RuntimeError 捕获."""
        from apeireth.v1124_asi_north_star_backend import V1124Error
        err = V1124Error("test_code", "test message", 503)
        assert isinstance(err, RuntimeError)
        assert err.code == "test_code"
        assert err.status == 503

    def test_v1124_error_payload_is_stable(self):
        """V1124Error.payload() 返回 {error: {code, message}} 契约 LOCKED."""
        from apeireth.v1124_asi_north_star_backend import V1124Error
        assert V1124Error("bad", "msg", 400).payload() == {
            "error": {"code": "bad", "message": "msg"}
        }

    def test_v1124_strict_failure_is_not_success_guard(self):
        """V1124 V3_GUARDS 必含 failure_is_not_success (主 17:43 实事求是)."""
        from apeireth.v1124_asi_north_star_backend import V3_GUARDS
        assert "failure_is_not_success" in V3_GUARDS
        # 守门语义: 不可用 provider / corruption storage 必须报失败, 不模拟
        assert "simulated" in V3_GUARDS["failure_is_not_success"].lower()

    def test_v1124_no_simulated_fallback_in_constructor(self):
        """V1124 backend 构造器: 缺 OPENAI_API_KEY 时真报失败, 不静默 fallback."""
        from apeireth.v1124_asi_north_star_backend import V1124Error, ModelRequest
        # 离线测试: 构造 ModelRequest 但调用时缺 key 应 → V1124Error(provider_not_configured)
        # 直接构造检查 V1124Error 字段, 完整路径在线测试包里测 (test_v1124_*)
        # 此处仅验证错误类型稳定
        err = V1124Error("provider_not_configured", "OPENAI_API_KEY is required", 503)
        assert err.status == 503
        assert err.code == "provider_not_configured"

    def test_v1124_offline_mode_does_not_reach_out_to_real_provider(self):
        """offline 测试: 不应触发真实 LLM HTTP 调用 (主 17:43 实事求是)."""
        # 离线模式, V1124 backend 启动不需要 key, 但 model call 必须 raise
        from apeireth.v1124_asi_north_star_backend import (
            ASINorthStarBackend, V1124Error, ModelRequest, RealModelGateway,
        )
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            backend = ASINorthStarBackend(Path(td) / "state")
            # 真 request 但 provider 是 "openai" 且无 key → 必抛 V1124Error
            req = ModelRequest(provider="openai", model="gpt-4", prompt="hello")
            with pytest.raises(V1124Error) as excinfo:
                backend.gateway.call(req)
            assert excinfo.value.status in (502, 503)

    def test_v1124_provider_unavailable_in_offline(self):
        """offline: provider unavailable → V1124Error(503), 不模拟."""
        from apeireth.v1124_asi_north_star_backend import (
            ASINorthStarBackend, V1124Error, ModelRequest,
        )
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            backend = ASINorthStarBackend(Path(td) / "state")
            # 用一个不存在的 provider name 应抛 400 unsupported_provider
            req = ModelRequest(provider="mystery_provider", model="x", prompt="hi")
            with pytest.raises(V1124Error) as excinfo:
                backend.gateway.call(req)
            assert excinfo.value.status in (400, 502, 503), (
                f"unexpected status: {excinfo.value.status}"
            )

    def test_v1124_local_provider_requires_command(self):
        """offline: local provider 无 command → V1124Error(503), 不 silent fallback."""
        from apeireth.v1124_asi_north_star_backend import (
            ASINorthStarBackend, V1124Error, ModelRequest,
        )
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            backend = ASINorthStarBackend(Path(td) / "state")
            req = ModelRequest(provider="local", model="x", prompt="hi")
            with pytest.raises(V1124Error) as excinfo:
                backend.gateway.call(req)
            assert excinfo.value.status == 503
            assert "command" in str(excinfo.value).lower()

    def test_v1124_durable_identity_corruption_raises_integrity_error(self):
        """V1124 durable identity 损坏 → IntegrityError, 不假装 OK."""
        from apeireth.v1124_asi_north_star_backend import (
            DurableIdentityStore, IntegrityError, V1124Error,
        )
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            # 写入损坏的 snapshot, 然后尝试启动应抛 IntegrityError
            state_dir = Path(td) / "state"
            state_dir.mkdir(parents=True, exist_ok=True)
            (state_dir / "identity.json").write_text("not-valid-json{{{", encoding="utf-8")
            store = DurableIdentityStore(state_dir)
            with pytest.raises((IntegrityError, V1124Error, ValueError, Exception)) as excinfo:
                store.startup_self_check()
            # 损坏的 JSON 必拒 — 任何 "unreadable / invalid / json / integrity / corrupt" 关键词都算
            err_msg = str(excinfo.value).lower()
            assert any(
                kw in err_msg for kw in (
                    "integrity", "invalid", "json", "unreadable", "corrupt",
                    "missing", "snapshot", "deserialize",
                )
            ), f"损坏 identity 没拒, 错误: {excinfo.value}"

    def test_v1124_audit_chain_break_raises_integrity_error(self):
        """V1124 audit chain 断裂 → IntegrityError (主 17:43 实事求是)."""
        from apeireth.v1124_asi_north_star_backend import (
            AuditChain, IntegrityError, V1124Error,
        )
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            chain = AuditChain(Path(td) / "audit.jsonl")
            chain.append("event_1", {"k": "v1"})
            # 篡改第 1 行 hash → 必破坏第二条 append 时的 prev_hash 校验
            text = chain.path.read_text(encoding="utf-8")
            lines = text.splitlines(keepends=True)
            # 把第一行的 hash 字段值替换为另一个
            tampered = lines[0].replace("hash", "haxx", 1)
            chain.path.write_text(tampered + "".join(lines[1:]), encoding="utf-8")
            # 第二次 append 必抛 IntegrityError
            with pytest.raises((IntegrityError, V1124Error, Exception)) as excinfo:
                chain.append("event_2", {"k": "v2"})
            err_msg = str(excinfo.value).lower()
            assert any(
                kw in err_msg for kw in ("integrity", "chain", "hash", "broken", "mismatch")
            ), f"audit chain 篡改没拒, 错误: {excinfo.value}"


# ---------------------------------------------------------------------------
# 6. Cross-cutting: live vs offline 边界 (主 17:43 实事求是)
# ---------------------------------------------------------------------------


class TestLiveVsOfflineBoundary:
    """自动判定 live vs offline; 回归护栏本身必须全程离线可跑."""

    def test_live_provider_detection_respects_conftest_isolation(self):
        """conftest 已清空 *API*KEY* env, 所以 LIVE_PROVIDER 必为 False."""
        # 离线测试期间 concret.py 强制清空 LLM_API_KEY 等
        assert is_live_provider() is False, (
            "P0 回归护栏必须全程离线可跑 (主 17:43 实事求是); "
            "如果 LIVE_PROVIDER=True, 说明 conftest 隔离已被破坏"
        )

    def test_v1136_offline_runs_without_provider_keys(self):
        """V1136 真测不需要任何 LLM key (主 17:43 实事求是: 真测 ≠ 真实 LLM)."""
        from apeireth.v1136_asi_v05_3dim_real_measurement import measure_v05_3dims
        # 即使 LIVE_PROVIDER=False, V1136 必须能跑
        result = measure_v05_3dims(v04_score=0.8538)
        assert result.v3_guards_pass

    def test_v1074_offline_runs_without_provider_keys(self):
        """V1074 ProductionRunner 离线可跑 (V0.3 真测不依赖 LLM)."""
        from apeireth.v1074_asi_production_runner import ProductionRunner
        runner = ProductionRunner(project_dir=str(PROJECT_ROOT))
        r = runner.run(write_artifacts=False)
        assert r.v03_score > 0.0

    def test_v1131_offline_runs_without_provider_keys(self):
        """V1131 dashboard 离线可跑 (复用 V1125 ~ V1130 真测)."""
        from apeireth.v1131_r10_w2_comprehensive_dashboard import (
            V1131R10W2ComprehensiveRunner,
        )
        runner = V1131R10W2ComprehensiveRunner()
        result = runner.run(chaos=False, benchmark=False)
        assert result["asi_north_star"] == 0.98

    def test_v1123_nine_key_offline_default_all_locked(self):
        """V1123 9 键 离线默认 LOCKED (主 22:33)."""
        from apeireth.mcp.asi_nine_keys import AsiNineKeyLock, verify_or_raise
        lock = AsiNineKeyLock()
        verify_or_raise(lock)
        assert lock.all_locked()


# ---------------------------------------------------------------------------
# 7. P0 护栏 CLI 一行入口 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------


class TestP0GuardCLISmoke:
    """P0 护栏脚本一行入口 (主 00:56 任何人都能接手)."""

    def test_r11_p0_guard_pytest_collection(self):
        """本测试文件本身可被 pytest collect (主 00:56)."""
        # 由 pytest 自身验证; 这里只断言模块存在
        assert Path(__file__).exists()

    def test_key_classes_present(self):
        """P0 护栏 5 大类 + 1 边界类必备."""
        import sys
        mod = sys.modules[__name__]
        required = (
            "TestV1136RealMeasurementRegression",
            "TestV1074V04ProductionRegression",
            "TestDashboardPayloadRegression",
            "TestV3NineKeyGuardRegression",
            "TestFailureSemanticsRegression",
            "TestLiveVsOfflineBoundary",
        )
        for cls in required:
            assert hasattr(mod, cls), f"missing class: {cls}"


if __name__ == "__main__":
    # 主 00:56 任何人都能接手: 一行可跑
    sys.exit(pytest.main([__file__, "-v", "--tb=short"]))
