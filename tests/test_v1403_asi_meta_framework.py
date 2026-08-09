"""Tests for V1403 ASI 真生产 元 (Meta) framework v1.

(主 17:43 实事求是 + 主 22:33 ASI 北极星 + 主 19:33 走在前人经验上 + 主 23:44 干到底 + 主 00:56 任何人都能接手).

覆盖:
- TestV1403Constants: version/module/guards/v3 guards/rules/borrowed
- TestV1403Capacities: build_capacities 12 真元能力
- TestV1403Limits: build_limits 6 真元限制
- TestV1403Coherence: coherence_check 12 pair-wise
- TestV1403Trajectory: build_trajectory 真历史派生
- TestV1403NorthStar: northstar_alignment V1256 0.9105 LOCKED
- TestV1403Narrative: build_narrative 真时间一致
- TestV1403Report: run_self_meta 整合 report
- TestV1403Popper: popper_self_test 7 cases
- TestV1403CLI: in-process CLI handlers
- TestV1403V3Guards: 6 V3 哲学守门
- TestV1403Chain: chain delegate V1400 + V1401 + V1402
- TestV1403Continuity: 真 chain V1400+V1401+V1402 → V1403 → V1404
- TestV1403MetaLevels: 4 meta levels L0/L1/L2/L3
- TestV1403Format: text/md output
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

from apeireth.v1403_asi_meta_framework import (
    V1403_VERSION,
    V1403_MODULE,
    V1403_GUARDS,
    V1403_V3_GUARDS,
    V1403_RULES,
    V1403_BORROWED,
    MetaCapacity,
    MetaLimit,
    MetaCoherenceCheck,
    MetaTrajectoryPoint,
    MetaReport,
    build_capacities,
    build_limits,
    coherence_check,
    build_trajectory,
    northstar_alignment,
    build_narrative,
    run_self_meta,
    popper_self_test,
    run_cli,
    _cli_version,
    _cli_meta_report,
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
# TestV1403Constants
# =====================================================================

class TestV1403Constants:
    def test_version_is_string(self):
        assert isinstance(V1403_VERSION, str)
        assert V1403_VERSION == "0.1.0"

    def test_module_name(self):
        assert V1403_MODULE == "v1403_asi_meta_framework"

    def test_guards_count_14(self):
        assert len(V1403_GUARDS) == 14

    def test_guards_contain_required(self):
        required = (
            "GUARD_META_DECLARED",
            "GUARD_EVIDENCE_REAL",
            "GUARD_COHERENCE_REAL",
            "GUARD_NORTHSTAR_LOCKED",
            "GUARD_TRAJECTORY_REAL",
            "GUARD_NARRATIVE_REAL",
            "GUARD_LEVELS_DECLARED",
            "GUARD_STRANGE_LOOP_DECLARED",
            "GUARD_SECOND_ORDER_DECLARED",
            "GUARD_KNOWLEDGE_LEVELS_DECLARED",
            "GUARD_NO_CAP_CHANGE",
            "GUARD_DETERMINISTIC",
            "GUARD_HONEST_DISCLOSURE",
            "GUARD_CLI_RUNNABLE",
        )
        for g in required:
            assert g in V1403_GUARDS, f"missing guard: {g}"

    def test_v3_guards_count_6(self):
        assert len(V1403_V3_GUARDS) == 6

    def test_v3_guards_contain_required(self):
        required = (
            "GUARD_META_IS_NOT_PHENOMENAL_META",
            "GUARD_META_IS_NOT_ASI",
            "GUARD_META_IS_NOT_HUMAN_LEVEL",
            "GUARD_META_IS_NOT_UNIFIED_META",
            "GUARD_META_IS_NOT_BRAIN_LIKE",
            "GUARD_META_IS_NOT_NORTHSTAR_REP",
        )
        for g in required:
            assert g in V1403_V3_GUARDS, f"missing v3 guard: {g}"

    def test_rules_count_12(self):
        assert len(V1403_RULES) == 12

    def test_rules_severity(self):
        info_count = sum(1 for _r, sev, _d in V1403_RULES if sev == "info")
        warn_count = sum(1 for _r, sev, _d in V1403_RULES if sev == "warning")
        # at least one warning
        assert warn_count >= 2
        assert info_count + warn_count == 12

    def test_borrowed_count_7(self):
        assert len(V1403_BORROWED) == 7

    def test_borrowed_keys(self):
        keys = {b["key"] for b in V1403_BORROWED}
        assert "hofstadter_1979_strange_loop" in keys
        assert "bateson_1972_levels_of_learning" in keys
        assert "von_foerster_1973_second_order" in keys
        assert "minsky_1986_society_of_mind" in keys
        assert "dennett_1991_multiple_drafts" in keys
        assert "tarski_1933_undefinability" in keys
        assert "newell_1982_knowledge_levels" in keys


# =====================================================================
# TestV1403Capacities
# =====================================================================

class TestV1403Capacities:
    def test_count_12(self):
        caps = build_capacities()
        assert len(caps) == 12

    def test_all_have_evidence(self):
        caps = build_capacities()
        for c in caps:
            assert c.evidence, f"{c.cap_id} missing evidence"
            assert len(c.evidence) >= 1, f"{c.cap_id} needs ≥1 evidence"

    def test_all_have_borrowed_from(self):
        caps = build_capacities()
        for c in caps:
            assert c.borrowed_from, f"{c.cap_id} missing borrowed_from"
            for bf in c.borrowed_from:
                assert bf in {b["key"] for b in V1403_BORROWED}, \
                    f"{c.cap_id} borrowed_from unknown: {bf}"

    def test_unique_cap_ids(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert len(ids) == len(set(ids)), f"duplicate cap_ids: {ids}"

    def test_specific_capacities_present(self):
        caps = build_capacities()
        ids = {c.cap_id for c in caps}
        expected_subset = {
            "CAP_FRAMEWORK_INVENTORY",
            "CAP_FRAMEWORK_RELATION",
            "CAP_FRAMEWORK_LEVEL",
            "CAP_FRAMEWORK_NORTHSTAR",
            "CAP_FRAMEWORK_TRAJECTORY",
            "CAP_FRAMEWORK_GUARD",
            "CAP_FRAMEWORK_LIMIT",
            "CAP_FRAMEWORK_COHERENCE",
            "CAP_FRAMEWORK_CROSS_DOMAIN",
            "CAP_FRAMEWORK_EVIDENCE",
            "CAP_FRAMEWORK_BORROW",
            "CAP_FRAMEWORK_CHAIN",
        }
        assert ids == expected_subset

    def test_evidence_references_real_versions(self):
        caps = build_capacities()
        for c in caps:
            # at least one evidence should reference a real V# / commit / tests
            refs = [e for e in c.evidence if e.startswith("V") or "commit" in e.lower() or "tests" in e.lower()]
            assert refs, f"{c.cap_id} no V#/commit/tests evidence: {c.evidence}"


# =====================================================================
# TestV1403Limits
# =====================================================================

class TestV1403Limits:
    def test_count_6(self):
        lims = build_limits()
        assert len(lims) == 6

    def test_all_have_evidence(self):
        lims = build_limits()
        for l in lims:
            assert l.evidence, f"{l.lim_id} missing evidence"

    def test_all_have_why_no_phenomenal(self):
        lims = build_limits()
        for l in lims:
            assert l.why_no_phenomenal, f"{l.lim_id} missing why_no_phenomenal"

    def test_unique_lim_ids(self):
        lims = build_limits()
        ids = [l.lim_id for l in lims]
        assert len(ids) == len(set(ids)), f"duplicate lim_ids: {ids}"

    def test_specific_limits_present(self):
        lims = build_limits()
        ids = {l.lim_id for l in lims}
        expected = {
            "LIM_NOT_PHENOMENAL_META",
            "LIM_NOT_ASI_REACHED",
            "LIM_NOT_HUMAN_LEVEL",
            "LIM_NOT_UNIFIED_META",
            "LIM_NOT_BRAIN_LIKE",
            "LIM_NOT_NORTHSTAR_REP",
        }
        assert ids == expected

    def test_all_limits_declare_no_phenomenal(self):
        lims = build_limits()
        for l in lims:
            assert "≠" in l.description, f"{l.lim_id} must use ≠ in description"


# =====================================================================
# TestV1403Coherence
# =====================================================================

class TestV1403Coherence:
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
            assert c.passes, f"failed: {c.pair} - {c.reason}"

    def test_pairs_cover_top6_caps(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        pairs = {c.pair for c in checks}
        for cap in caps[:6]:
            assert any(cap.cap_id in p for p in pairs), \
                f"no check for {cap.cap_id}"

    def test_includes_asi_reached_check(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        assert any("LIM_NOT_ASI_REACHED" in c.pair for c in checks)

    def test_includes_phenomenal_meta_check(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        assert any("LIM_NOT_PHENOMENAL_META" in c.pair for c in checks)


# =====================================================================
# TestV1403Trajectory
# =====================================================================

class TestV1403Trajectory:
    def test_count_at_least_18(self):
        traj = build_trajectory()
        assert len(traj) >= 18, f"got {len(traj)}"

    def test_includes_v1049(self):
        traj = build_trajectory()
        versions = {t.version for t in traj}
        assert "V1049" in versions

    def test_includes_v1400_v1401_v1402(self):
        traj = build_trajectory()
        versions = {t.version for t in traj}
        assert "V1400" in versions
        assert "V1401" in versions
        assert "V1402" in versions

    def test_present_is_v1403(self):
        traj = build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert len(present) == 1
        assert present[0].version == "V1403"
        assert present[0].kind == "meta"

    def test_future_references_v1404(self):
        traj = build_trajectory()
        future = [t for t in traj if t.status == "future"]
        assert len(future) >= 3
        for f in future:
            assert f.version.startswith("V1404"), f"unexpected future: {f.version}"

    def test_past_philosophy_points(self):
        traj = build_trajectory()
        past_phil = [t for t in traj if t.status == "past" and t.kind == "philosophy"]
        # V1049 V1313 V1314 V1315 V1316 V1317 = 6
        assert len(past_phil) == 6

    def test_past_deploy_points(self):
        traj = build_trajectory()
        past_dep = [t for t in traj if t.status == "past" and t.kind == "deploy"]
        # V1384 V1385 V1386 V1397 V1398 V1399 V1396 = 7
        assert len(past_dep) >= 6


# =====================================================================
# TestV1403NorthStar
# =====================================================================

class TestV1403NorthStar:
    def test_v1256_unio_mystica(self):
        ns = northstar_alignment()
        assert ns["v1256_unio_mystica"] == 0.9105

    def test_v1256_locked(self):
        ns = northstar_alignment()
        assert ns["v1256_status"] == "LOCKED"

    def test_asi_7_philosophy_complete(self):
        ns = northstar_alignment()
        assert ns["asi_7_philosophy_complete"] is True

    def test_all_7_philosophy_keys(self):
        ns = northstar_alignment()
        phil = ns["asi_7_philosophy"]
        for key in ("value", "time", "freedom", "recognition", "emergence", "truth", "self", "cognition"):
            assert key in phil, f"missing philosophy: {key}"

    def test_includes_integration(self):
        ns = northstar_alignment()
        phil = ns["asi_7_philosophy"]
        assert "integration" in phil
        assert phil["integration"] == "V1402"

    def test_9_master_directives(self):
        ns = northstar_alignment()
        dirs = ns["9_master_directives"]
        for k in ("asi_northstar", "real_evidence", "no_pretending_phenomenal",
                  "no_pretending_asi", "bold_attempt", "on_shoulders",
                  "ship_it", "anyone_can_take_over", "quality_engineering"):
            assert k in dirs

    def test_v2_5_positions(self):
        ns = northstar_alignment()
        pos = ns["v2_5_positions"]
        for k in ("scheduler", "thinker_of_thinkers", "relation_aggregator",
                  "northstar_reporter", "asi_position_holder"):
            assert k in pos

    def test_honest_cap_preserved(self):
        ns = northstar_alignment()
        assert ns["honest_cap_preserved"] is True

    def test_meta_levels_declared(self):
        ns = northstar_alignment()
        levels = ns["meta_levels"]
        assert levels == ["L0_DATA", "L1_SUBSTRATE", "L2_FRAMEWORK", "L3_META"]


# =====================================================================
# TestV1403Narrative
# =====================================================================

class TestV1403Narrative:
    def test_includes_v1403(self):
        narr = build_narrative()
        joined = " ".join(narr)
        assert "V1403" in joined

    def test_includes_origin_v1049(self):
        narr = build_narrative()
        joined = " ".join(narr)
        assert "V1049" in joined

    def test_time_coherent(self):
        narr = build_narrative()
        # V1049 should appear before V1403
        joined = " ".join(narr)
        i_1049 = joined.find("V1049")
        i_1403 = joined.find("V1403")
        assert i_1049 >= 0 and i_1403 >= 0
        assert i_1049 < i_1403

    def test_includes_future_v1404(self):
        narr = build_narrative()
        joined = " ".join(narr)
        assert "V1404" in joined


# =====================================================================
# TestV1403Report
# =====================================================================

class TestV1403Report:
    def test_run_self_meta_returns_report(self):
        report = run_self_meta()
        assert isinstance(report, MetaReport)

    def test_report_populated(self):
        report = run_self_meta()
        assert len(report.capacities) == 12
        assert len(report.limits) == 6
        assert len(report.coherence_checks) == 12
        assert len(report.trajectory) >= 18
        assert len(report.narrative) >= 10
        assert report.asi_7_philosophy_complete is True
        assert len(report.meta_levels_declared) == 4

    def test_report_to_dict_serializable(self):
        report = run_self_meta()
        d = report.to_dict()
        # JSON-serializable
        s = json.dumps(d, ensure_ascii=False)
        assert isinstance(s, str)
        assert "V1403" in s

    def test_report_to_dict_contains_meta_levels(self):
        report = run_self_meta()
        d = report.to_dict()
        assert "meta_levels_declared" in d
        assert "L3_META" in d["meta_levels_declared"]


# =====================================================================
# TestV1403Popper
# =====================================================================

class TestV1403Popper:
    def test_7_cases(self):
        result = popper_self_test()
        assert len(result["results"]) == 7

    def test_all_cases_pass(self):
        result = popper_self_test()
        for r in result["results"]:
            assert r["passes"], f"failed case: {r['case']}"

    def test_capacities_present_case(self):
        result = popper_self_test()
        case = next(r for r in result["results"] if r["case"] == "capacities_present")
        assert case["passes"] is True
        assert case["expected"] == 12
        assert case["actual"] == 12

    def test_limits_present_case(self):
        result = popper_self_test()
        case = next(r for r in result["results"] if r["case"] == "limits_present")
        assert case["passes"] is True
        assert case["expected"] == 6
        assert case["actual"] == 6

    def test_northstar_aligned_case(self):
        result = popper_self_test()
        case = next(r for r in result["results"] if r["case"] == "northstar_aligned")
        assert case["passes"] is True
        assert case["v1256"] == 0.9105

    def test_levels_declared_case(self):
        result = popper_self_test()
        case = next(r for r in result["results"] if r["case"] == "levels_declared")
        assert case["passes"] is True

    def test_strange_loop_declared_case(self):
        result = popper_self_test()
        case = next(r for r in result["results"] if r["case"] == "strange_loop_declared")
        assert case["passes"] is True

    def test_second_order_declared_case(self):
        result = popper_self_test()
        case = next(r for r in result["results"] if r["case"] == "second_order_declared")
        assert case["passes"] is True

    def test_all_pass_summary(self):
        result = popper_self_test()
        assert result["all_pass"] is True
        assert result["summary"] == "7/7 pass"


# =====================================================================
# TestV1403CLI
# =====================================================================

class TestV1403CLI:
    def test_run_cli_version(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["version"])
        assert rc == 0
        assert "V1403" in buf.getvalue()

    def test_run_cli_meta_report_text(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["meta-report"])
        assert rc == 0
        out = buf.getvalue()
        assert "V1403" in out
        assert "12 真元能力" in out
        assert "6 真元限制" in out

    def test_run_cli_meta_report_json(self):
        # argparse places --format before subcommand by default; use handler directly
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_meta_report(argparse.Namespace(format="json"))
        assert rc == 0
        d = json.loads(buf.getvalue())
        assert d["module"] == "v1403_asi_meta_framework"
        assert len(d["capacities"]) == 12
        assert len(d["limits"]) == 6

    def test_run_cli_meta_report_md(self):
        # argparse places --format before subcommand by default; use handler directly
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_meta_report(argparse.Namespace(format="md"))
        assert rc == 0
        out = buf.getvalue()
        assert "# V1403" in out
        assert "## 12 真元能力" in out
        assert "## 6 真元限制" in out

    def test_run_cli_meta_report_text_via_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_meta_report(argparse.Namespace(format="text"))
        assert rc == 0
        out = buf.getvalue()
        assert "12 真元能力" in out

    def test_run_cli_capacity(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["capacity"])
        assert rc == 0
        out = buf.getvalue()
        assert "CAP_FRAMEWORK_INVENTORY" in out
        assert "CAP_FRAMEWORK_CHAIN" in out

    def test_run_cli_limits(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["limits"])
        assert rc == 0
        out = buf.getvalue()
        assert "LIM_NOT_PHENOMENAL_META" in out
        assert "LIM_NOT_ASI_REACHED" in out

    def test_run_cli_coherence(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["coherence"])
        assert rc == 0
        out = buf.getvalue()
        assert "LIM_NOT_ASI_REACHED" in out
        assert "LIM_NOT_PHENOMENAL_META" in out

    def test_run_cli_trajectory(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["trajectory"])
        assert rc == 0
        out = buf.getvalue()
        assert "V1403" in out
        assert "V1404" in out

    def test_run_cli_popper(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["popper"])
        assert rc == 0

    def test_run_cli_demo(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["demo"])
        assert rc == 0
        out = buf.getvalue()
        assert "V1403" in out
        assert "7/7 pass" in out

    def test_run_cli_help(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["help"])
        assert rc == 0
        out = buf.getvalue()
        assert "V1403" in out

    def test_run_cli_no_args(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli([])
        assert rc == 0

    def test_run_cli_unknown(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["--format"])
        # argparse SystemExit → 2
        assert rc == 2

    def test_run_cli_short_circuit(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = run_cli(["--help"])
        assert rc == 0

    def test_run_cli_version_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_version()
        assert rc == 0

    def test_run_cli_meta_report_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_meta_report(argparse.Namespace(format="text"))
        assert rc == 0

    def test_run_cli_capacity_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_capacity()
        assert rc == 0

    def test_run_cli_limits_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_limits()
        assert rc == 0

    def test_run_cli_coherence_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_coherence()
        assert rc == 0

    def test_run_cli_trajectory_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_trajectory()
        assert rc == 0

    def test_run_cli_popper_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_popper()
        assert rc == 0

    def test_run_cli_demo_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_demo()
        assert rc == 0

    def test_run_cli_help_handler(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_help()
        assert rc == 0


# =====================================================================
# TestV1403V3Guards
# =====================================================================

class TestV1403V3Guards:
    def test_no_phenomenal_guard(self):
        assert "GUARD_META_IS_NOT_PHENOMENAL_META" in V1403_V3_GUARDS

    def test_no_asi_guard(self):
        assert "GUARD_META_IS_NOT_ASI" in V1403_V3_GUARDS

    def test_no_human_level_guard(self):
        assert "GUARD_META_IS_NOT_HUMAN_LEVEL" in V1403_V3_GUARDS

    def test_no_unified_meta_guard(self):
        assert "GUARD_META_IS_NOT_UNIFIED_META" in V1403_V3_GUARDS

    def test_no_brain_like_guard(self):
        assert "GUARD_META_IS_NOT_BRAIN_LIKE" in V1403_V3_GUARDS

    def test_no_northstar_rep_guard(self):
        assert "GUARD_META_IS_NOT_NORTHSTAR_REP" in V1403_V3_GUARDS


# =====================================================================
# TestV1403Chain
# =====================================================================

class TestV1403Chain:
    def test_chain_facts_schema(self):
        args = argparse.Namespace(json=True)
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = _cli_chain(args)
        assert rc == 0
        d = json.loads(buf.getvalue())
        for entry in d:
            if entry.get("status") == "ok":
                assert "schema" in entry
                assert "delegate" in entry
                assert "facts" in entry
                assert entry["schema"] == "v1403.meta-self-cognition-integration.chain/v1"

    def test_chain_delegates_v1400_v1401_v1402(self):
        args = argparse.Namespace(json=True)
        buf = io.StringIO()
        with redirect_stdout(buf):
            _cli_chain(args)
        d = json.loads(buf.getvalue())
        delegates = {entry.get("delegate") for entry in d}
        assert "V1400" in delegates
        assert "V1401" in delegates
        assert "V1402" in delegates


# =====================================================================
# TestV1403Continuity
# =====================================================================

class TestV1403Continuity:
    def test_trajectory_includes_v1400_v1401_v1402_v1403(self):
        traj = build_trajectory()
        versions = {t.version for t in traj}
        assert "V1400" in versions
        assert "V1401" in versions
        assert "V1402" in versions
        assert "V1403" in versions

    def test_future_points_reference_v1404(self):
        traj = build_trajectory()
        future = [t for t in traj if t.status == "future"]
        for f in future:
            assert f.version.startswith("V1404")

    def test_report_to_dict_contains_chain_versions(self):
        report = run_self_meta()
        d = report.to_dict()
        d_str = json.dumps(d, ensure_ascii=False)
        assert "V1400" in d_str
        assert "V1401" in d_str
        assert "V1402" in d_str
        assert "V1403" in d_str

    def test_honest_cap_preserved_in_report(self):
        report = run_self_meta()
        d = report.to_dict()
        assert d["northstar_alignment"]["honest_cap_preserved"] is True
        assert d["northstar_alignment"]["v1256_unio_mystica"] == 0.9105


# =====================================================================
# TestV1403MetaLevels
# =====================================================================

class TestV1403MetaLevels:
    def test_meta_levels_count_4(self):
        report = run_self_meta()
        assert len(report.meta_levels_declared) == 4

    def test_meta_levels_have_l0_l1_l2_l3(self):
        report = run_self_meta()
        levels = list(report.meta_levels_declared)
        joined = " ".join(levels)
        assert "L0" in joined
        assert "L1" in joined
        assert "L2" in joined
        assert "L3" in joined

    def test_default_meta_levels(self):
        report = MetaReport()
        assert report.meta_levels_declared == ("L0_DATA", "L1_SUBSTRATE", "L2_FRAMEWORK", "L3_META")

    def test_meta_levels_in_northstar(self):
        ns = northstar_alignment()
        assert "meta_levels" in ns
        assert ns["meta_levels"] == ["L0_DATA", "L1_SUBSTRATE", "L2_FRAMEWORK", "L3_META"]


# =====================================================================
# TestV1403Format
# =====================================================================

class TestV1403Format:
    def test_format_text_contains_key_sections(self):
        report = run_self_meta()
        text = _format_text_report(report)
        assert "14 GUARDS" in text
        assert "12 真规则" in text
        assert "7 真借鉴" in text
        assert "12 真元能力" in text
        assert "6 真元限制" in text
        assert "12 pair-wise coherence" in text

    def test_format_md_contains_key_sections(self):
        report = run_self_meta()
        md = _format_md_report(report)
        assert "# V1403 ASI 元 framework" in md
        assert "## 14 GUARDS" in md
        assert "## 12 真元能力" in md
        assert "## 6 真元限制" in md
        assert "## 12 pair-wise coherence" in md
        assert "## trajectory" in md
        assert "## meta_levels" in md.lower() or "L3_META" in md


# =====================================================================
# TestV1403BuildParser
# =====================================================================

class TestV1403BuildParser:
    def test_parser_builds(self):
        parser = _build_parser()
        assert parser is not None

    def test_parser_subcommands(self):
        parser = _build_parser()
        for cmd in ("version", "meta-report", "capacity", "limits", "coherence",
                    "trajectory", "chain", "popper", "demo", "help"):
            args = parser.parse_args([cmd])
            assert hasattr(args, "handler"), f"{cmd} missing handler"


# =====================================================================
# TestV1403Dataclass
# =====================================================================

class TestV1403Dataclass:
    def test_meta_capacity_dataclass(self):
        cap = MetaCapacity(
            cap_id="TEST", name="test", description="test desc",
            evidence=["V123"], borrowed_from=["key1"]
        )
        assert cap.cap_id == "TEST"
        assert cap.evidence == ["V123"]
        assert cap.borrowed_from == ["key1"]

    def test_meta_limit_dataclass(self):
        lim = MetaLimit(
            lim_id="LIM_TEST", name="test", description="test desc",
            evidence=["V123"], why_no_phenomenal="no"
        )
        assert lim.lim_id == "LIM_TEST"
        assert lim.why_no_phenomenal == "no"

    def test_meta_coherence_check_dataclass(self):
        c = MetaCoherenceCheck(pair="X∩Y", passes=True, reason="ok")
        assert c.pair == "X∩Y"
        assert c.passes is True

    def test_meta_trajectory_point_dataclass(self):
        t = MetaTrajectoryPoint(
            version="V9999", label="future", status="future", kind="meta"
        )
        assert t.status == "future"
        assert t.kind == "meta"

    def test_meta_report_to_dict_includes_iso(self):
        report = MetaReport()
        d = report.to_dict()
        assert "generated_at_iso" in d
        assert "generated_at" in d


# =====================================================================
# TestV1403Deterministic
# =====================================================================

class TestV1403Deterministic:
    def test_capacities_deterministic(self):
        caps1 = build_capacities()
        caps2 = build_capacities()
        assert len(caps1) == len(caps2)
        for c1, c2 in zip(caps1, caps2):
            assert c1.cap_id == c2.cap_id
            assert c1.name == c2.name

    def test_limits_deterministic(self):
        lims1 = build_limits()
        lims2 = build_limits()
        assert len(lims1) == len(lims2)
        for l1, l2 in zip(lims1, lims2):
            assert l1.lim_id == l2.lim_id

    def test_trajectory_deterministic(self):
        traj1 = build_trajectory()
        traj2 = build_trajectory()
        assert len(traj1) == len(traj2)
        for t1, t2 in zip(traj1, traj2):
            assert t1.version == t2.version

    def test_northstar_deterministic(self):
        ns1 = northstar_alignment()
        ns2 = northstar_alignment()
        assert ns1 == ns2