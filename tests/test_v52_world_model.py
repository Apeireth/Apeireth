"""v52_world_model.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v52_world_model import (
    V52_VERSION, WorldState, WorldPrediction, V52WorldModel,
)


class TestV52:
    def test_init(self):
        wm = V52WorldModel()
        assert wm.n_states() == 0

    def test_add_state(self):
        wm = V52WorldModel()
        sid = wm.add_state("obs")
        assert wm.n_states() == 1

    def test_predict_next(self):
        wm = V52WorldModel()
        s1 = wm.add_state("o1")
        pid = wm.predict_next(s1, "o2")
        assert wm.n_predictions() == 1

    def test_average_uncertainty(self):
        wm = V52WorldModel()
        s1 = wm.add_state("o1")
        wm.predict_next(s1, "o2", uncertainty=0.3)
        avg = wm.average_uncertainty()
        assert abs(avg - 0.3) < 0.01

    def test_stats(self):
        wm = V52WorldModel()
        stats = wm.stats()
        assert stats["latent_dim"] == 64
        assert stats["hidden_dim"] == 128


if __name__ == "__main__":
    pytest.main([__file__, "-v"])