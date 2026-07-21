"""v70_integration_test_suite.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v70_integration_test_suite import (
    V70_VERSION, IntegrationTest, V70IntegrationTestSuite,
)


class TestV70:
    def test_init(self):
        suite = V70IntegrationTestSuite()
        assert suite.tests == []

    def test_run_integration_tests(self):
        suite = V70IntegrationTestSuite()
        tests = suite.run_integration_tests()
        assert len(tests) == 5

    def test_all_pass(self):
        suite = V70IntegrationTestSuite()
        suite.run_integration_tests()
        # 真生产: 跨模块真测试
        assert suite.n_failed() == 0

    def test_stats(self):
        suite = V70IntegrationTestSuite()
        suite.run_integration_tests()
        stats = suite.stats()
        assert stats["n_tests"] == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])