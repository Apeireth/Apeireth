"""Tests for V1278 ASI Emergence Falsifier — 真生产 tests

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
from pathlib import Path
from typing import List
from unittest import mock

import pytest

# Add promethean root to sys.path (主 19:33 走在 V1274-V1277 肩上)
PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1274_asi_truth_falsifier import (
    HypothesisSpec,
    FalsifierResult,
    TruthLedger,
)
from apeireth.v1278_asi_emergence_falsifier import (
    V1278_VERSION,
    V1278_BUILD,
    V1278_ASI_NS_CURRENT,
    V1278_ASI_NS_LOCKED_PCT,
    V1278_THRESHOLD_ARTIFACT_GROWTH,
    V1278_THRESHOLD_SIZE_ENTROPY,
    V1278_THRESHOLD_TOPIC_BREADTH,
    V1278_SIZE_BUCKETS,
    _v1278_philosophy_gate,
    _builtin_hypotheses,
    _count_apeireth_modules,
    _size_bucket_distribution,
    _topic_breadth,
    _shannon_entropy_bits,
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
        assert re.match(r"^\d+\.\d+\.\d+$", V1278_VERSION)

    def test_build_format(self):
        # YYYY-MM-DD-HHMM+TZ
        assert re.match(r"^\d{4}-\d{2}-\d{2}-\d{4}\+\d{2}$", V1278_BUILD)

    def test_asi_ns_locked(self):
        assert V1278_ASI_NS_LOCKED_PCT == 92.91
        assert V1278_ASI_NS_CURRENT == 0.7905

    def test_thresholds_positive(self):
        assert V1278_THRESHOLD_ARTIFACT_GROWTH > 0
        assert V1278_THRESHOLD_SIZE_ENTROPY > 0
        assert V1278_THRESHOLD_TOPIC_BREADTH > 0

    def test_size_buckets_5(self):
        assert len(V1278_SIZE_BUCKETS) == 5
        names = [b[0] for b in V1278_SIZE_BUCKETS]
        assert names == ["XS", "S", "M", "L", "XL"]


# ============================================================
# 1. V3 Philosophy Gate
# ============================================================

class TestPhilosophyGate:
    def test_all_values_true(self):
        gate = _v1278_philosophy_gate()
        assert all(v is True for v in gate.values()), f"some gates False: {gate}"

    def test_inherits_v1274_base(self):
        gate = _v1278_philosophy_gate()
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
        ]:
            assert k in gate

    def test_inherits_v1275_v1276_v1277_layers(self):
        gate = _v1278_philosophy_gate()
        assert gate["v1275_extends_v1274_not_replaces"]
        assert gate["v1276_extends_v1275_not_replaces"]
        assert gate["v1277_extends_v1276_not_replaces"]
        assert gate["v1277_no_free_will_claim"]

    def test_v1278_new_gates(self):
        gate = _v1278_philosophy_gate()
        assert gate["v1278_extends_v1277_not_replaces"]
        assert gate["v1278_no_strong_emergence_claim"]

    def test_gate_count(self):
        gate = _v1278_philosophy_gate()
        # V1274 9 + V1275 1 + V1276 1 + V1277 2 + V1278 2 = 15
        assert len(gate) == 15


# ============================================================
# 2. Shannon entropy math
# ============================================================

class TestShannonEntropy:
    def test_single_type_returns_zero(self):
        assert _shannon_entropy_bits({"a": 10}) == 0.0

    def test_two_equal_returns_one(self):
        h = _shannon_entropy_bits({"a": 1, "b": 1})
        assert abs(h - 1.0) < 1e-6

    def test_four_equal_returns_two(self):
        h = _shannon_entropy_bits({"a": 1, "b": 1, "c": 1, "d": 1})
        assert abs(h - 2.0) < 1e-6

    def test_empty_returns_zero(self):
        assert _shannon_entropy_bits({}) == 0.0

    def test_three_unequal(self):
        # {a:1, b:2, c:1}: total=4, p=[0.25, 0.5, 0.25]
        # H = -(0.25*log2(0.25) + 0.5*log2(0.5) + 0.25*log2(0.25))
        #   = -(0.25*-2 + 0.5*-1 + 0.25*-2)
        #   = 0.5 + 0.5 + 0.5 = 1.5
        h = _shannon_entropy_bits({"a": 1, "b": 2, "c": 1})
        assert abs(h - 1.5) < 1e-6


# ============================================================
# 3. Module count gatherer
# ============================================================

class TestCountModules:
    def test_real_promethean_returns_positive(self):
        n, ok, errs = _count_apeireth_modules(PROMETHEAN_ROOT)
        assert ok
        assert errs == []
        assert n > 1000  # Real repo should have many modules

    def test_missing_dir_returns_zero(self, tmp_path):
        n, ok, errs = _count_apeireth_modules(tmp_path / "nonexistent_xyz")
        assert not ok
        assert n == 0
        assert len(errs) > 0


# ============================================================
# 4. Size bucket distribution
# ============================================================

class TestSizeBucket:
    def test_real_promethean_returns_distribution(self):
        dist, total, ok, errs = _size_bucket_distribution(PROMETHEAN_ROOT)
        assert ok
        assert total > 0
        assert sum(dist.values()) > 1000

    def test_all_buckets_present_in_real_repo(self):
        # Real repo has both XS and XL modules
        dist, _, ok, _ = _size_bucket_distribution(PROMETHEAN_ROOT)
        assert ok
        assert dist["XS"] > 0  # Many small utility modules
        assert dist["XL"] > 0  # Some large modules (v1274 etc.)

    def test_missing_dir(self, tmp_path):
        dist, total, ok, errs = _size_bucket_distribution(tmp_path / "nonexistent")
        assert not ok
        assert dist == {}
        assert total == 0


# ============================================================
# 5. Topic breadth
# ============================================================

class TestTopicBreadth:
    def test_real_promethean_returns_many_topics(self):
        n, topics, counter, ok, errs = _topic_breadth(PROMETHEAN_ROOT)
        assert ok
        assert errs == []
        assert n >= 6  # Real repo has many topics
        assert len(topics) == n
        assert sum(counter.values()) > 1000

    def test_real_promethean_includes_asi_topic(self):
        _, topics, _, ok, _ = _topic_breadth(PROMETHEAN_ROOT)
        assert ok
        # Most modules have "asi" as the topic
        assert "asi" in topics

    def test_missing_dir(self, tmp_path):
        n, topics, counter, ok, errs = _topic_breadth(tmp_path / "nonexistent")
        assert not ok
        assert n == 0


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
            assert s.evidence_type in ("module_count", "size_entropy", "topic_breadth")
            assert s.threshold is not None

    def test_hypothesis_ids_unique(self):
        specs = _builtin_hypotheses()
        ids = [s.hypothesis_id for s in specs]
        assert len(ids) == len(set(ids))


# ============================================================
# 7. Falsify hypothesis dispatch
# ============================================================

class TestFalsifyHypothesis:
    def test_module_count_runs(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_artifact_growth")
        r = falsify_hypothesis(s, PROMETHEAN_ROOT)
        assert isinstance(r, FalsifierResult)
        assert r.hypothesis_id == "h_artifact_growth"
        assert r.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        assert r.observed_value is not None

    def test_size_entropy_runs(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_size_entropy")
        r = falsify_hypothesis(s, PROMETHEAN_ROOT)
        assert isinstance(r, FalsifierResult)
        assert r.observed_value is not None
        assert 0 <= r.observed_value <= 5.0  # Reasonable entropy range

    def test_topic_breadth_runs(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_topic_breadth")
        r = falsify_hypothesis(s, PROMETHEAN_ROOT)
        assert isinstance(r, FalsifierResult)
        assert r.observed_value is not None
        assert r.observed_value > 0

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

    def test_inconclusive_when_no_apeireth_dir(self, tmp_path):
        s = HypothesisSpec(
            hypothesis_id="h_artifact_growth",
            claim="fake",
            falsification_rule="fake",
            severity="critical",
            evidence_type="module_count",
            threshold=1400.0,
        )
        # tmp_path has no apeireth/ subdir
        fake_root = tmp_path
        (fake_root / "tests").mkdir()  # bypass resolve_promethean_dir check
        r = falsify_hypothesis(s, fake_root)
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
        assert len(ledger.philosophy_gate) == 15

    def test_run_id_format(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert ledger.run_id.startswith("v1278-")
        assert ledger.run_id.split("-")[1].isdigit()

    def test_default_dir_resolution(self):
        ledger = run_all_hypotheses()  # no arg
        assert isinstance(ledger, TruthLedger)
        assert "promethean" in ledger.promethean_dir.lower()

    def test_real_promethean_all_pass(self):
        # Real repo at this point has 2500+ modules, well above thresholds
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        # All 3 should pass given the real size of the codebase
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
        # Should explicitly mention non-claiming stance
        assert "≠" in md or "不假装" in md or "不等于" in md

    def test_markdown_mentions_asi_5_philosophy_gaps_closure(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        # ASI 5 哲学空隙 完整闭环 = V1278 收官
        assert "ASI 5" in md or "涌现" in md or "Emergence" in md

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
        for r in snap["results"]:
            assert "hypothesis_id" in r
            assert "verdict" in r
            assert "observed_value" in r


# ============================================================
# 10. CLI smoke tests
# ============================================================

class TestCLI:
    def test_probe_runs(self, capsys):
        rc = main(["--probe"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1278" in captured.out
        assert "h_artifact_growth" in captured.out

    def test_run_outputs_markdown(self, capsys):
        rc = main(["--run"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "# V1278 ASI Emergence Falsifier" in captured.out
        assert "h_artifact_growth" in captured.out
        assert "h_size_entropy" in captured.out
        assert "h_topic_breadth" in captured.out

    def test_json_outputs_json(self, capsys):
        rc = main(["--json"])
        captured = capsys.readouterr()
        assert rc == 0
        parsed = json.loads(captured.out)
        assert "run_id" in parsed
        assert "results" in parsed
        assert len(parsed["results"]) == 3

    def test_report_writes_file(self, tmp_path):
        report_path = tmp_path / "v1278_report.md"
        rc = main(["--report", str(report_path)])
        assert rc == 0
        assert report_path.exists()
        content = report_path.read_text(encoding="utf-8")
        assert "V1278" in content
        assert "h_artifact_growth" in content

    def test_hypothesis_explain(self, capsys):
        rc = main(["--hypothesis", "h_artifact_growth"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "h_artifact_growth" in captured.out
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
        # Should fall back to cwd or file's parent
        assert pd.exists()

    def test_none_arg_falls_back(self):
        pd = resolve_promethean_dir(None)
        assert pd.exists()
        assert "promethean" in str(pd).lower()


# ============================================================
# 12. Regression — V1274-V1277 still work
# ============================================================

class TestRegression:
    def test_v1274_truth_falsifier_still_passes(self):
        from apeireth.v1274_asi_truth_falsifier import falsify_all_builtin as v1274_run
        ledger = v1274_run(promethean_dir=PROMETHEAN_ROOT)
        assert isinstance(ledger, TruthLedger)
        # V1274 5 hypotheses, must still work
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

    def test_v1274_dataclass_compat(self):
        # V1278 should produce FalsifierResult that's compatible
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        for r in ledger.results:
            # Verify all expected fields present
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
        """Real subprocess invocation — 主 00:56 任何人都能接手."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1278_asi_emergence_falsifier", "--probe"],
            cwd=str(PROMETHEAN_ROOT),
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert result.returncode == 0
        assert "V1278" in result.stdout
        assert "h_artifact_growth" in result.stdout
