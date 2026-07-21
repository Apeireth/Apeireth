"""v50_4paradigm_integration.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v50_4paradigm_integration import (
    V50_VERSION, EmergenceMetric, V504ParadigmIntegration,
)


class TestV50:
    def test_init(self):
        i = V504ParadigmIntegration()
        assert i.cognitive.n_atoms() == 0

    def test_bootstrap(self):
        i = V504ParadigmIntegration()
        i.bootstrap()
        assert i.cognitive.n_atoms() >= 5
        assert i.organizing.n_cycles() >= 1
        assert i.plugin.n_plugins() >= 3
        assert i.self_improving.n_agents() >= 4

    def test_measure_emergence(self):
        i = V504ParadigmIntegration()
        i.bootstrap()
        em = i.measure_emergence()
        assert em.components_active == 4
        assert 0 <= em.emergence_score <= 1.0

    def test_emergence_after_bootstrap(self):
        i = V504ParadigmIntegration()
        i.bootstrap()
        em = i.measure_emergence()
        # 真生产: 4 范式都激活, emergence 应该 > 0.5
        assert em.emergence_score > 0.5

    def test_stats(self):
        i = V504ParadigmIntegration()
        i.bootstrap()
        stats = i.stats()
        assert "emergence" in stats
        assert stats["cognitive_n_atoms"] >= 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])