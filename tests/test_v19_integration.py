"""v19_integration.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v19_integration import V19_VERSION, IntegrationResult, V19Integration


class TestV19:
    def test_init(self):
        s = V19Integration()
        assert s.results == []

    def test_integration_test_v3_v9(self):
        s = V19Integration()
        r = s.integration_test_v3_v9()
        assert r.n_passed >= 4
        assert r.n_failed == 0

    def test_integration_test_v11_v13(self):
        s = V19Integration()
        r = s.integration_test_v11_v13()
        assert r.n_passed >= 2
        assert r.n_failed == 0

    def test_integration_test_full_chain(self):
        s = V19Integration()
        r = s.integration_test_full_chain()
        assert r.n_passed >= 1
        assert r.n_failed == 0

    def test_run_all(self):
        s = V19Integration()
        results = s.run_all()
        assert len(results) == 3
        assert all(r.n_failed == 0 for r in results)

    def test_stats(self):
        s = V19Integration()
        s.run_all()
        stats = s.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["pass_rate"] > 0.9


if __name__ == "__main__":
    pytest.main([__file__, "-v"])