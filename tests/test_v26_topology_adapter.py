"""v26_topology_adapter.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v26_topology_adapter import (
    V26_VERSION, TopologyNode, TopologyAdapterResult,
    compute_klein_index, V26TopologyAdapter,
)


class TestV26Helpers:
    def test_klein_index_zero(self):
        assert compute_klein_index(0, 10) == 0.0

    def test_klein_index_full(self):
        assert compute_klein_index(5, 5) == 1.0

    def test_klein_index_partial(self):
        idx = compute_klein_index(2, 8)
        assert 0.0 < idx < 1.0

    def test_klein_index_empty(self):
        assert compute_klein_index(0, 0) == 0.0


class TestV26:
    def test_init(self):
        a = V26TopologyAdapter()
        assert a.nodes == {}

    def test_add_node(self):
        a = V26TopologyAdapter()
        nid = a.add_node("test")
        assert nid in a.nodes

    def test_link(self):
        a = V26TopologyAdapter()
        n1 = a.add_node("a")
        n2 = a.add_node("b")
        a.link(n1, n2)
        assert n2 in a.nodes[n1].neighbors

    def test_link_no_duplicate(self):
        a = V26TopologyAdapter()
        n1 = a.add_node("a")
        n2 = a.add_node("b")
        a.link(n1, n2)
        a.link(n1, n2)
        assert len(a.nodes[n1].neighbors) == 1

    def test_klein_adapt(self):
        a = V26TopologyAdapter()
        n1 = a.add_node("self_ref", is_self_referential=True)
        a.klein_adapt(n1)
        assert a.nodes[n1].is_self_referential is True

    def test_measure_empty(self):
        a = V26TopologyAdapter()
        m = a.measure()
        assert m.n_nodes == 0

    def test_measure_full(self):
        a = V26TopologyAdapter()
        n1 = a.add_node("1", is_self_referential=True)
        n2 = a.add_node("2")
        n3 = a.add_node("3")
        a.link(n1, n2)
        a.link(n2, n3)
        a.link(n3, n1)
        a.klein_adapt(n1)
        m = a.measure()
        assert m.n_nodes == 3
        # 3 边 (n1→n2, n2→n3, n3→n1) + 1 自指边 = 4 neighbors total, 2 unique undirected edges
        assert m.n_edges >= 2
        assert m.n_self_refs == 1
        assert m.klein_index > 0

    def test_stats(self):
        a = V26TopologyAdapter()
        a.add_node("1")
        stats = a.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])