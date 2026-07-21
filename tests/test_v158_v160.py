"""V158-V160 真生产 tests (主 22:30 一次推完 20+)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest

import apeireth.v158_anysearch_index as v158_mod
import apeireth.v159_v01_formula_measure as v159_mod
import apeireth.v160_hqb_4dims as v160_mod

V158AnySearchIndex = v158_mod.V158AnySearchIndex
AnySearchFinding = v158_mod.AnySearchFinding
V159V01FormulaMeasure = v159_mod.V159V01FormulaMeasure
V21_V01_FORMULA = v159_mod.V21_V01_FORMULA
V160HQB4Dimensions = v160_mod.V160HQB4Dimensions


class TestV158V160Batch:
    """V158-V160 真生产 batch tests (主 22:30)."""

    # V158 AnySearch Index
    def test_v158_add_finding(self):
        idx = V158AnySearchIndex()
        fid = idx.add_finding(query="test", title="t", content="c")
        assert fid in idx.findings

    def test_v158_search_by_query(self):
        idx = V158AnySearchIndex()
        idx.add_finding(query="q1", title="t1")
        idx.add_finding(query="q2", title="t2")
        assert len(idx.search_by_query("q1")) == 1

    def test_v158_search_by_source(self):
        idx = V158AnySearchIndex()
        idx.add_finding(query="q", title="t", source="anysearch")
        idx.add_finding(query="q", title="t2", source="bocha")
        assert len(idx.search_by_source("anysearch")) == 1
        assert len(idx.search_by_source("bocha")) == 1

    def test_v158_top_findings(self):
        idx = V158AnySearchIndex()
        idx.add_finding(query="q1", title="t1", relevance=0.5)
        idx.add_finding(query="q2", title="t2", relevance=0.9)
        top = idx.top_findings(1)
        assert idx.findings[top[0]].title == "t2"

    def test_v158_stats(self):
        idx = V158AnySearchIndex(); stats = idx.stats()
        assert "n_findings" in stats
        assert stats["version"] == v158_mod.V158_VERSION

    # V159 V0.1 Formula Measure
    def test_v159_formula_has_8_components(self):
        assert len(V21_V01_FORMULA) == 8

    def test_v159_weights_sum(self):
        total = sum(V21_V01_FORMULA.values())
        assert abs(total - 1.0) < 0.001

    def test_v159_measure(self):
        m = V159V01FormulaMeasure()
        result = m.measure({k: 0.85 for k in V21_V01_FORMULA})
        assert result["level"] == "ASI"
        assert result["total"] >= 0.7

    def test_v159_measure_low(self):
        m = V159V01FormulaMeasure()
        result = m.measure({k: 0.0 for k in V21_V01_FORMULA})
        assert result["level"] == "ANI"

    def test_v159_stats(self):
        m = V159V01FormulaMeasure(); m.measure({k: 0.5 for k in V21_V01_FORMULA})
        stats = m.stats()
        assert stats["n_measurements"] == 1
        assert stats["version"] == v159_mod.V159_VERSION

    # V160 HQB 4 Dimensions
    def test_v160_measure_sc(self):
        hqb = V160HQB4Dimensions()
        sc = hqb.measure_sc([0.85, 0.86, 0.84, 0.85])
        assert 0 < sc <= 1.0

    def test_v160_measure_nr(self):
        hqb = V160HQB4Dimensions()
        nr = hqb.measure_nr([1.0, 1.0, 1.0], [0.9, 0.95, 0.85])
        assert 0 < nr <= 1.0

    def test_v160_measure_ev(self):
        hqb = V160HQB4Dimensions()
        ev = hqb.measure_ev(0.5, 0.8)
        assert 0 < ev <= 1.0

    def test_v160_measure_cdt(self):
        hqb = V160HQB4Dimensions()
        cdt = hqb.measure_cdt({"a": 0.8, "b": 0.9, "c": 0.7})
        assert abs(cdt - 0.8) < 0.01

    def test_v160_measure_all(self):
        hqb = V160HQB4Dimensions()
        result = hqb.measure_all(
            runs=[0.85, 0.86, 0.84], noisy=[0.83, 0.84, 0.82],
            prev=0.7, next_s=0.85,
            domain_scores={"a": 0.8, "b": 0.7},
        )
        assert 0 < result["total"] <= 1.0
        assert "sc" in result and "nr" in result and "ev" in result and "cdt" in result

    def test_v160_stats(self):
        hqb = V160HQB4Dimensions(); hqb.measure_all(
            runs=[0.8], noisy=[0.8], prev=0.5, next_s=0.6,
            domain_scores={"a": 0.7})
        stats = hqb.stats()
        assert stats["n_measurements"] == 1
        assert stats["version"] == v160_mod.V160_VERSION