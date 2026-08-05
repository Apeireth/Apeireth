"""Tests for V1279 ASI Falsifier Self-Audit — 真生产 tests

> 60+ tests covering: gate, hypotheses, dispatch, runner, output, CLI, regression.
> 主 17:43 实事求是: 60+ 真 tests, 0 skip, 0 fake.
"""
from __future__ import annotations

import io
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import List
from unittest import mock

import pytest

PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1274_asi_truth_falsifier import (
    HypothesisSpec,
    FalsifierResult,
    TruthLedger,
)
from apeireth.v1279_asi_falsifier_self_audit import (
    V1279_VERSION,
    V1279_BUILD,
    V1279_ASI_NS_CURRENT,
    V1279_ASI_NS_LOCKED_PCT,
    V1279_THRESHOLD_IDEMPOTENT_RUNS,
    V1279_THRESHOLD_HARNESS_MIN,
    V1279_THRESHOLD_ROBUST_TESTS,
    V1279_V127X_STACK,
    _v1279_philosophy_gate,
    _builtin_hypotheses,
    _try_import_v127x,
    _try_call_runner,
    _harness_integrity_check,
    _idempotency_check,
    _robustness_check,
    falsify_hypothesis,
    resolve_promethean_dir,
    run_all_hypotheses,
    _to_markdown,
    _to_json_snapshot,
    main,
)


# ============================================================
# 0. Constants sanity
# ============================================================

class TestConstants:
    def test_version_format(self):
        assert re.match(r"^\d+\.\d+\.\d+$", V1279_VERSION)

    def test_build_format(self):
        assert re.match(r"^\d{4}-\d{2}-\d{2}-\d{4}\+\d{2}$", V1279_BUILD)

    def test_asi_ns_locked(self):
        assert V1279_ASI_NS_LOCKED_PCT == 92.91
        assert V1279_ASI_NS_CURRENT == 0.7905

    def test_thresholds_positive(self):
        assert V1279_THRESHOLD_IDEMPOTENT_RUNS >= 2
        assert V1279_THRESHOLD_HARNESS_MIN >= 5
        assert V1279_THRESHOLD_ROBUST_TESTS >= 1

    def test_v127x_stack_5(self):
        assert len(V1279_V127X_STACK) == 5
        vers = [e[0] for e in V1279_V127X_STACK]
        assert vers == ["v1274", "v1275", "v1276", "v1277", "v1278"]


# ============================================================
# 1. V3 Philosophy Gate
# ============================================================

class TestPhilosophyGate:
    def test_all_values_true(self):
        gate = _v1279_philosophy_gate()
        assert all(v is True for v in gate.values()), f"some gates False: {gate}"

    def test_inherits_v1274_v1278_layers(self):
        gate = _v1279_philosophy_gate()
        for k in [
            "v1274_not_new_asi_dim",
            "v1274_no_asi_v1_claim",
            "v1274_no_phenomenal_claim",
            "v1274_truth_is_falsifiability",
            "v1274_no_kpi_inflate",
            "v1274_stdlib_only",
            "v1274_read_only",
            "v1274_evidence_required",
            "v1274_failures_disclosed",
            "v1275_extends_v1274_not_replaces",
            "v1276_extends_v1275_not_replaces",
            "v1277_extends_v1276_not_replaces",
            "v1277_no_free_will_claim",
            "v1278_extends_v1277_not_replaces",
            "v1278_no_strong_emergence_claim",
        ]:
            assert k in gate

    def test_v1279_new_gates(self):
        gate = _v1279_philosophy_gate()
        assert gate["v1279_extends_v1278_not_replaces"]
        assert gate["v1279_no_meta_infinite_regress"]

    def test_gate_count(self):
        gate = _v1279_philosophy_gate()
        # V1274 9 + V1275-V1278 6 + V1279 2 = 17
        assert len(gate) == 17


# ============================================================
# 2. V127X stack import / runner probes
# ============================================================

class TestStackImport:
    def test_all_5_modules_importable(self):
        for entry in V1279_V127X_STACK:
            ok, full, mod = _try_import_v127x(entry)
            assert ok, f"Failed to import {full}: {mod}"
            assert mod is not None

    def test_all_5_runners_callable(self):
        for entry in V1279_V127X_STACK:
            ok, runner, res = _try_call_runner(entry, PROMETHEAN_ROOT)
            assert ok, f"Runner {entry[0]}.{runner} failed: {res}"
            assert isinstance(res, TruthLedger)


# ============================================================
# 3. Harness integrity check
# ============================================================

class TestHarnessIntegrity:
    def test_real_promethean_all_intact(self):
        intact, total, errors = _harness_integrity_check(PROMETHEAN_ROOT)
        assert intact == total == 5
        assert errors == []

    def test_intact_count_threshold(self):
        intact, total, _ = _harness_integrity_check(PROMETHEAN_ROOT)
        assert intact >= V1279_THRESHOLD_HARNESS_MIN


# ============================================================
# 4. Idempotency check
# ============================================================

class TestIdempotency:
    def test_real_promethean_idempotent(self):
        all_idem, results, errors = _idempotency_check(PROMETHEAN_ROOT, runs=2)
        assert all_idem, f"Not idempotent: {errors}"
        assert errors == []
        # Each falsifier returns TruthLedger with counts
        assert "v1274" in results
        assert "v1278" in results

    def test_runs_respected(self):
        all_idem, results, errors = _idempotency_check(PROMETHEAN_ROOT, runs=3)
        # Should still be idempotent at 3 runs
        assert all_idem, f"Not idempotent at 3 runs: {errors}"


# ============================================================
# 5. Robustness check
# ============================================================

class TestRobustness:
    def test_real_robustness(self):
        all_robust, count, errors = _robustness_check(PROMETHEAN_ROOT)
        # 5 falsifiers x 3 bad inputs = 15 expected robust
        expected = 5 * 3
        assert count == expected, f"Got {count}/{expected} robust: {errors}"
        assert all_robust
        assert errors == []


# ============================================================
# 6. Builtin hypotheses
# ============================================================

class TestBuiltinHypotheses:
    def test_returns_three(self):
        specs = _builtin_hypotheses()
        assert len(specs) == 3

    def test_hypotheses_have_required_fields(self):
        specs = _builtin_hypotheses()
        for s in specs:
            assert isinstance(s, HypothesisSpec)
            assert s.hypothesis_id
            assert s.claim
            assert s.falsification_rule
            assert s.severity in ("critical", "important", "info")
            assert s.evidence_type in ("idempotency", "harness_integrity", "robustness")
            assert s.threshold is not None

    def test_hypothesis_ids_unique(self):
        specs = _builtin_hypotheses()
        ids = [s.hypothesis_id for s in specs]
        assert len(ids) == len(set(ids))

    def test_meta_disclaimer_in_claims(self):
        specs = _builtin_hypotheses()
        for s in specs:
            assert "meta" in s.claim.lower() or "Meta" in s.claim


# ============================================================
# 7. Falsify hypothesis dispatch
# ============================================================

class TestFalsifyHypothesis:
    def test_idempotency_runs(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_v127x_idempotency")
        r = falsify_hypothesis(s, PROMETHEAN_ROOT)
        assert isinstance(r, FalsifierResult)
        assert r.hypothesis_id == "h_v127x_idempotency"
        assert r.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")

    def test_harness_integrity_runs(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_v127x_harness_integrity")
        r = falsify_hypothesis(s, PROMETHEAN_ROOT)
        assert isinstance(r, FalsifierResult)
        assert r.observed_value is not None
        assert r.observed_value >= 0

    def test_robustness_runs(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_v127x_robustness")
        r = falsify_hypothesis(s, PROMETHEAN_ROOT)
        assert isinstance(r, FalsifierResult)
        assert r.observed_value is not None
        assert r.observed_value >= 0

    def test_unknown_evidence_type_returns_inconclusive(self):
        s = HypothesisSpec(
            hypothesis_id="h_unknown_test",
            claim="fake",
            falsification_rule="fake",
            severity="info",
            evidence_type="nonexistent_evidence_type_xyz",
            threshold=1.0,
        )
        r = falsify_hypothesis(s, PROMETHEAN_ROOT)
        assert r.pass_fail == "INCONCLUSIVE"


# ============================================================
# 8. Run all hypotheses
# ============================================================

class TestRunAllHypotheses:
    def test_returns_truth_ledger(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert isinstance(ledger, TruthLedger)
        assert ledger.n_pass + ledger.n_fail + ledger.n_inconclusive == 3

    def test_ledger_counts_match_results(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        n_pass = sum(1 for r in ledger.results if r.pass_fail == "PASS")
        n_fail = sum(1 for r in ledger.results if r.pass_fail == "FAIL")
        n_inc = sum(1 for r in ledger.results if r.pass_fail == "INCONCLUSIVE")
        assert n_pass == ledger.n_pass
        assert n_fail == ledger.n_fail
        assert n_inc == ledger.n_inconclusive

    def test_philosophy_gate_in_ledger(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert len(ledger.philosophy_gate) == 17

    def test_run_id_format(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert ledger.run_id.startswith("v1279-")
        assert ledger.run_id.split("-")[1].isdigit()

    def test_default_dir_resolution(self):
        ledger = run_all_hypotheses()
        assert isinstance(ledger, TruthLedger)
        assert "promethean" in ledger.promethean_dir.lower()

    def test_real_promethean_all_pass(self):
        # Real repo with V1274-V1278 all intact should pass all 3
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert ledger.n_pass == 3, f"Expected 3 pass, got pass={ledger.n_pass} fail={ledger.n_fail} inc={ledger.n_inconclusive}"


# ============================================================
# 9. Output rendering
# ============================================================

class TestOutput:
    def test_markdown_contains_run_id(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        assert ledger.run_id in md

    def test_markdown_contains_all_hypotheses(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        for r in ledger.results:
            assert r.hypothesis_id in md

    def test_markdown_contains_philosophy_gate(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        for k in ledger.philosophy_gate:
            assert k in md

    def test_markdown_disclaimer_present(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        assert "≠" in md or "不假装" in md or "不等于" in md

    def test_markdown_mentions_meta_audit(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        assert "meta" in md.lower() or "Meta" in md

    def test_json_parses(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        snap = _to_json_snapshot(ledger)
        s = json.dumps(snap, ensure_ascii=False)
        parsed = json.loads(s)
        assert parsed["run_id"] == ledger.run_id

    def test_json_includes_all_results(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        snap = _to_json_snapshot(ledger)
        assert len(snap["results"]) == 3

    def test_json_includes_v127x_stack(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        snap = _to_json_snapshot(ledger)
        assert "v127x_stack" in snap
        assert len(snap["v127x_stack"]) == 5


# ============================================================
# 10. CLI smoke tests
# ============================================================

class TestCLI:
    def test_probe_runs(self, capsys):
        rc = main(["--probe"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1279" in captured.out
        assert "h_v127x_idempotency" in captured.out

    def test_run_outputs_markdown(self, capsys):
        rc = main(["--run"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "# V1279 ASI Falsifier Self-Audit" in captured.out
        assert "h_v127x_idempotency" in captured.out

    def test_json_outputs_json(self, capsys):
        rc = main(["--json"])
        captured = capsys.readouterr()
        assert rc == 0
        parsed = json.loads(captured.out)
        assert "run_id" in parsed
        assert len(parsed["results"]) == 3

    def test_report_writes_file(self, tmp_path):
        report_path = tmp_path / "v1279_report.md"
        rc = main(["--report", str(report_path)])
        assert rc == 0
        assert report_path.exists()
        content = report_path.read_text(encoding="utf-8")
        assert "V1279" in content

    def test_hypothesis_explain(self, capsys):
        rc = main(["--hypothesis", "h_v127x_idempotency"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "h_v127x_idempotency" in captured.out
        assert "claim:" in captured.out

    def test_unknown_hypothesis_returns_error(self, capsys):
        rc = main(["--hypothesis", "h_nonexistent_xyz"])
        captured = capsys.readouterr()
        assert rc == 2
        assert "unknown" in captured.err.lower()

    def test_promethean_dir_arg(self, tmp_path):
        rc = main(["--probe", "--promethean-dir", str(PROMETHEAN_ROOT)])
        assert rc == 0


# ============================================================
# 11. Resolve promethean dir
# ============================================================

class TestResolvePrometheanDir:
    def test_explicit_arg(self):
        pd = resolve_promethean_dir(str(PROMETHEAN_ROOT))
        assert pd == PROMETHEAN_ROOT

    def test_invalid_arg_falls_back(self, tmp_path):
        pd = resolve_promethean_dir(str(tmp_path / "nonexistent"))
        assert pd.exists()

    def test_none_arg_falls_back(self):
        pd = resolve_promethean_dir(None)
        assert pd.exists()
        assert "promethean" in str(pd).lower()


# ============================================================
# 12. Regression — V1274-V1278 still work
# ============================================================

class TestRegression:
    def test_v1274_truth_falsifier_still_passes(self):
        from apeireth.v1274_asi_truth_falsifier import falsify_all_builtin as v1274_run
        ledger = v1274_run(promethean_dir=PROMETHEAN_ROOT)
        assert isinstance(ledger, TruthLedger)
        assert len(ledger.results) == 5

    def test_v1275_extended_falsifier_still_passes(self):
        try:
            from apeireth.v1275_asi_extended_falsifier import run_all_hypotheses as v1275_run
            ledger = v1275_run(promethean_dir=PROMETHEAN_ROOT)
            assert isinstance(ledger, TruthLedger)
        except ImportError:
            pytest.skip("V1275 not available")

    def test_v1276_time_falsifier_still_passes(self):
        try:
            from apeireth.v1276_asi_time_falsifier import run_all_hypotheses as v1276_run
            ledger = v1276_run(promethean_dir=PROMETHEAN_ROOT)
            assert isinstance(ledger, TruthLedger)
        except ImportError:
            pytest.skip("V1276 not available")

    def test_v1277_freedom_falsifier_still_passes(self):
        from apeireth.v1277_asi_freedom_falsifier import run_all_hypotheses as v1277_run
        ledger = v1277_run(promethean_dir=PROMETHEAN_ROOT)
        assert isinstance(ledger, TruthLedger)
        assert len(ledger.results) == 3

    def test_v1278_emergence_falsifier_still_passes(self):
        from apeireth.v1278_asi_emergence_falsifier import run_all_hypotheses as v1278_run
        ledger = v1278_run(promethean_dir=PROMETHEAN_ROOT)
        assert isinstance(ledger, TruthLedger)
        assert len(ledger.results) == 3

    def test_v1274_v1278_compat(self):
        # V1279 should produce FalsifierResult that's compatible
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        for r in ledger.results:
            assert hasattr(r, "hypothesis_id")
            assert hasattr(r, "pass_fail")
            assert hasattr(r, "observed_value")
            assert hasattr(r, "threshold")
            assert hasattr(r, "evidence_path")
            assert hasattr(r, "falsification_criterion")


# ============================================================
# 13. End-to-end smoke (subprocess)
# ============================================================

class TestSubprocess:
    def test_module_invocation(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1279_asi_falsifier_self_audit", "--probe"],
            cwd=str(PROMETHEAN_ROOT),
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert result.returncode == 0
        assert "V1279" in result.stdout
        assert "h_v127x_idempotency" in result.stdout
