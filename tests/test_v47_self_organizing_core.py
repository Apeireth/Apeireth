"""v47_self_organizing_core.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v47_self_organizing_core import (
    V47_VERSION, AutopoieticCycle, RequisiteVariety,
    V47SelfOrganizingCore,
)


class TestV47:
    def test_init(self):
        c = V47SelfOrganizingCore()
        assert c.cycles == []

    def test_create_autopoietic_cycle(self):
        c = V47SelfOrganizingCore()
        cycle = c.create_autopoietic_cycle(
            components=["a", "b"],
            processes=["x", "y"],
            boundary="test",
        )
        assert cycle.is_autopoietic is True

    def test_create_autopoietic_cycle_no_boundary(self):
        c = V47SelfOrganizingCore()
        cycle = c.create_autopoietic_cycle(
            components=["a"],
            processes=["x"],
            boundary="",
        )
        assert cycle.is_autopoietic is False

    def test_requisite_variety_satisfied(self):
        c = V47SelfOrganizingCore()
        var = c.check_requisite_variety(environment_variety=5, system_variety=10)
        assert var.satisfied is True

    def test_requisite_variety_unsatisfied(self):
        c = V47SelfOrganizingCore()
        var = c.check_requisite_variety(environment_variety=10, system_variety=5)
        assert var.satisfied is False

    def test_add_autocatalytic_set(self):
        c = V47SelfOrganizingCore()
        c.add_autocatalytic_set("r1", ["a", "b"])
        assert "r1" in c.autocatalytic_components

    def test_is_raf(self):
        c = V47SelfOrganizingCore()
        c.add_autocatalytic_set("r1", ["a"])
        assert c.is_raf("r1") is True

    def test_is_raf_unknown(self):
        c = V47SelfOrganizingCore()
        assert c.is_raf("unknown") is False

    def test_stats(self):
        c = V47SelfOrganizingCore()
        c.create_autopoietic_cycle(["a"], ["x"], "test")
        stats = c.stats()
        assert stats["n_cycles"] == 1
        assert stats["n_autopoietic"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])