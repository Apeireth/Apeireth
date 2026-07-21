"""v79_observation_logging.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v79_observation_logging import (
    V79_VERSION, LogLevel, LogEntry, V79ObservationLogging,
)


class TestV79:
    def test_init(self):
        log = V79ObservationLogging()
        assert log.logs == []

    def test_log_basic(self):
        log = V79ObservationLogging()
        eid = log.log(LogLevel.INFO, "test")
        assert len(log.logs) == 1

    def test_log_with_trace(self):
        log = V79ObservationLogging()
        trace = "trace_123"
        log.log(LogLevel.INFO, "test", trace_id=trace)
        assert trace in log.trace_index

    def test_log_with_metadata(self):
        log = V79ObservationLogging()
        eid = log.log(LogLevel.INFO, "test", metadata={"key": "value"})
        assert log.logs[-1].metadata["key"] == "value"

    def test_n_errors(self):
        log = V79ObservationLogging()
        log.log(LogLevel.ERROR, "err1")
        log.log(LogLevel.ERROR, "err2")
        log.log(LogLevel.INFO, "ok")
        assert log.n_errors() == 2

    def test_n_traces(self):
        log = V79ObservationLogging()
        log.log(LogLevel.INFO, "t1", trace_id="trace_1")
        log.log(LogLevel.INFO, "t2", trace_id="trace_2")
        assert log.n_traces() == 2

    def test_stats(self):
        log = V79ObservationLogging()
        log.log(LogLevel.INFO, "test")
        stats = log.stats()
        assert stats["n_logs"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])