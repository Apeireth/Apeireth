"""v58_kuhn_paradigm.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v58_kuhn_paradigm import (
    V58_VERSION, KuhnPhase, KuhnParadigm, V58KuhnParadigm,
)


class TestV58:
    def test_init(self):
        k = V58KuhnParadigm()
        assert k.paradigms == {}

    def test_create_paradigm(self):
        k = V58KuhnParadigm()
        pid = k.create_paradigm("LLM-as-Agent", "AI")
        assert pid in k.paradigms

    def test_add_anomaly_to_paradigm(self):
        k = V58KuhnParadigm()
        pid = k.create_paradigm("test", "d", crisis_threshold=5)
        k.add_anomaly(pid)
        assert k.paradigms[pid].phase == KuhnPhase.PARADIGM

    def test_add_anomaly_reach_crisis(self):
        k = V58KuhnParadigm()
        pid = k.create_paradigm("test", "d", crisis_threshold=2)
        k.add_anomaly(pid)
        k.add_anomaly(pid)
        k.add_anomaly(pid)
        assert k.paradigms[pid].phase == KuhnPhase.CRISIS

    def test_solve_puzzle(self):
        k = V58KuhnParadigm()
        pid = k.create_paradigm("test", "d")
        k.solve_puzzle(pid)
        assert k.paradigms[pid].puzzle_solvers == 1

    def test_trigger_revolution(self):
        k = V58KuhnParadigm()
        p1 = k.create_paradigm("old", "AI", crisis_threshold=1)
        k.add_anomaly(p1)  # crisis
        new_id = k.trigger_revolution(p1, "new", "AI")
        assert new_id != ""
        assert k.paradigms[new_id].phase == KuhnPhase.PARADIGM

    def test_n_paradigms(self):
        k = V58KuhnParadigm()
        k.create_paradigm("p1", "d")
        k.create_paradigm("p2", "d")
        assert k.n_paradigms() == 2

    def test_stats(self):
        k = V58KuhnParadigm()
        k.create_paradigm("p1", "d")
        stats = k.stats()
        assert stats["n_paradigms"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])