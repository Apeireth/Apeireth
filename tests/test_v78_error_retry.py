"""v78_error_retry.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v78_error_retry import (
    V78_VERSION, RetryAttempt, RetryResult,
    exponential_backoff, V78ErrorRetry,
)


class TestV78Helpers:
    def test_exponential_backoff(self):
        delay = exponential_backoff(0)
        assert delay > 0
        delay2 = exponential_backoff(1)
        assert delay2 > delay


class TestV78:
    def test_init(self):
        er = V78ErrorRetry()
        assert er.max_attempts == 3

    def test_execute_success_first_try(self):
        er = V78ErrorRetry()
        rid = er.execute_with_retry(lambda: "ok", fn_name="test")
        assert er.results[-1].final_success is True
        assert er.results[-1].total_attempts == 1

    def test_execute_with_retry_success(self):
        er = V78ErrorRetry(max_attempts=3, backoff_base=0.01)
        attempts = [0]
        def flaky():
            attempts[0] += 1
            if attempts[0] < 2:
                raise ValueError("transient")
            return "ok"
        rid = er.execute_with_retry(flaky, fn_name="flaky")
        assert er.results[-1].final_success is True
        assert attempts[0] == 2

    def test_execute_all_attempts_fail(self):
        er = V78ErrorRetry(max_attempts=2, backoff_base=0.01)
        def always_fail():
            raise ValueError("always")
        rid = er.execute_with_retry(always_fail, fn_name="always_fail")
        assert er.results[-1].final_success is False
        assert er.results[-1].total_attempts == 2

    def test_n_success(self):
        er = V78ErrorRetry()
        er.execute_with_retry(lambda: "ok", fn_name="test1")
        er.execute_with_retry(lambda: "ok", fn_name="test2")
        assert er.n_success() == 2

    def test_average_attempts(self):
        er = V78ErrorRetry()
        er.execute_with_retry(lambda: "ok", fn_name="test")
        avg = er.average_attempts()
        assert avg == 1.0

    def test_stats(self):
        er = V78ErrorRetry()
        er.execute_with_retry(lambda: "ok", fn_name="test")
        stats = er.stats()
        assert stats["n_success"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])