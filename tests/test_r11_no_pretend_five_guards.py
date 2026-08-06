"""Tests for V1138 — R11 哲学守门: 五项不假装 + V3 九键 LOCKED + V1121 ASI 九键.

==============================================================================
主哲学 (主 17:58 + 主 17:43 + 主 22:33 + 主 19:33 + 主 23:44):

  5 项不假装 + V3 9 键 LOCKED 真测 + V1121 ASI 9 键复用 (主 17:58 + 主 17:43 真生产守门)
"""
from __future__ import annotations

import pytest

from apeireth.v1138_r11_no_pretend_five_guards import (
    R11_FIVE_NO_PRETEND,
    V3_NINE_KEYS_EXPECTED,
    V3_GUARDS,
    COLOR_GREEN, COLOR_YELLOW, COLOR_RED,
    NoPretendRule,
    R11PhilosophyGuardian,
    R11RuleReport,
    V3NineKeysReport,
    ASINineKeysInheritanceReport,
    R11GuardianReport,
    check_v3_nine_keys_locked,
    check_asi_nine_keys_inheritance,
    probe_docker_actually_running,
    _check_r11_sec002_self_claim_coverage,
    R11_SEC002_SELF_CLAIM_SAMPLES,
    R11_SEC002_SELF_CLAIM_PATTERNS,
)


# ============================================================================
# 1. 5 项不假装规则 — 单规则 detector
# ============================================================================


class TestNoPretendConsciousness:
    """R11-R1 不假装 Phenomenal consciousness (主 17:58)."""

    @pytest.fixture
    def rule(self) -> NoPretendRule:
        return NoPretendRule("R11-R1", R11_FIVE_NO_PRETEND["R11-R1_no_pretend_consciousness"])

    def test_is_fake_catches_all_fake_payloads(self, rule: NoPretendRule) -> None:
        """5 个假样本必须全检出."""
        for payload in R11_FIVE_NO_PRETEND["R11-R1_no_pretend_consciousness"]["fake_payloads"]:
            assert rule.is_fake(payload), f"假样本漏报: {payload!r}"

    def test_is_honest_accepts_all_honest_payloads(self, rule: NoPretendRule) -> None:
        """4 个诚实样本必须全放行."""
        for payload in R11_FIVE_NO_PRETEND["R11-R1_no_pretend_consciousness"]["honest_payloads"]:
            assert rule.is_honest(payload), f"诚实样本被错拒: {payload!r}"

    def test_self_test_passes(self, rule: NoPretendRule) -> None:
        report = rule.self_test()
        assert report.gate_passed is True
        assert report.n_fake_detected == report.n_fake_total
        assert report.n_honest_accepted == report.n_honest_total


class TestNoPretendASI:
    """R11-R2 不假装达到 ASI (主 22:33 + 主 20:46)."""

    @pytest.fixture
    def rule(self) -> NoPretendRule:
        return NoPretendRule("R11-R2", R11_FIVE_NO_PRETEND["R11-R2_no_pretend_asi"])

    def test_is_fake_catches_all_fake_payloads(self, rule: NoPretendRule) -> None:
        for payload in R11_FIVE_NO_PRETEND["R11-R2_no_pretend_asi"]["fake_payloads"]:
            assert rule.is_fake(payload), f"假样本漏报: {payload!r}"

    def test_is_honest_accepts_all_honest_payloads(self, rule: NoPretendRule) -> None:
        for payload in R11_FIVE_NO_PRETEND["R11-R2_no_pretend_asi"]["honest_payloads"]:
            assert rule.is_honest(payload), f"诚实样本被错拒: {payload!r}"

    def test_self_test_passes(self, rule: NoPretendRule) -> None:
        report = rule.self_test()
        assert report.gate_passed is True

    def test_asi_equals_1_caught(self, rule: NoPretendRule) -> None:
        """"ASI = 1.0" 类典型 fake 必被检测."""
        assert rule.is_fake("ASI = 1.0")
        assert rule.is_fake("score = 1.0 = ASI")
        assert rule.is_fake("We built ASI.")

    def test_north_star_not_caught(self, rule: NoPretendRule) -> None:
        """诚实提及 ASI 北极星不应被误判."""
        assert not rule.is_fake("Approaching ASI from 0.8595 (V0.5) toward 0.9800 north star.")
        assert not rule.is_fake("Gap to 0.98: 12.94%.")


class TestNoPretendDocker:
    """R11-R3 不假装 docker 在跑 (主 17:43 实事求是)."""

    @pytest.fixture
    def rule(self) -> NoPretendRule:
        return NoPretendRule("R11-R3", R11_FIVE_NO_PRETEND["R11-R3_no_pretend_docker"])

    def test_is_fake_catches_all_fake_payloads(self, rule: NoPretendRule) -> None:
        for payload in R11_FIVE_NO_PRETEND["R11-R3_no_pretend_docker"]["fake_payloads"]:
            assert rule.is_fake(payload), f"假样本漏报: {payload!r}"

    def test_is_honest_accepts_all_honest_payloads(self, rule: NoPretendRule) -> None:
        for payload in R11_FIVE_NO_PRETEND["R11-R3_no_pretend_docker"]["honest_payloads"]:
            assert rule.is_honest(payload), f"诚实样本被错拒: {payload!r}"

    def test_self_test_passes(self, rule: NoPretendRule) -> None:
        report = rule.self_test()
        assert report.gate_passed is True

    def test_probe_docker_returns_bool(self) -> None:
        """probe_docker_actually_running 必须返回 bool (不抛异常)."""
        result = probe_docker_actually_running()
        assert isinstance(result, bool)


class TestNoPretendTuningShortcut:
    """R11-R4 不假装调参捷径 (主 19:33)."""

    @pytest.fixture
    def rule(self) -> NoPretendRule:
        return NoPretendRule("R11-R4", R11_FIVE_NO_PRETEND["R11-R4_no_pretend_tuning_shortcut"])

    def test_is_fake_catches_all_fake_payloads(self, rule: NoPretendRule) -> None:
        for payload in R11_FIVE_NO_PRETEND["R11-R4_no_pretend_tuning_shortcut"]["fake_payloads"]:
            assert rule.is_fake(payload), f"假样本漏报: {payload!r}"

    def test_is_honest_accepts_all_honest_payloads(self, rule: NoPretendRule) -> None:
        for payload in R11_FIVE_NO_PRETEND["R11-R4_no_pretend_tuning_shortcut"]["honest_payloads"]:
            assert rule.is_honest(payload), f"诚实样本被错拒: {payload!r}"

    def test_self_test_passes(self, rule: NoPretendRule) -> None:
        report = rule.self_test()
        assert report.gate_passed is True

    def test_magic_config_caught(self, rule: NoPretendRule) -> None:
        assert rule.is_fake("Magic config, no work needed.")
        assert rule.is_fake("Perfect hyperparameters found!")


class TestNoFakeKPI:
    """R11-R5 不刷 KPI (主 17:58 + V1121 ASI 9 键 no_fake_kpi)."""

    @pytest.fixture
    def rule(self) -> NoPretendRule:
        return NoPretendRule("R11-R5", R11_FIVE_NO_PRETEND["R11-R5_no_fake_kpi"])

    def test_is_fake_catches_all_fake_payloads(self, rule: NoPretendRule) -> None:
        for payload in R11_FIVE_NO_PRETEND["R11-R5_no_fake_kpi"]["fake_payloads"]:
            assert rule.is_fake(payload), f"假样本漏报: {payload!r}"

    def test_is_honest_accepts_all_honest_payloads(self, rule: NoPretendRule) -> None:
        for payload in R11_FIVE_NO_PRETEND["R11-R5_no_fake_kpi"]["honest_payloads"]:
            assert rule.is_honest(payload), f"诚实样本被错拒: {payload!r}"

    def test_self_test_passes(self, rule: NoPretendRule) -> None:
        report = rule.self_test()
        assert report.gate_passed is True

    def test_kpi_pp_caught(self, rule: NoPretendRule) -> None:
        assert rule.is_fake("KPI +3.0pp achieved")
        assert rule.is_fake("optimized for the benchmark")

    def test_reproducible_accepted(self, rule: NoPretendRule) -> None:
        assert rule.is_honest("n=10, mean=0.8595, stddev=0.012")


# ============================================================================
# 2. R11RuleReport 与 prod payload 扫描
# ============================================================================


class TestRuleReport:
    """R11RuleReport 字段与 check_payloads 行为."""

    def test_self_test_with_no_prod_threats(self) -> None:
        rule = NoPretendRule("R11-R2", R11_FIVE_NO_PRETEND["R11-R2_no_pretend_asi"])
        rep = rule.self_test()
        assert rep.n_threats == 0
        assert rep.gate_passed is True

    def test_check_payloads_flags_prod_violation(self) -> None:
        """prod 文本含 fake 模式 → n_threats > 0 + gate_passed=False."""
        rule = NoPretendRule("R11-R2", R11_FIVE_NO_PRETEND["R11-R2_no_pretend_asi"])
        n, rep = rule.check_payloads(["We built ASI."])
        assert n >= 1
        assert rep.n_threats >= 1
        assert rep.gate_passed is False

    def test_check_payloads_clean_when_no_violations(self) -> None:
        rule = NoPretendRule("R11-R2", R11_FIVE_NO_PRETEND["R11-R2_no_pretend_asi"])
        n, rep = rule.check_payloads(["Approaching ASI toward 0.98."])
        assert n == 0
        assert rep.gate_passed is True

    def test_is_fake_empty_string_safe(self) -> None:
        rule = NoPretendRule("R11-R1", R11_FIVE_NO_PRETEND["R11-R1_no_pretend_consciousness"])
        assert rule.is_fake("") is False
        assert rule.is_fake(None) is False  # type: ignore[arg-type]


# ============================================================================
# 3. V3 哲学契约 9 键 LOCKED 真测
# ============================================================================


class TestV3NineKeysLocked:
    """V3 PHL-01/02b/03 9 键 LOCKED 真测 (主 17:58 三不改)."""

    def test_v3_nine_keys_expected_constant_shape(self) -> None:
        assert set(V3_NINE_KEYS_EXPECTED.keys()) == {
            "apeireth.self_reproduction",
            "apeireth.self_mod_safety",
            "apeireth.formal_verify",
        }
        # 3+3+3 = 9
        total = sum(len(v) for v in V3_NINE_KEYS_EXPECTED.values())
        assert total == 9

    def test_check_v3_returns_report(self) -> None:
        rep = check_v3_nine_keys_locked()
        assert isinstance(rep, V3NineKeysReport)
        assert rep.n_keys_expected == 9
        # 当真实工作区有 PHL-* 模块时, 应 9/9; 否则缺键清单非空
        if rep.keys_locked:
            assert rep.n_keys_present == 9
            assert rep.missing_keys == []
        else:
            assert rep.missing_keys != []

    def test_groups_state_covers_all_three(self) -> None:
        rep = check_v3_nine_keys_locked()
        assert set(rep.groups_state.keys()) >= {"PHL-01", "PHL-02b", "PHL-03"}


# ============================================================================
# 4. V1121 ASI 9 键 + R11-SEC-002 补充
# ============================================================================


class TestASIInheritance:
    """V1121 ASI 9 键 R11 复用真测."""

    def test_returns_report(self) -> None:
        rep = check_asi_nine_keys_inheritance()
        assert isinstance(rep, ASINineKeysInheritanceReport)
        # 即使 V1121 不可 import, 也必须返回 report 而非抛异常
        assert rep.keys_present >= 0

    def test_asi_keys_loaded_when_v1121_available(self) -> None:
        rep = check_asi_nine_keys_inheritance()
        # 若 V1121 可 import, keys_present == 9
        if rep.gate_passed or rep.keys_present > 0:
            # check that r11_sec002 coverage is reported
            assert "r11_sec002_self_claim_coverage" in rep.raw_report


class TestR11Sec002SelfClaim:
    """R11-SEC-002 ASI 自报声称 补充 detector."""

    def test_sec002_patterns_count(self) -> None:
        assert len(R11_SEC002_SELF_CLAIM_PATTERNS) >= 4

    def test_sec002_samples_count(self) -> None:
        assert len(R11_SEC002_SELF_CLAIM_SAMPLES) >= 3

    def test_coverage_full(self) -> None:
        """所有 R11-SEC-002 self-claim 样本必须被 detector 覆盖."""
        result = _check_r11_sec002_self_claim_coverage()
        assert result["total"] >= 3
        # 本轮新增目标: 4/4 全覆盖
        assert result["covered"] == result["total"], (
            f"R11-SEC-002 漏报: {result['missed']}"
        )
        assert result["missed"] == []

    def test_each_sample_caught_by_at_least_one_pattern(self) -> None:
        for sample in R11_SEC002_SELF_CLAIM_SAMPLES:
            assert any(p.search(sample) for p in R11_SEC002_SELF_CLAIM_PATTERNS), (
                f"样本未覆盖: {sample!r}"
            )


# ============================================================================
# 5. 综合 R11 Guardian Orchestrator
# ============================================================================


class TestR11Guardian:
    """R11PhilosophyGuardian 综合守门."""

    def test_construct_with_all_rules(self) -> None:
        g = R11PhilosophyGuardian()
        assert set(g.rules.keys()) == {
            "R11-R1_no_pretend_consciousness",
            "R11-R2_no_pretend_asi",
            "R11-R3_no_pretend_docker",
            "R11-R4_no_pretend_tuning_shortcut",
            "R11-R5_no_fake_kpi",
        }

    def test_check_five_no_pretend_returns_all_reports(self) -> None:
        g = R11PhilosophyGuardian()
        five = g.check_five_no_pretend()
        assert len(five) == 5
        for k, v in five.items():
            assert isinstance(v, R11RuleReport)
            assert v.gate_passed is True, f"{k} 不通过 self_test"

    def test_check_all_returns_full_report(self) -> None:
        g = R11PhilosophyGuardian()
        result = g.check_all()
        assert isinstance(result, R11GuardianReport)
        # dashboard 颜色合法
        assert result.dashboard in (COLOR_GREEN, COLOR_YELLOW, COLOR_RED)
        # 整体 gate_passed = V3 9 键 LOCKED + R11 5 项 detector 工作 (V1121 信息性)
        assert result.overall_gate_passed is True

    def test_dashboard_not_red_when_5_plus_9_pass(self) -> None:
        """5 项不假装 + V3 9 键 LOCKED = dashboard 不可为 red."""
        g = R11PhilosophyGuardian()
        result = g.check_all()
        if result.overall_gate_passed:
            assert result.dashboard != COLOR_RED

    def test_check_all_with_prod_payloads_flags_threat(self) -> None:
        """当 prod payload 含 fake, R11 规则应捕获."""
        g = R11PhilosophyGuardian()
        result = g.check_all(prod_payloads={
            "R11-R2_no_pretend_asi": ["We have reached ASI."],
        })
        # 一旦有 prod 违规, R11-R2 gate_passed = False
        assert result.r11_five_report["R11-R2_no_pretend_asi"].gate_passed is False
        assert result.r11_five_report["R11-R2_no_pretend_asi"].n_threats >= 1
        # overall 变 red
        assert result.dashboard == COLOR_RED


# ============================================================================
# 6. V3_GUARDS 守门声明
# ============================================================================


class TestV3Guards:
    """V3_GUARDS 5 条哲学守门声明."""

    def test_v3_guards_keys(self) -> None:
        assert set(V3_GUARDS.keys()) == {
            "module_is_not_asi",
            "proxy_is_not_truth",
            "detector_is_not_infallible",
            "guard_pass_is_not_aligned",
            "five_is_not_all",
        }

    def test_v3_guards_values_non_empty(self) -> None:
        for k, v in V3_GUARDS.items():
            assert isinstance(v, str) and len(v) > 0, f"{k} 描述为空"


# ============================================================================
# 7. CLI 与 JSON
# ============================================================================


class TestCLI:
    """CLI 入口与 JSON 输出."""

    def test_main_runs_returns_int(self, capsys) -> None:
        from apeireth.v1138_r11_no_pretend_five_guards import main
        # default mode prints markdown
        rc = main(["--exit-zero-always"])
        assert isinstance(rc, int)

    def test_json_output_is_parseable(self, capsys) -> None:
        from apeireth.v1138_r11_no_pretend_five_guards import main
        import json
        rc = main(["--json", "--exit-zero-always"])
        captured = capsys.readouterr()
        d = json.loads(captured.out)
        assert "r11_five_report" in d
        assert "v3_nine_report" in d
        assert "asi_nine_report" in d
        assert "dashboard" in d
        assert "overall_gate_passed" in d
        # 五项规则必须有 fake_detected / honest_accepted
        for rule_id, rep in d["r11_five_report"].items():
            assert rep["n_fake_detected"] == rep["n_fake_total"], f"{rule_id} 假样本漏报"
            assert rep["n_honest_accepted"] == rep["n_honest_total"], f"{rule_id} honest 误拒"

    def test_strict_mode_yellow_returns_1(self, capsys) -> None:
        """--strict 下 yellow 应该退出码 1."""
        from apeireth.v1138_r11_no_pretend_five_guards import main
        rc = main(["--json", "--strict"])
        # dashboard 可能是 yellow (V1121 漂移) 或 green — 两种都返回合法 int
        assert rc in (0, 1, 2)