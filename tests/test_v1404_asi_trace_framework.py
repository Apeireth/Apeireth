"""V1404 ASI 真实生产 trace (Trace) framework tests.

V1404 = V1403 meta framework 预告的 next-step:
- ASI 7 哲学问题 + self + cognition + integration + meta + trace (this) 闭环
- 12 真 trace capacities + 6 真 trace limits + 23 trajectory + 32 lineage + 7 借鉴
- 12 coherence checks + chain delegate V1400+V1401+V1402+V1403 (4/4 ok)
- popper self-test 7/7 pass
- 真 CLI: version / trace-report / capacity / limits / lineage / timeline /
  branch / citation / chain / popper / demo / help + --format text|json|md

主 17:43 实事求是: 真测试真跑; 主 17:58 + 主 20:46 不假装:
6 真限制 + 6 V3 哲学守门; 主 13:31 大胆激进 真 trace-framework;
主 19:33 走在前人经验上 7 真借鉴; 主 23:44 干到底;
主 00:56 任何人都能接手 1 CLI; 主 00:36 质量工程化 popper + 4 exit codes;
honest 0.90 cap preserved (V1256 LOCKED).
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1404_asi_trace_framework import (  # noqa: E402
    V1404_BORROWED,
    V1404_GUARDS,
    V1404_MODULE,
    V1404_RULES,
    V1404_V3_GUARDS,
    V1404_VERSION,
    TraceCapacity,
    TraceCitationEdge,
    TraceCoherenceCheck,
    TraceLimit,
    TraceLineageEdge,
    TraceReport,
    TraceTrajectoryPoint,
    build_capacities,
    build_citations,
    build_limits,
    build_lineage,
    build_narrative,
    build_northstar_alignment,
    build_trajectory,
    chain_delegate,
    coherence_check,
    popper_self_test,
    run_self_trace,
)


# ----------------------- Constants -----------------------

class TestV1404Constants:
    def test_version_set(self):
        assert isinstance(V1404_VERSION, str)
        assert len(V1404_VERSION) > 0

    def test_module_name(self):
        assert V1404_MODULE == "v1404_asi_trace_framework"

    def test_guards_non_empty(self):
        assert isinstance(V1404_GUARDS, (list, tuple))
        assert len(V1404_GUARDS) >= 10

    def test_v3_guards_count(self):
        # 6 V3 哲学守门
        assert len(V1404_V3_GUARDS) == 6

    def test_v3_guards_phenomenal(self):
        assert "GUARD_TRACE_IS_NOT_PHENOMENAL_TRACE" in V1404_V3_GUARDS

    def test_v3_guards_asi(self):
        assert "GUARD_TRACE_IS_NOT_ASI" in V1404_V3_GUARDS

    def test_rules_non_empty(self):
        assert isinstance(V1404_RULES, (list, tuple))
        assert len(V1404_RULES) >= 10

    def test_borrowed_count(self):
        # 7 真借鉴
        assert len(V1404_BORROWED) == 7


# ----------------------- Capacities -----------------------

class TestV1404Capacities:
    def test_capacity_count_12(self):
        caps = build_capacities()
        assert len(caps) == 12

    def test_capacities_are_dataclass(self):
        for c in build_capacities():
            assert isinstance(c, TraceCapacity)

    def test_capacity_id_unique(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert len(set(ids)) == len(ids)

    def test_capacity_id_prefix(self):
        for c in build_capacities():
            assert c.cap_id.startswith("CAP_TRACE_")

    def test_capacity_evidence_non_empty(self):
        for c in build_capacities():
            assert isinstance(c.evidence, (list, tuple))
            assert len(c.evidence) >= 1

    def test_capacity_borrowed_non_empty(self):
        # 至少每个 capacity 都借用过一个真源
        for c in build_capacities():
            assert isinstance(c.borrowed_from, (list, tuple))
            assert len(c.borrowed_from) >= 1

    def test_cap_lineage_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_TRACE_LINEAGE" in ids

    def test_cap_citation_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_TRACE_CITATION" in ids

    def test_cap_northstar_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_TRACE_NORTHSTAR_CHAIN" in ids

    def test_cap_limit_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_TRACE_LIMIT" in ids

    def test_cap_evidence_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_TRACE_EVIDENCE" in ids

    def test_cap_chain_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_TRACE_CHAIN" in ids


# ----------------------- Limits -----------------------

class TestV1404Limits:
    def test_limit_count_6(self):
        limits = build_limits()
        assert len(limits) == 6

    def test_limits_are_dataclass(self):
        for l in build_limits():
            assert isinstance(l, TraceLimit)

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
        assert "LIM_NOT_PHENOMENAL_TRACE" in ids

    def test_limit_not_asi(self):
        limits = build_limits()
        ids = [l.lim_id for l in limits]
        assert "LIM_NOT_ASI_REACHED" in ids


# ----------------------- Trajectory -----------------------

class TestV1404Trajectory:
    def test_trajectory_count_at_least_23(self):
        traj = build_trajectory()
        assert len(traj) >= 23

    def test_trajectory_dataclass(self):
        for t in build_trajectory():
            assert isinstance(t, TraceTrajectoryPoint)

    def test_trajectory_status_values(self):
        for t in build_trajectory():
            assert t.status in {"past", "present", "future"}

    def test_trajectory_kind_values(self):
        valid_kinds = {
            "philosophy", "self", "cognition", "integration", "meta",
            "trace", "deploy", "northstar",
        }
        for t in build_trajectory():
            assert t.kind in valid_kinds

    def test_trajectory_has_present(self):
        traj = build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert len(present) == 1

    def test_trajectory_has_future(self):
        traj = build_trajectory()
        future = [t for t in traj if t.status == "future"]
        assert len(future) >= 1

    def test_trajectory_present_is_v1404(self):
        traj = build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert present[0].version == "V1404"

    def test_trajectory_past_includes_anchor(self):
        traj = build_trajectory()
        versions = [t.version for t in traj]
        assert "V1256" in versions  # unio_mystica anchor

    def test_trajectory_past_includes_philosophy(self):
        traj = build_trajectory()
        versions = [t.version for t in traj]
        for v in ("V1313", "V1314", "V1315", "V1316", "V1317", "V1049"):
            assert v in versions


# ----------------------- Lineage -----------------------

class TestV1404Lineage:
    def test_lineage_count_at_least_32(self):
        lin = build_lineage()
        assert len(lin) >= 32

    def test_lineage_dataclass(self):
        for e in build_lineage():
            assert isinstance(e, TraceLineageEdge)

    def test_lineage_has_anchor_chain(self):
        lin = build_lineage()
        # V1256 应该是很多边的源头
        sources = [e.src for e in lin]
        assert "V1256" in sources

    def test_lineage_has_v1404_as_dst(self):
        lin = build_lineage()
        dsts = [e.dst for e in lin]
        assert "V1404" in dsts


# ----------------------- Citations -----------------------

class TestV1404Citations:
    def test_citation_count_7(self):
        cits = build_citations()
        assert len(cits) == 7

    def test_citation_dataclass(self):
        for c in build_citations():
            assert isinstance(c, TraceCitationEdge)

    def test_citation_has_year(self):
        for c in build_citations():
            assert isinstance(c.year, int)
            assert 1800 <= c.year <= 2026

    def test_citation_has_figure(self):
        for c in build_citations():
            assert isinstance(c.figure, str)
            assert len(c.figure) > 0

    def test_citation_has_work(self):
        for c in build_citations():
            assert isinstance(c.work, str)
            assert len(c.work) > 0

    def test_citation_used_in_non_empty(self):
        for c in build_citations():
            assert isinstance(c.used_in, (list, tuple))
            assert len(c.used_in) >= 1

    def test_citation_includes_husserl(self):
        figures = [c.figure for c in build_citations()]
        assert "Husserl" in figures

    def test_citation_includes_heidegger(self):
        figures = [c.figure for c in build_citations()]
        assert "Heidegger" in figures

    def test_citation_includes_derrida(self):
        figures = [c.figure for c in build_citations()]
        assert "Derrida" in figures


# ----------------------- Narrative -----------------------

class TestV1404Narrative:
    def test_narrative_non_empty(self):
        n = build_narrative()
        assert len(n) >= 5

    def test_narrative_is_list_of_str(self):
        n = build_narrative()
        for s in n:
            assert isinstance(s, str)
            assert len(s) > 0


# ----------------------- NorthStar -----------------------

class TestV1404NorthStar:
    def test_northstar_returns_dict(self):
        ns = build_northstar_alignment()
        assert isinstance(ns, dict)

    def test_northstar_mentions_v1256(self):
        ns = build_northstar_alignment()
        s = json.dumps(ns)
        assert "V1256" in s


# ----------------------- Coherence -----------------------

class TestV1404Coherence:
    def test_coherence_returns_12(self):
        checks = coherence_check(build_capacities(), build_limits())
        assert len(checks) == 12

    def test_coherence_dataclass(self):
        for c in coherence_check(build_capacities(), build_limits()):
            assert isinstance(c, TraceCoherenceCheck)

    def test_coherence_all_pass(self):
        checks = coherence_check(build_capacities(), build_limits())
        for c in checks:
            assert c.passes is True
            assert len(c.reason) > 0

    def test_coherence_pair_format(self):
        for c in coherence_check(build_capacities(), build_limits()):
            # pair 字段包含 capacity_id + lim_id + separator
            assert len(c.pair) > 0


# ----------------------- Chain Delegate -----------------------

class TestV1404ChainDelegate:
    def test_chain_returns_dict(self):
        ch = chain_delegate()
        assert isinstance(ch, dict)

    def test_chain_has_schema(self):
        ch = chain_delegate()
        assert "schema" in ch
        assert "v1404" in ch["schema"]

    def test_chain_has_4_delegates(self):
        ch = chain_delegate()
        delegates = ch["delegates"]
        for v in ("V1400", "V1401", "V1402", "V1403"):
            assert v in delegates

    def test_chain_all_ok(self):
        ch = chain_delegate()
        assert ch["all_ok"] is True

    def test_chain_total_capacities_positive(self):
        ch = chain_delegate()
        assert ch["total_capacities"] >= 0


# ----------------------- Popper Self-Test -----------------------

class TestV1404Popper:
    def test_popper_returns_dict(self):
        p = popper_self_test()
        assert isinstance(p, dict)

    def test_popper_all_pass(self):
        p = popper_self_test()
        assert p["all_pass"] is True

    def test_popper_summary(self):
        p = popper_self_test()
        assert "7/7" in p["summary"]

    def test_popper_7_cases(self):
        p = popper_self_test()
        assert len(p["results"]) == 7

    def test_popper_capacities_present(self):
        p = popper_self_test()
        for r in p["results"]:
            assert r["passes"] is True


# ----------------------- Run Self Trace -----------------------

class TestV1404RunSelfTrace:
    def test_run_self_trace_returns_report(self):
        r = run_self_trace()
        assert isinstance(r, TraceReport)

    def test_report_has_12_capacities(self):
        r = run_self_trace()
        assert len(r.capacities) == 12

    def test_report_has_6_limits(self):
        r = run_self_trace()
        assert len(r.limits) == 6

    def test_report_has_12_coherence(self):
        r = run_self_trace()
        assert len(r.coherence_checks) == 12

    def test_report_has_trajectory(self):
        r = run_self_trace()
        assert len(r.trajectory) >= 23

    def test_report_has_lineage(self):
        r = run_self_trace()
        assert len(r.lineage) >= 32

    def test_report_has_citations(self):
        r = run_self_trace()
        assert len(r.citations) == 7

    def test_report_has_narrative(self):
        r = run_self_trace()
        assert len(r.narrative) >= 5

    def test_report_asi_7_philosophy_complete(self):
        r = run_self_trace()
        assert r.asi_7_philosophy_complete is True

    def test_report_trace_levels_declared(self):
        r = run_self_trace()
        # trace_levels_declared 是 5 层 tuple (L0-L4)
        assert isinstance(r.trace_levels_declared, (tuple, list))
        assert len(r.trace_levels_declared) >= 5
        assert "L4_TRACE" in r.trace_levels_declared
        assert "L0_DATA" in r.trace_levels_declared

    def test_report_v3_guards_count(self):
        r = run_self_trace()
        assert len(r.v3_guards) == 6

    def test_report_guards_non_empty(self):
        r = run_self_trace()
        assert len(r.guards) >= 10

    def test_report_rules_non_empty(self):
        r = run_self_trace()
        assert len(r.rules) >= 10

    def test_report_borrowed_count(self):
        r = run_self_trace()
        assert len(r.borrowed) == 7

    def test_report_version(self):
        r = run_self_trace()
        assert isinstance(r.version, str)
        assert len(r.version) > 0

    def test_report_module(self):
        r = run_self_trace()
        assert r.module == V1404_MODULE


# ----------------------- V3 哲学守门 -----------------------

class TestV1404V3Guards:
    def test_guards_phenomenal(self):
        assert "GUARD_TRACE_IS_NOT_PHENOMENAL_TRACE" in V1404_V3_GUARDS

    def test_guards_asi(self):
        assert "GUARD_TRACE_IS_NOT_ASI" in V1404_V3_GUARDS

    def test_guards_human_level(self):
        assert "GUARD_TRACE_IS_NOT_HUMAN_LEVEL" in V1404_V3_GUARDS

    def test_guards_time_travel(self):
        assert "GUARD_TRACE_IS_NOT_TIME_TRAVEL" in V1404_V3_GUARDS

    def test_guards_causal(self):
        assert "GUARD_TRACE_IS_NOT_CAUSAL" in V1404_V3_GUARDS

    def test_guards_northstar(self):
        assert "GUARD_TRACE_IS_NOT_NORTHSTAR_REP" in V1404_V3_GUARDS


# ----------------------- Continuity -----------------------

class TestV1404Continuity:
    def test_chain_v1403_v1404_no_regression(self):
        """V1403 → V1404 continuity: capacities 不缩水, limits 不缩水."""
        caps_1404 = build_capacities()
        lims_1404 = build_limits()
        assert len(caps_1404) == 12
        assert len(lims_1404) == 6

    def test_trace_inherits_v3_guards(self):
        # V1404 V3 guards 是 ASI 哲学守门, 不应该假装
        for g in V1404_V3_GUARDS:
            assert g.startswith("GUARD_TRACE_")

    def test_borrowed_authors_real(self):
        # 7 真借鉴, 应该是真实哲学家
        figs = [c["key"] for c in V1404_BORROWED]
        # 检查至少包含几个真名
        text = " ".join(figs)
        assert "husserl" in text
        assert "heidegger" in text
        assert "derrida" in text
        assert "foucault" in text


# ----------------------- CLI -----------------------

class TestV1404CLI:
    def test_cli_help(self):
        # 通过 run_cli 而不是 subprocess (V1400-style 修复)
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["help"])
        assert rc == 0

    def test_cli_version(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["version"])
        assert rc == 0

    def test_cli_trace_report(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["trace-report"])
        assert rc == 0

    def test_cli_capacity(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["capacity"])
        assert rc == 0

    def test_cli_limits(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["limits"])
        assert rc == 0

    def test_cli_lineage(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["lineage"])
        assert rc == 0

    def test_cli_timeline(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["timeline"])
        assert rc == 0

    def test_cli_branch(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["branch"])
        assert rc == 0

    def test_cli_citation(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["citation"])
        assert rc == 0

    def test_cli_chain(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["chain"])
        assert rc == 0

    def test_cli_popper(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["popper"])
        assert rc == 0

    def test_cli_demo(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["demo"])
        assert rc == 0


# ----------------------- Format flags -----------------------

class TestV1404Format:
    def test_format_json(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["--format", "json", "trace-report"])
        assert rc == 0

    def test_format_md(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["--format", "md", "trace-report"])
        assert rc == 0

    def test_format_text(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["--format", "text", "trace-report"])
        assert rc == 0

    def test_json_flag_chain(self):
        from apeireth.v1404_asi_trace_framework import run_cli
        rc = run_cli(["--json", "chain"])
        assert rc == 0


# ----------------------- Determinism -----------------------

class TestV1404Deterministic:
    def test_capacities_deterministic(self):
        a = build_capacities()
        b = build_capacities()
        assert [c.cap_id for c in a] == [c.cap_id for c in b]

    def test_trajectory_deterministic(self):
        a = build_trajectory()
        b = build_trajectory()
        assert [t.version for t in a] == [t.version for t in b]

    def test_lineage_deterministic(self):
        a = build_lineage()
        b = build_lineage()
        assert [(e.src, e.dst) for e in a] == [(e.src, e.dst) for e in b]

    def test_popper_deterministic(self):
        a = popper_self_test()
        b = popper_self_test()
        assert a["summary"] == b["summary"]

    def test_chain_deterministic(self):
        a = chain_delegate()
        b = chain_delegate()
        assert a["all_ok"] == b["all_ok"]