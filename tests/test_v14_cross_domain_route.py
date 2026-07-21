"""v14_cross_domain_route.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v14_cross_domain_route import V14_VERSION, CrossDomainRoute, V14CrossDomainRouter


class TestV14:
    def test_init(self):
        r = V14CrossDomainRouter()
        assert r.routes == []

    def test_route(self):
        r = V14CrossDomainRouter()
        route = r.route("q", {"self": ["Simondon", "Merleau-Ponty"]}, "self")
        assert route.n_anchors_crossed == 2
        assert route.total_score == 0.5

    def test_route_unknown(self):
        r = V14CrossDomainRouter()
        route = r.route("q", {}, "unknown")
        assert route.n_anchors_crossed == 0
        assert route.total_score == 0.0

    def test_route_max_score(self):
        r = V14CrossDomainRouter()
        route = r.route("q", {"t": ["a", "b", "c", "d"]}, "t")
        assert route.total_score == 1.0

    def test_stats(self):
        r = V14CrossDomainRouter()
        r.route("q", {"t": ["a"]}, "t")
        stats = r.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])