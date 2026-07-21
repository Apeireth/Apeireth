"""v68_query_engine.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v68_query_engine import (
    V68_VERSION, Query, QueryResult, V68QueryEngine,
)


class TestV68:
    def test_init(self):
        qe = V68QueryEngine()
        assert qe.documents == {}

    def test_add_document(self):
        qe = V68QueryEngine()
        qe.add_document("d1", "content")
        assert qe.n_documents() == 1

    def test_execute_query(self):
        qe = V68QueryEngine()
        qe.add_document("d1", "Apeireth ASI")
        qe.add_document("d2", "VCP VCP")
        r = qe.execute_query("fulltext", text="Apeireth")
        assert r.n_results == 1
        assert "d1" in r.payload

    def test_execute_query_no_match(self):
        qe = V68QueryEngine()
        qe.add_document("d1", "content")
        r = qe.execute_query("fulltext", text="nonexistent")
        assert r.n_results == 0

    def test_n_queries(self):
        qe = V68QueryEngine()
        qe.execute_query("fulltext", text="x")
        qe.execute_query("semantic", text="y")
        assert qe.n_queries() == 2

    def test_stats(self):
        qe = V68QueryEngine()
        qe.add_document("d1", "test")
        stats = qe.stats()
        assert stats["n_documents"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])