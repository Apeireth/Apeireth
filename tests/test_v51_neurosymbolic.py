"""v51_neurosymbolic.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v51_neurosymbolic import (
    V51_VERSION, LogicOp, SymbolicExpression, NeuralPrediction, V51NeuroSymbolic,
)


class TestV51:
    def test_init(self):
        c = V51NeuroSymbolic()
        assert c.expressions == {}

    def test_add_expression(self):
        c = V51NeuroSymbolic()
        eid = c.add_expression(LogicOp.AND)
        assert eid in c.expressions

    def test_do_intervention(self):
        c = V51NeuroSymbolic()
        iid = c.do_intervention("X", 5)
        assert iid.startswith("do_")

    def test_neural_symbolic_predict(self):
        c = V51NeuroSymbolic()
        pid = c.neural_symbolic_predict({"x": 1.0}, "proof", confidence=0.9)
        assert pid.startswith("p_")

    def test_stats(self):
        c = V51NeuroSymbolic()
        c.add_expression(LogicOp.AND)
        stats = c.stats()
        assert stats["n_expressions"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])