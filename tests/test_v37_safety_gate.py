"""v37_safety_gate.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v37_safety_gate import (
    V37_VERSION, SafetyCheckResult, PROTECTED_PATHS,
    check_process_gate, check_sandbox_gate,
    check_evaluation_gate, check_human_gate, V37SafetyGate,
)


class TestV37Helpers:
    def test_l1_small_diff(self):
        r = check_process_gate(diff_size=50)
        assert r.passed is True

    def test_l1_large_diff(self):
        r = check_process_gate(diff_size=300)
        assert r.passed is False
        assert r.requires_human is True

    def test_l1_protected_path(self):
        r = check_process_gate(diff_size=10, file_paths=["MEMORY.md"])
        assert r.passed is False

    def test_l2_no_network(self):
        r = check_sandbox_gate(cmd=["python"])
        assert r.passed is True

    def test_l2_with_curl_blocked(self):
        r = check_sandbox_gate(cmd=["curl", "http://example.com"])
        assert r.passed is False

    def test_l2_with_curl_allowed(self):
        r = check_sandbox_gate(cmd=["curl"], allow_network=True)
        assert r.passed is True

    def test_l3_keep(self):
        r = check_evaluation_gate(prev_hqb_total=0.5, next_hqb_total=1.0)
        assert r.passed is True
        assert "keep" in r.reason

    def test_l3_revert(self):
        r = check_evaluation_gate(prev_hqb_total=0.5, next_hqb_total=-0.1)
        assert r.passed is False
        assert "revert" in r.reason

    def test_l4_no_human_needed(self):
        r = check_human_gate()
        assert r.passed is True

    def test_l4_human_required(self):
        r = check_human_gate(requires_human_pre=True, explicit_approval=True)
        assert r.passed is True

    def test_l4_human_missing(self):
        r = check_human_gate(requires_human_pre=True, explicit_approval=False)
        assert r.passed is False


class TestV37:
    def test_init(self):
        g = V37SafetyGate()
        assert g.checks == []

    def test_run_all_layers(self):
        g = V37SafetyGate()
        checks = g.run_all_layers()
        assert len(checks) == 4

    def test_is_safe_all_pass(self):
        g = V37SafetyGate()
        g.run_all_layers(diff_size=50, file_paths=["v36.py"], cmd=["python"])
        assert g.is_safe() is True

    def test_stats(self):
        g = V37SafetyGate()
        g.run_all_layers()
        stats = g.stats()
        assert stats["n_checks"] == 4


if __name__ == "__main__":
    pytest.main([__file__, "-v"])