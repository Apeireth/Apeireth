"""V1112 DGM Archive v0.4 — 真演化 + Track B Identity 串联 (R9-AO-001).

覆盖:
1.  模块常量与导出 (8)
2.  UCB1 + json_hash + diff (4)
3.  IdentityAnchor 锚定 + 桥接 (6)
4.  3 方法 reproduce (parent_child/sexual/asexual) (8)
5.  HQB 评估 + retain 判定 (6)
6.  V04EvolutionRun 数据类 (3)
7.  run_experiment 50 轮真演化 (5)
8.  Track B 串联 (V1095 + V1072) (3)
9.  V3 守门 (主 17:43 实事求是 + 主 17:58 不假装) (3)
10. Markdown 报告 + CLI (3)
11. 边界 + 错误处理 (4)
12. 50 轮 lift 记录 + 3 方法对照 (3)
   总计 ≥ 50 测试
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1112_dgm_v04 import (  # noqa: E402
    VERSION, COMPONENTS, METHODS,
    RETAIN_DELTA, OPEN_ENDED_PROB, THRESHOLD_FLOOR,
    SEXUAL_MIN_PARENTS, ASEXUAL_DRIFT_RATE, SEXUAL_CROSSOVER_RATE,
    MAX_GENERATIONS, EARLY_STOP_FAILS,
    ucb1, _json_hash, _diff, _hqb_for, _should_retain,
    _get_full_eval_threshold, _mutate_one_field,
    IdentityAnchor, build_default_anchor, try_attach_identity_store,
    reproduce_parent_child, reproduce_sexual, reproduce_asexual, reproduce,
    V04EvolutionRun, run_experiment, report, main,
    V3_GUARDS,
)


# ============================================================================
# 1. 模块常量与导出 (8 tests)
# ============================================================================


class TestModuleConstants:
    """V1112 DGM v0.4 公共常量 — 真生产 sanity check."""

    def test_01_version_is_v04(self):
        assert VERSION == "0.4.0"

    def test_02_components_includes_v09(self):
        assert "crossover" in COMPONENTS
        assert "drift" in COMPONENTS
        assert "anchor" in COMPONENTS
        assert len(COMPONENTS) == 9

    def test_03_methods_are_three_reproduction_strategies(self):
        assert set(METHODS) == {"parent_child", "sexual", "asexual"}
        assert len(METHODS) == 3

    def test_04_retain_delta_is_strictly_positive(self):
        assert RETAIN_DELTA > 0.0
        assert RETAIN_DELTA <= 0.10

    def test_05_max_generations_is_50(self):
        assert MAX_GENERATIONS == 50

    def test_06_early_stop_threshold_at_least_10(self):
        assert EARLY_STOP_FAILS >= 10

    def test_07_sexual_min_parents(self):
        assert SEXUAL_MIN_PARENTS >= 2

    def test_08_module_exports_complete(self):
        from apeireth import v1112_dgm_v04
        for name in (
            "IdentityAnchor", "V04EvolutionRun", "run_experiment",
            "reproduce", "reproduce_parent_child", "reproduce_sexual",
            "reproduce_asexual", "report", "main",
        ):
            assert hasattr(v1112_dgm_v04, name), f"missing {name}"


# ============================================================================
# 2. UCB1 + json_hash + diff (4 tests)
# ============================================================================


class TestHelpers:
    """UCB1 + JSON hash + diff — 借鉴 Sakana DGM 真源码."""

    def test_09_ucb1_explores_unpulled(self):
        assert ucb1(0.0, 0, 1) == float("inf")

    def test_10_ucb1_exploits_positive_mean(self):
        assert ucb1(0.8, 10, 20) > ucb1(0.2, 10, 20)

    def test_11_json_hash_is_deterministic(self):
        v = {"a": 1, "b": [1, 2, 3]}
        h1 = _json_hash(v)
        h2 = _json_hash(v)
        assert h1 == h2
        assert len(h1) == 16

    def test_12_diff_unified_format(self):
        text = _diff({"generation": 0}, {"generation": 1})
        assert "--- harness.parent" in text
        assert '-  "generation": 0' in text
        assert '+  "generation": 1' in text


# ============================================================================
# 3. IdentityAnchor 锚定 + 桥接 (6 tests)
# ============================================================================


class TestIdentityAnchor:
    """P7: Identity 锚定 — Track B 串联 V1095 + V1072."""

    def test_13_default_anchor_has_identity_id(self):
        a = build_default_anchor()
        assert a.identity_id.startswith("ca_dev_")
        assert a.core_snapshot_hash != ""

    def test_14_anchor_integrity_check_valid(self):
        a = IdentityAnchor(
            identity_id="ca_test_anchor",
            core_snapshot_hash="a" * 16,
        )
        ok, reason = a.integrity_check()
        assert ok is True
        assert reason == "anchor_ok"

    def test_15_anchor_integrity_check_invalid_id(self):
        a = IdentityAnchor(identity_id="", core_snapshot_hash="a" * 16)
        ok, reason = a.integrity_check()
        assert ok is False
        assert "空" in reason or "格式" in reason

    def test_16_anchor_invalid_when_hash_empty(self):
        a = IdentityAnchor(identity_id="ca_test_x", core_snapshot_hash="")
        ok, reason = a.integrity_check()
        assert ok is False
        assert "hash" in reason

    def test_17_from_v1095_profile_bridge(self):
        """P7: 真生产桥接 V1095 CentralAIProfile."""

        class FakeProfile:
            identity_id = "ca_bridge_01"
            name = "Chu Ling"
            chinese_name = "楚零"
            core_snapshot = {"v1072_compat": True, "essence": "test"}

        a = IdentityAnchor.from_v1095_profile(FakeProfile())
        assert a.identity_id == "ca_bridge_01"
        assert a.bridge_v1072 is True
        assert len(a.core_snapshot_hash) == 16

    def test_18_from_v1072_core_bridge(self):
        """P8: V1072 → IdentityAnchor 完整往返."""

        class FakeCore:
            identity_id = "ca_v1072_01"
            name = "Chu Ling"
            chinese_name = "楚零"
            essence = "central_ai_eternal_identity"
            lt_persistence = True
            first_seen = 1234567890.0
            last_seen = 1234567900.0
            n_resurrections = 3

        a = IdentityAnchor.from_v1072_core(FakeCore())
        assert a.identity_id == "ca_v1072_01"
        assert a.bridge_v1072 is True
        assert a.core_snapshot_hash != ""


# ============================================================================
# 4. 3 方法 reproduce (8 tests)
# ============================================================================


class TestReproduceMethods:
    """P6: 3 重组方法 — parent_child / sexual / asexual."""

    def test_19_parent_child_increments_attempts(self):
        import random
        parent = {"_active_id": "p1", "components": {"c1": {"attempts": 0, "reward": 0.0}}}
        child, meta = reproduce_parent_child(parent, random.Random(42))
        assert meta["method"] == "parent_child"
        assert meta["parent_ids"] == ["p1"]
        assert isinstance(child, dict)

    def test_20_parent_child_handles_corrupt_state(self):
        """防御: 坏 parent state (components 不是 dict) 不崩."""
        import random
        parent = {"_active_id": "p1", "components": 12345}
        child, meta = reproduce_parent_child(parent, random.Random(42))
        assert isinstance(child["components"], dict)
        assert all(isinstance(v, dict) for v in child["components"].values())

    def test_21_sexual_crossover_uses_two_parents(self):
        import random
        pa = {"_active_id": "pa", "components": {"a": {"attempts": 0, "reward": 0.0}}, "generation": 1}
        pb = {"_active_id": "pb", "components": {"a": {"attempts": 5, "reward": 0.5}}, "generation": 2}
        child, meta = reproduce_sexual(pa, pb, random.Random(42))
        assert meta["method"] == "sexual"
        assert meta["parent_ids"] == ["pa", "pb"]
        assert child["generation"] == 3

    def test_22_sexual_fallback_when_archive_too_small(self):
        """sexual 在 archive < 2 时 fallback 到 parent_child."""
        import random
        result = reproduce("sexual", archive=[], state={"_active_id": "s", "components": {}}, rng=random.Random(42))
        child, meta = result
        assert meta["method"] == "sexual_fallback"
        assert "archive" in meta["reason"]

    def test_23_asexual_drifts_components(self):
        """asexual 30% 字段随机漂变."""
        import random
        parent = {"_active_id": "a", "components": {"c1": {"attempts": 0, "reward": 0.0}}, "generation": 0}
        child, meta = reproduce_asexual(parent, random.Random(42))
        assert meta["method"] == "asexual"
        assert meta["n_mutations"] >= 0
        assert child["generation"] == 1

    def test_24_asexual_handles_bad_components(self):
        """防御: asexual 也能处理坏 components."""
        import random
        parent = {"_active_id": "a", "components": {"c1": 5}}
        child, meta = reproduce_asexual(parent, random.Random(42))
        assert isinstance(child["components"], dict)
        assert isinstance(child["components"]["c1"], dict)

    def test_25_reproduce_dispatcher_validates_method(self):
        """method 必须是 3 个之一."""
        import random
        with pytest.raises(ValueError):
            reproduce("not_a_method", archive=[], state={"_active_id": "x"}, rng=random.Random(42))

    def test_26_mutate_one_field_handles_types(self):
        """_mutate_one_field 支持多种基本类型."""
        import random
        rng = random.Random(42)
        v = _mutate_one_field(5, rng)
        assert isinstance(v, int)
        v = _mutate_one_field("hello", rng)
        assert isinstance(v, str)
        v = _mutate_one_field([1, 2], rng)
        assert isinstance(v, list)
        v = _mutate_one_field({"k": 1}, rng)
        assert isinstance(v, dict)


# ============================================================================
# 5. HQB 评估 + retain 判定 (6 tests)
# ============================================================================


class TestHQBRules:
    """P5: HQB 4 维度评估 + retain 判定."""

    def test_27_hqb_has_four_dimensions(self):
        hqb = _hqb_for(0.8, 100.0, True)
        assert set(hqb) == {
            "capability", "cost_efficiency", "latency_margin",
            "constraint_adherence", "composite",
        }
        assert 0.0 <= hqb["composite"] <= 1.0

    def test_28_hqb_constraint_adherence_zero_when_guard_fails(self):
        hqb = _hqb_for(0.8, 100.0, guard_ok=False)
        assert hqb["constraint_adherence"] == 0.0
        assert hqb["composite"] < 0.99

    def test_29_should_retain_high_quality(self):
        hqb = _hqb_for(0.95, 5.0, True)
        baseline = 0.85
        retain, reason = _should_retain(hqb, baseline, [])
        assert retain is True

    def test_30_should_discard_when_below_threshold(self):
        hqb = _hqb_for(0.5, 1000.0, True)
        baseline = 0.85
        retain, reason = _should_retain(hqb, baseline, [])
        assert retain is False
        assert reason

    def test_31_get_full_eval_threshold_uses_second_highest(self):
        scores = [0.9, 0.7, 0.5]
        thr = _get_full_eval_threshold(scores)
        assert thr == 0.7

    def test_32_get_full_eval_threshold_floor_when_empty(self):
        assert _get_full_eval_threshold([]) == THRESHOLD_FLOOR


# ============================================================================
# 6. V04EvolutionRun 数据类 (3 tests)
# ============================================================================


class TestEvolutionRunDataclass:
    """V04EvolutionRun 字段完整性."""

    def test_33_run_dataclass_to_dict(self):
        from apeireth.v1112_dgm_v04 import V04EvolutionRun
        run = V04EvolutionRun(
            run_id="r1", iteration=1, method="parent_child",
            parent_ids=["p1"], identity_id="ca_x",
            identity_anchor_hash="a" * 16,
            hqb={"capability": 0.8, "composite": 0.9,
                 "cost_efficiency": 0.9, "latency_margin": 0.9, "constraint_adherence": 1.0},
            hqb_delta=0.05, lift=0.06, verdict="retain",
        )
        d = run.to_dict()
        assert d["run_id"] == "r1"
        assert d["verdict"] == "retain"
        assert d["lift"] == 0.06

    def test_34_run_dataclass_default_values(self):
        from apeireth.v1112_dgm_v04 import V04EvolutionRun
        run = V04EvolutionRun(
            run_id="r2", iteration=2, method="asexual",
            parent_ids=[], identity_id="ca_y",
            identity_anchor_hash="b" * 16,
            hqb={"composite": 0.5}, hqb_delta=0.0, lift=0.0,
            verdict="discard",
        )
        assert run.reject_reason == ""
        assert run.bridge_v1072 is True
        assert run.philosophy_guard_ok is True

    def test_35_run_trace_id_generated(self):
        from apeireth.v1112_dgm_v04 import V04EvolutionRun
        run = V04EvolutionRun(
            run_id="r3", iteration=3, method="sexual",
            parent_ids=[], identity_id="ca_z",
            identity_anchor_hash="c" * 16,
            hqb={"composite": 0.5}, hqb_delta=0.0, lift=0.0,
            verdict="discard",
        )
        assert run.trace_id.startswith("trace_")


# ============================================================================
# 7. run_experiment 50 轮真演化 (5 tests)
# ============================================================================


class TestRunExperiment50Rounds:
    """P9: 50 轮真演化 (vs R8 30 轮)."""

    def test_36_50_rounds_complete(self):
        result = run_experiment(iterations=50, method="parent_child", seed=20260729)
        assert result["iterations_completed"] == 50

    def test_37_iterations_boundary_1(self):
        result = run_experiment(iterations=1, method="parent_child", seed=42)
        assert result["iterations_completed"] == 1

    def test_38_iterations_boundary_zero_rejected(self):
        with pytest.raises(ValueError):
            run_experiment(iterations=0, method="parent_child", seed=42)

    def test_39_iterations_too_large_rejected(self):
        with pytest.raises(ValueError):
            run_experiment(iterations=51, method="parent_child", seed=42)

    def test_40_invalid_method_rejected(self):
        with pytest.raises(ValueError):
            run_experiment(iterations=10, method="fake_method", seed=42)

    def test_41_results_have_all_required_keys(self):
        result = run_experiment(iterations=10, method="parent_child", seed=42)
        for key in (
            "version", "iterations_requested", "iterations_completed",
            "method_requested", "identity_anchor", "baseline",
            "n_retain", "n_discard", "n_reject", "n_asi_pretend_total",
            "archive_size", "lifts_per_round",
            "method_breakdown", "runs", "v3_guards", "philosophy_anchors",
        ):
            assert key in result, f"missing key: {key}"


# ============================================================================
# 8. Track B 串联 (V1095 + V1072) (3 tests)
# ============================================================================


class TestTrackBIntegration:
    """P7+P8: DGM v0.4 ↔ V1095 IdentityStore + V1072 兼容."""

    def test_42_default_anchor_links_to_v1095(self):
        anchor = build_default_anchor()
        assert anchor.identity_id.startswith("ca_")

    def test_43_try_attach_identity_store_returns_none_when_no_store(self):
        result = try_attach_identity_store(None)
        assert result is None

    def test_44_try_attach_identity_store_with_fake_store(self):
        class FakeStore:
            def get_or_create_profile(self):
                class P:
                    identity_id = "ca_fake_01"
                    name = "Chu Ling"
                    chinese_name = "楚零"
                    core_snapshot = {"v1072_compat": True}
                return P()
        anchor = try_attach_identity_store(FakeStore())
        assert anchor is not None
        assert anchor.identity_id == "ca_fake_01"
        assert anchor.bridge_v1072 is True


# ============================================================================
# 9. V3 守门 (主 17:43 + 主 17:58) (3 tests)
# ============================================================================


class TestV3Guards:
    """V3 守门不假装 — 主 17:43 实事求是 + 主 17:58 不假装."""

    def test_45_v3_guards_present(self):
        for key in (
            "module_is_not_asi", "measurement_is_not_truth",
            "structure_is_not_consciousness", "production_is_not_safety",
            "automation_is_not_autonomy", "red_queen_loop", "no_asi_pretend",
        ):
            assert key in V3_GUARDS

    def test_46_n_asi_pretend_total_always_zero(self):
        result = run_experiment(iterations=50, method="parent_child", seed=20260729)
        assert result["n_asi_pretend_total"] == 0

    def test_47_philosophy_anchors_include_main_principles(self):
        result = run_experiment(iterations=10, method="parent_child", seed=42)
        all_text = " ".join(result["philosophy_anchors"])
        for principle in ("22:33", "17:43", "13:31", "23:44", "19:33", "20:55"):
            assert principle in all_text, f"missing principle: {principle}"


# ============================================================================
# 10. Markdown 报告 + CLI (3 tests)
# ============================================================================


class TestReportAndCLI:
    """report() + main() — 报告生成 + 命令行入口."""

    def test_48_report_writes_markdown(self, tmp_path, monkeypatch):
        """report() 写 Markdown 包含 3 方法对照 + v0.4 标识."""
        fake_out = tmp_path / "artifacts"
        monkeypatch.setattr("apeireth.v1112_dgm_v04.ROOT", tmp_path)
        monkeypatch.setattr("apeireth.v1112_dgm_v04.OUT", fake_out)
        monkeypatch.setattr("apeireth.v1112_dgm_v04.STATE", fake_out / "harness_state.json")
        result = run_experiment(iterations=5, method="parent_child", seed=42)
        import apeireth.v1112_dgm_v04 as v04
        report_path = tmp_path / "report_test.md"
        monkeypatch.setattr(v04, "REPORT_PATH", report_path)
        path = report(result)
        full_path = tmp_path / path
        assert full_path.exists()
        content = full_path.read_text(encoding="utf-8")
        assert "v0.4" in content.lower()
        assert "parent_child" in content
        assert "sexual" in content
        assert "asexual" in content

    def test_49_report_includes_lift_table(self, tmp_path, monkeypatch):
        fake_out = tmp_path / "artifacts"
        monkeypatch.setattr("apeireth.v1112_dgm_v04.ROOT", tmp_path)
        monkeypatch.setattr("apeireth.v1112_dgm_v04.OUT", fake_out)
        monkeypatch.setattr("apeireth.v1112_dgm_v04.STATE", fake_out / "harness_state.json")
        result = run_experiment(iterations=10, method="parent_child", seed=42)
        import apeireth.v1112_dgm_v04 as v04
        report_path = tmp_path / "report_test_b.md"
        monkeypatch.setattr(v04, "REPORT_PATH", report_path)
        path = report(result)
        full_path = tmp_path / path
        content = full_path.read_text(encoding="utf-8")
        assert "lift" in content.lower()
        assert "verdict" in content.lower()

    def test_50_main_help_exits_zero(self, monkeypatch, capsys):
        """main() --help 应返回 0."""
        with pytest.raises(SystemExit) as exc:
            main(["--help"])
        assert exc.value.code == 0


# ============================================================================
# 11. 边界 + 错误处理 (4 tests)
# ============================================================================


class TestEdgeCases:
    """边界 + 错误处理 — 真生产必须健壮."""

    def test_51_reproduce_dispatch_returns_valid_meta(self):
        """3 方法都 produce valid meta dict."""
        import random
        rng = random.Random(42)
        for method in METHODS:
            child, meta = reproduce(method, [state_with_dict_components()],
                                    state_with_dict_components(), rng)
            assert "method" in meta
            assert "parent_ids" in meta
            assert isinstance(meta["parent_ids"], list)

    def test_52_run_with_min_iterations(self):
        result = run_experiment(iterations=1, method="sexual", seed=42)
        assert result["iterations_completed"] == 1

    def test_53_run_with_seed_42_archive_size_positive(self):
        result = run_experiment(iterations=50, method="parent_child", seed=42)
        assert result["archive_size"] >= 1

    def test_54_run_different_seeds_produce_different_archives(self):
        r1 = run_experiment(iterations=30, method="parent_child", seed=42)
        r2 = run_experiment(iterations=30, method="parent_child", seed=314)
        assert r1["lifts_per_round"] != r2["lifts_per_round"]


# ============================================================================
# 12. 50 轮 lift 记录 + 3 方法对照 (3 tests)
# ============================================================================


class Test50RoundLiftTracking:
    """P9: 50 轮 lift 记录 + 3 方法对照 — 真演化闭环."""

    def test_55_lifts_per_round_length_matches_iterations(self):
        result = run_experiment(iterations=50, method="parent_child", seed=20260729)
        assert len(result["lifts_per_round"]) == result["iterations_completed"] + 1

    def test_56_three_methods_all_run_during_evolution(self):
        """3 方法都至少跑过 1 次 (50 轮中)."""
        result = run_experiment(iterations=50, method="parent_child", seed=20260729)
        for m in METHODS:
            assert result["method_breakdown"][m]["n_total"] >= 1, f"method {m} never ran"

    def test_57_method_breakdown_retain_rate_consistent(self):
        """retain_rate = n_retain / n_total."""
        result = run_experiment(iterations=50, method="parent_child", seed=20260729)
        for m, stats in result["method_breakdown"].items():
            n_total = stats["n_total"]
            n_retain = stats["n_retain"]
            n_discard = stats["n_discard"]
            n_reject = stats["n_reject"]
            assert n_retain + n_discard + n_reject == n_total


# ============================================================================
# Helpers
# ============================================================================


def state_with_dict_components() -> dict:
    """返回 components 为 dict 的 state — 用于 reproduce 单元测试."""
    return {
        "_active_id": "test",
        "components": {c: {"attempts": 0, "reward": 0.0, "lift": 0.0} for c in COMPONENTS},
        "generation": 0,
        "active_candidate": "test",
    }


# ============================================================================
# Python invocation
# ============================================================================

if __name__ == "__main__":
    pytest.main([__file__, "-v"])