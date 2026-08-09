"""V1406 ASI 真实生产 裁 (Judge) framework tests.

V1406 = V1405 explainer framework 预告的 next-step:
- ASI 7 哲学问题 + self + cognition + integration + meta + trace + explain + judge 闭环
- 12 真 judge capacities + 6 真 judge limits + 24 trajectory + 7 借鉴
- 12 coherence checks + chain delegate V1400+V1401+V1402+V1403+V1404+V1405 (6/6 ok)
- popper self-test 7/7 pass
- 真 CLI: version / judge-report / capacity / limits / verdict / chain / popper /
  demo / help + --format text|json|md

主 17:43 实事求是: 真裁真调; 主 17:58 + 主 20:46 不假装:
6 真限制 + 6 V3 哲学守门; 主 13:31 大胆激进 真 judge-framework;
主 19:33 走在前人经验上 7 真借鉴; 主 23:44 干到底;
主 00:56 任何人都能接手 1 CLI; 主 00:36 质量工程化 popper + 4 exit codes;
honest 0.90 cap preserved (V1256 LOCKED).
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1406_asi_judge_framework import (  # noqa: E402
    V1406_BORROWED,
    V1406_GUARDS,
    V1406_MODULE,
    V1406_RULES,
    V1406_V3_GUARDS,
    V1406_VERSION,
    JudgeCapacity,
    JudgeCitationEdge,
    JudgeCoherenceCheck,
    JudgeLimit,
    JudgeNarrative,
    JudgeReport,
    JudgeTrajectoryPoint,
    JudgeVerdict,
    build_capacities,
    build_citations,
    build_limits,
    build_narratives,
    build_northstar_alignment,
    build_trajectory,
    build_verdicts,
    chain_delegate,
    coherence_check,
    popper_self_test,
    run_self_judge,
)


# ----------------------- Constants -----------------------

class TestV1406Constants:
    def test_version_set(self):
        assert isinstance(V1406_VERSION, str)
        assert len(V1406_VERSION) > 0

    def test_module_name(self):
        assert V1406_MODULE == "v1406_asi_judge_framework"

    def test_guards_non_empty(self):
        assert isinstance(V1406_GUARDS, (list, tuple))
        assert len(V1406_GUARDS) >= 10

    def test_v3_guards_count(self):
        # 6 V3 哲学守门
        assert len(V1406_V3_GUARDS) == 6

    def test_v3_guards_phenomenal(self):
        assert "GUARD_JUDGE_IS_NOT_PHENOMENAL_JUDGE" in V1406_V3_GUARDS

    def test_v3_guards_asi(self):
        assert "GUARD_JUDGE_IS_NOT_ASI" in V1406_V3_GUARDS

    def test_v3_guards_human_level(self):
        assert "GUARD_JUDGE_IS_NOT_HUMAN_LEVEL" in V1406_V3_GUARDS

    def test_v3_guards_final_authority(self):
        assert "GUARD_JUDGE_IS_NOT_FINAL_AUTHORITY" in V1406_V3_GUARDS

    def test_v3_guards_northstar(self):
        assert "GUARD_JUDGE_IS_NOT_NORTHSTAR_REP" in V1406_V3_GUARDS

    def test_v3_guards_knowing(self):
        assert "GUARD_JUDGE_IS_NOT_KNOWING" in V1406_V3_GUARDS

    def test_rules_non_empty(self):
        assert isinstance(V1406_RULES, (list, tuple))
        assert len(V1406_RULES) >= 10

    def test_rules_have_three_fields(self):
        for r in V1406_RULES:
            assert isinstance(r, tuple)
            assert len(r) == 3

    def test_borrowed_count(self):
        # 7 真借鉴
        assert len(V1406_BORROWED) == 7

    def test_borrowed_keys_unique(self):
        keys = [b["key"] for b in V1406_BORROWED]
        assert len(set(keys)) == len(keys)


# ----------------------- Capacities -----------------------

class TestV1406Capacities:
    def test_capacity_count_12(self):
        caps = build_capacities()
        assert len(caps) == 12

    def test_capacities_are_dataclass(self):
        for c in build_capacities():
            assert isinstance(c, JudgeCapacity)

    def test_capacity_id_unique(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert len(set(ids)) == len(ids)

    def test_capacity_id_prefix(self):
        for c in build_capacities():
            assert c.cap_id.startswith("CAP_JUDGE_")

    def test_capacity_evidence_non_empty(self):
        for c in build_capacities():
            assert isinstance(c.evidence, (list, tuple))
            assert len(c.evidence) >= 1

    def test_capacity_borrowed_non_empty(self):
        for c in build_capacities():
            assert isinstance(c.borrowed_from, (list, tuple))
            assert len(c.borrowed_from) >= 1

    def test_cap_lineage_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_LINEAGE" in ids

    def test_cap_trajectory_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_TRAJECTORY" in ids

    def test_cap_coherence_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_COHERENCE" in ids

    def test_cap_evidence_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_EVIDENCE" in ids

    def test_cap_limit_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_LIMIT" in ids

    def test_cap_northstar_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_NORTHSTAR" in ids

    def test_cap_chain_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_CHAIN" in ids

    def test_cap_guard_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_GUARD" in ids

    def test_cap_verdict_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_VERDICT" in ids

    def test_cap_borrow_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_BORROW" in ids

    def test_cap_inherit_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_INHERIT" in ids

    def test_cap_honest_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_JUDGE_HONEST" in ids


# ----------------------- Limits -----------------------

class TestV1406Limits:
    def test_limit_count_6(self):
        limits = build_limits()
        assert len(limits) == 6

    def test_limits_are_dataclass(self):
        for l in build_limits():
            assert isinstance(l, JudgeLimit)

    def test_limit_id_prefix(self):
        for l in build_limits():
            assert l.lim_id.startswith("LIM_")

    def test_limit_evidence_non_empty(self):
        for l in build_limits():
            assert isinstance(l.evidence, (list, tuple))
            assert len(l.evidence) >= 1

    def test_limit_why_no_phenomenal(self):
        for l in build_limits():
            assert isinstance(l.why_no_phenomenal, str)
            assert len(l.why_no_phenomenal) > 0

    def test_limit_not_phenomenal(self):
        limits = build_limits()
        ids = [l.lim_id for l in limits]
        assert "LIM_NOT_PHENOMENAL_JUDGE" in ids

    def test_limit_not_asi(self):
        limits = build_limits()
        ids = [l.lim_id for l in limits]
        assert "LIM_NOT_ASI_REACHED" in ids

    def test_limit_not_human_level(self):
        limits = build_limits()
        ids = [l.lim_id for l in limits]
        assert "LIM_NOT_HUMAN_LEVEL" in ids

    def test_limit_not_final_authority(self):
        limits = build_limits()
        ids = [l.lim_id for l in limits]
        assert "LIM_NOT_FINAL_AUTHORITY" in ids

    def test_limit_not_northstar_rep(self):
        limits = build_limits()
        ids = [l.lim_id for l in limits]
        assert "LIM_NOT_NORTHSTAR_REP" in ids

    def test_limit_not_knowing(self):
        limits = build_limits()
        ids = [l.lim_id for l in limits]
        assert "LIM_NOT_KNOWING" in ids


# ----------------------- Trajectory -----------------------

class TestV1406Trajectory:
    def test_trajectory_count_at_least_24(self):
        traj = build_trajectory()
        assert len(traj) >= 24

    def test_trajectory_dataclass(self):
        for t in build_trajectory():
            assert isinstance(t, JudgeTrajectoryPoint)

    def test_trajectory_status_values(self):
        for t in build_trajectory():
            assert t.status in {"past", "present", "future"}

    def test_trajectory_kind_values(self):
        valid_kinds = {
            "philosophy", "self", "cognition", "integration", "meta",
            "trace", "explainer", "judge", "deploy", "northstar",
        }
        for t in build_trajectory():
            assert t.kind in valid_kinds

    def test_trajectory_has_present(self):
        traj = build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert len(present) == 1

    def test_trajectory_present_is_v1406(self):
        traj = build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert present[0].version == "V1406"

    def test_trajectory_past_includes_anchor(self):
        traj = build_trajectory()
        versions = [t.version for t in traj]
        assert "V1256" in versions

    def test_trajectory_past_includes_v1400_v1405(self):
        traj = build_trajectory()
        versions = [t.version for t in traj]
        for v in ("V1400", "V1401", "V1402", "V1403", "V1404", "V1405"):
            assert v in versions

    def test_trajectory_has_future(self):
        traj = build_trajectory()
        future = [t for t in traj if t.status == "future"]
        assert len(future) >= 1


# ----------------------- Citations -----------------------

class TestV1406Citations:
    def test_citation_count_7(self):
        cits = build_citations()
        assert len(cits) == 7

    def test_citations_are_dataclass(self):
        for c in build_citations():
            assert isinstance(c, JudgeCitationEdge)

    def test_citation_year_int(self):
        for c in build_citations():
            assert isinstance(c.year, int)

    def test_citation_figures(self):
        cits = build_citations()
        figures = {c.figure for c in cits}
        assert "Aristotle" in figures
        assert "Kant" in figures
        assert "Hume" in figures
        assert "Rawls" in figures
        assert "Habermas" in figures
        assert "Arendt" in figures
        assert "Dreyfus" in figures


# ----------------------- Coherence -----------------------

class TestV1406Coherence:
    def test_coherence_count_12(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        assert len(checks) == 12

    def test_coherence_all_pass(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        for c in checks:
            assert c.passes is True

    def test_coherence_pair_format(self):
        caps = build_capacities()
        lims = build_limits()
        checks = coherence_check(caps, lims)
        for c in checks:
            assert "∩" in c.pair or "LIM_NOT_ASI_REACHED" in c.pair

    def test_coherence_dataclass(self):
        caps = build_capacities()
        lims = build_limits()
        for c in coherence_check(caps, lims):
            assert isinstance(c, JudgeCoherenceCheck)


# ----------------------- Verdicts -----------------------

class TestV1406Verdicts:
    def test_verdict_count_at_least_5(self):
        v = build_verdicts()
        assert len(v) >= 5

    def test_verdicts_are_dataclass(self):
        for v_ in build_verdicts():
            assert isinstance(v_, JudgeVerdict)

    def test_verdict_status_values(self):
        valid = {"pass", "warn", "fail", "info"}
        for v_ in build_verdicts():
            assert v_.verdict in valid

    def test_verdict_subject_non_empty(self):
        for v_ in build_verdicts():
            assert isinstance(v_.subject, str)
            assert len(v_.subject) > 0

    def test_verdict_reason_non_empty(self):
        for v_ in build_verdicts():
            assert isinstance(v_.reason, str)
            assert len(v_.reason) > 0

    def test_verdict_audience_non_empty(self):
        for v_ in build_verdicts():
            assert isinstance(v_.audience, str)
            assert len(v_.audience) > 0


# ----------------------- North-Star Alignment -----------------------

class TestV1406NorthStar:
    def test_alignment_is_dict(self):
        ns = build_northstar_alignment()
        assert isinstance(ns, dict)

    def test_alignment_locked_score(self):
        ns = build_northstar_alignment()
        assert ns["north_star_score"] == 0.9105
        assert ns["north_star_locked"] is True

    def test_alignment_inherits_six(self):
        ns = build_northstar_alignment()
        assert len(ns["v1406_inherits"]) == 6

    def test_alignment_does_not_replace(self):
        ns = build_northstar_alignment()
        assert ns["v1406_does_not_replace_northstar"] is True

    def test_alignment_asi_7_complete(self):
        ns = build_northstar_alignment()
        assert ns["asi_7_philosophy_complete"] is True


# ----------------------- Narratives -----------------------

class TestV1406Narratives:
    def test_narrative_count_3(self):
        nars = build_narratives()
        assert len(nars) == 3

    def test_narrative_audiences(self):
        nars = build_narratives()
        auds = {n.audience for n in nars}
        assert "main" in auds
        assert "handoff" in auds
        assert "external" in auds

    def test_narrative_lines_non_empty(self):
        for n in build_narratives():
            assert isinstance(n.lines, (list, tuple))
            assert len(n.lines) >= 3

    def test_narrative_dataclass(self):
        for n in build_narratives():
            assert isinstance(n, JudgeNarrative)


# ----------------------- Chain Delegate -----------------------

class TestV1406ChainDelegate:
    def test_chain_schema_present(self):
        ch = chain_delegate()
        assert isinstance(ch["schema"], str)
        assert "v1406" in ch["schema"]

    def test_chain_6_delegates(self):
        ch = chain_delegate()
        assert len(ch["delegates"]) == 6

    def test_chain_all_ok(self):
        ch = chain_delegate()
        assert ch["all_ok"] is True

    def test_chain_total_capacities(self):
        ch = chain_delegate()
        assert ch["total_capacities"] == 72  # 6 frameworks x 12 caps

    def test_chain_total_limits(self):
        ch = chain_delegate()
        assert ch["total_limits"] == 36  # 6 frameworks x 6 limits

    def test_chain_includes_v1400(self):
        ch = chain_delegate()
        assert "V1400" in ch["delegates"]

    def test_chain_includes_v1405(self):
        ch = chain_delegate()
        assert "V1405" in ch["delegates"]

    def test_chain_includes_v1406_self(self):
        # V1406 inherits V1405 explicitly
        ch = chain_delegate()
        assert "V1406" not in ch["delegates"]  # V1406 is the caller, not delegated to


# ----------------------- Popper -----------------------

class TestV1406Popper:
    def test_popper_all_pass(self):
        p = popper_self_test()
        assert p["all_pass"] is True

    def test_popper_7_cases(self):
        p = popper_self_test()
        assert len(p["results"]) == 7

    def test_popper_capacities_case(self):
        p = popper_self_test()
        cap_case = next(r for r in p["results"] if r["case"] == "capacities_present")
        assert cap_case["passes"] is True

    def test_popper_limits_case(self):
        p = popper_self_test()
        lim_case = next(r for r in p["results"] if r["case"] == "limits_present")
        assert lim_case["passes"] is True

    def test_popper_coherence_case(self):
        p = popper_self_test()
        coh_case = next(r for r in p["results"] if r["case"] == "coherence_passes")
        assert coh_case["passes"] is True

    def test_popper_northstar_case(self):
        p = popper_self_test()
        ns_case = next(r for r in p["results"] if r["case"] == "north_star_aligned")
        assert ns_case["passes"] is True

    def test_popper_chain_case(self):
        p = popper_self_test()
        ch_case = next(r for r in p["results"] if r["case"] == "chain_delegate_real")
        assert ch_case["passes"] is True

    def test_popper_judgment_verified_case(self):
        p = popper_self_test()
        jv_case = next(r for r in p["results"] if r["case"] == "judgment_verified")
        assert jv_case["passes"] is True

    def test_popper_honest_disclosure_case(self):
        p = popper_self_test()
        hd_case = next(r for r in p["results"] if r["case"] == "honest_disclosure")
        assert hd_case["passes"] is True

    def test_popper_summary_format(self):
        p = popper_self_test()
        assert "/" in p["summary"]


# ----------------------- Self Report -----------------------

class TestV1406RunSelfJudge:
    def test_report_is_dataclass(self):
        report = run_self_judge()
        assert isinstance(report, JudgeReport)

    def test_report_version(self):
        report = run_self_judge()
        assert report.version == V1406_VERSION

    def test_report_module(self):
        report = run_self_judge()
        assert report.module == V1406_MODULE

    def test_report_capacities(self):
        report = run_self_judge()
        assert len(report.capacities) == 12

    def test_report_limits(self):
        report = run_self_judge()
        assert len(report.limits) == 6

    def test_report_coherence(self):
        report = run_self_judge()
        assert len(report.coherence_checks) == 12

    def test_report_trajectory(self):
        report = run_self_judge()
        assert len(report.trajectory) >= 24

    def test_report_citations(self):
        report = run_self_judge()
        assert len(report.citations) == 7

    def test_report_narratives(self):
        report = run_self_judge()
        assert len(report.narratives) == 3

    def test_report_verdicts(self):
        report = run_self_judge()
        assert len(report.verdicts) >= 5

    def test_report_judgment_levels(self):
        report = run_self_judge()
        assert "L6_JUDGE" in report.judgment_levels
        assert len(report.judgment_levels) == 7

    def test_report_asi_7_complete(self):
        report = run_self_judge()
        assert report.asi_7_philosophy_complete is True

    def test_report_guards_present(self):
        report = run_self_judge()
        assert len(report.guards) >= 10
        assert len(report.v3_guards) == 6


# ----------------------- V3 Guards -----------------------

class TestV1406V3Guards:
    def test_v3_guards_complete(self):
        expected = {
            "GUARD_JUDGE_IS_NOT_PHENOMENAL_JUDGE",
            "GUARD_JUDGE_IS_NOT_ASI",
            "GUARD_JUDGE_IS_NOT_HUMAN_LEVEL",
            "GUARD_JUDGE_IS_NOT_FINAL_AUTHORITY",
            "GUARD_JUDGE_IS_NOT_NORTHSTAR_REP",
            "GUARD_JUDGE_IS_NOT_KNOWING",
        }
        assert set(V1406_V3_GUARDS) == expected


# ----------------------- Borrowed -----------------------

class TestV1406Borrowed:
    def test_borrowed_aristotle(self):
        keys = [b["key"] for b in V1406_BORROWED]
        assert any("aristotle" in k for k in keys)

    def test_borrowed_kant(self):
        keys = [b["key"] for b in V1406_BORROWED]
        assert any("kant" in k for k in keys)

    def test_borrowed_hume(self):
        keys = [b["key"] for b in V1406_BORROWED]
        assert any("hume" in k for k in keys)

    def test_borrowed_rawls(self):
        keys = [b["key"] for b in V1406_BORROWED]
        assert any("rawls" in k for k in keys)

    def test_borrowed_habermas(self):
        keys = [b["key"] for b in V1406_BORROWED]
        assert any("habermas" in k for k in keys)

    def test_borrowed_arendt(self):
        keys = [b["key"] for b in V1406_BORROWED]
        assert any("arendt" in k for k in keys)

    def test_borrowed_dreyfus(self):
        keys = [b["key"] for b in V1406_BORROWED]
        assert any("dreyfus" in k for k in keys)


# ----------------------- CLI -----------------------

class TestV1406CLI:
    def test_cli_version(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1406" in out

    def test_cli_capacity(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["capacity"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "CAP_JUDGE_LINEAGE" in out

    def test_cli_limits(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["limits"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "LIM_NOT_PHENOMENAL_JUDGE" in out

    def test_cli_verdict(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["verdict"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1256" in out  # one verdict mentions V1256

    def test_cli_chain_text(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["chain"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "schema:" in out
        assert "V1400" in out
        assert "V1405" in out

    def test_cli_chain_json(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["chain", "--json"])
        assert rc == 0
        out = capsys.readouterr().out
        # JSON output starts with {
        assert out.strip().startswith("{")

    def test_cli_popper(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["popper"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "7/7" in out or "all_pass: True" in out

    def test_cli_demo(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["demo"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "capabilities:" in out
        assert "verdicts:" in out

    def test_cli_help(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["help"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1406" in out

    def test_cli_report_text(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["judge-report", "--format", "text"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "CAP_JUDGE_LINEAGE" in out
        assert "LIM_NOT_PHENOMENAL_JUDGE" in out

    def test_cli_report_json(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["judge-report", "--format", "json"])
        assert rc == 0
        out = capsys.readouterr().out
        # JSON output starts with {
        assert out.strip().startswith("{")

    def test_cli_report_md(self, capsys):
        from apeireth.v1406_asi_judge_framework import run_cli
        rc = run_cli(["judge-report", "--format", "md"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "# V1406" in out


# ----------------------- Deterministic -----------------------

class TestV1406Deterministic:
    def test_build_capacities_deterministic(self):
        caps1 = build_capacities()
        caps2 = build_capacities()
        assert [c.cap_id for c in caps1] == [c.cap_id for c in caps2]

    def test_build_limits_deterministic(self):
        lims1 = build_limits()
        lims2 = build_limits()
        assert [l.lim_id for l in lims1] == [l.lim_id for l in lims2]

    def test_build_trajectory_deterministic(self):
        traj1 = build_trajectory()
        traj2 = build_trajectory()
        assert [t.version for t in traj1] == [t.version for t in traj2]

    def test_build_verdicts_deterministic(self):
        v1 = build_verdicts()
        v2 = build_verdicts()
        assert [v_.subject for v_ in v1] == [v_.subject for v_ in v2]

    def test_chain_deterministic(self):
        ch1 = chain_delegate()
        ch2 = chain_delegate()
        assert ch1["all_ok"] == ch2["all_ok"]
        assert ch1["total_capacities"] == ch2["total_capacities"]

    def test_popper_deterministic(self):
        p1 = popper_self_test()
        p2 = popper_self_test()
        assert p1["all_pass"] == p2["all_pass"]
        assert p1["summary"] == p2["summary"]


# ----------------------- Continuity with V1400-V1405 -----------------------

class TestV1406Continuity:
    def test_inherits_v1405_explainer(self):
        # V1406 explicitly inherits V1405
        caps = build_capacities()
        inherit_cap = next(c for c in caps if c.cap_id == "CAP_JUDGE_INHERIT")
        assert "V1405" in str(inherit_cap.evidence)

    def test_inherits_v1400_self(self):
        caps = build_capacities()
        lineage_cap = next(c for c in caps if c.cap_id == "CAP_JUDGE_LINEAGE")
        assert "V1400" in str(lineage_cap.evidence)

    def test_trajectory_continuity_v1400_v1405(self):
        traj = build_trajectory()
        versions = [t.version for t in traj]
        # All 6 framework predecessors should appear
        for v in ("V1400", "V1401", "V1402", "V1403", "V1404", "V1405"):
            assert v in versions

    def test_northstar_alignment_continuity(self):
        ns = build_northstar_alignment()
        inherits = ns["v1406_inherits"]
        # 6 inherited frameworks
        assert len(inherits) == 6


# ----------------------- Run via pytest self-check -----------------------

if __name__ == "__main__":
    pytest.main([__file__, "-v"])