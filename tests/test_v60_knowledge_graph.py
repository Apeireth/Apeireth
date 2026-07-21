"""v60_knowledge_graph.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v60_knowledge_graph import (
    V60_VERSION, KGNode, KGEdge, V60KnowledgeGraph,
)


class TestV60:
    def test_init(self):
        kg = V60KnowledgeGraph()
        assert kg.nodes == {}

    def test_add_node(self):
        kg = V60KnowledgeGraph()
        nid = kg.add_node("test")
        assert nid in kg.nodes

    def test_add_edge(self):
        kg = V60KnowledgeGraph()
        n1 = kg.add_node("a")
        n2 = kg.add_node("b")
        eid = kg.add_edge(n1, n2)
        assert eid != ""

    def test_add_edge_invalid(self):
        kg = V60KnowledgeGraph()
        eid = kg.add_edge("nonexistent", "also_nonexistent")
        assert eid == ""

    def test_query_related(self):
        kg = V60KnowledgeGraph()
        n1 = kg.add_node("a")
        n2 = kg.add_node("b")
        n3 = kg.add_node("c")
        kg.add_edge(n1, n2)
        kg.add_edge(n2, n3)
        related = kg.query_related(n1)
        assert n2 in related
        assert n3 in related

    def test_query_related_unknown(self):
        kg = V60KnowledgeGraph()
        related = kg.query_related("nonexistent")
        assert related == []

    def test_stats(self):
        kg = V60KnowledgeGraph()
        kg.add_node("a")
        stats = kg.stats()
        assert stats["n_nodes"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])