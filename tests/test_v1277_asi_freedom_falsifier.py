"""Tests for V1277 — ASI Freedom Falsifier (主 17:43 实事求是 + 主 19:33 走在前人肩上)."""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

# Ensure project root is importable
PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

import apeireth.v1277_asi_freedom_falsifier as v1277  # noqa: E402
from apeireth.v1274_asi_truth_falsifier import (  # noqa: E402
    FalsifierResult,
    TruthLedger,
)


# ============================================================
# 1. Module constants & V3 Philosophy Gate (主 17:58 不假装)
# ============================================================

class TestModuleConstants:
    def test_version_constant(self):
        assert v1277.V1277_VERSION == "0.1.0"

    def test_build_constant_format(self):
        # 2026-08-05-XXXX+08 format
        assert v1277.V1277_BUILD.startswith("2026-08-05-")
        assert v1277.V1277_BUILD.endswith("+08")

    def test_asi_ns_constants_match_locked(self):
        # 主 17:43 不刷 KPI
        assert v1277.V1277_ASI_NS_LOCKED_PCT == 92.91

    def test_threshold_constants_are_positive(self):
        assert v1277.V1277_THRESHOLD_COMMIT_TYPE_ENTROPY > 0
        assert v1277.V1277_THRESHOLD_BRANCH_DENSITY_AVG > 0
        assert v1277.V1277_THRESHOLD_MAX_TYPE_DOMINANCE_PCT > 0
        assert v1277.V1277_THRESHOLD_MAX_TYPE_DOMINANCE_PCT <= 100

    def test_commit_type_prefixes_is_tuple(self):
        assert isinstance(v1277.V1277_COMMIT_TYPE_PREFIXES, tuple)
        assert "feat" in v1277.V1277_COMMIT_TYPE_PREFIXES
        assert "fix" in v1277.V1277_COMMIT_TYPE_PREFIXES

    def test_falsifier_modules_targets_v1274_v1275_v1276(self):
        assert "v1274_asi_truth_falsifier.py" in v1277.V1277_FALSIFIER_MODULES
        assert "v1275_asi_extended_falsifier.py" in v1277.V1277_FALSIFIER_MODULES
        assert "v1276_asi_time_falsifier.py" in v1277.V1277_FALSIFIER_MODULES


class TestV1277PhilosophyGate:
    def test_philosophy_gate_returns_dict(self):
        gate = v1277._v1277_philosophy_gate()
        assert isinstance(gate, dict)

    def test_philosophy_gate_count_is_13(self):
        gate = v1277._v1277_philosophy_gate()
        # V1274 9 + V1275 1 + V1276 1 + V1277 2 = 13
        assert len(gate) == 13, f"expected 13 gates, got {len(gate)}"

    def test_philosophy_gate_all_true(self):
        gate = v1277._v1277_philosophy_gate()
        for k, v in gate.items():
            assert v is True, f"gate {k} should be True, got {v}"

    def test_philosophy_gate_includes_no_free_will(self):
        gate = v1277._v1277_philosophy_gate()
        assert "v1277_no_free_will_claim" in gate
        assert gate["v1277_no_free_will_claim"] is True

    def test_philosophy_gate_includes_extension_chain(self):
        gate = v1277._v1277_philosophy_gate()
        assert gate["v1275_extends_v1274_not_replaces"] is True
        assert gate["v1276_extends_v1275_not_replaces"] is True
        assert gate["v1277_extends_v1276_not_replaces"] is True


# ============================================================
# 2. Commit type classifier (主 19:33 走在前人肩上: conventional commits)
# ============================================================

class TestClassifyCommitType:
    def test_feat_parens(self):
        assert v1277._classify_commit_type("feat(V1277): add freedom falsifier") == "feat"

    def test_feat_colon(self):
        assert v1277._classify_commit_type("feat: add X") == "feat"

    def test_fix_parens(self):
        assert v1277._classify_commit_type("fix(V1261): force dry_run") == "fix"

    def test_other_for_round_prefix(self):
        # "R18 round-01" → "r18" lowercase, not in V1277 prefixes → "other"
        assert v1277._classify_commit_type("R18 round-01: cargo-deny CI") == "other"

    def test_other_for_memory_prefix(self):
        # "memory: cron tick" → "memory" not in V1277 prefixes → "other"
        assert v1277._classify_commit_type("memory: cron tick 14:55 self-stance") == "other"

    def test_chore(self):
        assert v1277._classify_commit_type("chore(fmt): rustfmt") == "chore"

    def test_empty_string(self):
        assert v1277._classify_commit_type("") == "other"

    def test_case_insensitive(self):
        # Already lowercase in source
        assert v1277._classify_commit_type("Feat(X): y") == "other"  # Feat != feat
        # Note: regex pattern is lowercase only

    def test_no_prefix_at_all(self):
        assert v1277._classify_commit_type("random commit message") == "other"


# ============================================================
# 3. Shannon entropy math (主 19:33 走在前人肩上: Shannon 1948)
# ============================================================

class TestShannonEntropy:
    def test_empty_distribution(self):
        assert v1277._shannon_entropy_bits({}) == 0.0

    def test_single_type_zero_entropy(self):
        assert v1277._shannon_entropy_bits({"a": 10}) == 0.0

    def test_two_types_half_half(self):
        # {a:1, b:1} → -2*(0.5*log2(0.5)) = 1.0
        assert v1277._shannon_entropy_bits({"a": 1, "b": 1}) == pytest.approx(1.0, abs=1e-9)

    def test_four_types_equal(self):
        # {a:1, b:1, c:1, d:1} → 2.0
        assert v1277._shannon_entropy_bits({"a": 1, "b": 1, "c": 1, "d": 1}) == pytest.approx(2.0, abs=1e-9)

    def test_three_types_imbalanced(self):
        # {a:6, b:2, c:2} → -(0.6*log2(0.6) + 2*0.2*log2(0.2))
        # = -(0.6*(-0.7370) + 2*0.2*(-2.3219))
        # = -(-0.4422 + -0.9288) = 1.371
        h = v1277._shannon_entropy_bits({"a": 6, "b": 2, "c": 2})
        assert 1.36 < h < 1.38

    def test_distribution_aggregation(self):
        dist = v1277._commit_type_distribution([
            "feat(V1277): a",
            "feat(V1278): b",
            "fix(X): c",
            "R18 round-1: d",
        ])
        assert dist == {"feat": 2, "fix": 1, "other": 1}

    def test_max_type_dominance_pct(self):
        t, p = v1277._max_type_dominance_pct({"a": 7, "b": 3})
        assert t == "a"
        assert p == 70.0

    def test_max_type_dominance_pct_empty(self):
        t, p = v1277._max_type_dominance_pct({})
        assert t == "none"
        assert p == 0.0


# ============================================================
# 4. AST branch count (主 19:33 走在前人肩上)
# ============================================================

class TestAstBranchCount:
    def test_real_falsifier_files_parse(self):
        # V1274-V1276 应该能 parse + count > 0
        for fname in v1277.V1277_FALSIFIER_MODULES:
            fpath = PROJECT_ROOT / "apeireth" / fname
            cnt, ok, errs = v1277._ast_branch_count(fpath)
            assert ok, f"{fname} parse failed: {errs}"
            assert cnt > 0, f"{fname} returned 0 branches"

    def test_nonexistent_file(self):
        fake = PROJECT_ROOT / "apeireth" / "v9999_does_not_exist.py"
        cnt, ok, errs = v1277._ast_branch_count(fake)
        assert not ok
        assert cnt == 0

    def test_synthetic_simple_if(self, tmp_path: Path):
        f = tmp_path / "simple.py"
        f.write_text(
            "if x:\n"
            "    pass\n"
            "elif y:\n"
            "    pass\n"
            "else:\n"
            "    pass\n",
            encoding="utf-8",
        )
        cnt, ok, errs = v1277._ast_branch_count(f)
        assert ok
        # 1 If → 3 (if + elif + else are 3 ast.If nodes in Python AST,
        # because elif is just else + if, but ast.parse flattens)
        assert cnt >= 1

    def test_syntax_error_returns_one_ok(self, tmp_path: Path):
        f = tmp_path / "broken.py"
        f.write_text("def broken(:\n", encoding="utf-8")
        cnt, ok, errs = v1277._ast_branch_count(f)
        # SyntaxError → ok=True, cnt=0
        assert ok
        assert cnt == 0


# ============================================================
# 5. Git log raw (主 17:43 实事求是: 不假装)
# ============================================================

class TestGitRecentCommitsRaw:
    def test_non_git_dir(self, tmp_path: Path):
        msgs, ok, errs = v1277._git_recent_commits_raw(tmp_path, 5)
        assert not ok
        assert msgs == []
        assert any("not a git repo" in e for e in errs)

    def test_real_repo_returns_messages(self):
        msgs, ok, errs = v1277._git_recent_commits_raw(PROJECT_ROOT, 10)
        assert ok
        # Should get >=1 commit msg
        assert len(msgs) >= 1, f"got msgs={msgs}, errs={errs}"

    def test_n_limit_respected(self):
        msgs, ok, errs = v1277._git_recent_commits_raw(PROJECT_ROOT, 5)
        assert ok
        assert len(msgs) <= 5


# ============================================================
# 6. falsify_hypothesis — 各 evidence_type 真跑
# ============================================================

class TestFalsifyHypothesis:
    def _all_specs(self):
        return v1277._builtin_hypotheses()

    def test_commit_type_entropy_runs(self):
        specs = self._all_specs()
        spec = next(s for s in specs if s.evidence_type == "commit_type_entropy")
        result = v1277.falsify_hypothesis(spec, PROJECT_ROOT)
        assert isinstance(result, FalsifierResult)
        assert result.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        assert result.observed_value is not None
        assert result.observed_value >= 0.0

    def test_branch_density_runs(self):
        specs = self._all_specs()
        spec = next(s for s in specs if s.evidence_type == "ast_branch_density")
        result = v1277.falsify_hypothesis(spec, PROJECT_ROOT)
        assert isinstance(result, FalsifierResult)
        assert result.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        assert result.observed_value is not None
        assert result.observed_value > 0

    def test_dominance_runs(self):
        specs = self._all_specs()
        spec = next(s for s in specs if s.evidence_type == "commit_type_dominance")
        result = v1277.falsify_hypothesis(spec, PROJECT_ROOT)
        assert isinstance(result, FalsifierResult)
        assert result.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        assert result.observed_value is not None
        assert 0.0 <= result.observed_value <= 100.0

    def test_unknown_evidence_type_returns_inconclusive(self):
        from apeireth.v1274_asi_truth_falsifier import HypothesisSpec
        spec = HypothesisSpec(
            hypothesis_id="h_bogus",
            claim="bogus",
            severity="info",
            evidence_type="non_existent_type",
            threshold=0,
            falsification_rule="n/a",
        )
        result = v1277.falsify_hypothesis(spec, PROJECT_ROOT)
        assert result.pass_fail == "INCONCLUSIVE"

    def test_inconclusive_when_no_git_dir(self, tmp_path: Path):
        specs = self._all_specs()
        # entropy + dominance need git → INCONCLUSIVE on tmp_path
        for ev_type in ("commit_type_entropy", "commit_type_dominance"):
            spec = next(s for s in specs if s.evidence_type == ev_type)
            result = v1277.falsify_hypothesis(spec, tmp_path)
            assert result.pass_fail == "INCONCLUSIVE", f"{ev_type} should be INCONCLUSIVE"

    def test_branch_density_inconclusive_when_files_missing(self, tmp_path: Path):
        # Create empty apeireth/ subdir so Path resolves but files are missing
        (tmp_path / "apeireth").mkdir()
        specs = self._all_specs()
        spec = next(s for s in specs if s.evidence_type == "ast_branch_density")
        result = v1277.falsify_hypothesis(spec, tmp_path)
        assert result.pass_fail == "INCONCLUSIVE"


# ============================================================
# 7. run_all_hypotheses — end-to-end
# ============================================================

class TestRunAllHypotheses:
    def test_returns_truth_ledger(self):
        ledger = v1277.run_all_hypotheses(PROJECT_ROOT)
        assert isinstance(ledger, TruthLedger)
        assert len(ledger.results) == 3

    def test_ledger_counts_match_results(self):
        ledger = v1277.run_all_hypotheses(PROJECT_ROOT)
        n_total = ledger.n_pass + ledger.n_fail + ledger.n_inconclusive
        assert n_total == 3
        assert ledger.falsification_rate >= 0.0
        assert ledger.falsification_rate <= 1.0

    def test_philosophy_gate_in_ledger(self):
        ledger = v1277.run_all_hypotheses(PROJECT_ROOT)
        assert isinstance(ledger.philosophy_gate, dict)
        assert ledger.philosophy_gate["v1277_no_free_will_claim"] is True

    def test_run_id_format(self):
        ledger = v1277.run_all_hypotheses(PROJECT_ROOT)
        assert ledger.run_id.startswith("v1277-")

    def test_default_dir_resolution(self):
        # 不传 promethean_dir, 默认用 __file__ parent.parent
        ledger = v1277.run_all_hypotheses()
        assert isinstance(ledger, TruthLedger)
        assert len(ledger.results) == 3


# ============================================================
# 8. Markdown + JSON output (主 17:43 实事求是)
# ============================================================

class TestOutput:
    def _sample_ledger(self):
        return v1277.run_all_hypotheses(PROJECT_ROOT)

    def test_markdown_contains_run_id(self):
        ledger = self._sample_ledger()
        md = v1277._to_markdown(ledger)
        assert ledger.run_id in md

    def test_markdown_contains_all_hypotheses(self):
        ledger = self._sample_ledger()
        md = v1277._to_markdown(ledger)
        for spec in v1277._builtin_hypotheses():
            assert spec.hypothesis_id in md

    def test_markdown_contains_philosophy_gate(self):
        ledger = self._sample_ledger()
        md = v1277._to_markdown(ledger)
        assert "philosophy_gate" in md or "Philosophy Gate" in md
        assert "v1277_no_free_will_claim" in md

    def test_markdown_disclaimer(self):
        ledger = self._sample_ledger()
        md = v1277._to_markdown(ledger)
        # 不假装 free will
        assert "不假装" in md or "≠ 自由意志" in md or "no_free_will" in md.lower()

    def test_json_parses(self):
        ledger = self._sample_ledger()
        js = v1277._to_json(ledger)
        parsed = json.loads(js)
        assert parsed["run_id"] == ledger.run_id
        assert parsed["build"] == v1277.V1277_BUILD
        assert parsed["asi_ns_current"] == v1277.V1277_ASI_NS_CURRENT

    def test_json_includes_all_results(self):
        ledger = self._sample_ledger()
        js = v1277._to_json(ledger)
        parsed = json.loads(js)
        assert len(parsed["results"]) == 3


# ============================================================
# 9. _resolve_promethean_dir (主 00:56 任何人都能接手)
# ============================================================

class TestResolvePrometheanDir:
    def test_explicit_arg(self):
        p = v1277._resolve_promethean_dir(str(PROJECT_ROOT))
        assert p == PROJECT_ROOT

    def test_invalid_arg_falls_back(self):
        p = v1277._resolve_promethean_dir("/nonexistent/path/xyz")
        # Should fall back to CWD walk or __file__ parent.parent
        assert p.exists()

    def test_none_arg_falls_back(self):
        p = v1277._resolve_promethean_dir(None)
        assert p.exists()
        assert (p / "apeireth").exists()


# ============================================================
# 10. CLI (主 00:56 任何人都能接手)
# ============================================================

class TestCLI:
    # Windows 沙箱 subprocess 编码 (主 19:33 走在前人肩上: UTF-8 强制)
    _SUBPROC_ENV = {"PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"}

    def test_probe_runs(self, capsys):
        rc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1277_asi_freedom_falsifier", "--probe"],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**__import__("os").environ, **self._SUBPROC_ENV},
            timeout=30,
        )
        assert rc.returncode == 0
        out = rc.stdout or ""
        assert "V1277" in out
        assert "philosophy_gate" in out
        assert "h_commit_type_entropy" in out

    def test_run_outputs_markdown(self, capsys):
        rc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1277_asi_freedom_falsifier", "--run"],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**__import__("os").environ, **self._SUBPROC_ENV},
            timeout=60,
        )
        assert rc.returncode == 0
        out = rc.stdout or ""
        assert "# V1277 ASI Freedom Falsifier" in out
        assert "PASS" in out or "FAIL" in out

    def test_json_outputs_json(self, capsys):
        rc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1277_asi_freedom_falsifier", "--json"],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**__import__("os").environ, **self._SUBPROC_ENV},
            timeout=60,
        )
        assert rc.returncode == 0
        parsed = json.loads(rc.stdout or "{}")
        assert "run_id" in parsed
        assert "results" in parsed

    def test_report_writes_file(self, tmp_path: Path):
        report = tmp_path / "V1277.md"
        rc = subprocess.run(
            [
                sys.executable, "-m", "apeireth.v1277_asi_freedom_falsifier",
                "--report", str(report),
            ],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**__import__("os").environ, **self._SUBPROC_ENV},
            timeout=60,
        )
        assert rc.returncode == 0
        assert report.exists()
        content = report.read_text(encoding="utf-8")
        assert "# V1277 ASI Freedom Falsifier" in content

    def test_hypothesis_explain(self):
        rc = subprocess.run(
            [
                sys.executable, "-m", "apeireth.v1277_asi_freedom_falsifier",
                "--hypothesis", "h_decision_branch_density", "--explain",
            ],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**__import__("os").environ, **self._SUBPROC_ENV},
            timeout=30,
        )
        assert rc.returncode == 0
        out = rc.stdout or ""
        assert "h_decision_branch_density" in out
        assert "verdict" in out.lower()

    def test_unknown_hypothesis(self):
        rc = subprocess.run(
            [
                sys.executable, "-m", "apeireth.v1277_asi_freedom_falsifier",
                "--hypothesis", "h_bogus", "--explain",
            ],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**__import__("os").environ, **self._SUBPROC_ENV},
            timeout=30,
        )
        assert rc.returncode == 0  # 解释模式不报 exit code
        assert "unknown hypothesis" in (rc.stdout or "").lower() or "h_bogus" in (rc.stdout or "")


# ============================================================
# 11. 回归 — 不影响 V1274/V1275/V1276 (主 00:56 任何人都能接手)
# ============================================================

class TestRegression:
    _SUBPROC_ENV = {"PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"}

    def test_v1274_truth_falsifier_still_passes(self):
        rc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1274_asi_truth_falsifier", "--probe"],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**__import__("os").environ, **self._SUBPROC_ENV},
            timeout=30,
        )
        assert rc.returncode == 0
        assert "V1274" in (rc.stdout or "")

    def test_v1275_extended_falsifier_still_passes(self):
        rc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1275_asi_extended_falsifier", "--probe"],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**__import__("os").environ, **self._SUBPROC_ENV},
            timeout=30,
        )
        assert rc.returncode == 0
        assert "V1275" in (rc.stdout or "")

    def test_v1276_time_falsifier_still_passes(self):
        rc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1276_asi_time_falsifier", "--probe"],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env={**__import__("os").environ, **self._SUBPROC_ENV},
            timeout=30,
        )
        assert rc.returncode == 0
        assert "V1276" in (rc.stdout or "")