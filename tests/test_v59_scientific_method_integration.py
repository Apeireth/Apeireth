"""v59_scientific_method_integration.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v59_scientific_method_integration import (
    V59_VERSION, ScientificMethod, ResearchProgram, ScientificProgress,
    V59ScientificMethodIntegration,
)


class TestV59:
    def test_init(self):
        sm = V59ScientificMethodIntegration()
        assert sm.programs == {}

    def test_create_research_program(self):
        sm = V59ScientificMethodIntegration()
        pid = sm.create_research_program(
            "test", hard_core=["c1"], protective_belt=["p1"]
        )
        assert pid in sm.programs

    def test_evaluate_program_progressive(self):
        sm = V59ScientificMethodIntegration()
        pid = sm.create_research_program("t", hard_core=[], protective_belt=[])
        result = sm.evaluate_program(pid, problem_solving=10, anomalies_unresolved=3)
        assert result is True
        assert sm.programs[pid].is_progressive is True

    def test_evaluate_program_regressive(self):
        sm = V59ScientificMethodIntegration()
        pid = sm.create_research_program("t", hard_core=[], protective_belt=[])
        result = sm.evaluate_program(pid, problem_solving=3, anomalies_unresolved=10)
        assert result is False

    def test_popper_workflow(self):
        sm = V59ScientificMethodIntegration()
        r = sm.run_popper_falsification_workflow("h", "d", n_evidence=5)
        assert r["is_scientific"] is True
        assert r["n_survived"] == 5

    def test_kuhn_workflow(self):
        sm = V59ScientificMethodIntegration()
        r = sm.run_kuhn_paradigm_workflow("p", "d", n_anomalies=5)
        assert r["phase"] in ("pre_paradigm", "paradigm", "normal_science", "crisis", "revolution")

    def test_stats(self):
        sm = V59ScientificMethodIntegration()
        sm.create_research_program("t", hard_core=[], protective_belt=[])
        stats = sm.stats()
        assert stats["n_programs"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])