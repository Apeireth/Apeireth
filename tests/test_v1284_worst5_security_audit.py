"""Tests for V1284 Worst-5 Rust Security Depth Audit — 真生产 tests

> 主 17:43 实事求是: 真 tests, 0 skip, 0 fake.
> 主 19:33 走在前人肩上: 继承 V1280 + V1281 + V1282 + V1283 dataclasses / falsifier pattern.
> 主 13:31 大胆激进 + 主 23:44 干到底 + 主 00:56 任何人都能接手.
> 主 17:58 不假装: 不刷 KPI, 不假装 ASI V1, FAIL 也展示.
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
from unittest import mock

import pytest

PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1284_worst5_security_audit import (
    V1284_VERSION,
    V1284_BUILD,
    V1284_ASI_NS_CURRENT,
    V1284_ASI_NS_LOCKED_PCT,
    V1284_WORST5_CRATES,
    V1284_THRESHOLD_UNWRAP,
    V1284_THRESHOLD_EXPECT,
    V1284_THRESHOLD_PANIC,
    V1284_THRESHOLD_TODO,
    V1284_THRESHOLD_UNSAFE,
    SecurityFinding,
    CrateSecurityMetrics,
    CrateHypothesisResult,
    Worst5SecurityLedger,
    SECURITY_PATTERNS,
    _COMPILED,
    _v1284_philosophy_gate,
    _strip_line_comment,
    _classify_severity,
    _suggest_fix,
    resolve_promethean_dir,
    find_crate_src,
    scan_crate,
    falsify_zero_unwrap,
    falsify_zero_expect,
    falsify_zero_panic,
    falsify_zero_todo,
    falsify_zero_unsafe,
    FALSIFIER_DISPATCH,
    run_worst5_audit,
    _to_json_snapshot,
    _to_markdown,
    _format_findings_table,
    main,
)


# ============================================================
# Constants
# ============================================================

class TestConstants:
    def test_version(self):
        assert V1284_VERSION == "0.1.0"

    def test_build_format(self):
        assert V1284_BUILD.startswith("2026-08-05-")
        assert V1284_BUILD.endswith("+08")

    def test_asi_ns_current(self):
        # 主 22:33 LOCKED (ceiling V0.1 = 0.7905)
        assert V1284_ASI_NS_CURRENT == 0.7905

    def test_asi_ns_locked_pct(self):
        assert V1284_ASI_NS_LOCKED_PCT == 92.91

    def test_thresholds_all_zero(self):
        # V1284 zero-tolerance in production src/
        assert V1284_THRESHOLD_UNWRAP == 0
        assert V1284_THRESHOLD_EXPECT == 0
        assert V1284_THRESHOLD_PANIC == 0
        assert V1284_THRESHOLD_TODO == 0
        assert V1284_THRESHOLD_UNSAFE == 0

    def test_worst5_count(self):
        assert len(V1284_WORST5_CRATES) == 5

    def test_worst5_includes_known_crates(self):
        for c in ("apeireth-formal", "apeireth-tauri-stub", "apeireth-vector",
                  "apeireth-cli", "apeireth-consciousness"):
            assert c in V1284_WORST5_CRATES


# ============================================================
# Patterns
# ============================================================

class TestPatterns:
    def test_pattern_count(self):
        assert len(SECURITY_PATTERNS) == 6

    def test_pattern_keys(self):
        expected = {
            "unwrap_call", "expect_call", "panic_macro",
            "todo_macro", "unimplemented_macro", "unsafe_block",
        }
        assert set(SECURITY_PATTERNS.keys()) == expected

    def test_unwrap_pattern_matches_dot_unwrap(self):
        p = _COMPILED["unwrap_call"]
        assert p.search("foo.unwrap()") is not None
        # .unwrap_or 不会误匹配 — 因为 pattern 要求 `.unwrap()` 字面量 (含 ()), 而 .unwrap_or 后是 `_or(`
        assert p.search("foo.unwrap_or(0)") is None
        assert p.search("rewrap()") is None  # no leading dot
        assert p.search("unwrap()") is None  # no leading dot
        # 闭括号后面接 .unwrap() 也应匹配 (e.g. open_in_memory().unwrap())
        assert p.search("open_in_memory().unwrap()") is not None

    def test_expect_pattern(self):
        p = _COMPILED["expect_call"]
        assert p.search("foo.expect(\"bad\")") is not None
        assert p.search("foo_expect") is None  # identifier, not method call

    def test_panic_pattern(self):
        p = _COMPILED["panic_macro"]
        assert p.search("panic!(\"oops\")") is not None
        assert p.search("panic!{x}") is not None
        assert p.search("mypanic()") is None  # word boundary

    def test_todo_pattern(self):
        p = _COMPILED["todo_macro"]
        assert p.search("todo!()") is not None
        assert p.search("todo_macro") is None

    def test_unimplemented_pattern(self):
        p = _COMPILED["unimplemented_macro"]
        assert p.search("unimplemented!()") is not None
        assert p.search("fn unimplemented_helper()") is None

    def test_unsafe_pattern(self):
        p = _COMPILED["unsafe_block"]
        assert p.search("unsafe {") is not None
        assert p.search("unsafe fn foo()") is not None
        assert p.search("unsafe impl Send") is not None
        # Note: comment lines containing "unsafe" should NOT match — but this is the regex test only;
        # comment handling is in scan_crate / _strip_line_comment


# ============================================================
# Comment stripping
# ============================================================

class TestStripLineComment:
    def test_simple_comment(self):
        assert _strip_line_comment("x = 5; // comment") == "x = 5; "

    def test_no_comment(self):
        assert _strip_line_comment("x = 5;") == "x = 5;"

    def test_full_line_comment(self):
        assert _strip_line_comment("// just a comment") == ""

    def test_string_with_slashes(self):
        # `//` inside string literal should NOT be stripped
        s = 'let x = "http://example.com"; // comment'
        assert _strip_line_comment(s) == 'let x = "http://example.com"; '

    def test_string_with_escaped_quote(self):
        s = r'let x = "he said \"hi//there\""; // real comment'
        result = _strip_line_comment(s)
        # The \" should be treated as escape, so string stays open
        assert "real comment" not in result

    def test_char_literal_with_slashes(self):
        # char literal containing // should not be stripped
        s = "let c = '/'; // comment"
        result = _strip_line_comment(s)
        assert "/'; " in result


# ============================================================
# Severity + suggestion
# ============================================================

class TestClassifySeverity:
    def test_unwrap_is_critical(self):
        assert _classify_severity("unwrap_call", 1) == "critical"
        assert _classify_severity("unwrap_call", 0) == "info"

    def test_expect_is_important(self):
        assert _classify_severity("expect_call", 1) == "important"
        assert _classify_severity("expect_call", 0) == "info"

    def test_panic_is_critical(self):
        assert _classify_severity("panic_macro", 1) == "critical"

    def test_todo_is_critical(self):
        assert _classify_severity("todo_macro", 1) == "critical"

    def test_unimplemented_is_critical(self):
        assert _classify_severity("unimplemented_macro", 1) == "critical"

    def test_unsafe_is_critical(self):
        assert _classify_severity("unsafe_block", 1) == "critical"


class TestSuggestFix:
    def test_unwrap_fix(self):
        fix = _suggest_fix("unwrap_call")
        assert "?" in fix or "match" in fix

    def test_expect_fix(self):
        fix = _suggest_fix("expect_call")
        assert "expect" in fix.lower()

    def test_panic_fix(self):
        fix = _suggest_fix("panic_macro")
        assert "Result" in fix

    def test_unsafe_fix(self):
        fix = _suggest_fix("unsafe_block")
        assert "SAFETY" in fix or "safe" in fix


# ============================================================
# Dataclasses
# ============================================================

class TestSecurityFinding:
    def test_construction(self):
        f = SecurityFinding(
            crate_name="apeireth-vector",
            pattern_id="unwrap_call",
            file_path="/tmp/foo.rs",
            line_number=42,
            line_text="foo.unwrap()",
            severity="critical",
            notes="use ?",
        )
        assert f.crate_name == "apeireth-vector"
        assert f.pattern_id == "unwrap_call"
        assert f.line_number == 42

    def test_to_dict(self):
        f = SecurityFinding(
            crate_name="x", pattern_id="unwrap_call", file_path="f",
            line_number=1, line_text="x", severity="critical",
        )
        d = f.to_dict()
        assert d["crate_name"] == "x"
        assert d["pattern_id"] == "unwrap_call"


class TestCrateSecurityMetrics:
    def test_total_hotspots_empty(self):
        m = CrateSecurityMetrics(crate_name="x", crate_src="x", src_files_scanned=0, src_lines_scanned=0)
        assert m.n_total_hotspots == 0

    def test_total_hotspots_aggregated(self):
        m = CrateSecurityMetrics(
            crate_name="x", crate_src="x",
            src_files_scanned=1, src_lines_scanned=100,
            n_unwrap=3, n_expect=2, n_panic=1, n_todo=0,
            n_unimplemented=1, n_unsafe=1,
        )
        assert m.n_total_hotspots == 8


class TestCrateHypothesisResult:
    def test_construction(self):
        r = CrateHypothesisResult(
            crate_name="x", hypothesis_id="h_zero_unwrap_in_production_src",
            claim="x zero unwrap",
            severity="critical",
            observed_value=0.0, threshold=0.0,
            pass_fail="PASS", notes="clean",
        )
        assert r.pass_fail == "PASS"


class TestWorst5SecurityLedger:
    def test_empty_ledger(self):
        l = Worst5SecurityLedger()
        assert l.n_pass == 0
        assert l.n_fail == 0
        assert l.n_inconclusive == 0
        assert l.total_hotspots == 0

    def test_aggregates(self):
        m1 = CrateSecurityMetrics(
            crate_name="a", crate_src="a",
            src_files_scanned=1, src_lines_scanned=100,
            n_unwrap=2,
        )
        m2 = CrateSecurityMetrics(
            crate_name="b", crate_src="b",
            src_files_scanned=1, src_lines_scanned=100,
            n_expect=3,
        )
        results = [
            CrateHypothesisResult(crate_name="a", hypothesis_id="h1", claim="x",
                                  severity="critical", pass_fail="FAIL"),
            CrateHypothesisResult(crate_name="a", hypothesis_id="h2", claim="y",
                                  severity="critical", pass_fail="PASS"),
            CrateHypothesisResult(crate_name="b", hypothesis_id="h3", claim="z",
                                  severity="critical", pass_fail="PASS"),
        ]
        l = Worst5SecurityLedger(crates_audited=2, crate_metrics=[m1, m2], results=results)
        assert l.n_pass == 2
        assert l.n_fail == 1
        assert l.total_hotspots == 5


# ============================================================
# V3 Philosophy Gate
# ============================================================

class TestPhilosophyGate:
    def test_inherited_21_gates(self):
        gate = _v1284_philosophy_gate()
        assert sum(1 for k in gate if k.startswith("v1283_inherited_gate_")) == 21

    def test_v1284_new_gates(self):
        gate = _v1284_philosophy_gate()
        assert gate["v1284_extends_v1283_not_replaces"] is True
        assert gate["v1284_audit_only_no_fix"] is True
        assert gate["v1284_production_src_only"] is True

    def test_total_gates_count(self):
        gate = _v1284_philosophy_gate()
        # 21 inherited + 3 new = 24
        assert len(gate) == 24

    def test_all_gates_true(self):
        gate = _v1284_philosophy_gate()
        for k, v in gate.items():
            assert v is True, f"gate {k} should be True"


# ============================================================
# Resolvers / finders
# ============================================================

class TestResolvePrometheanDir:
    def test_default_resolution(self):
        pd = resolve_promethean_dir()
        # Either the workspace or the promethean subdir
        assert pd.is_dir()

    def test_explicit_path(self):
        explicit = Path.cwd()
        pd = resolve_promethean_dir(str(explicit))
        assert pd == explicit.resolve()


class TestFindCrateSrc:
    def test_find_known_crate(self):
        pd = resolve_promethean_dir()
        src = find_crate_src("apeireth-formal", pd)
        assert src is not None
        assert src.is_dir()
        assert any(src.glob("*.rs"))

    def test_unknown_crate(self):
        pd = resolve_promethean_dir()
        assert find_crate_src("apeireth-nonexistent", pd) is None


# ============================================================
# Scanner (real data)
# ============================================================

class TestScanCrate:
    def test_scan_apeireth_formal_clean(self):
        pd = resolve_promethean_dir()
        src = find_crate_src("apeireth-formal", pd)
        assert src is not None
        m = scan_crate("apeireth-formal", src)
        assert m.crate_name == "apeireth-formal"
        assert m.src_files_scanned >= 1
        # apeireth-formal is known clean
        assert m.n_total_hotspots == 0

    def test_scan_apeireth_vector_finds_unwraps(self):
        pd = resolve_promethean_dir()
        src = find_crate_src("apeireth-vector", pd)
        assert src is not None
        m = scan_crate("apeireth-vector", src)
        assert m.crate_name == "apeireth-vector"
        # Real data: apeireth-vector has 26 unwraps in sqlite_backend.rs
        assert m.n_unwrap >= 20
        assert m.n_unsafe == 0  # the comment-only match must be filtered

    def test_scan_apeireth_consciousness_finds_unwraps_and_panic(self):
        pd = resolve_promethean_dir()
        src = find_crate_src("apeireth-consciousness", pd)
        assert src is not None
        m = scan_crate("apeireth-consciousness", src)
        assert m.n_unwrap >= 5
        assert m.n_panic >= 1

    def test_scan_apeireth_tauri_stub_finds_expect(self):
        pd = resolve_promethean_dir()
        src = find_crate_src("apeireth-tauri-stub", pd)
        assert src is not None
        m = scan_crate("apeireth-tauri-stub", src)
        assert m.n_expect >= 1

    def test_scan_apeireth_cli_clean(self):
        pd = resolve_promethean_dir()
        src = find_crate_src("apeireth-cli", pd)
        assert src is not None
        m = scan_crate("apeireth-cli", src)
        assert m.n_total_hotspots == 0

    def test_findings_have_valid_locations(self):
        pd = resolve_promethean_dir()
        src = find_crate_src("apeireth-vector", pd)
        m = scan_crate("apeireth-vector", src)
        for f in m.findings:
            assert f.line_number > 0
            assert f.severity in ("critical", "important", "info")
            assert f.pattern_id in SECURITY_PATTERNS
            # File path should be under the crate_src
            assert str(src) in f.file_path


# ============================================================
# Falsifiers
# ============================================================

class TestFalsifiers:
    def _make_metrics(self, **kw) -> CrateSecurityMetrics:
        defaults = dict(
            crate_name="x", crate_src="x",
            src_files_scanned=1, src_lines_scanned=100,
        )
        defaults.update(kw)
        return CrateSecurityMetrics(**defaults)

    def test_zero_unwrap_pass(self):
        m = self._make_metrics(n_unwrap=0)
        r = falsify_zero_unwrap(m)
        assert r.pass_fail == "PASS"
        assert r.observed_value == 0.0

    def test_zero_unwrap_fail(self):
        m = self._make_metrics(n_unwrap=5)
        r = falsify_zero_unwrap(m)
        assert r.pass_fail == "FAIL"
        assert r.observed_value == 5.0

    def test_zero_expect_pass(self):
        m = self._make_metrics(n_expect=0)
        r = falsify_zero_expect(m)
        assert r.pass_fail == "PASS"

    def test_zero_expect_fail(self):
        m = self._make_metrics(n_expect=3)
        r = falsify_zero_expect(m)
        assert r.pass_fail == "FAIL"

    def test_zero_panic(self):
        m_panic = self._make_metrics(n_panic=0)
        m_safe = self._make_metrics(n_panic=0)
        m_bad = self._make_metrics(n_panic=1)
        assert falsify_zero_panic(m_panic).pass_fail == "PASS"
        assert falsify_zero_panic(m_safe).pass_fail == "PASS"
        assert falsify_zero_panic(m_bad).pass_fail == "FAIL"

    def test_zero_todo_includes_unimplemented(self):
        m = self._make_metrics(n_todo=0, n_unimplemented=1)
        r = falsify_zero_todo(m)
        assert r.pass_fail == "FAIL"
        assert r.observed_value == 1.0

    def test_zero_unsafe(self):
        m_clean = self._make_metrics(n_unsafe=0)
        m_bad = self._make_metrics(n_unsafe=1)
        assert falsify_zero_unsafe(m_clean).pass_fail == "PASS"
        assert falsify_zero_unsafe(m_bad).pass_fail == "FAIL"

    def test_all_falsifiers_dispatched(self):
        for hyp_id in FALSIFIER_DISPATCH:
            assert hyp_id in (
                "h_zero_unwrap_in_production_src",
                "h_zero_expect_in_production_src",
                "h_zero_panic_in_production_src",
                "h_zero_todo_in_production_src",
                "h_zero_unsafe_in_production_src",
            )


# ============================================================
# Runner
# ============================================================

class TestRunner:
    def test_run_full_audit(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd)
        assert ledger.crates_audited == 5
        # 5 crates × 5 hypotheses = 25 results
        assert len(ledger.results) == 25
        # Each crate should appear in metrics
        assert len(ledger.crate_metrics) == 5
        # Real data: apeireth-vector has unwraps
        vec = next(m for m in ledger.crate_metrics if m.crate_name == "apeireth-vector")
        assert vec.n_unwrap >= 20

    def test_run_with_filter(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd, crate_filter=lambda c: c == "apeireth-formal")
        assert ledger.crates_audited == 1
        assert len(ledger.results) == 5  # 5 hypotheses for 1 crate
        only = ledger.crate_metrics[0]
        assert only.crate_name == "apeireth-formal"

    def test_run_no_unwrap_in_formal_passes_unwrap_hyp(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd, crate_filter=lambda c: c == "apeireth-formal")
        hyp_unwrap = next(r for r in ledger.results
                          if r.hypothesis_id == "h_zero_unwrap_in_production_src")
        assert hyp_unwrap.pass_fail == "PASS"

    def test_run_vector_unwrap_fails(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd, crate_filter=lambda c: c == "apeireth-vector")
        hyp_unwrap = next(r for r in ledger.results
                          if r.hypothesis_id == "h_zero_unwrap_in_production_src")
        assert hyp_unwrap.pass_fail == "FAIL"
        assert hyp_unwrap.observed_value >= 20

    def test_run_elapsed_tracked(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd)
        assert ledger.elapsed_ms >= 0


# ============================================================
# Markdown / JSON output
# ============================================================

class TestOutput:
    def test_to_markdown_contains_header(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd)
        md = _to_markdown(ledger)
        assert "# V1284 Worst-5 Rust Security Depth Audit" in md
        assert "V1284_WORST5_CRATES" not in md  # placeholder leak check

    def test_to_markdown_contains_findings(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd)
        md = _to_markdown(ledger)
        # Should contain at least one finding reference for apeireth-vector
        assert "apeireth-vector" in md
        assert "sqlite_backend.rs" in md

    def test_to_markdown_includes_disclaimers(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd)
        md = _to_markdown(ledger)
        assert "不假装" in md or "不刷" in md
        assert "worst-5" in md
        assert "production src/" in md or "production" in md

    def test_to_markdown_includes_v1283_reference(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd)
        md = _to_markdown(ledger)
        assert "V1283" in md
        assert "ASI NS" in md or "ceiling" in md

    def test_to_json_valid(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd)
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        assert d["crates_audited"] == 5
        assert len(d["per_crate_metrics"]) == 5
        assert len(d["results"]) == 25
        # Each crate should have expected fields
        for cm in d["per_crate_metrics"]:
            assert "n_unwrap" in cm
            assert "n_expect" in cm
            assert "n_panic" in cm
            assert "n_unsafe" in cm

    def test_to_json_roundtrip(self):
        pd = resolve_promethean_dir()
        ledger = run_worst5_audit(promethean_dir=pd)
        js = _to_json_snapshot(ledger)
        d = json.loads(js)
        # Validate structure
        assert "philosophy_gate" in d
        assert "worst5_crates" in d
        assert d["worst5_crates"] == [m.crate_name for m in ledger.crate_metrics]


class TestFormatFindingsTable:
    def test_format_empty(self):
        s = _format_findings_table([], "/tmp")
        assert "(no findings)" in s

    def test_format_with_findings(self):
        f = SecurityFinding(
            crate_name="x", pattern_id="unwrap_call",
            file_path="/tmp/foo.rs", line_number=10,
            line_text="x.unwrap()", severity="critical",
        )
        s = _format_findings_table([f], "/tmp")
        assert "unwrap_call" in s
        assert "foo.rs:10" in s


# ============================================================
# CLI
# ============================================================

class TestCLI:
    def test_probe_mode(self, capsys):
        code = main(["--probe"])
        out = capsys.readouterr().out
        assert code == 0
        assert "Worst-5 crates" in out
        assert "apeireth-vector" in out
        assert "Hypotheses: 5" in out

    def test_run_mode(self, capsys):
        code = main(["--run"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1284 Worst-5" in out
        assert "PASS" in out

    def test_json_mode(self, capsys):
        code = main(["--json"])
        out = capsys.readouterr().out
        assert code == 0
        d = json.loads(out)
        assert "crates_audited" in d
        assert d["crates_audited"] == 5

    def test_report_mode(self, capsys, tmp_path):
        out_path = tmp_path / "report.md"
        code = main(["--report", str(out_path)])
        assert code == 0
        assert out_path.exists()
        content = out_path.read_text(encoding="utf-8")
        assert "V1284 Worst-5" in content
        captured = capsys.readouterr().out
        assert "wrote report" in captured

    def test_crate_filter(self, capsys):
        code = main(["--run", "--crate", "apeireth-vector"])
        out = capsys.readouterr().out
        assert code == 0
        assert "apeireth-vector" in out

    def test_crate_invalid(self, capsys):
        code = main(["--run", "--crate", "apeireth-nonexistent"])
        captured = capsys.readouterr()
        assert code == 2
        assert "ERROR" in captured.err

    def test_severity_filter(self, capsys):
        code = main(["--run", "--severity", "critical"])
        out = capsys.readouterr().out
        assert code == 0
        assert "Filtered findings" in out

    def test_module_invocation(self):
        """Run as module and verify exit code 0."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1284_worst5_security_audit", "--probe"],
            capture_output=True, text=True, cwd=str(PROMETHEAN_ROOT),
            encoding="utf-8", errors="replace",
        )
        assert result.returncode == 0
        # subprocess may emit stderr/stdout via GBK on Windows; tolerate with errors="replace".
        # Probe output may include both English ("Worst-5") and Chinese; check stdout + stderr combined.
        combined = (result.stdout or "") + (result.stderr or "")
        assert "Worst-5" in combined or "apeireth-formal" in combined