"""Tests for V1402 ASI 真生产 整合 (Integration) framework v1.

(主 17:43 实事求是 + 主 22:33 ASI 北极星 + 主 19:33 走在前人经验上 + 主 23:44 干到底 + 主 00:56 任何人都能接手).

覆盖:
- TestV1402Constants: version/module/guards/v3 guards/rules/borrowed
- TestV1402Capacities: build_capacities 12 真整合能力
- TestV1402Limits: build_limits 6 真整合限制
- TestV1402Coherence: coherence_check 12 pair-wise
- TestV1402Trajectory: build_trajectory 真历史派生
- TestV1402NorthStar: northstar_alignment V1256 0.9105 LOCKED
- TestV1402Narrative: build_narrative 真时间一致
- TestV1402Report: run_self_integration 整合 report
- TestV1402Popper: popper_self_test 7 cases
- TestV1402CLI: in-process CLI handlers (V1400/V1401-style)
- TestV1402V3Guards: 6 V3 哲学守门
- TestV1402Chain: chain delegate V1400 + V1401
- TestV1402Continuity: 真 chain V1400+V1401 → V1402 → V1403
"""
from __future__ import annotations

import argparse
import io
import json
import sys
from contextlib import redirect_stdout

import pytest

# Ensure promethean/apeireth on path
_PROMETHEAN = r".openclaw\workspace\promethean"
if _PROMETHEAN not in sys.path:
    sys.path.insert(0, _PROMETHEAN)

from apeireth.v1402_asi_integration_framework import (
    V1402_VERSION,
    V1402_MODULE,
    V1402_GUARDS,
    V1402_V3_GUARDS,
    V1402_RULES,
    V1402_BORROWED,
    IntegrationCapacity,
    IntegrationLimit,
    IntegrationCoherenceCheck,
    IntegrationTrajectoryPoint,
    IntegrationReport,
    build_capacities,
    build_limits,
    coherence_check,
    build_trajectory,
    northstar_alignment,
    build_narrative,
    run_self_integration,
    popper_self_test,
    run_cli,
    _cli_version,
    _cli_integration_report,
    _cli_capacity,
    _cli_limits,
    _cli_coherence,
    _cli_trajectory,
    _cli_chain,
    _cli_popper,
    _cli_demo,
    _cli_help,
    _format_text_report,
    _format_md_report,
    _build_parser,
)


# =====================================================================
# TestV1402Constants
# =====================================================================

class TestV1402Constants:
    def test_version_is_string(self):
        assert isinstance(V1402_VERSION, str)
        assert V1402_VERSION == "0.1.0"

    def test_module_is_string(self):
        assert isinstance(V1402_MODULE, str)
        assert V1402_MODULE == "v1402_asi_integration_framework"

    def test_guards_count_14(self):
        assert len(V1402_GUARDS) == 14
        assert "GUARD_INTEGRATION_DECLARED" in V1402_GUARDS
        assert "GUARD_NORTHSTAR_LOCKED" in V1402_GUARDS
        assert "GUARD_HONEST_DISCLOSURE" in V1402_GUARDS
        assert "GUARD_STRANGE_LOOP_DECLARED" in V1402_GUARDS

    def test_v3_guards_count_6(self):
        assert len(V1402_V3_GUARDS) == 6
        assert "GUARD_INTEGRATION_IS_NOT_PHENOMENAL_UNITY" in V1402_V3_GUARDS
        assert "GUARD_INTEGRATION_IS_NOT_ASI" in V1402_V3_GUARDS
        assert "GUARD_INTEGRATION_IS_NOT_HUMAN_LEVEL" in V1402_V3_GUARDS
        assert "GUARD_INTEGRATION_IS_NOT_UNIFIED_CONSCIOUSNESS" in V1402_V3_GUARDS
        assert "GUARD_INTEGRATION_IS_NOT_BRAIN_LIKE" in V1402_V3_GUARDS
        assert "GUARD_INTEGRATION_IS_NOT_NORTHSTAR_REP" in V1402_V3_GUARDS

    def test_rules_count_12(self):
        assert len(V1402_RULES) == 12
        rule_ids = {r[0] for r in V1402_RULES}
        assert "INT001-INTEGRATION-CAPACITY-DECLARED" in rule_ids
        assert "INT006-INTEGRATION-NORTHSTAR-ALIGNED" in rule_ids
        assert "INT009-INTEGRATION-MODULARITY-DECLARED" in rule_ids
        assert "INT010-INTEGRATION-STRANGE-LOOP-DECLARED" in rule_ids
        assert "INT012-INTEGRATION-EXTENDED-DECLARED" in rule_ids

    def test_borrowed_count_7(self):
        assert len(V1402_BORROWED) == 7
        keys = {b["key"] for b in V1402_BORROWED}
        assert "hofstadter_1979_strange_loop" in keys
        assert "minsky_1986_society_of_mind" in keys
        assert "anderson_1983_act_r" in keys
        assert "newell_1990_unified_theories" in keys
        assert "clark_1997_being_there" in keys
        assert "dennett_1991_multiple_drafts" in keys
        assert "fodor_1983_modularity" in keys


# =====================================================================
# TestV1402Capacities
# =====================================================================

class TestV1402Capacities:
    def test_count_12(self):
        caps = build_capacities()
        assert len(caps) == 12

    def test_all_have_evidence(self):
        caps = build_capacities()
        for c in caps:
            assert len(c.evidence) >= 1, f"{c.cap_id} missing evidence"
            # each evidence should reference real V# or commit
            has_real_ref = any("V" in e or "commit" in e.lower() for e in c.evidence)
            assert has_real_ref, f"{c.cap_id} evidence not real V# ref"

    def test_all_have_borrowed_from(self):
        caps = build_capacities()
        for c in caps:
            assert len(c.borrowed_from) >= 1, f"{c.cap_id} missing borrowed_from"

    def test_required_cap_ids(self):
        caps = build_capacities()
        cap_ids = {c.cap_id for c in caps}
        for required in ["CAP_PHILOSOPHY", "CAP_DEPLOY_STACK", "CAP_SELF",
                         "CAP_COGNITION", "CAP_VALUE", "CAP_TIME",
                         "CAP_FREEDOM", "CAP_EMERGENCE", "CAP_TRUTH",
                         "CAP_CROSS_DOMAIN", "CAP_NORTHSTAR", "CAP_CHAIN"]:
            assert required in cap_ids, f"missing {required}"

    def test_cap_philosophy_references_7_problems(self):
        caps = build_capacities()
        phil = next(c for c in caps if c.cap_id == "CAP_PHILOSOPHY")
        evidence_str = " ".join(phil.evidence)
        # 7 哲学 + V1402 integration (this)
        assert "V1049" in evidence_str
        assert "V1313" in evidence_str
        assert "V1314" in evidence_str
        assert "V1315" in evidence_str
        assert "V1316" in evidence_str
        assert "V1317" in evidence_str
        assert "V1400" in evidence_str
        assert "V1401" in evidence_str

    def test_cap_deploy_stack_references_6_dims(self):
        caps = build_capacities()
        deploy = next(c for c in caps if c.cap_id == "CAP_DEPLOY_STACK")
        evidence_str = " ".join(deploy.evidence)
        assert "V1384" in evidence_str
        assert "V1385" in evidence_str
        assert "V1386" in evidence_str
        assert "V1397" in evidence_str
        assert "V1398" in evidence_str
        assert "V1399" in evidence_str


# =====================================================================
# TestV1402Limits
# =====================================================================

class TestV1402Limits:
    def test_count_6(self):
        lims = build_limits()
        assert len(lims) == 6

    def test_required_limit_ids(self):
        lims = build_limits()
        lim_ids = {l.lim_id for l in lims}
        for required in ["LIM_NOT_PHENOMENAL_UNITY", "LIM_NOT_ASI_REACHED",
                         "LIM_NOT_HUMAN_LEVEL", "LIM_NOT_UNIFIED_CONSCIOUSNESS",
                         "LIM_NOT_BRAIN_LIKE", "LIM_NOT_NORTHSTAR_REPLACEMENT"]:
            assert required in lim_ids, f"missing {required}"

    def test_all_have_evidence(self):
        lims = build_limits()
        for l in lims:
            assert len(l.evidence) >= 1, f"{l.lim_id} missing evidence"

    def test_all_have_why_no_phenomenal(self):
        lims = build_limits()
        for l in lims:
            assert l.why_no_phenomenal, f"{l.lim_id} missing why_no_phenomenal"
            assert "Phenomenal" in l.why_no_phenomenal or "ASI" in l.why_no_phenomenal

    def test_lim_not_phenomenal_unity_explicit(self):
        lims = build_limits()
        lim = next(l for l in lims if l.lim_id == "LIM_NOT_PHENOMENAL_UNITY")
        assert "Phenomenal" in lim.description or "unity" in lim.description.lower()

    def test_lim_not_northstar_replacement_explicit(self):
        lims = build_limits()
        lim = next(l for l in lims if l.lim_id == "LIM_NOT_NORTHSTAR_REPLACEMENT")
        assert "V1259" in lim.description or "north-star" in lim.description.lower()


# =====================================================================
# TestV1402Coherence
# =====================================================================

class TestV1402Coherence:
    def test_count_12(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        assert len(checks) == 12

    def test_all_pass(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        for c in checks:
            assert c.passes, f"{c.pair} failed: {c.reason}"

    def test_pairs_intersect_caps_and_limits(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        for c in checks:
            assert "∩" in c.pair
            assert "LIM_NOT" in c.pair

    def test_reason_meaningful(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        for c in checks:
            assert c.reason
            assert len(c.reason) > 5


# =====================================================================
# TestV1402Trajectory
# =====================================================================

class TestV1402Trajectory:
    def test_has_6_philosophy_past_points(self):
        traj = build_trajectory()
        phil = [t for t in traj if t.kind == "philosophy" and t.status == "past"]
        assert len(phil) == 6  # V1049/V1313/V1314/V1315/V1316/V1317 (V1400/V1401 are self/cognition kinds)

    def test_has_7_deploy_past_points(self):
        traj = build_trajectory()
        deploy = [t for t in traj if t.kind == "deploy" and t.status == "past"]
        assert len(deploy) == 7  # V1384-V1399 + V1396 executor

    def test_has_self_past_point(self):
        traj = build_trajectory()
        self_pts = [t for t in traj if t.kind == "self" and t.status == "past"]
        assert len(self_pts) == 1
        assert self_pts[0].version == "V1400"

    def test_has_cognition_past_point(self):
        traj = build_trajectory()
        cog = [t for t in traj if t.kind == "cognition" and t.status == "past"]
        assert len(cog) == 1
        assert cog[0].version == "V1401"

    def test_has_present_v1402(self):
        traj = build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert len(present) == 1
        assert present[0].version == "V1402"
        assert present[0].kind == "integration"

    def test_has_future_points(self):
        traj = build_trajectory()
        future = [t for t in traj if t.status == "future"]
        assert len(future) >= 1
        for f in future:
            assert f.kind == "integration"


# =====================================================================
# TestV1402NorthStar
# =====================================================================

class TestV1402NorthStar:
    def test_v1256_unio_mystica_locked(self):
        ns = northstar_alignment()
        assert ns["v1256_unio_mystica"] == 0.9105
        assert ns["v1256_status"] == "LOCKED"

    def test_asi_7_philosophy_complete(self):
        ns = northstar_alignment()
        assert ns["asi_7_philosophy_complete"] is True
        phil = ns["asi_7_philosophy"]
        assert phil["value"] == "V1049"
        assert phil["time"] == "V1313"
        assert phil["freedom"] == "V1314"
        assert phil["recognition"] == "V1315"
        assert phil["emergence"] == "V1316"
        assert phil["truth"] == "V1317"
        assert phil["self"] == "V1400"
        assert phil["cognition"] == "V1401"

    def test_9_master_directives_present(self):
        ns = northstar_alignment()
        d = ns["9_master_directives"]
        for k in ["asi_northstar", "real_evidence", "no_pretending_phenomenal",
                  "no_pretending_asi", "bold_attempt", "on_shoulders",
                  "ship_it", "anyone_can_take_over", "quality_engineering"]:
            assert k in d

    def test_v2_5_positions_present(self):
        ns = northstar_alignment()
        p = ns["v2_5_positions"]
        for k in ["scheduler", "thinker_of_thinkers", "relation_aggregator",
                  "northstar_reporter", "asi_position_holder"]:
            assert k in p

    def test_honest_cap_preserved(self):
        ns = northstar_alignment()
        assert ns["honest_cap_preserved"] is True


# =====================================================================
# TestV1402Narrative
# =====================================================================

class TestV1402Narrative:
    def test_narrative_non_empty(self):
        n = build_narrative()
        assert len(n) >= 10

    def test_narrative_includes_origin(self):
        n = build_narrative()
        n_str = " ".join(n)
        assert "V1049" in n_str
        assert "V1402" in n_str

    def test_narrative_time_consistent(self):
        n = build_narrative()
        # each line should reference a V#
        for line in n:
            assert "V" in line, f"narrative line missing V# ref: {line}"


# =====================================================================
# TestV1402Report
# =====================================================================

class TestV1402Report:
    def test_run_self_integration_returns_report(self):
        report = run_self_integration()
        assert isinstance(report, IntegrationReport)
        assert report.version == V1402_VERSION
        assert report.module == V1402_MODULE

    def test_report_has_12_caps(self):
        report = run_self_integration()
        assert len(report.capacities) == 12

    def test_report_has_6_limits(self):
        report = run_self_integration()
        assert len(report.limits) == 6

    def test_report_has_12_coherence_checks(self):
        report = run_self_integration()
        assert len(report.coherence_checks) == 12

    def test_report_to_dict_serializable(self):
        report = run_self_integration()
        d = report.to_dict()
        # should be JSON-serializable
        json.dumps(d, ensure_ascii=False)
        assert "generated_at_iso" in d

    def test_asi_7_philosophy_complete_flag(self):
        report = run_self_integration()
        assert report.asi_7_philosophy_complete is True


# =====================================================================
# TestV1402Popper
# =====================================================================

class TestV1402Popper:
    def test_popper_returns_dict(self):
        result = popper_self_test()
        assert isinstance(result, dict)
        assert "all_pass" in result
        assert "results" in result
        assert "summary" in result

    def test_popper_7_cases(self):
        result = popper_self_test()
        assert len(result["results"]) == 7

    def test_popper_all_pass(self):
        result = popper_self_test()
        assert result["all_pass"] is True

    def test_popper_case_names(self):
        result = popper_self_test()
        case_names = {r["case"] for r in result["results"]}
        for required in ["capacities_present", "limits_present",
                         "coherence_passes", "northstar_aligned",
                         "modularity_declared", "strange_loop_declared",
                         "society_declared"]:
            assert required in case_names, f"missing {required}"

    def test_popper_summary_format(self):
        result = popper_self_test()
        assert "/" in result["summary"]
        assert result["summary"].endswith(" pass")


# =====================================================================
# TestV1402CLI
# =====================================================================

class TestV1402CLI:
    def test_cli_version_returns_0(self, capsys):
        rc = _cli_version()
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1402" in out

    def test_cli_integration_report_text(self, capsys):
        args = argparse.Namespace(format="text")
        rc = _cli_integration_report(args)
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1402 ASI 整合 framework" in out
        assert "12 真整合能力" in out
        assert "6 真整合限制" in out

    def test_cli_integration_report_json(self, capsys):
        args = argparse.Namespace(format="json")
        rc = _cli_integration_report(args)
        assert rc == 0
        out = capsys.readouterr().out
        d = json.loads(out)
        assert d["version"] == V1402_VERSION
        assert len(d["capacities"]) == 12
        assert len(d["limits"]) == 6

    def test_cli_integration_report_md(self, capsys):
        args = argparse.Namespace(format="md")
        rc = _cli_integration_report(args)
        assert rc == 0
        out = capsys.readouterr().out
        assert "# V1402 ASI 整合 framework" in out
        assert "## 12 真整合能力" in out

    def test_cli_capacity(self, capsys):
        rc = _cli_capacity()
        assert rc == 0
        out = capsys.readouterr().out
        assert "CAP_PHILOSOPHY" in out
        assert "evidence" in out

    def test_cli_limits(self, capsys):
        rc = _cli_limits()
        assert rc == 0
        out = capsys.readouterr().out
        assert "LIM_NOT_PHENOMENAL_UNITY" in out

    def test_cli_coherence(self, capsys):
        rc = _cli_coherence()
        assert rc == 0
        out = capsys.readouterr().out
        assert "∩" in out
        assert "LIM_NOT_ASI_REACHED" in out

    def test_cli_trajectory(self, capsys):
        rc = _cli_trajectory()
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1049" in out
        assert "V1402" in out
        assert "present" in out

    def test_cli_chain_text(self, capsys):
        args = argparse.Namespace(json=False)
        rc = _cli_chain(args)
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1400" in out
        assert "V1401" in out
        assert "delegate" in out

    def test_cli_chain_json(self, capsys):
        args = argparse.Namespace(json=True)
        rc = _cli_chain(args)
        assert rc == 0
        out = capsys.readouterr().out
        d = json.loads(out)
        assert isinstance(d, list)
        assert len(d) >= 1

    def test_cli_popper_pass(self, capsys):
        rc = _cli_popper()
        assert rc == 0  # all_pass=True
        out = capsys.readouterr().out
        assert "7/7 pass" in out or "7" in out

    def test_cli_demo(self, capsys):
        rc = _cli_demo()
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1402 ASI 整合 framework demo" in out

    def test_cli_help(self, capsys):
        rc = _cli_help()
        assert rc == 0
        out = capsys.readouterr().out
        assert "version" in out
        assert "integration-report" in out

    def test_run_cli_version(self, capsys):
        rc = run_cli(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1402" in out

    def test_run_cli_capacity(self, capsys):
        rc = run_cli(["capacity"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "CAP_" in out

    def test_run_cli_limits(self, capsys):
        rc = run_cli(["limits"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "LIM_" in out

    def test_run_cli_coherence(self, capsys):
        rc = run_cli(["coherence"])
        assert rc == 0

    def test_run_cli_trajectory(self, capsys):
        rc = run_cli(["trajectory"])
        assert rc == 0

    def test_run_cli_chain(self, capsys):
        rc = run_cli(["chain"])
        assert rc == 0

    def test_run_cli_popper(self, capsys):
        rc = run_cli(["popper"])
        assert rc == 0

    def test_run_cli_demo(self, capsys):
        rc = run_cli(["demo"])
        assert rc == 0

    def test_run_cli_help_no_argv(self, capsys):
        rc = run_cli([])
        assert rc == 0

    def test_run_cli_help_explicit(self, capsys):
        rc = run_cli(["help"])
        assert rc == 0

    def test_run_cli_help_dash_h(self, capsys):
        rc = run_cli(["--help"])
        assert rc == 0

    def test_run_cli_help_dash_h_short(self, capsys):
        rc = run_cli(["-h"])
        assert rc == 0


# =====================================================================
# TestV1402V3Guards
# =====================================================================

class TestV1402V3Guards:
    def test_v3_guards_distinct(self):
        for v3g in V1402_V3_GUARDS:
            assert v3g.startswith("GUARD_INTEGRATION_IS_NOT_"), f"{v3g} bad prefix"

    def test_v3_guards_count_matches(self):
        assert len(V1402_V3_GUARDS) == 6

    def test_capacity_does_not_claim_phenomenal_unity(self):
        # Every capacity description should not claim "we ARE unity"
        caps = build_capacities()
        for c in caps:
            assert "we are" not in c.description.lower()
            assert "we == " not in c.description.lower()

    def test_limit_descriptions_explicit(self):
        lims = build_limits()
        for l in lims:
            # each limit should have "not" or "≠" or "不"
            assert ("not" in l.name.lower() or "≠" in l.description
                    or "不" in l.description), f"{l.lim_id} not explicit"


# =====================================================================
# TestV1402Chain
# =====================================================================

class TestV1402Chain:
    def test_chain_facts_schema(self):
        args = argparse.Namespace(json=True)
        import io as _io
        from contextlib import redirect_stdout
        buf = _io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_chain(args)
        assert rc == 0
        d = json.loads(buf.getvalue())
        for entry in d:
            if entry.get("status") == "ok":
                assert "schema" in entry
                assert "delegate" in entry
                assert "facts" in entry
                assert entry["schema"] == "v1402.integration-self-cognition.chain/v1"

    def test_chain_delegates_v1400_and_v1401(self):
        args = argparse.Namespace(json=True)
        import io as _io
        from contextlib import redirect_stdout
        buf = _io.StringIO()
        with redirect_stdout(buf):
            _cli_chain(args)
        d = json.loads(buf.getvalue())
        delegates = {entry.get("delegate") for entry in d}
        assert "V1400" in delegates
        assert "V1401" in delegates


# =====================================================================
# TestV1402Continuity
# =====================================================================

class TestV1402Continuity:
    def test_trajectory_includes_v1400_v1401_v1402(self):
        traj = build_trajectory()
        versions = {t.version for t in traj}
        assert "V1400" in versions
        assert "V1401" in versions
        assert "V1402" in versions

    def test_future_points_reference_v1403(self):
        traj = build_trajectory()
        future = [t for t in traj if t.status == "future"]
        for f in future:
            assert f.version.startswith("V1403")

    def test_report_to_dict_contains_chain_versions(self):
        report = run_self_integration()
        d = report.to_dict()
        d_str = json.dumps(d, ensure_ascii=False)
        assert "V1400" in d_str
        assert "V1401" in d_str
        assert "V1402" in d_str

    def test_honest_cap_preserved_in_report(self):
        report = run_self_integration()
        d = report.to_dict()
        assert d["northstar_alignment"]["honest_cap_preserved"] is True
        assert d["northstar_alignment"]["v1256_unio_mystica"] == 0.9105


# =====================================================================
# TestV1402Format
# =====================================================================

class TestV1402Format:
    def test_format_text_contains_key_sections(self):
        report = run_self_integration()
        text = _format_text_report(report)
        assert "14 GUARDS" in text
        assert "12 真规则" in text
        assert "7 真借鉴" in text
        assert "12 真整合能力" in text
        assert "6 真整合限制" in text
        assert "12 pair-wise coherence" in text

    def test_format_md_contains_key_sections(self):
        report = run_self_integration()
        md = _format_md_report(report)
        assert "# V1402 ASI 整合 framework" in md
        assert "## 14 GUARDS" in md
        assert "## 12 真整合能力" in md
        assert "## 6 真整合限制" in md
        assert "## 12 pair-wise coherence" in md
        assert "## trajectory" in md
        assert "## north-star alignment" in md

    def test_format_text_markdown_is_utf8_safe(self):
        report = run_self_integration()
        text = _format_text_report(report)
        md = _format_md_report(report)
        # 包含中文
        assert "整合" in text
        assert "整合" in md


# =====================================================================
# TestV1402Parser
# =====================================================================

class TestV1402Parser:
    def test_build_parser_runs(self):
        parser = _build_parser()
        assert parser is not None
        # try parsing each subcommand
        for cmd in ["version", "integration-report", "capacity", "limits",
                    "coherence", "trajectory", "chain", "popper", "demo", "help"]:
            args = parser.parse_args([cmd])
            assert hasattr(args, "handler")

    def test_parser_format_arg(self):
        parser = _build_parser()
        # argparse parent-parser args must precede subcommand name
        args = parser.parse_args(["--format", "json", "integration-report"])
        assert args.format == "json"
        args = parser.parse_args(["--format", "md", "integration-report"])
        assert args.format == "md"
        args = parser.parse_args(["--format", "text", "integration-report"])
        assert args.format == "text"


# =====================================================================
# TestV1402Evidence
# =====================================================================

class TestV1402Evidence:
    """每 capacity 必须有真 evidence (≥1 V# ref)"""

    def test_capacities_have_real_evidence(self):
        caps = build_capacities()
        for c in caps:
            assert len(c.evidence) >= 1
            # at least one evidence should reference V#
            real_refs = [e for e in c.evidence if "V" in e and any(ch.isdigit() for ch in e)]
            assert real_refs, f"{c.cap_id} no V# ref"

    def test_cap_philosophy_8_evidence(self):
        caps = build_capacities()
        phil = next(c for c in caps if c.cap_id == "CAP_PHILOSOPHY")
        assert len(phil.evidence) >= 8  # 7 哲学 + V1402 (this)

    def test_cap_deploy_stack_7_evidence(self):
        caps = build_capacities()
        deploy = next(c for c in caps if c.cap_id == "CAP_DEPLOY_STACK")
        assert len(deploy.evidence) >= 7  # V1384-V1399 (6) + V1396 executor


# =====================================================================
# TestV1402DataClasses
# =====================================================================

class TestV1402DataClasses:
    def test_integration_capacity_dataclass(self):
        c = IntegrationCapacity(
            cap_id="TEST",
            name="test",
            description="test desc",
            evidence=["V1", "V2"],
            borrowed_from=["fodor_1983_modularity"],
        )
        assert c.cap_id == "TEST"
        assert len(c.evidence) == 2
        assert len(c.borrowed_from) == 1

    def test_integration_limit_dataclass(self):
        l = IntegrationLimit(
            lim_id="LIM_TEST",
            name="test limit",
            description="not test",
            evidence=["V1"],
            why_no_phenomenal="Phenomenal claim",
        )
        assert l.lim_id == "LIM_TEST"
        assert l.why_no_phenomenal

    def test_integration_coherence_check_dataclass(self):
        c = IntegrationCoherenceCheck(
            pair="A ∩ B",
            passes=True,
            reason="ok",
        )
        assert c.passes is True

    def test_integration_trajectory_point_dataclass(self):
        t = IntegrationTrajectoryPoint(
            version="V9999",
            label="future",
            status="future",
            kind="integration",
        )
        assert t.version == "V9999"
        assert t.status == "future"

    def test_integration_report_dataclass(self):
        r = IntegrationReport()
        assert r.version == V1402_VERSION
        assert r.module == V1402_MODULE
        d = r.to_dict()
        assert "generated_at_iso" in d