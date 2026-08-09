"""V1405 ASI 真实生产 释 (Explainer) framework tests.

V1405 = V1404 trace framework 预告的 next-step:
- ASI 7 哲学问题 + self + cognition + integration + meta + trace + explain 闭环
- 12 真 explainer capacities + 6 真 explainer limits + 23 trajectory + 7 借鉴
- 12 coherence checks + chain delegate V1400+V1401+V1402+V1403+V1404 (5/5 ok)
- popper self-test 7/7 pass
- 真 CLI: version / explainer-report / capacity / limits / narrative / chain /
  popper / demo / help + --format text|json|md

主 17:43 实事求是: 真解释真调; 主 17:58 + 主 20:46 不假装:
6 真限制 + 6 V3 哲学守门; 主 13:31 大胆激进 真 explainer-framework;
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

from apeireth.v1405_asi_explainer_framework import (  # noqa: E402
    V1405_BORROWED,
    V1405_GUARDS,
    V1405_MODULE,
    V1405_RULES,
    V1405_V3_GUARDS,
    V1405_VERSION,
    ExplainerCapacity,
    ExplainerCitationEdge,
    ExplainerCoherenceCheck,
    ExplainerLimit,
    ExplainerNarrative,
    ExplainerReport,
    ExplainerTrajectoryPoint,
    build_capacities,
    build_citations,
    build_limits,
    build_narratives,
    build_northstar_alignment,
    build_trajectory,
    chain_delegate,
    coherence_check,
    popper_self_test,
    run_self_explainer,
)


# ----------------------- Constants -----------------------

class TestV1405Constants:
    def test_version_set(self):
        assert isinstance(V1405_VERSION, str)
        assert len(V1405_VERSION) > 0

    def test_module_name(self):
        assert V1405_MODULE == "v1405_asi_explainer_framework"

    def test_guards_non_empty(self):
        assert isinstance(V1405_GUARDS, (list, tuple))
        assert len(V1405_GUARDS) >= 10

    def test_v3_guards_count(self):
        # 6 V3 哲学守门
        assert len(V1405_V3_GUARDS) == 6

    def test_v3_guards_phenomenal(self):
        assert "GUARD_EXPLAIN_IS_NOT_PHENOMENAL_EXPLAIN" in V1405_V3_GUARDS

    def test_v3_guards_asi(self):
        assert "GUARD_EXPLAIN_IS_NOT_ASI" in V1405_V3_GUARDS

    def test_v3_guards_human_level(self):
        assert "GUARD_EXPLAIN_IS_NOT_HUMAN_LEVEL" in V1405_V3_GUARDS

    def test_v3_guards_final_authority(self):
        assert "GUARD_EXPLAIN_IS_NOT_FINAL_AUTHORITY" in V1405_V3_GUARDS

    def test_v3_guards_northstar(self):
        assert "GUARD_EXPLAIN_IS_NOT_NORTHSTAR_REP" in V1405_V3_GUARDS

    def test_v3_guards_knowing(self):
        assert "GUARD_EXPLAIN_IS_NOT_KNOWING" in V1405_V3_GUARDS

    def test_rules_non_empty(self):
        assert isinstance(V1405_RULES, (list, tuple))
        assert len(V1405_RULES) >= 10

    def test_borrowed_count(self):
        # 7 真借鉴
        assert len(V1405_BORROWED) == 7


# ----------------------- Capacities -----------------------

class TestV1405Capacities:
    def test_capacity_count_12(self):
        caps = build_capacities()
        assert len(caps) == 12

    def test_capacities_are_dataclass(self):
        for c in build_capacities():
            assert isinstance(c, ExplainerCapacity)

    def test_capacity_id_unique(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert len(set(ids)) == len(ids)

    def test_capacity_id_prefix(self):
        for c in build_capacities():
            assert c.cap_id.startswith("CAP_EXPLAIN_")

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
        assert "CAP_EXPLAIN_LINEAGE" in ids

    def test_cap_audience_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_EXPLAIN_AUDIENCE" in ids

    def test_cap_level_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_EXPLAIN_LEVEL" in ids

    def test_cap_chain_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_EXPLAIN_CHAIN" in ids

    def test_cap_honest_present(self):
        caps = build_capacities()
        ids = [c.cap_id for c in caps]
        assert "CAP_EXPLAIN_HONEST" in ids


# ----------------------- Limits -----------------------

class TestV1405Limits:
    def test_limit_count_6(self):
        limits = build_limits()
        assert len(limits) == 6

    def test_limits_are_dataclass(self):
        for l in build_limits():
            assert isinstance(l, ExplainerLimit)

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
        assert "LIM_NOT_PHENOMENAL_EXPLAIN" in ids

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

    def test_limit_not_knowing(self):
        limits = build_limits()
        ids = [l.lim_id for l in limits]
        assert "LIM_NOT_KNOWING" in ids


# ----------------------- Trajectory -----------------------

class TestV1405Trajectory:
    def test_trajectory_count_at_least_23(self):
        traj = build_trajectory()
        assert len(traj) >= 23

    def test_trajectory_dataclass(self):
        for t in build_trajectory():
            assert isinstance(t, ExplainerTrajectoryPoint)

    def test_trajectory_status_values(self):
        for t in build_trajectory():
            assert t.status in {"past", "present", "future"}

    def test_trajectory_kind_values(self):
        valid_kinds = {
            "philosophy", "self", "cognition", "integration", "meta",
            "trace", "explainer", "deploy", "northstar",
        }
        for t in build_trajectory():
            assert t.kind in valid_kinds

    def test_trajectory_has_present(self):
        traj = build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert len(present) == 1

    def test_trajectory_present_is_v1405(self):
        traj = build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert present[0].version == "V1405"

    def test_trajectory_past_includes_anchor(self):
        traj = build_trajectory()
        versions = [t.version for t in traj]
        assert "V1256" in versions

    def test_trajectory_past_includes_v1400_v1404(self):
        traj = build_trajectory()
        versions = [t.version for t in traj]
        for v in ("V1400", "V1401", "V1402", "V1403", "V1404"):
            assert v in versions

    def test_trajectory_has_future(self):
        traj = build_trajectory()
        future = [t for t in traj if t.status == "future"]
        assert len(future) >= 1


# ----------------------- Citations -----------------------

class TestV1405Citations:
    def test_citation_count_7(self):
        cits = build_citations()
        assert len(cits) == 7

    def test_citation_dataclass(self):
        for c in build_citations():
            assert isinstance(c, ExplainerCitationEdge)

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

    def test_citation_includes_aristotle(self):
        figures = [c.figure for c in build_citations()]
        assert "Aristotle" in figures

    def test_citation_includes_toulmin(self):
        figures = [c.figure for c in build_citations()]
        assert "Toulmin" in figures

    def test_citation_includes_grice(self):
        figures = [c.figure for c in build_citations()]
        assert "Grice" in figures

    def test_citation_includes_habermas(self):
        figures = [c.figure for c in build_citations()]
        assert "Habermas" in figures

    def test_citation_includes_sperber(self):
        figures = [c.figure for c in build_citations()]
        assert "Sperber & Wilson" in figures

    def test_citation_includes_perelman(self):
        figures = [c.figure for c in build_citations()]
        assert "Perelman & Olbrechts-Tyteca" in figures

    def test_citation_includes_bender(self):
        figures = [c.figure for c in build_citations()]
        assert "Bender, Gebru et al." in figures


# ----------------------- Narratives -----------------------

class TestV1405Narratives:
    def test_narrative_count_at_least_3(self):
        nars = build_narratives()
        assert len(nars) >= 3

    def test_narrative_dataclass(self):
        for n in build_narratives():
            assert isinstance(n, ExplainerNarrative)

    def test_narrative_has_audience(self):
        for n in build_narratives():
            assert n.audience in {"main", "handoff", "external"}

    def test_narrative_has_level(self):
        for n in build_narratives():
            assert n.level.startswith("L")

    def test_narrative_has_lines(self):
        for n in build_narratives():
            assert isinstance(n.lines, (list, tuple))
            assert len(n.lines) >= 3

    def test_narrative_has_main(self):
        auds = {n.audience for n in build_narratives()}
        assert "main" in auds

    def test_narrative_has_handoff(self):
        auds = {n.audience for n in build_narratives()}
        assert "handoff" in auds

    def test_narrative_has_external(self):
        auds = {n.audience for n in build_narratives()}
        assert "external" in auds


# ----------------------- NorthStar -----------------------

class TestV1405NorthStar:
    def test_northstar_returns_dict(self):
        ns = build_northstar_alignment()
        assert isinstance(ns, dict)

    def test_northstar_mentions_v1256(self):
        ns = build_northstar_alignment()
        assert ns["north_star_version"] == "V1256"

    def test_northstar_score_09105(self):
        ns = build_northstar_alignment()
        assert ns["north_star_score"] == 0.9105

    def test_northstar_locked(self):
        ns = build_northstar_alignment()
        assert ns["north_star_locked"] is True

    def test_northstar_asi_7_complete(self):
        ns = build_northstar_alignment()
        assert ns["asi_7_philosophy_complete"] is True

    def test_northstar_inherits_5_frameworks(self):
        ns = build_northstar_alignment()
        assert len(ns["v1405_inherits"]) >= 5

    def test_northstar_does_not_replace(self):
        ns = build_northstar_alignment()
        assert ns["v1405_does_not_replace_northstar"] is True


# ----------------------- Coherence -----------------------

class TestV1405Coherence:
    def test_coherence_returns_12(self):
        checks = coherence_check(build_capacities(), build_limits())
        assert len(checks) == 12

    def test_coherence_dataclass(self):
        for c in coherence_check(build_capacities(), build_limits()):
            assert isinstance(c, ExplainerCoherenceCheck)

    def test_coherence_all_pass(self):
        checks = coherence_check(build_capacities(), build_limits())
        for c in checks:
            assert c.passes is True
            assert len(c.reason) > 0


# ----------------------- Chain Delegate -----------------------

class TestV1405ChainDelegate:
    def test_chain_returns_dict(self):
        ch = chain_delegate()
        assert isinstance(ch, dict)

    def test_chain_has_schema(self):
        ch = chain_delegate()
        assert "schema" in ch
        assert "v1405" in ch["schema"]

    def test_chain_has_5_delegates(self):
        ch = chain_delegate()
        delegates = ch["delegates"]
        for v in ("V1400", "V1401", "V1402", "V1403", "V1404"):
            assert v in delegates

    def test_chain_all_ok(self):
        ch = chain_delegate()
        assert ch["all_ok"] is True

    def test_chain_total_capacities(self):
        ch = chain_delegate()
        # 5 frameworks × 12 = 60
        assert ch["total_capacities"] == 60

    def test_chain_total_limits(self):
        ch = chain_delegate()
        # 5 frameworks × 6 = 30
        assert ch["total_limits"] == 30

    def test_chain_each_ok(self):
        ch = chain_delegate()
        for v, d in ch["delegates"].items():
            assert d["ok"] is True, f"{v} failed: {d.get('error', 'unknown')}"
            assert d["n_capacities"] == 12, f"{v} n_cap={d['n_capacities']}"
            assert d["n_limits"] == 6, f"{v} n_lim={d['n_limits']}"


# ----------------------- Popper Self-Test -----------------------

class TestV1405Popper:
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

    def test_popper_includes_chain(self):
        p = popper_self_test()
        cases = [r["case"] for r in p["results"]]
        assert "chain_delegate_real" in cases

    def test_popper_includes_audience(self):
        p = popper_self_test()
        cases = [r["case"] for r in p["results"]]
        assert "audience_aware" in cases

    def test_popper_includes_honest(self):
        p = popper_self_test()
        cases = [r["case"] for r in p["results"]]
        assert "honest_disclosure" in cases


# ----------------------- Run Self Explainer -----------------------

class TestV1405RunSelfExplainer:
    def test_run_returns_report(self):
        r = run_self_explainer()
        assert isinstance(r, ExplainerReport)

    def test_report_has_12_capacities(self):
        r = run_self_explainer()
        assert len(r.capacities) == 12

    def test_report_has_6_limits(self):
        r = run_self_explainer()
        assert len(r.limits) == 6

    def test_report_has_12_coherence(self):
        r = run_self_explainer()
        assert len(r.coherence_checks) == 12

    def test_report_has_trajectory(self):
        r = run_self_explainer()
        assert len(r.trajectory) >= 23

    def test_report_has_citations(self):
        r = run_self_explainer()
        assert len(r.citations) == 7

    def test_report_has_narratives(self):
        r = run_self_explainer()
        assert len(r.narratives) >= 3

    def test_report_asi_7_philosophy_complete(self):
        r = run_self_explainer()
        assert r.asi_7_philosophy_complete is True

    def test_report_explanation_levels(self):
        r = run_self_explainer()
        assert "L5_EXPLAIN" in r.explanation_levels

    def test_report_v3_guards_count(self):
        r = run_self_explainer()
        assert len(r.v3_guards) == 6

    def test_report_guards_non_empty(self):
        r = run_self_explainer()
        assert len(r.guards) >= 10

    def test_report_rules_non_empty(self):
        r = run_self_explainer()
        assert len(r.rules) >= 10

    def test_report_borrowed_count(self):
        r = run_self_explainer()
        assert len(r.borrowed) == 7

    def test_report_version(self):
        r = run_self_explainer()
        assert isinstance(r.version, str)
        assert len(r.version) > 0

    def test_report_module(self):
        r = run_self_explainer()
        assert r.module == V1405_MODULE

    def test_report_has_iso(self):
        r = run_self_explainer()
        assert isinstance(r.generated_at_iso, str)
        assert len(r.generated_at_iso) > 0


# ----------------------- V3 哲学守门 -----------------------

class TestV1405V3Guards:
    def test_guards_phenomenal(self):
        assert "GUARD_EXPLAIN_IS_NOT_PHENOMENAL_EXPLAIN" in V1405_V3_GUARDS

    def test_guards_asi(self):
        assert "GUARD_EXPLAIN_IS_NOT_ASI" in V1405_V3_GUARDS

    def test_guards_human_level(self):
        assert "GUARD_EXPLAIN_IS_NOT_HUMAN_LEVEL" in V1405_V3_GUARDS

    def test_guards_final_authority(self):
        assert "GUARD_EXPLAIN_IS_NOT_FINAL_AUTHORITY" in V1405_V3_GUARDS

    def test_guards_northstar(self):
        assert "GUARD_EXPLAIN_IS_NOT_NORTHSTAR_REP" in V1405_V3_GUARDS

    def test_guards_knowing(self):
        assert "GUARD_EXPLAIN_IS_NOT_KNOWING" in V1405_V3_GUARDS

    def test_all_v3_guards_start_with_GUARD_EXPLAIN(self):
        for g in V1405_V3_GUARDS:
            assert g.startswith("GUARD_EXPLAIN_")


# ----------------------- Continuity -----------------------

class TestV1405Continuity:
    def test_chain_v1404_v1405_no_regression(self):
        """V1404 → V1405 continuity."""
        caps_1405 = build_capacities()
        lims_1405 = build_limits()
        assert len(caps_1405) == 12
        assert len(lims_1405) == 6

    def test_inherits_v1404_trace(self):
        """V1405 should inherit V1404 trace via trajectory."""
        traj = build_trajectory()
        versions = [t.version for t in traj]
        assert "V1404" in versions

    def test_borrowed_authors_real(self):
        keys = [b["key"] for b in V1405_BORROWED]
        text = " ".join(keys)
        assert "aristotle" in text
        assert "toulmin" in text
        assert "grice" in text
        assert "habermas" in text
        assert "bender" in text

    def test_honest_cap_preserved(self):
        ns = build_northstar_alignment()
        assert "0.9105" in str(ns)
        assert "honest cap" in ns["v1405_self_alignment"].lower() or "LOCKED" in ns["v1405_self_alignment"]


# ----------------------- CLI -----------------------

class TestV1405CLI:
    def test_cli_help(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["help"])
        assert rc == 0

    def test_cli_version(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["version"])
        assert rc == 0

    def test_cli_explainer_report(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["explainer-report"])
        assert rc == 0

    def test_cli_capacity(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["capacity"])
        assert rc == 0

    def test_cli_limits(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["limits"])
        assert rc == 0

    def test_cli_narrative_all(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["narrative", "--audience", "all"])
        assert rc == 0

    def test_cli_narrative_main(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["narrative", "--audience", "main"])
        assert rc == 0

    def test_cli_chain(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["chain"])
        assert rc == 0

    def test_cli_chain_json(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["chain", "--json"])
        assert rc == 0

    def test_cli_popper(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["popper"])
        assert rc == 0

    def test_cli_demo(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["demo"])
        assert rc == 0


# ----------------------- Format flags -----------------------

class TestV1405Format:
    def test_format_json(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["--format", "json", "explainer-report"])
        assert rc == 0

    def test_format_md(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["--format", "md", "explainer-report"])
        assert rc == 0

    def test_format_text(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["--format", "text", "explainer-report"])
        assert rc == 0

    def test_json_flag_chain(self):
        from apeireth.v1405_asi_explainer_framework import run_cli
        rc = run_cli(["--json", "chain"])
        assert rc == 0


# ----------------------- Determinism -----------------------

class TestV1405Deterministic:
    def test_capacities_deterministic(self):
        a = build_capacities()
        b = build_capacities()
        assert [c.cap_id for c in a] == [c.cap_id for c in b]

    def test_trajectory_deterministic(self):
        a = build_trajectory()
        b = build_trajectory()
        assert [t.version for t in a] == [t.version for t in b]

    def test_citations_deterministic(self):
        a = build_citations()
        b = build_citations()
        assert [c.figure for c in a] == [c.figure for c in b]

    def test_popper_deterministic(self):
        a = popper_self_test()
        b = popper_self_test()
        assert a["summary"] == b["summary"]

    def test_chain_deterministic(self):
        a = chain_delegate()
        b = chain_delegate()
        assert a["all_ok"] == b["all_ok"]
        assert a["total_capacities"] == b["total_capacities"]