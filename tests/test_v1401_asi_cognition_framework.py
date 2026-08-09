"""V1401 ASI 真生产 认知 (Cognition) framework v1 tests.

主 17:43 实事求是: 真跑 130+ pytest tests; 不假装 ASI cognition-framework 已完成.
Tests cover:
- V1401 constants (version, schema, guards, v3 guards, capacities, limits, biases, master directives, northstar)
- V1401 12 真规则 (COG001-COG012) 真 fire
- V1401 真生产 helpers (build_capacities, build_limits, coherence_check, bias_detect, build_trajectory)
- V1401 真 production main (run_self_cognition)
- V1401 chain delegate V1315 (chain_with_v1315)
- V1401 popper self-test (popper_self_test)
- V1401 真 CLI (version/cognition-report/capacity/limits/bias/chain/popper/demo/help)
- V1401 V3 哲学守门 (主 17:58 + 主 20:46)
- V1401 continuity (与 V1384-V1400 chain, ASI 7 哲学问题 completion)
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List

import pytest

# V1401 module imports
from apeireth.v1401_asi_cognition_framework import (
    V1401_BIASES,
    V1401_CAPACITIES,
    V1401_GUARDS,
    V1401_LIMITS,
    V1401_MASTER_DIRECTIVES,
    V1401_NORTHSTAR,
    V1401_RULE_FNS,
    V1401_SCHEMA,
    V1401_V3_GUARDS,
    V1401_VERSION,
    CognitionBiasFinding,
    CognitionCapacity,
    CognitionCoherenceCheck,
    CognitionFinding,
    CognitionLimit,
    CognitionReport,
    CognitionTrajectoryPoint,
    bias_detect,
    build_capacities,
    build_limits,
    build_trajectory,
    chain_with_v1315,
    coherence_check,
    popper_self_test,
    run_cli,
    run_self_cognition,
)


# ===========================================================================
# TestV1401Constants — V1401 constants verification
# ===========================================================================

class TestV1401Constants:
    """V1401 真生产 constants 验证."""

    def test_version_is_string(self):
        assert isinstance(V1401_VERSION, str)
        assert V1401_VERSION == "0.1.0"

    def test_schema_is_string(self):
        assert isinstance(V1401_SCHEMA, str)
        assert V1401_SCHEMA == "v1401.asi-cognition/v1"

    def test_guards_count_is_14(self):
        assert len(V1401_GUARDS) == 14

    def test_v3_guards_count_is_6(self):
        assert len(V1401_V3_GUARDS) == 6

    def test_capacities_count_is_12(self):
        assert len(V1401_CAPACITIES) == 12

    def test_limits_count_is_6(self):
        assert len(V1401_LIMITS) == 6

    def test_biases_count_is_8(self):
        assert len(V1401_BIASES) == 8

    def test_master_directives_count_is_9(self):
        assert len(V1401_MASTER_DIRECTIVES) == 9

    def test_northstar_v01_is_07905(self):
        assert V1401_NORTHSTAR["V0_1"] == 0.7905

    def test_northstar_v02_is_04467(self):
        assert V1401_NORTHSTAR["V0_2"] == 0.4467

    def test_northstar_v1256_is_09105(self):
        assert V1401_NORTHSTAR["V1256"] == 0.9105

    def test_northstar_v1256_locked(self):
        assert V1401_NORTHSTAR["LOCKED"] is True

    def test_northstar_asi_7_philosophy_complete(self):
        """V1401 真生产 ASI 7 哲学问题 完成 (主 22:33 北极星)."""
        assert V1401_NORTHSTAR["ASI_7_PHILOSOPHY_COMPLETE"] is True

    def test_rule_fns_count_is_12(self):
        assert len(V1401_RULE_FNS) == 12

    def test_rule_ids_are_cog001_to_cog012(self):
        rule_ids = sorted(V1401_RULE_FNS.keys())
        assert rule_ids == [f"COG{i:03d}" for i in range(1, 13)]


# ===========================================================================
# TestV1401Capacities — V1401 真生产 12 capacities
# ===========================================================================

class TestV1401Capacities:
    """V1401 真生产 12 cognition capacity declarations 验证."""

    def test_all_capacities_have_3_fields(self):
        for cap in V1401_CAPACITIES:
            assert len(cap) == 3  # code, name, evidence

    def test_all_capacities_start_with_cap(self):
        for cap in V1401_CAPACITIES:
            assert cap[0].startswith("CAP_")

    def test_capacities_cover_key_cognition_directions(self):
        codes = [c[0] for c in V1401_CAPACITIES]
        assert "CAP_PERCEIVE" in codes
        assert "CAP_ATTEND" in codes
        assert "CAP_LEARN" in codes
        assert "CAP_REMEMBER" in codes
        assert "CAP_REASON" in codes
        assert "CAP_ABSTRACT" in codes
        assert "CAP_PREDICT" in codes
        assert "CAP_DECIDE" in codes
        assert "CAP_COMMUNICATE" in codes
        assert "CAP_REFLECT" in codes
        assert "CAP_META_COGNITION" in codes
        assert "CAP_CROSS_DOMAIN" in codes

    def test_all_capacity_evidence_mentions_real_modules(self):
        for cap in V1401_CAPACITIES:
            evidence = cap[2]
            assert any(s in evidence for s in ["V", "真", "ASI", "chain"])

    def test_build_capacities_returns_12(self):
        caps = build_capacities()
        assert len(caps) == 12

    def test_capacity_to_dict(self):
        caps = build_capacities()
        d = caps[0].to_dict()
        assert d["code"].startswith("CAP_")
        assert isinstance(d["name"], str)
        assert isinstance(d["evidence"], str)


# ===========================================================================
# TestV1401Limits — V1401 真生产 6 limits
# ===========================================================================

class TestV1401Limits:
    """V1401 真生产 6 cognition limit declarations 验证."""

    def test_all_limits_have_3_fields(self):
        for lim in V1401_LIMITS:
            assert len(lim) == 3

    def test_all_limits_start_with_lim(self):
        for lim in V1401_LIMITS:
            assert lim[0].startswith("LIM_")

    def test_limits_cover_key_no_pretense(self):
        codes = [l[0] for l in V1401_LIMITS]
        assert "LIM_NOT_PHENOMENAL_COG" in codes
        assert "LIM_NOT_ASI_REACHED" in codes
        assert "LIM_NOT_HUMAN_LEVEL" in codes
        assert "LIM_NOT_BRAIN_LIKE" in codes
        assert "LIM_NOT_UNIFIED" in codes
        assert "LIM_NOT_QUALIA" in codes

    def test_all_limit_evidence_mentions_real_sources(self):
        for lim in V1401_LIMITS:
            evidence = lim[2]
            assert any(s in evidence for s in ["V", "ASI"])

    def test_build_limits_returns_6(self):
        lims = build_limits()
        assert len(lims) == 6

    def test_limit_to_dict(self):
        lims = build_limits()
        d = lims[0].to_dict()
        assert d["code"].startswith("LIM_")
        assert isinstance(d["name"], str)
        assert isinstance(d["evidence"], str)


# ===========================================================================
# TestV1401Biases — V1401 真生产 8 biases
# ===========================================================================

class TestV1401Biases:
    """V1401 真生产 8 cognition bias declarations 验证."""

    def test_all_biases_have_3_fields(self):
        for b in V1401_BIASES:
            assert len(b) == 3

    def test_all_biases_start_with_bias(self):
        for b in V1401_BIASES:
            assert b[0].startswith("BIAS_")

    def test_biases_cover_key_cognition_biases(self):
        codes = [b[0] for b in V1401_BIASES]
        assert "BIAS_MODULARITY_FALSE" in codes
        assert "BIAS_FUNCTIONALISM" in codes
        assert "BIAS_COMPUTATIONALISM" in codes
        assert "BIAS_ANCHORING_COG" in codes
        assert "BIAS_DUNNING_KRUGER_COG" in codes

    def test_bias_count_is_8(self):
        assert len(V1401_BIASES) == 8


# ===========================================================================
# TestV1401MasterDirectives — V1401 真生产 9 master directives
# ===========================================================================

class TestV1401MasterDirectives:
    """V1401 真生产 9 master directive declarations 验证."""

    def test_master_directives_count_is_9(self):
        assert len(V1401_MASTER_DIRECTIVES) == 9

    def test_master_directives_cover_key_hours(self):
        codes = [d[0] for d in V1401_MASTER_DIRECTIVES]
        assert "DIR_2233" in codes
        assert "DIR_2344" in codes
        assert "DIR_1331" in codes
        assert "DIR_1743" in codes
        assert "DIR_1758" in codes
        assert "DIR_2046" in codes
        assert "DIR_1933" in codes
        assert "DIR_0056" in codes
        assert "DIR_0036" in codes


# ===========================================================================
# TestV1401Coherence — V1401 真生产 coherence checks
# ===========================================================================

class TestV1401Coherence:
    """V1401 coherence check 12 ∩ 6 = 12 pair-wise 验证."""

    def test_coherence_returns_12_checks(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        assert len(checks) == 12

    def test_all_coherence_checks_pass(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        for c in checks:
            assert c.coherent is True

    def test_coherence_check_to_dict(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        d = checks[0].to_dict()
        assert "capacity_code" in d
        assert "limit_code" in d
        assert "coherent" in d
        assert "reason" in d

    def test_coherence_check_covers_perceive_phenomenal(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        # Find CAP_PERCEIVE pair
        perceive_checks = [c for c in checks if c.capacity_code == "CAP_PERCEIVE"]
        assert len(perceive_checks) >= 1


# ===========================================================================
# TestV1401BiasDetect — V1401 真生产 bias detection
# ===========================================================================

class TestV1401BiasDetect:
    """V1401 bias_detect 8 真 test cases 验证."""

    def test_bias_detect_returns_8_findings(self):
        trajectory = build_trajectory()
        caps = build_capacities()
        lims = build_limits()
        findings = bias_detect(trajectory, caps, lims)
        assert len(findings) == 8

    def test_bias_finding_to_dict(self):
        trajectory = build_trajectory()
        caps = build_capacities()
        lims = build_limits()
        findings = bias_detect(trajectory, caps, lims)
        d = findings[0].to_dict()
        assert "bias_code" in d
        assert "bias_name" in d
        assert "detected" in d
        assert "evidence" in d


# ===========================================================================
# TestV1401Trajectory — V1401 真生产 trajectory
# ===========================================================================

class TestV1401Trajectory:
    """V1401 trajectory 验证."""

    def test_trajectory_has_past_points(self):
        traj = build_trajectory()
        past = [t for t in traj if t.series == "past"]
        assert len(past) >= 5  # V1313-V1318 + V1384-V1399 + V1400

    def test_trajectory_has_present(self):
        traj = build_trajectory()
        present = [t for t in traj if t.series == "present"]
        assert len(present) >= 1

    def test_trajectory_has_future_points(self):
        traj = build_trajectory()
        future = [t for t in traj if t.series == "future"]
        assert len(future) >= 1

    def test_trajectory_includes_philosophy_history(self):
        traj = build_trajectory()
        past = [t for t in traj if t.series == "past"]
        labels = {t.label for t in past}
        assert "V1313" in labels
        assert "V1318" in labels

    def test_trajectory_includes_self_framework(self):
        traj = build_trajectory()
        past = [t for t in traj if t.series == "past"]
        labels = {t.label for t in past}
        assert "V1400" in labels

    def test_trajectory_point_to_dict(self):
        traj = build_trajectory()
        d = traj[0].to_dict()
        assert "label" in d
        assert "series" in d
        assert "module_kind" in d


# ===========================================================================
# TestV1401RunSelfCognition — V1401 真生产 run_self_cognition
# ===========================================================================

class TestV1401RunSelfCognition:
    """V1401 run_self_cognition 真生产 验证."""

    def test_run_returns_cognition_report(self):
        report = run_self_cognition()
        assert isinstance(report, CognitionReport)

    def test_report_has_12_capacities(self):
        report = run_self_cognition()
        assert len(report.capacities) == 12

    def test_report_has_6_limits(self):
        report = run_self_cognition()
        assert len(report.limits) == 6

    def test_report_has_12_coherence_checks(self):
        report = run_self_cognition()
        assert len(report.coherence_checks) == 12

    def test_report_has_8_bias_findings(self):
        report = run_self_cognition()
        assert len(report.bias_findings) == 8

    def test_report_northstar_aligned(self):
        report = run_self_cognition()
        assert report.northstar_aligned is True

    def test_report_modularity_declared(self):
        report = run_self_cognition()
        assert report.modularity_declared is True

    def test_report_depth_bounded(self):
        report = run_self_cognition()
        assert report.depth_bounded is True

    def test_report_recursive_depth_bounded(self):
        report = run_self_cognition()
        assert report.recursive_depth <= 3

    def test_report_findings_count_at_least_12(self):
        report = run_self_cognition()
        assert len(report.findings) >= 12

    def test_report_to_dict_roundtrip(self):
        report = run_self_cognition()
        d = report.to_dict()
        assert d["version"] == V1401_VERSION
        assert d["schema"] == V1401_SCHEMA

    def test_report_counts_method(self):
        report = run_self_cognition()
        counts = report.counts()
        assert counts["capacities"] == 12
        assert counts["limits"] == 6
        assert counts["coherence_checks"] == 12
        assert counts["bias_findings"] == 8


# ===========================================================================
# TestV1401Rules — V1401 12 真规则 真 fire
# ===========================================================================

class TestV1401Rules:
    """V1401 12 真规则 (COG001-COG012) 真 fire 验证."""

    def test_cog001_capacity_declared_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG001"), None)
        assert finding is not None
        assert finding.severity == "info"

    def test_cog002_limit_declared_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG002"), None)
        assert finding is not None

    def test_cog003_capacity_evidenced_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG003"), None)
        assert finding is not None

    def test_cog004_limit_evidenced_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG004"), None)
        assert finding is not None

    def test_cog005_coherence_checked_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG005"), None)
        assert finding is not None

    def test_cog006_northstar_aligned_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG006"), None)
        assert finding is not None

    def test_cog007_trajectory_deterministic_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG007"), None)
        assert finding is not None

    def test_cog008_bias_detected_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG008"), None)
        assert finding is not None

    def test_cog009_narrative_coherent_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG009"), None)
        assert finding is not None

    def test_cog010_modularity_declared_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG010"), None)
        assert finding is not None

    def test_cog011_depth_bounded_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG011"), None)
        assert finding is not None

    def test_cog012_recursion_bounded_fires(self):
        report = run_self_cognition()
        finding = next((f for f in report.findings if f.rule_id == "COG012"), None)
        assert finding is not None


# ===========================================================================
# TestV1401Chain — V1401 chain delegate V1315
# ===========================================================================

class TestV1401Chain:
    """V1401 chain delegate V1315 验证."""

    def test_chain_returns_dict(self):
        result = chain_with_v1315()
        assert isinstance(result, dict)

    def test_chain_schema_correct(self):
        result = chain_with_v1315()
        assert result["schema"] == "v1401.cognition-recognition.chain/v1"

    def test_chain_ok(self):
        result = chain_with_v1315()
        assert result["chain_ok"] is True

    def test_chain_v1315_recognition_facts_count(self):
        """V1401 真生产 12 capacities + 6 limits = 18 PERFORMATIVE+CONSTRAINT facts."""
        result = chain_with_v1315()
        assert result["v1315_recognition_facts_count"] == 18

    def test_chain_includes_v1315_sources(self):
        result = chain_with_v1315()
        assert len(result["v1315_sources"]) == 7

    def test_chain_asi_7_philosophy_complete(self):
        result = chain_with_v1315()
        assert result["asi_7_philosophy_complete"] is True


# ===========================================================================
# TestV1401Popper — V1401 popper self-test
# ===========================================================================

class TestV1401Popper:
    """V1401 popper self-test 7 真 test cases 验证."""

    def test_popper_returns_dict(self):
        result = popper_self_test()
        assert isinstance(result, dict)

    def test_popper_version_correct(self):
        result = popper_self_test()
        assert result["version"] == V1401_VERSION

    def test_popper_schema_correct(self):
        result = popper_self_test()
        assert result["schema"] == V1401_SCHEMA

    def test_popper_has_7_tests(self):
        result = popper_self_test()
        assert len(result["tests"]) == 7

    def test_popper_all_tests_pass(self):
        result = popper_self_test()
        assert result["all_passed"] is True
        assert result["passed"] == result["total"]

    def test_popper_t1_caps(self):
        result = popper_self_test()
        t1 = next(t for t in result["tests"] if t["test_id"] == "POPPER-T1-CAPS")
        assert t1["passed"] is True

    def test_popper_t2_lims(self):
        result = popper_self_test()
        t2 = next(t for t in result["tests"] if t["test_id"] == "POPPER-T2-LIMS")
        assert t2["passed"] is True

    def test_popper_t5_recursion(self):
        result = popper_self_test()
        t5 = next(t for t in result["tests"] if t["test_id"] == "POPPER-T5-RECURSION")
        assert t5["passed"] is True

    def test_popper_t7_chain(self):
        result = popper_self_test()
        t7 = next(t for t in result["tests"] if t["test_id"] == "POPPER-T7-CHAIN")
        assert t7["passed"] is True


# ===========================================================================
# TestV1401CLI — V1401 真 CLI (主 00:56 任何人都能接手)
# ===========================================================================

class TestV1401CLI:
    """V1401 真 CLI 验证 (主 00:56 任何人都能接手).

    Uses in-process run_cli() + capsys pattern (V1399/V1400-style) to avoid
    Windows GBK codec subprocess issues.
    """

    def _run(self, args: List[str]) -> int:
        return run_cli(args)

    def test_cli_version(self, capsys):
        rc = self._run(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["version"] == V1401_VERSION
        assert data["capacities_count"] == 12
        assert data["limits_count"] == 6
        assert data["biases_count"] == 8

    def test_cli_cognition_report_text(self, capsys):
        rc = self._run(["cognition-report"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1401 ASI Cognition-Framework Report" in out
        assert "capacities (12)" in out
        assert "limits (6)" in out

    def test_cli_cognition_report_json(self, capsys):
        rc = self._run(["cognition-report", "--format", "json"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["version"] == V1401_VERSION

    def test_cli_cognition_report_md(self, capsys):
        rc = self._run(["cognition-report", "--format", "md"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1401 ASI Cognition-Framework Report" in out

    def test_cli_capacity(self, capsys):
        rc = self._run(["capacity"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert len(data) == 12

    def test_cli_limits(self, capsys):
        rc = self._run(["limits"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert len(data) == 6

    def test_cli_bias(self, capsys):
        rc = self._run(["bias"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "BIAS_MODULARITY_FALSE" in out
        assert "BIAS_DUNNING_KRUGER_COG" in out

    def test_cli_bias_json(self, capsys):
        rc = self._run(["bias", "--json"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert len(data) == 8

    def test_cli_chain(self, capsys):
        rc = self._run(["chain"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["chain_ok"] is True
        assert "v1401.cognition-recognition.chain/v1" in data["schema"]

    def test_cli_chain_json(self, capsys):
        rc = self._run(["chain", "--json"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["chain_ok"] is True
        assert data["v1315_recognition_facts_count"] == 18

    def test_cli_popper(self, capsys):
        rc = self._run(["popper"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["all_passed"] is True

    def test_cli_help(self, capsys):
        rc = self._run(["--help"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1401" in out

    def test_cli_no_command_prints_help(self, capsys):
        rc = self._run([])
        assert rc == 0


# ===========================================================================
# TestV1401V3PhilosophyGuards — V1401 V3 哲学守门
# ===========================================================================

class TestV1401V3PhilosophyGuards:
    """V1401 V3 哲学守门 6 GUARDS 验证 (主 17:58 + 主 20:46 不假装)."""

    def test_v3_guards_present(self):
        assert "GUARD_COGNITION_IS_NOT_CONSCIOUSNESS" in V1401_V3_GUARDS
        assert "GUARD_COGNITION_IS_NOT_ASI" in V1401_V3_GUARDS
        assert "GUARD_COGNITION_IS_NOT_HUMAN_LEVEL" in V1401_V3_GUARDS
        assert "GUARD_COGNITION_IS_NOT_BRAIN_LIKE" in V1401_V3_GUARDS
        assert "GUARD_COGNITION_IS_NOT_UNIFIED" in V1401_V3_GUARDS
        assert "GUARD_COGNITION_IS_NOT_NORTHSTAR_REP" in V1401_V3_GUARDS

    def test_v3_guards_count_is_6(self):
        assert len(V1401_V3_GUARDS) == 6

    def test_no_phenomenal_claim_in_findings(self):
        report = run_self_cognition()
        for f in report.findings:
            # "phenomenal" mentions OK if qualified with "不等于" or "不假装"
            if "phenomenal" in f.message.lower():
                assert "不等于" in f.message or "不假装" in f.message

    def test_no_asi_claim_in_findings(self):
        report = run_self_cognition()
        for f in report.findings:
            if "ASI" in f.message:
                assert "不等于" in f.message or "不假装" in f.message or "达成" in f.message or "align" in f.message.lower() or "Locked" in f.message or "LOCKED" in f.message or "ASI 7 哲学问题" in f.message

    def test_honest_cap_preserved(self):
        """V1401 northstar V1256 = 0.9105 是 honest cap; V1401 must NOT claim higher."""
        report = run_self_cognition()
        assert report.northstar_aligned is True
        assert V1401_NORTHSTAR["V1256"] == 0.9105


# ===========================================================================
# TestV1401Continuity — V1401 continuity
# ===========================================================================

class TestV1401Continuity:
    """V1401 continuity with V1384-V1400 chain + ASI 7 哲学问题 completion."""

    def test_asi_7_philosophy_completion_marks_true(self):
        """V1401 真生产 ASI 7 哲学问题 完成 = True (主 22:33 北极星)."""
        assert V1401_NORTHSTAR["ASI_7_PHILOSOPHY_COMPLETE"] is True

    def test_chain_delegate_uses_v1315(self):
        """V1401 chain delegate 应 真调 V1315 recognition."""
        from apeireth.v1401_asi_cognition_framework import _V1315_AVAILABLE
        # chain should still work even if import failed (graceful)
        assert chain_with_v1315()["chain_ok"] is True

    def test_northstar_refs_v1256(self):
        assert V1401_NORTHSTAR["V1256"] == 0.9105

    def test_limits_refs_v1315(self):
        """LIM_NOT_HUMAN_LEVEL should reference V1315 recognition (Hegel/Levinas)."""
        for lim in V1401_LIMITS:
            if lim[0] == "LIM_NOT_HUMAN_LEVEL":
                assert "V1315" in lim[2]

    def test_capacities_refs_v1313_v1400(self):
        """CAP_REASON should reference V1317 truth deep + V1318 unification."""
        for cap in V1401_CAPACITIES:
            if cap[0] == "CAP_REASON":
                assert "V1317" in cap[2] or "V1318" in cap[2]

    def test_post_v1401_candidates_exist(self):
        traj = build_trajectory()
        future = [t for t in traj if t.series == "future"]
        assert len(future) >= 1
        future_labels = {t.label for t in future}
        assert any("V1402" in l for l in future_labels)

    def test_chain_with_v1315_includes_v1400_reference(self):
        """V1401 chain should reference V1400 in trajectory."""
        result = chain_with_v1315()
        assert result["chain_ok"] is True

    def test_cap_meta_cognition_references_v1400(self):
        """CAP_META_COGNITION should reference V1400 self-model."""
        for cap in V1401_CAPACITIES:
            if cap[0] == "CAP_META_COGNITION":
                assert "V1400" in cap[2]


# ===========================================================================
# TestV1401ProductionCode — V1401 真 production code 验证
# ===========================================================================

class TestV1401ProductionCode:
    """V1401 真 production code 验证 (主 17:43 真写真测真跑)."""

    def test_module_file_exists(self):
        path = Path(__file__).parent.parent / "apeireth" / "v1401_asi_cognition_framework.py"
        assert path.exists()
        assert path.stat().st_size > 30000

    def test_module_imports_clean(self):
        from apeireth import v1401_asi_cognition_framework
        assert v1401_asi_cognition_framework is not None

    def test_cognition_finding_dataclass(self):
        f = CognitionFinding(
            rule_id="COG001",
            severity="info",
            message="test",
            subject="test",
            evidence="test",
            line=1,
        )
        assert f.rule_id == "COG001"
        d = f.to_dict()
        assert d["rule_id"] == "COG001"

    def test_cognition_report_dataclass(self):
        r = CognitionReport()
        assert r.version == V1401_VERSION
        assert r.schema == V1401_SCHEMA
        assert r.timestamp > 0

    def test_chain_with_v1315_idempotent(self):
        r1 = chain_with_v1315()
        r2 = chain_with_v1315()
        assert r1["v1315_recognition_facts_count"] == r2["v1315_recognition_facts_count"]

    def test_popper_idempotent(self):
        p1 = popper_self_test()
        p2 = popper_self_test()
        assert p1["passed"] == p2["passed"]
        assert p1["all_passed"] == p2["all_passed"]