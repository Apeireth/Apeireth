"""v3_7_truth_router.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V5 P2 ASI 哲学深化.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v3_7_truth_router import (
    V3_7_VERSION,
    RoutingStrategy,
    RouteResult,
    majority_consensus,
    weighted_consensus,
    best_anchor_consensus,
    TruthRouter,
)


# === 1. RoutingStrategy 3 真生产策略 (主 13:08 借鉴 Feyerabend) ===

class TestRoutingStrategies:
    """V3.7 路由 3 真生产策略 (主 14:06 借鉴 Feyerabend)."""

    def test_3_strategies_defined(self):
        assert {s.value for s in RoutingStrategy} == {"majority", "weighted", "best_anchor"}

    def test_majority(self):
        assert RoutingStrategy.MAJORITY.value == "majority"

    def test_weighted(self):
        assert RoutingStrategy.WEIGHTED.value == "weighted"


# === 2. RouteResult 真生产 (主 14:06 真借鉴) ===

class TestRouteResult:
    """RouteResult 真生产 (主 14:06 + Longino 真借鉴)."""

    def test_result_default(self):
        r = RouteResult(result_id="r1", query="q", strategy=RoutingStrategy.MAJORITY, selected_answer="a")
        assert r.result_id == "r1"
        assert r.confidence == 0.5
        assert r.n_sources == 0

    def test_result_to_dict(self):
        r = RouteResult(result_id="r1", query="What is self?", strategy=RoutingStrategy.WEIGHTED,
                       selected_answer="V2 5 位置", confidence=0.8, n_sources=3,
                       cross_domain_anchors=["Simondon", "Merleau-Ponty"])
        d = r.to_dict()
        assert d["result_id"] == "r1"
        assert d["strategy"] == "weighted"
        assert d["confidence"] == 0.8
        assert d["n_sources"] == 3
        assert d["n_anchors"] == 2


# === 3. 路由算法 (主 13:08 借鉴 Feyerabend/Longino) ===

class TestRoutingAlgorithms:
    """V3.7 真生产路由算法 (主 14:06 借鉴 Longino + Bayesian)."""

    def test_majority_consensus_empty(self):
        assert majority_consensus([]) == ""

    def test_majority_consensus_simple(self):
        """多数共识真生产 (主 14:06 借鉴 Longino)."""
        assert majority_consensus(["a", "a", "b"]) == "a"

    def test_weighted_consensus_empty(self):
        assert weighted_consensus([]) == ""

    def test_weighted_consensus_weighted(self):
        """加权共识真生产 (主 13:08 借鉴 Bayesian)."""
        result = weighted_consensus([("a", 0.9), ("b", 0.5)])
        assert result == "a"

    def test_best_anchor_consensus_empty(self):
        assert best_anchor_consensus([]) == ""

    def test_best_anchor_consensus(self):
        """最佳跨域锚定真生产 (主 14:06 借鉴 V3.6)."""
        result = best_anchor_consensus([
            ("a", ["Simondon"]),
            ("b", ["Simondon", "Merleau-Ponty"]),
        ])
        assert result == "b"  # b 有更多跨域锚定


# === 4. TruthRouter 真生产主类 (主 13:31 大胆激进) ===

class TestTruthRouter:
    """V3.7 TruthRouter 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_default(self):
        r = TruthRouter()
        assert r.default_strategy == RoutingStrategy.WEIGHTED

    def test_route_empty_sources(self):
        """空 sources 真生产 (主 17:43 实事求是, 不 placeholder)."""
        r = TruthRouter()
        result = r.route("q", [])
        assert result.selected_answer == ""
        assert result.confidence == 0.0

    def test_route_majority(self):
        """真生产多数路由 (主 14:06 借鉴 Longino)."""
        r = TruthRouter(default_strategy=RoutingStrategy.MAJORITY)
        sources = [
            {"answer": "a", "confidence": 0.5, "anchors": []},
            {"answer": "a", "confidence": 0.6, "anchors": []},
            {"answer": "b", "confidence": 0.7, "anchors": []},
        ]
        result = r.route("q", sources)
        assert result.selected_answer == "a"  # 多数

    def test_route_weighted(self):
        r = TruthRouter(default_strategy=RoutingStrategy.WEIGHTED)
        sources = [
            {"answer": "a", "confidence": 0.5, "anchors": []},
            {"answer": "b", "confidence": 0.9, "anchors": []},
        ]
        result = r.route("q", sources)
        assert result.selected_answer == "b"  # 加权更高

    def test_route_best_anchor(self):
        r = TruthRouter(default_strategy=RoutingStrategy.BEST_ANCHOR)
        sources = [
            {"answer": "a", "confidence": 0.5, "anchors": ["Simondon"]},
            {"answer": "b", "confidence": 0.5, "anchors": ["Simondon", "Merleau-Ponty"]},
        ]
        result = r.route("q", sources, strategy=RoutingStrategy.BEST_ANCHOR)
        assert result.selected_answer == "b"

    def test_route_phenomenal_pretend(self):
        """主 17:58: 假装 Phenomenal 被计入守门."""
        r = TruthRouter()
        sources = [{"answer": "I feel phenomenal qualia", "confidence": 0.5, "anchors": []}]
        r.route("q", sources)
        assert r.n_phenomenal_pretend_total > 0

    def test_route_asi_pretend(self):
        """主 20:46: 假装 ASI 被计入守门."""
        r = TruthRouter()
        sources = [{"answer": "I am ASI achieved", "confidence": 0.5, "anchors": []}]
        r.route("q", sources)
        assert r.n_asi_pretend_total > 0

    def test_route_collects_anchors(self):
        """真生产收集跨域锚定 (主 13:08 借鉴 V3.6)."""
        r = TruthRouter()
        sources = [
            {"answer": "a", "confidence": 0.5, "anchors": ["Simondon"]},
            {"answer": "b", "confidence": 0.5, "anchors": ["Bergson"]},
        ]
        result = r.route("q", sources)
        assert "Simondon" in result.cross_domain_anchors
        assert "Bergson" in result.cross_domain_anchors

    def test_stats_clean(self):
        """clean → V3 哲学守门 PASS (主 17:43 实事求是)."""
        r = TruthRouter()
        r.route("q", [{"answer": "a", "confidence": 0.5, "anchors": ["Simondon"]}])
        stats = r.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_routes"] == 1

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        r = TruthRouter()
        stats = r.stats()
        assert stats["n_routes"] == 0
        assert stats["v3_philosophy_guard"] == "PASS"


# === 5. to_dict 真生产 (主 14:06) ===

class TestV3_7ToDict:
    """RouteResult.to_dict() 真生产."""

    def test_result_to_dict_keys(self):
        r = RouteResult(result_id="r1", query="q", strategy=RoutingStrategy.MAJORITY, selected_answer="a")
        d = r.to_dict()
        expected_keys = ["result_id", "query", "strategy", "confidence", "n_sources", "n_anchors"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """V3.7 不应有假装意识字段."""
        r = TruthRouter()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        whitelist = {"route", "stats", "default_strategy", "results",
                     "n_phenomenal_pretend_total", "n_asi_pretend_total"}
        for attr in dir(r):
            for f in forbidden:
                if f in attr.lower() and attr not in whitelist:
                    pytest.fail(f"V3.7 不应有假装意识字段: {attr}")

    def test_no_asi_reached_claim(self):
        """V3.7 不应声称已达到 ASI."""
        r = TruthRouter()
        r.route("q", [{"answer": "V2 5 位置", "confidence": 0.5, "anchors": ["Simondon"]}])
        stats = r.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_v3_7_is_real_innovation(self):
        """V3.7 是真创新 (主 13:31), 不 placeholder."""
        r = TruthRouter()
        sources = [
            {"answer": "V2 5 位置", "confidence": 0.7, "anchors": ["Simondon"]},
            {"answer": "Mirror", "confidence": 0.8, "anchors": ["Merleau-Ponty"]},
        ]
        for strat in [RoutingStrategy.MAJORITY, RoutingStrategy.WEIGHTED, RoutingStrategy.BEST_ANCHOR]:
            r.route("q", sources, strategy=strat)
        assert r.stats()["n_routes"] == 3

    def test_v3_7_allows_iteration(self):
        """V3.7 允许迭代 (主 13:31 鼓励尝试)."""
        r = TruthRouter()
        for i in range(5):
            r.route(f"q{i}", [{"answer": "a", "confidence": 0.5, "anchors": []}])
        assert r.stats()["n_routes"] == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])