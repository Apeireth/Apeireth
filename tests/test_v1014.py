"""V1014 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1014_cost_optimization import (
    V1014_VERSION, MODEL_PRICING, UsageRecord, ModelRouter,
    V1014CostOptimization,
)


class TestV1014:
    def test_init(self):
        co = V1014CostOptimization()
        assert co.n_records() == 0
        assert len(co.routers) == 3
        assert len(co.cache) == 0

    def test_compute_cost_gpt4(self):
        """V1014 真测 OpenAI pricing 真借鉴."""
        co = V1014CostOptimization()
        cost = co.compute_cost("gpt-4", 1000, 500)
        expected = (1000 / 1000.0) * 0.03 + (500 / 1000.0) * 0.06
        assert abs(cost - expected) < 0.001

    def test_compute_cost_claude_opus(self):
        co = V1014CostOptimization()
        cost = co.compute_cost("claude-3-opus", 1000, 1000)
        expected = (1000 / 1000.0) * 0.015 + (1000 / 1000.0) * 0.075
        assert abs(cost - expected) < 0.001

    def test_compute_cost_unknown_model(self):
        co = V1014CostOptimization()
        with pytest.raises(ValueError):
            co.compute_cost("unknown", 100, 100)

    def test_record_usage(self):
        co = V1014CostOptimization()
        rec = co.record_usage("gpt-4", 1000, 500)
        assert rec.cost_usd > 0
        assert co.n_records() == 1

    def test_record_usage_cached(self):
        """V1014 真测 cached 不计费 (主 17:43 实事求是)."""
        co = V1014CostOptimization()
        rec = co.record_usage("gpt-4", 1000, 500, cached=True)
        assert rec.cost_usd == 0.0

    def test_route_primary(self):
        """V1014 真测 LiteLLM router 真借鉴 (主 19:33)."""
        co = V1014CostOptimization()
        model = co.route("balanced", 1000, 0.5)
        assert model == "gpt-4-turbo"

    def test_route_fallback(self):
        co = V1014CostOptimization()
        # balanced cost_threshold 默认 1.0
        model = co.route("balanced", 1000, 2.0)
        assert model == "claude-3-haiku"

    def test_route_unknown(self):
        co = V1014CostOptimization()
        with pytest.raises(ValueError):
            co.route("unknown", 100, 0.1)

    def test_register_router(self):
        co = V1014CostOptimization()
        co.register_router(ModelRouter(
            router_id="custom", name="Custom",
            primary_model="gpt-4", fallback_model="gpt-3.5-turbo",
        ))
        assert len(co.routers) == 4

    def test_cache_prompt(self):
        """V1014 真测 Anthropic prompt cache 真借鉴."""
        co = V1014CostOptimization()
        co.cache_prompt("hash1", "response1")
        assert co.get_cached("hash1") == "response1"
        assert co.get_cached("missing") is None

    def test_total_cost(self):
        co = V1014CostOptimization()
        co.record_usage("gpt-4", 1000, 500)
        co.record_usage("gpt-3.5-turbo", 2000, 1000)
        assert co.total_cost() > 0

    def test_total_saved(self):
        """V1014 真测 cached 真节省 (主 17:43 实事求是)."""
        co = V1014CostOptimization()
        co.record_usage("gpt-4", 1000, 500, cached=True)
        co.record_usage("gpt-4", 1000, 500, cached=True)
        assert co.total_saved() > 0

    def test_stats(self):
        co = V1014CostOptimization()
        co.record_usage("gpt-4", 100, 100)
        s = co.stats()
        assert s["n_records"] == 1
        assert s["n_routers"] == 3
        assert s["total_cost"] > 0

    def test_v22_33_asi_integration(self):
        """V1014 真测主 22:33 ASI 北极星."""
        co = V1014CostOptimization()
        s = co.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_openai_anthropic_litellm(self):
        """V1014 真测主 19:33 OpenAI + Anthropic + LiteLLM 真借鉴."""
        co = V1014CostOptimization()
        assert "gpt-4" in MODEL_PRICING
        assert "claude-3-opus" in MODEL_PRICING
        assert "balanced" in co.routers

    def test_v17_43_truth(self):
        """V1014 真测主 17:43 实事求是 — 真计费, 不假装."""
        co = V1014CostOptimization()
        # 真测 gpt-4 比 gpt-3.5-turbo 贵
        cost_4 = co.compute_cost("gpt-4", 1000, 1000)
        cost_35 = co.compute_cost("gpt-3.5-turbo", 1000, 1000)
        assert cost_4 > cost_35

    def test_complete_integration(self):
        """V1014 真测完整 cost optimization (主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        co = V1014CostOptimization()
        co.record_usage("gpt-4", 1000, 500)
        co.cache_prompt("h1", "r1")
        co.route("balanced", 500, 0.5)
        s = co.stats()
        assert s["n_records"] == 1
        assert s["n_cached"] == 1
        assert s["total_cost"] > 0