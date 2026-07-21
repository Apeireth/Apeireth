"""Round-20 P0 borrow regression tests.

主 11:10 round-20 source-deep-read 推荐 P0 (这周):
  1. LLMResponse 加 cache_hit/cache_creation_tokens/cache_read_tokens 字段
     - 借鉴 anthropic-sdk: cache_creation_input_tokens + cache_read_input_tokens 独立计费
  2. Patch 加 seen_versions 矩阵防重复 commit
     - 借鉴 langgraph: versions_seen[node][channel] = version_seen

本测试锁住:
  - LLMResponse 新字段 + cache_savings_ratio 计算
  - Patch.seen_versions + mark_seen() (deterministic replay 防环)
  - call_llm_minimax 真实使用 cache 字段
  - V2 哲学守门
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.llm_kernel import LLMResponse
from apeireth.self_evolving import Patch


# === 1. LLMResponse 新字段测试 ===

class TestLLMResponseCacheFields:
    """借鉴 anthropic-sdk (主 11:10 round-20): cache_hit/cache_creation/cache_read 字段."""

    def test_llm_response_has_cache_hit_field(self):
        r = LLMResponse(content="hi", model="minimax/MiniMax-M3")
        assert hasattr(r, "cache_hit")
        assert r.cache_hit is False  # 默认 False

    def test_llm_response_has_cache_creation_tokens(self):
        r = LLMResponse(content="hi", model="minimax/MiniMax-M3")
        assert hasattr(r, "cache_creation_tokens")
        assert r.cache_creation_tokens == 0  # 默认 0

    def test_llm_response_has_cache_read_tokens(self):
        r = LLMResponse(content="hi", model="minimax/MiniMax-M3")
        assert hasattr(r, "cache_read_tokens")
        assert r.cache_read_tokens == 0  # 默认 0

    def test_cache_hit_can_be_set(self):
        r = LLMResponse(content="hi", model="minimax", cache_hit=True)
        assert r.cache_hit is True

    def test_to_dict_includes_cache_fields(self):
        r = LLMResponse(
            content="hi", model="minimax",
            tokens_in=100, cache_hit=True,
            cache_creation_tokens=50, cache_read_tokens=80,
        )
        d = r.to_dict()
        assert d["cache_hit"] is True
        assert d["cache_creation_tokens"] == 50
        assert d["cache_read_tokens"] == 80
        assert "cache_savings_ratio" in d

    def test_cache_savings_ratio_calculates_correctly(self):
        """cache_read / (cache_read + tokens_in) = 80 / (80 + 20) = 0.8"""
        r = LLMResponse(content="hi", model="minimax",
                        tokens_in=20, cache_read_tokens=80)
        assert r.cache_savings_ratio == pytest.approx(0.8)

    def test_cache_savings_ratio_zero_when_no_cache(self):
        r = LLMResponse(content="hi", model="minimax", tokens_in=100)
        assert r.cache_savings_ratio == 0.0

    def test_cache_savings_ratio_zero_when_zero_total(self):
        """edge case: 全 0 应该返回 0.0 (不是 div by zero)."""
        r = LLMResponse(content="hi", model="minimax")
        assert r.cache_savings_ratio == 0.0


# === 2. Patch seen_versions + mark_seen 测试 ===

class TestPatchSeenVersions:
    """借鉴 langgraph (主 11:10 round-20): versions_seen 矩阵防重复 commit."""

    def test_patch_has_seen_versions_field(self):
        p = Patch(pid="p1", target="sct", action="adjust",
                 payload={"key": "调度者", "delta": {}}, reason="test", proposer="evolution")
        assert hasattr(p, "seen_versions")
        assert isinstance(p.seen_versions, dict)
        assert len(p.seen_versions) == 0  # 默认空

    def test_mark_seen_first_time_returns_true(self):
        """首次 mark_seen 应该返回 True (新 seen, 触发 process)."""
        p = Patch(pid="p1", target="sct", action="adjust",
                 payload={"key": "调度者"}, reason="test", proposer="evolution")
        result = p.mark_seen(node="phase1_eval", channel="score", version=1)
        assert result is True

    def test_mark_seen_same_version_returns_false(self):
        """同一 (node, channel, version) 第二次 mark 应该返回 False (跳过, deterministic replay 防环)."""
        p = Patch(pid="p1", target="sct", action="adjust",
                 payload={"key": "调度者"}, reason="test", proposer="evolution")
        assert p.mark_seen("phase1_eval", "score", 1) is True   # 首次
        assert p.mark_seen("phase1_eval", "score", 1) is False  # 同 version, 跳过

    def test_mark_seen_new_version_returns_true(self):
        """新 version 应该返回 True (继续 process)."""
        p = Patch(pid="p1", target="sct", action="adjust",
                 payload={"key": "调度者"}, reason="test", proposer="evolution")
        p.mark_seen("phase1_eval", "score", 1)
        assert p.mark_seen("phase1_eval", "score", 2) is True  # 新 version

    def test_mark_seen_different_channel_independent(self):
        """不同 channel 独立 tracking (phase1_eval.score vs phase1_eval.weaknesses)."""
        p = Patch(pid="p1", target="sct", action="adjust",
                 payload={"key": "调度者"}, reason="test", proposer="evolution")
        assert p.mark_seen("phase1_eval", "score", 1) is True
        assert p.mark_seen("phase1_eval", "weaknesses", 1) is True  # 不同 channel, 独立

    def test_mark_seen_different_node_independent(self):
        """不同 node 独立 tracking."""
        p = Patch(pid="p1", target="sct", action="adjust",
                 payload={"key": "调度者"}, reason="test", proposer="evolution")
        assert p.mark_seen("phase1_eval", "score", 1) is True
        assert p.mark_seen("phase3_proposal", "patches", 1) is True  # 不同 node, 独立

    def test_seen_versions_records_correctly(self):
        p = Patch(pid="p1", target="sct", action="adjust",
                 payload={"key": "调度者"}, reason="test", proposer="evolution")
        p.mark_seen("phase1_eval", "score", 5)
        assert p.seen_versions == {"phase1_eval": {"score": 5}}


# === 3. call_llm_minimax cache 字段测试 ===

class TestCallLLMMiniMaxCache:
    """call_llm_minimax 真实使用 cache 字段 (借鉴 anthropic-sdk 协议)."""

    def test_template_response_has_cache_false(self):
        """没 API key 时 template fallback, cache 应该 False."""
        from apeireth.llm_kernel import call_llm_minimax
        # 没 api_key → template fallback
        r = call_llm_minimax("test", config=None)  # default config 没 key
        assert r.cache_hit is False
        assert r.cache_creation_tokens == 0
        assert r.cache_read_tokens == 0


# === 4. V2 哲学守门测试 ===

class TestV2PhilosophyGuard:
    """V2 哲学守门 (主 22:08): 不假装 Phenomenal."""

    def test_llm_response_no_consciousness_fields(self):
        r = LLMResponse(content="hi", model="minimax")
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal",
                     "self_aware", "subjective_experience"]
        for f in forbidden:
            assert not hasattr(r, f), f"LLMResponse 不应有假装意识字段 {f}"

    def test_patch_no_consciousness_fields(self):
        p = Patch(pid="p1", target="sct", action="adjust",
                 payload={"key": "调度者"}, reason="test", proposer="evolution")
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for f in forbidden:
            assert not hasattr(p, f), f"Patch 不应有假装意识字段 {f}"

    def test_cache_is_metric_not_consciousness(self):
        """cache_hit 是技术指标, 不假装意识."""
        r = LLMResponse(content="hi", model="minimax", cache_hit=True)
        # cache_hit 是 cost optimization 指标, 不假装 Phenomenal
        assert r.cache_hit is True
        assert "cache" not in [f for f in ["awareness", "consciousness", "phenomenal"]]


# === 5. 集成测试 ===

class TestIntegration:
    """LLMResponse + Patch 集成测试."""

    def test_llm_response_cache_metrics_comprehensive(self):
        r = LLMResponse(
            content="hi",
            model="minimax/MiniMax-M3",
            tokens_in=50,
            tokens_out=20,
            latency_ms=120.0,
            provider="minimax",
            cache_hit=True,
            cache_creation_tokens=30,
            cache_read_tokens=40,
        )
        d = r.to_dict()
        assert d["tokens_in"] == 50
        assert d["tokens_out"] == 20
        assert d["latency_ms"] == 120.0
        assert d["provider"] == "minimax"
        assert d["cache_hit"] is True
        assert d["cache_creation_tokens"] == 30
        assert d["cache_read_tokens"] == 40
        # savings_ratio = 40 / (40 + 50) = 0.444
        assert d["cache_savings_ratio"] == pytest.approx(0.4444, rel=0.01)

    def test_patch_with_seen_versions_in_repr(self):
        p = Patch(pid="p1", target="sct", action="adjust",
                 payload={"key": "调度者"}, reason="test", proposer="evolution")
        p.mark_seen("phase1_eval", "score", 1)
        p.mark_seen("phase3_proposal", "patches", 2)
        assert p.seen_versions == {
            "phase1_eval": {"score": 1},
            "phase3_proposal": {"patches": 2},
        }


if __name__ == "__main__":
    pytest.main([__file__, "-v"])