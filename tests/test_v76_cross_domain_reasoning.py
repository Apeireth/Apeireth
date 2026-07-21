"""v76_cross_domain_reasoning.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v76_cross_domain_reasoning import (
    V76_VERSION, ReasoningStep, ReasoningResult, V76CrossDomainReasoning,
)


class TestV76:
    def test_init(self):
        cr = V76CrossDomainReasoning()
        assert cr.reasonings == []

    def test_reason(self):
        cr = V76CrossDomainReasoning()
        cr.query_engine.add_document("d1", "test content")
        r = cr.reason("test")
        assert len(r.steps) >= 2
        assert r.final_answer is not None

    def test_reason_multiple(self):
        cr = V76CrossDomainReasoning()
        cr.reason("q1")
        cr.reason("q2")
        assert cr.n_reasonings() == 2

    def test_average_confidence(self):
        cr = V76CrossDomainReasoning()
        cr.reason("test")
        avg = cr.average_confidence()
        assert 0.0 <= avg <= 1.0

    def test_stats(self):
        cr = V76CrossDomainReasoning()
        cr.reason("test")
        stats = cr.stats()
        assert stats["n_reasonings"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])