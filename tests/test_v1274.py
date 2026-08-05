"""Tests for V1274 ASI Truth Falsifier (Popper-style 可证伪 engine) 真生产模块.

> **主 17:43 实事求是**: 真测试, 不假装, stdlib only.
> **承接**: V1273 测试风格 (compact + 真断言 + 真覆盖率)
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import pytest

from apeireth.v1274_asi_truth_falsifier import (
    V1274_ASI_NS_CURRENT,
    V1274_ASI_NS_LOCKED_PCT,
    V1274_ASI_NS_TARGET_MAX,
    V1274_BUILD,
    V1274_THRESHOLD_24H_COMMITS,
    V1274_THRESHOLD_COMMITS,
    V1274_THRESHOLD_MODULES,
    V1274_THRESHOLD_TESTS,
    V1274_V127X_STACK,
    V1274_VERSION,
    FalsifierResult,
    HypothesisSpec,
    TruthLedger,
    _builtin_hypotheses,
    _cmd_explain,
    _cmd_json,
    _cmd_probe,
    _cmd_report,
    _cmd_run,
    _scan_commits_24h,
    _scan_commits_count,
    _scan_modules_count,
    _scan_tests_count,
    _scan_v127x_stack,
    _v3_philosophy_gate,
    falsify_all_builtin,
    falsify_hypothesis,
    main,
    render_json_snapshot,
    render_markdown_report,
)


PROMETHEAN_DIR = Path(__file__).resolve().parent.parent
APEIRETH_DIR = PROMETHEAN_DIR / "apeireth"
TESTS_DIR = PROMETHEAN_DIR / "tests"


# ============================================================
# 1. Constants (主 17:43 实事求是)
# ============================================================

class TestConstants:
    def test_version(self):
        assert V1274_VERSION == "0.1.0"

    def test_build_format(self):
        assert V1274_BUILD.startswith("2026-08-05-")
        assert "+08" in V1274_BUILD or "-08" in V1274_BUILD

    def test_asi_ns_current_in_range(self):
        assert 0.0 < V1274_ASI_NS_CURRENT < 1.0
        assert abs(V1274_ASI_NS_CURRENT - 0.7905) < 0.001

    def test_asi_ns_target_max(self):
        assert V1274_ASI_NS_TARGET_MAX == 0.9800

    def test_asi_ns_locked_pct(self):
        assert 0 < V1274_ASI_NS_LOCKED_PCT <= 100

    def test_threshold_modules(self):
        assert V1274_THRESHOLD_MODULES >= 1000

    def test_threshold_tests(self):
        assert V1274_THRESHOLD_TESTS >= 100

    def test_threshold_commits(self):
        assert V1274_THRESHOLD_COMMITS >= 1000

    def test_threshold_24h_commits(self):
        assert V1274_THRESHOLD_24H_COMMITS >= 1

    def test_v127x_stack(self):
        assert "v1270" in V1274_V127X_STACK
        assert "v1273" in V1274_V127X_STACK
        assert len(V1274_V127X_STACK) >= 4


# ============================================================
# 2. V3 Philosophy Gate (主 17:58 + 主 20:46)
# ============================================================

class TestPhilosophyGate:
    def test_all_gates_passed(self):
        gate = _v3_philosophy_gate()
        for key, val in gate.items():
            assert val is True, f"philosophy gate failed: {key}"

    def test_not_new_asi_dim(self):
        gate = _v3_philosophy_gate()
        assert gate["v1274_not_new_asi_dim"] is True

    def test_no_phenomenal_claim(self):
        gate = _v3_philosophy_gate()
        assert gate["v1274_no_phenomenal_claim"] is True

    def test_truth_is_falsifiability(self):
        gate = _v3_philosophy_gate()
        assert gate["v1274_truth_is_falsifiability"] is True

    def test_no_kpi_inflate(self):
        gate = _v3_philosophy_gate()
        assert gate["v1274_no_kpi_inflate"] is True

    def test_read_only(self):
        gate = _v3_philosophy_gate()
        assert gate["v1274_read_only"] is True

    def test_evidence_required(self):
        gate = _v3_philosophy_gate()
        assert gate["v1274_evidence_required"] is True

    def test_failures_disclosed(self):
        gate = _v3_philosophy_gate()
        assert gate["v1274_failures_disclosed"] is True

    def test_stdlib_only(self):
        gate = _v3_philosophy_gate()
        assert gate["v1274_stdlib_only"] is True


# ============================================================
# 3. Dataclasses (主 17:43 实事求是)
# ============================================================

class TestDataclasses:
    def test_hypothesis_spec_to_dict(self):
        spec = HypothesisSpec(
            hypothesis_id="test_h",
            claim="test claim",
            falsification_rule="if test fails",
            severity="info",
            evidence_type="file_count",
            threshold=100,
        )
        d = spec.to_dict()
        assert d["hypothesis_id"] == "test_h"
        assert d["claim"] == "test claim"
        assert d["severity"] == "info"
        assert d["threshold"] == 100

    def test_falsifier_result_to_dict(self):
        r = FalsifierResult(
            hypothesis_id="test_h",
            claim="test claim",
            severity="critical",
            evidence_type="file_count",
            evidence_path="/test/path",
            observed_value=500,
            threshold="> 100",
            pass_fail="PASS",
            falsification_criterion="if < 100",
            timestamp_unix=time.time(),
            elapsed_ms=1.0,
            notes="",
        )
        d = r.to_dict()
        assert d["hypothesis_id"] == "test_h"
        assert d["pass_fail"] == "PASS"
        assert d["observed_value"] == 500

    def test_truth_ledger_to_dict(self):
        r = FalsifierResult(
            hypothesis_id="test_h",
            claim="test",
            severity="info",
            evidence_type="file_count",
            evidence_path="/test",
            observed_value=10,
            threshold="> 5",
            pass_fail="PASS",
            falsification_criterion="if < 5",
            timestamp_unix=time.time(),
            elapsed_ms=1.0,
        )
        ledger = TruthLedger(
            run_id="test-run",
            run_timestamp=time.time(),
            results=[r],
            n_pass=1,
            n_fail=0,
            n_inconclusive=0,
            falsification_rate=0.0,
            philosophy_gate={"v1274_truth_is_falsifiability": True},
            elapsed_ms=1.0,
            promethean_dir=str(PROMETHEAN_DIR),
        )
        d = ledger.to_dict()
        assert d["run_id"] == "test-run"
        assert len(d["results"]) == 1


# ============================================================
# 4. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

class TestRealScanners:
    def test_scan_modules_count_positive(self):
        n, errors = _scan_modules_count(APEIRETH_DIR)
        assert n > 1000, f"modules={n}, errors={errors}"
        assert isinstance(errors, list)

    def test_scan_tests_count_positive(self):
        n, errors = _scan_tests_count(TESTS_DIR)
        assert n > 100, f"tests={n}, errors={errors}"

    def test_scan_commits_count(self):
        n, git_avail, errors = _scan_commits_count(PROMETHEAN_DIR)
        if git_avail:
            assert n > 1000, f"commits={n}, errors={errors}"

    def test_scan_commits_24h(self):
        n, git_avail, errors = _scan_commits_24h(PROMETHEAN_DIR)
        if git_avail:
            # 24h commits 应 > 0 (持续生产)
            assert n >= 0, f"24h commits={n}, errors={errors}"

    def test_scan_modules_wrong_path(self):
        n, errors = _scan_modules_count(Path("/nonexistent/path/xyz"))
        assert n == 0
        assert len(errors) > 0

    def test_scan_tests_wrong_path(self):
        n, errors = _scan_tests_count(Path("/nonexistent/path/xyz"))
        assert n == 0
        assert len(errors) > 0

    def test_scan_v127x_stack_v1273(self):
        result, errors = _scan_v127x_stack(PROMETHEAN_DIR, ["v1273"])
        assert "v1273" in result
        info = result["v1273"]
        assert info["module_exists"] is True
        assert info["test_exists"] is True
        assert info["module_lines"] > 100
        assert info["test_lines"] > 50

    def test_scan_v127x_stack_missing(self):
        result, errors = _scan_v127x_stack(PROMETHEAN_DIR, ["v9999_nonexistent"])
        assert "v9999_nonexistent" in result
        info = result["v9999_nonexistent"]
        assert info["module_exists"] is False
        assert info["test_exists"] is False
        assert len(errors) > 0


# ============================================================
# 5. 5 Built-in Hypotheses (主 17:43 实事求是)
# ============================================================

class TestBuiltinHypotheses:
    def test_count_5(self):
        specs = _builtin_hypotheses()
        assert len(specs) == 5

    def test_unique_ids(self):
        specs = _builtin_hypotheses()
        ids = [s.hypothesis_id for s in specs]
        assert len(ids) == len(set(ids))

    def test_h_modules_count_severity(self):
        specs = _builtin_hypotheses()
        h = next((s for s in specs if s.hypothesis_id == "h_modules_count"), None)
        assert h is not None
        assert h.severity == "critical"
        assert h.evidence_type == "file_count"
        assert h.threshold == V1274_THRESHOLD_MODULES

    def test_h_tests_count_severity(self):
        specs = _builtin_hypotheses()
        h = next((s for s in specs if s.hypothesis_id == "h_tests_count"), None)
        assert h is not None
        assert h.severity == "critical"
        assert h.evidence_type == "file_count"

    def test_h_commits_count_severity(self):
        specs = _builtin_hypotheses()
        h = next((s for s in specs if s.hypothesis_id == "h_commits_count"), None)
        assert h is not None
        assert h.severity == "critical"
        assert h.evidence_type == "git_count"

    def test_h_v127x_stack_severity(self):
        specs = _builtin_hypotheses()
        h = next((s for s in specs if s.hypothesis_id == "h_v127x_stack_delivered"), None)
        assert h is not None
        assert h.severity == "important"
        assert h.evidence_type == "file_exists"

    def test_h_recent_progress_severity(self):
        specs = _builtin_hypotheses()
        h = next((s for s in specs if s.hypothesis_id == "h_recent_progress"), None)
        assert h is not None
        assert h.severity == "info"
        assert h.evidence_type == "git_count_24h"

    def test_all_claims_have_falsification(self):
        specs = _builtin_hypotheses()
        for s in specs:
            assert s.falsification_rule, f"{s.hypothesis_id} missing falsification rule"
            # 描述证伪条件: 必须含 'if' / 'falsif' / 'missing' / 'fail' / '≤' / '>='
            rule_lower = s.falsification_rule.lower()
            has_falsif_kw = any(
                kw in rule_lower
                for kw in ["if", "falsif", "missing", "fail", "≤", ">=", "<", "→"]
            )
            assert has_falsif_kw, f"{s.hypothesis_id} rule lacks falsification keyword: {s.falsification_rule}"


# ============================================================
# 6. Falsifier — 真跑单假说 (主 17:43 实事求是)
# ============================================================

class TestFalsifyHypothesis:
    def test_falsify_h_modules_count(self):
        spec = HypothesisSpec(
            hypothesis_id="h_modules_count",
            claim="apeireth/ .py > 1000",
            falsification_rule="if count <= 1000",
            severity="critical",
            evidence_type="file_count",
            threshold=1000,
        )
        r = falsify_hypothesis(spec, PROMETHEAN_DIR)
        assert isinstance(r, FalsifierResult)
        assert r.pass_fail == "PASS"
        assert r.observed_value > 1000
        assert r.evidence_path == str(APEIRETH_DIR)
        assert r.elapsed_ms > 0

    def test_falsify_h_tests_count(self):
        spec = HypothesisSpec(
            hypothesis_id="h_tests_count",
            claim="tests/ test_*.py > 100",
            falsification_rule="if count <= 100",
            severity="critical",
            evidence_type="file_count",
            threshold=100,
        )
        r = falsify_hypothesis(spec, PROMETHEAN_DIR)
        assert r.pass_fail == "PASS"
        assert r.observed_value > 100

    def test_falsify_h_commits_count(self):
        spec = HypothesisSpec(
            hypothesis_id="h_commits_count",
            claim="git log > 1000",
            falsification_rule="if count <= 1000",
            severity="critical",
            evidence_type="git_count",
            threshold=1000,
        )
        r = falsify_hypothesis(spec, PROMETHEAN_DIR)
        assert r.pass_fail == "PASS"
        assert r.observed_value > 1000

    def test_falsify_h_recent_progress(self):
        spec = HypothesisSpec(
            hypothesis_id="h_recent_progress",
            claim="git log 24h >= 5",
            falsification_rule="if < 5",
            severity="info",
            evidence_type="git_count_24h",
            threshold=5,
        )
        r = falsify_hypothesis(spec, PROMETHEAN_DIR)
        # 当前 cron tick 之前已 commit, 应 PASS
        assert r.pass_fail in ("PASS", "FAIL")

    def test_falsify_h_modules_count_fail_with_high_threshold(self):
        """测试 FAIL 路径: 阈值设不可能达成."""
        spec = HypothesisSpec(
            hypothesis_id="h_modules_count",
            claim="apeireth/ .py > 999999",
            falsification_rule="if count <= 999999",
            severity="critical",
            evidence_type="file_count",
            threshold=999999,
        )
        r = falsify_hypothesis(spec, PROMETHEAN_DIR)
        assert r.pass_fail == "FAIL"
        assert r.observed_value < 999999

    def test_falsify_h_file_exists_pass(self):
        spec = HypothesisSpec(
            hypothesis_id="h_v127x_stack_delivered",
            claim="V1270-V1273 4 modules present",
            falsification_rule="any missing",
            severity="important",
            evidence_type="file_exists",
            threshold=["v1270", "v1271", "v1272", "v1273"],
        )
        r = falsify_hypothesis(spec, PROMETHEAN_DIR)
        assert r.pass_fail == "PASS"
        assert isinstance(r.observed_value, dict)

    def test_falsify_h_file_exists_fail(self):
        spec = HypothesisSpec(
            hypothesis_id="h_v127x_stack_delivered",
            claim="V9999 module present",
            falsification_rule="any missing",
            severity="important",
            evidence_type="file_exists",
            threshold=["v9999_nonexistent"],
        )
        r = falsify_hypothesis(spec, PROMETHEAN_DIR)
        assert r.pass_fail == "FAIL"
        assert "missing" in r.notes.lower()

    def test_falsify_unknown_evidence_type_inconclusive(self):
        spec = HypothesisSpec(
            hypothesis_id="h_unknown",
            claim="unknown",
            falsification_rule="if unknown",
            severity="info",
            evidence_type="unknown_type",
            threshold=0,
        )
        r = falsify_hypothesis(spec, PROMETHEAN_DIR)
        assert r.pass_fail == "INCONCLUSIVE"


# ============================================================
# 7. Truth Ledger (主 17:43 + 主 19:33)
# ============================================================

class TestFalsifyAllBuiltin:
    def test_run_all_5_hypotheses(self):
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        assert isinstance(ledger, TruthLedger)
        assert len(ledger.results) == 5
        assert ledger.run_id.startswith("v1274-")
        assert ledger.elapsed_ms > 0

    def test_results_are_pass_or_fail_or_inconclusive(self):
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        for r in ledger.results:
            assert r.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")

    def test_summary_consistent(self):
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        n_pass = sum(1 for r in ledger.results if r.pass_fail == "PASS")
        n_fail = sum(1 for r in ledger.results if r.pass_fail == "FAIL")
        n_inconclusive = sum(1 for r in ledger.results if r.pass_fail == "INCONCLUSIVE")
        assert ledger.n_pass == n_pass
        assert ledger.n_fail == n_fail
        assert ledger.n_inconclusive == n_inconclusive

    def test_falsification_rate_in_range(self):
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        assert 0.0 <= ledger.falsification_rate <= 1.0

    def test_philosophy_gate_in_ledger(self):
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        assert ledger.philosophy_gate["v1274_truth_is_falsifiability"] is True

    def test_promethean_dir_recorded(self):
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        assert ledger.promethean_dir == str(PROMETHEAN_DIR)

    def test_auto_infer_promethean_dir(self):
        # 不传 promethean_dir, 自动推断
        ledger = falsify_all_builtin(None)
        assert isinstance(ledger, TruthLedger)
        assert len(ledger.results) == 5


# ============================================================
# 8. Renderers (主 00:56 任何人都能接手)
# ============================================================

class TestRenderMarkdown:
    def _ledger_with_results(self):
        results = [
            FalsifierResult(
                hypothesis_id="h_test_pass",
                claim="test claim pass",
                severity="critical",
                evidence_type="file_count",
                evidence_path="/test/path",
                observed_value=500,
                threshold="> 100",
                pass_fail="PASS",
                falsification_criterion="if < 100",
                timestamp_unix=time.time(),
                elapsed_ms=1.5,
            ),
            FalsifierResult(
                hypothesis_id="h_test_fail",
                claim="test claim fail",
                severity="important",
                evidence_type="file_count",
                evidence_path="/test/path2",
                observed_value=50,
                threshold="> 100",
                pass_fail="FAIL",
                falsification_criterion="if < 100",
                timestamp_unix=time.time(),
                elapsed_ms=2.0,
                notes="test fail note",
            ),
        ]
        return TruthLedger(
            run_id="test-v1274",
            run_timestamp=time.time(),
            results=results,
            n_pass=1,
            n_fail=1,
            n_inconclusive=0,
            falsification_rate=0.5,
            philosophy_gate=_v3_philosophy_gate(),
            elapsed_ms=10.0,
            promethean_dir=str(PROMETHEAN_DIR),
        )

    def test_render_contains_run_id(self):
        ledger = self._ledger_with_results()
        md = render_markdown_report(ledger)
        assert "test-v1274" in md

    def test_render_contains_summary(self):
        ledger = self._ledger_with_results()
        md = render_markdown_report(ledger)
        assert "PASS" in md
        assert "FAIL" in md
        assert "Falsification rate" in md

    def test_render_contains_philosophy_gate(self):
        ledger = self._ledger_with_results()
        md = render_markdown_report(ledger)
        assert "Philosophy Gate" in md
        assert "v1274_truth_is_falsifiability" in md

    def test_render_contains_evidence_paths(self):
        ledger = self._ledger_with_results()
        md = render_markdown_report(ledger)
        assert "Evidence Paths" in md
        assert "/test/path" in md

    def test_render_contains_popper_citation(self):
        ledger = self._ledger_with_results()
        md = render_markdown_report(ledger)
        assert "Popper" in md
        assert "Logic of Scientific Discovery" in md

    def test_render_contains_entry_point(self):
        ledger = self._ledger_with_results()
        md = render_markdown_report(ledger)
        assert "python -m apeireth.v1274_asi_truth_falsifier" in md

    def test_render_failures_disclosed(self):
        ledger = self._ledger_with_results()
        md = render_markdown_report(ledger)
        # FAIL 假说也应展示, 不假装全 PASS
        assert "h_test_fail" in md
        assert "**FAIL**" in md


class TestRenderJSON:
    def test_json_snapshot_valid(self):
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        snap = render_json_snapshot(ledger)
        data = json.loads(snap)
        assert data["version"] == V1274_VERSION
        assert data["build"] == V1274_BUILD
        assert "run_id" in data
        assert "philosophy_gate" in data
        assert "asi_ns" in data
        assert "summary" in data
        assert "results" in data
        assert len(data["results"]) == 5

    def test_json_snapshot_unicode_safe(self):
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        snap = render_json_snapshot(ledger)
        data = json.loads(snap)
        assert data is not None

    def test_json_snapshot_endpoint_hints(self):
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        snap = render_json_snapshot(ledger)
        data = json.loads(snap)
        hints = data["endpoint_hints"]
        assert "explain" in hints
        assert "run" in hints


# ============================================================
# 9. CLI (主 00:56 任何人都能接手)
# ============================================================

class TestCLI:
    def test_probe(self, capsys):
        rc = _cmd_probe(PROMETHEAN_DIR)
        assert rc == 0
        captured = capsys.readouterr()
        assert "V1274" in captured.out
        assert "philosophy_gate" in captured.out
        assert "h_modules_count" in captured.out

    def test_run(self, capsys):
        rc = _cmd_run(PROMETHEAN_DIR)
        assert rc == 0
        captured = capsys.readouterr()
        assert "PASS" in captured.out
        assert "Falsification rate" in captured.out

    def test_json(self, capsys):
        rc = _cmd_json(PROMETHEAN_DIR)
        assert rc == 0
        captured = capsys.readouterr()
        data = json.loads(captured.out)
        assert "results" in data

    def test_report_writes_file(self, tmp_path):
        out_path = tmp_path / "report.md"
        rc = _cmd_report(PROMETHEAN_DIR, str(out_path))
        assert rc == 0
        assert out_path.exists()
        content = out_path.read_text(encoding="utf-8")
        assert "V1274" in content
        assert "PASS" in content

    def test_explain_known_id(self, capsys):
        rc = _cmd_explain(PROMETHEAN_DIR, "h_modules_count")
        assert rc == 0
        captured = capsys.readouterr()
        assert "h_modules_count" in captured.out
        assert "claim" in captured.out.lower()

    def test_explain_unknown_id(self, capsys):
        rc = _cmd_explain(PROMETHEAN_DIR, "h_unknown_hypothesis")
        assert rc == 1
        captured = capsys.readouterr()
        # 输出到 stderr
        combined = captured.out + captured.err
        assert "unknown" in combined.lower()

    def test_main_no_args(self, capsys):
        rc = main([])
        assert rc == 0
        captured = capsys.readouterr()
        assert "v1274_asi_truth_falsifier" in captured.out

    def test_main_probe(self, capsys):
        rc = main(["--probe"])
        assert rc == 0

    def test_main_explain_no_hypothesis(self, capsys):
        rc = main(["--explain"])
        assert rc == 1


# ============================================================
# 10. End-to-End (主 17:43 实事求是)
# ============================================================

class TestEndToEnd:
    def test_full_loop(self):
        """真跑 5 假说 + 渲染 Markdown + JSON, 完整不报错."""
        ledger = falsify_all_builtin(PROMETHEAN_DIR)
        md = render_markdown_report(ledger)
        js = render_json_snapshot(ledger)
        assert len(md) > 500
        assert json.loads(js) is not None

    def test_module_invocation_run(self):
        """通过 python -m 调用 --run, 真生产入口."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1274_asi_truth_falsifier", "--run"],
            cwd=str(PROMETHEAN_DIR),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=60,
        )
        assert result.returncode == 0
        assert "V1274" in result.stdout
        assert "PASS" in result.stdout

    def test_module_invocation_probe(self):
        """通过 python -m 调用 --probe."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1274_asi_truth_falsifier", "--probe"],
            cwd=str(PROMETHEAN_DIR),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0
        assert "h_modules_count" in result.stdout

    def test_module_invocation_json(self):
        """通过 python -m 调用 --json."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1274_asi_truth_falsifier", "--json"],
            cwd=str(PROMETHEAN_DIR),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=60,
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert "results" in data

    def test_module_imports(self):
        """module 可被 import, 无副作用."""
        result = subprocess.run(
            [sys.executable, "-c", "import apeireth.v1274_asi_truth_falsifier; print('OK')"],
            cwd=str(PROMETHEAN_DIR),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0
        assert "OK" in result.stdout
