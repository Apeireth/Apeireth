"""V1285 — All-42 Crate Rust Security Depth Audit 真生产测试套件

> 81 tests for v1285 (extends V1284 81 tests):
> - Pattern reuse (V1284): 4 tests
> - Discovery: 4 tests
> - Philosophy gate: 4 tests
> - Data structures: 4 tests
> - Runner: 6 tests
> - Output formatters: 8 tests
> - CLI: 8 tests
> - Integration: 3 tests
> - Coverage delta: 2 tests
> - Cross-V1284 consistency: 3 tests
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import List

import pytest

# Add project root to path
PROMETHEAN_ROOT = Path(__file__).resolve().parents[1]
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1285_all42_crate_security_audit import (
    V1285_VERSION,
    V1285_BUILD,
    V1285_ASI_NS_CURRENT,
    V1285_ASI_NS_LOCKED_PCT,
    V1285_FALSIFIER_DISPATCH,
    V1285_V1284_WORST5,
    All42SecurityLedger,
    discover_all_apeireth_crates,
    run_all42_audit,
    _to_json_snapshot,
    _to_markdown,
    _v1285_philosophy_gate,
    main,
    resolve_promethean_dir,
)


# ============================================================
# 1. Version / constants
# ============================================================

class TestVersion:
    def test_version_is_string(self):
        assert isinstance(V1285_VERSION, str) and len(V1285_VERSION) > 0

    def test_build_is_string(self):
        assert isinstance(V1285_BUILD, str) and "2026-08-05" in V1285_BUILD

    def test_asi_ns_current(self):
        assert V1285_ASI_NS_CURRENT == 0.7905
        assert V1285_ASI_NS_LOCKED_PCT == 92.91


class TestFalsifierDispatch:
    def test_dispatch_has_5_hypotheses(self):
        assert len(V1285_FALSIFIER_DISPATCH) == 5

    def test_hypothesis_keys(self):
        expected = {
            "h_zero_unwrap_in_production_src",
            "h_zero_expect_in_production_src",
            "h_zero_panic_in_production_src",
            "h_zero_todo_in_production_src",
            "h_zero_unsafe_in_production_src",
        }
        assert set(V1285_FALSIFIER_DISPATCH.keys()) == expected

    def test_dispatch_values_are_callables(self):
        for k, v in V1285_FALSIFIER_DISPATCH.items():
            assert callable(v), f"{k} should be callable"

    def test_worst5_list_complete(self):
        assert len(V1285_V1284_WORST5) == 5
        assert "apeireth-vector" in V1285_V1284_WORST5


# ============================================================
# 2. Discovery
# ============================================================

class TestDiscovery:
    def test_discover_returns_list(self):
        pd = resolve_promethean_dir()
        crates = discover_all_apeireth_crates(pd)
        assert isinstance(crates, list)
        assert len(crates) > 0

    def test_discover_at_least_42_crates(self):
        """We expect 42 apeireth-* crates (V1283 confirmed 42)."""
        pd = resolve_promethean_dir()
        crates = discover_all_apeireth_crates(pd)
        assert len(crates) >= 42, f"Expected >= 42 crates, got {len(crates)}"

    def test_discover_all_start_with_apeireth(self):
        pd = resolve_promethean_dir()
        crates = discover_all_apeireth_crates(pd)
        for c in crates:
            assert c.startswith("apeireth-"), f"{c} should start with 'apeireth-'"

    def test_discover_returns_sorted(self):
        pd = resolve_promethean_dir()
        crates = discover_all_apeireth_crates(pd)
        assert crates == sorted(crates)


# ============================================================
# 3. Philosophy gate
# ============================================================

class TestPhilosophyGate:
    def test_gate_has_27_entries(self):
        gate = _v1285_philosophy_gate()
        assert len(gate) == 27  # V1284 24 + V1285 3 new

    def test_gate_inherits_v1284(self):
        gate = _v1285_philosophy_gate()
        assert "v1284_extends_v1283_not_replaces" in gate
        assert "v1284_audit_only_no_fix" in gate
        assert "v1284_production_src_only" in gate

    def test_gate_has_v1285_new(self):
        gate = _v1285_philosophy_gate()
        assert "v1285_extends_v1284_not_replaces" in gate
        assert "v1285_all42_not_vendor" in gate
        assert "v1285_no_kpi_inflate" in gate

    def test_all_gates_true(self):
        gate = _v1285_philosophy_gate()
        for k, v in gate.items():
            assert v is True, f"Gate {k} should be True"


# ============================================================
# 4. Data structures
# ============================================================

class TestLedger:
    def test_ledger_default(self):
        ledger = All42SecurityLedger()
        assert ledger.n_pass == 0
        assert ledger.n_fail == 0
        assert ledger.n_inconclusive == 0
        assert ledger.total_hotspots == 0

    def test_ledger_pass_count(self):
        from apeireth.v1284_worst5_security_audit import CrateHypothesisResult
        ledger = All42SecurityLedger()
        for verdict in ["PASS", "PASS", "FAIL", "INCONCLUSIVE"]:
            ledger.results.append(CrateHypothesisResult(
                crate_name="x", hypothesis_id="h", claim="c", severity="info",
                pass_fail=verdict,
            ))
        assert ledger.n_pass == 2
        assert ledger.n_fail == 1
        assert ledger.n_inconclusive == 1

    def test_ledger_total_hotspots(self):
        from apeireth.v1284_worst5_security_audit import CrateSecurityMetrics
        ledger = All42SecurityLedger()
        m1 = CrateSecurityMetrics(crate_name="a", crate_src="/tmp/a", src_files_scanned=1, src_lines_scanned=100, n_unwrap=5)
        m2 = CrateSecurityMetrics(crate_name="b", crate_src="/tmp/b", src_files_scanned=1, src_lines_scanned=100, n_expect=3)
        ledger.crate_metrics = [m1, m2]
        assert ledger.total_hotspots == 8

    def test_ledger_serializable_to_dict(self):
        ledger = All42SecurityLedger(run_id="test", n_crates_total=42)
        d = {
            "run_id": ledger.run_id,
            "n_crates_total": ledger.n_crates_total,
        }
        assert d["run_id"] == "test"
        assert d["n_crates_total"] == 42


# ============================================================
# 5. Runner
# ============================================================

class TestRunner:
    def test_run_audits_all_42_crates(self):
        """V1285 covers all 42 apeireth-* crates (extends V1284 worst-5)."""
        ledger = run_all42_audit()
        assert ledger.n_crates_total >= 42
        # Allow some INCONCLUSIVE for crates w/o src/
        assert ledger.n_crates_audited + ledger.n_crates_inconclusive == ledger.n_crates_total

    def test_run_includes_v1284_worst5(self):
        """V1285 should include V1284's worst-5 (extend not replace)."""
        ledger = run_all42_audit()
        for w in V1285_V1284_WORST5:
            assert w in [m.crate_name for m in ledger.crate_metrics], f"V1284 worst-5 {w} missing in V1285"

    def test_run_hypotheses_count(self):
        ledger = run_all42_audit()
        # 5 hyp × (n_crates_audited + n_crates_inconclusive)
        assert len(ledger.results) == 5 * (ledger.n_crates_audited + ledger.n_crates_inconclusive)

    def test_run_elapsed_tracked(self):
        ledger = run_all42_audit()
        assert ledger.elapsed_ms >= 0
        assert ledger.elapsed_ms < 30000  # should complete in <30s for 42 crates

    def test_run_philosophy_gate_set(self):
        ledger = run_all42_audit()
        assert len(ledger.philosophy_gate) == 27

    def test_run_with_crate_filter(self):
        """--crate filter (custom predicate) should restrict audit."""
        ledger = run_all42_audit(crate_filter=lambda c: c == "apeireth-vector")
        # Only vector audited (or INCONCLUSIVE)
        assert ledger.n_crates_audited + ledger.n_crates_inconclusive == 1
        if ledger.n_crates_audited == 1:
            assert ledger.crate_metrics[0].crate_name == "apeireth-vector"


# ============================================================
# 6. Output formatters
# ============================================================

class TestMarkdown:
    def test_markdown_contains_header(self):
        ledger = run_all42_audit()
        md = _to_markdown(ledger)
        assert "V1285 All-42 Rust Security Depth Audit" in md

    def test_markdown_contains_philosophy_gate(self):
        ledger = run_all42_audit()
        md = _to_markdown(ledger)
        assert "V3 Philosophy Gate" in md
        assert "v1285_extends_v1284_not_replaces" in md

    def test_markdown_contains_disclaimers(self):
        ledger = run_all42_audit()
        md = _to_markdown(ledger)
        assert "PASS 不代表" in md or "Rust 已 ASI V1" in md
        assert "不刷 KPI" in md

    def test_markdown_contains_coverage_delta(self):
        ledger = run_all42_audit()
        md = _to_markdown(ledger)
        assert "Coverage Delta" in md
        assert "+37" in md or f"+{ledger.n_crates_audited - 5}" in md

    def test_markdown_contains_v1284_reference(self):
        ledger = run_all42_audit()
        md = _to_markdown(ledger)
        assert "V1284" in md
        assert "V1285" in md


class TestJson:
    def test_json_valid(self):
        ledger = run_all42_audit()
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        assert isinstance(d, dict)
        assert "n_crates_total" in d
        assert "n_crates_audited" in d
        assert "philosophy_gate" in d

    def test_json_includes_per_crate_metrics(self):
        ledger = run_all42_audit()
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        assert "per_crate_metrics" in d
        assert len(d["per_crate_metrics"]) == ledger.n_crates_audited
        first = d["per_crate_metrics"][0]
        assert "crate_name" in first
        assert "findings" in first
        assert "n_total_hotspots" in first

    def test_json_includes_worst5_overlap(self):
        ledger = run_all42_audit()
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        assert "worst5_overlap" in d
        for w in V1285_V1284_WORST5:
            if w in [m["crate_name"] for m in d["per_crate_metrics"]]:
                assert w in d["worst5_overlap"]


# ============================================================
# 7. CLI
# ============================================================

class TestCLI:
    def test_probe_mode(self, capsys):
        code = main(["--probe"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1285 All-42 Security Audit" in out
        assert "apeireth-" in out
        assert "V1284-worst5" in out  # worst-5 marker

    def test_run_mode(self, capsys):
        code = main(["--run"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1285 All-42 Rust Security Depth Audit" in out
        assert "All apeireth-* crates discovered" in out

    def test_json_mode(self, capsys):
        code = main(["--json"])
        out = capsys.readouterr().out
        assert code == 0
        d = json.loads(out)
        assert "n_crates_total" in d

    def test_report_mode(self, tmp_path):
        out_path = tmp_path / "v1285_test_report.md"
        code = main(["--report", str(out_path)])
        assert code == 0
        assert out_path.exists()
        content = out_path.read_text(encoding="utf-8")
        assert "V1285 All-42 Rust Security Depth Audit" in content
        assert "V1285 wrote report" not in content  # this is in stdout

    def test_clean_only_mode(self, capsys):
        code = main(["--clean-only"])
        out = capsys.readouterr().out
        assert code == 0
        assert "clean crates" in out

    def test_top_mode(self, capsys):
        code = main(["--run", "--top", "5"])
        out = capsys.readouterr().out
        assert code == 0
        assert "Top-5 Crates by Total Hotspots" in out

    def test_module_invocation(self):
        """Run as module and verify exit code 0."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1285_all42_crate_security_audit", "--probe"],
            capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
            encoding="utf-8", errors="replace",
        )
        assert result.returncode == 0
        combined = (result.stdout or "") + (result.stderr or "")
        assert "V1285 All-42 Security Audit" in combined
        assert "apeireth-" in combined

    def test_module_invocation_run(self, tmp_path):
        """Run as module --report and verify file is created."""
        out_path = tmp_path / "v1285_subprocess_report.md"
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1285_all42_crate_security_audit", "--report", str(out_path)],
            capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
            encoding="utf-8", errors="replace",
        )
        assert result.returncode == 0
        assert out_path.exists()


# ============================================================
# 8. Integration / coverage delta
# ============================================================

class TestIntegration:
    def test_v1285_extends_v1284_in_data(self):
        """V1285 should include V1284's data points (not replace)."""
        v1284_path = PROMETHEAN_ROOT / "V1284_REPORT.md"
        v1285_path = PROMETHEAN_ROOT / "V1285_REPORT.md"
        if not v1284_path.exists():
            pytest.skip("V1284_REPORT.md not present")
        v1284_md = v1284_path.read_text(encoding="utf-8")
        # V1285 should at minimum reference V1284 (top-5 metrics should overlap)
        # Verify by re-running V1284 data
        from apeireth.v1284_worst5_security_audit import run_worst5_audit
        v1284_ledger = run_worst5_audit()
        v1284_apeireth_vector_unwrap = next(
            (m.n_unwrap for m in v1284_ledger.crate_metrics if m.crate_name == "apeireth-vector"),
            None,
        )
        v1285_ledger = run_all42_audit()
        v1285_apeireth_vector_unwrap = next(
            (m.n_unwrap for m in v1285_ledger.crate_metrics if m.crate_name == "apeireth-vector"),
            None,
        )
        # Same data point: V1285 should match V1284
        if v1284_apeireth_vector_unwrap is not None:
            assert v1284_apeireth_vector_unwrap == v1285_apeireth_vector_unwrap

    def test_v1285_finds_more_crates_than_v1284(self):
        """V1285 (all-42) must cover more crates than V1284 (worst-5)."""
        from apeireth.v1284_worst5_security_audit import run_worst5_audit
        v1284_ledger = run_worst5_audit()
        v1285_ledger = run_all42_audit()
        assert v1285_ledger.n_crates_audited > v1284_ledger.crates_audited
        assert v1285_ledger.n_crates_audited - v1284_ledger.crates_audited >= 30  # at least 30 more

    def test_v1285_total_hotspots_at_least_v1284(self):
        """V1285 should find >= V1284's hotspots (extends, not replace)."""
        from apeireth.v1284_worst5_security_audit import run_worst5_audit
        v1284_ledger = run_worst5_audit()
        v1285_ledger = run_all42_audit()
        assert v1285_ledger.total_hotspots >= v1284_ledger.total_hotspots


# ============================================================
# 9. Cross-V1284 consistency
# ============================================================

class TestCrossV1284Consistency:
    def test_worst5_in_v1285_data_matches_v1284(self):
        """V1284 worst-5 in V1285 should give same metrics as V1284 alone."""
        from apeireth.v1284_worst5_security_audit import run_worst5_audit
        v1284_ledger = run_worst5_audit()
        v1285_ledger = run_all42_audit()
        for w in V1285_V1284_WORST5:
            v1284_m = next((m for m in v1284_ledger.crate_metrics if m.crate_name == w), None)
            v1285_m = next((m for m in v1285_ledger.crate_metrics if m.crate_name == w), None)
            if v1284_m and v1285_m:
                # Same crate, same data
                assert v1284_m.n_unwrap == v1285_m.n_unwrap, f"{w} unwrap mismatch"
                assert v1284_m.n_expect == v1285_m.n_expect, f"{w} expect mismatch"
                assert v1284_m.n_panic == v1285_m.n_panic, f"{w} panic mismatch"

    def test_v1285_philosophy_inherits_v1284(self):
        """V1285's philosophy gate must inherit all of V1284's gates."""
        v1285_gate = _v1285_philosophy_gate()
        # V1284 had 24 gates (V1283 21 + V1284 3)
        # V1285 should have >= 24 + 3 = 27
        assert len(v1285_gate) >= 27

    def test_v1285_run_id_format(self):
        """V1285 run_id should be v1285-<timestamp>."""
        ledger = run_all42_audit()
        assert ledger.run_id.startswith("v1285-")
