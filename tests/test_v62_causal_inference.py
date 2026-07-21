"""v62_causal_inference.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v62_causal_inference import (
    V62_VERSION, CausalLevel, CausalGraph, FreeEnergyEstimate,
    compute_free_energy, V62CausalInference,
)


class TestV62Helpers:
    def test_compute_free_energy(self):
        fe = compute_free_energy(0.5, 0.3)
        assert fe.free_energy == pytest.approx(0.8, abs=0.01)


class TestV62:
    def test_init(self):
        ci = V62CausalInference()
        assert ci.causal_graphs == {}

    def test_create_causal_graph(self):
        ci = V62CausalInference()
        gid = ci.create_causal_graph(
            nodes=["X", "Y"],
            edges=[],
        )
        assert gid in ci.causal_graphs

    def test_intervene(self):
        ci = V62CausalInference()
        iid = ci.intervene("X", 1.0)
        assert iid.startswith("do_")

    def test_compute_free_energy_method(self):
        ci = V62CausalInference()
        fe_id = ci.compute_free_energy(0.5, 0.3)
        assert fe_id.startswith("fe_")

    def test_n_graphs(self):
        ci = V62CausalInference()
        ci.create_causal_graph(nodes=["a"], edges=[])
        assert ci.n_graphs() == 1

    def test_n_interventions(self):
        ci = V62CausalInference()
        ci.intervene("X", 1)
        assert ci.n_interventions() == 1

    def test_stats(self):
        ci = V62CausalInference()
        ci.create_causal_graph(nodes=["a"], edges=[])
        ci.intervene("X", 1)
        stats = ci.stats()
        assert stats["n_graphs"] == 1
        assert stats["n_interventions"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])