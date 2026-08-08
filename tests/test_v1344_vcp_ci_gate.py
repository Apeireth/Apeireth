#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1344_vcp_ci_gate.py — tests for V1344 VCP CI Gate Integration

Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(APEIRETH_DIR))

import v1344_vcp_ci_gate as v1344  # noqa: E402


# --- CIGateConfig ----------------------------------------------------------
class TestCIGateConfig:
    def test_default_config(self):
        cfg = v1344.CIGateConfig()
        assert cfg.tier_min == "high"
        assert cfg.fail_on_coverage_loss is True
        assert cfg.max_critical_failures == 0
        assert cfg.fail_on_unclassified is False
        assert cfg.baseline_coverage is None
        assert cfg.format == "markdown"

    def test_config_to_dict(self):
        cfg = v1344.CIGateConfig(tier_min="medium", max_critical_failures=2)
        d = cfg.to_dict()
        assert d["tier_min"] == "medium"
        assert d["max_critical_failures"] == 2
        assert "fail_on_coverage_loss" in d

    def test_config_validation(self):
        cfg = v1344.CIGateConfig(tier_min="all", fail_on_coverage_loss=False)
        assert cfg.tier_min == "all"


# --- CIGateResult ----------------------------------------------------------
class TestCIGateResult:
    def test_default_result(self):
        cfg = v1344.CIGateConfig()
        r = v1344.CIGateResult(
            passed=False, exit_code=1, config=cfg, summary={}
        )
        assert r.passed is False
        assert r.exit_code == 1
        assert r.violations == []
        assert r.coverage == {}

    def test_result_to_dict(self):
        cfg = v1344.CIGateConfig()
        r = v1344.CIGateResult(
            passed=True, exit_code=0, config=cfg,
            summary={"key": "value"}, violations=[{"ruleId": "x"}],
        )
        d = r.to_dict()
        assert d["passed"] is True
        assert d["summary"]["key"] == "value"
        assert d["violations"][0]["ruleId"] == "x"


# --- Helpers ---------------------------------------------------------------
class TestHelpers:
    def test_ledger_hash_format(self):
        h = v1344._ledger_hash()
        assert len(h) == 16
        assert all(c in "0123456789abcdef" for c in h)

    def test_ledger_hash_stable(self):
        h1 = v1344._ledger_hash()
        h2 = v1344._ledger_hash()
        assert h1 == h2

    def test_now_iso_format(self):
        ts = v1344._now_iso()
        assert "T" in ts
        assert ts.endswith("+00:00") or ts.endswith("Z") or "+" in ts

    def test_get_modules(self):
        modules = v1344._get_modules()
        assert isinstance(modules, list)
        assert len(modules) > 0
        assert all("module_filename" in m for m in modules)

    def test_get_ledger(self):
        ledger = v1344._get_ledger()
        assert isinstance(ledger, list)
        assert len(ledger) > 0
        assert hasattr(ledger[0], "substrate_name")


# --- Main gate runner ------------------------------------------------------
class TestLintV1335LedgerCi:
    def test_default_run_passes_structure(self):
        cfg = v1344.CIGateConfig()
        r = v1344.lint_v1335_ledger_ci(cfg)
        assert r is not None
        assert isinstance(r.summary, dict)
        assert isinstance(r.tier_breakdown, dict)
        assert "ledger_hash" in r.__dataclass_fields__ or hasattr(r, "ledger_hash")
        assert r.ledger_hash != ""

    def test_coverage_is_numeric(self):
        r = v1344.lint_v1335_ledger_ci()
        assert isinstance(r.coverage.get("current"), float)
        assert isinstance(r.coverage.get("baseline"), float)

    def test_tier_breakdown_has_all_tiers(self):
        r = v1344.lint_v1335_ledger_ci()
        for tier in ("HIGH", "MEDIUM", "LOW", "UNCLASSIFIED"):
            assert tier in r.tier_breakdown

    def test_ledger_hash_consistent(self):
        r = v1344.lint_v1335_ledger_ci()
        h1 = v1344._ledger_hash()
        assert r.ledger_hash == h1

    def test_timestamp_set(self):
        r = v1344.lint_v1335_ledger_ci()
        assert r.timestamp != ""

    def test_tier_min_all_has_fewer_violations(self):
        r_all = v1344.lint_v1335_ledger_ci(v1344.CIGateConfig(tier_min="all"))
        r_high = v1344.lint_v1335_ledger_ci(v1344.CIGateConfig(tier_min="high"))
        tier_violations_all = [v for v in r_all.violations if v.get("ruleId") == "tier-below-threshold"]
        tier_violations_high = [v for v in r_high.violations if v.get("ruleId") == "tier-below-threshold"]
        assert len(tier_violations_all) == 0
        assert len(tier_violations_high) > 0

    def test_coverage_loss_detection(self):
        cfg = v1344.CIGateConfig(tier_min="high", fail_on_coverage_loss=True, baseline_coverage=1.5)
        r = v1344.lint_v1335_ledger_ci(cfg)
        cov_violations = [v for v in r.violations if v.get("ruleId") == "coverage-loss"]
        assert len(cov_violations) > 0

    def test_no_coverage_loss_when_baseline_lower(self):
        cfg = v1344.CIGateConfig(tier_min="all", fail_on_coverage_loss=True, baseline_coverage=0.0)
        r = v1344.lint_v1335_ledger_ci(cfg)
        cov_violations = [v for v in r.violations if v.get("ruleId") == "coverage-loss"]
        assert len(cov_violations) == 0

    def test_fail_on_unclassified(self):
        cfg = v1344.CIGateConfig(tier_min="high", fail_on_unclassified=True)
        r = v1344.lint_v1335_ledger_ci(cfg)
        unclass_violations = [v for v in r.violations if v.get("ruleId") == "unclassified-substrates"]
        assert len(unclass_violations) > 0

    def test_duplicate_substrates(self):
        r = v1344.lint_v1335_ledger_ci()
        assert isinstance(r.duplicate_substrates, list)


# --- SARIF formatter -------------------------------------------------------
class TestToSarif:
    def test_sarif_schema(self):
        r = v1344.lint_v1335_ledger_ci()
        sarif = v1344.to_sarif(r)
        assert sarif["$schema"] == v1344.SARIF_SCHEMA
        assert sarif["version"] == "2.1.0"

    def test_sarif_has_runs(self):
        r = v1344.lint_v1335_ledger_ci()
        sarif = v1344.to_sarif(r)
        assert "runs" in sarif
        assert len(sarif["runs"]) > 0

    def test_sarif_tool_name(self):
        r = v1344.lint_v1335_ledger_ci()
        sarif = v1344.to_sarif(r)
        driver = sarif["runs"][0]["tool"]["driver"]
        assert driver["name"] == "apeireth-vcp-linter"
        assert driver["version"] == "0.1.0"

    def test_sarif_results_array(self):
        r = v1344.lint_v1335_ledger_ci()
        sarif = v1344.to_sarif(r)
        assert "results" in sarif["runs"][0]
        assert isinstance(sarif["runs"][0]["results"], list)

    def test_sarif_with_violations(self):
        cfg = v1344.CIGateConfig(tier_min="high", fail_on_coverage_loss=True, baseline_coverage=1.5)
        r = v1344.lint_v1335_ledger_ci(cfg)
        sarif = v1344.to_sarif(r)
        assert len(sarif["runs"][0]["results"]) > 0

    def test_sarif_rule_levels(self):
        cfg = v1344.CIGateConfig(tier_min="high", fail_on_coverage_loss=True, baseline_coverage=1.5)
        r = v1344.lint_v1335_ledger_ci(cfg)
        sarif = v1344.to_sarif(r)
        rules = sarif["runs"][0]["tool"]["driver"]["rules"]
        assert isinstance(rules, list)
        assert any("id" in rule for rule in rules)

    def test_sarif_json_serializable(self):
        r = v1344.lint_v1335_ledger_ci()
        sarif = v1344.to_sarif(r)
        json.dumps(sarif)  # must not raise

    def test_sarif_properties(self):
        r = v1344.lint_v1335_ledger_ci()
        sarif = v1344.to_sarif(r)
        props = sarif["runs"][0]["properties"]
        assert "ledger_hash" in props
        assert "tier_min" in props
        assert "tier_breakdown" in props


# --- GitHub Actions summary -------------------------------------------------
class TestToGithubActionsSummary:
    def test_summary_has_status(self):
        r = v1344.lint_v1335_ledger_ci()
        md = v1344.to_github_actions_summary(r)
        assert "PASSED" in md or "FAILED" in md

    def test_summary_has_ledger_hash(self):
        r = v1344.lint_v1335_ledger_ci()
        md = v1344.to_github_actions_summary(r)
        assert r.ledger_hash in md

    def test_summary_has_tier_table(self):
        r = v1344.lint_v1335_ledger_ci()
        md = v1344.to_github_actions_summary(r)
        assert "HIGH" in md
        assert "MEDIUM" in md
        assert "LOW" in md
        assert "UNCLASSIFIED" in md

    def test_summary_has_violations_section(self):
        cfg = v1344.CIGateConfig(tier_min="high", fail_on_coverage_loss=True, baseline_coverage=1.5)
        r = v1344.lint_v1335_ledger_ci(cfg)
        md = v1344.to_github_actions_summary(r)
        assert "Violations" in md


# --- pre-commit output -----------------------------------------------------
class TestToPreCommitOutput:
    def test_pass_message(self):
        cfg = v1344.CIGateConfig(tier_min="all", fail_on_coverage_loss=False)
        r = v1344.lint_v1335_ledger_ci(cfg)
        if r.passed:
            out = v1344.to_pre_commit_output(r)
            assert "PASSED" in out

    def test_fail_message(self):
        cfg = v1344.CIGateConfig(tier_min="high", fail_on_coverage_loss=True, baseline_coverage=1.5)
        r = v1344.lint_v1335_ledger_ci(cfg)
        out = v1344.to_pre_commit_output(r)
        assert "FAILED" in out


# --- Deployment artifacts ---------------------------------------------------
class TestMakeGithubActionsWorkflow:
    def test_workflow_has_name(self):
        wf = v1344.make_github_actions_workflow()
        assert "name:" in wf
        assert "VCP CI Gate" in wf

    def test_workflow_has_triggers(self):
        wf = v1344.make_github_actions_workflow()
        assert "push:" in wf
        assert "pull_request:" in wf

    def test_workflow_uses_v1344(self):
        wf = v1344.make_github_actions_workflow()
        assert "v1344_vcp_ci_gate.py" in wf

    def test_workflow_uploads_sarif(self):
        wf = v1344.make_github_actions_workflow()
        assert "upload-sarif" in wf

    def test_workflow_emits_summary(self):
        wf = v1344.make_github_actions_workflow()
        assert "GITHUB_STEP_SUMMARY" in wf


class TestMakePreCommitConfig:
    def test_precommit_has_id(self):
        pc = v1344.make_pre_commit_config()
        assert "id: vcp-ci-gate" in pc

    def test_precommit_uses_v1344(self):
        pc = v1344.make_pre_commit_config()
        assert "v1344_vcp_ci_gate.py" in pc

    def test_precommit_has_entry(self):
        pc = v1344.make_pre_commit_config()
        assert "entry:" in pc

    def test_precommit_uses_python_language(self):
        pc = v1344.make_pre_commit_config()
        assert "language: python" in pc


class TestMakeDockerfile:
    def test_dockerfile_has_from(self):
        df = v1344.make_dockerfile()
        assert "FROM python" in df

    def test_dockerfile_has_entrypoint(self):
        df = v1344.make_dockerfile()
        assert "ENTRYPOINT" in df

    def test_dockerfile_uses_v1344(self):
        df = v1344.make_dockerfile()
        assert "v1344_vcp_ci_gate.py" in df

    def test_dockerfile_default_args(self):
        df = v1344.make_dockerfile()
        assert "tier-min" in df
        assert "high" in df


# --- ASI pole-star ---------------------------------------------------------
class TestPoleStar:
    def test_pole_star_locked(self):
        assert v1344.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1344.ASI_POLE_STAR["V0_2_baseline"] == 0.4467
        assert v1344.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105
        assert v1344.ASI_POLE_STAR["V1049_value_alignment_done"] is True
        assert v1344.ASI_POLE_STAR["asi_achieved_false"] is True
        assert v1344.ASI_POLE_STAR["V1344_modifies_pole_star"] is False


# --- CLI --------------------------------------------------------------------
class TestCli:
    def test_self_test_via_cli(self, capsys):
        code = v1344.gate_main(["--self-test"])
        captured = capsys.readouterr()
        assert "V1344 self-tests:" in captured.out
        assert code == 0  # 28/28 pass

    def test_emit_workflow_via_cli(self, capsys):
        code = v1344.gate_main(["--emit-workflow"])
        captured = capsys.readouterr()
        assert "VCP CI Gate" in captured.out
        assert code == 0

    def test_emit_precommit_via_cli(self, capsys):
        code = v1344.gate_main(["--emit-precommit"])
        captured = capsys.readouterr()
        assert "vcp-ci-gate" in captured.out
        assert code == 0

    def test_emit_dockerfile_via_cli(self, capsys):
        code = v1344.gate_main(["--emit-dockerfile"])
        captured = capsys.readouterr()
        assert "FROM python" in captured.out
        assert code == 0

    def test_markdown_format_cli(self, capsys):
        code = v1344.gate_main(["--tier-min", "all", "--no-fail-on-coverage-loss", "--format", "markdown"])
        captured = capsys.readouterr()
        assert "apeireth-vcp-linter" in captured.out or "PASSED" in captured.out or "FAILED" in captured.out

    def test_sarif_format_cli(self, capsys):
        code = v1344.gate_main(["--tier-min", "all", "--no-fail-on-coverage-loss", "--format", "sarif"])
        captured = capsys.readouterr()
        # JSON output
        data = json.loads(captured.out)
        assert "runs" in data


# --- Self-test summary -----------------------------------------------------
class TestSelfTestSummary:
    def test_summary_returns_tuple(self):
        result = v1344._self_test_summary()
        assert isinstance(result, tuple)
        assert len(result) == 3
        passed, total, failures = result
        assert isinstance(passed, int)
        assert isinstance(total, int)
        assert isinstance(failures, list)

    def test_summary_all_pass(self):
        passed, total, failures = v1344._self_test_summary()
        assert passed == total
        assert failures == []
        assert total >= 28


# --- Integration tests -----------------------------------------------------
class TestIntegration:
    def test_default_run_is_deterministic(self):
        cfg = v1344.CIGateConfig()
        r1 = v1344.lint_v1335_ledger_ci(cfg)
        r2 = v1344.lint_v1335_ledger_ci(cfg)
        assert r1.ledger_hash == r2.ledger_hash
        assert r1.coverage == r2.coverage
        assert r1.tier_breakdown == r2.tier_breakdown

    def test_sarif_round_trip(self):
        r = v1344.lint_v1335_ledger_ci()
        sarif = v1344.to_sarif(r)
        json_str = json.dumps(sarif)
        reparsed = json.loads(json_str)
        assert reparsed["version"] == "2.1.0"

    def test_result_round_trip(self):
        r = v1344.lint_v1335_ledger_ci()
        d = r.to_dict()
        json_str = json.dumps(d, default=str)
        reparsed = json.loads(json_str)
        assert reparsed["passed"] == r.passed

    def test_high_tier_min_filters_low_tier(self):
        cfg = v1344.CIGateConfig(tier_min="high", fail_on_coverage_loss=False)
        r = v1344.lint_v1335_ledger_ci(cfg)
        medium_violations = [v for v in r.violations if v.get("tier") == "MEDIUM"]
        assert len(medium_violations) >= 3  # at least 3 MEDIUM in ledger

    def test_unclassified_substrates_list_populated(self):
        r = v1344.lint_v1335_ledger_ci()
        assert isinstance(r.unclassified_substrates, list)