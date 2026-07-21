"""v12_cross_domain_graph.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v12_cross_domain_graph import (
    V12_VERSION, GraphNode, GraphEdge, V12CrossDomainGraph,
)


class TestV12Graph:
    def test_init_empty(self):
        g = V12CrossDomainGraph()
        assert g.nodes == {}
        assert g.edges == []

    def test_add_truth_node(self):
        g = V12CrossDomainGraph()
        n = g.add_truth_node("self", "What is self?")
        assert "truth_self" in g.nodes
        assert n.node_type == "truth"

    def test_add_anchor_node(self):
        g = V12CrossDomainGraph()
        n = g.add_anchor_node("Simondon")
        assert "anchor_simondon" in g.nodes

    def test_link_truth_to_anchor(self):
        g = V12CrossDomainGraph()
        g.add_truth_node("self", "What is self?")
        g.add_anchor_node("Simondon")
        e = g.link_truth_to_anchor("self", "Simondon")
        assert e is not None
        assert e.src == "truth_self"

    def test_link_missing_truth(self):
        g = V12CrossDomainGraph()
        e = g.link_truth_to_anchor("nonexistent", "Simondon")
        assert e is None

    def test_connect_truths(self):
        g = V12CrossDomainGraph()
        g.add_truth_node("self", "What is self?")
        g.add_truth_node("truth", "What is truth?")
        e = g.connect_truths("self", "truth")
        assert e is not None

    def test_neighbors(self):
        g = V12CrossDomainGraph()
        g.add_truth_node("self", "What is self?")
        g.add_anchor_node("Simondon")
        g.link_truth_to_anchor("self", "Simondon")
        neighbors = g.neighbors("truth_self")
        assert "anchor_simondon" in neighbors

    def test_density(self):
        g = V12CrossDomainGraph()
        g.add_truth_node("self", "What is self?")
        g.add_truth_node("time", "What is time?")
        g.add_edge("truth_self", "truth_time")
        assert g.density() > 0

    def test_stats_clean(self):
        g = V12CrossDomainGraph()
        g.add_truth_node("self", "What is self?")
        g.add_anchor_node("Simondon")
        g.link_truth_to_anchor("self", "Simondon")
        stats = g.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_nodes"] == 2
        assert stats["n_edges"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])