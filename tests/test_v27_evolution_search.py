"""v27_evolution_search.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v27_evolution_search import (
    V27_VERSION, EvolutionCandidate, mutate_payload, V27EvolutionSearch,
)


class TestV27Helpers:
    def test_mutate_payload(self):
        new = mutate_payload({"x": 1.0})
        assert new["x"] == 1.1

    def test_mutate_new_key(self):
        new = mutate_payload({})
        assert "x" in new


class TestV27:
    def test_init(self):
        s = V27EvolutionSearch()
        assert s.candidates == []

    def test_genesis(self):
        s = V27EvolutionSearch()
        c = s.genesis()
        assert c.generation == 0

    def test_mutate(self):
        s = V27EvolutionSearch()
        parent = s.genesis(seed_payload={"x": 0.5})
        child = s.mutate(parent)
        assert child.parent_id == parent.candidate_id
        assert child.generation == parent.generation + 1

    def test_falsify(self):
        s = V27EvolutionSearch()
        c = EvolutionCandidate(candidate_id="x", fitness=-1.0)
        assert s.falsify(c) is True

    def test_falsify_not(self):
        s = V27EvolutionSearch()
        c = EvolutionCandidate(candidate_id="x", fitness=0.5)
        assert s.falsify(c) is False

    def test_best_empty(self):
        s = V27EvolutionSearch()
        with pytest.raises(ValueError):
            s.best()

    def test_best(self):
        s = V27EvolutionSearch()
        s.evolve_n_generations(n=3)
        b = s.best()
        assert isinstance(b, EvolutionCandidate)

    def test_evolve_n_generations(self):
        s = V27EvolutionSearch()
        results = s.evolve_n_generations(n=5, seed_payload={"x": 0.0})
        assert len(results) > 0

    def test_stats(self):
        s = V27EvolutionSearch()
        s.evolve_n_generations(n=3)
        stats = s.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])