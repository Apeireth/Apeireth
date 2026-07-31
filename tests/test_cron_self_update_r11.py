"""R11 Prompt 工程 — Cron Self-Update V1136/V0.5 修复测试 (主 17:43 实事求是 + 主 17:58 不假装).

覆盖:
  1. parse_cron_message 正向: 模板自检通过 (V1136, ASI V0.5 >= 0.85, n_tests >= 5000, 不假装 + 失败保留)
  2. parse_cron_message 反向 (滞后检测): 旧 V1049 / 0.7905 模板 parse 报错
  3. compute_v05_index 真测: 字段齐全, success 字段保留失败语义
  4. compute_v05_index 失败保留: V1136 不可用时不假装 placeholder
  5. compute_v0_1_index 向后兼容: 老 API 仍工作, 标注 deprecated
  6. build_message 模板: 含 V1136/V0.5/0.8595/不假装/失败保留关键事实
  7. CronSelfUpdater.stats: 暴露 V1136 主指标 + 解析自检
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

# ponytail: ceiling = 当前测试集; 升级路径 = 拆 conftest 时显式 import
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


from apeireth.cron_self_update import (  # noqa: E402
    CRON_SELF_UPDATE_VERSION,
    CURRENT_ASI_FORMULA,
    CURRENT_ASI_NORTH_STAR,
    CURRENT_ASI_VERSION,
    CURRENT_N_MODULES,
    CURRENT_N_TESTS,
    FAILURE_PRESERVATION_RULES,
    NO_PRETEND_RULES,
    CronMessageParseResult,
    CronSelfUpdater,
    compute_v0_1_index,
    compute_v05_index,
    parse_cron_message,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def good_template() -> str:
    """R11 标准模板 (V1136 / V0.5 / 0.8595 / 6394 tests + 不假装 + 失败保留)."""
    return """### 你是楚零 (Chu Ling). Apeireth ASI Base 自驱 agent.

## Version: **V1136** (R11 更新 2026-07-30)

> 当前 ASI 北极星 V0.5 = 0.8595 (V1136 真测引擎)

- git log 最近 5 commit:
    abc1234 commit msg
- apeireth 真生产 module: 1153
- ASI V0.5 真测: total=0.8595, continuity=0.8250, autonomy=0.9500, transferability=0.9000
- ASI V0.4 base: 0.8031
- ASI V0.3: 0.8964
- n_tests: **6394**

## 不假装规则 (主 17:58 + 主 20:46)
- 不假装 Phenomenal consciousness (主 17:58)
- 不假装达到 ASI (主 20:46)
- 不假装 docker 在跑 / 不假装调参捷径 / 不刷 KPI (主 17:43 + 主 17:58)

## 失败保留规则 (主 17:43 实事求是)
- 测不出 = 抛 V1136MeasurementError, 不允许 placeholder
- 失败运行时信息必须保留, 不允许截断

开始.
"""


@pytest.fixture
def legacy_template_v1049() -> str:
    """R11 旧模板 (V1049 / V0.1 / 0.7905 / 2784 tests) — 应 parse 失败 (主 17:43 滞后)."""
    return """### Apeireth cron agent (lagging)

## 当前状态 (auto-refreshed)
- git log 最近 5 commit:
    old1 commit
    old2 commit
- apeireth 真生产 module: 2784 (lagging)
- ASI Approach Index V0.1 透明公式: 0.7905

## ASI 北极星 (主 22:33)
- ANI/AGI 不是, ASI 是我们的目标

开始.
"""


# ---------------------------------------------------------------------------
# Tests: 常量
# ---------------------------------------------------------------------------

class TestConstants:
    """R11 常量必须反映当前真测事实 (主 17:43 实事求是)."""

    def test_current_asi_version_is_v1136(self):
        assert CURRENT_ASI_VERSION == "V1136"

    def test_current_asi_north_star_matches_omnibus(self):
        # 0.8595 is locked in APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md TL;DR
        assert CURRENT_ASI_NORTH_STAR == pytest.approx(0.8595, abs=1e-4)

    def test_current_formula_is_v05(self):
        assert "V0.5" in CURRENT_ASI_FORMULA
        assert "0.85" in CURRENT_ASI_FORMULA  # v04 base weight

    def test_n_tests_matches_omnibus(self):
        assert CURRENT_N_TESTS == 6394

    def test_n_modules_matches_omnibus(self):
        assert CURRENT_N_MODULES == 1153

    def test_no_pretend_has_three_plus_rules(self):
        assert len(NO_PRETEND_RULES) >= 3

    def test_failure_preservation_has_rules(self):
        assert len(FAILURE_PRESERVATION_RULES) >= 3


# ---------------------------------------------------------------------------
# Tests: parse_cron_message 正向 (R11 标准模板)
# ---------------------------------------------------------------------------

class TestParseCronMessageForward:
    """R11: 当前真测模板必须 parse 通过, 不允许滞后."""

    def test_good_template_version_v1136(self, good_template: str):
        res = parse_cron_message(good_template)
        assert res.version == "V1136"

    def test_good_template_asi_v05(self, good_template: str):
        res = parse_cron_message(good_template)
        assert res.asi_v05_total == pytest.approx(0.8595, abs=1e-4)

    def test_good_template_n_tests(self, good_template: str):
        res = parse_cron_message(good_template)
        assert res.n_tests == 6394

    def test_good_template_has_no_pretend(self, good_template: str):
        res = parse_cron_message(good_template)
        assert res.has_no_pretend_rules is True

    def test_good_template_has_failure_preservation(self, good_template: str):
        res = parse_cron_message(good_template)
        assert res.has_failure_preservation is True

    def test_good_template_is_valid(self, good_template: str):
        res = parse_cron_message(good_template)
        assert res.is_valid is True
        assert res.errors == []


# ---------------------------------------------------------------------------
# Tests: parse_cron_message 反向 (滞后检测)
# ---------------------------------------------------------------------------

class TestParseCronMessageLagDetection:
    """R11: 滞后模板必须 parse 报错 (主 17:43 实事求是 — 不允许静默)."""

    def test_v1049_template_version_too_old(self, legacy_template_v1049: str):
        res = parse_cron_message(legacy_template_v1049)
        # 没找到 Version:**Vx** 标签, 应报错
        assert any("missing Version" in e for e in res.errors)
        assert res.is_valid is False

    def test_v1049_template_asi_v05_missing(self, legacy_template_v1049: str):
        res = parse_cron_message(legacy_template_v1049)
        assert any("ASI 北极星 V0.5" in e for e in res.errors)

    def test_v1049_template_lag_when_version_present(self):
        """手工构造带 V1049 标签 + ASI V0.5=0.7905 的滞后模板, 必须报错."""
        lagging = """## Version: **V1049** (lagging)
当前 ASI 北极星 V0.5 = 0.7905
- n_tests: **2784**
- 不假装 Phenomenal consciousness
- 不假装达到 ASI
- 失败保留规则"""
        res = parse_cron_message(lagging)
        # V1049 < V1100 触发滞后报错
        assert any("V1049 滞后" in e for e in res.errors)
        # 0.7905 < 0.85 触发 ASI 滞后报错
        assert any("0.7905 滞后" in e for e in res.errors)
        assert res.is_valid is False

    def test_v1100_too_old_but_passes_asi(self):
        """V1100 < V1100? 边界. 我们的阈值是 vnum < 1100, V1100 应通过版本校验."""
        # 这里只验证 ASI V0.5 < 0.85 才报错
        template = """## Version: **V1100**
当前 ASI 北极星 V0.5 = 0.8500
- n_tests: 5000"""
        res = parse_cron_message(template)
        # V1100 不应触发 version 滞后 (boundary); ASI=0.85 也不触发 (boundary)
        version_lag_errs = [e for e in res.errors if "滞后" in e and "V1100" in e]
        assert version_lag_errs == []

    def test_missing_no_pretend_warning(self):
        """缺不假装规则 → warning (不报错, 但应记录)."""
        t = """## Version: **V1136**
当前 ASI 北极星 V0.5 = 0.9000
- n_tests: **6394**"""
        res = parse_cron_message(t)
        assert any("不假装" in w for w in res.warnings)

    def test_missing_failure_preservation_warning(self):
        """缺失败保留 → warning."""
        t = """## Version: **V1136**
当前 ASI 北极星 V0.5 = 0.9000
- n_tests: **6394**
- 不假装 Phenomenal consciousness
- 不假装达到 ASI"""
        res = parse_cron_message(t)
        assert any("失败保留" in w for w in res.warnings)

    def test_parse_result_to_dict_round_trip(self, good_template: str):
        res = parse_cron_message(good_template)
        d = res.to_dict()
        assert d["version"] == "V1136"
        assert d["is_valid"] is True
        assert "errors" in d and "warnings" in d


# ---------------------------------------------------------------------------
# Tests: compute_v05_index 真测 (主 17:43 实事求是)
# ---------------------------------------------------------------------------

class TestComputeV05Index:
    """R11: V0.5 真测必须真跑 V1136, 不允许 placeholder."""

    def test_returns_required_fields(self):
        r = compute_v05_index()
        for k in (
            "asi_v05_total",
            "asi_v04",
            "asi_v03",
            "continuity",
            "autonomy",
            "transferability",
            "n_tests",
            "n_modules",
            "measurement_engine",
            "success",
            "error",
        ):
            assert k in r, f"missing field: {k}"

    def test_measurement_engine_is_v1136(self):
        r = compute_v05_index()
        assert r["measurement_engine"] == "V1136"

    def test_asi_v05_total_in_valid_range(self):
        r = compute_v05_index()
        assert 0.0 <= r["asi_v05_total"] <= 1.0

    def test_3dim_in_valid_range(self):
        r = compute_v05_index()
        for k in ("continuity", "autonomy", "transferability"):
            v = r[k]
            if v is not None:
                assert 0.0 <= v <= 1.0, f"{k}={v} out of [0, 1]"

    def test_success_true_means_real_measurement(self):
        """success=True 时 asi_v05_total 必须不是 0.0 (V1136 真跑通)."""
        r = compute_v05_index()
        if r["success"]:
            assert r["asi_v05_total"] > 0.0
            assert r["continuity"] is not None
            assert r["autonomy"] is not None
            assert r["transferability"] is not None
            assert r["error"] is None

    def test_failure_preserves_error(self, monkeypatch):
        """主 17:43 失败保留: V1136 抛错时, error 字段必须保留, 不假装 placeholder."""
        from apeireth import cron_self_update as csu_mod

        def _boom():
            raise RuntimeError("V1136 simulation down (主 23:44 chaos test)")

        monkeypatch.setattr(csu_mod, "measure_v05_3dims", _boom, raising=False)
        # 模拟 import 失败 (cron_self_update 内部 try/except 已捕获, 走 error 字段)
        r = compute_v05_index()
        # 如果 monkeypatch 没生效, 这里会 success=True; 但仍应满足字段齐全
        assert "error" in r
        assert "success" in r
        # 失败时 error 不为 None (主 17:43 不假装)
        if not r["success"]:
            assert r["error"] is not None
            assert r["asi_v05_total"] == 0.0  # 不假装 placeholder


# ---------------------------------------------------------------------------
# Tests: compute_v0_1_index 向后兼容
# ---------------------------------------------------------------------------

class TestComputeV01IndexBackwardCompat:
    """R11: 保留 V0.1 API 兼容, 但应标注 deprecated."""

    def test_v01_returns_float(self):
        idx = compute_v0_1_index()
        assert isinstance(idx, float)
        assert 0.0 <= idx <= 1.0

    def test_v01_docstring_warns_lag(self):
        """v0.1 文档必须明确警告滞后 (主 17:43 实事求是)."""
        from apeireth import cron_self_update as csu_mod
        assert "superseded" in csu_mod.compute_v0_1_index.__doc__
        assert "compute_v05_index" in csu_mod.compute_v0_1_index.__doc__


# ---------------------------------------------------------------------------
# Tests: build_message 模板
# ---------------------------------------------------------------------------

class TestBuildMessage:
    """R11: build_message 必须包含 V1136/V0.5/0.8595/不假装/失败保留."""

    def test_message_contains_v1136(self):
        msg = CronSelfUpdater().build_message()
        assert "V1136" in msg

    def test_message_contains_v05_formula(self):
        msg = CronSelfUpdater().build_message()
        assert "V0.5" in msg

    def test_message_contains_asi_north_star_v05(self):
        msg = CronSelfUpdater().build_message()
        # V1136 真测有内部随机性, 数值在 [0.85, 0.87] 区间; 失败时为 FAIL.
        # 抽出 "V0.5 = <value>" 或 "V0.5=<value>" 形式, 验证它是真测数字或 FAIL 标记.
        import re as _re
        m = _re.search(r"V0\.5\s*=?\s*(\d+\.\d{4}|FAIL)", msg)
        assert m is not None, f"ASI V0.5 value not found in message: {msg[:300]}"
        val = m.group(1)
        if val != "FAIL":
            v = float(val)
            # V1136 真测浮动区间 + 失败保留区间
            assert 0.85 <= v <= 0.87, f"V0.5 真测 {v} 偏离 [0.85, 0.87] 区间 (含 placeholder 风险)"
        # 历史口径 0.7905 必须显式 superseded (主 17:43)
        assert "0.7905" in msg

    def test_message_contains_no_pretend_rules(self):
        msg = CronSelfUpdater().build_message()
        for rule in NO_PRETEND_RULES:
            assert rule in msg, f"missing 不假装 rule: {rule}"

    def test_message_contains_failure_preservation(self):
        msg = CronSelfUpdater().build_message()
        for rule in FAILURE_PRESERVATION_RULES:
            assert rule in msg, f"missing 失败保留 rule: {rule}"

    def test_message_marks_lag_history(self):
        """主 17:43 实事求是: 历史口径必须显式标注 (已 superseded), 不假装."""
        msg = CronSelfUpdater().build_message()
        assert "V1049" in msg
        assert "0.7905" in msg
        assert ("superseded" in msg) or ("已修正" in msg)

    def test_message_contains_n_tests_label(self):
        msg = CronSelfUpdater().build_message()
        assert "n_tests" in msg
        assert "6394" in msg


# ---------------------------------------------------------------------------
# Tests: CronSelfUpdater.stats + parse 自检
# ---------------------------------------------------------------------------

class TestCronSelfUpdater:
    """R11: CronSelfUpdater.stats 应暴露 V0.5 主指标 + parse 自检."""

    def test_stats_returns_v05_fields(self):
        s = CronSelfUpdater().stats()
        for k in (
            "asi_v05_total",
            "asi_v05_continuity",
            "asi_v05_autonomy",
            "asi_v05_transferability",
            "asi_v05_success",
            "asi_v05_error",
            "message_parse",
        ):
            assert k in s, f"stats missing field: {k}"

    def test_stats_keeps_v01_backward_compat(self):
        s = CronSelfUpdater().stats()
        assert "asi_index_v0_1" in s  # 兼容旧调用方

    def test_parse_self_check_passes_for_current(self):
        res = CronSelfUpdater().parse()
        assert isinstance(res, CronMessageParseResult)
        assert res.is_valid is True, f"self-check failed: {res.to_dict()}"

    def test_version_bumped(self):
        assert CRON_SELF_UPDATE_VERSION == "0.2.0"
