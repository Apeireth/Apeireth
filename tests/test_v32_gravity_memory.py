"""v32_gravity_memory.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v32_gravity_memory import (
    V32_VERSION, GravityMemoryNode, GravityAttraction,
    gravity_force, euclidean_distance, tag_overlap, V32GravityMemory,
)


class TestV32Helpers:
    def test_gravity_force_basic(self):
        f = gravity_force(1.0, 1.0, 1.0)
        assert f > 0

    def test_gravity_force_zero_distance(self):
        f = gravity_force(1.0, 1.0, 0.0)
        assert f == float("inf")

    def test_euclidean_distance_same(self):
        d = euclidean_distance((0, 0, 0), (0, 0, 0))
        assert d == 0.0

    def test_euclidean_distance_3d(self):
        d = euclidean_distance((0, 0, 0), (1, 1, 1))
        assert abs(d - 1.732) < 0.01

    def test_tag_overlap(self):
        n = tag_overlap(["a", "b", "c"], ["b", "c", "d"])
        assert n == 2

    def test_tag_overlap_none(self):
        n = tag_overlap(["a"], ["b"])
        assert n == 0


class TestV32:
    def test_init(self):
        m = V32GravityMemory()
        assert m.nodes == {}

    def test_add_node(self):
        m = V32GravityMemory()
        nid = m.add_node("test", mass=2.0)
        assert nid in m.nodes

    def test_compute_attraction(self):
        m = V32GravityMemory()
        n1 = m.add_node("a", mass=2.0, position=(0, 0, 0))
        n2 = m.add_node("b", mass=1.0, position=(1, 0, 0))
        attr = m.compute_attraction(n1, n2)
        assert attr is not None
        assert attr.force > 0

    def test_compute_attraction_unknown(self):
        m = V32GravityMemory()
        assert m.compute_attraction("a", "b") is None

    def test_compute_all_attractions(self):
        m = V32GravityMemory()
        m.add_node("a", mass=1.0)
        m.add_node("b", mass=1.0)
        m.add_node("c", mass=1.0)
        attractions = m.compute_all_attractions()
        assert len(attractions) == 3  # 3 nodes = 3 pairs

    def test_field_at(self):
        m = V32GravityMemory()
        m.add_node("a", mass=2.0, position=(0, 0, 0))
        m.add_node("b", mass=1.0, position=(1, 0, 0))
        field = m.field_at((2, 0, 0))
        assert field > 0

    def test_stats(self):
        m = V32GravityMemory()
        m.add_node("a")
        stats = m.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])