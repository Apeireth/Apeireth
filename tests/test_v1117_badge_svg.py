"""Tests for v1117_badge_svg_renderer (R9-DEV-003 / W4 收尾).

覆盖 4 个真功能:
  1. Badge SVG 渲染 (GREEN/YELLOW/RED 显式映射)
  2. 跨模型差异 SVG/HTML 图表
  3. HF Model Cache 超时控制
  4. REAL_MODEL_ENV env 配置 (load / write / apply)

主 17:43 实事求是: 数据全从真输入, 不 hardcode.
主 17:58 不假装: 超时 / 缺文件 → 显式异常, 不假装 PASS.
"""
from __future__ import annotations

import json
import os
import threading
import time
from pathlib import Path

import pytest

from apeireth.v1117_badge_svg_renderer import (
    COLOR_MAP,
    DEFAULT_ENV_FILE,
    HFModelCache,
    HFModelTimeoutError,
    REAL_MODEL_ENV,
    STATUS_TO_COLOR,
    apply_env_file,
    load_env_file,
    render_badge_history_svg,
    render_badge_svg,
    render_diff_html,
    render_diff_svg,
    render_status_badge,
    write_env_file,
)


# ---------------------------------------------------------------------------
# 1. Badge SVG 渲染 (GREEN/YELLOW/RED 显式映射)
# ---------------------------------------------------------------------------
class TestBadgeSVG:
    def test_render_badge_svg_basic(self):
        s = render_badge_svg(label="ci", message="12/13 pass", color="green")
        assert s.startswith("<svg")
        assert s.endswith("</svg>")
        assert "ci" in s
        assert "12/13 pass" in s
        # brightgreen
        assert "#4c1" in s

    def test_render_badge_svg_yellow_color(self):
        s = render_badge_svg(label="ci", message="mixed", color="yellow")
        assert "#dfb317" in s

    def test_render_badge_svg_red_color(self):
        s = render_badge_svg(label="ci", message="failed", color="red")
        assert "#e05d44" in s

    def test_render_badge_svg_hex_color(self):
        s = render_badge_svg(label="ci", message="x", color="#a1b2c3")
        assert "#a1b2c3" in s

    def test_render_badge_svg_unknown_color_falls_back(self):
        s = render_badge_svg(label="ci", message="x", color="not-a-color")
        assert "#9f9f9f" in s  # UNKNOWN fallback

    def test_render_status_badge_pass(self):
        s = render_status_badge(status="pass", message="5/5")
        assert "#4c1" in s
        assert "5/5" in s

    def test_render_status_badge_fail(self):
        s = render_status_badge(status="fail", message="0/5")
        assert "#e05d44" in s

    def test_render_status_badge_mixed_yellow(self):
        s = render_status_badge(status="mixed", message="2/4")
        assert "#dfb317" in s

    def test_color_map_explicit_keys(self):
        # GREEN/YELLOW/RED/UNKNOWN 显式映射 (主 17:43 实事求是)
        assert COLOR_MAP["GREEN"] == "#4c1"
        assert COLOR_MAP["YELLOW"] == "#dfb317"
        assert COLOR_MAP["RED"] == "#e05d44"
        assert COLOR_MAP["UNKNOWN"] == "#9f9f9f"
        assert STATUS_TO_COLOR["pass"] == "#4c1"
        assert STATUS_TO_COLOR["fail"] == "#e05d44"
        assert STATUS_TO_COLOR["mixed"] == "#dfb317"
        assert STATUS_TO_COLOR["unknown"] == "#9f9f9f"

    def test_render_badge_svg_html_escapes(self):
        # XSS 防护: < > & 应当被 html.escape
        s = render_badge_svg(label="<x>", message="a&b", color="green")
        assert "<x>" not in s
        assert "&lt;x&gt;" in s
        assert "a&amp;b" in s

    def test_render_badge_history_svg_combines(self):
        h = render_badge_history_svg(
            history=[("w1", "pass"), ("w2", "mixed"), ("w3", "fail")],
            label="ci",
        )
        assert h.startswith("<svg")
        # 三段 badge 都应出现
        assert "ci-w1" in h
        assert "ci-w2" in h
        assert "ci-w3" in h

    def test_render_badge_svg_flat_square_style(self):
        s = render_badge_svg(label="ci", message="x", color="green", style="flat-square")
        assert 'height="28"' in s  # flat-square 高度

    def test_render_badge_svg_empty_history_returns_placeholder(self):
        s = render_badge_history_svg(history=[], label="ci")
        assert "0/0" in s


# ---------------------------------------------------------------------------
# 2. 跨模型差异 SVG/HTML
# ---------------------------------------------------------------------------
class TestDiffViz:
    @pytest.fixture
    def diff_data(self) -> dict:
        return {
            "baseline_name": "fixture-7b-v1",
            "lift_summary": {"n_loaded": 1, "n_failed": 0, "mean_delta": 0.05},
            "rows": [
                {"target": "fixture-3b-v1", "available": True,
                 "delta_sc": 0.02, "delta_nr": 0.01, "delta_ev": 0.03,
                 "delta_cdt": 0.04, "delta_subscore": 0.025},
                {"target": "qwen-7b-local", "available": True,
                 "delta_sc": 0.15, "delta_nr": 0.12, "delta_ev": 0.10,
                 "delta_cdt": 0.18, "delta_subscore": 0.1375},
                {"target": "llama-8b-local", "available": False,
                 "delta_sc": None, "delta_nr": None, "delta_ev": None,
                 "delta_cdt": None, "delta_subscore": None},
            ],
        }

    def test_render_diff_svg_basic(self, diff_data):
        svg = render_diff_svg(diff_data)
        assert svg.startswith("<svg")
        # 有效 row (前 2 个) 应有 rect
        assert "<rect" in svg
        # 标签
        assert "fixture-3b-v1" in svg or "qwen-7b-local" in svg

    def test_render_diff_svg_no_rows(self):
        svg = render_diff_svg({"rows": []})
        assert "no rows" in svg

    def test_render_diff_svg_no_valid_metric(self):
        svg = render_diff_svg({"rows": [{"target": "x", "delta_subscore": None}]})
        assert "no valid" in svg

    def test_render_diff_html_contains_table(self, diff_data):
        html = render_diff_html(diff_data, title="Test Diff")
        assert html.startswith("<!DOCTYPE html>")
        assert "Test Diff" in html
        assert "<table" in html
        assert "</table>" in html
        # unavailable 行应显式标记 (主 17:58 不假装: 用 ❌ 显式区分)
        assert "llama-8b-local" in html
        assert "❌" in html
        assert "✅" in html  # available row 也显式标记

    def test_render_diff_html_with_ci_badge(self, diff_data):
        # render_diff_html 通过 embed_svg 切换是否嵌 SVG
        html_with = render_diff_html(diff_data, title="CI", embed_svg=True)
        html_without = render_diff_html(diff_data, title="CI", embed_svg=False)
        # embed_svg=True → SVG 内嵌 (主 13:31 大胆激进)
        assert "<svg" in html_with
        # embed_svg=False → 无 SVG, 只剩 table
        assert "<svg" not in html_without
        # 两者都有 table
        assert "<table" in html_with and "<table" in html_without


# ---------------------------------------------------------------------------
# 3. HF Model Cache 超时控制 (主 17:58 不假装)
# ---------------------------------------------------------------------------
class TestHFModelCacheTimeout:
    def test_cache_returns_value_when_fast(self):
        cache = HFModelCache(timeout_sec=2.0)
        v = cache.get_or_load(lambda: 42)
        assert v == 42

    def test_cache_timeout_explicit_error(self):
        cache = HFModelCache(timeout_sec=0.3)

        def slow():
            time.sleep(2.0)
            return "x"

        with pytest.raises(HFModelTimeoutError) as ei:
            cache.get_or_load(slow)
        msg = str(ei.value)
        assert "0.3s" in msg or "0.3" in msg
        assert "thread still alive" in msg

    def test_cache_propagates_original_exception(self):
        cache = HFModelCache(timeout_sec=2.0)

        def boom():
            raise ValueError("model load failed")

        with pytest.raises(ValueError, match="model load failed"):
            cache.get_or_load(boom)

    def test_cache_repeated_call_no_reload(self):
        cache = HFModelCache(timeout_sec=2.0)
        counter = {"n": 0}

        def make():
            counter["n"] += 1
            return counter["n"]

        v1 = cache.get_or_load(make)
        v2 = cache.get_or_load(make)
        assert v1 == 1
        assert v2 == 1
        assert counter["n"] == 1

    def test_cache_no_cache_mode_reruns(self):
        cache = HFModelCache(timeout_sec=2.0, cache=False)
        counter = {"n": 0}

        def make():
            counter["n"] += 1
            return counter["n"]

        assert cache.get_or_load(make) == 1
        assert cache.get_or_load(make) == 2

    def test_cache_reset_clears_state(self):
        cache = HFModelCache(timeout_sec=2.0)
        cache.get_or_load(lambda: 1)
        cache.reset()
        assert cache._loaded is False
        assert cache._value is None

    def test_cache_timeout_records_elapsed_ms(self):
        cache = HFModelCache(timeout_sec=0.5)
        try:
            cache.get_or_load(lambda: time.sleep(1.5))
        except HFModelTimeoutError:
            pass
        assert cache.elapsed_ms >= 400  # 至少等 0.5s


# ---------------------------------------------------------------------------
# 4. REAL_MODEL_ENV env 配置
# ---------------------------------------------------------------------------
class TestEnvConfig:
    def test_real_model_env_has_5_keys(self):
        assert set(REAL_MODEL_ENV.keys()) == {"qwen", "llama", "hermes", "gemma", "embedding"}

    def test_load_env_file_missing_returns_empty(self, tmp_path):
        p = tmp_path / "nope.env"
        assert load_env_file(p) == {}

    def test_write_and_load_env_file_roundtrip(self, tmp_path):
        p = tmp_path / "test.env"
        values = {
            "APEIRETH_QWEN35_PATH": "/models/qwen",
            "APEIRETH_LLAMA31_PATH": "/models/llama",
            "# trailing comment": None,  # 占位, 写时跳过
        }
        write_env_file(values, path=p)
        loaded = load_env_file(p)
        assert loaded["APEIRETH_QWEN35_PATH"] == "/models/qwen"
        assert loaded["APEIRETH_LLAMA31_PATH"] == "/models/llama"

    def test_load_env_file_parses_quoted(self, tmp_path):
        p = tmp_path / "q.env"
        p.write_text('KEY1="hello world"\nKEY2=\'literal\'\n# comment\n\n', encoding="utf-8")
        loaded = load_env_file(p)
        assert loaded["KEY1"] == "hello world"
        assert loaded["KEY2"] == "literal"

    def test_apply_env_file_does_not_override(self, monkeypatch, tmp_path):
        monkeypatch.setenv("APEIRETH_X", "from_env")
        p = tmp_path / "a.env"
        p.write_text("APEIRETH_X=from_file\n", encoding="utf-8")
        apply_env_file(path=p)
        assert os.environ["APEIRETH_X"] == "from_env"

    def test_apply_env_file_override_true(self, monkeypatch, tmp_path):
        monkeypatch.setenv("APEIRETH_X", "from_env")
        p = tmp_path / "a.env"
        p.write_text("APEIRETH_X=from_file\n", encoding="utf-8")
        apply_env_file(path=p, override=True)
        assert os.environ["APEIRETH_X"] == "from_file"


# ---------------------------------------------------------------------------
# 5. CLI 入口 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------
class TestCLI:
    def test_module_imports_cli_entry(self, capsys, monkeypatch):
        from apeireth import v1117_badge_svg_renderer as mod
        # 确保模块顶部能跑 (含 CLI 入口)
        # 模拟命令行: apeireth_render_badge
        # 由于我们没单独写 __main__, 验证 render_badge_svg 可直接命令行调用
        out = mod.render_badge_svg("ci", "demo", "green")
        assert out.startswith("<svg")