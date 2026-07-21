"""v55_ultimate_integration.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v55_ultimate_integration import (
    V55_VERSION, UltimateIntegrationResult, V55UltimateIntegration,
)


class TestV55:
    def test_init(self):
        i = V55UltimateIntegration()
        assert i.integrations == []

    def test_run_full_integration(self):
        i = V55UltimateIntegration()
        r = i.run_full_integration()
        assert r.v54_asi_level in ("ANI", "AGI", "ASI")
        assert r.n_modules_integrated == 12
        assert r.integration_completeness == 1.0

    def test_v50_emergence(self):
        i = V55UltimateIntegration()
        r = i.run_full_integration()
        assert r.v50_emergence_score > 0.5

    def test_v54_asi(self):
        i = V55UltimateIntegration()
        r = i.run_full_integration()
        assert r.v54_asi_total > 0.7  # ASI level
        assert r.v54_asi_level == "ASI"

    def test_stats(self):
        i = V55UltimateIntegration()
        i.run_full_integration()
        stats = i.stats()
        assert stats["n_integrations"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])