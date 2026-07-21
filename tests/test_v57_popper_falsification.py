"""v57_popper_falsification.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v57_popper_falsification import (
    V57_VERSION, ScientificHypothesis, FalsificationAttempt,
    V57PopperFalsification,
)


class TestV57:
    def test_init(self):
        pf = V57PopperFalsification()
        assert pf.hypotheses == {}

    def test_propose_hypothesis(self):
        pf = V57PopperFalsification()
        hid = pf.propose_hypothesis("test", "domain")
        assert hid in pf.hypotheses

    def test_falsify_attempt_survives(self):
        pf = V57PopperFalsification()
        hid = pf.propose_hypothesis("test", "domain")
        aid = pf.falsify_attempt(hid, "consistent evidence")
        assert aid != ""
        assert pf.hypotheses[hid].survived_attempts == 1

    def test_falsify_attempt_falsified(self):
        pf = V57PopperFalsification()
        hid = pf.propose_hypothesis("test", "domain")
        pf.falsify_attempt(hid, "this is falsified by experiment")
        assert pf.hypotheses[hid].is_corroborated is False

    def test_corroboration_after_3(self):
        pf = V57PopperFalsification()
        hid = pf.propose_hypothesis("test", "domain")
        for _ in range(3):
            pf.falsify_attempt(hid, "consistent evidence")
        assert pf.hypotheses[hid].is_corroborated is True

    def test_is_scientific_falsifiable(self):
        pf = V57PopperFalsification()
        hid = pf.propose_hypothesis("test", "domain", falsifiable=True)
        assert pf.is_scientific(hid) is True

    def test_is_scientific_not_falsifiable(self):
        pf = V57PopperFalsification()
        hid = pf.propose_hypothesis("test", "domain", falsifiable=False)
        assert pf.is_scientific(hid) is False

    def test_stats(self):
        pf = V57PopperFalsification()
        pf.propose_hypothesis("h1", "d1")
        stats = pf.stats()
        assert stats["n_hypotheses"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])