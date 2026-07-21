"""v33_fact_timeline.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v33_fact_timeline import (
    V33_VERSION, FactEntry, PyramidLayer, FactTimelineEntry,
    V33FactTimeline, V33ResidualPyramid,
)


class TestV33FactTimeline:
    def test_init(self):
        ft = V33FactTimeline()
        assert ft.facts == {}

    def test_assert_fact(self):
        ft = V33FactTimeline()
        fid = ft.assert_fact("test", source="me", confidence=0.9)
        assert fid in ft.facts

    def test_assert_with_invalidate(self):
        ft = V33FactTimeline()
        f1 = ft.assert_fact("old fact")
        f2 = ft.assert_fact("new fact", invalidates=[f1])
        assert f1 in ft.invalidated

    def test_query_at(self):
        import time
        ft = V33FactTimeline()
        f1 = ft.assert_fact("fact A")
        time.sleep(0.05)
        t_mid = time.time()
        time.sleep(0.05)
        f2 = ft.assert_fact("fact B")
        past_facts = ft.query_at(t_mid)
        assert len(past_facts) == 1
        assert past_facts[0].fact_id == f1

    def test_stats(self):
        ft = V33FactTimeline()
        ft.assert_fact("a")
        stats = ft.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


class TestV33ResidualPyramid:
    def test_init(self):
        rp = V33ResidualPyramid()
        assert rp.layers == {}

    def test_add_layer(self):
        rp = V33ResidualPyramid()
        lid = rp.add_layer("test", level=0)
        assert lid in rp.layers

    def test_add_layer_with_parent(self):
        rp = V33ResidualPyramid()
        l0 = rp.add_layer("L0", level=0)
        l1 = rp.add_layer("L1", level=1, parent_layer_id=l0)
        assert l0 in rp.layers[l1].parent_layer_id or rp.layers[l1].parent_layer_id == l0

    def test_residual_between(self):
        rp = V33ResidualPyramid()
        l0 = rp.add_layer("parent", level=0)
        l1 = rp.add_layer("child", level=1, parent_layer_id=l0)
        residual = rp.residual_between(l0, l1)
        assert "RESIDUAL" in residual

    def test_residual_between_unknown(self):
        rp = V33ResidualPyramid()
        assert rp.residual_between("a", "b") == ""

    def test_stats(self):
        rp = V33ResidualPyramid()
        rp.add_layer("test", level=0)
        stats = rp.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])