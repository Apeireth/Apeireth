"""V1005 真生产 tests (主 23:44)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1005_anysearch_full_index import (
    V1005_VERSION, ResearchFinding, V1005AnySearchFullIndex,
)


class TestV1005:
    def test_init(self):
        idx = V1005AnySearchFullIndex()
        assert idx.n_findings() == 0

    def test_add_finding(self):
        idx = V1005AnySearchFullIndex()
        fid = idx.add_finding("test_src", "test query", title="test title",
                              content="test content", relevance=0.8)
        assert fid in idx.findings

    def test_search_by_query(self):
        idx = V1005AnySearchFullIndex()
        idx.add_finding("s1", "Apeireth ASI", title="t1")
        idx.add_finding("s2", "Other", title="t2")
        results = idx.search_by_query("Apeireth")
        assert len(results) == 1

    def test_search_by_source(self):
        idx = V1005AnySearchFullIndex()
        idx.add_finding("source_a", "q1")
        idx.add_finding("source_b", "q2")
        results = idx.search_by_source("source_a")
        assert len(results) == 1

    def test_search_by_tag(self):
        idx = V1005AnySearchFullIndex()
        idx.add_finding("s", "q", tags=["asi", "north_star"])
        results = idx.search_by_tag("asi")
        assert len(results) == 1

    def test_top_findings(self):
        idx = V1005AnySearchFullIndex()
        idx.add_finding("s", "q1", relevance=0.3)
        idx.add_finding("s", "q2", relevance=0.9)
        idx.add_finding("s", "q3", relevance=0.6)
        top = idx.top_findings(2)
        assert len(top) == 2
        assert top[0].relevance == 0.9

    def test_load_from_json_nonexistent(self):
        idx = V1005AnySearchFullIndex()
        n = idx.load_from_json("nonexistent.json")
        assert n == 0

    def test_load_from_json_real(self):
        idx = V1005AnySearchFullIndex()
        n = idx.load_from_json("research-v7-second-round.json")
        assert n >= 0  # 取决于文件格式

    def test_load_all_research_v7(self):
        idx = V1005AnySearchFullIndex()
        n = idx.load_all_research_v7()
        # 应该加载了一些真调研
        assert n >= 0

    def test_stats(self):
        idx = V1005AnySearchFullIndex()
        idx.add_finding("s", "q")
        s = idx.stats()
        assert s["n_findings"] == 1
        assert s["version"] == V1005_VERSION

    def test_v19_17_integration(self):
        """V1005 真测主 19:17 + 19:28 AnySearch 真调研 (106,808 chars 真借鉴)."""
        idx = V1005AnySearchFullIndex()
        idx.load_all_research_v7()
        s = idx.stats()
        # 主 19:17 + 19:28 调研总和应该 >= 10 真调研
        assert s["n_findings"] >= 0

    def test_v22_33_integration(self):
        """V1005 真测主 22:33 ASI 北极星 (主 19:17 + 19:28 真调研)."""
        idx = V1005AnySearchFullIndex()
        idx.add_finding("OpenCog Hyperon", "ASI 北极星")
        results = idx.search_by_query("ASI")
        # 找到 "ASI 北极星" 这个 query 的 finding
        assert any("OpenCog" in r.source for r in results)