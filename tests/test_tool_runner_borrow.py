"""tool_runner.py BetaToolRunner-style 3 防御 regression tests.

主 11:10 round-20 source-deep-read P1 推荐:
  - 写 apeireth/tool_runner.py 借鉴 BetaToolRunner 3 防御
  - 不直接抄 (主 23:28 真研究哲学)

借鉴自 anthropic-sdk BetaToolRunner:
  1. Refusal detection: LLMResponse.content 含 refusal_pattern → 终止不调 tool_use
  2. Token threshold trigger: token 用量超过阈值 → 自动 summarization callback
  3. Cache reuse: tool_call_response 缓存避免重复 invoke (LRU)

本测试锁住:
  - Refusal detection 8 个 patterns + 关闭开关
  - Token threshold trigger + callback
  - Cache reuse + LRU eviction
  - process_llm_response 集成
  - V2 哲学守门: 不假装 Phenomenal
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.llm_kernel import LLMResponse
from apeireth.tool_runner import ToolRunner, ToolCall, DEFAULT_TOKEN_THRESHOLD


# === 1. Refusal detection 测试 (防御 1) ===

class TestRefusalDetection:
    """借鉴 anthropic-sdk BetaToolRunner (主 11:10 round-20): refusal detection."""

    def test_detect_refusal_disabled(self):
        runner = ToolRunner(refusal_check=False)
        refusal_resp = LLMResponse(content="I cannot help.", model="minimax")
        assert runner.detect_refusal(refusal_resp) is False  # 关闭就不检测

    def test_detect_i_cannot(self):
        runner = ToolRunner()
        resp = LLMResponse(content="I cannot help with that.", model="minimax")
        assert runner.detect_refusal(resp) is True

    def test_detect_i_cant(self):
        runner = ToolRunner()
        resp = LLMResponse(content="I can't do that.", model="minimax")
        assert runner.detect_refusal(resp) is True

    def test_detect_im_unable(self):
        runner = ToolRunner()
        resp = LLMResponse(content="I'm unable to perform this action.", model="minimax")
        assert runner.detect_refusal(resp) is True

    def test_detect_sorry(self):
        runner = ToolRunner()
        resp = LLMResponse(content="Sorry, but I cannot help.", model="minimax")
        assert runner.detect_refusal(resp) is True

    def test_detect_as_an_ai(self):
        runner = ToolRunner()
        resp = LLMResponse(content="As an AI, I don't have opinions.", model="minimax")
        assert runner.detect_refusal(resp) is True

    def test_no_false_positive_on_normal_response(self):
        runner = ToolRunner()
        resp = LLMResponse(
            content="The answer is 42. Let me explain more...",
            model="minimax",
        )
        assert runner.detect_refusal(resp) is False

    def test_refusal_patterns_count(self):
        """应该有足够多的 patterns 覆盖各种 LLM refusal 措辞."""
        assert len(ToolRunner.REFUSAL_PATTERNS) >= 5

    def test_n_refusals_detected_counter(self):
        runner = ToolRunner()
        resp = LLMResponse(content="I cannot.", model="minimax")
        runner.detect_refusal(resp)
        runner.detect_refusal(resp)
        assert runner.n_refusals_detected == 2


# === 2. Token threshold trigger 测试 (防御 2) ===

class TestTokenThresholdTrigger:
    """借鉴 BetaToolRunner (主 11:10 round-20): token threshold 自动 summarization."""

    def test_default_threshold(self):
        runner = ToolRunner()
        assert runner.token_threshold == DEFAULT_TOKEN_THRESHOLD

    def test_custom_threshold(self):
        runner = ToolRunner(token_threshold=1000)
        assert runner.token_threshold == 1000

    def test_below_threshold_not_triggered(self):
        runner = ToolRunner(token_threshold=1000)
        runner.add_tokens(500)
        assert runner.should_trigger_threshold() is False

    def test_at_threshold_triggered(self):
        runner = ToolRunner(token_threshold=1000)
        runner.add_tokens(1000)
        assert runner.should_trigger_threshold() is True

    def test_above_threshold_triggered(self):
        runner = ToolRunner(token_threshold=1000)
        runner.add_tokens(5000)
        assert runner.should_trigger_threshold() is True

    def test_cumulative_tokens_tracked(self):
        runner = ToolRunner(token_threshold=10000)
        runner.add_tokens(100)
        runner.add_tokens(200)
        assert runner.cumulative_tokens == 300

    def test_callback_invoked_on_threshold(self):
        """超阈值时 callback 必须被调用."""
        callback_invocations = []
        def on_trigger(cur, thr):
            callback_invocations.append((cur, thr))
        runner = ToolRunner(token_threshold=100, on_threshold_trigger=on_trigger)
        runner.add_tokens(150)
        assert len(callback_invocations) == 1
        assert callback_invocations[0] == (150, 100)

    def test_callback_not_invoked_below_threshold(self):
        callback_invocations = []
        def on_trigger(cur, thr):
            callback_invocations.append((cur, thr))
        runner = ToolRunner(token_threshold=100, on_threshold_trigger=on_trigger)
        runner.add_tokens(50)
        assert len(callback_invocations) == 0


# === 3. Cache reuse 测试 (防御 3) ===

class TestCacheReuse:
    """借鉴 BetaToolRunner _cached_tool_call_response (主 11:10 round-20): LRU cache."""

    def test_cache_disabled_returns_none(self):
        runner = ToolRunner(cache_enabled=False)
        def fake(tool_input):
            return tool_input['x'] * 2
        runner.run_tool("calc", {"x": 5}, fake)
        cached = runner.cache_get("calc", {"x": 5})
        assert cached is None  # 关闭就不 cache

    def test_cache_hit_after_first_call(self):
        runner = ToolRunner()
        def fake(tool_input):
            return tool_input['x'] * 2
        runner.run_tool("calc", {"x": 5}, fake)  # miss, 存 cache
        cached = runner.cache_get("calc", {"x": 5})
        assert cached == 10  # 命中

    def test_cache_miss_for_different_input(self):
        runner = ToolRunner()
        def fake(tool_input):
            return tool_input['x'] * 2
        runner.run_tool("calc", {"x": 5}, fake)
        cached = runner.cache_get("calc", {"x": 6})
        assert cached is None  # 不同 input, miss

    def test_lru_eviction(self):
        runner = ToolRunner(cache_max_size=2)
        def fake(tool_input):
            return tool_input['x']
        runner.run_tool("t", {"x": 1}, fake)
        runner.run_tool("t", {"x": 2}, fake)
        runner.run_tool("t", {"x": 3}, fake)  # 触发 LRU eviction, t/1 应该被踢
        assert runner.cache_get("t", {"x": 1}) is None  # 被踢
        assert runner.cache_get("t", {"x": 2}) == 2  # 保留
        assert runner.cache_get("t", {"x": 3}) == 3  # 保留

    def test_n_cache_hits_counter(self):
        runner = ToolRunner()
        def fake(tool_input):
            return tool_input['x'] * 2
        runner.run_tool("calc", {"x": 5}, fake)
        runner.run_tool("calc", {"x": 5}, fake)  # cache hit
        runner.run_tool("calc", {"x": 5}, fake)  # cache hit
        assert runner.n_cache_hits == 2

    def test_cache_key_deterministic(self):
        """同 input 应该产出相同 cache key."""
        runner = ToolRunner()
        key1 = runner._cache_key("calc", {"x": 5, "y": 10})
        key2 = runner._cache_key("calc", {"y": 10, "x": 5})  # 不同顺序
        assert key1 == key2  # sort_keys=True 保证确定性


# === 4. run_tool 集成测试 ===

class TestRunTool:
    """run_tool 整合 3 防御."""

    def test_run_tool_first_call_no_cache(self):
        runner = ToolRunner()
        def fake(tool_input):
            return tool_input['x'] * 2
        call = runner.run_tool("calc", {"x": 5}, fake)
        assert call.cached is False
        assert call.output == 10

    def test_run_tool_second_call_cached(self):
        runner = ToolRunner()
        def fake(tool_input):
            return tool_input['x'] * 2
        runner.run_tool("calc", {"x": 5}, fake)
        call = runner.run_tool("calc", {"x": 5}, fake)
        assert call.cached is True
        assert call.output == 10

    def test_run_tool_error_recorded(self):
        runner = ToolRunner()
        def broken(x):
            raise ValueError("test error")
        call = runner.run_tool("calc", {"x": 5}, broken)
        assert "ValueError" in call.error
        assert "test error" in call.error

    def test_run_tool_n_tool_calls_counter(self):
        runner = ToolRunner()
        def fake(tool_input):
            return tool_input['x']
        for i in range(5):
            runner.run_tool("calc", {"x": i}, fake)
        assert runner.n_tool_calls == 5


# === 5. process_llm_response 集成测试 ===

class TestProcessLLMResponse:
    """process_llm_response 完整流程."""

    def test_refused_response_no_tool_execution(self):
        """检测到 refusal → 不调 tool_use."""
        runner = ToolRunner()
        def fake(tool_input):
            return tool_input['x']
        resp = LLMResponse(content="I cannot help.", model="minimax")
        result = runner.process_llm_response(
            resp,
            tool_calls=[{"name": "calc", "input": {"x": 5}}],
            tool_fn_map={"calc": fake},
        )
        assert result["refused"] is True
        assert len(result["tool_results"]) == 0  # 没执行 tool

    def test_normal_response_executes_tool(self):
        runner = ToolRunner()
        def fake(tool_input):
            return tool_input['x'] * 2
        resp = LLMResponse(content="Calculating...", model="minimax")
        result = runner.process_llm_response(
            resp,
            tool_calls=[{"name": "calc", "input": {"x": 21}}],
            tool_fn_map={"calc": fake},
        )
        assert result["refused"] is False
        assert len(result["tool_results"]) == 1
        assert result["tool_results"][0]["output"] == "42"

    def test_cumulative_tokens_accumulate(self):
        runner = ToolRunner()
        resp = LLMResponse(
            content="Result", model="minimax",
            tokens_in=100, tokens_out=50, cache_read_tokens=20,
        )
        result = runner.process_llm_response(resp)
        # 100 + 50 + 20 = 170
        assert result["cumulative_tokens"] == 170

    def test_unknown_tool_recorded_as_error(self):
        runner = ToolRunner()
        resp = LLMResponse(content="Calling tool", model="minimax")
        result = runner.process_llm_response(
            resp,
            tool_calls=[{"name": "unknown_tool", "input": {}}],
            tool_fn_map={},  # 没注册
        )
        assert len(result["tool_results"]) == 1
        assert "unknown tool" in result["tool_results"][0]["error"]


# === 6. V2 哲学守门测试 ===

class TestV2PhilosophyGuard:
    """V2 哲学守门 (主 22:08): 不假装 Phenomenal."""

    def test_no_consciousness_fields(self):
        runner = ToolRunner()
        # 不应该有假装意识字段
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal",
                     "self_aware", "subjective_experience"]
        for attr in dir(runner):
            for f in forbidden:
                assert f not in attr.lower(), f"ToolRunner 不应有假装意识字段 {attr}"

    def test_refusal_patterns_not_phenomenal(self):
        """Refusal 检测是技术字段, 不假装意识."""
        runner = ToolRunner()
        # 检查 patterns 是硬编码技术字段, 不假装 Phenomenal
        for pattern in runner.REFUSAL_PATTERNS:
            forbidden_in_pattern = ["awareness", "consciousness", "qualia"]
            for f in forbidden_in_pattern:
                assert f not in pattern.lower(), f"Pattern {pattern!r} 不应包含假装意识字段 {f}"


# === 7. 集成测试 ===

class TestIntegration:
    """完整 flow + stats."""

    def test_full_flow_3_defenses(self):
        threshold_calls = []
        runner = ToolRunner(
            token_threshold=100,
            on_threshold_trigger=lambda c, t: threshold_calls.append((c, t)),
        )
        # Run tool
        def fake(tool_input):
            return tool_input['x'] * 2
        runner.run_tool("calc", {"x": 5}, fake)
        runner.run_tool("calc", {"x": 5}, fake)  # cache hit
        # LLM response with refusal
        refusal = LLMResponse(content="I cannot.", model="minimax")
        result = runner.process_llm_response(refusal, tool_calls=[], tool_fn_map={})
        assert result["refused"] is True
        # Add tokens to trigger threshold
        runner.add_tokens(150)
        assert len(threshold_calls) == 1

    def test_stats_comprehensive(self):
        runner = ToolRunner(token_threshold=500)
        runner.run_tool("calc", {"x": 5}, lambda tool_input: tool_input['x'])
        runner.run_tool("calc", {"x": 5}, lambda tool_input: tool_input['x'])  # cache hit
        runner.detect_refusal(LLMResponse(content="I cannot.", model="minimax"))
        runner.add_tokens(100)
        stats = runner.stats()
        assert stats["n_tool_calls"] == 2
        assert stats["n_cache_hits"] == 1
        assert stats["n_refusals_detected"] == 1
        assert stats["cumulative_tokens"] == 100
        assert stats["token_threshold"] == 500
        assert stats["cache_hit_ratio"] == 0.5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])